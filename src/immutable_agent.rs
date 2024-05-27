// use crate::exec_python::run_python_capture;
// use crate::exec_python::*;
use crate::llama_structs::*;
use crate::llm_llama_local::*;
use crate::message_store::*;
use crate::{
    ROUTER_AGENT_SYSTEM_PROMPT,
    IS_TERMINATION_SYSTEM_PROMPT,
    CODE_PYTHON_SYSTEM_MESSAGE,
    ITERATE_CODING_FAIL_TEMPLATE,
    ITERATE_CODING_SUCCESS_TEMPLATE,
};
use anyhow;
use async_openai::types::Role;
use rusqlite::Connection;
use serde::{ Deserialize, Serialize };
use serde_json::{ Value };
use std::collections::{ HashMap };
use regex::Regex;

pub fn run_python_capture(code: &str) -> anyhow::Result<String, String> {
    todo!()
}
pub fn extract_code(text: &str) -> String {
    let multi_line_pattern = r"```python(.*?)```";
    let mut program = String::new();

    let multi_line_regex = Regex::new(multi_line_pattern).unwrap();
    for cap in multi_line_regex.captures_iter(text) {
        let code = cap.get(1).unwrap().as_str().trim().to_string();
        program.push_str(&code);
    }

    program
}

pub fn parse_next_speaker(input: &str) -> (String, String) {
    let json_regex = Regex::new(r"\{[^}]*\}").unwrap();
    let json_str = json_regex
        .captures(input)
        .and_then(|cap| cap.get(0))
        .map_or(String::new(), |m| m.as_str().to_string());

    let continue_to_work_or_end_regex = Regex::new(
        r#""continue_to_work_or_end":\s*"([^"]*)""#
    ).unwrap();
    let next_speaker_regex = Regex::new(r#""next_speaker":\s*"([^"]*)""#).unwrap();

    let continue_to_work_or_end = continue_to_work_or_end_regex
        .captures(&json_str)
        .and_then(|cap| cap.get(1))
        .map_or(String::new(), |m| m.as_str().to_string());

    let next_speaker = next_speaker_regex
        .captures(&json_str)
        .and_then(|cap| cap.get(1))
        .map_or(String::new(), |m| m.as_str().to_string());

    (continue_to_work_or_end, next_speaker)
}

pub fn parse_result_and_key_points(input: &str) -> (bool, String) {
    let json_regex = Regex::new(r"\{[^}]*\}").unwrap();
    let json_str = json_regex
        .captures(input)
        .and_then(|cap| cap.get(0))
        .map_or(String::new(), |m| m.as_str().to_string());

    let continue_to_work_or_end_regex = Regex::new(
        r#""continue_to_work_or_end":\s*"([^"]*)""#
    ).unwrap();
    let next_speaker_regex = Regex::new(r#""key_points":\s*"([^"]*)""#).unwrap();

    let continue_to_work_or_end = continue_to_work_or_end_regex
        .captures(&json_str)
        .and_then(|cap| cap.get(1))
        .map_or(String::new(), |m| m.as_str().to_string());

    let key_points = next_speaker_regex
        .captures(&json_str)
        .and_then(|cap| cap.get(1))
        .map_or(String::new(), |m| m.as_str().to_string());

    (&continue_to_work_or_end == "TERMINATE", key_points)
}

type Context = HashMap<String, String>;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    pub content: Content,
    pub name: Option<String>,
    pub role: Role,
}

impl Default for Message {
    fn default() -> Self {
        Message {
            content: Content::Text("placeholder".to_string()),
            name: None,
            role: Role::User,
        }
    }
}

impl Message {
    pub fn new(content: Content, name: Option<String>, role: Option<Role>) -> Self {
        Message {
            content,
            name,
            role: Role::Assistant, // Set default role to Assistant if None is provided
        }
    }
}

pub struct ImmutableAgent {
    pub name: String,
    pub system_prompt: String,
    pub llm_config: Option<Value>,
    pub tools_map_meta: String,
    pub description: String,
}

impl ImmutableAgent {
    pub fn new(
        name: &str,
        system_prompt: &str,
        llm_config: Option<Value>,
        tools_map_meta: &str,
        description: &str
    ) -> Self {
        ImmutableAgent {
            name: name.to_string(),
            system_prompt: system_prompt.to_string(),
            llm_config,
            tools_map_meta: tools_map_meta.to_string(),
            description: description.to_string(),
        }
    }

    pub fn router_agent(name: &str, llm_config: Option<Value>, tools_map_meta: &str) -> Self {
        ImmutableAgent {
            name: "router_agent".to_string(),
            system_prompt: ROUTER_AGENT_SYSTEM_PROMPT.to_string(),
            llm_config,
            tools_map_meta: tools_map_meta.to_string(),
            description: "router agent".to_string(),
        }
    }

    pub async fn send(&self, message: Message, conn: &Connection, next_speaker: Option<&str>) {
        let _ = save_message(conn, &self.name, message, next_speaker.unwrap()).await;
    }

    pub async fn receive_message(&self, conn: &Connection) -> Option<Message> {
        let next_speaker = get_next_speaker_db(conn).await.ok()?;
        if next_speaker == self.name {
            retrieve_most_recent_message(conn, &self.name).await.ok()
        } else {
            None
        }
    }

    pub async fn a_generate_reply(
        &self,
        messages: Vec<Message>,
        conn: &Connection
    ) -> Option<Vec<Message>> {
        let mut message_vec = messages.clone();
        let user_prompt = match messages.clone().get(1) {
            Some(p) => p.content_to_string(),
            None => String::new(),
        };
        let max_token = 1000u16;
        let output: LlamaResponseMessage = chat_inner_async_llama(messages, max_token).await.expect(
            "Failed to generate reply"
        );

        match &output.content {
            Content::Text(_out) => {
                let message = Message {
                    name: None,
                    content: output.content,
                    role: Role::User,
                };
                let (terminate_or_not, next_speaker) = self.assign_next_speaker(
                    &message,
                    &user_prompt,
                    conn
                ).await;
                if terminate_or_not {
                } else {
                    let _ = save_message(
                        conn,
                        &self.name,
                        message.clone(),
                        &next_speaker.unwrap_or("placeholder".to_string())
                    ).await;
                    message_vec.push(message);
                }
            }
            Content::ToolCall(call) => {
                // let func = call.name;
                // let args = call.arguments.unwrap_or_default();
                // Execute the tool call function
                // func(args);
            }
        }

        Some(message_vec)
    }

    pub async fn assign_next_speaker(
        &self,
        message: &Message,
        instruction: &str,
        conn: &Connection
    ) -> (bool, Option<String>) {
        let user_prompt = match get_agent_names_and_abilities(conn).await {
            Err(_) =>
                format!(
                    "Given the task: {:?}, examine current result: {}, please decide whether the task is done or need further work",
                    instruction,
                    message.content_to_string()
                ),
            Ok(c) =>
                format!(
                    "Here are the list of agents and their abilities: {:?}, examine current result: {} against the task {:?}, please decide which is the next speaker to handle",
                    c,
                    instruction,
                    message.content_to_string()
                ),
        };

        let messages = vec![
            Message {
                role: Role::System,
                name: None,
                content: Content::Text(self.system_prompt.clone()),
            },
            Message {
                role: Role::User,
                name: None,
                content: Content::Text(user_prompt),
            }
        ];

        let raw_reply = chat_inner_async_llama(messages, 100).await.expect(
            "llm generation failure"
        );

        let (terminate_or_not, next_speaker) = parse_next_speaker(&raw_reply.content_to_string());

        match terminate_or_not == "TERMINATE" {
            true => (true, None),
            false => {
                let _ = save_message(conn, &self.name, message.clone(), &next_speaker).await;

                (false, Some(next_speaker.to_string()))
            }
        }
    }
    pub async fn _is_termination(
        &self,
        current_text_result: &str,
        instruction: &str,
        conn: &Connection
    ) -> (bool, String) {
        let user_prompt = format!(
            "Given the task: {:?}, examine current result: {}, please decide whether the task is done or not",
            instruction,
            current_text_result
        );

        let messages = vec![
            Message {
                role: Role::System,
                name: None,
                content: Content::Text(IS_TERMINATION_SYSTEM_PROMPT.to_string()),
            },
            Message {
                role: Role::User,
                name: None,
                content: Content::Text(user_prompt),
            }
        ];

        let raw_reply = chat_inner_async_llama(messages, 300).await.expect(
            "llm generation failure"
        );

        let (terminate_or_not, key_points) = parse_result_and_key_points(
            &raw_reply.content_to_string()
        );

        let result_message = Message {
            name: None,
            content: Content::Text(key_points.clone()),
            role: Role::Assistant,
        };
        if terminate_or_not {
            let _ = save_message(conn, &self.name, result_message, "user_proxy").await;
        }

        (terminate_or_not, key_points)
    }

    pub fn execute_code_blocks(&self, code_blocks: &str) -> String {
        todo!()
        // match run_python_capture(code_blocks) {
        //     Ok(success_result_text) => res,
        //     Err(res) => res,
        // }
    }

    pub async fn start_coding(
        &self,
        user_message: &Message,
        conn: &Connection
    ) -> anyhow::Result<()> {
        let user_prompt = format!(
            "Here is the task for you: {:?}",
            user_message.content_to_string()
        );

        let messages = vec![
            Message {
                role: Role::System,
                name: None,
                content: Content::Text(CODE_PYTHON_SYSTEM_MESSAGE.to_string()),
            },
            Message {
                role: Role::User,
                name: None,
                content: Content::Text(user_prompt),
            }
        ];
        let mut message_vec = messages.clone();

        let output = chat_inner_async_llama(messages, 1000u16).await?;

        match &output.content {
            Content::Text(_out) => {
                let code = extract_code(_out);

                match run_python_capture(&code) {
                    Ok(success_result_text) => {
                        let (terminate_or_not, key_points) = self._is_termination(
                            &success_result_text,
                            &user_prompt,
                            conn
                        ).await;

                        let result_message = Message {
                            name: None,
                            content: Content::Text(key_points.clone()),
                            role: Role::Assistant,
                        };
                        if terminate_or_not {
                            let _ = save_message(
                                conn,
                                &self.name,
                                result_message.clone(),
                                "user_proxy"
                            ).await;
                        } else {
                            message_vec.push(result_message);
                            return Ok(message_vec);
                        }
                    }
                    Err(res) => {
                        let formatter = ITERATE_CODING_FAIL_TEMPLATE.lock().unwrap();

                        let wrapped_fail_result = formatter(res);
                        let result_message = Message {
                            name: None,
                            content: Content::Text(wrapped_fail_result),
                            role: Role::Assistant,
                        };

                        message_vec.push(result_message);

                        for i in 1..10 {
                            message_vec = self.iterate_coding(&message_vec.clone(), conn).await;
                        }
                    }
                }
            }
            Content::ToolCall(call) => {
                // let func = call.name;
                // let args = call.arguments.unwrap_or_default();
                // Execute the tool call function
                // func(args);
            }
        }

        Ok(())
    }

    pub async fn iterate_coding_error_case(
        &self,
        message_history: &Vec<Message>,
        error_msg: &str,
        conn: &Connection
    ) -> (bool, String, Vec<Message>) {
        let mut message_vec = message_history.clone();
        let formatter = ITERATE_CODING_FAIL_TEMPLATE.lock().unwrap();

        let wrapped_fail_result = formatter(&[error_msg]);
        let result_message = Message {
            name: None,
            content: Content::Text(wrapped_fail_result),
            role: Role::User,
        };

        message_vec.push(result_message);

        let output = chat_inner_async_llama(message_vec, 1000u16).await?;

        match &output.content {
            Content::Text(_out) => {
                let code = extract_code(_out);

                match run_python_capture(&code) {
                    Ok(success_result_text) => {
                        let result_message = Message {
                            name: None,
                            content: Content::Text(success_result_text.clone()),
                            role: Role::Assistant,
                        };
                        message_vec.push(result_message);
                        (true, success_result_text, message_vec)
                    }
                    Err(error_msg) => {
                        let result_message = Message {
                            name: None,
                            content: Content::Text(error_msg.clone()),
                            role: Role::Assistant,
                        };
                        message_vec.push(result_message);
                        (false, error_msg, message_vec)
                    }
                }
            }
            Content::ToolCall(call) => panic!(),
        }
    }
    pub async fn iterate_coding_success_case(
        &self,
        message_history: &Vec<Message>,
        success_result_text: &str,
        conn: &Connection
    ) -> (bool, String, Vec<Message>) {
        let mut message_vec = message_history.clone();
        let formatter = ITERATE_CODING_SUCCESS_TEMPLATE.lock().unwrap();

        let result_message = Message {
            name: None,
            content: Content::Text(formatter(&[success_result_text])),
            role: Role::User,
        };

        message_vec.push(result_message);

        let output = chat_inner_async_llama(message_vec, 1000u16).await.expect(
            "error LLM generation"
        );

        match &output.content {
            Content::Text(_out) => {
                let code = extract_code(_out);

                match run_python_capture(&code) {
                    Ok(success_result_text) => {
                        let result_message = Message {
                            name: None,
                            content: Content::Text(success_result_text.clone()),
                            role: Role::Assistant,
                        };
                        message_vec.push(result_message);
                        (true, success_result_text, message_vec)
                    }
                    Err(error_msg) => {
                        let result_message = Message {
                            name: None,
                            content: Content::Text(error_msg.clone()),
                            role: Role::Assistant,
                        };
                        message_vec.push(result_message);
                        (false, error_msg, message_vec)
                    }
                }
            }
            Content::ToolCall(call) => panic!(),
        }
    }
    pub async fn iterate_coding_error_case_copy(
        &self,
        message_history: &Vec<Message>,
        error_msg: &str,
        conn: &Connection
    ) -> (bool, String, Vec<Message>) {
        let mut message_vec = message_history.clone();
        let formatter = ITERATE_CODING_FAIL_TEMPLATE.lock().unwrap();

        let wrapped_fail_result = formatter(&[error_msg]);
        let result_message = Message {
            name: None,
            content: Content::Text(wrapped_fail_result),
            role: Role::User,
        };

        message_vec.push(result_message);

        let output = chat_inner_async_llama(message_vec, 1000u16).await?;

        match &output.content {
            Content::Text(_out) => {
                let code = extract_code(_out);

                match run_python_capture(&code) {
                    Ok(success_result_text) => {
                        let (terminate_or_not, key_points) = self._is_termination(
                            &success_result_text,
                            &instruction,
                            conn
                        ).await;

                        let result_message = Message {
                            name: None,
                            content: Content::Text(key_points.clone()),
                            role: Role::Assistant,
                        };
                        if terminate_or_not {
                            let _ = save_message(
                                conn,
                                &self.name,
                                result_message.clone(),
                                "user_proxy"
                            ).await;
                        } else {
                            message_vec.push(result_message);
                            return Ok(message_vec);
                        }
                    }
                    Err(error_msg) => {
                        let message_vec = self.iterate_coding_error_case(
                            message_history,
                            &error_msg,
                            conn
                        ).await;

                        for i in 1..10 {
                            message_vec = self.iterate_coding(&message_vec.clone(), conn).await;
                        }
                    }
                }
            }
            Content::ToolCall(call) => {
                // let func = call.name;
                // let args = call.arguments.unwrap_or_default();
                // Execute the tool call function
                // func(args);
            }
        }
        Ok(message_vec)
    }
    pub async fn iterate_coding_success(
        &self,
        message_history: &Vec<Message>,
        code: &str,
        success_run_result: &str,
        conn: &Connection
    ) -> anyhow::Result<Vec<Message>> {
        let mut message_vec = message_history.clone();
        let formatter = ITERATE_CODING_SUCCESS_TEMPLATE.lock().unwrap();

        let result_message = Message {
            name: None,
            content: Content::Text(formatter(&[code, success_run_result])),
            role: Role::User,
        };

        message_vec.push(result_message);

        let output = chat_inner_async_llama(message_vec, 1000u16).await?;

        match &output.content {
            Content::Text(_out) => {
                let code = extract_code(_out);

                match run_python_capture(code) {
                    Ok(success_result_text) => {
                        let (terminate_or_not, key_points) = self._is_termination(
                            res,
                            &user_prompt,
                            conn
                        ).await;

                        let result_message = Message {
                            name: None,
                            content: Content::Text(key_points.clone()),
                            role: Role::Assistant,
                        };
                        if terminate_or_not {
                            let _ = save_message(
                                conn,
                                &self.name,
                                result_message.clone(),
                                "user_proxy"
                            ).await;
                        } else {
                            message_vec.push(result_message);
                            return Ok(message_vec);
                        }
                    }
                    Err(res) => {
                        let message_vec = self.iterate_coding_error_case(
                            message_history,
                            error_msg,
                            conn
                        ).await;

                        for i in 1..10 {
                            message_vec = self.iterate_coding(&message_vec.clone(), conn).await;
                        }
                    }
                }
            }
            Content::ToolCall(call) => {
                // let func = call.name;
                // let args = call.arguments.unwrap_or_default();
                // Execute the tool call function
                // func(args);
            }
        }
        Ok(message_vec)
    }

    pub async fn extract_and_run_python_capture(
        &self,
        in_message: &Message
    ) -> anyhow::Result<String> {
        let raw = in_message.content_to_string();

        // let code = extract_code(&raw);

        // match run_python_capture_capture(&code) {
        //     Ok(success_result_text) => Ok(success_result_text),
        //     Err(e) => Err(anyhow::Error::new(e)),
        // }

        Ok(String::new())
    }

    pub async fn last_message(&self, conn: &Connection) -> Option<Message> {
        todo!()
        // let messages = self.message_history(conn).await.expect("failed to get message history");

        // messages.last().cloned()
    }
}

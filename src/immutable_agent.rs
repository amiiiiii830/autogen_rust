// use crate::exec_python::run_python_capture;
use crate::exec_python::*;
use crate::llama_structs::*;
use crate::llm_llama_local::*;
use crate::message_store::*;
use crate::{
    ROUTER_AGENT_SYSTEM_PROMPT,
    IS_TERMINATION_SYSTEM_PROMPT,
    CODE_PYTHON_SYSTEM_MESSAGE,
    ITERATE_CODING_FAIL_TEMPLATE,
    ITERATE_CODING_START_TEMPLATE,
    ITERATE_CODING_INCORRECT_TEMPLATE,
};
use anyhow;
use async_openai::types::Role;
use rusqlite::Connection;
use serde::{ Deserialize, Serialize };
use serde_json::{ Value };
use regex::Regex;

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
    pub fn new(content: Content, name: Option<String>, role: Role) -> Self {
        Message {
            content,
            name,
            role, // Set default role to Assistant if None is provided
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

    pub fn router_agent(llm_config: Option<Value>, tools_map_meta: &str) -> Self {
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
            Content::ToolCall(_call) => {
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

    pub async fn start_coding(
        &self,
        user_message: &Message,
        conn: &Connection
    ) -> anyhow::Result<()> {
        let formatter = ITERATE_CODING_START_TEMPLATE.lock().unwrap();
        let user_prompt = formatter(&[&user_message.content_to_string()]);

        let mut messages = vec![
            Message {
                role: Role::System,
                name: None,
                content: Content::Text(CODE_PYTHON_SYSTEM_MESSAGE.to_string()),
            },
            Message {
                role: Role::User,
                name: None,
                content: Content::Text(user_prompt.clone()),
            }
        ];

        for n in 1..9 {
            match chat_inner_async_llama(messages.clone(), 1000u16).await?.content {
                Content::Text(_out) => {
                    // let head: String = _out.chars().take(200).collect::<String>();
                    println!("Head {n}: {}\n", _out.clone());
                    let (this_round_good, code, exec_result) = run_python_wrapper(&_out).await;

                    if this_round_good {
                        let (terminate_or_not, key_points) = self._is_termination(
                            &exec_result,
                            &user_prompt,
                            conn
                        ).await;
                        if terminate_or_not {
                            println!("key_points:{:?}", key_points);
                            let result_message = Message {
                                name: None,
                                content: Content::Text(key_points),
                                role: Role::Assistant,
                            };

                            let _ = save_message(
                                conn,
                                "agent_name_holder",
                                result_message,
                                "user_proxy"
                            ).await;
                            break;
                        }
                    }

                    let formatter = if this_round_good {
                        ITERATE_CODING_INCORRECT_TEMPLATE.lock().unwrap()
                    } else {
                        ITERATE_CODING_FAIL_TEMPLATE.lock().unwrap()
                    };

                    let user_prompt = formatter(&[&code, &exec_result]);
                    let result_message = Message {
                        name: None,
                        content: Content::Text(user_prompt),
                        role: Role::User,
                    };

                    messages.push(result_message);
                }
                Content::ToolCall(_call) => {
                    // let func = call.name;
                    // let args = call.arguments.unwrap_or_default();
                    // Execute the tool call function
                    // func(args);
                }
            }
        }

        Ok(())
    }
}


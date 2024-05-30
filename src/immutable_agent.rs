use crate::exec_python::*;
use crate::llama_structs::*;
use crate::utils::*;
use crate::llm_llama_local::*;
use crate::message_store::*;
use crate::webscraper_hook::get_webpage_text;
use crate::webscraper_hook::search_bing;
use crate::{
    PLANNING_SYSTEM_PROMPT,
    IS_TERMINATION_SYSTEM_PROMPT,
    CODE_PYTHON_SYSTEM_MESSAGE,
    ITERATE_CODING_FAIL_TEMPLATE,
    ITERATE_CODING_START_TEMPLATE,
    ITERATE_CODING_INCORRECT_TEMPLATE,
    FURTER_TASK_BY_TOOLCALL_PROMPT,
};
use anyhow;
use async_openai::types::Role;
use rusqlite::Connection;
use serde::{ Deserialize, Serialize };
use serde_json::{ Value };

const INTERNAL_ROUTING_PROMPT: &'static str = r#"
You are a helpful AI assistant acting as a task dispatcher. Below are several paths that an agent can take and their abilities. Examine the task instruction and the current result, then decide whether the task is complete or needs further work. If further work is needed, dispatch the task to one of the agents. Please also extract key points from the current result. The descriptions of the agents are as follows:

1. **coding_python**: Specializes in generating clean, executable Python code for various tasks.
2. **user_proxy**: Represents the user by delegating tasks to agents, reviewing their outputs, and ensuring tasks meet user requirements; it is also responsible for receiving final task results.

Use this format to reply:
```json
{
    "continue_or_terminate": "TERMINATE" or "CONTINUE",
    "next_task_handler": "some_task_handler" (leave empty if "TERMINATE"),
    "key_points": ["point1", "point2", ...]
}
```
Dispatch to user_proxy when all tasks are complete.
"#;

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
    pub fn simple(name: &str, system_prompt: &str) -> Self {
        ImmutableAgent {
            name: name.to_string(),
            system_prompt: system_prompt.to_string(),
            llm_config: None,
            tools_map_meta: String::from(""),
            description: String::from(""),
        }
    }

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

    pub async fn send(&self, message_text: &str, conn: &Connection, next_step: &str) {
        let _ = save_message(conn, &self.name, message_text, next_step).await;

        if next_step == "user_proxy" {
            let inp = self.get_user_feedback().await;

            if inp == "stop" {
                // Exit on any non-empty input
                std::process::exit(0);
            } else {
                println!("{:?}", inp);
                // std::process::exit(0);
            }
        }
    }

    pub async fn get_user_feedback(&self) -> String {
        use std::io::{ self, Write };
        print!("User input:");

        io::stdout().flush().expect("Failed to flush stdout");

        let mut input = String::new();

        io::stdin().read_line(&mut input).expect("Failed to read line");

        if let Some('\n') = input.chars().next_back() {
            input.pop();
        }
        if let Some('\r') = input.chars().next_back() {
            input.pop();
        }

        return input;
    }

    pub async fn receive_message(&self, conn: &Connection) -> Option<String> {
        retrieve_most_recent_message(conn, &self.name).await
    }

    pub async fn furter_task_by_toolcall(&self, input: &str) -> Option<String> {
        let messages = vec![
            Message {
                role: Role::System,
                name: None,
                content: Content::Text(FURTER_TASK_BY_TOOLCALL_PROMPT.to_string()),
            },
            Message {
                role: Role::User,
                name: None,
                content: Content::Text(input.to_owned()),
            }
        ];

        let max_token = 1000u16;
        let output: LlamaResponseMessage = chat_inner_async_llama(
            messages.clone(),
            max_token
        ).await.expect("Failed to generate reply");

        match &output.content {
            Content::Text(_) => {
                todo!();
            }
            Content::ToolCall(call) => {
                let args = call.clone().arguments.unwrap_or_default();

                let res = match call.name.as_str() {
                    "get_webpage_text" => {
                        let url = args
                            .get("url")
                            .ok_or_else(|| anyhow::anyhow!("Missing 'url' argument"))
                            .ok()?
                            .to_string();

                        get_webpage_text(url).await.ok()?
                    }
                    "search_bing" => {
                        let query = args
                            .get("query")
                            .ok_or_else(|| anyhow::anyhow!("Missing 'query' argument"))
                            .ok()?
                            .to_string();
                        search_bing(&query).await.ok()?
                    }
                    "start_coding" => {
                        let key_points = args
                            .get("key_points")
                            .ok_or_else(|| anyhow::anyhow!("Missing 'key_points' argument"))
                            .ok()?
                            .to_string();
                        let _ = self.start_coding(&key_points).await;

                        String::from("code is being generated")
                    }
                    _ => {
                        return None;
                    }
                };
                Some(res)
            }
        }
    }
    pub async fn planning(&self, input: &str) -> Vec<String> {
        let messages = vec![
            Message {
                role: Role::System,
                name: None,
                content: Content::Text(PLANNING_SYSTEM_PROMPT.to_string()),
            },
            Message {
                role: Role::User,
                name: None,
                content: Content::Text(input.to_owned()),
            }
        ];

        let max_token = 500u16;
        let output: LlamaResponseMessage = chat_inner_async_llama(
            messages.clone(),
            max_token
        ).await.expect("Failed to generate reply");

        match &output.content {
            Content::Text(_out) => {
                println!("{:?}\n\n", _out.clone());
                parse_planning_steps(_out)
            }
            _ => unreachable!(),
        }
    }

    pub async fn run(&self, conn: &Connection, stop_toggle: bool) -> anyhow::Result<()> {
        match self.receive_message(conn).await {
            Some(message_text) => {
                println!("{} received: {}", self.name, message_text);
                let stop = self.a_generate_reply(&message_text).await?;
                if stop_toggle && stop {
                    std::process::exit(0);
                }
                Ok(())
            }
            None => Ok(()),
        }
    }

    pub async fn a_generate_reply(&self, content_text: &str) -> anyhow::Result<bool> {
        let user_prompt = format!("Here is the task for you: {:?}", content_text);

        let messages = vec![
            Message {
                role: Role::System,
                name: None,
                content: Content::Text(self.system_prompt.clone()),
            },
            Message {
                role: Role::User,
                name: None,
                content: Content::Text(user_prompt.clone()),
            }
        ];

        let max_token = 1000u16;
        let output: LlamaResponseMessage = chat_inner_async_llama(
            messages.clone(),
            max_token
        ).await.expect("Failed to generate reply");

        match &output.content {
            Content::Text(_out) => {
                let (terminate_or_not, next_step, key_points) = self.choose_next_step_and_(
                    &_out,
                    &user_prompt
                ).await;

                println!(
                    "terminate?: {:?}, speaker: {:?}, points: {:?}\n",
                    terminate_or_not.clone(),
                    next_step.clone(),
                    key_points.clone()
                );
                if terminate_or_not {
                    self.get_user_feedback().await;
                }
                return Ok(terminate_or_not);
            }
            _ => unreachable!(),
        }
    }

    pub async fn choose_next_step_and_(
        &self,
        current_text_result: &str,
        instruction: &str
    ) -> (bool, Option<String>, String) {
        let user_prompt = format!(
            "Given the task: {:?}, examine current result: {}, please decide whether the task is done or need further work",
            instruction,
            current_text_result
        );

        let messages = vec![
            Message {
                role: Role::System,
                name: None,
                content: Content::Text(INTERNAL_ROUTING_PROMPT.to_string()),
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
        println!("{:?}", raw_reply.content_to_string().clone());
        let (stop_here, speaker, key_points) = parse_next_move_and_(
            &raw_reply.content_to_string(),
            Some("next_task_handler")
        );

        // let _ = save_message(conn, &self.name, &key_points, &speaker).await;
        (stop_here, speaker, key_points.join(","))
    }

    pub async fn _is_termination(
        &self,
        current_text_result: &str,
        instruction: &str
    ) -> (bool, String) {
        let user_prompt = format!(
            "Given the task: {:?}, examine current result: {}, please decide whether the task is done or not",
            instruction,
            current_text_result
        );

        println!("{:?}", user_prompt.clone());
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

        println!("_is_termination raw_reply: {:?}", raw_reply.content_to_string());

        let (terminate_or_not, _, key_points) = parse_next_move_and_(
            &raw_reply.content_to_string(),
            None
        );

        (terminate_or_not, key_points.join(","))
    }

    pub async fn start_coding(&self, message_text: &str) -> anyhow::Result<()> {
        let formatter = ITERATE_CODING_START_TEMPLATE.lock().unwrap();
        let user_prompt = formatter(&[message_text]);

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
            println!("Iteration: {}", n);
            match chat_inner_async_llama(messages.clone(), 1000u16).await?.content {
                Content::Text(_out) => {
                    // let head: String = _out.chars().take(200).collect::<String>();
                    println!("Raw generation {n}:\n {}\n\n", _out.clone());
                    let (this_round_good, code, exec_result) = run_python_wrapper(&_out).await;
                    println!("code:\n{}\n\n", code.clone());
                    println!("Run result {n}: {}\n", exec_result.clone());

                    if this_round_good {
                        let (terminate_or_not, key_points) = self._is_termination(
                            &exec_result,
                            &user_prompt
                        ).await;
                        println!("Termination Check: {}\n", terminate_or_not);
                        if terminate_or_not {
                            println!("key_points:{:?}\n", key_points);

                            self.get_user_feedback().await;
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

                    if messages.len() > 5 {
                        messages = compress_chat_history(&messages.clone()).await;
                    }
                }
                _ => unreachable!(),
            }
        }
        Ok(())
    }
}

pub async fn compress_chat_history(message_history: &Vec<Message>) -> Vec<Message> {
    let message_history = message_history.clone();
    let (system_messages, messages) = message_history.split_at(2);
    let mut system_messages = system_messages.to_vec();

    let chat_history_text = messages
        .into_iter()
        .map(|m| m.content_to_string())
        .collect::<Vec<String>>()
        .join("\n");

    let messages = vec![
        Message {
            role: Role::System,
            name: None,
            content: Content::Text(FURTER_TASK_BY_TOOLCALL_PROMPT.to_string()),
        },
        Message {
            role: Role::User,
            name: None,
            content: Content::Text(chat_history_text),
        }
    ];

    let max_token = 1000u16;
    let output: LlamaResponseMessage = chat_inner_async_llama(
        messages.clone(),
        max_token
    ).await.expect("Failed to generate reply");

    match output.content {
        Content::Text(compressed) => {
            let message = Message {
                role: Role::User,
                name: None,
                content: Content::Text(compressed),
            };

            system_messages.push(message);
        }
        _ => unreachable!(),
    }

    system_messages
}

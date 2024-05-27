// use crate::exec_python::run_python;
use crate::exec_python::*;
use crate::llama_structs::*;
use crate::llm_llama_local::*;
use crate::message_store::*;
use crate::ROUTER_AGENT_SYSTEM_PROMPT;
use anyhow;
use async_openai::types::Role;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use regex::Regex;

pub fn parse_next_speaker(input: &str) -> (String, String) {
    let json_regex = Regex::new(r"\{[^}]*\}").unwrap();
    let json_str = json_regex
        .captures(input)
        .and_then(|cap| cap.get(0))
        .map_or(String::new(), |m| m.as_str().to_string());

    let continue_to_work_or_end_regex =
        Regex::new(r#""continue_to_work_or_end":\s*"([^"]*)""#).unwrap();
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

type Context = HashMap<String, String>;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Message {
    pub content: Option<Content>,
    pub name: Option<String>,
    pub role: Option<Role>,
}

impl Default for Message {
    fn default() -> Self {
        Message {
            content: None,
            name: None,
            role: None,
        }
    }
}

impl Message {
    pub fn new(content: Option<Content>, name: Option<String>, role: Option<Role>) -> Self {
        Message {
            content,
            name,
            role: role.or(Some(Role::Assistant)), // Set default role to Assistant if None is provided
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
        description: &str,
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

    pub fn _is_termination(&self, content: &str) -> bool {
        content.contains("TERMINATE")
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
        conn: &Connection,
    ) -> Option<Vec<Message>> {
let mut message_vec = messages.clone();
        let user_prompt = messages.clone().get(1).unwrap_or_default().content_to_string();
        let message =   messages.last().unwrap();
        let max_token = 1000u16;
        let output: LlamaResponseMessage = chat_inner_async_llama(messages, max_token)
            .await
            .expect("Failed to generate reply");

        match output.content {
            Content::Text(out) => {

let (terminate_or_not, next_speaker) = self.assign_next_speaker(message, &user_prompt, conn).await?;

let new_message = Message {
                    content: Some(Content::Text(out)),
                    role: Some(Role::Assistant),
                    name: None,

};
message_vec.push(new_message);

            }
            Content::ToolCall(call) => {
                let func = call.name;
                let args = call.arguments.unwrap_or_default();
                // Execute the tool call function
                func(args)
            }
        };

        Some(Message {
            content: Some(res),
            name: None,
            role: None,
        })
    }

    pub async fn assign_next_speaker(
        &self,
        message: &Message,
        instruction: &str,
        conn: &Connection,
    ) -> anyhow::Result<(bool, String)> {
        let user_prompt = match get_agent_names_and_abilities(conn).await {
            Err(_) => format!(
                "Given the task: {:?}, examine current result: {}, please decide whether the task is done or need further work",
              instruction,  message.content_to_string().unwrap(), 
            ),
            Ok(c) => format!(
                "Here are the list of agents and their abilities: {:?}, examine current result: {} against the task {:?}, please decide which is the next speaker to handle",
               c, instruction,message.content_to_string().unwrap() ),
        };

        let messages = vec![
            Message {
                role: Some(Role::System),
                name: None,
                content: Some(Content::Text(&self.system_prompt)),
            },
            Message {
                role: Some(Role::User),
                name: None,
                content: Some(Content::Text(user_prompt)),
            },
        ];

        let raw_reply = chat_inner_async_llama(messages, 100).await?;

let (terminate_or_not, next_speaker) = parse_next_speaker(&raw_reply.content_to_string());

match terminate_or_not == "TERMINATE" {
true => Ok((true, String::from(""))),
false => {
        save_message(conn, &self.name, message.clone(), &next_speaker).await;

    
    Ok((false, next_speaker.to_string()))},


    

    }
}

    pub fn execute_code_blocks(&self, code_blocks: &str) -> String {
        todo!()
        // match run_python(code_blocks) {
        //     Ok(res) => res,
        //     Err(res) => res,
        // }
    }

    pub async fn start_coding(
        &self,
        user_message: &Message,
        conn: &Connection,
    ) -> anyhow::Result<String> {
        let system_message = self.system_message(conn).await?;
        let user_prompt = format!(
            "Here is the task for you: {:?}",
            user_message.content_to_string()
        );

        let messages = vec![
            system_message,
            Message {
                role: Some(Role::User),
                name: None,
                content: Some(Content::Text(user_prompt)),
            },
        ];

        let code = chat_inner_async_llama(messages, 1000u16).await?;

        let content = match code.content {
            Content::Text(c) => c,
            Content::ToolCall(_) => panic!(),
        };

        Ok(content)
    }

    pub async fn iterate_coding(
        &self,
        message_history: &Vec<Message>,
        conn: &Connection,
    ) -> anyhow::Result<String> {
        // need to wrap the code and error msgs in template
        let user_prompt = format!(
            "Here is the task for you: {:?}",
            message_history.last().unwrap().content_to_string()
        );

        let messages = vec![Message {
            role: Some(Role::User),
            name: None,
            content: Some(Content::Text(user_prompt)),
        }];

        let code = chat_inner_async_llama(messages, 1000u16).await?;

        let content = match code.content {
            Content::Text(c) => c,
            Content::ToolCall(_) => panic!(),
        };

        Ok(content)
    }

    pub async fn extract_and_run_python(&self, in_message: &Message) -> anyhow::Result<String> {
        let raw = in_message
            .content_to_string()
            .expect("failed to convert message to String");

        let code = extract_code(&raw);

        match run_python_capture(&code) {
            Ok(res) => Ok(res),
            Err(e) => Err(anyhow::Error::new(e)),
        }
    }

    pub async fn last_message(&self, conn: &Connection) -> Option<Message> {
        let messages = self
            .message_history(conn)
            .await
            .expect("failed to get message history");

        messages.last().cloned()
    }
}

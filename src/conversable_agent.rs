// use crate::exec_python::run_python;
use crate::exec_python::*;
use crate::llama_structs::*;
use crate::llm_llama_local::*;
use crate::message_store::*;
// use crate::CODE_PYTHON_SYSTEM_MESSAGE;
use anyhow;
use async_openai::types::Role;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
type Context = HashMap<String, String>;

#[derive(Serialize, Deserialize, Debug, Clone)]
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



pub struct ConversableAgent {
    pub name: String,
    pub max_consecutive_auto_reply: u8,
    pub human_input_mode: String,
    pub llm_config: Option<Value>,
    pub default_auto_reply: Value,
    pub description: String,
}
impl Clone for ConversableAgent {
    fn clone(&self) -> Self {
        ConversableAgent {
            name: self.name.clone(),
            max_consecutive_auto_reply: self.max_consecutive_auto_reply,
            human_input_mode: self.human_input_mode.clone(),
            llm_config: self.llm_config.clone(),
            default_auto_reply: self.default_auto_reply.clone(),
            description: self.description.clone(),
        }
    }
}

impl ConversableAgent {
    pub fn default(name: &str) -> Self {
        ConversableAgent {
            name: name.to_string(),
            max_consecutive_auto_reply: 10u8,
            human_input_mode: String::from("ALWAYS"),
            llm_config: None,
            default_auto_reply: json!("this is user_proxy"),
            description: String::from("agent acting as user_proxy"),
        }
    }
    pub fn router_agent(&self, name: &str, conn: &Connection) -> Self {

        let agent = ConversableAgent {
            name: "router_agent".to_string(),
            max_consecutive_auto_reply: 1u8,
            human_input_mode: String::from("NEVER"),
            llm_config: None,
            default_auto_reply: json!("this is router_agent"),
            description: String::from("agent to select next speaker"),
        };
        register_agent(conn, agent_name, agent_description, &ROUTER_AGENT_SYSTEM_PROMPT, tools_map_meta).await;
        
    }
    pub fn new(
        name: &str,
        max_consecutive_auto_reply: u8,
        human_input_mode: &str,
        llm_config: Option<Value>,
        default_auto_reply: Value,
        description: &str,
    ) -> Self {
        ConversableAgent {
            name: name.to_string(),
            max_consecutive_auto_reply,
            human_input_mode: String::from(human_input_mode),
            llm_config,
            default_auto_reply,
            description: String::from(description),
        }
    }

    pub async fn system_message(&self, conn: &Connection) -> Result<Message, rusqlite::Error> {
        get_system_message_db(conn, &self.name).await
    }

    // pub async fn tool_calls_meta() -> String {}
    // pub async fn in_tool_call() -> bool {}
    pub async fn message_history(
        &self,
        conn: &Connection,
    ) -> Result<Vec<Message>, rusqlite::Error> {
        retrieve_messages(conn, &self.name).await
    }

    pub fn _is_termination(&self, content: &str) -> bool {
        content.contains("TERMINATE")
    }

    pub async fn send(&self, message: Message, conn: &Connection, next_speaker: Option<&str>) {
        let _ = save_message(conn, &self.name, message, next_speaker.unwrap()).await;
    }

    pub async fn receive_message(&self, conn: &Connection) -> Option<Message> {
        let next_speaker = get_next_speaker_db(conn).await.ok()?;
        match next_speaker == self.name {
            true => retrieve_most_recent_message(conn, &self.name).await.ok(),

            false => None,
        }
    }

    pub async fn a_generate_reply(
        &self,
        messages: Vec<Message>,
        conn: &Connection,
    ) -> Option<Message> {
        let max_token = 1000u16;
        let output: LlamaResponseMessage = chat_inner_async_llama(messages, max_token)
            .await
            .expect("Failed to generate reply");

        let res = match output.content {
            Content::Text(out) => {
                if self._is_termination(&out) {
                    todo!()
                } else {
                    let next_speaker = assign_next_speaker(&conn).await.ok()?;

                    if next_speaker == self.name || next_speaker.is_none() {
                    } else {
                        save_message(conn, agent_name, message, next_speaker).await;
                    }
                }
            }
            Content::ToolCall(call) => {
                let func = call.name;
                let args = call.arguments;

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
        &mut self,
        message: &Message,
        instruction: &str,
        conn: &Connection,
    ) -> anyhow::Result<()> {
        let speakers_and_abilities = get_agent_names_and_abilities(conn).await?;
        let system_message = self.system_message(conn).await?;
        let user_prompt = format!(
            "Here are the list of agents and their abilties: {:?}, please examine current result: {} against the instructed task {:?}, and decide which speaker to be the next",
           speakers_and_abilities, message.content_to_string().unwrap(),instruction, 
        );

        let messages = vec![
            system_message,
            Message {
                role: Some(Role::User),
                name: None,
                content: Some(Content::Text(user_prompt)),
            },
        ];

        let selected_speaker = chat_inner_async_llama(messages, 100).await?;

        save_message(conn, agent_name, message, selected_speaker).await;

        Ok(())
    }

    pub async fn update_system_prompt(
        &mut self,
        new_prompt: &str,
        conn: &Connection,
    ) -> anyhow::Result<()> {
        let _ = update_system_prompt_db(conn, &self.name, new_prompt).await;
        Ok(())
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
            user_message.content_to_string()
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
            Ok(res) => todo!(),

            Err(e) => todo!(),
        }
    }

    // pub async fn execute_tool_call(&self, call: &ToolCall) -> anyhow::Result<String, String> {
    //     let func = call.name.clone();
    //     let args = call.arguments.unwrap_or_default();

    //     let len = args.len();

    //     match len {
    //         0 => call_function!(&func),
    //         1 => call_function!(&func, args, single),
    //         _ => call_function!(&func, args, multi),
    //     }
    // }

    pub async fn last_message(&self, conn: &Connection) -> Option<Message> {
        let messages = self
            .message_history(conn)
            .await
            .expect("failed to get message history");

        match messages.last() {
            Some(msg) => Some(msg.clone()),
            None => None,
        }
    }
}

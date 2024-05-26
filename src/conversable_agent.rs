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

pub trait Agent {
    fn name(&self) -> String;

    fn description(&self) -> String;
}

pub struct ConversableAgent {
    pub name: String,
    pub max_consecutive_auto_reply: i32,
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
impl Agent for ConversableAgent {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn description(&self) -> String {
        self.description.clone()
    }
}

impl ConversableAgent {
    pub fn new(name: &str) -> Self {
        ConversableAgent {
            name: name.to_string(),
            max_consecutive_auto_reply: 10,
            human_input_mode: String::from("ALWAYS"),
            llm_config: None,
            default_auto_reply: json!("this is user_proxy"),
            description: String::from("agent acting as user_proxy"),
        }
    }

    pub async fn system_prompt(&self) -> Result<String, rusqlite::Error> {
        let conn = Connection::open_in_memory().expect("error openning sqlite connection");

        get_system_prompt_db(&conn, &self.name).await
    }

    // pub async fn tool_calls_meta() -> String {}
    // pub async fn in_tool_call() -> bool {}
    pub async fn message_history(&self) -> Result<Vec<Message>, rusqlite::Error> {
        let conn = Connection::open_in_memory().expect("error openning sqlite connectoins");

        retrieve_messages(&conn, &self.name).await
    }

    pub async fn send(
        &self,
        message: Message,
        message_store: Arc<Mutex<HashMap<String, VecDeque<Message>>>>,
        recipient: Arc<Mutex<ConversableAgent>>,
        request_reply: Option<bool>,
    ) {
        let agent_id = recipient.lock().unwrap().name.clone();
        let mut store = message_store.lock().unwrap();
        let queue = store.entry(agent_id).or_insert_with(VecDeque::new);
        queue.push_back(message);
    }

    pub async fn receive(
        &self,
        message_store: Arc<Mutex<HashMap<String, VecDeque<Message>>>>,
        sender: Arc<Mutex<ConversableAgent>>,
        request_reply: Option<bool>,
    ) -> Option<Message> {
        let agent_id = sender.lock().unwrap().name.clone();
        let store = message_store.lock().unwrap();
        store.get(&agent_id).and_then(|queue| queue.back().cloned())
    }

    pub async fn a_generate_reply(
        &self,
        messages: Vec<Message>,
        sender: Option<Arc<ConversableAgent>>,
    ) -> Option<Message> {
        let max_token = 1000u16;
        let output: LlamaResponseMessage = chat_inner_async_llama(messages, max_token)
            .await
            .expect("Failed to generate reply");

        Some(Message {
            content: Some(output.content),
            name: None,
            role: None,
        })
    }

    pub async fn update_system_prompt(&mut self, new_prompt: &str) -> anyhow::Result<()> {
        let conn = Connection::open_in_memory()?;

        let _ = update_system_prompt_db(&conn, &self.name, new_prompt).await;
        Ok(())
    }

    pub fn get_human_input(&self) -> String {
        self.human_input_mode.clone()
    }

    pub fn execute_code_blocks(&self, code_blocks: &str) -> String {
        todo!()
        // match run_python(code_blocks) {
        //     Ok(res) => res,
        //     Err(res) => res,
        // }
    }

    pub async fn start_coding(&self, user_message: &Message) -> anyhow::Result<String> {
        let conn = Connection::open_in_memory()?;

        let system_prompt = self.system_prompt().await?;
        let user_prompt = format!(
            "Here is the task for you: {:?}",
            user_message.content_to_string()
        );

        let messages = vec![
            Message {
                role: Some(Role::System),
                name: None,
                content: Some(Content::Text(system_prompt)),
            },
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

    pub async fn last_message(&self) -> Option<Message> {
        let messages = self
            .message_history()
            .await
            .expect("failed to get message history");

        match messages.last() {
            Some(msg) => Some(msg.clone()),
            None => None,
        }
    }
}

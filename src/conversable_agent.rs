// use crate::exec_python::run_python;
use crate::llama_structs::*;
use crate::llm_llama_local::chat_inner_async_llama;
use anyhow::anyhow;
use async_openai::types::Role;
use regex::Regex;
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
    pub context: Option<Context>,
}

impl Default for Message {
    fn default() -> Self {
        Message {
            content: None,
            name: None,
            role: None,
            context: None,
        }
    }
}

impl Message {
    pub fn new(
        content: Option<Content>,
        name: Option<String>,
        role: Option<Role>,
        context: Option<Context>,
    ) -> Self {
        Message {
            content,
            name,
            role: role.or(Some(Role::Assistant)), // Set default role to Assistant if None is provided
            context,
        }
    }
}

pub trait Agent {
    fn name(&self) -> String;

    fn description(&self) -> String;

    fn system_message(&self) -> String;

    fn set_description(&mut self, description: String);
}

pub struct ConversableAgent {
    pub name: String,
    pub system_message: String,
    pub max_consecutive_auto_reply: i32,
    pub human_input_mode: String,
    pub tool_calls_meta: String,
    pub in_tool_call: bool,
    pub llm_config: Option<Value>,
    pub default_auto_reply: Value,
    pub description: String,
    pub chat_messages: Option<Vec<Message>>,
}
impl Clone for ConversableAgent {
    fn clone(&self) -> Self {
        ConversableAgent {
            name: self.name.clone(),
            system_message: self.system_message.clone(),
            max_consecutive_auto_reply: self.max_consecutive_auto_reply,
            human_input_mode: self.human_input_mode.clone(),
            tool_calls_meta: self.tool_calls_meta.clone(),
            in_tool_call: self.in_tool_call.clone(),
            llm_config: self.llm_config.clone(),
            default_auto_reply: self.default_auto_reply.clone(),
            description: self.description.clone(),
            chat_messages: self.chat_messages.clone(),
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

    fn system_message(&self) -> String {
        self.system_message.clone()
    }

    fn set_description(&mut self, description: String) {
        self.description = description;
    }
}

impl ConversableAgent {
    pub fn new(name: &str) -> Self {
        ConversableAgent {
            name: name.to_string(),
            system_message: String::from("you act as user proxy"),
            max_consecutive_auto_reply: 10,
            human_input_mode: String::from("ALWAYS"),
            tool_calls_meta: String::from("fake functions"),
            in_tool_call: false,
            llm_config: None,
            default_auto_reply: json!("this is user_proxy"),
            description: String::from("agent acting as user_proxy"),
            chat_messages: Some(vec![]),
        }
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
            context: None,
        })
    }

    pub async fn update_system_message(&mut self, system_message: String) {
        self.system_message = system_message.to_string();
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

    pub fn set_description(&mut self, description: String) {
        self.description = description;
    }

    pub fn last_message(&self) -> Option<Message> {
        match &self.chat_messages {
            Some(messages) => messages.last().cloned(),
            None => None,
        }
    }
}

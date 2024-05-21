use crate::exec_python::run_python;
use crate::llama_structs::*;
use crate::llm_llama_local::chat_inner_async_llama;
use async_openai::types::Role;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

pub struct ToolsOutputs {
    pub tool_call_id: Option<String>,
    pub output: Option<String>,
}

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

#[async_trait]
pub trait Agent: Send + Sync {
    fn name(&self) -> String;

    fn description(&self) -> String;

    async fn send(
        &self,
        message: Message,
        recipient: Arc<dyn Agent + Send>,
        request_reply: Option<bool>,
    );

    async fn a_generate_reply(
        &self,
        messages: Vec<Message>,
        sender: Option<Arc<dyn Agent + Send>>,
    ) -> Option<Message>;

    fn system_message(&self) -> String;
    fn chat_messages(&self) -> Option<Vec<Message>>;
    async fn update_system_message(&mut self, system_message: String);
}

pub struct ConversableAgent {
    pub name: String,
    pub system_message: String,
    pub max_consecutive_auto_reply: i32,
    pub human_input_mode: String,
    pub function_map: String,
    pub llm_config: Option<Value>,
    pub default_auto_reply: Value,
    pub description: String,
    pub chat_messages: Option<Vec<Message>>,
}

#[async_trait]
impl Agent for ConversableAgent {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn description(&self) -> String {
        self.description.clone()
    }

    async fn send(
        &self,
        message: Message,
        recipient: Arc<dyn Agent + Send>,
        request_reply: Option<bool>,
    ) {
        recipient.chat_messages().expect("REASON").push(message);
    }

    async fn a_generate_reply(
        &self,
        messages: Vec<Message>,
        sender: Option<Arc<dyn Agent + Send>>,
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

    fn system_message(&self) -> String {
        self.system_message.clone()
    }

    fn chat_messages(&self) -> Option<Vec<Message>> {
        self.chat_messages
    }
    async fn update_system_message(&mut self, system_message: String) {
        self.system_message = system_message.to_string();
    }
}

impl ConversableAgent {
    pub fn get_human_input(&self) -> String {
        self.human_input_mode.clone()
    }

    pub fn execute_code_blocks(&self, code_blocks: &str) -> String {
        match run_python(code_blocks) {
            Ok(res) => res,
            Err(res) => res,
        }
    }

    pub fn set_description(&mut self, description: String) {
        self.description = description;
    }

    pub fn chat_messages(&self) -> Option<Vec<Message>> {
        self.chat_messages
    }

    pub fn last_message(&self) -> Option<Message> {
        match self.chat_messages {
            Some(messages) => messages.last().cloned(),
            None => None,
        }
    }
}

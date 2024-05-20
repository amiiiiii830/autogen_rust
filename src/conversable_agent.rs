/* use crate::llama_structs::*;
use async_openai::types::{CompletionUsage, CreateChatCompletionResponse, Role};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::default;
use std::sync::Arc;

pub struct ToolsOutputs {
    pub tool_call_id: Option<String>,
    pub output: Option<String>,
}

// Define a struct for the context, which is a map with String keys and String values.
type Context = HashMap<String, String>;

// Define the main Message struct with the possible fields as described.
#[derive(Serialize, Deserialize, Debug)]
struct Message {
    content: Option<Content>,
    name: Option<String>,
    role: Option<Role>,
    context: Option<Context>,
}

impl Message {
    // Define a constructor or a method to create a new `Message` instance if needed.
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
pub trait Agent {
    /// Returns the name of the agent.
    fn name(&self) -> &str;

    /// Returns the description of the agent.
    fn description(&self) -> &str;

    /// Sends a message to another agent.
    async fn send(&self, message: Message, recipient: &dyn Agent, request_reply: Option<bool>);

    /// Receives a message from another agent.
    async fn receive(&self, message: Message, sender: &dyn Agent, request_reply: Option<bool>);

    /// Asynchronously generates a reply based on the received messages.
    async fn a_generate_reply(
        &self,
        messages: Option<Vec<Message>>,
        sender: Option<&dyn Agent>,
    ) -> Option<Message>;

    fn system_message(&self) -> &str;

    async fn update_system_message(&mut self, system_message: &str);
}

pub struct ConversableAgent<Agent: ?Sized> {
    name: String,
    system_message: String,
    max_consecutive_auto_reply: i32,
    human_input_mode: String,
    function_map: HashMap<String, Arc<dyn Fn(&[Value]) -> Value + Send + Sync>>,
    // code_execution_config: Value,
    llm_config: Option<Value>,
    default_auto_reply: Value,
    description: String,
    chat_messages: Option<Vec<Message>>,
    // Other fields as needed
}

impl Agent for ConversableAgent<Agent> {
    fn name(&self) -> &str {
        self.name.clone().as_str()
    }

    fn description(&self) -> &str {
        self.description.clone().as_str()
    }

    async fn send(&self, message: Message, recipient: &dyn Agent, request_reply: Option<bool>) {
        recipient.chat_messages.push(message);
    }

    /// Receives a message from another agent.
    async fn receive(&self, message: Message, sender: &dyn Agent, request_reply: Option<bool>);

    /// Asynchronously generates a reply based on the received messages.
    async fn a_generate_reply(
        &self,
        messages: Option<Vec<Message>>,
        sender: Option<&dyn Agent>,
    ) -> Option<Message>;

    fn system_message(&self) -> &str;

    async fn update_system_message(&mut self, system_message: &str);
}

impl ConversableAgent<Agent> {
    // Additional methods specific to ConversableAgent
    fn get_human_input(&self) -> String;
    fn execute_code_blocks(&self, code_blocks: &str) -> String;
    fn run_code(&self, code: &str) -> String;
    fn execute_function(&self, function_name: &str, args: &[Value]) -> String;
    fn convert(&self) -> String;
    fn set_description(&mut self, description: String);

    // Getter for code_executor
    // fn code_executor(&self) -> Option<&YourCodeExecutorType>;

    // Method to register a reply function
    fn register_reply(
        &mut self,
        trigger: Trigger,
        reply_func: Arc<dyn Fn(&Self, &Value, &Self) -> String + Send + Sync>,
        position: usize,
        // ... other parameters
    );

    // Method to replace a reply function
    fn replace_reply_func(
        &mut self,
        old_reply_func: Arc<dyn Fn(&Self, &Value, &Self) -> String + Send + Sync>,
        new_reply_func: Arc<dyn Fn(&Self, &Value, &Self) -> String + Send + Sync>,
    );

    fn is_termination_msg(&self, message: &Value) -> bool {
        // Call the closure that determines if the message is a termination message
        (self.is_termination_msg_fn)(message)
    }

    // Method to update max_consecutive_auto_reply
    fn update_max_consecutive_auto_reply(&mut self, value: i32, sender: Option<&Self>);

    // Method to get max_consecutive_auto_reply
    fn max_consecutive_auto_reply(&self, sender: Option<&Self>) -> i32;

    // Method to get chat_messages
    fn chat_messages(&self) -> HashMap<AgentId, Vec<Value>>;

    // Method to get chat_messages for summary
    fn chat_messages_for_summary(&self, agent: AgentId) -> Vec<Value>;

    // Method to get last_message
    fn last_message(&self, agent: Option<AgentId>) -> Option<Value>;

    // Getter for use_docker
    fn use_docker(&self) -> Option<bool>;
}
 */

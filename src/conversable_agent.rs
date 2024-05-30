use crate::llama_structs::*;
use crate::llm_llama_local::*;
use crate::utils::*;
use crate::message_store::*;
use crate::{
    ROUTING_SYSTEM_PROMPT,
    PLANNING_SYSTEM_PROMPT,
    IS_TERMINATION_SYSTEM_PROMPT,
    USER_PROXY_SYSTEM_PROMPT,
};
use anyhow;
use async_openai::types::Role;
use rusqlite::Connection;
use serde::{ Deserialize, Serialize };
use serde_json::{ Value };
use regex::Regex;

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

/* pub struct ImmutableAgent {
    pub name: String,
    pub system_prompt: String,
    pub llm_config: Option<Value>,
    pub tools_map_meta: String,
    pub description: String,
} */

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
        register_agent(
            conn,
            agent_name,
            agent_description,
            &ROUTER_AGENT_SYSTEM_PROMPT,
            tools_map_meta
        ).await;
    }
    pub fn new(
        name: &str,
        max_consecutive_auto_reply: u8,
        human_input_mode: &str,
        llm_config: Option<Value>,
        default_auto_reply: Value,
        description: &str
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
        conn: &Connection
    ) -> Result<Vec<Message>, rusqlite::Error> {
        retrieve_messages(conn, &self.name).await
    }

    pub async fn update_system_prompt(
        &mut self,
        new_prompt: &str,
        conn: &Connection
    ) -> anyhow::Result<()> {
        let _ = update_system_prompt_db(conn, &self.name, new_prompt).await;
        Ok(())
    }

    pub async fn send(&self, message_text: &str, conn: &Connection, next_speaker: &str) {
        let _ = save_message(conn, &self.name, message_text, next_speaker).await;

        if next_speaker == "user_proxy" {
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

    pub async fn run(&self, conn: &Connection, stop_toggle: bool) -> anyhow::Result<()> {
        match self.receive_message(conn).await {
            Some(message_text) => {
                println!("{} received: {}", self.name, message_text);
                let stop = self.a_generate_reply(&message_text, conn).await?;
                if stop_toggle && stop {
                    std::process::exit(0);
                }
                Ok(())
            }
            None => Ok(()),
        }
    }

    pub async fn a_generate_reply(
        &self,
        content_text: &str,
        conn: &Connection
    ) -> anyhow::Result<bool> {
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
                let (terminate_or_not, next_speaker, key_points) =
                    self.assign_next_speaker_and_send(&_out, &user_prompt, conn).await;

                println!(
                    "terminate?: {:?}, speaker: {:?}, points: {:?}\n",
                    terminate_or_not.clone(),
                    next_speaker.clone(),
                    key_points.clone()
                );
                return Ok(terminate_or_not);
            }
            _ => unreachable!(),
        }
        Ok(false)
    }

    pub async fn assign_next_speaker_and_send(
        &self,
        current_text_result: &str,
        instruction: &str,
        conn: &Connection
    ) -> (bool, String, String) {
        let user_prompt = format!(
            "Given the task: {:?}, examine current result: {}, please decide whether the task is done or need further work",
            instruction,
            current_text_result
        );

        let messages = vec![
            Message {
                role: Role::System,
                name: None,
                content: Content::Text(ROUTING_SYSTEM_PROMPT.to_string()),
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
        let (stop_here, speaker, key_points) = parse_next_speaker_and_key_points(
            &raw_reply.content_to_string()
        );

        let _ = save_message(conn, &self.name, &key_points, &speaker).await;
        (stop_here, speaker, key_points)
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

        let (terminate_or_not, key_points) = parse_next_move_and_(
            &raw_reply.content_to_string()
        );

        if terminate_or_not {
            let _ = save_message(conn, &self.name, &key_points, "user_proxy").await;
        }

        (terminate_or_not, key_points)
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



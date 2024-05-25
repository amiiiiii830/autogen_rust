use crate::conversable_agent::*;
use crate::llama_structs::*;
use async_openai::types::Role;
use rusqlite::{params, Connection, Result};


trait RoleToString {
    fn to_string(&self) -> String;
}

impl RoleToString for Role {
    fn to_string(&self) -> String {
        match self {
            Role::Assistant => String::from("assistant"),
            Role::System => String::from("system"),
            Role::User => String::from("user"),
            _ => String::from("assistant"),
        }
    }
}

trait RoleFromStr {
    fn from_str(s: &str) -> Role;
}

impl RoleFromStr for Role {
    fn from_str(s: &str) -> Role {
        match s {
            "assistant" => Role::Assistant,
            "system" => Role::System,
            "user" => Role::User,
            _ => Role::User, // Default case
        }
    }
}

pub struct GroupChat {
    pub agent_name: String,
    pub message_content: String,
    pub message_role: String,
    pub tokens_count: u16,
    pub next_speaker: String,
}

pub struct NaiveMessage {
    pub content: String,
    pub role: String,
}

impl From<NaiveMessage> for Message {
    fn from(naive: NaiveMessage) -> Self {
        let content = if naive.content.starts_with("toolcall:") {
            // Assuming a specific format for ToolCall
            let tool_name = naive
                .content
                .strip_prefix("toolcall:")
                .unwrap_or("")
                .to_string();
            Some(Content::ToolCall(ToolCall {
                name: tool_name,
                arguments: None,
            }))
        } else {
            Some(Content::Text(naive.content))
        };

        let role = match naive.role.as_str() {
            "system" => Some(Role::System),
            "user" => Some(Role::User),
            "assistant" => Some(Role::Assistant),
            _ => Some(Role::Assistant),
        };

        Message {
            content,
            role,
            name: None,
        }
    }
}

impl From<Message> for NaiveMessage {
    fn from(message: Message) -> Self {
        let content = match message.content {
            Some(Content::Text(text)) => text,
            Some(Content::ToolCall(tool_call)) => format!("toolcall:{}", tool_call.name),
            None => String::new(),
        };

        let role = match message.role {
            Some(Role::System) => "system".to_string(),
            Some(Role::User) => "user".to_string(),
            Some(Role::Assistant) => "assistant".to_string(),
            _ => "user".to_string(),
        };

        NaiveMessage { content, role }
    }
}

pub fn save_message(
    conn: &Connection,
    agent_name: String,
    message: Message,
    next_speaker: String,
) -> Result<()> {
    let tokens_count = message
        .content_to_string()
        .map_or(0, |s| s.split_whitespace().count() as i32);

    let naive_message = NaiveMessage::from(message);
    conn.execute(
        "INSERT INTO GroupChat (agent_name, message_content, message_role, tokens_count, next_speaker) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![agent_name, naive_message.content, naive_message.role, tokens_count, next_speaker],
    )?;
    Ok(())
}

pub fn retrieve_messages(conn: &Connection, agent_name: String) -> Result<Vec<Message>> {
    let mut stmt = conn.prepare("SELECT message_content, message_role, message_context FROM GroupChat WHERE agent_name = ?1")?;
    let rows = stmt.query_map(params![agent_name], |row| {
        Ok(Message {
            content: Some(Content::Text(row.get::<_, String>(0)?)), // Specify type as String
            role: Some(Role::from_str(&row.get::<_, String>(1)?)), // Specify type as String and use from_str
            name: Some(agent_name.clone()),
        })
    })?;

    let mut messages = Vec::new();
    for message_result in rows {
        messages.push(message_result?);
    }
    Ok(messages)
}

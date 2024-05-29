use crate::immutable_agent::*;
use crate::llama_structs::*;
use async_openai::types::Role;
use rusqlite::{ params, Connection, Result };

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

pub async fn create_message_store_table(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS GroupChat (
            id INTEGER PRIMARY KEY,
            agent_name TEXT NOT NULL,
            message_content TEXT,
            tokens_count INTEGER,
            next_speaker TEXT
        )",
        []
    )?;
    Ok(())
}

pub async fn retrieve_most_recent_message(conn: &Connection, agent_name: &str) -> Option<String> {
    let mut stmt = conn
        .prepare("SELECT message_content, next_speaker FROM GroupChat ORDER BY id DESC LIMIT 1")
        .ok()?;

    let result = stmt
        .query_row(params![], |row| {
            let content: String = row.get(0)?;
            let speaker: String = row.get(1)?;

            if speaker == agent_name {
                Ok(Some(content))
            } else {
                Ok(None)
            }
        })
        .ok()?;

    result
}

pub async fn save_message(
    conn: &Connection,
    agent_name: &str,
    message_text: &str,
    next_speaker: &str
) -> Result<()> {
    let tokens_count = message_text.split_whitespace().count();

    conn.execute(
        "INSERT INTO GroupChat (agent_name, message_content, tokens_count, next_speaker) VALUES (?1, ?2, ?3, ?4)",
        params![agent_name, message_text, tokens_count, next_speaker]
    )?;
    Ok(())
}

pub async fn get_next_speaker_db(conn: &Connection) -> Result<String> {
    let mut stmt = conn.prepare("SELECT next_speaker FROM GroupChat")?;
    let mut rows = stmt.query([])?;

    if let Some(row) = rows.next()? {
        let next_speaker: String = row.get(0)?;
        Ok(next_speaker)
    } else {
        Err(rusqlite::Error::QueryReturnedNoRows.into())
    }
}

pub struct AgentStore {
    pub agent_name: String,
    pub agent_description: String,
    pub current_system_prompt: String,
    pub tools_map_meta: String,
    pub recent_instruction: String,
}

pub async fn create_agent_store_table(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS AgentStore (
            agent_name TEXT PRIMARY KEY,
            current_system_prompt TEXT NOT NULL,
            recent_instruction TEXT,
            tools_map_meta TEXT
        )",
        []
    )?;
    Ok(())
}

pub async fn register_agent(
    conn: &Connection,
    agent_name: &str,
    agent_description: &str,
    system_prompt: &str,
    tools_map_meta: &str
) -> Result<()> {
    conn.execute(
        "INSERT INTO AgentStore (agent_name, agent_description, current_system_prompt, tools_map_meta) VALUES (?1, ?2, ?3, ?4)",
        params![agent_name, agent_description, system_prompt, tools_map_meta]
    )?;
    Ok(())
}

pub async fn get_agent_names_and_abilities(conn: &Connection) -> Result<String> {
    let mut stmt = conn.prepare("SELECT agent_name, agent_description FROM AgentStore")?;
    let rows = stmt.query_map([], |row| {
        Ok(
            format!(
                "agent_name: {:?}, abilities: {:?}",
                &row.get::<_, String>(0)?,
                &row.get::<_, String>(1)?
            )
        ) // Specify type as String
    })?;

    let mut agent_names = String::new();
    for agent_name_result in rows {
        agent_names.push_str(&agent_name_result?);
    }
    Ok(agent_names)
}

pub async fn update_system_prompt_db(
    conn: &Connection,
    agent_name: &str,
    new_prompt: &str
) -> Result<()> {
    conn.execute(
        "UPDATE AgentStore SET current_system_prompt = ?1 WHERE agent_name = ?2",
        params![new_prompt, agent_name]
    )?;
    Ok(())
}

pub async fn get_system_prompt_db(conn: &Connection, agent_name: &str) -> Result<String> {
    let mut stmt = conn.prepare(
        "SELECT current_system_prompt FROM AgentStore WHERE agent_name = ?1"
    )?;
    let mut rows = stmt.query(params![agent_name])?;

    if let Some(row) = rows.next()? {
        let prompt: String = row.get(0)?;
        Ok(prompt)
    } else {
        Err(rusqlite::Error::QueryReturnedNoRows)
    }
}

pub async fn get_tools_meta_db(conn: &Connection, agent_name: &str) -> Result<String> {
    let mut stmt = conn.prepare("SELECT tools_map_meta FROM AgentStore WHERE agent_name = ?1")?;
    let mut rows = stmt.query(params![agent_name])?;

    if let Some(row) = rows.next()? {
        let prompt: String = row.get(0)?;
        Ok(prompt)
    } else {
        Err(rusqlite::Error::QueryReturnedNoRows)
    }
}

pub async fn get_system_message_db(conn: &Connection, agent_name: &str) -> Result<Message> {
    let mut stmt = conn.prepare(
        "SELECT current_system_prompt FROM AgentStore WHERE agent_name = ?1"
    )?;
    let mut rows = stmt.query(params![agent_name])?;

    if let Some(row) = rows.next()? {
        let prompt: String = row.get(0)?;
        Ok(Message {
            content: Content::Text(prompt),
            name: None,
            role: Role::System,
        })
    } else {
        Err(rusqlite::Error::QueryReturnedNoRows)
    }
}

pub async fn save_recent_instruction(
    conn: &Connection,
    agent_name: &str,
    recent_instruction: &str
) -> Result<()> {
    conn.execute(
        "UPDATE AgentStore SET recent_instruction = ?1 WHERE agent_name = ?2",
        params![recent_instruction, agent_name]
    )?;
    Ok(())
}

pub async fn recent_instruction_db(conn: &Connection, agent_name: &str) -> Result<String> {
    let mut stmt = conn.prepare("SELECT recent_instruction FROM AgentStore WHERE agent_name = ?1")?;
    let mut rows = stmt.query(params![agent_name])?;

    if let Some(row) = rows.next()? {
        let instruction: String = row.get(0)?;
        Ok(instruction)
    } else {
        Err(rusqlite::Error::QueryReturnedNoRows)
    }
}

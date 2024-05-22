use crate::conversable_agent::*;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

pub struct GroupChat {
    pub agents: HashMap<String, Arc<ConversableAgent>>,
    pub messages_store: Arc<Mutex<HashMap<String, VecDeque<Message>>>>,
    pub next_speaker: Option<String>,
}

impl GroupChat {
    pub fn new() -> Self {
        GroupChat {
            agents: HashMap::new(),
            messages_store: Arc::new(Mutex::new(HashMap::new())),
            next_speaker: None,
        }
    }

    pub fn register(&mut self, agent: &ConversableAgent) {
        let agent_arc = Arc::new(agent.clone());
        self.agents.insert(agent.name.clone(), agent_arc);
    }
}

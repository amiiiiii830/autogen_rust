pub struct TaskLedger {
    pub current_task: usize,
    pub task_list: Vec<String>,
    pub result_list: Vec<String>,
    pub parent_task: Option<String>,
    pub task_done: bool,
}

impl TaskLedger {
    pub fn new(task_list: Vec<String>, parent_task: Option<String>) -> Self {
        TaskLedger {
            current_task: 0,
            task_list,
            result_list: Vec::new(),
            parent_task,
            task_done: false,
        }
    }

    pub fn record_result(&mut self, task_result: String) {
        self.result_list.push(task_result);
        if self.current_task < self.task_list.len() - 1 {
            self.current_task += 1;
        } else {
            self.task_done = true;
        }
    }

    pub fn current_task(&self) -> Option<String> {
        let idx = self.current_task;
        match self.task_list.get(idx) {
            Some(ct) => Some(ct.to_string()),
            None => None,
        }
    }
}

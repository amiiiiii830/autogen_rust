use crate::exec_python::*;
use crate::llama_structs::*;
use crate::llm_llama_local::*;
use crate::utils::*;
use crate::webscraper_hook::{ get_webpage_text, search_with_bing };
use crate::{
    CODE_PYTHON_SYSTEM_MESSAGE,
    IS_TERMINATION_SYSTEM_PROMPT,
    ITERATE_CODING_FAIL_TEMPLATE,
    ITERATE_CODING_INCORRECT_TEMPLATE,
    ITERATE_CODING_START_TEMPLATE,
};
use anyhow;
use async_openai::types::Role;
use chrono::Utc;
use serde::{ Deserialize, Serialize };
use serde_json::Value;

use once_cell::sync::Lazy;

pub static GROUNDING_CHECK_TEMPLATE: Lazy<String> = Lazy::new(|| {
    let today = Utc::now().format("%Y-%m-%dT").to_string();
    format!(r#"
<|im_start|>system You are an AI assistant. Your task is to determine whether a question requires grounding in real-world date, time, location, or physics.

When given a task, please follow these steps to think it through and then act:

Identify Temporal Relevance: Determine if the question requires current or time-sensitive information. Note that today's date is {}.
Check for Location Specificity: Identify if the question is location-specific.
Determine Real-time Data Dependency: Assess if the answer depends on real-time data or specific locations.
Suggest Grounding Information: If grounding is needed, suggest using today's date to cross-validate the reply. Otherwise, suggest reliable sources to obtain the necessary grounding data.

Remember that you are a dispatcher; you DO NOT work on tasks yourself. Your role is to direct the process.

In your reply, list out your think-aloud steps clearly:

Example 1:

When tasked with "What is the weather like in New York?" reshape your answer as follows:

{{
    \"my_thought_process\": [
        \"my_goal: goal is to identify whether grounding is needed for this task\",
        \"Determine if the question requires current or time-sensitive information: YES\",
        \"Provide assistance to agent for this task grounding information: today's date is xxxx-xx-xx\",
        \"Echo original input verbatim in key_points section\"
    ],
    \"key_points\": [\"today's date is xxxx-xx-xx, What is the weather like in New York\"]
}}

Example 2:

When tasked with \"Who killed John Lennon?\" reshape your answer as follows:

{{
    \"my_thought_process\": [
        \"my_goal: goal is to identify whether grounding is needed for this task\",
        \"Determine if the question requires current or time-sensitive information: NO\",
        \"Echo original input verbatim in key_points section\"
    ],
    \"key_points\": [\"Who killed John Lennon?\"]
}}

Use this format for your response:

```json
{{
    \"my_thought_process\": [
        \"my_goal: goal is to identify whether grounding is needed for this task\",
        \"thought_process_one: my judgement at this step\",
        \"...\",
        \"thought_process_N: my judgement at this step\"
    ],
    \"grounded_or_not\": \"YES\" or \"NO\",
    \"key_points\": [\"point1\", \"point2\", ...]
}}
"#, today)
});

const NEXT_STEP_PLANNING: &'static str =
    r#"
You are a helpful AI assistant with extensive capabilities. Your goal is to help complete tasks and create plausible answers with minimal steps.

You have three built-in tools to solve problems:

use_intrinsic_knowledge: You can answer many questions and provide a wealth of knowledge from within yourself. This should be your first approach to problem-solving.
search_with_bing: Performs an internet search using Bing and returns relevant results based on a query. Use it to get information you don't have.
code_with_python: Generates and executes Python code for various tasks based on user input. It can handle mathematical computations, data analysis, large datasets, complex operations through optimized algorithms, providing precise, deterministic outputs.

When given a task, follow these steps:

Determine whether the task/question can be answered with intrinsic knowledge.
Determine whether the task can be completed with the other two built-in tools (search_with_bing or code_with_python).
If yes in above two cases, consider this as one-step completion which should be placed in the "steps_to_take" section.

DO NOT try to answer it yourself.

Pass the task to the next agent by using the original input text verbatim as one single step in the "steps_to_take" section.
If neither intrinsic knowledge nor built-in tools suffice:

Strategize and outline necessary steps to achieve the final goal.
Preferably, each step corresponds to a task that can be completed with one of three approaches: intrinsic knowledge, creating Python code, or searching with Bing.

When listing steps:

Think about why you outlined such a step.
When cascading down to coding tasks:
Constrain them ideally into one coding task. Add "Work out the following task/tasks as one coding problem: " before your reply in the "key_points" section.  
Fill out the "steps_to_take" section of your reply template accordingly.

In your reply list out your think-aloud steps clearly:

Example 1:

When tasked with "calculate prime numbers up to 100" 

Here's how you would structure your response:

{
    "my_goal": "goal is to help complete this mathematical computation efficiently",
    "my_thought_process": [
        "Determine if this task/question can be answered with intrinsic knowledge: NO",
        "Determine if this task can be done with coding or search: YES",
        "Determine if this task can be done in single step: NO",
        "[Define function checking if number is prime]",
        "[Loop through numbers 2-100 calling function]",
        "[Print each number if it's prime]",
        "...",
        "[Check for unnecessary breakdowns especially for 'coding' tasks]: merge into single coding action"
    ],
    "steps_to_take": ["Work out the following task/tasks as one coding problem: Define function checking primes; loop through 2-100 calling function; print primes"]
}

Example 2:

When tasked with "Find the current population of Tokyo"

Here's how you would structure your response:

{
    "my_goal": "goal is to find the current population of Tokyo accurately",
    "my_thought_process": [
        "Determine if this task/question can be answered with intrinsic knowledge: NO",
        "Determine if this task can be done with coding or search: YES",
        "Determine if this task can be done in single step: YES"
    ],
    "steps_to_take": ["Find the current population of Tokyo"]
}

Example 3:

When tasked with "find out when Steve Jobs died" 

Here's how you would structure your response:

{
    "my_goal": "goal is finding Steve Jobs' date of death accurately",
     "my_thought_process": [
        "Determine if this task/question can be answered with intrinsic knowledge: YES",
      ],
      "steps_to_take": ["find out when Steve Jobs died"]
}

Example 4:

When tasked with "how would you describe Confucius"

Here's how you would structure your response:

{
     "my_goal": "goal is providing accurate description of Confucius",
     "my_thought_process": [
        "Determine if this task/question can be answered with intrinsic knowledge: YES",
      ],
      "steps_to_take": ["how would you describe Confucius"]
}

Use this format for your response:
```json
{
    "my_thought_process": [
        "thought_process_one: my judgement at this step",
        "...",
        "though_process_N: : my judgement at this step"
    ],
    "steps_to_take": ["Step description", "..."] 
}
```
"#;

/* 2. **do_grounding_check**: 
Description: Provides current date or suggests ways to get grounding information in real-world locality, history of events or physics.
Current Date: 2024-06-02
Example Call:
<tool_call>
{"arguments": {"task": "It's now 2024-06-02, find out how old would Steve Jobs be if he didn't die"}, 
"name": "do_grounding_check"}
</tool_call>


do_grounding_check
Description: Provides current date or suggests ways to get grounding information in real-world locality or physics (current date is 2024-06-02).
Parameters: "task" The task you receive (type:string)
Required Parameters: ["task"] */

const NEXT_STEP_BY_TOOLCALL: &'static str =
    r#"
<|im_start|>system You are a function-calling AI model. You are provided with function signatures within <tools></tools> XML tags. You may call one function to assist with the user query. Do not make assumptions about what values to plug into functions.

<tools>
1. **use_intrinsic_knowledge**: 
Description: Solves tasks using capabilities and knowledge obtained at training time, but it is frozen by the cut-off date and unaware of real-world dates post-training.
Example Call:
<tool_call>
{"arguments": {"task": "tell a joke"}, 
"name": "use_intrinsic_knowledge"}
</tool_call>

2. **search_with_bing**: 
Description: Conducts an internet search using Bing search engine and returns relevant results based on the query provided by the user.
Special Note 1: This function helps narrow down potential sources of information before extracting specific content.
Special Note 2: Using this as an initial step can make subsequent tasks more targeted by providing exact links that can then be scraped using get_webpage_text.
Example Call:
<tool_call>
{"arguments": {"query": "latest AI research trends"}, 
"name": "search_with_bing"}
</tool_call>

3. **code_with_python**: 
Description: Generates clean, executable Python code for various tasks based on user input.
Special Note: When task requires precise mathematical operations; processing, analyzing and creating complex data types, where AI models can not efficiently represent and manipulate in natural language terms, this is the way out.
Example Call:
<tool_call>
{"arguments": {"key_points": "Create a Python script that reads a CSV file and plots a graph"}, 
"name": "code_with_python"}
</tool_call>

4. **get_webpage_text**: 
Description: Fetches textual content from the specified webpage URL.
Special Note 1: This function retrieves raw text from a given URL without filtering out irrelevant content; use precise URLs for best results.
Special Note 2: It is often more effective to first use search_with_bing to find precise URLs before scraping them for targeted information.
Example Call:
<tool_call>
{"arguments":{"url":"https://example.com"},
"name":"get_webpage_text"}
</tool_call>
</tools>

Function Definitions

use_intrinsic_knowledge
Description: Solves tasks using built-in capabilities obtained at training time (frozen post cut-off date).
Parameters: "task" The task you receive (type:string)
Required Parameters: ["task"]

search_with_bing
Description: Conducts an internet search using Bing search engine and returns relevant results based on the query provided by the user.
Parameters: "query" The search query to be executed on Bing (type:string)
Required Parameters: ["query"]

code_with_python
Description: Generates clean executable Python code for various tasks based on key points describing what needs to be solved with code.
Parameters: "key_points" Key points describing what kind of problem needs to be solved with Python code (type:string)
Required Parameters: ["key_points"]

get_webpage_text
Description: Retrieves all textual content froma specified website URL.It does not parse or structure data in any specific format; hence, it may include extraneous information such as navigation menus advertisements, and other non-essential text elements present onthe page.
Parameters: "url" The URL of the website from which to fetch textual content (type:string)
Required Parameters: ["url"]

Remember that you are a dispatcher; you DO NOT work on tasks yourself, especially when you see specific coding suggestions, don't write any code, just dispatch.

For each function call, return a JSON object with function name and arguments within <tool_call></tool_call> XML tags as follows:

<tool_call>  
{"arguments": <args-dict>,   
"name": "<function_name>"}
</tool_call>
"#;

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

pub struct ImmutableAgent {
    pub name: String,
    pub system_prompt: String,
    pub llm_config: Option<Value>,
    pub tools_map_meta: String,
    pub description: String,
}

impl ImmutableAgent {
    pub fn simple(name: &str, system_prompt: &str) -> Self {
        ImmutableAgent {
            name: name.to_string(),
            system_prompt: system_prompt.to_string(),
            llm_config: None,
            tools_map_meta: String::from(""),
            description: String::from(""),
        }
    }

    pub fn new(
        name: &str,
        system_prompt: &str,
        llm_config: Option<Value>,
        tools_map_meta: &str,
        description: &str
    ) -> Self {
        ImmutableAgent {
            name: name.to_string(),
            system_prompt: system_prompt.to_string(),
            llm_config,
            tools_map_meta: tools_map_meta.to_string(),
            description: description.to_string(),
        }
    }

    pub async fn get_user_feedback(&self) -> String {
        use std::io::{ self, Write };
        print!("User input: ");

        io::stdout().flush().expect("Failed to flush stdout");

        let mut input = String::new();

        io::stdin().read_line(&mut input).expect("Failed to read line");

        if let Some('\n') = input.chars().next_back() {
            input.pop();
        }
        if let Some('\r') = input.chars().next_back() {
            input.pop();
        }

        if input == "stop" {
            std::process::exit(0);
        }
        return input;
    }

    pub async fn next_step_by_toolcall(&self, input: &str) -> anyhow::Result<String> {
        let messages = vec![
            Message {
                role: Role::System,
                name: None,
                content: Content::Text(NEXT_STEP_BY_TOOLCALL.to_string()),
            },
            Message {
                role: Role::User,
                name: None,
                content: Content::Text(input.to_owned()),
            }
        ];

        let max_token = 1000u16;
        let output: LlamaResponseMessage = chat_inner_async_llama(
            messages.clone(),
            max_token
        ).await.expect("Failed to generate reply");
        match &output.content {
            Content::Text(unexpected_result) => {
                return Ok(
                    format!("attempt to run tool_call failed, returning text result: {} ", unexpected_result)
                );
            }
            Content::ToolCall(call) => {
                let args = call.clone().arguments.unwrap_or_default();
                let res = match call.name.as_str() {
                    "use_intrinsic_knowledge" =>
                        match args.get("task") {
                            Some(t) => self.planning(&t).await.join(", "),
                            None => String::from("failed in use_intrinsic_knowledge"),
                        }
                    "get_webpage_text" =>
                        match args.get("url") {
                            Some(u) =>
                                get_webpage_text(u.to_string()).await.unwrap_or(
                                    "get_webpage_text failed".to_string()
                                ),
                            None => String::from("failed in get_webpage_text"),
                        }
                    "search_with_bing" =>
                        match args.get("query") {
                            Some(q) =>
                                search_with_bing(&q).await.unwrap_or(
                                    "search_with_bing failed".to_string()
                                ),
                            None => String::from("failed in search_with_bing"),
                        }
                    "code_with_python" =>
                        match args.get("key_points") {
                            Some(k) => {
                                let _ = self.code_with_python(&k).await;
                                "code_with_python working".to_string()
                            }
                            None => String::from("failed in code_with_python"),
                        }
                    _ => {
                        panic!();
                    }
                };
                Ok(res)
            }
        }
    }
    pub async fn planning(&self, input: &str) -> Vec<String> {
        let messages = vec![
            Message {
                role: Role::System,
                name: None,
                content: Content::Text(NEXT_STEP_PLANNING.to_string()),
            },
            Message {
                role: Role::User,
                name: None,
                content: Content::Text(input.to_owned()),
            }
        ];

        let max_token = 500u16;
        let output: LlamaResponseMessage = chat_inner_async_llama(
            messages.clone(),
            max_token
        ).await.expect("Failed to generate reply");

        match &output.content {
            Content::Text(_out) => {
                let res = parse_planning_steps(_out);
                println!("steps_to_take: {:?}\n", res);
                let mut res = res;
                res.reverse();
                res
            }
            _ => unreachable!(),
        }
    }

    pub async fn stepper(&self, task_vec: &Vec<String>) -> anyhow::Result<String> {
        let mut task_vec = task_vec.clone();
        let mut initial_input = match task_vec.pop() {
            Some(s) => s,
            None => {
                return Err(anyhow::Error::msg("no task to handle"));
            }
        };
        let mut res;
        loop {
            res = self
                .next_step_by_toolcall(&initial_input).await
                .unwrap_or("next_step_by_toolcall failed".to_string());
            initial_input = match task_vec.pop() {
                Some(s) =>
                    format!(
                        "Here is the result from previous step: {}, here is the next task: {}",
                        res,
                        s
                    ),
                None => {
                    break;
                }
            };
        }
        Ok(res)
    }

    pub async fn simple_reply(&self, input: &str) -> anyhow::Result<bool> {
        let user_prompt = format!("Here is the task for you: {:?}", input);

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
                let (terminate_or_not, key_points) = self._is_termination(
                    &_out,
                    &user_prompt
                ).await;

                println!(
                    "terminate?: {:?}, points: {:?}\n",
                    terminate_or_not.clone(),
                    key_points.clone()
                );
                if terminate_or_not {
                    self.get_user_feedback().await;
                }
                return Ok(terminate_or_not);
            }
            _ => unreachable!(),
        }
    }

    pub async fn _is_termination(
        &self,
        current_text_result: &str,
        instruction: &str
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

        let (terminate_or_not, _, key_points) = parse_next_move_and_(
            &raw_reply.content_to_string(),
            None
        );

        (terminate_or_not, key_points.join(","))
    }

    pub async fn code_with_python(&self, message_text: &str) -> anyhow::Result<()> {
        let formatter = ITERATE_CODING_START_TEMPLATE.lock().unwrap();
        let user_prompt = formatter(&[message_text]);

        let mut messages = vec![
            Message {
                role: Role::System,
                name: None,
                content: Content::Text(CODE_PYTHON_SYSTEM_MESSAGE.to_string()),
            },
            Message {
                role: Role::User,
                name: None,
                content: Content::Text(user_prompt.clone()),
            }
        ];
        for n in 1..9 {
            println!("Iteration: {}", n);
            match chat_inner_async_llama(messages.clone(), 1000u16).await?.content {
                Content::Text(_out) => {
                    // let head: String = _out.chars().take(200).collect::<String>();
                    println!("Raw generation {n}:\n {}\n\n", _out.clone());
                    let (this_round_good, code, exec_result) = run_python_wrapper(&_out).await;
                    println!("code:\n{}\n\n", code.clone());
                    println!("Run result {n}: {}\n", exec_result.clone());

                    if this_round_good {
                        let (terminate_or_not, key_points) = self._is_termination(
                            &exec_result,
                            &user_prompt
                        ).await;
                        println!("Termination Check: {}\n", terminate_or_not);
                        if terminate_or_not {
                            println!("key_points: {:?}\n", key_points);

                            self.get_user_feedback().await;
                        }
                    }

                    let formatter = if this_round_good {
                        ITERATE_CODING_INCORRECT_TEMPLATE.lock().unwrap()
                    } else {
                        ITERATE_CODING_FAIL_TEMPLATE.lock().unwrap()
                    };

                    let user_prompt = formatter(&[&code, &exec_result]);
                    let result_message = Message {
                        name: None,
                        content: Content::Text(user_prompt),
                        role: Role::User,
                    };

                    messages.push(result_message);

                    if messages.len() > 5 {
                        messages = compress_chat_history(&messages.clone()).await;
                    }
                }
                _ => unreachable!(),
            }
        }
        Ok(())
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
            content: Content::Text(NEXT_STEP_BY_TOOLCALL.to_string()),
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

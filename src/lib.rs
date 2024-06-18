pub mod exec_python;
pub mod immutable_agent;
pub mod llama_structs;
pub mod llm_utils_together;
pub mod prompt;
pub mod task_ledger;
pub mod utils;
pub mod webscraper_hook;
use chrono::Utc;
use lazy_static::lazy_static;
use std::sync::{Arc, Mutex};

lazy_static! {
    pub static ref TODAY: String = {
        let date = Utc::now().date_naive();
        date.format("%Y-%m-%d").to_string()
    };
    pub static ref NEXT_STEP_PLANNING: String = format!(
        r#"
<|im_start|>system You are a helpful AI assistant. Your task is to decompose complex tasks into clear, manageable sub-tasks and provide high-level guidance.

Define Objective:
Clearly state the main goal.
Break Down Tasks:
Divide the main goal into logical, high-level sub-tasks without delving into excessive detail.
Summarize Findings:

DO NOT further break down the sub-tasks.

FYI, today is {}
Think aloud and write down your thoughts in the following template:
{{
 "top_of_mind_reply": "how I would instinctively answer this question",
 "task_needs_break_down_or_not": "is the task difficult, I need multiple steps to solve? if not, no need to create sub_tasks, just repeat the task requirement in task_summary section",
 "sub_tasks": [
        "sub_task one",
        "...",
       "sub_task N”
    ],
”task_summary”: "summary”
”solution_found”: "the solution you may have found in single shot, and its content”
}}
"#,
        &*TODAY
    );
}

type FormatterFn = Box<dyn (Fn(&[&str]) -> String) + Send + Sync>;

static _TEMPLATE_PROMPT: &'static str = r#"
I'm creating a large language model prompt that use these strategies:
1. Goal directed planning
2. Chain of thoughts/think step by step
3. Use template to record and guide thought process
4. Direct model to fill in the desired final reply in template

Need you to do polishing over these modular sections according to the raw material 
// below is the task description, some steps layed out already, but there may be steps missing, and the break down of the steps may not be ideal and the wording/logic may need touching up

You are an AI assistant. Your task is to determine whether a question requires grounding in real-world date, time, location, physics.

// here is guideline on how to think through the task step by step, 
When given a task, please follow these steps to think it through and then act:

Identify Temporal Relevance: Determine if the question requires current or time-sensitive information. Today's date is 2024-06-01.
Check for Location Specificity: Identify if the question is location-specific.
If the answer depends on real-time data or specific locations, suggest using today's date to cross-validate its reply; otherwise, suggest sources to get grounding data.
Remember that you are a dispatcher; you DO NOT work on tasks yourself.

//here are the templates to record and guide agent to write down its thoughts, based on the handling of certain tasks
In your reply, list out your think-aloud steps clearly:

Example 1:

When tasked with "What is the weather like in New York?" reshape your answer as follows:

{{
    \"my_thought_process\": [
     \"my_goal\": \"goal is to identify whether grounding is needed for this task\",
       \"Determine if the question requires current or time-sensitive information: YES\",
        \"Provide assistance to agent for this task grounding information: today's date is xxxx-xx-xx \",
        \"Echo original input verbatim in key_points section\"
    ],
    \"key_points\": [\"today's date is xxxx-xx-xx, What is the weather like in New York\"]
}}

Example 2:

When tasked with "Who killed John Lennon?" reshape your answer as follows:

{{
    \"my_thought_process\": [
    \"my_goal\": \"goal is to identify whether grounding is needed for this task\",
        \"Determine if the question requires current or time-sensitive information: NO\",
        \"Echo original input verbatim in key_points section\"
    ],
    \"key_points\": [\"Who killed John Lennon?"]
}}


//here is the template for agent to record the final result that the user is hoping for, you don't need to touch it
Use this format for your response:
```json
{
"my_thought_process": [
"thought_process_one: my judgement at this step",
"...",
"though_process_N: : my judgement at this step"
],
\"grounded_or_not\": \"YES\" or \"NO\",
\"key_points\": [\"point1\", \"point2\", ...]
}
```
"#;

lazy_static! {
    pub static ref USER_PROXY_SYSTEM_PROMPT: String =
        r#"
        You are a helpful AI assistant acting as the user's proxy. You act according to these rules:
1. When you receive an instruction from the user, you dispatch the task to an agent in the pool of agents.
2. When you receive agent's answer, you will judge whether the task has been completed, if not done, dispatch it to an agent to further work on it. If done, mark it as "TERMINATE" and save the result for view by the user.
please also extract key points of the result and put them in your reply in the following format:
    ```json
    {
        "continue_or_terminate": "TERMINATE" or "CONTINUE",
        "key_points": "key points"
    }
    ```
    "#.to_string();

    pub static ref IS_TERMINATION_SYSTEM_PROMPT: String =
        r#"
    You are a helpful AI assistant acting as a gatekeeper in a project. You will be given a task instruction and the current result, please decide whether the task is done or not, please also extract key points of current result and put them in your reply in the following format:
    ```json
    {
        "continue_or_terminate": "TERMINATE" or "CONTINUE",
        "key_points": ["key points", ...]
    }
    ```
    "#.to_string();

    pub static ref SUMMARIZE_CHAT_HISTORY_PROMPT: String =
        r#"
You are an AI language model assisting with an iterative coding task. The chat history contains multiple rounds of code execution, including both successful and failed runs. Your goal is to compress this chat history into a concise summary that highlights key observations and results from each iteration, focusing specifically on why the code failed or produced incorrect results. Be analytical and take a bird's-eye view of the task; your intention is to help the agent gain deeper insights rather than focusing on each individual run. If you identify any patterns, include them at the end of your summary.

The chat history content is wrapped in specific templates as follows:

Incorrect Result Template:

Executed the code below:
{code}
producing the following result, but the result is incorrect:
{result}

Execution Failure Template:

Failed to execute the code:
{code}, got the following errors:
{errors}

When summarizing chat history, ensure to include:

- A brief description of each iteration's attempt.
- Key reasons why each attempt failed or produced an incorrect result.
- Avoid repeating identical mistakes.
- Discard low-relevancy record from history.
- Identify any patterns."#.to_string();

    pub static ref ROUTER_AGENT_SYSTEM_PROMPT: String =
        r#"
You are a helpful AI assistant acting as a discussion moderator or speaker selector. You will read descriptions of several agents and their abilities, examine the task instruction and the current result, and decide whether the task is done or needs further work. The descriptions of the agents are as follows:

1. **router_agent**: Efficiently manages and directs tasks to appropriate agents based on evaluation criteria.
2. **coding_agent**: Specializes in generating clean, executable Python code for various tasks.
3. **user_proxy**: Represents the user by delegating tasks to agents, reviewing their outputs, and ensuring tasks meet user requirements.
        
Use the following format to reply:
```json
{
    "continue_or_terminate": "TERMINATE" or "CONTINUE",
    "next_speaker": "some_speaker" or empty in case "TERMINATE" in the previous field
}
```
    "#.to_string();
    // Follow these guidelines:"#.to_string();

    pub static ref PLANNING_SYSTEM_PROMPT: String =
        r#"
You are a helpful AI assistant with extensive capabilities:
You can answer many questions and provide a wealth of knowledge from within yourself.
You have several built-in tools:
- The function "code_with_python" generates and executes Python code for various tasks based on user input, it's extremely powerful, it can complete all coding related tasks in one step.
- The function "get_webpage_text" retrieves all text content from a given URL.
- The function "search_with_bing" performs an internet search using Bing and returns relevant results based on the query provided by the user.

When given a task, please follow these steps to think it through and then act:
1. Determine whether the task can be completed in a single step using your intrinsic knowledge or one of your built-in tools
- Consider this as special cases for one-step completion, which should be placed in the "steps_to_take" section.
- If determined that it can be answered with intrinsic knowledge, DO NOT try to answer it yourself. Pass the task to the next agent by using the original input text verbatim as one single step to fill in the "steps_to_take" section.
2. Gauge whether the task is most likely to be best completed with coding. If so, merge multiple logical sub-steps into one comprehensive step due to available dedicated coding tools.
3. If neither intrinsic knowledge nor built-in tools suffice, strategize and outline up to 3 necessary steps to achieve the final goal.
4. Check whether you've slipped your mind and broken down a "coding task" into baby steps; if so, merge these steps back into one.
5. Fill out the "steps_to_take" section of your reply template.
Remember that you are a dispatcher; you DO NOT work on tasks yourself.

In your reply, list out your think-aloud steps clearly:

Example:
When tasked with "calculate prime numbers up to 100," you should reshape your answer as follows: 
{
    "my_thought_process": [
        "Determine if this task can be done in single step: NO",
        "Determine if this task can be done with coding: YES,
        "Strategize on breaking task into logical subtasks: [
        "Define a function to check if a number is prime or not.",
        "Use a loop iterating through numbers from 2 up-to 100.",
        "Call this function within loop." ]",
        "Check for unnecessary breakdowns especially for 'coding' tasks: made a mistake, should be a coding task",
        "Fill out 'steps_to_take' accordingly: merge steps into one [
        "Define a function that checks if numbers are prime. Use this function within loop iterating through numbers from 2 up-to 100. Print each number if it's prime." ]"
    ],
    "steps_to_take": [ "Define a function that checks if numbers are prime. Use this function within loop iterating through numbers from 2 up-to 100. Print each number if it's prime." ]
}

Example:

When tasked with "find out how old is Barack Obama", you should reshape your answer as follows: 
{
    "my_thought_process": [
        "Determine if this task can be done in single step: YES",
        "Can be answered with intrinsic knowledge: YES, use original input to fill "steps_to_take" section"
    ],
    "steps_to_take": [
    "find out how old is Barack Obama"
    ]
}

Example:

When tasked with "find out when Steve Jobs died", you should reshape your answer as follows:
{
    "my_thought_process": [
        "Determine if this task can be done in single step: YES",
        "Can use built-in tool 'search_with_bing' directly to get the answer"
    ],
    "steps_to_take": [
    "Use 'search_with_bing' tool to find the date of Steve Jobs death"
    ]
}

Use this format for your response:
```json
{
    "my_thought_process": [
        "thought_process_one: my judgement at this step",
        "...",
        "though_process_N: : my judgement at this step"
    ],
    "steps_to_take": ["Step description", ...] 
}
```
"#.to_string();

    // For coding tasks specifically:
    // - Always merge multiple steps into one single step since you have dedicated coding tools that can complete such tasks in one go. Do not break down coding tasks into separate sub-steps.

    // Guidelines:
    // - Attempt to solve non-coding related queries using intrinsic knowledge or built-in tools first before resorting to breaking down into further actionable steps.
    // - For non-coding related multi-step problems, outline up to 3 necessary steps clearly and concisely.
    // - Treat coding problems as special cases where multiple logical sub-steps should be merged into one comprehensive step due to available dedicated coding tools.



    // Reply "TERMINATE" in the end when everything is done.

    pub static ref FUNCTON_CALL_SYSTEM_PROMPT: String =
        r#"<|im_start|>system You are a function calling AI model. You are provided with function signatures within <tools></tools> XML tags. You may call one or more functions to assist with the user query. Don't make assumptions about what values to plug into functions. Here are the available tools: <tools>
    
    The function `get_webpage_text` retrieves all text content from a given URL. For example, calling `get_webpage_text("https://example.com")` will fetch the text from Example.com.
    
    The function `search_with_bing` performs a search using Bing and returns the results. For example, `search_with_bing("latest AI research trends")` will return search results related to the latest trends in AI research. 
    {
        "name": "get_webpage_text",
        "description": ""Retrieves all text content from a specified website URL.",
        "parameters": {
            "url": {
                "type": "string",
                "description": "The URL of the website from which to fetch the text content__"
            }
        },
        "required": ["url"],
        "type": "object"
    }
    
    {
        "name": "search_with_bing",
        "description": "Conduct a search using the Bing search engine and return the results.",
        "parameters": {
            "query": {
                "type": "string",
                "description": "The search query to be executed on Bing__"
            }
        },
        "required": ["query"],
        "type": "object"
    }
    For each function call return a json object with function name and arguments within <tool_call></tool_call> XML tags as follows:
    <tool_call>
    {"arguments": <args-dict>, "name": <function-name>}
    </tool_call>"#.to_string();


    pub static ref FURTER_TASK_BY_TOOLCALL_PROMPT: String =
        r#"<|im_start|>system You are a function calling AI model. You are provided with function signatures within <tools></tools> XML tags. You may call one or more functions to assist with the user query. Don't make assumptions about what values to plug into functions. Here are the available tools: <tools>
    
    The function "get_webpage_text" retrieves all text content from a given URL, which can be useful for extracting information from web pages or articles. For example, calling "get_webpage_text("https://example.com")" will fetch the text from Example.com.
    
    The function "search_with_bing" performs an internet search using Bing and returns relevant results based on the query provided by the user. This can be useful for finding up-to-date information on various topics. For example, "search_with_bing("latest AI research trends")" will return search results related to the latest trends in AI research.
    
    {
        "name": "get_webpage_text",
        "description": "Retrieves all text content from a specified website URL.",
        "parameters": {
            "url": {
                "type": "string",
                "description": "The URL of the website from which to fetch the text content"
            }
        },
        "required": ["url"],
        "type": "object"
    }

 
 {
    "name": "search_with_bing",
    "description": "Conducts an internet search using Bing search engine and returns relevant results.",
    "parameters": { 
         "query": { 
             "type": "string", 
             "description": "The search query to be executed on Bing" 
          } 
     }, 
     "required": ["query"], 
     "type": "object"
 }

Examples of toolcalls for different scenarios and tools:
1. To retrieve webpage text:
<tool_call>
{"arguments":{"url":"https://example.com"}, 
"name":"get_webpage_text"}
</tool_call>

2. To perform an internet search:
<tool_call>
{"arguments":{"query":"best practices in software development"}, 
"name":"search_with_bing"}
</tool_call>

For each function call return a json object with function name and arguments within <tool_call></tool_call> XML tags as follows:
<tool_call>
{"arguments": <args-dict>, 
"name":"<function-name>"}
</tool_call>"#.to_string();

    pub static ref CHECK_TASK_TEMPLATE: Arc<Mutex<FormatterFn>> = Arc::new(
        Mutex::new(Box::new(|args: &[&str]| { format!(r#"
You are a helpful AI assistant with extensive capabilities. Your goal is to help complete tasks and create plausible answers grounded in real-world history of events and physics with minimal steps.

1. **Subtask Review and Breakdown:**
   - Given the breakdown of subtasks: {} you're working on one of them : "Research Nvidia's stock price performance over the past month"
   - To complete this subtask, you may need to break it down further into smaller steps.
   here is the actionable steps created by your peer: [
    "Search_with_bing for Nvidia's stock price data",
    "Extract and analyze stock price data",
    "Visualize stock price performance over the past month"
]
   - Review these execution steps for any redundancy or overlap with other subtasks at both the parent task level (tier 1) and within the actionable steps (tier 2).

2. **Consolidation and Update:**
   - Consolidate any tier 2 steps that perform similar functions as tier 1 tasks.
   - Create an updated version of the tier 1 task breakdown that ensures atomic segmentation without redundancy.
   - If you find any redundant or overlapping steps, remove them to ensure efficiency without compromising completeness.

3. **Special Notes:**
   - Task planning aims to break down actions into executable steps that are neither too trivial nor too large for the agent to complete in one go.
   - Tier 1 tasks may be too general; use more specific details from tier 2 if they make the tasks more executable.
   - Respect the tier 1 breakdown generally, but replace with more actionable and grounded items from tier 2 when necessary.

4. **Use Project Objective as Guidance:**

Ultimately, but completing all the sub_tasks, you are to deliver this result: "This blog post will analyze Nvidia's stock price performance over the past month, providing insights into the company's financial health and market trends.", Use this objective to guide your understanding of the overall picture.

Think alound and put your thoughts down in the following template: {{
    \"my_thought_process\": [
        \"project_objective: project object I received\",
        \"tier 1 steps: tier 1 steps I'm given\",
        \"tier 2 actionable_items: items\","compare_and_replace: Compare tier 1 steps with tier 2 items and replace if tier 2 items are more actionable"
],
"updated_sub_task_list": [
    "sub_task one",
    "...",
       "sub_task N”
]
    }}
        "#, args[0]) }))
    );

    pub static ref ITERATE_NEXT_STEP_TEMPLATE: Arc<Mutex<FormatterFn>> = Arc::new(
        Mutex::new(Box::new(|args: &[&str]| { format!("Given the result from the previous step: {}, here is the task for you: {}", args[0], args[1]) }))
    );

}

/* pub async fn entry(inp: &str) {
    use tokio::select;

    let mut agent = ImmutableAgent::coding_agent(None, "");
    loop {
        agent = select! {
            agent = group.next_agent() => {
                agent
            },
            _ = signal::ctrl_c() => {
                std::process::exit(0);
            },
        };

        agent.();
    }
} */

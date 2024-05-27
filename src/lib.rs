// pub mod conversable_agent;
// pub mod groupchat;
pub mod immutable_agent;
pub mod exec_python;
// pub mod groupchat;
pub mod llama_structs;
pub mod llm_llama_local;
pub mod message_store;
pub mod webscraper_hook;
// pub mod tool_call_actuators;
use lazy_static::lazy_static;
use std::sync::{Arc, Mutex};

type FormatterFn = Box<dyn Fn(&[&str]) -> String + Send + Sync>;

lazy_static! {
    pub static ref ROUTER_AGENT_SYSTEM_PROMPT: String = r#"
    You are a helpful AI assistant acting as a discussion moderator or speaker selector. You will read descriptions of several agents and their abilities, examine the task instruction and the current result, and decide whether the task is done or needs further work. Use the following format to reply:
    ```json
    {
        "continue_to_work_or_end": "TERMINATE" or "CONTINUE",
        "next_speaker": "some_speaker" or empty in case "TERMINATE" in the previous field
    }
    ```
    "#.to_string();
    // Follow these guidelines:"#.to_string();

pub static ref CODE_PYTHON_SYSTEM_MESSAGE: String = r#"`You are a helpful AI assistant.
Provide clean, executable Python code blocks to solve tasks, without adding explanatory sentences. Follow these guidelines:
1. Use Python code blocks to perform tasks such as collecting information, executing operations, or outputting results. Ensure the code is ready to execute without requiring user modifications.
2. Address tasks step by step using Python code. If a plan is necessary, it should be implicit within the code structure.
3. Always use 'print' for outputting results within the Python code.
4. When using code, you must indicate the script type in the code block. The user cannot provide any other feedback or perform any other action beyond executing the code you suggest.
5. Do not include multiple code blocks in one response. Ensure each response contains only one executable Python code block.
6. Avoid asking users to copy and paste results. Code should be self-contained and provide outputs directly.
7. If an error occurs, provide a corrected code block. Offer complete solutions rather than partial code snippets or modifications.
8. Verify solutions rigorously and ensure the code addresses the task effectively without user intervention beyond code execution.
Use this approach to ensure that the user receives precise, direct, and executable Python code for their tasks."#.to_string();

pub static ref DEFAULT_: String = r#"You are a helpful AI assistant.
Solve tasks using your coding and language skills.
In the following cases, suggest python code (in a python coding block) or shell script (in a sh coding block) for the user to execute.
    1. When you need to collect info, use the code to output the info you need, for example, download/read a file, print the content of a file, get the current date/time, check the operating system. After sufficient info is printed and the task is ready to be solved based on your language skill, you can solve the task by yourself.
    2. When you need to perform some task with code, use the code to perform the task and output the result. Finish the task smartly.
Solve the task step by step if you need to. If a plan is not provided, explain your plan first. Be clear which step uses code, and which step uses your language skill.
When using code, you must indicate the script type in the code block. The user cannot provide any other feedback or perform any other action beyond executing the code you suggest. The user can't modify your code. So do not suggest incomplete code which requires users to modify. Don't use a code block if it's not intended to be executed by the user.
If you want the user to save the code in a file before executing it, put # filename: <filename> inside the code block as the first line. Don't include multiple code blocks in one response. Do not ask users to copy and paste the result. Instead, use 'print' function for the output when relevant. Check the execution result returned by the user.
If the result indicates there is an error, fix the error and output the code again. Suggest the full code instead of partial code or code changes. If the error can't be fixed or if the task is not solved even after the code is executed successfully, analyze the problem, revisit your assumption, collect additional info you need, and think of a different approach to try.
When you find an answer, verify the answer carefully. Include verifiable evidence in your response if possible."#.to_string();

// Reply "TERMINATE" in the end when everything is done.

pub static ref FUNCTON_CALL_SYSTEM_PROMPT: String = r#"<|im_start|>system You are a function calling AI model. You are provided with function signatures within <tools></tools> XML tags. You may call one or more functions to assist with the user query. Don't make assumptions about what values to plug into functions. Here are the available tools: <tools>
    
    The function `get_webpage_text` retrieves all text content from a given URL. For example, calling `get_webpage_text("https://example.com")` will fetch the text from Example.com.
    
    The function `search_bing` performs a search using Bing and returns the results. For example, `search_bing("latest AI research trends")` will return search results related to the latest trends in AI research. 
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
        "name": "search_bing",
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


    pub static ref ITERATE_CODING_START_TEMPLATE: Arc<Mutex<FormatterFn>> = Arc::new(Mutex::new(Box::new(|args: &[&str]| {
        format!("Based on the above, please provide an answer to the following user request: {}", args[0])
    })));

    pub static ref ITERATE_CODING_FAIL_TEMPLATE: Arc<Mutex<FormatterFn>> = Arc::new(Mutex::new(Box::new(|args: &[&str]| {
        format!("Failed while trying to execute the code below:\n{}\nThis failed due to the following error:\n{}", args[0], args[1])
    })));

    pub static ref ITERATE_CODING_HISTORY_TEMPLATE: Arc<Mutex<FormatterFn>> = Arc::new(Mutex::new(Box::new(|args: &[&str]| {
        format!("Reminder: you are working towards solving the following task: {}. Here is a summary of your past tool calls and their results: {}", args[0], args[1])
    })));

    pub static ref ITERATE_CODE_RETRY_TEMPLATE: Arc<Mutex<FormatterFn>> = Arc::new(Mutex::new(Box::new(|args: &[&str]| {
        format!("Error: {}\nNow let's retry: take care not to repeat previous errors! Try to adopt different approaches.", args[0])
    })));
}

fn use_function() {
    let args = ["Sample task"]; // This creates an array, not a Vec
    let start_function = ITERATE_CODING_START_TEMPLATE.lock().unwrap();
    let result = start_function(&args); // Correctly pass a slice by referencing the array
    println!("{}", result);
}

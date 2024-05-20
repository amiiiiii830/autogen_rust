use autogen_rust::exec_python::*;
use autogen_rust::llama_structs::*;
use autogen_rust::llm_llama_local::*;
use autogen_rust::webscraper_hook::*;
#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    let system_prompt = r#"<|im_start|>system You are a function calling AI model. You are provided with function signatures within <tools></tools> XML tags. You may call one or more functions to assist with the user query. Don't make assumptions about what values to plug into functions. Here are the available tools: <tools> {"type": "function", "function": {"name": "get_current_weather", "description": "Get the current weather", "parameters": {"type": "object", "properties": {"location": {"type": "string", "description": "The city and state, e.g. San Francisco, CA"}, "format": {"type": "string", "enum": ["celsius", "fahrenheit"], "description": "The temperature unit to use. Infer this from the users location."}}, "required": ["location", "format"]}}} </tools> Use the following pydantic model json schema for each tool call you will make: {"properties": {"arguments": {"title": "Arguments", "type": "object"}, "name": {"title": "Name", "type": "string"}}, "required": ["arguments", "name"], "title": "FunctionCall", "type": "object"} For each function call return a json object with function name and arguments within <tool_call></tool_call> XML tags as follows:
    <tool_call>
    {"arguments": <args-dict>, "name": <function-name>}
    </tool_call>"#;

    let user_prompt = r#"<|im_start|>user Fetch the weather of Glasgow Scottland <|im_end|>"#;

    // let system_prompt = "you're an AI assistant";

    // let user_prompt = "tell me a joke";

    // let res_obj = chat_inner_async(system_prompt, &user_prompt, 50)
    //     .await
    //     .expect("llm generation failed");

    // let res = output_llama_response(res_obj).expect("msg");
    // let res = fire_tool_call("go read https://news.google.ca")
    //     .await
    //     .expect("failed to get webpage text");
    // match std::process::Command::new("/Users/jichen/miniconda3/bin/python")

    let code_in_file = include_str!("search_paper.py");
    // let code = r#"print("hello")"#;

    let res = run_python(&code_in_file);

    println!("{:?}", res);
}

// export RUSTPYTHONPATH="/Users/jichen/Downloads/RustPython-0.3.1/pylib/Lib"
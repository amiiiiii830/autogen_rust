use async_openai::types::Role;
use autogen_rust::conversable_agent::*;
use autogen_rust::llama_structs::*;
use autogen_rust::message_store::*;
// use autogen_rust::tool_call_actuators::*;
use anyhow::Result;
use rusqlite::Connection;
use tokio;

#[tokio::main]
async fn main() -> Result<()> {
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

    // let code_in_file = include_str!("search_paper.py");
    // let code = "def is_prime(n):\n    if n <= 1:\n        return False\n    if n == 2:\n        return True\n    if n % 2 == 0:\n        return False\n    i = 3\n    while i * i <= n:\n        if n % i == 0:\n            return False\n        i += 2 \n    return True\n\nprime_numbers_below_100 = [num for num in range(2, 100) if is_prime(num)]\n\nprint(prime_numbers_below_100)";

    // match run_python_capture(&code) {
    //     Ok(res) => println!("{:?}", res),
    //     Err(res) => println!("{:?}", res),
    // };

    // let raw ="Here's a Python code block that calculates and prints all prime numbers below 100:\n\n```python\ndef is_prime(n):\n    if n <= 1:\n        return False\n    for i in range(2, int(n ** 0.5) + 1):\n        if n % i == 0:\n            return False\n    return True\n\nfor number in range(2, 100):\n    if is_prime(number):\n        print(number)\n```\nThis code will output all prime numbers below 100.";

    // let code = extract_code(raw);

    // println!("{:?}", code);

    // let mut coding_agent = ConversableAgent::new("coding");
    // let message = Message::new(
    //     Some(Content::Text(
    //         "create code to calculate prime numbers below 100".to_string(),
    //     )),
    //     Some("random".to_string()),
    //     Some(Role::User),
    //     None,
    // );

    // let code = coding_agent.start_coding(&message).await?;
    // println!("{:?}", code);
    let conn = Connection::open_in_memory()?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS GroupChat (
            id INTEGER PRIMARY KEY,
            agent_name TEXT NOT NULL,
            message_content TEXT,
            message_role TEXT,
            message_context TEXT,
            tokens_count INTEGER,
            next_speaker TEXT
        )",
        [],
    )?;

    let messages = vec![
        Message {
            content: Some(Content::Text("Hello".to_string())),
            name: Some("Agent1".to_string()),
            role: Some(Role::User),
        },
        Message {
            content: Some(Content::Text("How can I assist you?".to_string())),
            name: Some("Agent2".to_string()),
            role: Some(Role::Assistant),
        },
        Message {
            content: Some(Content::ToolCall(ToolCall {
                name: "search".to_string(),
                arguments: Some(std::collections::HashMap::new()),
            })),
            name: Some("Agent1".to_string()),
            role: Some(Role::Tool),
        },
    ];

    for message in messages {
        save_message(conn, "Agent1", message.clone(), "Agent2").await?;
    }
    let messages = retrieve_messages(conn, "Agent1").await?;
    for message in messages {
        println!("{:?}", message);
    }

    Ok(())
}

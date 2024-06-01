use anyhow::Result;
use autogen_rust::immutable_agent::*;
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
    // let conn = Connection::open_in_memory()?;
    let conn = Connection::open("src/database.db")?;

    // conn.execute(
    //     "CREATE TABLE IF NOT EXISTS GroupChat (
    //         id INTEGER PRIMARY KEY,
    //         agent_name TEXT NOT NULL,
    //         message_content TEXT NOT NULL,
    //         tokens_count INTEGER NOT NULL,
    //         next_speaker TEXT
    //     )",
    //     []
    // )?;
    //   let _ =  conn.execute("DELETE FROM GroupChat", [])?;

    use tokio::{select, signal};

    let agent = ImmutableAgent::simple("placeholder", "");
    let coding_agent = ImmutableAgent::simple("coding_agent", "");
    let user_proxy = ImmutableAgent::simple("user_proxy", "");

    // loop {
    //     select! {

    //        result= user_proxy.send("find fibonacci up to 15", &conn, "coding_agent") => agent = user_proxy.clone(),

    //        result= coding_agent.run (&conn,true)=> match result{
    //            Ok(new_agent) => {agent = coding_agent.clone();},
    //            Err(e) => eprintln!("Run error: {}", e),
    //        },

    //        _= signal::ctrl_c ()=> {std ::process ::exit (0);}
    //    }

    //     agent.run(&conn, true).await;
    // }
    // let message: Message = Message::new(
    //     Content::Text("find fibonacci up to 15".to_string()),
    //     Some("random".to_string()),
    //     Role::User
    // );

    // let code = coding_agent.code_with_python(&message, &conn).await?;

    //    let  res  = user_proxy.planning("go get today's weather forecast").await;

    // for _ in 1..9 {

    // let res = user_proxy.send("what's the Canadian dollar vs USD exchange rate", &conn, "user_proxy").await;
    // let _ = user_proxy.run(&conn, true).await;
    // coding_agent.run(&conn, false).await;
    // user_proxy.run(&conn, true).await;

    // coding_agent.send(message.clone(), &conn, Some("router_agent")).await;

    // router_agent.send(message.clone(), &conn, Some("router_agent")).await;
    // }
    // println!("{:?}", inp);

    // let messages = vec![
    //     Message {
    //         content: Content::Text("Hello".to_string()),
    //         name: Some("Agent1".to_string()),
    //         role: Role::User,
    //     },
    //     Message {
    //         content: Content::Text("How can I assist you?".to_string()),
    //         name: Some("Agent2".to_string()),
    //         role: Role::Assistant,
    //     },
    //     Message {
    //         content: Content::ToolCall(ToolCall {
    //             name: "search".to_string(),
    //             arguments: Some(std::collections::HashMap::new()),
    //         }),
    //         name: Some("Agent1".to_string()),
    //         role: Role::Tool,
    //     }
    // ];

    // for message in messages {
    //     save_message(&conn, "Agent1", message.clone(), "Agent2").await?;
    // }
    // let messages = retrieve_messages(&conn, "Agent1").await?;
    // for message in messages {
    //     println!("{:?}", message);
    // }

    Ok(())
}

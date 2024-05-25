use crate::FUNCTON_CALL_SYSTEM_PROMPT;
use async_openai::types::{CompletionUsage, CreateChatCompletionResponse, Role};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{llm_llama_local::chat_inner_async, webscraper_hook::get_webpage_text};

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct ToolCall {
    pub name: String,
    pub arguments: Option<HashMap<String, String>>,
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub enum Content {
    Text(String),
    ToolCall(ToolCall),
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct LlamaResponseMessage {
    pub content: Content,
    pub role: Role,
    pub usage: CompletionUsage,
}


fn extract_json_from_xml_like(xml_like_data: &str) -> Option<String> {
    let start_tag = "<tool_call>";
    let end_tag = "</tool_call>";

    if xml_like_data.trim().starts_with(start_tag) && xml_like_data.trim().ends_with(end_tag) {
        let start_pos = start_tag.len();
        let end_pos = xml_like_data.len() - end_tag.len();
        Some(xml_like_data[start_pos..end_pos].trim().to_string())
    } else {
        None
    }
}

pub fn output_llama_response(
    res_obj: CreateChatCompletionResponse,
) -> Option<LlamaResponseMessage> {
    let usage = res_obj.clone().usage.unwrap();
    let msg_obj = res_obj.clone().choices[0].message.clone();
    let role = msg_obj.clone().role;
    if let Some(data) = msg_obj.content {
        if let Some(json_str) = extract_json_from_xml_like(&data) {
            let tool_call: ToolCall = serde_json::from_str(&json_str).unwrap();
            return Some(LlamaResponseMessage {
                content: Content::ToolCall(tool_call),
                role: role,
                usage: usage,
            });
        } else {
            return Some(LlamaResponseMessage {
                content: Content::Text(data.to_owned()),
                role: role,
                usage: usage,
            });
        }
    }
    None
}

pub async fn fire_tool_call(
    // system_prompt: &str,
    // tool_call_obj: &str,
    user_prompt: &str,
) -> anyhow::Result<LlamaResponseMessage> {
    let system_prompt = &FUNCTON_CALL_SYSTEM_PROMPT;

    // let content = match function.name.as_str() {
    //     "getWeather" => {
    //         let argument_obj =
    //             serde_json::from_str::<HashMap<String, String>>(&function.arguments)?;

    //         get_weather(&argument_obj["city"].to_string())
    //     }
    //     "scraper" => {
    //         let argument_obj =
    //             serde_json::from_str::<HashMap<String, String>>(&function.arguments)?;

    //         scraper(argument_obj["url"].clone()).await
    //     }
    //     "getTimeOfDay" => get_time_of_day(),
    //     _ => "".to_string(),
    // };
    let res = chat_inner_async(&system_prompt, &user_prompt, 500).await?;

    // if let Some(parsed) = output_llama_response(res) {
    //     match parsed.content {
    //         Content::ToolCall(ref tool_call) => {
    //             let func_name = tool_call.name.clone();
    //             let arguments = tool_call.arguments.clone().unwrap();
    //             let url = &arguments["url"];
    //             if func_name == "get_webpage_text" {
    //                 let res = get_webpage_text(&url).await?;
    //                 println!("{:?}", res);
    //             }
    //         }

    //         _ => (),
    //     }

    //     return Ok(parsed);
    // }
    Err(anyhow::Error::msg("parsing error"))
}

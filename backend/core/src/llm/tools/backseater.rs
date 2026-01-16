use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Execute the backseater tool - generates unhelpful comments on user's writing
/// Uses direct function calling API (single call, no Agent loop)
pub async fn execute_tool(content: &str, api_key: &str) -> Result<Vec<BackseaterArgs>> {
    let client = reqwest::Client::new();

    // Limit content length to avoid token limits
    let truncated_content = if content.len() > 2000 {
        &content[content.len() - 2000..]
    } else {
        content
    };

    let request_payload = json!({
        "model": "gpt-4o-mini",
        "messages": [
            {
                "role": "system",
                "content": "You will generate very short, unhelpful and nitpicky comments on the user's writing. Use the commenter tool to provide your comments."
            },
            {
                "role": "user",
                "content": format!("Generate unhelpful comments on this text:\n\n{}", truncated_content)
            }
        ],
        "tools": [
            {
                "type": "function",
                "function": {
                    "name": "commenter",
                    "description": "Generate a read-only comment on a specific part of the user's writing.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "comment_on": {
                                "type": "string",
                                "description": "The specific part of the user's writing to comment on."
                            },
                            "comment": {
                                "type": "string",
                                "description": "The comment to generate."
                            },
                            "color_hex": {
                                "type": "string",
                                "description": "The color of the comment in hex format."
                            }
                        },
                        "required": ["comment_on", "comment"]
                    }
                }
            }
        ],
        "tool_choice": {
            "type": "function",
            "function": {
                "name": "commenter"
            }
        }
    });

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&request_payload)
        .send()
        .await
        .context("Failed to connect to OpenAI during backseater execution")?;

    if !response.status().is_success() {
        let error_msg = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("Backseater Tool Error: {}", error_msg));
    }

    let result: serde_json::Value = response.json().await?;

    // Extract function call arguments directly from the first response
    // No second API call needed!
    let tool_calls = result["choices"][0]["message"]["tool_calls"]
        .as_array()
        .context("No tool_calls in response")?;

    let mut comments = Vec::new();
    for tool_call in tool_calls {
        if let Some(function) = tool_call.get("function") {
            if let Some(args_str) = function.get("arguments").and_then(|v| v.as_str()) {
                if let Ok(comment) = serde_json::from_str::<BackseaterArgs>(args_str) {
                    comments.push(comment);
                } else {
                    tracing::warn!("Failed to parse tool call arguments: {}", args_str);
                }
            }
        }
    }

    // Limit to 3 comments max
    let limited_comments: Vec<BackseaterArgs> = comments.into_iter().take(3).collect();

    Ok(limited_comments)
}


#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct BackseaterArgs {
    pub comment_on: String,
    pub comment: String,
    pub color_hex: Option<String>,
}

// atb_ai_utils way. I'm using a direct API call instead to prevent looping back to the agent. And also using faster and cheaper models.

// use atb_ai_utils::{
//     agent::Agent,
//     openai::{responses::FunctionTool, GPT5_MINI},
// };
// use atb_types::Uuid;
// use serde::{Deserialize, Serialize};
// use serde_json::json;

// pub fn new_agent(
//     run_id: Uuid,
//     api_key: &str,
//     prompt: String,
//     _user_data: String,
// ) -> Agent {
//     Agent::new(run_id, api_key, &prompt, None)
//         .with_model(GPT5_MINI)
//         .with_tool(
//             commenter_tool(),
//             std::sync::Arc::new(move |f| {
//                 tracing::info!("new search request: {}", f.arguments);
//                 //search for category codes
//                 let args: BackseaterArgs = match serde_json::from_str(&f.arguments) {
//                     Ok(args) => args,
//                     Err(e) => {
//                         tracing::error!("Failed to deserialize find_match arguments: {}. Raw args: {}", e, f.arguments);
//                         return Box::pin(async move {
//                             Err(anyhow::anyhow!("Invalid arguments provided to find_match tool. Please check the format. Error: {}", e))
//                         });
//                     }
//                 };
//                 Box::pin(async move {
//                     Ok(serde_json::to_value(args).unwrap())
//                 })
//             }),
//         )
// }


// #[derive(Debug, Default, Clone, Serialize, Deserialize)]
// pub struct BackseaterArgs {
//     pub comment_on: String,
//     pub comment: String,
//     pub color_hex: Option<String>,
// }


// pub fn commenter_tool() -> FunctionTool {
//     FunctionTool {
//             name: "commenter".into(),
//             description: Some(
//                 "Generate a read-only comment on a specific part of the user's writing.".into(),
//             ),
//             strict: None,
//             parameters: Some(json!({
//                 "type": "object",
//                 "properties": {
//                     "comment_on": {
//                         "type": "string",
//                         "description": "The specific part of the user's writing to comment on."
//                     },
//                     "comment": {
//                         "type": "string",
//                         "description": "The comment to generate."
//                     },
//                     "color_hex": {
//                         "type": "string",
//                         "description": "The color of the comment in hex format."
//                     }
//                 },
//                 "required": ["comment_on", "comment"],
//                 "additionalProperties": false
//             })),
//         }
// }
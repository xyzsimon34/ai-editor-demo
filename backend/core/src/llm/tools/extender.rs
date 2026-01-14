use crate::llm::types::McpTool;
use anyhow::{Context, Result};
use serde_json::json;

pub fn to_tool_definition() -> McpTool {
    McpTool {
        name: "extender".to_string(),
        description: "Use this tool to continue or extend an article draft while maintaining the same style and identity"
            .to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "article_draft": {
                    "type": "string",
                    "description": "The article draft to continue"
                },
                "identity": {
                    "type": "string",
                    "description": "The identity or role of the writer (e.g., 'Science fiction writer', 'Technical writer')"
                }
            },
            "required": ["article_draft", "identity"]
        }),
    }
}

pub async fn execute_tool(article_draft: &str, identity: &str, api_key: &str) -> Result<String> {
    let client = reqwest::Client::new();

    let system_content = format!(
        "You are a {}. Please continue the article according to the user's content, keep the same style, and the content and style should not change too much. The content of the continuation should not repeat the content of the previous article.",
        identity
    );

    let request_payload = json!({
        "model": "gpt-4o",
        "messages": [
            {
                "role": "system",
                "content": system_content
            },
            {
                "role": "user",
                "content": article_draft
            }
        ],
        "temperature": 0.7
    });

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&request_payload)
        .send()
        .await
        .context("Failed to connect to OpenAI during extender execution")?;

    if !response.status().is_success() {
        let error_msg = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("Extender Tool Error: {}", error_msg));
    }

    let result: serde_json::Value = response.json().await?;

    let extended_output = result["choices"][0]["message"]["content"]
        .as_str()
        .context("Failed to get content from Extender response")?
        .to_string();

    Ok(extended_output)
}

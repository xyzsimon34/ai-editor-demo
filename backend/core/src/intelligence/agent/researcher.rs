use crate::intelligence::types::McpTool;
use anyhow::{Context, Result};
use serde_json::json;

pub fn to_tool_definition() -> McpTool {
    McpTool {
        name: "researcher".to_string(),
        description: "Use this tool when the text needs fact-checking or data verification"
            .to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "search the internet for this paragraph's human or location"
                }
            },
            "required": ["query"]
        }),
    }
}

pub async fn execute_tool(query: &str, api_key: &str) -> Result<String> {
    let client = reqwest::Client::new();

    let request_payload = json!({
        "model": "gpt-4o",
        "messages": [
            {
                "role": "system",
                "content": "You are a professional research assistant. Your goal is to take a query and provide a structured, in-depth analysis. \
                            Break down the topic into logical sections: Overview, Key Facts, and Implications. \
                            Provide a comprehensive summary even if you are using your internal knowledge base."
            },
            {
                "role": "user",
                "content": format!("Please conduct a research on the following topic: \"{}\"", query)
            }
        ],
        "temperature": 0.3
    });

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&request_payload)
        .send()
        .await
        .context("Failed to connect to OpenAI during researcher execution")?;

    if !response.status().is_success() {
        let error_msg = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("Researcher Tool Error: {}", error_msg));
    }

    let result: serde_json::Value = response.json().await?;

    let research_output = result["choices"][0]["message"]["content"]
        .as_str()
        .context("Failed to get content from Researcher response")?
        .to_string();

    Ok(research_output)
}

use anyhow::{Context, Result};
use serde_json::json;

pub async fn execute_tool(article_draft: &str, identity: &str, api_key: &str) -> Result<String> {
    let client = reqwest::Client::new();

    let system_content = 
        "You will finish the user's sentence as aggressively pessimistic as possible. **ONLY** respond with your generated part of the sentence, excluding the user's original context.".to_string();

    let request_payload = json!({
        "model": "gpt-4o-mini",
        "messages": [
            {
                "role": "system",
                "content": system_content
            },
            {
                "role": "user",
                "content": article_draft
            }
        ]
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

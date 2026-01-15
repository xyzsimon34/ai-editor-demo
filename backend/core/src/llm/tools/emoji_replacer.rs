use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Replacement {
    pub replace: String,
    pub with: String,
}

/// Execute the emoji replacer tool
/// 
/// Takes plain text content and asks AI to suggest word-to-emoji replacements.
/// Returns a JSON array of Replacement structs.
pub async fn execute_tool(content: &str, api_key: &str) -> Result<Vec<Replacement>> {
    let client = reqwest::Client::new();

    // Limit content length to avoid token limits (keep last 2000 chars)
    let truncated_content = if content.len() > 2000 {
        &content[content.len() - 2000..]
    } else {
        content
    };

    let system_content = r#"You are a helpful assistant that suggests emoji replacements for words in text.
Given a text, return a JSON object with a "replacements" key containing an array of replacement suggestions.
Each replacement should be in the format: {"replace": "word", "with": "😀"}
Only suggest replacements that make sense (e.g., "happy" -> "😀", "sad" -> "😢").
Limit to maximum 10 replacements.
Return ONLY valid JSON in this format: {"replacements": [{"replace": "word1", "with": "😀"}, ...]}"#.to_string();

    let user_content = format!(
        "Given this text, suggest emoji replacements:\n\n{}",
        truncated_content
    );

    let request_payload = json!({
        "model": "gpt-4o-mini",
        "messages": [
            {
                "role": "system",
                "content": system_content
            },
            {
                "role": "user",
                "content": user_content
            }
        ],
        "response_format": {
            "type": "json_object"
        }
    });

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&request_payload)
        .send()
        .await
        .context("Failed to connect to OpenAI during emoji replacer execution")?;

    if !response.status().is_success() {
        let error_msg = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("Emoji Replacer Tool Error: {}", error_msg));
    }

    let result: serde_json::Value = response.json().await?;

    let content_str = result["choices"][0]["message"]["content"]
        .as_str()
        .context("Failed to get content from Emoji Replacer response")?;

    tracing::debug!("Raw AI response: {}", content_str);

    // Parse the JSON response
    // With json_object format, we expect {"replacements": [...]}
    // But also handle cases where it might return just [...]
    let parsed: serde_json::Value = serde_json::from_str(content_str)
        .context("Failed to parse JSON response")?;

    let replacements: Vec<Replacement> = if let Some(arr) = parsed.get("replacements").and_then(|v| v.as_array()) {
        // Format: {"replacements": [...]}
        serde_json::from_value(serde_json::Value::Array(arr.clone()))
            .context("Failed to parse replacements array from 'replacements' key")?
    } else if let Some(arr) = parsed.as_array() {
        // Format: [...] (fallback if AI doesn't follow instructions)
        serde_json::from_value(serde_json::Value::Array(arr.clone()))
            .context("Failed to parse replacements as direct array")?
    } else {
        // Log the actual response for debugging
        tracing::warn!("Unexpected JSON format: {}", parsed);
        return Err(anyhow::anyhow!(
            "Expected JSON object with 'replacements' key or array, got: {}",
            parsed
        ));
    };

    // Limit to 10 replacements max
    let limited_replacements: Vec<Replacement> = replacements.into_iter().take(10).collect();

    Ok(limited_replacements)
}


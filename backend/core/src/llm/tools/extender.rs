use anyhow::{Context, Result};
use serde_json::json;

use crate::llm::types::ExtenderContext;

pub async fn execute_tool(
    article_draft: &str,
    role: &str,
    context: Option<&ExtenderContext>,
    api_key: &str,
) -> Result<String> {
    let client = reqwest::Client::new();

    let system_content = format!(
        "Role: {role}\n\nTask: Either finish the user's sentence if it's not complete, or write ONE new sentence.  Start a new paragraph as needed.\nRules:\n- Preserve the existing tone, style, and meaning. Do not repeat what you have already generated.\n- If context metadata is provided, use it to stay accurate and consistent.\n- ONLY respond with your generated part (do not repeat the original text)."
    );

    let mut user_content = String::new();
    if let Some(ctx) = context {
        if let Some(instruction) = ctx.instruction.as_deref() {
            user_content.push_str("<instruction>\n");
            user_content.push_str(instruction);
            user_content.push_str("\n</instruction>\n\n");
        }

        if let Some(metadata) = ctx.metadata.as_ref() {
            user_content.push_str("<context>\n");
            user_content.push_str(&serde_json::to_string_pretty(metadata)?);
            user_content.push_str("\n</context>\n\n");
        }
    }

    tracing::info!("article_draft: {}", article_draft);

    user_content.push_str("<document>\n");
    user_content.push_str(article_draft);
    user_content.push_str("\n</document>");

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

    let mut extended_output = result["choices"][0]["message"]["content"]
        .as_str()
        .context("Failed to get content from Extender response")?
        .to_string();

    let needs_leading_space = !article_draft
        .chars()
        .last()
        .is_some_and(|c| c.is_whitespace());

    if needs_leading_space && !extended_output.is_empty() {
        let first_char = extended_output.chars().next();
        let starts_with_whitespace = first_char.is_some_and(|c| c.is_whitespace());
        let attaches_to_previous = first_char.is_some_and(|c| {
            matches!(
                c,
                '.' | ',' | ';' | ':' | '!' | '?' | ')' | ']' | '}' | '\'' | '’'
            )
        });

        if !starts_with_whitespace && !attaches_to_previous {
            extended_output.insert(0, ' ');
        }
    }

    Ok(extended_output)
}

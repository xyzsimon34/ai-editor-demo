use crate::llm::llm_model;
use crate::refiner::types::{RefineInput, RefineOutput};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
}

#[derive(Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessageResponse,
}

#[derive(Deserialize)]
struct ChatMessageResponse {
    content: String,
}

pub async fn call_improve_api(input: RefineInput, api_key: &str) -> Result<RefineOutput> {
    let client = reqwest::Client::new();

    let system_message = "You are an AI writing assistant that improves existing text. Limit your response to no more than 200 characters, but make sure to construct complete sentences. Use Markdown formatting when appropriate.";
    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&ChatRequest {
            model: llm_model::GPT4_O.to_string(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: system_message.to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: format!("The existing text is: {}", input.content),
                },
            ],
        })
        .send()
        .await
        .context("Failed to send request to OpenAI API")?;

    let result: ChatResponse = response
        .json()
        .await
        .context("Failed to parse OpenAI API response")?;

    Ok(RefineOutput {
        content: result
            .choices
            .first()
            .and_then(|c| Some(c.message.content.clone()))
            .context("No choices in OpenAI API response")?,
    })
}

pub async fn call_fix_api(input: RefineInput, api_key: &str) -> Result<RefineOutput> {
    let client = reqwest::Client::new();

    let system_message = "You are an AI writing assistant that fixes grammar and spelling errors in existing text. Limit your response to no more than 200 characters, but make sure to construct complete sentences. Use Markdown formatting when appropriate.".to_string();
    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&ChatRequest {
            model: llm_model::GPT4_O.to_string(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: system_message.to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: format!("The existing text is: {}", input.content),
                },
            ],
        })
        .send()
        .await
        .context("Failed to send request to OpenAI API")?;

    let result: ChatResponse = response
        .json()
        .await
        .context("Failed to parse OpenAI API response")?;

    Ok(RefineOutput {
        content: result
            .choices
            .first()
            .and_then(|c| Some(c.message.content.clone()))
            .context("No choices in OpenAI API response")?,
    })
}

pub async fn call_longer_api(input: RefineInput, api_key: &str) -> Result<RefineOutput> {
    let client = reqwest::Client::new();

    let system_message ="You are an AI writing assistant that lengthens existing text. Use Markdown formatting when appropriate.".to_string();
    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&ChatRequest {
            model: llm_model::GPT4_O.to_string(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: system_message.to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: format!("The existing text is: {}", input.content),
                },
            ],
        })
        .send()
        .await
        .context("Failed to send request to OpenAI API")?;

    let result: ChatResponse = response
        .json()
        .await
        .context("Failed to parse OpenAI API response")?;

    Ok(RefineOutput {
        content: result
            .choices
            .first()
            .and_then(|c| Some(c.message.content.clone()))
            .context("No choices in OpenAI API response")?,
    })
}

pub async fn call_shorter_api(input: RefineInput, api_key: &str) -> Result<RefineOutput> {
    let client = reqwest::Client::new();

    let system_message = "You are an AI writing assistant that shortens existing text. Use Markdown formatting when appropriate.".to_string();
    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&ChatRequest {
            model: llm_model::GPT4_O.to_string(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: system_message.to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: format!("The existing text is: {}", input.content),
                },
            ],
        })
        .send()
        .await
        .context("Failed to send request to OpenAI API")?;

    let result: ChatResponse = response
        .json()
        .await
        .context("Failed to parse OpenAI API response")?;

    Ok(RefineOutput {
        content: result
            .choices
            .first()
            .and_then(|c| Some(c.message.content.clone()))
            .context("No choices in OpenAI API response")?,
    })
}

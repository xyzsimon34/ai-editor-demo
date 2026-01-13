use crate::intelligence::types::{McpTool, get_sub_agent_definitions};
use crate::model::{Agent, PulseInput, PulseOutput};
use anyhow::{Context, Result};
use futures::future::join_all;
use serde_json::{Value, json};
use std::collections::HashMap;

pub struct Brain;

impl Brain {
    pub async fn evaluate_pulse(req: PulseInput, api_key: &str) -> PulseOutput {
        use tracing::{error, info, warn};

        let available_tools: Vec<McpTool> = get_sub_agent_definitions();

        info!(
            available_tools_count = available_tools.len(),
            "Starting pulse evaluation"
        );

        let tool_requests =
            match Self::decide_tools_to_use(&req.text, &available_tools, api_key).await {
                Ok(requests) => {
                    info!(count = requests.len(), "Sub-agents selected by AI");
                    requests
                }
                Err(e) => {
                    error!(error = %e, "Decision phase failed");
                    vec![]
                }
            };

        let mut tasks = Vec::new();
        for (tool_name, arguments) in tool_requests {
            tasks.push(Self::execute_sub_agent(tool_name, arguments, api_key));
        }

        let results = join_all(tasks).await;

        // 4. 彙整結果
        let mut suggestions = HashMap::new();
        for result in results {
            match result {
                Ok((agent, output)) => {
                    suggestions.insert(agent, output);
                }
                Err(e) => error!(error = %e, "Sub-agent execution failed"),
            }
        }

        PulseOutput { suggestions }
    }

    pub async fn execute_sub_agent(
        name: String,
        arguments: Value,
        api_key: &str,
    ) -> Result<(Agent, String)> {
        use crate::intelligence::agent::{refiner, researcher};

        match name.as_str() {
            "research_agent" => {
                let query = arguments["query"].as_str().unwrap_or("");
                let output = researcher::execute_tool(query, api_key).await?;
                Ok((Agent::Researcher, output))
            }
            "refine_agent" => {
                let text = arguments["text"].as_str().unwrap_or("");
                let output = refiner::execute_tool(text, api_key).await?;
                Ok((Agent::Refiner, output))
            }
            _ => Err(anyhow::anyhow!("Unknown sub-agent: {}", name)),
        }
    }

    pub async fn decide_tools_to_use(
        text: &str,
        available_tools: &[McpTool],
        api_key: &str,
    ) -> Result<Vec<(String, Value)>> {
        use tracing::debug;
        let client = reqwest::Client::new();

        let tools: Vec<Value> = available_tools
            .iter()
            .map(|tool| {
                json!({
                    "type": "function",
                    "function": {
                        "name": tool.name,
                        "description": tool.description,
                        "parameters": tool.input_schema
                    }
                })
            })
            .collect();

        let request_payload = json!({
            "model": "gpt-4o",
            "messages": [
                {
                    "role": "system",
                    "content": "You are a task orchestrator. Delegate tasks to specialized sub-agents based on the user's request. Always prefer using tools over answering directly."
                },
                {
                    "role": "user",
                    "content": text
                }
            ],
            "tools": tools,
            "tool_choice": "auto"
        });

        let response = client
            .post("https://api.openai.com/v1/chat/completions")
            .bearer_auth(api_key)
            .json(&request_payload)
            .send()
            .await
            .context("OpenAI API unreachable")?;

        let result: Value = response.json().await?;

        let mut tool_requests = Vec::new();
        if let Some(calls) = result["choices"][0]["message"]["tool_calls"].as_array() {
            for call in calls {
                let name = call["function"]["name"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string();
                let args_str = call["function"]["arguments"].as_str().unwrap_or("{}");
                let args: Value = serde_json::from_str(args_str).unwrap_or(json!({}));

                tool_requests.push((name, args));
            }
        }

        Ok(tool_requests)
    }
}

use crate::intelligence::types::{ToolAction, ToolDefinition};
use crate::model::{Agent, PulseInput, PulseOutput};
use anyhow::{Context, Result};
use futures::future::join_all;
use serde_json::json;
use std::collections::HashMap;

pub struct Brain;

impl Brain {
    pub async fn evaluate_pulse(req: PulseInput, api_key: &'static str) -> PulseOutput {
        use tracing::{error, info, warn};

        let available_tools: Vec<ToolDefinition> =
            req.agents.iter().map(|a| a.to_tool_definition()).collect();

        info!(
            available_tools_count = available_tools.len(),
            "Starting pulse evaluation"
        );

        let decisions = match Self::decide_tools_to_use(&req.text, &available_tools, api_key).await
        {
            Ok(actions) => {
                info!(actions_count = actions.len(), "Tools decided by AI");
                actions
            }
            Err(e) => {
                error!(error = %e, "Failed to decide tools, returning empty list");
                vec![]
            }
        };

        let mut tasks = Vec::new();
        for action in &decisions {
            tasks.push(Self::execute_tool(action.clone(), &req.text, api_key));
        }

        let results = join_all(tasks).await;

        let mut suggestions = HashMap::new();
        let mut error_count = 0;
        for result in results {
            match result {
                Ok((agent, output)) => {
                    info!(
                        ?agent,
                        output_len = output.len(),
                        "Tool executed successfully"
                    );
                    suggestions.insert(agent, output);
                }
                Err(e) => {
                    error_count += 1;
                    error!(error = %e, "Tool execution failed");
                }
            }
        }

        if suggestions.is_empty() && error_count > 0 {
            warn!(error_count, "All tool executions failed");
        }

        info!(
            suggestions_count = suggestions.len(),
            "Pulse evaluation completed"
        );
        PulseOutput { suggestions }
    }

    pub async fn decide_tools_to_use(
        text: &str,
        available_tools: &[ToolDefinition],
        api_key: &'static str,
    ) -> Result<Vec<ToolAction>> {
        // Convert available_tools to OpenAI function calling format
        let tools: Vec<serde_json::Value> = available_tools
            .iter()
            .map(|tool| {
                json!({
                    "type": "function",
                    "function": {
                        "name": tool.name,
                        "description": tool.description,
                        "parameters": tool.parameters
                    }
                })
            })
            .collect();

        use tracing::debug;

        let client = reqwest::Client::new();

        // Build a more explicit prompt to encourage tool usage
        let tool_descriptions: Vec<String> = available_tools
            .iter()
            .map(|t| format!("- {}: {}", t.name, t.description))
            .collect();

        let request_payload = json!({
            "model": "gpt-4o",
            "messages": [{
                "role": "system",
                "content": format!("You are a tool selection assistant. You MUST use the available tools to process the user's text. Available tools:\n{}", tool_descriptions.join("\n"))
            }, {
                "role": "user",
                "content": format!("Process the following text using the appropriate tools. Text: \"{}\"", text)
            }],
            "tools": tools,
            "tool_choice": "auto"
        });

        debug!(request = %serde_json::to_string_pretty(&request_payload).unwrap_or_default(), "Sending request to OpenAI");

        let response = client
            .post("https://api.openai.com/v1/chat/completions")
            .bearer_auth(api_key)
            .json(&request_payload)
            .send()
            .await
            .context("Failed to send request to OpenAI API")?;

        let status = response.status();
        debug!(status = %status, "Received response from OpenAI");

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "OpenAI API returned error status {}: {}",
                status,
                error_text
            ));
        }

        let result: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse OpenAI API response")?;

        // Parse tool_calls from response
        use tracing::warn;

        let empty_vec = Vec::new();
        let tool_calls = result
            .get("choices")
            .and_then(|c| c.as_array())
            .and_then(|c| c.first())
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("tool_calls"))
            .and_then(|tc| tc.as_array())
            .unwrap_or(&empty_vec);

        if tool_calls.is_empty() {
            debug!("OpenAI response contains no tool_calls");
            // Log the full response for debugging
            if let Ok(response_str) = serde_json::to_string_pretty(&result) {
                debug!(response = %response_str, "Full OpenAI response");
            }
        }

        // Map tool_calls to ToolAction
        let actions: Vec<ToolAction> = tool_calls
            .iter()
            .filter_map(|tc| {
                let function_name = tc
                    .get("function")
                    .and_then(|f| f.get("name"))
                    .and_then(|n| n.as_str());

                match function_name {
                    Some(name) => available_tools
                        .iter()
                        .find(|tool| tool.name == name)
                        .map(|tool| {
                            debug!(tool_name = name, ?tool.action, "Mapped tool call to action");
                            tool.action.clone()
                        })
                        .or_else(|| {
                            warn!(tool_name = name, "Unknown tool name from OpenAI");
                            None
                        }),
                    None => {
                        warn!("Tool call missing function name");
                        None
                    }
                }
            })
            .collect();

        debug!(
            actions_count = actions.len(),
            "Mapped tool calls to actions"
        );
        Ok(actions)
    }

    pub async fn execute_tool(
        tool: ToolAction,
        text: &str,
        api_key: &'static str,
    ) -> Result<(Agent, String)> {
        match tool {
            ToolAction::Research => {
                let output = Self::execute_research_tool(text, api_key).await?;
                Ok((Agent::Researcher, output))
            }
            ToolAction::Refine => {
                let output = Self::execute_refine_tool(text, api_key).await?;
                Ok((Agent::Refiner, output))
            }
        }
    }

    async fn execute_research_tool(text: &str, _api_key: &'static str) -> Result<String> {
        // TODO: Implement research tool
        Ok(format!("Research result for: {}", text))
    }

    async fn execute_refine_tool(text: &str, api_key: &'static str) -> Result<String> {
        use crate::refiner::processor;
        use crate::refiner::types::RefineInput;

        let input = RefineInput {
            content: text.to_string(),
        };
        let output = processor::call_improve_api(input, api_key.to_string()).await?;
        Ok(output.content)
    }
}

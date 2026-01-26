use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use yrs::Doc;

/// 工具定義（MCP 格式）
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// Agent 配置
#[derive(Clone, Debug)]
pub struct AgentConfig {
    pub temperature: f32,
    pub max_iterations: usize,
    pub max_tokens: Option<u32>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            temperature: 0.7,
            max_iterations: 10,
            max_tokens: None,
        }
    }
}

/// Agent 結構 - 支援多次迭代和長期存活
pub struct Agent {
    pub name: String,
    pub model: String,
    pub api_key: String,
    pub description: String,
    pub tools: Vec<ToolDefinition>,
    pub config: AgentConfig,

    // 內部狀態（長期存活，保留歷史）
    conversation_history: Vec<Message>,
    tool_call_history: Vec<ToolCall>,
    initialized: bool, // 是否已初始化 system message
}

/// 對話消息
#[derive(Clone, Debug)]
pub struct Message {
    pub role: String, // "system", "user", "assistant", "tool"
    pub content: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub tool_call_id: Option<String>, // 用於 tool role
}

/// 工具調用記錄
#[derive(Clone, Debug)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
    pub result: Option<Value>,
}

/// Agent 執行上下文
pub struct AgentContext {
    pub doc: Arc<Doc>,
    pub user_last_used_at: Arc<AtomicU64>,
    pub user_writing_timeout_ms: u64,
    pub preview_mode: bool,
}

/// Agent 執行結果
pub struct AgentResult {
    pub output: Option<String>,    // 最終輸出（預覽模式）
    pub iterations: usize,         // 迭代次數
    pub tool_calls: Vec<ToolCall>, // 工具調用歷史
}

/// LLM 回應結構（內部使用）
struct LLMResponse {
    content: Option<String>,
    tool_calls: Option<Vec<ToolCall>>,
}

impl Agent {
    pub fn new(
        name: String,
        model: String,
        api_key: String,
        description: String,
        tools: Vec<ToolDefinition>,
    ) -> Self {
        Self {
            name,
            model,
            api_key,
            description,
            tools,
            config: AgentConfig::default(),
            conversation_history: Vec::new(),
            tool_call_history: Vec::new(),
            initialized: false,
        }
    }

    /// 重置 Agent 狀態（清空歷史）
    pub fn reset(&mut self) {
        self.conversation_history.clear();
        self.tool_call_history.clear();
        self.initialized = false;
        tracing::info!("🔄 Agent '{}' reset", self.name);
    }

    /// 獲取對話歷史（用於調試或持久化）
    pub fn get_conversation_history(&self) -> &[Message] {
        &self.conversation_history
    }

    /// 獲取工具調用歷史
    pub fn get_tool_call_history(&self) -> &[ToolCall] {
        &self.tool_call_history
    }

    /// 檢查是否已初始化
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Builder: 設置配置
    pub fn with_config(mut self, config: AgentConfig) -> Self {
        self.config = config;
        self
    }

    /// Builder: 添加工具
    pub fn with_tool(mut self, tool: ToolDefinition) -> Self {
        self.tools.push(tool);
        self
    }

    /// Builder: 批量添加工具
    pub fn with_tools(mut self, tools: Vec<ToolDefinition>) -> Self {
        self.tools.extend(tools);
        self
    }

    /// 執行 Agent（支援多次迭代和長期存活）
    ///
    /// 如果 Agent 尚未初始化，會先添加 system message
    /// 每次調用會保留歷史記錄，只添加新的 user message
    pub async fn run(&mut self, task: &str, context: &AgentContext) -> Result<AgentResult> {
        // 如果尚未初始化，添加 system message
        if !self.initialized {
            self.conversation_history.push(Message {
                role: "system".to_string(),
                content: Some(self.description.clone()),
                tool_calls: None,
                tool_call_id: None,
            });
            self.initialized = true;
            tracing::info!("🚀 Agent '{}' initialized", self.name);
        }

        // 添加新的 user message（保留歷史）
        self.conversation_history.push(Message {
            role: "user".to_string(),
            content: Some(task.to_string()),
            tool_calls: None,
            tool_call_id: None,
        });

        tracing::info!(
            "📝 Agent '{}' received new task (history: {} messages, {} tool calls)",
            self.name,
            self.conversation_history.len(),
            self.tool_call_history.len()
        );

        let mut iterations = 0;

        // Agent Loop: 多次迭代直到完成
        loop {
            if iterations >= self.config.max_iterations {
                return Err(anyhow::anyhow!(
                    "Agent '{}' reached max iterations ({}). Last message: {:?}",
                    self.name,
                    self.config.max_iterations,
                    self.conversation_history.last()
                ));
            }

            tracing::info!(
                "🔄 Agent '{}' iteration {}/{}",
                self.name,
                iterations + 1,
                self.config.max_iterations
            );

            // 1. 調用 LLM
            let response = self.call_llm().await?;

            // 2. 檢查是否有工具調用
            if let Some(tool_calls) = response.tool_calls {
                tracing::info!(
                    "🔧 Agent '{}' wants to call {} tools",
                    self.name,
                    tool_calls.len()
                );

                // 3. 執行每個工具
                let mut tool_results = Vec::new();
                for tool_call in &tool_calls {
                    let result = self.execute_tool(tool_call, context).await?;

                    // 記錄工具調用
                    let mut recorded_call = tool_call.clone();
                    recorded_call.result = Some(result.clone());
                    self.tool_call_history.push(recorded_call.clone());

                    tool_results.push((tool_call.id.clone(), result));
                }

                // 4. 將 assistant 消息加入歷史
                self.conversation_history.push(Message {
                    role: "assistant".to_string(),
                    content: response.content,
                    tool_calls: Some(tool_calls),
                    tool_call_id: None,
                });

                // 5. 將工具結果加入歷史
                for (tool_call_id, result) in tool_results {
                    self.conversation_history.push(Message {
                        role: "tool".to_string(),
                        content: Some(serde_json::to_string(&result)?),
                        tool_calls: None,
                        tool_call_id: Some(tool_call_id),
                    });
                }

                iterations += 1;
                continue; // 繼續 loop，讓 LLM 根據工具結果決定下一步
            } else {
                // 6. 沒有工具調用，Agent 完成任務
                tracing::info!(
                    "✅ Agent '{}' completed after {} iterations",
                    self.name,
                    iterations
                );

                return Ok(AgentResult {
                    output: response.content,
                    iterations,
                    tool_calls: self.tool_call_history.clone(),
                });
            }
        }
    }

    /// 調用 LLM API
    async fn call_llm(&self) -> Result<LLMResponse> {
        let client = reqwest::Client::new();

        // 構建 messages
        let messages: Vec<Value> = self
            .conversation_history
            .iter()
            .map(|msg| {
                let mut msg_obj = json!({
                    "role": msg.role,
                });

                if let Some(content) = &msg.content {
                    msg_obj["content"] = json!(content);
                }

                if let Some(tool_calls) = &msg.tool_calls {
                    msg_obj["tool_calls"] = json!(
                        tool_calls
                            .iter()
                            .map(|tc| {
                                json!({
                                    "id": tc.id,
                                    "type": "function",
                                    "function": {
                                        "name": tc.name,
                                        "arguments": tc.arguments  // arguments 已經是 Value，直接使用
                                    }
                                })
                            })
                            .collect::<Vec<_>>()
                    );
                }

                if let Some(tool_call_id) = &msg.tool_call_id {
                    msg_obj["tool_call_id"] = json!(tool_call_id);
                }

                msg_obj
            })
            .collect();

        // 構建工具定義
        let tool_definitions: Vec<Value> = self
            .tools
            .iter()
            .map(|t| {
                json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.input_schema
                    }
                })
            })
            .collect();

        let mut request_payload = json!({
            "model": self.model,
            "messages": messages,
            "tools": tool_definitions,
            "temperature": self.config.temperature,
        });

        if let Some(max_tokens) = self.config.max_tokens {
            request_payload["max_tokens"] = json!(max_tokens);
        }

        let response = client
            .post("https://api.openai.com/v1/chat/completions")
            .bearer_auth(&self.api_key)
            .json(&request_payload)
            .send()
            .await
            .context("Failed to call OpenAI API")?;

        if !response.status().is_success() {
            let error_msg = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("OpenAI API error: {}", error_msg));
        }

        let result: Value = response.json().await?;

        // 解析回應
        let message = &result["choices"][0]["message"];
        let content = message["content"].as_str().map(|s| s.to_string());

        let tool_calls = if let Some(tc_array) = message["tool_calls"].as_array() {
            Some(
                tc_array
                    .iter()
                    .filter_map(|tc| {
                        let id = tc["id"].as_str()?.to_string();
                        let function = tc["function"].as_object()?;
                        let name = function["name"].as_str()?.to_string();
                        // arguments 可能是 string 或 object，需要處理兩種情況
                        let arguments = if let Some(arg_str) = function["arguments"].as_str() {
                            // 如果是 string，嘗試解析為 JSON
                            serde_json::from_str(arg_str).unwrap_or_else(|_| json!({}))
                        } else {
                            // 如果已經是 object，直接使用
                            function["arguments"].clone()
                        };

                        Some(ToolCall {
                            id,
                            name,
                            arguments,
                            result: None,
                        })
                    })
                    .collect(),
            )
        } else {
            None
        };

        Ok(LLMResponse {
            content,
            tool_calls,
        })
    }

    /// 執行工具
    async fn execute_tool(&self, tool_call: &ToolCall, context: &AgentContext) -> Result<Value> {
        tracing::info!(
            "🔨 Executing tool: {} with args: {:?}",
            tool_call.name,
            tool_call.arguments
        );

        // 根據工具名稱路由到對應的執行函數
        match tool_call.name.as_str() {
            "linter" => {
                let (_result, _doc) =
                    crate::llm::tools::linter::execute_tool(context.doc.clone(), &self.api_key)
                        .await?;
                Ok(json!({"status": "success", "message": "Linter completed"}))
            }
            "researcher" => {
                let query = tool_call.arguments["query"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing query parameter"))?;
                let result =
                    crate::llm::tools::researcher::execute_tool(query, &self.api_key).await?;
                Ok(json!({"status": "success", "result": result}))
            }
            "paragraph_inserter" => {
                let paragraph_index = tool_call.arguments["paragraph_index"]
                    .as_u64()
                    .map(|v| v as u32)
                    .unwrap_or(0);
                let context_str = tool_call.arguments["context"]
                    .as_str()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| crate::editor::get_doc_content(&context.doc));

                crate::llm::tools::paragraph_inserter::execute_tool(
                    &context.doc,
                    paragraph_index,
                    &context_str,
                    &self.api_key,
                )
                .await?;
                Ok(json!({"status": "success", "message": "Content inserted"}))
            }
            "refiner" => {
                let text = tool_call.arguments["text"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing text parameter"))?;
                let result = crate::llm::tools::refiner::execute_tool(text, &self.api_key).await?;
                Ok(json!({"status": "success", "result": result}))
            }
            _ => Err(anyhow::anyhow!("Unknown tool: {}", tool_call.name)),
        }
    }
}

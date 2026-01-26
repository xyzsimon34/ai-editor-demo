use super::types::{Agent, AgentConfig, ToolDefinition};
use serde_json::json;

/// 創建預設的潤飾文章 Agent
pub fn create_polishing_agent(name: String, api_key: String, model: Option<String>) -> Agent {
    Agent::new(
        name,
        model.unwrap_or_else(|| "gpt-4o".to_string()),
        api_key,
        "You are a professional writing assistant specialized in polishing and refining articles. \
         Your task is to analyze the article structure, identify areas for improvement, and use \
         appropriate tools to enhance the writing quality step by step. \
         Always think carefully about which tools to use and in what order."
            .to_string(),
        vec![
            linter_tool_definition(),
            researcher_tool_definition(),
            paragraph_inserter_tool_definition(),
            refiner_tool_definition(),
        ],
    )
    .with_config(AgentConfig {
        temperature: 0.7,
        max_iterations: 15,
        max_tokens: Some(4000),
    })
}

/// 創建預設的寫作助手 Agent
pub fn create_writing_assistant_agent(
    name: String,
    api_key: String,
    model: Option<String>,
) -> Agent {
    Agent::new(
        name,
        model.unwrap_or_else(|| "gpt-4o".to_string()),
        api_key,
        "You are a helpful writing assistant. Help users improve their writing by using \
         appropriate tools to fix errors, enhance clarity, and refine the text."
            .to_string(),
        vec![linter_tool_definition(), refiner_tool_definition()],
    )
    .with_config(AgentConfig {
        temperature: 0.7,
        max_iterations: 10,
        max_tokens: Some(2000),
    })
}

/// Linter 工具定義
pub fn linter_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "linter".to_string(),
        description: "Fix grammar and spelling errors in the document. Use this tool to correct \
                     linguistic mistakes while preserving the original meaning and tone."
            .to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {},
            "required": []
        }),
    }
}

/// Researcher 工具定義
pub fn researcher_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "researcher".to_string(),
        description: "Research facts and verify information. Use this tool when you need to \
                     fact-check content, gather background information, or verify claims in the text.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The research topic, question, or claim to investigate"
                }
            },
            "required": ["query"]
        }),
    }
}

/// Paragraph Inserter 工具定義
pub fn paragraph_inserter_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "paragraph_inserter".to_string(),
        description: "Insert new content into a specific paragraph in the document. Use this tool \
                     when you need to add explanations, examples, or additional information to enhance \
                     the article.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "paragraph_index": {
                    "type": "integer",
                    "description": "The index of the paragraph (0-based) where to insert content"
                },
                "context": {
                    "type": "string",
                    "description": "Instructions or context about what content to insert"
                }
            },
            "required": ["paragraph_index", "context"]
        }),
    }
}

/// Refiner 工具定義
pub fn refiner_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "refiner".to_string(),
        description: "Improve and refine text quality. Use this tool to enhance clarity, improve \
                     word choice, and polish the writing style while maintaining the original meaning.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "text": {
                    "type": "string",
                    "description": "The text to refine or improve"
                }
            },
            "required": ["text"]
        }),
    }
}

/// 獲取所有可用工具的定義
pub fn get_all_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        linter_tool_definition(),
        researcher_tool_definition(),
        paragraph_inserter_tool_definition(),
        refiner_tool_definition(),
    ]
}

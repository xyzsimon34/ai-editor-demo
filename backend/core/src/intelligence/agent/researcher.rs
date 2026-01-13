use crate::intelligence::types::{ToolAction, ToolDefinition};
use serde_json::json;

pub fn to_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "researcher".to_string(),
        description: "Use this tool when the text needs fact-checking or data verification"
            .to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "search the internet for this paragraph's human or location"
                }
            },
            "required": ["query"]
        }),
        action: ToolAction::Research,
    }
}

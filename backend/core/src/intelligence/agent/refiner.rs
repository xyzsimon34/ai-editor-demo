use crate::intelligence::types::{ToolAction, ToolDefinition};
use serde_json::json;

pub fn to_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "refiner".to_string(),
        description: "Use this tool to improve, fix, or refine the text quality".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "text": {
                    "type": "string",
                    "description": "The text to refine"
                }
            },
            "required": ["text"]
        }),
        action: ToolAction::Refine,
    }
}

use crate::intelligence::types::McpTool;
use anyhow::Result;
use serde_json::json;

pub fn to_tool_definition() -> McpTool {
    McpTool {
        name: "refiner".to_string(),
        description: "Use this tool to improve, fix, or refine the text quality".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "text": {
                    "type": "string",
                    "description": "The text to refine"
                }
            },
            "required": ["text"]
        }),
    }
}

pub async fn execute_tool(text: &str, api_key: &str) -> Result<String> {
    use crate::refiner::processor;
    use crate::refiner::types::RefineInput;

    let input = RefineInput {
        content: text.to_string(),
    };
    let output = processor::call_improve_api(input, api_key).await?;
    Ok(output.content)
}

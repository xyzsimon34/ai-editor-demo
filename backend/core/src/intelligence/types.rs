use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize, Deserialize, Clone)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

pub fn get_sub_agent_definitions() -> Vec<McpTool> {
    vec![
        McpTool {
            name: "research_agent".to_string(),
            description: "Assigns a research task to a specialized sub-agent. Use this tool when you need to gather detailed information, search for background context, or synthesize data on a specific topic.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": { 
                        "type": "string", 
                        "description": "The specific research topic, question, or keywords to investigate." 
                    }
                },
                "required": ["query"]
            }),
        },
        McpTool {
            name: "refine_agent".to_string(),
            description: "Assigns a refinement task to a specialized sub-agent. Use this tool to improve text quality, fix grammatical errors, polish the tone, or adjust formatting for better readability.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "text": { 
                        "type": "string", 
                        "description": "The raw text or draft that needs to be refined or polished." 
                    }
                },
                "required": ["text"]
            }),
        },
    ]
}

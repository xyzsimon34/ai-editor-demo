mod refiner;
mod researcher;

use crate::intelligence::types::ToolDefinition;
use crate::model::Agent;

impl Agent {
    pub fn to_tool_definition(&self) -> ToolDefinition {
        match self {
            Agent::Researcher => researcher::to_tool_definition(),
            Agent::Refiner => refiner::to_tool_definition(),
        }
    }
}

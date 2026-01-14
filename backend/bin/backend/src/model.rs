use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RefineAction {
    Longer,
    Shorter,
    Fix,
    Improve,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RefineRequest {
    pub text: String,
    pub action: RefineAction,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RefineResponse {
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentTriggerRequest {
    #[serde(default)]
    pub role: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentTriggerResponse {
    pub ok: bool,
    pub result: Option<String>,
}

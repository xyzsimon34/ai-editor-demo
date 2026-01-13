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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Agent {
    Researcher,
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
pub struct PulseRequest {
    pub text: String,
    pub agents: Vec<Agent>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PulseResponse {
    /// 使用 HashMap 時，Serde 會自動將 Enum 轉為字串 Key
    pub suggestions: HashMap<Agent, String>,
}

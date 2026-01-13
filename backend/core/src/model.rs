use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Agent {
    Researcher,
    Refiner,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PulseInput {
    pub text: String,
    pub agents: Vec<Agent>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PulseOutput {
    pub suggestions: HashMap<Agent, String>,
}

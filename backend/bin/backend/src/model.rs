use serde::{Deserialize, Serialize};

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

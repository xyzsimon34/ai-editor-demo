use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct RefineRequest {
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RefineResponse {
    pub text: String,
}

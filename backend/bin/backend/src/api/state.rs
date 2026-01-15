use crate::{
    graphql::AppSchema,
    opts::{Decoder, Encoder},
};

use axum::extract::FromRef;
use backend_core::{editor, temporal::WorkflowEngine};
use serde::Deserialize;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::broadcast;
use yrs::Doc;

#[derive(Clone, FromRef)]
pub struct AppState {
    pub schema: AppSchema,
    pub wf_engine: WorkflowEngine,
    pub pg_pool: PgPool,
    pub jwt_encoder: Encoder,
    pub jwt_decoder: Decoder,
    pub api_key: String,
    pub editor_doc: Arc<Doc>,
    pub editor_broadcast_tx: broadcast::Sender<MessageStructure>,
    pub user_writing_state: Option<Arc<editor::UserWritingState>>,
}

impl AppState {
    pub fn new(
        schema: AppSchema,
        wf_engine: WorkflowEngine,
        pg_pool: PgPool,
        jwt_encoder: Encoder,
        jwt_decoder: Decoder,
        api_key: String,
        editor_doc: Arc<Doc>,
        editor_broadcast_tx: broadcast::Sender<MessageStructure>,
        user_writing_state: Option<Arc<editor::UserWritingState>>,
    ) -> Self {
        Self {
            schema,
            wf_engine,
            pg_pool,
            jwt_encoder,
            jwt_decoder,
            api_key,
            editor_doc,
            editor_broadcast_tx,
            user_writing_state,
        }
    }
}

#[derive(Clone, Debug)]
pub enum MessageStructure {
    // Lane A: The Y.js binary update
    YjsUpdate(Vec<u8>),
    // Lane B: A JSON string for UI commands (Comments, Toasts, etc)
    AiCommand(String),
}

#[derive(Clone, Debug, Deserialize)]
pub struct AiCommand {
    pub r#type: String,
    pub action: String,
    pub payload: Option<serde_json::Value>,
}

pub struct AiCommandPayload {
    pub role : String,
}

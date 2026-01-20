use crate::{
    graphql::AppSchema,
    opts::{Decoder, Encoder},
};

use axum::extract::FromRef;
use backend_core::temporal::WorkflowEngine;
use serde::Deserialize;
use sqlx::PgPool;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64},
};
use tokio::sync::broadcast;
use yrs::Doc;

#[derive(Clone)]
pub struct AppState {
    pub schema: AppSchema,
    pub wf_engine: WorkflowEngine,
    pub pg_pool: PgPool,
    pub jwt_encoder: Encoder,
    pub jwt_decoder: Decoder,
    pub api_key: String,
    pub editor_doc: Arc<Doc>,
    pub editor_broadcast_tx: broadcast::Sender<MessageStructure>,
    pub user_last_used_at: Arc<AtomicU64>,
    pub user_writing_timeout_ms: u64,
    pub linter_enabled: Arc<AtomicBool>,
    pub emoji_replacer_enabled: Arc<AtomicBool>,
    pub backseater_enabled: Arc<AtomicBool>,
}

// Manual FromRef implementations for fields that need extraction
impl FromRef<AppState> for Decoder {
    fn from_ref(state: &AppState) -> Self {
        state.jwt_decoder.clone()
    }
}

impl FromRef<AppState> for Encoder {
    fn from_ref(state: &AppState) -> Self {
        state.jwt_encoder.clone()
    }
}

impl FromRef<AppState> for AppSchema {
    fn from_ref(state: &AppState) -> Self {
        state.schema.clone()
    }
}

impl FromRef<AppState> for WorkflowEngine {
    fn from_ref(state: &AppState) -> Self {
        state.wf_engine.clone()
    }
}

impl FromRef<AppState> for PgPool {
    fn from_ref(state: &AppState) -> Self {
        state.pg_pool.clone()
    }
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
        user_writing_timeout_ms: u64,
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
            user_last_used_at: Arc::new(AtomicU64::new(0)),
            user_writing_timeout_ms,
            linter_enabled: Arc::new(AtomicBool::new(false)),
            emoji_replacer_enabled: Arc::new(AtomicBool::new(false)),
            backseater_enabled: Arc::new(AtomicBool::new(false)),
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
    pub payload: Option<AiCommandPayload>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct AgentPayload {
    pub role: String,
    pub mode: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum AiCommandPayload {
    Refiner(String),
    Agent(AgentPayload),
}

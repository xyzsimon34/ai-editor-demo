use crate::{
    graphql::AppSchema,
    opts::{Decoder, Encoder},
};

use axum::extract::FromRef;
use backend_core::temporal::WorkflowEngine;
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
    pub editor_broadcast_tx: broadcast::Sender<Vec<u8>>,
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
        editor_broadcast_tx: broadcast::Sender<Vec<u8>>,
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
        }
    }
}

use crate::api::{
    errors::Error,
    state::{AppState, MessageStructure},
};
use crate::model::{RefineRequest, RefineResponse};
use atb_ai_utils::agent::AgentContext;
use atb_types::Uuid;
use axum::{
    Router,
    extract::{Json, State},
    routing::post,
};
use backend_core::llm::new_linter;
use backend_core::refiner::processor::{
    call_fix_api, call_improve_api, call_longer_api, call_shorter_api,
};
use backend_core::refiner::types::{RefineInput, RefineOutput};
use std::pin::Pin;
use tracing::instrument;
use yrs::{ReadTxn, Transact};

pub type AgentCache = mini_moka::sync::Cache<Uuid, (String, AgentContext)>;

type RefineFuture<'a> =
    Pin<Box<dyn std::future::Future<Output = anyhow::Result<RefineOutput>> + Send + 'a>>;

pub fn routes() -> Router<AppState> {
    Router::new()
        // refine API
        .route("/improve", post(improve_text_handler))
        .route("/fix", post(fix_text_handler))
        .route("/longer", post(longer_text_handler))
        .route("/shorter", post(shorter_text_handler))
        .route("/linter", post(linter_text_handler))
}

// refine by single task
async fn handle_refine_request<F>(
    state: &AppState,
    req: RefineRequest,
    refine_fn: F,
) -> Result<Json<RefineResponse>, Error>
where
    F: for<'a> FnOnce(RefineInput, &'a str) -> RefineFuture<'a>,
{
    let input = RefineInput { content: req.text };
    refine_fn(input, &state.api_key)
        .await
        .map(|result| {
            Json(RefineResponse {
                text: result.content,
            })
        })
        .map_err(|e| {
            tracing::error!("Refine failed: {:?}", e);
            Error::InvalidInput(e.to_string())
        })
}

/// Improve text quality and clarity.
#[instrument(skip(state, req))]
pub async fn improve_text_handler(
    State(state): State<AppState>,
    Json(req): Json<RefineRequest>,
) -> Result<Json<RefineResponse>, Error> {
    handle_refine_request(&state, req, |input, key| {
        Box::pin(call_improve_api(input, key))
    })
    .await
}

/// Fix grammar and spelling errors in text.
#[instrument(skip(state, req))]
pub async fn fix_text_handler(
    State(state): State<AppState>,
    Json(req): Json<RefineRequest>,
) -> Result<Json<RefineResponse>, Error> {
    handle_refine_request(&state, req, |input, key| Box::pin(call_fix_api(input, key))).await
}

/// Lengthen text while maintaining meaning.
#[instrument(skip(state, req))]
pub async fn longer_text_handler(
    State(state): State<AppState>,
    Json(req): Json<RefineRequest>,
) -> Result<Json<RefineResponse>, Error> {
    handle_refine_request(&state, req, |input, key| {
        Box::pin(call_longer_api(input, key))
    })
    .await
}

/// Shorten text while maintaining meaning.
#[instrument(skip(state, req))]
pub async fn shorter_text_handler(
    State(state): State<AppState>,
    Json(req): Json<RefineRequest>,
) -> Result<Json<RefineResponse>, Error> {
    handle_refine_request(&state, req, |input, key| {
        Box::pin(call_shorter_api(input, key))
    })
    .await
}

#[instrument(skip(state, _req))]
pub async fn linter_text_handler(
    State(state): State<AppState>,
    Json(_req): Json<RefineRequest>,
) -> Result<Json<RefineResponse>, Error> {
    tracing::info!(
        "Linter handler called, modifying document. Current subscribers: {}",
        state.editor_broadcast_tx.receiver_count()
    );

    // The linter modifies the document, which should trigger the observer
    // in mono.rs to automatically broadcast the update via WebSocket
    new_linter(&state.api_key, state.editor_doc.clone())
        .await
        .map_err(|e| {
            tracing::error!("Linter failed: {:?}", e);
            Error::InvalidInput(e.to_string())
        })?;

    // Manually encode and broadcast the update to ensure it's sent
    // (Observer might not trigger in async context, so we do it manually)
    let update = {
        let txn = state.editor_doc.transact();
        txn.encode_state_as_update_v1(&yrs::StateVector::default())
    };

    if let Err(e) = state
        .editor_broadcast_tx
        .send(MessageStructure::YjsUpdate(update.to_vec()))
    {
        tracing::warn!("Failed to manually broadcast linter update: {:?}", e);
    } else {
        tracing::info!(
            "✅ Manually broadcasted linter update to {} subscribers",
            state.editor_broadcast_tx.receiver_count()
        );
    }

    Ok(Json(RefineResponse {
        text: "".to_string(),
    }))
}

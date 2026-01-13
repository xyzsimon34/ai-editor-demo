use crate::api::{errors::Error, state::AppState};
use crate::model::{Agent, PulseRequest, PulseResponse};
use crate::model::{RefineRequest, RefineResponse};
use axum::{
    Router,
    extract::{Json, State},
    routing::{get, post},
};
use backend_core::refiner::processor::{
    call_fix_api, call_improve_api, call_longer_api, call_shorter_api,
};
use backend_core::refiner::types::RefineInput;
use std::collections::HashMap;
use tracing::instrument;

pub fn routes() -> Router<AppState> {
    Router::new()
        // refine API
        .route("/improve", post(improve_text_handler))
        .route("/fix", post(fix_text_handler))
        .route("/longer", post(longer_text_handler))
        .route("/shorter", post(shorter_text_handler))
        // agent API
        .route("/agent/pulse", post(agent_pulse_handler))
        .route("/agent/list", get(list_agents_handler))
}

// refine by single task
async fn handle_refine_request<F, Fut>(
    state: &AppState,
    req: RefineRequest,
    refine_fn: F,
) -> Result<Json<RefineResponse>, Error>
where
    F: FnOnce(RefineInput, String) -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<backend_core::refiner::types::RefineOutput>>,
{
    let input = RefineInput { content: req.text };
    refine_fn(input, state.api_key.clone())
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
    handle_refine_request(&state, req, call_improve_api).await
}

/// Fix grammar and spelling errors in text.
#[instrument(skip(state, req))]
pub async fn fix_text_handler(
    State(state): State<AppState>,
    Json(req): Json<RefineRequest>,
) -> Result<Json<RefineResponse>, Error> {
    handle_refine_request(&state, req, call_fix_api).await
}

/// Lengthen text while maintaining meaning.
#[instrument(skip(state, req))]
pub async fn longer_text_handler(
    State(state): State<AppState>,
    Json(req): Json<RefineRequest>,
) -> Result<Json<RefineResponse>, Error> {
    handle_refine_request(&state, req, call_longer_api).await
}

/// Shorten text while maintaining meaning.
#[instrument(skip(state, req))]
pub async fn shorter_text_handler(
    State(state): State<AppState>,
    Json(req): Json<RefineRequest>,
) -> Result<Json<RefineResponse>, Error> {
    handle_refine_request(&state, req, call_shorter_api).await
}

// agent API
#[instrument(skip(state, req))]
pub async fn agent_pulse_handler(
    State(state): State<AppState>,
    Json(req): Json<PulseRequest>,
) -> Result<Json<PulseResponse>, Error> {
    use backend_core::intelligence::Brain;
    use backend_core::model::PulseInput;

    // Convert bin/backend model to core model
    let core_req = PulseInput {
        text: req.text,
        agents: req
            .agents
            .iter()
            .map(|a| match a {
                Agent::Researcher => backend_core::model::Agent::Researcher,
                Agent::Refiner => backend_core::model::Agent::Refiner,
            })
            .collect(),
    };

    // Convert api_key to &'static str for Brain
    let api_key: &'static str = Box::leak(state.api_key.clone().into_boxed_str());

    let core_resp = Brain::evaluate_pulse(core_req, api_key).await;

    // Convert core model back to bin/backend model
    let suggestions: HashMap<Agent, String> = core_resp
        .suggestions
        .into_iter()
        .map(|(agent, text)| {
            let backend_agent = match agent {
                backend_core::model::Agent::Researcher => Agent::Researcher,
                backend_core::model::Agent::Refiner => Agent::Refiner,
            };
            (backend_agent, text)
        })
        .collect();

    Ok(Json(PulseResponse { suggestions }))
}

/// List all available agents.
#[instrument]
pub async fn list_agents_handler() -> Result<Json<Vec<Agent>>, Error> {
    Ok(Json(enum_iterator::all::<Agent>().collect()))
}

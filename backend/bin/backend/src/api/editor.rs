use crate::api::state::{AiCommand, AppState, MessageStructure};
use atb_ai_utils::agent::AgentContext;
use atb_types::Uuid;
use axum::{
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
    routing::get,
};
use backend_core::llm::new_composer;
use backend_core::refiner::processor::{
    call_fix_api, call_improve_api, call_longer_api, call_shorter_api,
};
use backend_core::refiner::types::RefineInput;
use futures::{sink::SinkExt, stream::StreamExt};
use std::{cmp::max, sync::atomic::Ordering};
use tokio::time::Instant;
use yrs::{ReadTxn, Transact, Update, updates::decoder::Decode};
pub type AgentCache = mini_moka::sync::Cache<Uuid, (String, AgentContext)>;

pub fn routes() -> axum::Router<AppState> {
    axum::Router::new().route("/ws", get(ws_handler))
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();

    // 1. ON CONNECT: Send the full document state immediately
    // (This ensures the user sees existing text, not just new updates)
    let full_state = {
        let txn = state.editor_doc.transact();
        txn.encode_state_as_update_v1(&yrs::StateVector::default())
    };
    if sender
        .send(Message::Binary(full_state.into()))
        .await
        .is_err()
    {
        return;
    }

    let linter_enabled = crate::http::LINTER_FLAG.load(std::sync::atomic::Ordering::Relaxed);
    let emoji_replacer_enabled = crate::http::EMOJI_REPLACER_FLAG.load(std::sync::atomic::Ordering::Relaxed);
    let backseater_enabled = crate::http::BACKSEATER_FLAG.load(std::sync::atomic::Ordering::Relaxed);

    let _ = sender.send(Message::Text(
        serde_json::json!({
            "type": "AI_STATUS",
            "status": "complete",
            "message": format!("Linter {}", if linter_enabled { "enabled" } else { "disabled" })
        })
        .to_string()
        .into(),
    )).await;
    
    let _ = sender.send(Message::Text(
        serde_json::json!({
            "type": "AI_STATUS",
            "status": "complete",
            "message": format!("Emoji replacer {}", if emoji_replacer_enabled { "enabled" } else { "disabled" })
        })
        .to_string()
        .into(),
    )).await;
    
    let _ = sender.send(Message::Text(
        serde_json::json!({
            "type": "AI_STATUS",
            "status": "complete",
            "message": format!("Backseater {}", if backseater_enabled { "enabled" } else { "disabled" })
        })
        .to_string()
        .into(),
    )).await;

    let mut rx = state.editor_broadcast_tx.subscribe();

    let mut send_task = tokio::spawn(async move {
        // rx.recv() now returns a SyncMessage
        while let Ok(msg) = rx.recv().await {
            let ws_msg = match msg {
                // Unpack Lane A -> Binary
                MessageStructure::YjsUpdate(data) => Message::Binary(data.into()),

                // Unpack Lane B -> Text
                MessageStructure::AiCommand(json_string) => Message::Text(json_string.into()),
            };

            if sender.send(ws_msg).await.is_err() {
                break;
            }
        }
    });

    let state_clone = state.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                // LANE A: Binary Sync (Existing)
                Message::Binary(data) => {
                    let now = Instant::now().elapsed().as_millis() as u64;

                    state_clone.user_last_used_at.store(
                        max(now, state_clone.user_last_used_at.load(Ordering::Relaxed)),
                        Ordering::Relaxed,
                    );
                    // 標記用戶正在寫入
                    let mut txn = state_clone.editor_doc.transact_mut();
                    if let Ok(update) = Update::decode_v1(&data) {
                        if let Err(e) = txn.apply_update(update) {
                            tracing::warn!("Failed to apply update: {:?}", e);
                        }
                    }
                }
                // LANE B: AI Commands
                Message::Text(text) => {
                    println!("Received command: {:?}", text);
                    if let Ok(cmd) = serde_json::from_str::<AiCommand>(&text) {
                        println!("Command: {:?}", cmd);
                        // CLONE STATE FOR THE ASYNC TASK
                        // We spawn a new thread/task so we don't block the websocket heartbeat
                        let state_for_task = state.clone();
                        let cmd_action = cmd.action.clone();
                        let cmd_payload = cmd.payload.clone();
                        
                        if cmd_action != "TOGGLE" {
                            let _ =
                                state_for_task
                                    .editor_broadcast_tx
                                    .send(MessageStructure::AiCommand(
                                        serde_json::json!({
                                            "type": "AI_STATUS",
                                            "status": "thinking",
                                            "message": "Polishing your text..."
                                        })
                                        .to_string(),
                                    ));
                        }
                        
                        tokio::spawn(async move {
                            match cmd_action.as_str() {
                                "IMPROVE" | "FIX" | "LONGER" | "SHORTER" => {
                                    tracing::info!("🤖 processing {}...", cmd_action);

                                    // Extract text from Refiner payload
                                    let content = match cmd_payload {
                                        Some(crate::api::state::AiCommandPayload::Refiner(
                                            text,
                                        )) => text,
                                        Some(crate::api::state::AiCommandPayload::Agent(_)) => {
                                            tracing::error!(
                                                "Refiner command received Agent payload"
                                            );
                                            delegate_to_frontend(
                                                &state_for_task,
                                                "AI_STATUS",
                                                "error",
                                                "Invalid payload type for refiner command",
                                            );
                                            return;
                                        }
                                        None => {
                                            tracing::error!(
                                                "No payload found for command: {:?}",
                                                cmd_action
                                            );
                                            delegate_to_frontend(
                                                &state_for_task,
                                                "AI_STATUS",
                                                "error",
                                                "No payload found for command",
                                            );
                                            return;
                                        }
                                    };

                                    // Create the input struct your existing processor expects
                                    let input = RefineInput { content };
                                    let api_key = &state_for_task.api_key;

                                    // Select the correct function based on action
                                    let result = match cmd_action.as_str() {
                                        "IMPROVE" => call_improve_api(input, api_key).await,
                                        "FIX" => call_fix_api(input, api_key).await,
                                        "LONGER" => call_longer_api(input, api_key).await,
                                        "SHORTER" => call_shorter_api(input, api_key).await,
                                        _ => return, // Should be unreachable
                                    };

                                    // 3. APPLY PHASE (Mutation)
                                    match result {
                                        Ok(output) => {
                                            delegate_to_frontend(
                                                &state_for_task,
                                                "AI_STATUS",
                                                "complete",
                                                &format!("Applied {}", cmd_action),
                                            );
                                            delegate_to_frontend(
                                                &state_for_task,
                                                "AI_RESULT",
                                                "complete",
                                                &output.content,
                                            );
                                        }
                                        Err(e) => {
                                            tracing::error!("❌ AI failed: {:?}", e);
                                            delegate_to_frontend(
                                                &state_for_task,
                                                "AI_STATUS",
                                                "error",
                                                &format!("AI failed: {:?}", e),
                                            );
                                        }
                                    }
                                }
                                "AGENT" => {
                                    tracing::info!("🤖 processing {}...", cmd_action);

                                    // Extract role from Agent payload
                                    let (role, mode) = match cmd_payload {
                                        Some(crate::api::state::AiCommandPayload::Agent(
                                            agent_payload,
                                        )) => (agent_payload.role, agent_payload.mode),
                                        Some(crate::api::state::AiCommandPayload::Refiner(_)) => {
                                            tracing::error!(
                                                "Agent command received Refiner payload"
                                            );
                                            delegate_to_frontend(
                                                &state_for_task,
                                                "AI_STATUS",
                                                "error",
                                                "Invalid payload type for agent command",
                                            );
                                            return;
                                        }
                                        None => {
                                            tracing::error!(
                                                "No payload found for command: {:?}",
                                                cmd_action
                                            );
                                            delegate_to_frontend(
                                                &state_for_task,
                                                "AI_STATUS",
                                                "error",
                                                "No payload found for command",
                                            );
                                            return;
                                        }
                                    };

                                    // 0. PRE-CHECK: Verify document has content structure
                                    if !backend_core::editor::write::is_field_populated(
                                        &state_for_task.editor_doc,
                                        "content",
                                    ) {
                                        tracing::warn!("Document has no content structure yet");
                                        delegate_to_frontend(
                                            &state_for_task,
                                            "AI_STATUS",
                                            "error",
                                            "Please start typing in the editor first. The AI agent needs existing content to work with.",
                                        );
                                        return;
                                    }

                                    // 1. AI PROCESSING PHASE
                                    // Create the input struct your existing processor expects
                                    let api_key = &state_for_task.api_key;
                                    let preview_mode = mode.as_deref() == Some("preview");

                                    // Select the correct function based on action
                                    let result: Result<String, anyhow::Error> = match cmd_action
                                        .as_str()
                                    {
                                        // #TODO: This should definitely be matching agent_payload's content to determine which agent to run. We only have one right now.
                                        "AGENT" => {
                                            let user_last_used_at =
                                                state_for_task.user_last_used_at.clone();

                                            match new_composer(
                                                api_key,
                                                &role,
                                                &state_for_task.editor_doc,
                                                user_last_used_at,
                                                state_for_task.user_writing_timeout_ms,
                                                preview_mode
                                            )
                                            .await
                                            {
                                                Ok(_) => {
                                                    // Manually encode and broadcast the update to ensure it's sent
                                                    // (Observer might not trigger reliably in async context)
                                                    let update = {
                                                        let txn = state_for_task.editor_doc.transact();
                                                        txn.encode_state_as_update_v1(&yrs::StateVector::default())
                                                    };
                                                    
                                                    if let Err(e) = state_for_task
                                                        .editor_broadcast_tx
                                                        .send(MessageStructure::YjsUpdate(update.to_vec()))
                                                    {
                                                        tracing::warn!("Failed to manually broadcast agent update: {:?}", e);
                                                    } else {
                                                        tracing::info!(
                                                            "✅ Manually broadcasted agent update to {} subscribers",
                                                            state_for_task.editor_broadcast_tx.receiver_count()
                                                        );
                                                    }
                                                    
                                                    Ok("Agent executed successfully".to_string())
                                                }
                                                Err(e) => {
                                                    // Check if it's the "no content" error and handle gracefully
                                                    let error_msg = e.to_string();
                                                    if error_msg.contains(
                                                        "Document has no content structure",
                                                    ) {
                                                        Err(anyhow::anyhow!(
                                                            "Document has no content structure yet. User needs to create content first."
                                                        ))
                                                    } else {
                                                        Err(anyhow::anyhow!(
                                                            "Agent failed: {}",
                                                            error_msg
                                                        ))
                                                    }
                                                }
                                            }
                                        }
                                        _ => return, // Should be unreachable
                                    };

                                    // 3. APPLY PHASE (Mutation)
                                    match result {
                                        Ok(output) => {
                                            // The agent modifies the doc directly via new_composer
                                            tracing::info!("✅ Applied AI changes via CRDT");
                                            delegate_to_frontend(
                                                &state_for_task,
                                                "AI_STATUS",
                                                "complete",
                                                &output,
                                            );
                                        }
                                        Err(e) => {
                                            let error_msg = e.to_string();
                                            // Provide user-friendly error messages
                                            let user_message: String = if error_msg
                                                .contains("Document has no content structure")
                                            {
                                                "Please start typing in the editor first. The AI agent needs existing content to work with.".to_string()
                                            } else if error_msg.contains("Agent failed: ") {
                                                // Extract a cleaner error message if possible
                                                error_msg
                                                    .strip_prefix("Agent failed: ")
                                                    .map(|s| s.to_string())
                                                    .unwrap_or(error_msg)
                                            } else {
                                                error_msg
                                            };

                                            tracing::warn!("❌ AI agent failed: {}", user_message);
                                            delegate_to_frontend(
                                                &state_for_task,
                                                "AI_STATUS",
                                                "error",
                                                &user_message,
                                            );
                                        }
                                    }
                                }
                                "TOGGLE" => {
                                    let content = match cmd_payload {
                                        Some(crate::api::state::AiCommandPayload::Refiner(
                                            text,
                                        )) => text,
                                        Some(crate::api::state::AiCommandPayload::Agent(_)) => {
                                            tracing::error!(
                                                "Refiner command received Agent payload"
                                            );
                                            delegate_to_frontend(
                                                &state_for_task,
                                                "AI_STATUS",
                                                "error",
                                                "Invalid payload type for refiner command",
                                            );
                                            return;
                                        }
                                        None => {
                                            tracing::error!(
                                                "No payload found for command: {:?}",
                                                cmd_action
                                            );
                                            delegate_to_frontend(
                                                &state_for_task,
                                                "AI_STATUS",
                                                "error",
                                                "No payload found for command",
                                            );
                                            return;
                                        }
                                    };
                                    match content.as_str() {
                                        "LINTER" => {
                                            tracing::info!("🤖 toggling linter...");
                                            let current = crate::http::LINTER_FLAG
                                                .load(std::sync::atomic::Ordering::Relaxed);
                                            crate::http::LINTER_FLAG.store(
                                                !current,
                                                std::sync::atomic::Ordering::Relaxed,
                                            );
                                            delegate_to_frontend(
                                                &state_for_task,
                                                "AI_STATUS",
                                                "complete",
                                                &format!(
                                                    "Linter {}",
                                                    if !current { "enabled" } else { "disabled" }
                                                ),
                                            );
                                        }
                                        "EMOJI_REPLACER" => {
                                            tracing::info!("🤖 toggling emoji replacer...");
                                            let current = crate::http::EMOJI_REPLACER_FLAG
                                                .load(std::sync::atomic::Ordering::Relaxed);
                                            crate::http::EMOJI_REPLACER_FLAG.store(
                                                !current,
                                                std::sync::atomic::Ordering::Relaxed,
                                            );
                                            delegate_to_frontend(
                                                &state_for_task,
                                                "AI_STATUS",
                                                "complete",
                                                &format!(
                                                    "Emoji replacer {}",
                                                    if !current { "enabled" } else { "disabled" }
                                                ),
                                            );
                                        }
                                        "BACKSEATER" => {
                                            tracing::info!("💬 toggling backseater...");
                                            let current = crate::http::BACKSEATER_FLAG
                                                .load(std::sync::atomic::Ordering::Relaxed);
                                            crate::http::BACKSEATER_FLAG.store(
                                                !current,
                                                std::sync::atomic::Ordering::Relaxed,
                                            );
                                            delegate_to_frontend(
                                                &state_for_task,
                                                "AI_STATUS",
                                                "complete",
                                                &format!(
                                                    "Backseater {}",
                                                    if !current { "enabled" } else { "disabled" }
                                                ),
                                            );
                                        }
                                        _ => {
                                            tracing::error!("Unknown toggle target: {}", content);
                                            delegate_to_frontend(
                                                &state_for_task,
                                                "AI_STATUS",
                                                "error",
                                                &format!("Unknown toggle target: {}", content),
                                            );
                                        }
                                    }
                                }
                                _ => {
                                    tracing::error!("Unknown command: {:?}", cmd_action);
                                }
                            }
                        });
                    }
                }
                _ => {}
            }
        }
    });

    // Keep connection alive until one side closes
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };
}

fn delegate_to_frontend(state: &AppState, command_type: &str, status: &str, message: &str) {
    let json_msg = serde_json::json!({
        "type": command_type,
        "status": status,
        "message": message
    })
    .to_string();
    
    match state.editor_broadcast_tx.send(MessageStructure::AiCommand(json_msg.clone())) {
        Ok(_) => {
            tracing::info!(
                "✅ Broadcasted {} message to {} subscribers: status={}, message={}",
                command_type,
                state.editor_broadcast_tx.receiver_count(),
                status,
                message
            );
        }
        Err(e) => {
            tracing::warn!("Failed to broadcast {} message: {:?}", command_type, e);
        }
    }
}

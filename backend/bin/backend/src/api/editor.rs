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
use std::time::Duration;
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

    // 2. Subscribe to server broadcasts
    let mut rx = state.editor_broadcast_tx.subscribe();

    // 3. Handle Incoming/Outgoing Tasks
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
                    // 標記用戶正在寫入
                    if let Some(user_state) = &state_clone.user_writing_state {
                        user_state.mark_user_writing();

                        // 設置定時器，自動清除標記
                        let user_state_clone = user_state.clone();
                        tokio::spawn(async move {
                            tokio::time::sleep(Duration::from_millis(
                                user_state_clone.writing_timeout_ms,
                            ))
                            .await;
                            user_state_clone.clear_user_writing();
                        });
                    }

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
                                    let role = match cmd_payload {
                                        Some(crate::api::state::AiCommandPayload::Agent(
                                            agent_payload,
                                        )) => agent_payload.role,
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
                                    if !backend_core::editor::write::has_content_structure(
                                        &state_for_task.editor_doc,
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

                                    // Select the correct function based on action
                                    let result: Result<String, anyhow::Error> = match cmd_action
                                        .as_str()
                                    {
                                        // #TODO: This should definitely be matching agent_payload's content to determine which agent to run. We only have one right now.
                                        "AGENT" => {
                                            // 獲取共享的 UserWritingState
                                            let Some(user_state) =
                                                &state_for_task.user_writing_state
                                            else {
                                                return delegate_to_frontend(
                                                    &state_for_task,
                                                    "AI_STATUS",
                                                    "error",
                                                    "User writing state not available",
                                                );
                                            };

                                            match new_composer(
                                                api_key,
                                                &role,
                                                &state_for_task.editor_doc,
                                                user_state,
                                            )
                                            .await
                                            {
                                                Ok(_) => {
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
                                        Ok(_output) => {
                                            // The agent modifies the doc directly via new_composer
                                            tracing::info!("✅ Applied AI changes via CRDT");
                                            delegate_to_frontend(
                                                &state_for_task,
                                                "AI_STATUS",
                                                "complete",
                                                "AI agent finished successfully",
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
                                        "LINT" => {
                                            tracing::info!("🤖 toggling linter...");
                                            crate::mono::LINTER_FLAG.store(!crate::mono::LINTER_FLAG.load(std::sync::atomic::Ordering::Relaxed), std::sync::atomic::Ordering::Relaxed);
                                        }
                                        _ => return, // Should be unreachable
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
    let _ = state.editor_broadcast_tx.send(MessageStructure::AiCommand(
        serde_json::json!({
            "type": command_type,
            "status": status,
            "message": message
        })
        .to_string(),
    ));
}

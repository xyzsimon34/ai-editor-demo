use crate::api::state::{AppState, MessageStructure};

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
};
use futures::{sink::SinkExt, stream::StreamExt};
use yrs::{ReadTxn, Transact, Update, updates::decoder::Decode};

pub fn routes() -> axum::Router<AppState> {
    axum::Router::new().route("/ws", get(ws_handler))
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
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
                MessageStructure::YjsUpdate(data) => {
                    Message::Binary(data.into())
                },
                
                // Unpack Lane B -> Text
                MessageStructure::AiCommand(json_string) => {
                    Message::Text(json_string.into())
                },
            };
    
            if let Err(e) = sender.send(ws_msg).await {
                break;
            }
        }
    });
    

    let state_clone = state.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Binary(data) = msg {
                // Client typed something -> Update Server Doc
                let mut txn = state_clone.editor_doc.transact_mut();
                if let Ok(update) = Update::decode_v1(&data) {
                    if let Err(e) = txn.apply_update(update) {
                        tracing::warn!("Failed to apply update: {:?}", e);
                    }
                }
            }
        }
    });

    // Keep connection alive until one side closes
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };
}


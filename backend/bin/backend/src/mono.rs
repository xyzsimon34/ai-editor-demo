use crate::{api::state::MessageStructure, http, opts::*};

use std::{sync::Arc, time::Duration};

use atb_cli_utils::AtbCli;
use backend_core::{sqlx_postgres, temporal};
use tokio::sync::broadcast;
use yrs::{Doc, Text, Transact, XmlFragment};

pub async fn run(
    db_opts: DatabaseOpts,
    http_opts: HttpOpts,
    worker_opts: WorkerOpts,
    opts: Opts,
) -> anyhow::Result<()> {
    let client_id = crate::Cli::client_id();
    let pg_pool = sqlx_postgres::connect_pg(&db_opts.postgres, 30, Some(&client_id)).await?;
    let client = temporal::try_connect_temporal(
        &worker_opts.temporal.temporal,
        &worker_opts.temporal.namespace,
        Duration::from_secs(30),
    )
    .await?;
    let http_client = client.clone();

    let task_queue = worker_opts.temporal.task_queue.clone();
    let worker_config = crate::worker::worker_config(&worker_opts)?;
    let worker_handle =
        std::thread::spawn(move || crate::worker::start_worker(client, worker_config));

    // Initialize the Yrs Document for collaborative editing
    // Start with empty fragment - y-prosemirror will handle structure automatically
    let doc = Arc::new(Doc::new());
    let _xml_fragment = doc.get_or_insert_xml_fragment("content");

    // Create Broadcast Channel (Server -> All Clients)
    let (broadcast_tx, _) = broadcast::channel::<MessageStructure>(100);

    // Setup Observer: When Yrs changes (by User OR AI), broadcast the delta
    let tx_clone = broadcast_tx.clone();
    let _sub = doc.observe_update_v1(move |_txn, update_event| {
        let update = update_event.update.to_vec();
        // Send binary update to all connected clients
        let _ = tx_clone.send(MessageStructure::YjsUpdate(update));
    });

    // Spawn "The AI Agent" (Ghost Writer Demo)
    let ai_doc = doc.clone();
    let test_clone = broadcast_tx.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(12)).await;
            tracing::info!("Sending AI COMMANDS YOU!");
            let _ = test_clone.send(MessageStructure::AiCommand("AI COMMANDS YOU!".to_string()));
        }
    });

    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(10)).await;
            tracing::info!("🤖 AI is writing...");
            let xml_fragment = ai_doc.get_or_insert_xml_fragment("content");
            let mut txn = ai_doc.transact_mut();
            
            // Wait for user to create content first (paragraph structure)
            // Then append AI text to the last paragraph
            let len = xml_fragment.len(&txn);
            if len > 0 {
                // Get the last element (should be a paragraph)
                if let Some(last_elem) = xml_fragment.get(&txn, len - 1) {
                    // Check if it's a paragraph element
                    if let yrs::types::xml::XmlOut::Element(para) = last_elem {
                        // Get the paragraph's tag name
                        let tag = para.tag();
                        if tag.as_ref() == "paragraph" {
                            // Try to find a text node in the paragraph and append to it
                            let para_len = para.len(&txn);
                            if para_len > 0 {
                                // Check the last child - if it's text, append to it
                                // First, get the text node (immutable borrow)
                                let text_node_opt = para.get(&txn, para_len - 1)
                                    .and_then(|child| {
                                        if let yrs::types::xml::XmlOut::Text(text_ref) = child {
                                            Some(text_ref)
                                        } else {
                                            None
                                        }
                                    });
                                
                                // Now use it mutably
                                if let Some(text_ref) = text_node_opt {
                                    // XmlTextRef should work like Text - try using Text trait methods
                                    // Get current length and insert at the end
                                    let current_len = text_ref.len(&txn);
                                    text_ref.insert(&mut txn, current_len, " [AI was here] ");
                                    tracing::info!("✅ AI appended text!");
                                } else {
                                    // Last child isn't text
                                    tracing::info!("⚠️ Last child is not text, would create new text node");
                                }
                            } else {
                                // Empty paragraph - would create text node
                                tracing::info!("⚠️ Empty paragraph, would create text node");
                            }
                        }
                    }
                }
            } else {
                tracing::info!("⏳ Waiting for user to create content first...");
            }
            
            // The observer above automatically catches this and updates the frontend!
        }
    });

    http::start_http(
        pg_pool,
        http_client,
        http_opts,
        task_queue,
        opts.openai_api_key,
        doc,
        broadcast_tx,
    )
    .await?;

    worker_handle
        .join()
        .map_err(|e| anyhow::anyhow!("worker thread panicked: {:?}", e))??;

    Ok(())
}

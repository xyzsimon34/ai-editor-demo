use crate::{api::state::MessageStructure, http, opts::*};

use std::{sync::Arc, sync::atomic::{AtomicBool, Ordering}, time::Duration};

use atb_cli_utils::AtbCli;
use backend_core::{editor, sqlx_postgres, temporal};
use tokio::sync::broadcast;
use yrs::Doc;

// Doc 讀寫操作已移至 backend_core::editor 模組
// Use AtomicBool for thread-safe flag access (no unsafe blocks needed)
pub static LINTER_FLAG: AtomicBool = AtomicBool::new(false);
pub static EMOJI_REPLACER_FLAG: AtomicBool = AtomicBool::new(true);

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

    // Create User Writing State for user writing detection
    let user_writing_state = Arc::new(editor::UserWritingState::new(2000)); // 2 second timeout

    // Setup Observer: When Yrs changes (by User OR AI), broadcast the delta
    let tx_clone = broadcast_tx.clone();
    let _sub = doc.observe_update_v1(move |_txn, update_event| {
        let update = update_event.update.to_vec();
        // Send binary update to all connected clients
        let _ = tx_clone.send(MessageStructure::YjsUpdate(update));
    });

    // Clone values before moving into the async task
    let api_key_for_task = opts.openai_api_key.clone();
    let doc_for_task = doc.clone();
    let mut before_content = "".to_string();
    tokio::spawn(async move {
        tracing::info!("🚀 Auto-linter task started, will check every 10 seconds");
        loop {
            tokio::time::sleep(Duration::from_secs(10)).await;

            // 先讀取當前文檔內容
            let current_content = editor::get_doc_content(&doc_for_task);

            if current_content.is_empty() {
                tracing::info!("🔍 Doc is empty, skipping checks");
                continue;
            }

            if before_content == current_content {
                tracing::info!("🔍 Doc is not changed, skipping checks");
                continue;
            }


            // Load the flag atomically (thread-safe, no unsafe block needed)
            if LINTER_FLAG.load(Ordering::Relaxed) {

                tracing::info!("🔍 AI is checking for grammar and vocabulary...");
                match backend_core::llm::new_linter(&api_key_for_task, doc_for_task.clone()).await {
                    Ok(_) => {
                        tracing::info!("✅ AI checked for grammar and vocabulary successfully");
                    }
                    Err(e) => {
                        tracing::error!("❌ AI failed to check for grammar and vocabulary: {:?}", e);
                    }
                }
            } else {
                tracing::debug!("Linter is disabled, skipping check");
            }

            if EMOJI_REPLACER_FLAG.load(Ordering::Relaxed) {
                tracing::info!("🔍 AI is suggesting emoji replacements...");
                match backend_core::llm::new_emoji_replacer(&api_key_for_task, &doc_for_task).await {
                    Ok(_) => {
                        tracing::info!("✅ AI applied emoji replacements successfully");
                    }
                    Err(e) => {
                        tracing::error!("❌ AI failed to apply emoji replacements: {:?}", e);
                    }
                }
            } else {
                tracing::debug!("Emoji replacer is disabled, skipping check");
            }

            before_content = current_content;
        }
    });

    // tokio::spawn(async move {
    //     loop {
    //         tokio::time::sleep(Duration::from_secs(10)).await;
    //         tracing::info!("🤖 AI is writing...");

    //         // 先讀取當前文檔內容
    //         let current_content = editor::get_doc_content(&ai_doc);

    //         if !current_content.is_empty() {
    //             tracing::info!("📄 Current document content: {}", current_content);
    //         }

    //         // 然後在同一個可寫事務中進行寫入操作
    //         let xml_fragment = ai_doc.get_or_insert_xml_fragment("content");
    //         let mut txn = ai_doc.transact_mut();

    //         // Wait for user to create content first (paragraph structure)
    //         // Then append AI text to the last paragraph
    //         let len = xml_fragment.len(&txn);
    //         if len == 0 {
    //             tracing::info!("⏳ Waiting for user to create content first...");
    //             continue;
    //         }

    //         // Get the last element (should be a paragraph)
    //         let Some(last_elem) = xml_fragment.get(&txn, len - 1) else {
    //             continue;
    //         };

    //         // Check if it's a paragraph element
    //         let yrs::types::xml::XmlOut::Element(para) = last_elem else {
    //             continue;
    //         };

    //         // Get the paragraph's tag name
    //         if para.tag().as_ref() != "paragraph" {
    //             continue;
    //         }

    //         // Try to find a text node in the paragraph and append to it
    //         let para_len = para.len(&txn);
    //         if para_len == 0 {
    //             tracing::info!("⚠️ Empty paragraph, would create text node");
    //             continue;
    //         }

    //         // Check the last child - if it's text, append to it
    //         let Some(yrs::types::xml::XmlOut::Text(text_ref)) = para.get(&txn, para_len - 1) else {
    //             tracing::info!("⚠️ Last child is not text, would create new text node");
    //             continue;
    //         };

    //         // Insert text at the end
    //         let current_len = text_ref.len(&txn);
    //         text_ref.insert(&mut txn, current_len, " [AI was here] ");
    //         tracing::info!("✅ AI appended text!");

    //         // The observer above automatically catches this and updates the frontend!
    //     }
    // });

    http::start_http(
        pg_pool,
        http_client,
        http_opts,
        task_queue,
        opts.openai_api_key,
        doc,
        broadcast_tx,
        Some(user_writing_state),
    )
    .await?;

    worker_handle
        .join()
        .map_err(|e| anyhow::anyhow!("worker thread panicked: {:?}", e))??;

    Ok(())
}

// 測試已移至 backend_core::editor 模組

use crate::{api::state::MessageStructure, http, opts::*};
use atb_cli_utils::AtbCli;
use atb_tokio_ext::shutdown_signal;
use backend_core::{editor, sqlx_postgres, temporal};
use std::time::Instant;
use std::{
    sync::Arc,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use tokio::sync::broadcast;
use tokio::sync::watch;
use yrs::Doc;
// Doc 讀寫操作已移至 backend_core::editor 模組
// Use AtomicBool for thread-safe flag access (no unsafe blocks needed)
pub static LINTER_FLAG: AtomicBool = AtomicBool::new(false);
pub static EMOJI_REPLACER_FLAG: AtomicBool = AtomicBool::new(false);
pub static BACKSEATER_FLAG: AtomicBool = AtomicBool::new(false);

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
    let (notify_tx, mut notify_rx) = watch::channel(Instant::now());

    // Setup Observer: When Yrs changes (by User OR AI), broadcast the delta
    let tx_clone = broadcast_tx.clone();
    let _sub = doc.observe_update_v1(move |_txn, update_event| {
        let update = update_event.update.to_vec();
        // Send binary update to all connected clients
        let _ = tx_clone.send(MessageStructure::YjsUpdate(update));
        let _ = notify_tx.send(Instant::now());
    });

    // Clone values before moving into the async task
    let api_key_for_task = opts.openai_api_key.clone();
    let doc_for_task = doc.clone();
    let broadcast_tx_for_task = broadcast_tx.clone();

    
    tokio::spawn(async move {
        tracing::info!("🚀 Smart Auto-linter started (Debounce: 5s)");
        let mut before_content = "".to_string();
        
        // Create shutdown signal inside the async task
        let mut shutdown = shutdown_signal();
        tokio::pin!(shutdown);
        
        // 核心邏輯：等待變動 -> 觸發 5 秒冷卻 -> 執行
        loop {
            tokio::select! {
                // Check for shutdown signal
                _ = &mut shutdown => {
                    tracing::info!("🛑 Shutdown signal received, stopping AI task");
                    break;
                }
                // Wait for document changes
                changed = notify_rx.changed() => {
                    if changed.is_err() {
                        tracing::info!("🔍 Notify RX channel closed, stopping AI task");
                        break;
                    }
                }
            }

            // Debounce loop: wait 5 seconds or until another change
            loop {
                let delay = tokio::time::sleep(std::time::Duration::from_secs(5));
                tokio::pin!(delay);

                tokio::select! {
                    // Check for shutdown signal
                    _ = &mut shutdown => {
                        tracing::info!("🛑 Shutdown signal received during debounce, stopping AI task");
                        return;
                    }
                    changed = notify_rx.changed() => {
                        if changed.is_err() { return; }
                        tracing::debug!("⌨️ User still typing, skipping checks");
                        continue;
                    }
                    _ = &mut delay => {
                        break;
                    }
                }
            }

            let linter_enabled = LINTER_FLAG.load(Ordering::Relaxed);
            let emoji_replacer_enabled = EMOJI_REPLACER_FLAG.load(Ordering::Relaxed);
            let backseater_enabled = BACKSEATER_FLAG.load(Ordering::Relaxed);

            let current_content = editor::get_doc_content(&doc_for_task);
            if current_content.is_empty() || current_content == before_content {
                tracing::info!("🔍 Doc is empty or not changed, skipping checks");
                continue;
            }

            if linter_enabled {
                tracing::info!("🤖 Calling AI Linter...");
                match backend_core::llm::new_linter(&api_key_for_task, doc_for_task.clone()).await {
                    Ok(_) => {
                        tracing::info!("✅ AI check successful");
                    }
                    Err(e) => tracing::error!("❌ AI check failed: {:?}", e),
                }
            }

            if emoji_replacer_enabled {
                tracing::info!("🤖 Calling AI Emoji Replacer...");
                match backend_core::llm::new_emoji_replacer(&api_key_for_task, &doc_for_task).await {
                    Ok(_) => {
                        tracing::info!("✅ AI emoji replacer successful");
                    }
                    Err(e) => tracing::error!("❌ AI emoji replacer failed: {:?}", e),
                }
            }

            if backseater_enabled {
                tracing::info!("💬 Calling AI Backseater...");
                match backend_core::llm::new_backseating_agent(&api_key_for_task, &doc_for_task).await {
                    Ok(comments) => {
                        if !comments.is_empty() {
                            tracing::info!("✅ Generated {} comments from backseater", comments.len());
                            // Send each comment to frontend via broadcast channel
                            for comment in comments {
                                let comment_json = serde_json::json!({
                                    "type": "COMMENT",
                                    "comment_on": comment.comment_on,
                                    "comment": comment.comment,
                                    "color_hex": comment.color_hex
                                });
                                if let Err(e) = broadcast_tx_for_task.send(MessageStructure::AiCommand(comment_json.to_string())) {
                                    tracing::warn!("Failed to broadcast backseater comment: {:?}", e);
                                }
                            }
                        } else {
                            tracing::info!("⚠️ No comments generated by backseater");
                        }
                    }
                    Err(e) => tracing::error!("❌ AI backseater failed: {:?}", e),
                }
            }

            // Update before_content AFTER all tools have run (or been skipped)
            before_content = editor::get_doc_content(&doc_for_task);
        }
        tracing::info!("🔌 Linter task exiting");
    });

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

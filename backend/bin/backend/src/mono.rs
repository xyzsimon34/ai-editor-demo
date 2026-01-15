use crate::{api::state::MessageStructure, http, opts::*};
use atb_cli_utils::AtbCli;
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
pub static LINTER_FLAG: AtomicBool = AtomicBool::new(true);

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

    let linter_enabled = LINTER_FLAG.load(Ordering::Relaxed);
    tracing::info!("🔍 LINTER_FLAG status: {}", linter_enabled);

    // Store subscription outside if block to keep observer alive
    let _linter_subscription = if linter_enabled {
        let (notify_tx, mut notify_rx) = watch::channel(Instant::now());
        let tx_clone_for_observer = broadcast_tx.clone();

        // Observer: 當 notify_tx 隨 sub 一起被 drop 時，背景任務會自動停止
        let sub = doc.observe_update_v1(move |_txn, update_event| {
            let update = update_event.update.to_vec();
            let _ = notify_tx.send(Instant::now());
            let _ = tx_clone_for_observer.send(MessageStructure::YjsUpdate(update));
        });

        let api_key_for_task = opts.openai_api_key.clone();
        let doc_for_task = doc.clone();

        tokio::spawn(async move {
            tracing::info!("🚀 Smart Auto-linter started (Debounce: 5s)");
            let mut before_content = String::new();

            // 核心邏輯：等待變動 -> 觸發 5 秒冷卻 -> 執行
            loop {
                if notify_rx.changed().await.is_err() {
                    break;
                }

                loop {
                    let delay = tokio::time::sleep(std::time::Duration::from_secs(5));
                    tokio::pin!(delay);

                    tokio::select! {
                        changed = notify_rx.changed() => {
                            if changed.is_err() { return; }
                            tracing::debug!("⌨️ User still typing, resetting timer...");
                            continue;
                        }
                        _ = &mut delay => {
                            break;
                        }
                    }
                }

                let current_content = editor::get_doc_content(&doc_for_task);
                if current_content.is_empty() || current_content == before_content {
                    continue;
                }

                tracing::info!("🤖 Calling AI Linter...");
                match backend_core::llm::new_linter(&api_key_for_task, doc_for_task.clone()).await {
                    Ok(_) => {
                        before_content = current_content;
                        tracing::info!("✅ AI check successful");
                    }
                    Err(e) => tracing::error!("❌ AI check failed: {:?}", e),
                }
            }
            tracing::info!("🔌 Linter task exiting");
        });

        Some(sub)
    } else {
        None
    };

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

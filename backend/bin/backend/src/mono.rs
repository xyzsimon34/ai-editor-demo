use crate::{api::state::MessageStructure, http, opts::*};

use std::{sync::Arc, time::Duration};

use atb_cli_utils::AtbCli;
use backend_core::{editor, sqlx_postgres, temporal};
use tokio::sync::broadcast;
use yrs::{Doc, Transact, XmlFragment};

// Doc 讀寫操作已移至 backend_core::editor 模組

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

    // Spawn "The AI Agent" (Ghost Writer Demo)
    let test_clone = broadcast_tx.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(12)).await;
            tracing::info!("Sending AI COMMANDS YOU!");
            let _ = test_clone.send(MessageStructure::AiCommand("AI COMMANDS YOU!".to_string()));
        }
    });

    // Spawn AI Agent with word-by-word writing
    let ai_doc_for_writing = doc.clone();
    let user_state = user_writing_state.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(10)).await;
            tracing::info!("🤖 AI is writing...");

            // 先讀取當前文檔內容
            let current_content = editor::get_doc_content(&ai_doc_for_writing);

            if !current_content.is_empty() {
                tracing::info!("📄 Current document content: {}", current_content);
            }

            // 檢查文檔是否有內容結構
            {
                let xml_fragment = ai_doc_for_writing.get_or_insert_xml_fragment("content");
                let txn = ai_doc_for_writing.transact();
                let len = xml_fragment.len(&txn);
                if len == 0 {
                    tracing::info!("⏳ Waiting for user to create content first...");
                    continue;
                }
            }

            // 檢查用戶是否正在寫入
            if !user_state.on_write() {
                tracing::info!("⏸️  User is writing, skipping AI append");
                continue;
            }

            // 預先準備單詞列表（立即執行，不等待寫入時機）
            let ai_text = " [AI was here] ";
            let words = editor::prepare_words(ai_text);

            if words.is_empty() {
                continue;
            }

            tracing::info!("📝 Prepared {} words for appending", words.len());

            // 逐字追加（帶用戶寫入檢測）
            match editor::append_ai_content_word_by_word(
                &ai_doc_for_writing,
                words,
                100, // 100ms delay between words
                &user_state,
            )
            .await
            {
                Ok(()) => {
                    tracing::info!("✅ AI finished appending words");
                }
                Err(e) => {
                    tracing::warn!("❌ AI append failed: {:?}", e);
                }
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
        Some(user_writing_state),
    )
    .await?;

    worker_handle
        .join()
        .map_err(|e| anyhow::anyhow!("worker thread panicked: {:?}", e))??;

    Ok(())
}

// 測試已移至 backend_core::editor 模組

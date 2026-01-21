use crate::{api, opts::*};

use std::{
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};

use crate::api::state::MessageStructure;
use atb_cli_utils::AtbCli;
use atb_tokio_ext::shutdown_signal;
use backend_core::{editor, sqlx_postgres, temporal};
use sqlx::PgPool;
use std::sync::atomic::Ordering;
use tokio::{net::TcpListener, sync::watch};

// Use AtomicBool for thread-safe flag access (no unsafe blocks needed)
pub static LINTER_FLAG: AtomicBool = AtomicBool::new(false);
pub static EMOJI_REPLACER_FLAG: AtomicBool = AtomicBool::new(false);
pub static BACKSEATER_FLAG: AtomicBool = AtomicBool::new(false);

pub async fn run(
    db_opts: DatabaseOpts,
    http_opts: HttpOpts,
    temporal_opts: TemporalOpts,
    opts: Opts,
) -> anyhow::Result<()> {
    let client_id = crate::Cli::client_id();
    let pg_pool = sqlx_postgres::connect_pg(&db_opts.postgres, 30, Some(&client_id)).await?;
    let client = temporal::try_connect_temporal(
        &temporal_opts.temporal,
        &temporal_opts.namespace,
        Duration::from_secs(30),
    )
    .await?;

    // Initialize the Yrs Document for collaborative editing
    // Start with empty fragment - y-prosemirror will handle structure automatically
    let doc = std::sync::Arc::new(yrs::Doc::new());
    let _xml_fragment = doc.get_or_insert_xml_fragment("content");

    start_http(
        pg_pool,
        client,
        http_opts,
        temporal_opts.task_queue,
        opts.openai_api_key,
        opts.user_writing_timeout_ms,
    )
    .await
}

pub async fn start_http(
    pg_pool: PgPool,
    client: temporal::TemporalClient,
    http_opts: HttpOpts,
    task_queue: String,
    api_key: String,
    user_writing_timeout_ms: u64,
) -> anyhow::Result<()> {
    // Create User Writing State for user writing detection
    let (notify_tx, mut notify_rx) = watch::channel(Instant::now());

    let doc = std::sync::Arc::new(yrs::Doc::new());

    // Setup Observer: When Yrs changes (by User OR AI), broadcast the delta
    let (broadcast_tx, _) = tokio::sync::broadcast::channel::<MessageStructure>(100);

    // Clone for Yjs observer (needed in move closure)
    let broadcast_tx_for_yjs = broadcast_tx.clone();

    let _sub = doc.observe_update_v1(move |_txn, update_event| {
        let update = update_event.update.to_vec();
        tracing::info!(
            "📡 Yjs document updated, broadcasting {} bytes to {} subscribers",
            update.len(),
            broadcast_tx_for_yjs.receiver_count()
        );
        // Send binary update to all connected clients
        let send_result = broadcast_tx_for_yjs.send(MessageStructure::YjsUpdate(update));
        if send_result.is_err() {
            tracing::warn!("⚠️ Failed to broadcast Yjs update (no subscribers?)");
        } else {
            tracing::info!(
                "✅ Yjs update broadcasted successfully to {} subscribers",
                broadcast_tx_for_yjs.receiver_count()
            );
        }
        let _ = notify_tx.send(Instant::now());
    });
    tracing::info!("👂 Yjs update observer registered and ready");

    // Clone values before moving into the async task
    let api_key_for_task = api_key.clone();
    let doc_for_task = doc.clone();
    let broadcast_tx_for_task = broadcast_tx.clone();

    // Start smart auto-check task (linter/emoji replacer/backseater)
    tokio::spawn(async move {
        tracing::info!("🚀 Smart Auto-linter started (Debounce: 5s)");
        let mut before_content = "".to_string();
        // 核心邏輯：等待變動 -> 觸發 5 秒冷卻 -> 執行
        loop {
            if notify_rx.changed().await.is_err() {
                tracing::error!("🔍 Notify RX changed error");
                break;
            }

            loop {
                let delay = tokio::time::sleep(std::time::Duration::from_secs(5));
                tokio::pin!(delay);

                tokio::select! {
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
                match backend_core::llm::run_linter(&api_key_for_task, doc_for_task.clone()).await {
                    Ok(_) => {
                        tracing::info!("✅ AI check successful");
                    }
                    Err(e) => tracing::error!("❌ AI check failed: {:?}", e),
                }
            }

            if emoji_replacer_enabled {
                tracing::info!("🤖 Calling AI Emoji Replacer...");
                match backend_core::llm::run_emoji_replacer(&api_key_for_task, &doc_for_task).await
                {
                    Ok(_) => {
                        tracing::info!("✅ AI emoji replacer successful");
                    }
                    Err(e) => tracing::error!("❌ AI emoji replacer failed: {:?}", e),
                }
            }

            if backseater_enabled {
                tracing::info!("💬 Calling AI Backseater...");
                match backend_core::llm::run_backseating(&api_key_for_task, &doc_for_task)
                    .await
                {
                    Ok(comments) => {
                        if !comments.is_empty() {
                            tracing::info!(
                                "✅ Generated {} comments from backseater",
                                comments.len()
                            );
                            // Send each comment to frontend via broadcast channel
                            for comment in comments {
                                let comment_json = serde_json::json!({
                                    "type": "COMMENT",
                                    "comment_on": comment.comment_on,
                                    "comment": comment.comment,
                                    "color_hex": comment.color_hex
                                });
                                if let Err(e) = broadcast_tx_for_task
                                    .send(MessageStructure::AiCommand(comment_json.to_string()))
                                {
                                    tracing::warn!(
                                        "Failed to broadcast backseater comment: {:?}",
                                        e
                                    );
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

    let wf_engine = temporal::WorkflowEngine::new(client, task_queue);
    let schema = crate::graphql::schema()
        .data(wf_engine.clone())
        .data(pg_pool.clone())
        .finish();
    let (jwt_encoder, jwt_decoder) = http_opts.load_jwt()?;
    let app_state = api::state::AppState::new(
        schema,
        wf_engine,
        pg_pool,
        jwt_encoder,
        jwt_decoder,
        api_key,
        doc,
        broadcast_tx,
        user_writing_timeout_ms,
    );

    tracing::info!("http listening on {}", http_opts.host);
    let app = api::build_app(&http_opts, app_state)?;
    let listener = TcpListener::bind(&http_opts.host).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    Ok(())
}

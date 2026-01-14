use crate::{http, opts::*};

use std::{sync::Arc, time::Duration};

use atb_cli_utils::AtbCli;
use backend_core::{sqlx_postgres, temporal};
use tokio::sync::broadcast;
use yrs::{Doc, Text, Transact};

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
    let doc = Arc::new(Doc::new());
    let text = doc.get_or_insert_text("content");
    {
        let mut txn = doc.transact_mut();
        text.insert(&mut txn, 0, "Welcome to the AI Editor.\n");
    }

    // Create Broadcast Channel (Server -> All Clients)
    let (broadcast_tx, _) = broadcast::channel(100);

    // Setup Observer: When Yrs changes (by User OR AI), broadcast the delta
    let tx_clone = broadcast_tx.clone();
    let _sub = doc.observe_update_v1(move |_txn, update_event| {
        let update = update_event.update.to_vec();
        // Send binary update to all connected clients
        let _ = tx_clone.send(update);
    });

    // Spawn "The AI Agent" (Ghost Writer Demo)
    let ai_doc = doc.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(10)).await;
            tracing::info!("🤖 AI is writing...");
            let text = ai_doc.get_or_insert_text("content");
            let mut txn = ai_doc.transact_mut();
            text.push(&mut txn, " [AI was here] ");
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

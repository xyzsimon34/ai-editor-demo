use crate::{api, opts::*};

use std::time::Duration;

use atb_cli_utils::AtbCli;
use atb_tokio_ext::shutdown_signal;
use backend_core::{sqlx_postgres, temporal};
use sqlx::PgPool;
use tokio::net::TcpListener;

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

    start_http(
        pg_pool,
        client,
        http_opts,
        temporal_opts.task_queue,
        opts.openai_api_key,
    )
    .await
}

pub async fn start_http(
    pg_pool: PgPool,
    client: temporal::TemporalClient,
    http_opts: HttpOpts,
    task_queue: String,
    api_key: String,
) -> anyhow::Result<()> {
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

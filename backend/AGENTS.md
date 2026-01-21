# Project Guide for AI Agents

This AGENTS.md file provides comprehensive guidance for AI agents working with the backend monorepo codebase.

## Directory Overview

- **bin/backend**  
  The Rust API server binary
- **core**  
  Shared business logic, domain types, and utilities used by the server and clients.
- **frontend/admin**  
  Next.js + React TypeScript admin panel for internal management.
- **frontend/app**  
  Next.js + React TypeScript user-facing app.

## Running Unit Tests

Some unit tests in `bin/server` require an active local database.  
Avoid using a blanket `cargo test` at the workspace root, as it will run DB-dependent tests that may fail without a running database.

Instead, run only your new tests by crate and test name:

```bash
# From the workspace root, run a single test in the server crate
cargo test -p core ${specific_test_name}

# Or go into bin/server and filter by name
cd core && cargo test ${specific_test_name}
```

## When you need to call tools from the shell, use this rubric

- Find Files: `fd`
- Find Text: `rg` (ripgrep)
- Find Code Structure: `ast-grep`
- Select among matches: pipe to `fzf`
- JSON: `jq`
- YAML/XML: `yq`

## Efficiency and Speed

**IMPORTANT**
Do NOT RUN LINTING, TESTS, BUILD, OR CODEGEN commands unless asked to do so.  Many are nuanced and require the whole docker compose environment to be available.
DO NOT MODIFY FILES THAT ARE CREATED BY CODEGEN such as gql.ts or graphql.ts, schema.graphql files
**IMPORTANT**

## Backend Coding Practices

- **Workspace layout**: Application logic is split between a binary crate under
  `bin/server` and a reusable library crate in `core`.
  - `core` holds domain models, database stores, and utilities.
  - `bin/server` wires these pieces together to expose GraphQL and REST APIs.
- Keep business logic out of the binary crate whenever possible. New APIs should
  call into functions in `core` instead of performing DB queries directly in the
  handlers.

### Naming Conventions

Follow Rust API Guidelines and project-specific patterns:

#### Service/Workflow Execution
- **`run_`** prefix: For functions that execute a complete agent service workflow
  - Orchestrates multiple steps: calling LLM APIs, processing results, modifying documents
  - Examples: `run_composer`, `run_linter`, `run_backseating`
  - ❌ Avoid: `new_*` (misleading - suggests constructor), `execute_*` (less clear about full workflow)

#### Tool/Operation Execution  
- **`execute_`** prefix: For single tool operations
  - Examples: `tools::extender::execute_tool`, `tools::linter::execute_tool`

#### Getters
- **`get_`** prefix: For functions that retrieve data
  - Examples: `get_doc_content`, `get_text_refs_in_paragraph`
  - Note: Rust convention allows omitting `get_`, but we keep it for clarity

#### Boolean Checks
- **`is_`** prefix: For predicate functions returning `bool`
  - Examples: `is_user_writing`, `is_field_populated`, `is_block_level_element`

#### Action Verbs
- **Direct verbs**: For simple operations
  - Examples: `insert_ai_content_to_paragraph`, `append_ai_content_to_doc`, `apply_replacements`, `format_word_stream`

#### Handlers
- **`handle_`** prefix: For processing specific events/nodes
  - Examples: `handle_text_node`, `handle_element_node`

#### Extractors
- **`extract_`** prefix: For extracting data from structures
  - Examples: `extract_text_from_fragment`, `extract_text_from_node`

#### Conversions (Rust API Guidelines)
- **`as_`**: Low-cost, borrowing conversions (e.g., `as_str()`)
- **`to_`**: High-cost, usually allocates (e.g., `to_string()`)
- **`into_`**: Consumes original value, transfers ownership (e.g., `into_vec()`)

#### Constructors
- **`new()`**: Only for struct constructors that create instances
  - ❌ Never use `new_` prefix for functions that perform operations

#### Collectors
- **`collect_`** prefix: For gathering multiple items
  - Examples: `collect_text_nodes`, `collect_text_nodes_from_elem`

#### Connection/Setup
- **Direct verbs**: For connection and setup operations
  - Examples: `connect_pg`, `migrate`

### Database Queries

- **Simple reads/writes** are wrapped in helper functions that accept a
  `PgPool` and map the result into your domain struct:

  ```rust
  #[derive(sqlx::FromRow)]
  pub struct Company { id: Uuid, company_name: Option<String> }

  pub async fn get_company(pool: &PgPool, id: Uuid) -> Result<Company, sqlx::Error> {
      sqlx::query_as::<_, Company>(
          "SELECT id, company_name FROM companies WHERE id = $1",
      )
      .bind(id)
      .fetch_one(pool)
      .await
  }
  ```

- **Transactional operations** are run within a `Transaction<'_, Postgres>` so
  multiple statements succeed or fail together:

  ```rust
  let mut tx = pool.begin().await?;
  insert_conversation(tx.as_mut(), &conversation).await?;
  insert_ai_interaction(tx.as_mut(), &interaction).await?;
  tx.commit().await?;
  ```

- **Reusable queries** that should work with either a `PgPool` or a transaction
  take a generic `PgExecutor` argument. Define the trait alias once and re-use
  it for functions that may be called in both contexts:

  ```rust
  use sqlx::{Executor, Postgres};

  /// An alias for `Executor<'_, Database = Postgres>`.
  pub trait PgExecutor<'c>: Executor<'c, Database = Postgres> {}
  impl<'c, T: Executor<'c, Database = Postgres>> PgExecutor<'c> for T {}

  pub async fn update_address_by_id<'c, T: PgExecutor<'c>>(
      conn: T,
      user_id: &i64,
      address: &str,
  ) -> Result<User, sqlx::Error> {
      sqlx::query(
          r#"
          UPDATE users SET
          address = $1
          WHERE id = $2
          RETURNING *
          "#,
      )
      .bind(address)
      .bind(user_id)
      .try_map(TryInto::try_into)
      .fetch_one(conn)
      .await
  }
  ```

- **Complex/optional filters** use [SeaQuery] with the `identity` enums in
  `core::store::identity` to dynamically build SQL, then `query_as_with` to
  execute:

  ```rust
  let (sql, values) = sea_query::Query::select()
      .columns(ALL_COMPANY_COLUMNS)
      .from(Companies::Table)
      .and_where_option(name_filter.map(|f| Expr::col((CompanyRegistry::Table,
          CompanyRegistry::CompanyName)).ilike(format!("%{}%", f))))
      .build_sqlx(PostgresQueryBuilder);

  let companies = sqlx::query_as_with(&sql, values).fetch_all(&pool).await?;
  ```

### Migrations

- SQL migration files live under `core/migrations` and are executed using
  `sqlx::migrate!()` through the helper in `core::sqlx_postgres::migrate`.
- Create new migrations by adding a timestamped `.sql` file to that directory.

### Temporal Workflows

We register activities with the helper macros re-exported from `atb_temporal_ext`. Import them with `use atb_temporal_ext::{activity, local_activity};`, annotate each activity, and then call the generated `bind_*` helpers instead of manual `worker.register_activity` calls.

#### Running a Worker and Starting a Workflow

```rust
use std::sync::Arc;

use temporalio_sdk::{Worker, sdk_client_options};
use temporalio_sdk_core::{init_worker, CoreRuntime, RuntimeOptions, Url};
use temporalio_common::telemetry::TelemetryOptions;
use temporalio_common::worker::WorkerConfig;
use temporalio_client::WorkflowOptions;
use uuid::Uuid;
use crate::activities; // generated bind_* helpers live here

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Connect client
    let client = sdk_client_options(Url::parse("http://localhost:7233")?)
        .build()
        .connect("default", None)
        .await?;

    // Core runtime & worker
    let telemetry = TelemetryOptions::builder().build();
    let runtime = CoreRuntime::new_assume_tokio(
        RuntimeOptions::builder()
            .telemetry_options(telemetry)
            .build()
            .expect("runtime opts"),
    )
    .expect("runtime");
    let worker_cfg = WorkerConfig::builder()
        .namespace("default".to_string())
        .task_queue("default".to_string())
        .build()
        .expect("worker cfg");
    let core_worker = Arc::new(init_worker(&runtime, worker_cfg, client.clone())?);
    let mut worker = Worker::new_from_core(core_worker, "default");

    // Register activities generated by #[activity]/#[local_activity]
    activities::bind_say_hello_activity(&mut worker);
    activities::bind_add_numbers_activity(&mut worker);

    // Register workflow
    worker.register_wf("example_workflow", crate::example_workflow);

    // Run worker in background
    tokio::spawn(async move {
        worker.run().await.unwrap();
    });

    // Start workflow
    let wf_id = format!("wf-{}", Uuid::new_v4());
    let run = client
        .start_workflow(
            vec![],
            "default".into(),
            wf_id.clone(),
            "example_workflow".into(),
            None,
            WorkflowOptions::default(),
        )
        .await?;

    println!("Started workflow {} run_id={}", wf_id, run.run_id);

    let handle = client.get_untyped_workflow_handle(wf_id, run.run_id.clone());
    let result: String = handle.get_workflow_result(Default::default()).await?;
    println!("Workflow completed with: {}", result);

    Ok(())
}
```

#### Defining an Activity Function

```rust
use atb_temporal_ext::{activity, local_activity};
use temporalio_sdk::{ActContext, ActExitValue, ActivityError};
use temporalio_common::protos::{
    TaskToken,
    temporal::api::common::v1::{Payload, Payloads},
};

// Remote activity; macro generates bind_say_hello_activity
#[activity]
pub async fn say_hello_activity(
    _ctx: ActContext,
    name: String,
) -> Result<ActExitValue<String>, ActivityError> {
    Ok(format!("Hello, {}", name).into())
}

// Local activity (no server round-trip)
#[local_activity]
pub async fn add_numbers_activity(
    _ctx: ActContext,
    (a, b): (i32, i32),
) -> Result<ActExitValue<i32>, ActivityError> {
    Ok((a + b).into())
}

// Async completion: capture the task token now, finish later
#[activity]
pub async fn kick_off_async_job(
    ctx: ActContext,
    job_id: String,
) -> Result<ActExitValue<()>, ActivityError> {
    let token = ctx.get_info().task_token.clone(); // Vec<u8>
    // Persist the token in your own store keyed by job_id (DB/queue/etc.).
    persist_task_token(job_id, token)?;
    Ok(ActExitValue::WillCompleteAsync)
}

// Later, when the external work is done, complete by task token
pub async fn complete_async_job(
    task_token: Vec<u8>,
    result_json: &str,
    client: &temporalio_sdk_core::RetryClient<temporalio_sdk_core::Client>,
    namespace: &str,
) -> anyhow::Result<()> {
    let payload = Payload { data: result_json.as_bytes().to_vec(), metadata: Default::default() };
    let payloads = Payloads { payloads: vec![payload] };

    client
        .clone()
        .with_namespace(namespace.to_string())
        .complete_activity_task(TaskToken(task_token), Some(payloads))
        .await?;
    Ok(())
}
```

#### Defining a Workflow

The `#[activity]` / `#[local_activity]` macros generate lightweight builders you can call from a workflow context. You pass `&ctx` and the input, optionally tweak options, then `.await` the result:

```rust
use temporalio_sdk::{WfContext, WorkflowResult};

// Workflows take only `WfContext`. Read inputs from `ctx.get_args()` and
// deserialize using the blanket `FromJsonPayload` impl for all serde types.
#[derive(serde::Deserialize)]
struct YourInput { /* fields... */ }

pub async fn example_workflow(ctx: WfContext) -> WorkflowResult<()> {
    let _req: YourInput = ctx
        .get_args()
        .first()
        .ok_or(anyhow::anyhow!("missing input payload"))
        .and_then(|p| YourInput::from_json_payload(p).map_err(Into::into))?;

    // Remote activity with custom timeout
    let greeting: ActivityResult<String> = my_activities::kick_off_async_job(&ctx, "Ada".into())?
        .with_options(ActivityOptions {
            start_to_close_timeout: Some(Duration::from_secs(15)),
            ..Default::default()
        })
        .await?;

    // Local activity (no server round-trip) with its own options
    let sum: ActivityResult<i32> = my_activities::add_numbers_activity(&ctx, (2, 3))?
        .with_options(LocalActivityOptions {
            start_to_close_timeout: Some(Duration::from_secs(5)),
            ..Default::default()
        })
        .await?;

    tracing::info!(%greeting, %sum, "workflow work done");
    Ok(().into())
}
```

Notes

- Activities can either:
  - Return `Result<T, temporalio_sdk::ActivityError>` directly (recommended by the SDK), while still using `anyhow` internally via `?` because `ActivityError: From<anyhow::Error>`.
  - Or return `anyhow::Result<T>` and be wrapped at registration time to map errors into an `ActivityError`.
  - For explicit completion, return `ActExitValue::WillCompleteAsync`, persist `task_token`, and later call `complete_activity_task` with your result payload.

## Frontend Basics

- Frontend apps live under `frontend/` and use Next.js with TypeScript.
- UI components primarily come from shadcn (Radix UI). Complex client state can
  be managed with Zustand.

pub use atb_cli_utils::{
    AtbCli, BaseCli,
    clap::{self, Parser},
};

use crate::opts::{DatabaseOpts, HttpOpts, Opts, TemporalOpts, WorkerOpts};

#[derive(Parser, Debug)]
#[clap(
    name = "backend",
    rename_all = "kebab-case",
    rename_all_env = "screaming-snake"
)]
pub struct Cli {
    #[clap(flatten)]
    pub base: BaseCli,

    /// Tokio worker threads (optional override)
    #[arg(env = "BACKEND_WORKER_THREADS")]
    pub worker_threads: Option<usize>,

    /// Subcommands
    #[clap(subcommand)]
    pub subcommand: Commands,
}

#[derive(clap::Subcommand, Debug, Clone)]
pub enum Commands {
    /// Start the Monolithic HTTP server
    Mono {
        #[clap(flatten)]
        db_opts: DatabaseOpts,

        #[clap(flatten)]
        http: HttpOpts,

        #[clap(flatten)]
        worker: WorkerOpts,

        #[clap(flatten)]
        opts: Opts,
    },
    /// Run Temporal worker only (no HTTP server)
    Worker {
        #[clap(flatten)]
        worker: WorkerOpts,
    },
    /// Run Http Only
    Http {
        #[clap(flatten)]
        db_opts: DatabaseOpts,

        #[clap(flatten)]
        http: HttpOpts,

        #[clap(flatten)]
        temporal: TemporalOpts,

        #[clap(flatten)]
        opts: Opts,
    },
    /// Print the GraphQL schema SDL to stdout
    GenerateSchema,
}

impl Cli {
    pub fn create_runtime(
        worker_threads: Option<usize>,
    ) -> anyhow::Result<tokio::runtime::Runtime> {
        let mut builder = tokio::runtime::Builder::new_multi_thread();
        if let Some(n) = worker_threads {
            builder.worker_threads(n);
        }
        builder.enable_all().build().map_err(Into::into)
    }
}

impl AtbCli for Cli {
    fn name() -> String {
        env!("CARGO_PKG_NAME").to_owned()
    }
    fn version() -> String {
        env!("CARGO_PKG_VERSION").to_owned()
    }
    fn authors() -> Vec<String> {
        env!("CARGO_PKG_AUTHORS")
            .split(':')
            .map(str::to_string)
            .collect()
    }
    fn description() -> String {
        env!("CARGO_PKG_DESCRIPTION").to_owned()
    }
    fn repository() -> String {
        env!("CARGO_PKG_REPOSITORY").to_owned()
    }
    fn impl_version() -> String {
        env!("ATB_CLI_IMPL_VERSION").to_owned()
    }
    fn commit() -> String {
        env!("ATB_CLI_GIT_COMMIT_HASH").to_owned()
    }
    fn branch() -> String {
        env!("ATB_CLI_GIT_BRANCH").to_owned()
    }
    fn platform() -> String {
        env!("ATB_CLI_PLATFORM").to_owned()
    }
    fn rustc_info() -> String {
        env!("ATB_CLI_RUSTC_INFO").to_owned()
    }
}

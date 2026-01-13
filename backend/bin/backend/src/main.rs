pub mod api;
pub mod cli;
pub mod graphql;

pub mod http;
pub mod model;
pub mod mono;
pub mod opts;
pub mod worker;

use anyhow::Result;
use atb::logging::init_tracer;
use atb_cli_utils::AtbCli;

use crate::cli::{Cli, Commands};

fn main() -> Result<()> {
    let cli = Cli::parse();
    Cli::set_globals(&cli.base);

    match cli.subcommand {
        Commands::GenerateSchema => {
            println!("{}", graphql::schema().finish().sdl());
            Ok(())
        }
        Commands::Worker { worker } => {
            let _guard = init_tracer(Default::default()).expect("tracer setup succeeds. qed");
            let runtime = Cli::create_runtime(cli.worker_threads)?;
            runtime.block_on(async move { worker::run(worker).await })
        }
        Commands::Http {
            db_opts,
            http,
            temporal,
            opts,
        } => {
            let _guard = init_tracer(Default::default()).expect("tracer setup succeeds. qed");
            let runtime = Cli::create_runtime(cli.worker_threads)?;
            runtime.block_on(async move { http::run(db_opts, http, temporal, opts).await })
        }
        Commands::Mono {
            db_opts,
            http,
            worker,
            opts,
        } => {
            let _guard = init_tracer(Default::default()).expect("tracer setup succeeds. qed");
            let runtime = Cli::create_runtime(cli.worker_threads)?;
            runtime.block_on(async move { mono::run(db_opts, http, worker, opts).await })
        }
    }
}

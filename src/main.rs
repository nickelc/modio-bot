#![deny(rust_2018_idioms)]
#![deny(clippy::pedantic)]
#![allow(
    clippy::module_name_repetitions,
    clippy::similar_names,
    clippy::wildcard_imports
)]

use std::path::PathBuf;

use dotenv::dotenv;
use futures_util::StreamExt;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::EnvFilter;

mod bot;
mod commands;
mod config;
mod db;
mod error;
mod metrics;
mod tasks;
mod util;

use db::init_db;
use metrics::Metrics;
use util::*;

const HELP: &str = "\
🤖 modbot. modbot. modbot.

USAGE:
  modbot [-c <config>]

OPTIONS:
  -c <config>       Path to config file

ENV:
  MODBOT_DEBUG_TIMESTAMP        Start time as Unix timestamp for polling the mod events
";

#[tokio::main]
async fn main() {
    if let Err(e) = try_main().await {
        tracing::error!("{e}");
        std::process::exit(1);
    }
}

async fn try_main() -> CliResult {
    dotenv().ok();
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let mut args = pico_args::Arguments::from_env();
    if args.contains(["-h", "--help"]) {
        println!("{HELP}");
        std::process::exit(0);
    }

    let path = args
        .opt_value_from_os_str("-c", |s| PathBuf::try_from(s))?
        .unwrap_or_else(|| PathBuf::from("bot.toml"));

    let config = config::load_from_file(&path)
        .map_err(|e| format!("Failed to load config {path:?}: {e}"))?;

    let metrics = Metrics::new()?;
    let pool = init_db(&config.bot.database_url)?;
    let modio = init_modio(&config)?;

    let (cluster, mut events, context) =
        bot::initialize(&config, modio, pool, metrics.clone()).await?;

    if let Some(cmd) = args.subcommand()? {
        match cmd {
            cmd if cmd == "check" => {
                check_subscriptions(&context).await?;
            }
            cmd => {
                eprintln!("unknown subcommand: {cmd:?}");
            }
        }
        std::process::exit(0);
    }

    tokio::spawn(metrics::serve(&config.metrics, metrics));
    tokio::spawn(tasks::events::task(context.clone()));

    if let Some(token) = config.bot.dbl_token {
        tracing::info!("Spawning DBL task");
        let bot = context.application.id.get();
        let metrics = context.metrics.clone();
        tokio::spawn(tasks::dbl::task(bot, metrics, &token)?);
    }

    cluster.up().await;

    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to listen to ctrlc event.");
        tracing::info!("Shutting down cluster");
        cluster.down();
    });

    while let Some((_, event)) = events.next().await {
        let context = context.clone();
        tokio::spawn(bot::handle_event(event, context));
    }

    Ok(())
}

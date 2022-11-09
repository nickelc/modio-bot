#![deny(rust_2018_idioms)]
#![deny(clippy::pedantic)]
#![allow(
    clippy::module_name_repetitions,
    clippy::similar_names,
    clippy::wildcard_imports
)]

use dotenv::dotenv;
use futures_util::StreamExt;

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
        tracing::error!("{}", e);
        std::process::exit(1);
    }
}

async fn try_main() -> CliResult {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    let mut args = pico_args::Arguments::from_env();
    if args.contains(["-h", "--help"]) {
        println!("{}", HELP);
        std::process::exit(0);
    }

    let path = args
        .opt_value_from_str("-c")?
        .unwrap_or_else(|| String::from("bot.toml"));

    let config = config::load_from_file(&path)
        .map_err(|e| format!("Failed to load config {:?}: {}", path, e))?;

    let metrics = Metrics::new()?;
    let pool = init_db(&config.bot.database_url)?;
    let modio = init_modio(&config)?;

    tokio::spawn(metrics::serve(&config.metrics, metrics.clone()));

    let (cluster, mut events, context) = bot::initialize(&config, modio, pool, metrics).await?;

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

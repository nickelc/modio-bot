#![deny(rust_2018_idioms)]
#![deny(clippy::pedantic)]
#![allow(
    clippy::module_name_repetitions,
    clippy::similar_names,
    clippy::wildcard_imports
)]

use std::convert::Infallible;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use dotenv::dotenv;
use futures_util::future;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::EnvFilter;
use twilight_gateway::{CloseFrame, Event, Shard, StreamExt as _};

mod bot;
mod commands;
mod config;
mod db;
mod error;
mod metrics;
mod tasks;
mod util;

use bot::Context;
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

static SHUTDOWN: AtomicBool = AtomicBool::new(false);

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
        .opt_value_from_os_str("-c", |s| Ok::<_, Infallible>(PathBuf::from(s)))?
        .unwrap_or_else(|| PathBuf::from("bot.toml"));

    let config = config::load_from_file(&path)
        .map_err(|e| format!("Failed to load config {path:?}: {e}"))?;

    let metrics = Metrics::new()?;
    let pool = init_db(&config.bot.database_url)?;
    let modio = init_modio(&config)?;

    let (shards, context) = bot::initialize(&config, modio, pool, metrics.clone()).await?;

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

    tokio::spawn(metrics::serve(config.metrics, metrics));
    tokio::spawn(tasks::events::task(context.clone()));

    let mut senders = Vec::with_capacity(shards.len());
    let mut tasks = Vec::with_capacity(shards.len());

    for shard in shards {
        senders.push(shard.sender());
        tasks.push(tokio::spawn(runner(context.clone(), shard)));
    }

    tokio::signal::ctrl_c().await?;

    tracing::info!("Shutting down");
    SHUTDOWN.store(true, Ordering::Relaxed);

    for sender in senders {
        let _ = sender.close(CloseFrame::NORMAL);
    }

    future::join_all(tasks).await;

    Ok(())
}

async fn runner(context: Context, mut shard: Shard) {
    while let Some(event) = shard.next_event(bot::EVENTS).await {
        let event = match event {
            Ok(Event::GatewayClose(_)) if SHUTDOWN.load(Ordering::Relaxed) => break,
            Ok(event) => event,
            Err(source) => {
                tracing::warn!(?source, "error receiving event");
                continue;
            }
        };

        let context = context.clone();
        tokio::spawn(bot::handle_event(event, context));
    }
}

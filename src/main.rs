#![deny(rust_2018_idioms)]
#![deny(clippy::pedantic)]
#![allow(
    clippy::module_name_repetitions,
    clippy::similar_names,
    clippy::wildcard_imports
)]

use std::convert::Infallible;
use std::path::PathBuf;

use dotenv::dotenv;
use futures_util::future;
use tokio_stream::StreamExt;
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

use bot::Context;
use db::init_db;
use metrics::Metrics;
use twilight_gateway::stream::ShardEventStream;
use twilight_gateway::{CloseFrame, Shard};
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
        .opt_value_from_os_str("-c", |s| Ok::<_, Infallible>(PathBuf::from(s)))?
        .unwrap_or_else(|| PathBuf::from("bot.toml"));

    let config = config::load_from_file(&path)
        .map_err(|e| format!("Failed to load config {path:?}: {e}"))?;

    let metrics = Metrics::new()?;
    let pool = init_db(&config.bot.database_url)?;
    let modio = init_modio(&config)?;

    let (mut shards, context) = bot::initialize(&config, modio, pool, metrics.clone()).await?;

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

    let (tx, mut rx) = tokio::sync::watch::channel(false);

    let handle = tokio::spawn(async move {
        tokio::select! {
            () = gateway_runner(context, shards.iter_mut()) => {},
            _ = rx.changed() => {
                future::join_all(shards.iter_mut().map(|shard| async move {
                    shard.close(CloseFrame::NORMAL).await
                })).await;
            }
        }
    });

    tokio::signal::ctrl_c().await?;

    tracing::info!("Shutting down");
    tx.send(true)?;

    handle.await?;
    Ok(())
}

async fn gateway_runner(context: Context, shards: impl Iterator<Item = &mut Shard>) {
    let mut stream = ShardEventStream::new(shards);

    loop {
        let event = match stream.next().await {
            Some((_, Ok(event))) => event,
            Some((_, Err(source))) => {
                tracing::warn!(?source, "error receiving event");

                if source.is_fatal() {
                    break;
                }
                continue;
            }
            None => break,
        };

        let context = context.clone();
        tokio::spawn(bot::handle_event(event, context));
    }
}

//! ![MODBOT logo][logo]
//!
//! ![Rust version][rust-version]
//! ![Rust edition][rust-edition]
//! ![License][license-badge]
//!
//! MODBOT is a Discord bot for [mod.io] using [`modio-rs`] and [`serenity`].
//!
//!
//! [rust-version]: https://img.shields.io/badge/rust-1.31%2B-blue.svg
//! [rust-edition]: https://img.shields.io/badge/edition-2018-red.svg
//! [license-badge]: https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg
//! [logo]: https://raw.githubusercontent.com/nickelc/modio-bot/master/logo.png
//! [mod.io]: https://mod.io
//! [`modio-rs`]: https://github.com/nickelc/modio-rs
//! [`serenity`]: https://github.com/serenity-rs/serenity
#![deny(rust_2018_idioms)]

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;

use argh::FromArgs;
use dotenv::dotenv;

mod bot;
mod commands;
mod config;
mod db;
mod error;
mod tasks;
mod util;

use db::init_db;
use util::*;

/// 🤖 modbot. modbot. modbot.
#[derive(FromArgs)]
struct Arguments {
    /// path to config file.
    #[argh(option, short = 'c')]
    config: Option<String>,
}

#[tokio::main]
async fn main() {
    if let Err(e) = try_main().await {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}

async fn try_main() -> CliResult {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    let args: Arguments = argh::from_env();

    let path = args.config.unwrap_or_else(|| String::from("bot.toml"));
    let config = config::load_from_file(&path)
        .map_err(|e| format!("Failed to load config {:?}: {}", path, e))?;

    let pool = init_db(&config.bot.database_url)?;
    let modio = init_modio(&config)?;

    let (mut client, bot) = bot::initialize(&config, modio.clone(), pool.clone()).await?;

    tokio::spawn(tasks::events::task(&client, modio.clone()));

    if let Some(token) = config.bot.dbl_token {
        tracing::info!("Spawning DBL task");
        let cache = client.cache_and_http.cache.clone();
        tokio::spawn(tasks::dbl::task(bot, cache, &token)?);
    }

    client.start().await?;
    Ok(())
}

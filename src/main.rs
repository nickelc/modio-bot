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

use dotenv::dotenv;

mod bot;
mod commands;
mod config;
mod db;
mod error;
mod tasks;
mod tools;
mod util;

use util::*;

fn main() {
    if let Err(e) = try_main() {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}

fn try_main() -> CliResult {
    dotenv().ok();
    env_logger::init();

    let config = config::from_env()?;

    if tools::tools(&config) {
        return Ok(());
    }

    let (mut client, modio, rt, bot) = bot::initialize(config)?;

    rt.spawn(rt.enter(|| tasks::events::task(&client, modio.clone())));

    if let Ok(token) = util::var(config::DBL_TOKEN) {
        log::info!("Spawning DBL task");
        let cache = client.cache_and_http.cache.clone();
        rt.spawn(tasks::dbl::task(bot, cache, &token)?);
    }

    client.start()?;
    Ok(())
}

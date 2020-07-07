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

mod commands;
mod db;
mod error;
#[rustfmt::skip]
mod schema;
mod tasks;
mod tools;
mod util;

use util::*;

const DATABASE_URL: &str = "DATABASE_URL";
const DISCORD_BOT_TOKEN: &str = "DISCORD_BOT_TOKEN";
const DBL_TOKEN: &str = "DBL_TOKEN";
const DBL_OVERRIDE_BOT_ID: &str = "DBL_OVERRIDE_BOT_ID";
const MODIO_HOST: &str = "MODIO_HOST";
const MODIO_API_KEY: &str = "MODIO_API_KEY";
const MODIO_TOKEN: &str = "MODIO_TOKEN";

const DEFAULT_MODIO_HOST: &str = "https://api.mod.io/v1";

fn main() {
    if let Err(e) = try_main() {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}

fn try_main() -> CliResult {
    dotenv().ok();
    env_logger::init();

    if tools::tools() {
        return Ok(());
    }

    let (mut client, modio, rt, bot) = util::initialize()?;

    rt.spawn(rt.enter(|| tasks::events::task(&client, modio.clone())));

    if let Ok(token) = util::var(DBL_TOKEN) {
        log::info!("Spawning DBL task");
        let cache = client.cache_and_http.cache.clone();
        rt.spawn(tasks::dbl::task(bot, cache, &token)?);
    }

    client.start()?;
    Ok(())
}

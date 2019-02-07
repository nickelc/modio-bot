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

use dotenv::dotenv;
use modio::Modio;
use serenity::framework::standard::{help_commands, StandardFramework};
use serenity::prelude::*;
use tokio::runtime::Runtime;

#[macro_use]
mod macros;

mod commands;
mod util;

use commands::{Game, ListGames, ListMods};
use util::*;

const DISCORD_BOT_TOKEN: &str = "DISCORD_BOT_TOKEN";
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

    let token = var(DISCORD_BOT_TOKEN)?;

    let modio = {
        let host = var_or(MODIO_HOST, DEFAULT_MODIO_HOST)?;

        Modio::host(host, "modbot", credentials()?)
    };
    let rt = Runtime::new()?;

    let games_cmd = ListGames::new(modio.clone(), rt.executor());
    let game_cmd = Game::new(modio.clone(), rt.executor());
    let mods_cmd = ListMods::new(modio.clone(), rt.executor());

    let mut client = Client::new(&token, Handler)?;
    {
        let mut data = client.data.lock();
        data.insert::<GameKey>(Default::default());
    }

    client.with_framework(
        StandardFramework::new()
            .configure(|c| c.prefix("~").on_mention(true))
            .cmd("games", games_cmd)
            .cmd("game", game_cmd)
            .cmd("mods", mods_cmd)
            .help(help_commands::with_embeds),
    );
    client.start()?;
    Ok(())
}

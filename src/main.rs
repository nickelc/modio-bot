use dotenv::dotenv;
use modio::Modio;
use serenity::framework::standard::{help_commands, StandardFramework};
use serenity::prelude::*;
use tokio::runtime::Runtime;

mod commands;
mod util;

use commands::{Game, ListGames, ListMods};
use util::*;

const DISCORD_TOKEN: &str = "DISCORD_TOKEN";
const MODIO_HOST: &str = "MODIO_HOST";
const MODIO_API_KEY: &str = "MODIO_API_KEY";
const MODIO_TOKEN: &str = "MODIO_TOKEN";

const DEFAULT_MODIO_HOST: &str = "https://api.mod.io/v1";

fn main() -> CliResult {
    dotenv().ok();

    let token = var(DISCORD_TOKEN)?;

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
            .help(help_commands::plain),
    );
    client.start()?;
    Ok(())
}

use crate::util::{var, var_or};
use crate::Result;

const DATABASE_URL: &str = "DATABASE_URL";
const DISCORD_BOT_TOKEN: &str = "DISCORD_BOT_TOKEN";
pub const DBL_TOKEN: &str = "DBL_TOKEN";
pub const DBL_OVERRIDE_BOT_ID: &str = "DBL_OVERRIDE_BOT_ID";

const MODIO_HOST: &str = "MODIO_HOST";
const MODIO_API_KEY: &str = "MODIO_API_KEY";
const MODIO_TOKEN: &str = "MODIO_TOKEN";

const DEFAULT_MODIO_HOST: &str = "https://api.mod.io/v1";

pub struct Config {
    pub bot: BotConfig,
    pub modio: ModioConfig,
}

pub struct BotConfig {
    pub token: String,
    pub database_url: String,
}

pub struct ModioConfig {
    pub host: String,
    pub api_key: String,
    pub token: Option<String>,
}

pub fn from_env() -> Result<Config> {
    use std::env;
    use std::env::VarError::*;

    let bot = BotConfig {
        token: var(DISCORD_BOT_TOKEN)?,
        database_url: var(DATABASE_URL)?,
    };

    let host = var_or(MODIO_HOST, DEFAULT_MODIO_HOST)?;
    let api_key = env::var(MODIO_API_KEY);
    let token = env::var(MODIO_TOKEN);

    let (api_key, token) = match (api_key, token) {
        (Ok(key), Ok(token)) => (key, Some(token)),
        (Ok(key), _) => ((key), None),
        (Err(NotPresent), _) => {
            return Err("Environment variable 'MODIO_API_KEY' is required".into())
        }
        (Err(NotUnicode(_)), _) => {
            return Err("Environment variable 'MODIO_API_KEY' is not valid unicode".into())
        }
    };

    let modio = ModioConfig {
        host,
        api_key,
        token,
    };
    Ok(Config { bot, modio })
}

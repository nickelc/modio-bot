use std::future::Future;
use std::sync::Arc;

use modio::Modio;

mod discord;
mod server;
mod task;

use crate::config::Config;
use crate::db::{DbPool, Users};
use discord::{Client, OAuthConfig};

#[derive(Clone)]
struct Context {
    client: Client,
    config: Arc<OAuthConfig>,
    modio: Modio,
    users: Users,
}

pub fn start(
    config: Config,
    modio: Modio,
    pool: DbPool,
) -> (impl Future<Output = ()>, impl Future<Output = ()>) {
    let (addr, config, location) = {
        let addr = config.bot.oauth.addr;
        let location = config.bot.oauth.location_after_login;

        let config = Arc::new(OAuthConfig {
            client_id: config.bot.oauth.client_id,
            client_secret: config.bot.oauth.client_secret,
            auth_url: config.bot.oauth.auth_url,
            token_url: config.bot.oauth.token_url,
            redirect_uri: config.bot.oauth.redirect_uri,
            scope: "identify",
        });
        (addr, config, location)
    };

    let ctx = Context {
        client: Client::new(Arc::clone(&config)),
        config,
        modio,
        users: Users { pool },
    };

    (server::serve(addr, location, ctx.clone()), task::task(ctx))
}

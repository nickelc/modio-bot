use std::convert::{Infallible, TryFrom};
use std::sync::Arc;

use http::Uri;
use modio::auth::DiscordOptions;
use modio::Modio;
use serde::Deserialize;
use warp::redirect::temporary;
use warp::Filter;

use crate::config::Config;
use crate::db::{DbPool, NewToken, Users};

use super::discord;
use super::discord::{Client, OAuthConfig};

#[derive(Clone)]
struct Context {
    client: Client,
    config: Arc<OAuthConfig>,
    modio: Modio,
    users: Users,
}

#[derive(Deserialize)]
pub struct AuthCode {
    pub code: String,
}

pub async fn serve(config: Config, modio: Modio, pool: DbPool) {
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

    let login = warp::get()
        .and(warp::path!("login"))
        .and(with_context(ctx.clone()))
        .and_then(authorize);

    let callback = warp::get()
        .and(warp::path!("login" / "callback"))
        .and(with_context(ctx))
        .and(with_location(location))
        .and(warp::query())
        .and_then(handle_code);

    tracing::info!("Starting login endpoint: {}", addr);
    warp::serve(login.or(callback)).run(addr).await
}

async fn authorize(ctx: Context) -> Result<impl warp::Reply, Infallible> {
    let url = ctx.config.authorize_url();
    Ok(temporary(Uri::try_from(url.as_str()).unwrap()))
}

async fn handle_code(
    ctx: Context,
    location: Uri,
    code: AuthCode,
) -> Result<impl warp::Reply, warp::Rejection> {
    let token = ctx.client.request_token(code.code).await;
    let token = token.map_err(request_token_failed)?;

    let user = ctx.client.current_user(&token.access_token).await;
    let user = user.map_err(current_user_failed)?;

    let opts = DiscordOptions::new(&token.access_token);
    let creds = ctx.modio.auth().external(opts).await;
    let creds = creds.map_err(external_auth_failed)?;

    let new = NewToken {
        access_token: creds.token.unwrap().value,
        refresh_token: token.refresh_token,
        expires_in: token.expires_in,
    };

    ctx.users.save_token(user.id, new).map_err(db_error)?;

    Ok(temporary(location))
}

fn with_location(location: Uri) -> impl Filter<Extract = (Uri,), Error = Infallible> + Clone {
    warp::any().map(move || location.clone())
}

fn with_context(ctx: Context) -> impl Filter<Extract = (Context,), Error = Infallible> + Clone {
    warp::any().map(move || ctx.clone())
}

#[derive(Debug)]
enum Error {
    RequestDiscordToken(discord::Error),
    DiscordUser(discord::Error),
    ExternalAuth(modio::Error),
    Database(crate::db::Error),
}

impl warp::reject::Reject for Error {}

fn request_token_failed(e: discord::Error) -> warp::Rejection {
    warp::reject::custom(Error::RequestDiscordToken(e))
}

fn current_user_failed(e: discord::Error) -> warp::Rejection {
    warp::reject::custom(Error::DiscordUser(e))
}

fn external_auth_failed(e: modio::Error) -> warp::Rejection {
    warp::reject::custom(Error::ExternalAuth(e))
}

fn db_error(e: crate::db::Error) -> warp::Rejection {
    warp::reject::custom(Error::Database(e))
}

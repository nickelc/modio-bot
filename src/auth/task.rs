use std::time::Duration;

use modio::auth::DiscordOptions;
use tokio::time::Instant;

use super::Context;
use crate::db::NewToken;

const MIN: Duration = Duration::from_secs(60);
const INTERVAL_DURATION: Duration = Duration::from_secs(300);

pub(super) async fn task(ctx: Context) {
    let mut interval = tokio::time::interval_at(Instant::now() + MIN, INTERVAL_DURATION);

    loop {
        interval.tick().await;

        if let Ok(tokens) = ctx.users.tokens_to_refresh() {
            for (user_id, token) in tokens {
                match ctx.client.refresh_token(token).await {
                    Ok(token) => {
                        let opts = DiscordOptions::new(&token.access_token);
                        match ctx.modio.auth().external(opts).await {
                            Ok(creds) => {
                                let new = NewToken {
                                    access_token: creds.token.unwrap().value,
                                    refresh_token: token.refresh_token,
                                    expires_in: token.expires_in,
                                };
                                if let Err(e) = ctx.users.save_token(user_id, new) {
                                    eprintln!("save_token: {}", e);
                                }
                            }
                            Err(e) => eprintln!("external: {}", e),
                        }
                    }
                    Err(e) => eprintln!("refresh token: {:?}", e),
                }
            }
        }
    }
}

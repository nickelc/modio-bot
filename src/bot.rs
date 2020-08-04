use modio::auth::Credentials;
use modio::Modio;
use serenity::framework::standard::{DispatchError, StandardFramework};
use serenity::model::channel::Message;
use serenity::model::gateway::{Activity, Ready};
use serenity::model::guild::GuildStatus;
use serenity::prelude::*;
use tokio::runtime::Handle;
use tokio::runtime::Runtime;

use crate::commands::*;
use crate::config::Config;
use crate::db::{init_db, load_blocked, load_settings};
use crate::db::{DbPool, Settings, Subscriptions};
use crate::Result;

impl TypeMapKey for Settings {
    type Value = Settings;
}

impl TypeMapKey for Subscriptions {
    type Value = Subscriptions;
}

pub struct PoolKey;

impl TypeMapKey for PoolKey {
    type Value = DbPool;
}

pub struct ModioKey;

impl TypeMapKey for ModioKey {
    type Value = Modio;
}

pub struct ExecutorKey;

impl TypeMapKey for ExecutorKey {
    type Value = Handle;
}
pub struct Handler;

impl EventHandler for Handler {
    fn ready(&self, ctx: Context, ready: Ready) {
        let settings = {
            let data = ctx.data.read();
            let pool = data
                .get::<PoolKey>()
                .expect("failed to get connection pool");

            let guilds = ready.guilds.iter().map(GuildStatus::id).collect::<Vec<_>>();
            log::info!("Guilds: {:?}", guilds);

            let subs = data
                .get::<Subscriptions>()
                .expect("failed to get subscriptions");

            if let Err(e) = subs.cleanup(&guilds) {
                eprintln!("{}", e);
            }

            load_settings(&pool, &guilds).unwrap_or_default()
        };
        let mut data = ctx.data.write();
        data.get_mut::<Settings>()
            .expect("get settings failed")
            .data
            .extend(settings);

        let game = Activity::playing(&format!("~help| @{} help", ready.user.name));
        ctx.set_activity(game);
    }
}

fn dynamic_prefix(ctx: &mut Context, msg: &Message) -> Option<String> {
    let data = ctx.data.read();
    data.get::<Settings>()
        .map(|s| s.prefix(msg.guild_id))
        .flatten()
}

pub fn initialize(config: Config) -> Result<(Client, Modio, Runtime, u64)> {
    let rt = Runtime::new()?;
    let pool = init_db(&config.bot.database_url)?;
    let blocked = load_blocked(&pool)?;

    let modio = {
        let host = config.modio.host;
        let credentials = match (config.modio.api_key, config.modio.token) {
            (key, None) => Credentials::new(key),
            (key, Some(token)) => Credentials::with_token(key, token),
        };

        Modio::builder(credentials)
            .host(host)
            .user_agent("modbot")
            .build()?
    };

    let mut client = Client::new(&config.bot.token, Handler)?;

    let (bot, owners) = match client.cache_and_http.http.get_current_application_info() {
        Ok(info) => (info.id, vec![info.owner.id].into_iter().collect()),
        Err(e) => panic!("Couldn't get application info: {}", e),
    };

    client.with_framework(
        StandardFramework::new()
            .configure(|c| {
                c.prefix("~")
                    .dynamic_prefix(dynamic_prefix)
                    .on_mention(Some(bot))
                    .owners(owners)
                    .blocked_guilds(blocked.guilds)
                    .blocked_users(blocked.users)
            })
            .bucket("simple", |b| b.delay(1))
            .before(|_, msg, _| {
                log::debug!("cmd: {:?}: {:?}: {}", msg.guild_id, msg.author, msg.content);
                true
            })
            .group(&OWNER_GROUP)
            .group(if crate::tasks::dbl::is_dbl_enabled() { &with_vote::GENERAL_GROUP } else { &GENERAL_GROUP })
            .group(&MODIO_GROUP)
            .on_dispatch_error(|ctx, msg, error| match error {
                DispatchError::NotEnoughArguments { .. } => {
                    let _ = msg.channel_id.say(ctx, "Not enough arguments.");
                }
                DispatchError::LackingPermissions(_) => {
                    let _ = msg
                        .channel_id
                        .say(ctx, "You have insufficient rights for this command, you need the `MANAGE_CHANNELS` permission.");
                }
                DispatchError::Ratelimited(_) => {
                    let _ = msg.channel_id.say(ctx, "Try again in 1 second.");
                }
                e => eprintln!("Dispatch error: {:?}", e),
            })
            .help(&HELP),
    );

    {
        let mut data = client.data.write();
        data.insert::<PoolKey>(pool.clone());
        data.insert::<Settings>(Settings {
            pool: pool.clone(),
            data: Default::default(),
        });
        data.insert::<Subscriptions>(Subscriptions { pool });
        data.insert::<ModioKey>(modio.clone());
        data.insert::<ExecutorKey>(rt.handle().clone());
    }

    Ok((client, modio, rt, *bot.as_u64()))
}

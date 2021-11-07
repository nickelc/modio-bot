use modio::Modio;
use serenity::async_trait;
use serenity::framework::standard::macros::hook;
use serenity::framework::standard::StandardFramework;
use serenity::http::Http;
use serenity::model::channel::Message;
use serenity::model::gateway::{Activity, Ready};
use serenity::model::guild::GuildStatus;
use serenity::prelude::*;

use crate::commands::*;
use crate::config::Config;
use crate::db::{load_blocked, load_settings};
use crate::db::{DbPool, Settings, Subscriptions};
use crate::metrics::Metrics;
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

impl TypeMapKey for Metrics {
    type Value = Metrics;
}

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        let settings = {
            let data = ctx.data.read().await;
            let pool = data
                .get::<PoolKey>()
                .expect("failed to get connection pool");

            let guilds = ready.guilds.iter().map(GuildStatus::id).collect::<Vec<_>>();
            tracing::info!("Guilds: {:?}", guilds);

            let subs = data
                .get::<Subscriptions>()
                .expect("failed to get subscriptions");

            if let Err(e) = subs.cleanup(&guilds) {
                tracing::error!("{}", e);
            }

            load_settings(pool, &guilds).unwrap_or_default()
        };
        let mut data = ctx.data.write().await;
        data.get_mut::<Settings>()
            .expect("get settings failed")
            .data
            .extend(settings);

        let game = Activity::playing(&format!("~help| @{} help", ready.user.name));
        ctx.set_activity(game).await;
    }
}

use serenity::model::event::Event;

#[serenity::async_trait]
impl RawEventHandler for Handler {
    async fn raw_event(&self, ctx: Context, evt: Event) {
        match evt {
            Event::GuildCreate(_) => {
                let data = ctx.data.read().await;
                let metrics = data.get::<Metrics>().expect("get metrics failed");
                metrics.guilds.inc();
            }
            Event::GuildDelete(_) => {
                let data = ctx.data.read().await;
                let metrics = data.get::<Metrics>().expect("get metrics failed");
                metrics.guilds.dec();
            }
            _ => {}
        }
    }
}

#[hook]
async fn dynamic_prefix(ctx: &Context, msg: &Message) -> Option<String> {
    let data = ctx.data.read().await;
    data.get::<Settings>().and_then(|s| s.prefix(msg.guild_id))
}

pub async fn initialize(
    config: &Config,
    modio: Modio,
    pool: DbPool,
    metrics: Metrics,
) -> Result<(Client, u64)> {
    let blocked = load_blocked(&pool)?;

    let http = Http::new_with_token(&config.bot.token);

    let (bot, owners) = match http.get_current_application_info().await {
        Ok(info) => (info.id, vec![info.owner.id].into_iter().collect()),
        Err(e) => panic!("Couldn't get application info: {}", e),
    };

    let disabled = std::env::var("MODBOT_DISABLED_COMMANDS")
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .map(String::from)
        .collect();

    let framework = StandardFramework::new()
        .configure(|c| {
            c.prefix("~")
                .dynamic_prefix(dynamic_prefix)
                .on_mention(Some(bot))
                .owners(owners)
                .blocked_guilds(blocked.guilds)
                .blocked_users(blocked.users)
                .disabled_commands(disabled)
        })
        .bucket("simple", |b| b.delay(1))
        .await
        .before(before)
        .after(after)
        .group(&OWNER_GROUP)
        .group(&GENERAL_GROUP)
        .group(&BASIC_GROUP)
        .group(&SUBSCRIPTIONS_GROUP)
        .on_dispatch_error(dispatch_error)
        .help(&HELP);

    let client = Client::builder(&config.bot.token)
        .event_handler(Handler)
        .raw_event_handler(Handler)
        .framework(framework)
        .await?;
    {
        let mut data = client.data.write().await;
        data.insert::<PoolKey>(pool.clone());
        data.insert::<Settings>(Settings {
            pool: pool.clone(),
            data: Default::default(),
        });
        data.insert::<Subscriptions>(Subscriptions { pool });
        data.insert::<ModioKey>(modio);
        data.insert::<Metrics>(metrics);
    }

    Ok((client, *bot.as_u64()))
}

use std::sync::Arc;

use modio::Modio;
use twilight_cache_inmemory::{InMemoryCache, ResourceType};
use twilight_gateway::{stream, ConfigBuilder, EventTypeFlags, Intents, Shard};
use twilight_http::client::InteractionClient;
use twilight_http::Client;
use twilight_model::application::interaction::InteractionData;
use twilight_model::gateway::event::Event;
use twilight_model::gateway::payload::outgoing::update_presence::UpdatePresencePayload;
use twilight_model::gateway::presence::{ActivityType, MinimalActivity, Status};
use twilight_model::oauth::Application;

use crate::commands;
use crate::config::Config;
use crate::db::types::GuildId;
use crate::db::{DbPool, Settings, Subscriptions};
use crate::error::Error;
use crate::metrics::Metrics;

#[derive(Clone)]
pub struct Context {
    pub application: Application,
    pub client: Arc<Client>,
    pub cache: Arc<InMemoryCache>,
    pub modio: Modio,
    pub pool: DbPool,
    pub settings: Settings,
    pub subscriptions: Subscriptions,
    pub metrics: Metrics,
}

impl Context {
    pub fn interaction(&self) -> InteractionClient<'_> {
        self.client.interaction(self.application.id)
    }
}

pub async fn initialize(
    config: &Config,
    modio: Modio,
    pool: DbPool,
    metrics: Metrics,
) -> Result<(Vec<Shard>, Context), Error> {
    let client = Arc::new(Client::new(config.bot.token.clone()));
    let application = client.current_user_application().await?.model().await?;

    let interaction = client.interaction(application.id);
    commands::register(&interaction).await?;

    let presence = UpdatePresencePayload::new(
        [MinimalActivity {
            kind: ActivityType::Playing,
            name: "/help".into(),
            url: None,
        }
        .into()],
        false,
        None,
        Status::Online,
    )
    .expect("required activity is provided");

    let config = ConfigBuilder::new(config.bot.token.clone(), Intents::GUILDS)
        .event_types(
            EventTypeFlags::READY
                | EventTypeFlags::GUILD_CREATE
                | EventTypeFlags::GUILD_DELETE
                | EventTypeFlags::INTERACTION_CREATE,
        )
        .presence(presence)
        .build();

    let shards = stream::create_recommended(&client, config, |_, config| config.build())
        .await?
        .collect::<Vec<_>>();

    let cache = InMemoryCache::builder()
        .resource_types(ResourceType::USER_CURRENT)
        .build();

    let ctx = Context {
        application,
        client,
        cache: Arc::new(cache),
        modio,
        pool: pool.clone(),
        settings: Settings { pool: pool.clone() },
        subscriptions: Subscriptions { pool },
        metrics,
    };

    Ok((shards, ctx))
}

pub async fn handle_event(event: Event, context: Context) {
    context.cache.update(&event);

    match event {
        Event::Ready(ready) => {
            let guilds = ready
                .guilds
                .iter()
                .map(|g| GuildId(g.id))
                .collect::<Vec<_>>();
            tracing::info!("Guilds: {guilds:?}");
            context.metrics.guilds.set(ready.guilds.len() as u64);

            if let Err(e) = context.subscriptions.cleanup(&guilds) {
                tracing::error!("{e}");
            }
            let guilds = ready
                .guilds
                .into_iter()
                .map(|g| GuildId(g.id))
                .collect::<Vec<_>>();
            if let Err(e) = context.settings.cleanup(&guilds) {
                tracing::error!("{e}");
            }
        }
        Event::InteractionCreate(interaction) => match &interaction.data {
            Some(InteractionData::ApplicationCommand(command)) => {
                commands::handle_command(&context, &interaction, command).await;
            }
            Some(InteractionData::MessageComponent(component)) => {
                commands::handle_component(&context, &interaction, component).await;
            }
            _ => {}
        },
        _ => {}
    }
}

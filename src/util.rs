use std::time::{Duration, SystemTime, UNIX_EPOCH};

use modio::request::filter::Filter;
use modio::types::List;
use modio::Client;
use tokio_stream::{self as stream, StreamExt};
use twilight_http::api_error::ApiError;
use twilight_http::error::ErrorType;

use crate::bot::Context;
use crate::config::{Config, Host};
use crate::db::types::ChannelId;
use crate::error::Error;

pub mod text;

pub type CliResult = std::result::Result<(), Error>;
pub type Result<T> = std::result::Result<T, Error>;

pub fn init_modio(config: &Config) -> Result<Client> {
    let builder = match (&config.modio.api_key, &config.modio.token) {
        (key, None) => Client::builder(key.clone()),
        (key, Some(token)) => Client::builder(key.clone()).token(token.clone()),
    };

    let builder = match &config.modio.host {
        Some(Host::Default) => builder.use_default_env(),
        Some(Host::Test) => builder.use_test_env(),
        Some(Host::Dynamic) | None => builder.dynamic_game_host(),
        Some(Host::DynamicWithCustom(host)) => builder.dynamic_game_host_with_custom(&**host),
        Some(Host::Custom(host)) => builder.host(&**host),
    };
    let modio = builder.user_agent("modbot").build()?;
    Ok(modio)
}

pub fn is_unknown_channel_error(err: &ErrorType) -> bool {
    matches!(err,
        ErrorType::Response {
            error: ApiError::General(e),
            status,
            ..
        } if status.get() == 404 && e.code == 10003
    )
}

async fn get_unknown_channels(ctx: &Context) -> Result<Vec<ChannelId>> {
    let channels = ctx.subscriptions.get_channels()?;

    let requests = channels
        .into_iter()
        .map(|id| async move { (id, ctx.client.channel(*id).await) });

    let stream = stream::iter(requests).throttle(Duration::from_millis(40));

    tokio::pin!(stream);

    let mut unknown_channels = Vec::new();

    while let Some(fut) = stream.next().await {
        if let (channel, Err(e)) = fut.await {
            if is_unknown_channel_error(e.kind()) {
                unknown_channels.push(channel);
            } else {
                tracing::error!("unexpected error for channel {channel}: {e}");
            }
        }
    }

    Ok(unknown_channels)
}

pub trait IntoFilter {
    fn into_filter(self) -> Filter;
}

impl<T: AsRef<str>> IntoFilter for T {
    fn into_filter(self) -> Filter {
        fn search_filter(value: &str) -> Filter {
            use modio::request::filter::prelude::*;
            use modio::types::id::ResourceId;

            match value.parse::<ResourceId>() {
                Ok(id) => Id::eq(id),
                Err(_) => value
                    .strip_prefix('@')
                    .map_or_else(|| Fulltext::eq(value), NameId::eq),
            }
        }

        search_filter(self.as_ref())
    }
}

/// Helper trait as replacement for the old `Page<T>` type of the query loader.
pub trait Page {
    fn current(&self) -> usize;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
    fn page_count(&self) -> usize;
    fn page_size(&self) -> usize;
    fn total(&self) -> usize;
    //fn map_data(&self) -> impl Iterator;
}

impl<T> Page for List<T> {
    fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    fn len(&self) -> usize {
        self.data.len()
    }

    fn current(&self) -> usize {
        self.offset as usize / self.page_size() + 1
    }

    fn page_count(&self) -> usize {
        (self.total() - 1) / self.page_size() + 1
    }

    fn page_size(&self) -> usize {
        self.limit as usize
    }

    fn total(&self) -> usize {
        self.total as usize
    }

    //fn map_data(&self) -> impl Iterator {}
}

pub async fn check_subscriptions(ctx: &Context) -> Result<()> {
    let unknown_channels = get_unknown_channels(ctx).await?;

    tracing::info!("Found {} unknown channels", unknown_channels.len());

    ctx.subscriptions
        .cleanup_unknown_channels(&unknown_channels)?;
    Ok(())
}

pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

pub fn format_timestamp(seconds: i64) -> String {
    use time::format_description::FormatItem;
    use time::macros::format_description;
    use time::OffsetDateTime;

    const FMT: &[FormatItem<'_>] = format_description!("[year]-[month]-[day] [hour]:[minute]");

    if let Ok(v) = OffsetDateTime::from_unix_timestamp(seconds) {
        if let Ok(s) = v.format(&FMT) {
            return s;
        }
    }
    String::new()
}

// vim: fdm=marker

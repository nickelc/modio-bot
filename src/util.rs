use std::fmt;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use modio::filter::Filter;
use modio::{Credentials, Modio};
use tokio_stream::{self as stream, StreamExt};
use twilight_http::api_error::ApiError;
use twilight_http::error::ErrorType;

use crate::bot::Context;
use crate::config::Config;
use crate::db::types::ChannelId;
use crate::error::Error;

pub type CliResult = std::result::Result<(), Error>;
pub type Result<T> = std::result::Result<T, Error>;

pub fn init_modio(config: &Config) -> Result<Modio> {
    let credentials = match (&config.modio.api_key, &config.modio.token) {
        (key, None) => Credentials::new(key),
        (key, Some(token)) => Credentials::with_token(key, token),
    };

    let modio = Modio::builder(credentials)
        .host(&config.modio.host)
        .user_agent("modbot")
        .build()?;
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
            use modio::filter::prelude::*;

            match value.parse::<u32>() {
                Ok(id) => Id::eq(id),
                Err(_) => value
                    .strip_prefix('@')
                    .map_or_else(|| Fulltext::eq(value), NameId::eq),
            }
        }

        search_filter(self.as_ref())
    }
}

pub async fn check_subscriptions(ctx: &Context) -> Result<()> {
    let unknown_channels = get_unknown_channels(ctx).await?;

    tracing::info!("Found {} unknown channels", unknown_channels.len());

    ctx.subscriptions
        .cleanup_unknown_channels(&unknown_channels)?;
    Ok(())
}

#[derive(Debug)]
pub struct ContentBuilder {
    limit: usize,
    pub content: Vec<String>,
}

impl ContentBuilder {
    pub fn new(limit: usize) -> Self {
        Self {
            content: vec![],
            limit,
        }
    }
}

impl Default for ContentBuilder {
    fn default() -> Self {
        Self::new(2000)
    }
}

impl IntoIterator for ContentBuilder {
    type Item = String;
    type IntoIter = std::vec::IntoIter<String>;

    fn into_iter(self) -> Self::IntoIter {
        self.content.into_iter()
    }
}

impl fmt::Write for ContentBuilder {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        match self.content.last_mut() {
            Some(current) => {
                if current.len() + s.len() > self.limit {
                    self.content.push(String::from(s));
                } else {
                    current.push_str(s);
                }
            }
            None => {
                self.content.push(String::from(s));
            }
        };
        Ok(())
    }

    fn write_char(&mut self, c: char) -> fmt::Result {
        match self.content.last_mut() {
            Some(current) => {
                if current.len() + c.len_utf8() > self.limit {
                    self.content.push(c.to_string());
                } else {
                    current.push(c);
                }
            }
            None => self.content.push(c.to_string()),
        };
        Ok(())
    }
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

pub fn strip_html_tags<S: AsRef<str>>(input: S) -> String {
    use html5ever::tendril::TendrilSink;
    use html5ever::{parse_document, ParseOpts};
    use markup5ever_rcdom::{Node, NodeData, RcDom};

    fn fold_text(elem: &Node, out: &mut String) {
        if let NodeData::Text { contents } = &elem.data {
            out.push_str(&contents.borrow());
        }
        for child in &*elem.children.borrow() {
            fold_text(child, out);
        }
    }

    let dom = parse_document(RcDom::default(), ParseOpts::default()).one(input.as_ref());
    let mut out = String::new();
    fold_text(&dom.document, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::{strip_html_tags, ContentBuilder};
    use std::fmt::Write;

    #[test]
    fn content_builder() {
        let mut c = ContentBuilder::new(20);

        let _ = write!(&mut c, "{}", "foo".repeat(5));
        assert_eq!(c.content.len(), 1);

        let _ = write!(&mut c, "{}", "foo".repeat(5));
        assert_eq!(c.content.len(), 2);
        assert_eq!(c.content[0], "foo".repeat(5));
        assert_eq!(c.content[1], "foo".repeat(5));

        let _ = c.write_char('f');
        let _ = c.write_char('o');
        let _ = c.write_char('o');
        assert_eq!(c.content.len(), 2);
        assert_eq!(c.content[1], "foo".repeat(6));

        let _ = c.write_str("foobar");
        assert_eq!(c.content.len(), 3);
        assert_eq!(c.content[0], "foo".repeat(5));
        assert_eq!(c.content[1], "foo".repeat(6));
        assert_eq!(c.content[2], "foobar");
    }

    #[test]
    fn test_strip_html_tags() {
        let input = "aaa<br/>";
        assert_eq!("aaa", strip_html_tags(input));

        let input = "aaa<br/> bbb";
        assert_eq!("aaa bbb", strip_html_tags(input));

        let input = "- aaa\n- bbb\n - ccc\n";
        assert_eq!(input, strip_html_tags(input));

        let input = "<div>- aaa\n- bbb\n - ccc\n</div>";
        assert_eq!("- aaa\n- bbb\n - ccc\n", strip_html_tags(input));
    }
}

// vim: fdm=marker

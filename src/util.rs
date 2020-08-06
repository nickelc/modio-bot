use std::env;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::prelude::*;
use modio::{Credentials, Modio};
use serenity::client::Context;
use serenity::model::id::GuildId;

use crate::config::Config;
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

pub fn guild_stats(ctx: &mut Context) -> (usize, usize) {
    // ignore Discord Bot List server
    let dbl = GuildId(264_445_053_596_991_498);
    ctx.cache
        .read()
        .guilds
        .iter()
        .filter(|&(&id, _)| dbl != id)
        .fold((0, 0), |(count, sum), (_, guild)| {
            let guild = guild.read();
            (count + 1, sum + guild.members.len())
        })
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

pub fn format_timestamp(seconds: i64) -> impl fmt::Display {
    NaiveDateTime::from_timestamp(seconds, 0).format("%Y-%m-%d %H:%M")
}

pub fn strip_html_tags<S>(input: S) -> String
where
    S: AsRef<str>,
{
    use kuchiki::traits::*;

    kuchiki::parse_html().one(input.as_ref()).text_contents()
}

pub fn var(key: &'static str) -> Result<String> {
    env::var(key).map_err(|e| Error::Env(key, e))
}

pub fn var_or<S: Into<String>>(key: &'static str, default: S) -> Result<String> {
    match env::var(key) {
        Ok(v) => Ok(v),
        Err(env::VarError::NotPresent) => Ok(default.into()),
        Err(e) => Err(Error::Env(key, e)),
    }
}

#[cfg(test)]
mod tests {
    use super::ContentBuilder;
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
}

// vim: fdm=marker

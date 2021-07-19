<img src="https://raw.githubusercontent.com/nickelc/modio-bot/master/logo.png" width="200" align="right"/>

# ModBot for Discord
[![Crates.io][crates-badge]][crates-url]
![Rust version][rust-version]
![License][license-badge]
[![GitHub Action][gha-badge]][gha-url]
[![Discord][discord-badge]][discord]

ModBot is a Discord bot for [mod.io] using [`modio-rs`] and [`serenity`].

<p align="center">
    <a href="#setup">Setup</a> •
    <a href="#commands">Commands</a> •
    <a href="#screenshots">Screenshots</a> •
    <a href="#building">Building</a> •
    <a href="#installation">Installation</a> •
    <a href="#usage">Usage</a> •
    <a href="#license">License</a>
</p>

## Setup

You can invite the officially hosted ModBot to join your Discord server using the
following URL https://discordbot.mod.io, or you can build and install your
own version of ModBot by following the [instructions](#building) below.

 1. Invite the ModBot https://discordbot.mod.io/
 2. View the games list `~games` and set the default game `~game ID`
 3. In the channel(s) you want the bot to post updates (mod added / edited), run the command `~subscribe ID`
 4. Ensure the bot has `Read Messages`, `Send Messages` and `Embed Links` permissions in the channel(s) it is in to be able to function correctly

<img src="https://user-images.githubusercontent.com/2128532/118098374-1adc0e80-b3d4-11eb-808a-4024b7e79d9b.png" width="500"/>

## Commands

By default `~` is the prefix used to issue commands to ModBot. Once you have invited ModBot to your server, you can set the default game using the command `~game ID`. Now when a user issues the command `~mods`, all of the mods for the game you specified will be returned. You can change the default game at any time.

We recommend you also `~subscribe ID` to games you are interested in receiving push notifications from. For example in our [#bot channel][modio-bot-channel], we have subscribed to a bunch of games and whenever a mod is updated, the channel is notified.

Popular commands include:

 * `~help` show these commands
 * `~prefix CHARACTER` change the default prefix from `~` to something else
 * `~game <ID|Name>` set the default game
 * `~game` return information about the default game
 * `~games` return a list of all games
 * `~mod <ID|Name>` return information about the mod(s) requested
 * `~mods [ID|Name]` return a list of all mods belonging to the default game
 * `~popular` return a list of mods ordered by popularity
 * `~subscribe <ID|Name> [Tag..]` subscribe to a game for updates (mods added/edited) \[alias: `sub`\]
   ```
   ~sub 51
   ~sub xcom
   ~sub xcom "UFO Defense" Major
   ~sub "Skate XL" "Real World Spot"
   ~sub skate Gear Deck
   ```
 * `~subscriptions` see all games subscribed too \[alias: `subs`\]
 * `~unsubscribe <ID|Name> [Tag..]` unsubscribe from a game \[alias: `unsub`\]
   ```
   ~unsub 51
   ~unsub OpenXcom
   ~unsub xcom "UFO Defense" Major
   ~unsub "Skate XL" "Real World Spot"
   ~unsub skate Gear Deck
   ```
 * `~mute <Game> <Mod>` mute a mod from update notifications
 * `~muted` return a list of all muted mods
 * `~unmute <Game> <Mod>` unmute a mod from update notifications

## Screenshots

### Mod details
![details](https://user-images.githubusercontent.com/2128532/98248314-0de9e880-1f75-11eb-8598-add24e232cea.png)

### New Mod notification
![notification](https://user-images.githubusercontent.com/2128532/98248318-0e827f00-1f75-11eb-89d5-a55174d9fed5.png)

## Building

MODBOT is written in Rust, so you'll need to grab a [Rust installation][rust-lang] in order to compile it.
Building is easy:

```
$ git clone https://github.com/nickelc/modio-bot
$ cd modio-bot
$ cargo build --release
$ ./target/release/modbot
```

### Building with bundled sqlite3

Use the feature `sqlite-bundled` to compile sqlite3 from source and link against that.

```
$ cargo build --features sqlite-bundled
```

## Installation

### Cargo

```
$ cargo install --git https://github.com/nickelc/modio-bot
$ $HOME/.cargo/bin/modbot
```

## Usage

Create a `bot.toml` config file in the working directory.

```toml
[bot]
token="your discord bot token"
database_url="/path/to/sqlite.db"

[modio]
api_key="your mod.io api key"
```

A example is provided as [`bot.example.toml`](bot.example.toml).

#### Running the bot
```bash
./path/to/modbot

./path/to/modbot --config path/to/bot.toml
```

#### Logging

Logging can be configured via environment variables.

```bash
RUST_LOG=modio=debug,modbot=debug
```

See [`tracing_subscriber::EnvFilter`] for more information.

#### Metrics

By default, the metrics are exposed via Prometheus endpoint listing on `http://127.0.0.1:8080/metrics`.

```toml
[metrics]
addr = "127.0.0.1:3000"
```

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you,
as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.


[crates-badge]: https://img.shields.io/crates/v/modbot.svg
[crates-url]: https://crates.io/crates/modbot
[rust-version]: https://img.shields.io/badge/rust-1.43%2B-lightgrey.svg?logo=rust
[gha-badge]: https://github.com/nickelc/modio-bot/workflows/CI/badge.svg
[gha-url]: https://github.com/nickelc/modio-bot/actions?query=workflow%3ACI
[license-badge]: https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg
[discord]: https://discord.gg/XNX9665
[discord-badge]: https://img.shields.io/discord/541627648112066581.svg?label=support&logo=discord&color=7289DA&labelColor=2C2F33
[bot-invite-badge]: https://img.shields.io/static/v1.svg?label=%20&logo=discord&message=Invite%20ModBot&color=7289DA&labelColor=2C2F33
[bot-invite-url]: https://discordbot.mod.io
[modio-bot-channel]: https://discord.gg/QR7DGD7
[mod.io]: https://mod.io
[`modio-rs`]: https://github.com/nickelc/modio-rs
[`serenity`]: https://github.com/serenity-rs/serenity
[`tracing_subscriber::EnvFilter`]: https://docs.rs/tracing-subscriber/0.2/?search=EnvFilter
[rust-lang]: https://www.rust-lang.org

<img src="https://raw.githubusercontent.com/nickelc/modio-bot/master/logo.png" width="200" align="right"/>

# ModBot for Discord
[![Crates.io][crates-badge]][crates-url]
![Rust version][rust-version]
![License][license-badge]
[![GitHub Action][gha-badge]][gha-url]
[![Discord][discord-badge]][discord]

ModBot is a Discord bot for [mod.io] using [`modio-rs`] and [`twilight`].

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

 1. Invite the ModBot <https://discordbot.mod.io/>.
 2. View the games list `/games` and set the default game `/settings default-game ID`.
 3. In the channel(s) you want the bot to post updates (mod added / edited),
    run the command `/subs add <GameID>`.
 4. Ensure the bot has `Send Messages` and `Embed Links` permissions in the
    channel(s) it is in to be able to function correctly.

<img src="https://user-images.githubusercontent.com/2128532/118098374-1adc0e80-b3d4-11eb-808a-4024b7e79d9b.png" width="500"/>

## Commands

Once you have invited ModBot to your server, you can set the default game using
the command `/settings default-game ID`. Now when a user issues the command
`/mods`, all of the mods for the game you specified will be returned. You can
change the default game at any time.

We recommend you also `/subs add <GameID>` to games you are interested in
receiving push notifications from. For example in our [#bot channel], we have
subscribed to a bunch of games and whenever a mod is updated, the channel is
notified.

Popular commands include:

 * `/game` return information about the default game
 * `/games [search]` return a list of all games
 * `/mods [ID|Name]` return a list of all mods belonging to the default game
 * `/popular` return a list of mods ordered by popularity
 * `/settings default-game <ID|Name>` set the default game
 * `/subs add <ID|Name> [Tag..] [Type]` subscribe to a game for updates (mods added/edited)
   ```
   /sub add 51
   /sub add OpenXcom
   /sub add OpenXcom tags:"UFO Defense",Major
   /sub add "Skate XL" tags:"Real World Spot"
   /sub add Skate* tags:Gear,Deck
   ```

 * `/subs list` see all games subscribed too
 * `/subs rm <ID|Name> [Tag..] [Type]` unsubscribe from a game
   ```
   /subs rm 51
   /subs rm OpenXcom
   /subs rm OpenXcom tags:"UFO Defense",Major
   /subs rm "Skate XL" tags:"Real World Spot"
   /subs rm skate tags:Gear,Deck
   ```

 * `/subs mods mute <Game> <Mod>` mute a mod from update notifications
 * `/subs mods muted` return a list of all muted mods
 * `/subs mods unmute <Game> <Mod>` unmute a mod from update notifications

## Screenshots

### Mod details
![command](https://user-images.githubusercontent.com/2128532/199087924-87e56fcd-a049-42d5-be92-c776799bbb21.png)
![details](https://user-images.githubusercontent.com/2128532/199013232-dc2468f0-0c79-4645-bc69-403cb65648c3.png)

### New Mod notification
![notification](https://user-images.githubusercontent.com/2128532/98248318-0e827f00-1f75-11eb-89d5-a55174d9fed5.png)

## Building

ModBot is written in Rust, so you'll need to grab a [Rust installation] in
order to compile it. Building is easy:

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

Install latest version from <https://crates.io>.

```
$ cargo install modbot
$ $HOME/.cargo/bin/modbot
```

Install modbot from the `master` branch.

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
[rust-version]: https://img.shields.io/badge/rust-1.65%2B-lightgrey.svg?logo=rust
[gha-badge]: https://github.com/nickelc/modio-bot/workflows/CI/badge.svg
[gha-url]: https://github.com/nickelc/modio-bot/actions?query=workflow%3ACI
[license-badge]: https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg
[discord]: https://discord.gg/XNX9665
[discord-badge]: https://img.shields.io/discord/541627648112066581.svg?label=support&logo=discord&color=7289DA&labelColor=2C2F33
[bot-invite-badge]: https://img.shields.io/static/v1.svg?label=%20&logo=discord&message=Invite%20ModBot&color=7289DA&labelColor=2C2F33
[bot-invite-url]: https://discordbot.mod.io
[#bot channel]: https://discord.gg/QR7DGD7
[mod.io]: https://mod.io
[`modio-rs`]: https://github.com/nickelc/modio-rs
[`twilight`]: https://github.com/twilight-rs/twilight
[`tracing_subscriber::EnvFilter`]: https://docs.rs/tracing-subscriber/0.2/?search=EnvFilter
[Rust Installation]: https://www.rust-lang.org/tools/install

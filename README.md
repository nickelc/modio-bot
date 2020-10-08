<img src="https://raw.githubusercontent.com/nickelc/modio-bot/master/logo.png" width="200" align="right"/>

# ModBot for Discord
![Rust version][rust-version]
![Rust edition][rust-edition]
![License][license-badge]
[![Discord][discord-badge]][discord]
[![Invite ModBot][bot-invite-badge]][bot-invite-url]

ModBot is a Discord bot for [mod.io] using [`modio-rs`] and [`serenity`]. ModBot provides your community with an easy way to search the mod listing. Additionally, if you subscribe to games following the Quick Start introductions below, the ModBot will let you know whenever a mod is added or edited.

## Example

<img src="https://image.mod.io/members/c4ca/1/profileguides/modbot.png" width="500"/>

## Setup

1. Invite ModBot to your [Discord server](https://discordbot.mod.io)
2. Set the default game using `~game {GAME NAME or ID}`
3. In the channel(s) you want the bot to post updates (mod added / edited), run the command `~subscribe {GAME NAME or ID}`
4. Ensure the bot has `Read Messages`, `Send Messages` and `Embed Links` permissions in the channel(s) it is in to be able to function correctly
5. All done, the bot will keep you updated and is there to query the mod.io API.

If you followed the steps above, your Discord community will be able to query mods for the default game, and each channel you subscribed to updates in will receive a push notification each time a mod is added or edited. For example in our [#modbot channel](https://discord.mod.io) we subscribe to every game on mod.io so our Discord community continually gets updated.
 
<img src="https://image.mod.io/mods/3cf1/499/screen_shot_2019-05-17_at_10.59.16_am.png" width="500"/>

## Commands

By default `~` is the prefix used to issue commands to ModBot. Once you have invited ModBot to your server, you can set the default game using the command `~game ID`. Now when a user issues the command `~mods`, all of the mods for the game you specified will be returned. You can change the default game at any time.

We recommend you also `~subscribe ID` to games you are interested in receiving push notifications from. For example in our [#bot channel][modio-bot-channel], we have subscribed to a bunch of games and whenever a mod is updated, the channel is notified.

Popular commands include:

 * `~help` show these commands
 * `~prefix CHARACTER` change the default prefix from `~` to something else
 * `~game ID|Name` set the default game
 * `~game` return information about the default game
 * `~games` return a list of all games
 * `~mod ID|Name` return information about the mod(s) requested
 * `~mods` return a list of all mods belonging to the default game
 * `~popular` return a list of mods ordered by popularity
 * `~subscribe ID|Name` subscribe to a game for updates (mods added/edited)
 * `~subscriptions` see all games subscribed too
 * `~unsubscribe ID|Name` unsubscribe from a game

## Building

If you want to build and host your own version of ModBot, these instructions are for you. ModBot is written in Rust, so you'll need to grab a [Rust installation][rust-lang] in order to compile it.

Building is easy:

```
$ mkdir bot-compile
$ cd bot-compile
$ git clone https://github.com/nickelc/modio-bot
$ cd modio-bot
$ cargo build --release
$ cp target/release/modbot /home/modbot/
$ chown modbot:modbot /home/modbot/modbot
$ sudo su modbot 
$ cd ~
$ ./modbot &
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

Set up the environment variables with `export` or by creating a `.env` file.

- `DISCORD_BOT_TOKEN`
- `MODIO_API_KEY` or `MODIO_TOKEN`
- `MODIO_HOST` (optional)

A `.env` sample is provided as [`.env.sample`](.env.sample).

#### Running the bot
```bash
DISCORD_BOT_TOKEN="your token" \
MODIO_API_KEY="your api key" \
./path/to/modbot
```

#### Logging

Logging can be configured via environment variables.

```bash
RUST_LOG=modio=debug,modbot=debug
```

See the [env\_logger](https://crates.io/crates/env_logger) crate for more information.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you,
as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.


[rust-version]: https://img.shields.io/badge/rust-1.31%2B-blue.svg
[rust-edition]: https://img.shields.io/badge/edition-2018-red.svg
[license-badge]: https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg
[discord]: https://discord.gg/XNX9665
[discord-badge]: https://img.shields.io/discord/541627648112066581.svg?label=Discord&logo=discord&color=7289DA&labelColor=2C2F33
[bot-invite-badge]: https://img.shields.io/static/v1.svg?label=%20&logo=discord&message=Invite%20ModBot&color=7289DA&labelColor=2C2F33
[bot-invite-url]: https://discordbot.mod.io
[modio-bot-channel]: https://discord.gg/QR7DGD7
[repo]: https://github.com/nickelc/modio-bot
[logo]: https://raw.githubusercontent.com/nickelc/modio-bot/master/logo.png
[mod.io]: https://mod.io
[`modio-rs`]: https://github.com/nickelc/modio-rs
[`serenity`]: https://github.com/serenity-rs/serenity
[rust-lang]: https://www.rust-lang.org

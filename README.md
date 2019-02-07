[![MODBOT logo][logo]][repo]

![Rust version][rust-version]
![Rust edition][rust-edition]
![License][license-badge]
[![Discord][discord-badge]][discord]

MODBOT is a Discord bot for [mod.io] using [`modio-rs`] and [`serenity`].

## Building

MODBOT is written in Rust, so you'll need to grab a [Rust installation][rust-lang] in order to compile it.
Building is easy:

```
$ git clone https://github.com/nickelc/modio-bot
$ cd modio-bot
$ cargo build --release
$ ./target/release/modbot
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
[discord]: https://discord.gg/4akZJFf
[discord-badge]: https://img.shields.io/discord/541627648112066581.svg
[repo]: https://github.com/nickelc/modio-bot
[logo]: https://raw.githubusercontent.com/nickelc/modio-bot/master/logo.png
[mod.io]: https://mod.io
[`modio-rs`]: https://github.com/nickelc/modio-rs
[`serenity`]: https://github.com/serenity-rs/serenity
[rust-lang]: https://www.rust-lang.org

# Contributing to rs-pug

Thanks for considering a contribution! This project is maintained in spare time, so clear, focused contributions are the easiest to review and merge.

## Getting started

1. Fork the repo and clone your fork.
2. Install dependencies: `mpv` (required), `yt-dlp` (recommended).
3. Build and run:
   ```bash
   cargo build --release
   ./target/release/rs-pug
   ```
4. Make your changes on a feature branch.
5. Open a pull request against `main`.

> [!NOTE]
> Rust can fight back sometimes - if you get stuck, open a draft PR early and ask questions there.

## What's especially welcome

- **Lua plugin PRs** - new plugins for [all-rspug](https://github.com/JustRoccat/all-rspug/), or improvements to the plugin API itself. See [`docs.md`](./docs.md) for the full API reference.
- **Themes and EQ presets** - contribute these to [all-rspug](https://github.com/JustRoccat/all-rspug/) rather than this repo.
- Bug fixes, especially around playback, IPC, and the SQLite migration path.

## Scope notes

- **AUR packaging is unmaintained.** If you'd like to take it over, please open an issue rather than a PR.
- **crates.io remains actively maintained** by the author.

## Before opening a PR

- Run `cargo fmt` and `cargo clippy` and address any warnings your change introduces.
- Test manually with both a local file and a streamed source if your change touches playback.
- If you're adding a config option, document it in `README.md` (and `docs.md` if it's plugin-facing).
- Keep PRs focused - unrelated refactors make review slower and are best split out.

## Reporting bugs

- Check you're running the latest `mpv` and `yt-dlp` first - a surprising number of playback issues trace back to one of these being outdated.
- Include your `rs-pug` version, OS, and steps to reproduce.
- If relevant, attach a debug log: run with `--debug` to write logs to `~/.config/rs-pug/rs-pug.log`.

## Community

Questions, ideas, or just want to chat about the project? Join the [Discord](https://discord.gg/6FcBWwRQBX).

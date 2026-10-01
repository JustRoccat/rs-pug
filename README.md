# rs-pug

[![dependency status](https://deps.rs/repo/github/JustRoccat/rs-pug/status.svg)](https://deps.rs/repo/github/JustRoccat/rs-pug)
[![License: GPL-2.0](https://img.shields.io/badge/license-GPL--2.0-blue.svg)](LICENSE)

> No browser, no ads, no Electron. Search YouTube, SoundCloud, or your own selfhosted [Sonum](https://github.com/JustRoccat/Sonum) server, queue tracks, play local files, all from your terminal.

![demo](https://github.com/user-attachments/assets/d0ee7dcf-a751-4942-adeb-0d738d66095e)

`rs-pug` is a terminal music player built in Rust on top of `mpv`, `yt-dlp`, and `ratatui`. It streams and downloads from YouTube and SoundCloud, pulls tracks from a self-hosted [Sonum](https://github.com/JustRoccat/Sonum) server, manages a local library and playlists, and extends with Lua plugins, all without leaving the terminal.

> [!IMPORTANT]
> AUR is no longer maintained by the author. If you'd like to take over as AUR maintainer, please open an issue. `crates.io` continues to be maintained.

> [!TIP]
> Before reporting a bug, make sure you're running the latest `yt-dlp` and `mpv`.

Community plugins, themes, and EQ presets: [all-rspug](https://github.com/JustRoccat/all-rspug/) · [Discord](https://discord.gg/6FcBWwRQBX)

## Features

- Search and stream from YouTube, SoundCloud, or a self hosted [Sonum](https://github.com/JustRoccat/Sonum) server, or play local files, all from one interface
- Queue management with multiselect bulkadd
- Playlists and library backed by SQLite, with automatic migration from legacy JSON
- Smart Queue (im not very proud about this feature): finds similar tracks to keep the music flowing automatically
- Real time fft audio spectrum visualizer (with a synthetic fallback)
- Minimal mode: pixelated cover art, small fft strip, title only
- Cover-tinted Minimal background that follows the current track, toggleable in Options
- 10 band graphic equalizer with savable presets
- Fully remappable keybinds, including modifiers and multi key sequences
- Built in and custom themes
- Command palette (`:`) for fuzzy-searching every action
- Control a running instance over IPC, for use in status bars or keybindings
- Extensible with Lua plugins: custom keybinds, live panels, and full UI customization
- Hot reload: configuration and theme changes apply automatically
- Add any streaming source you want.

## Requirements

- [`mpv`](https://mpv.io/) (required)
- [`yt-dlp`](https://github.com/yt-dlp/yt-dlp) (recommended, without it streaming and downloading are unavailable, but local playback still works)
- MPRIS2 works out of the box through a native in process daemon, no `mpv-mpris` needed. Set `mpris_enabled = false` in the config to keep rs-pug off the session bus.

## Installation

```bash
# crates.io
cargo install rs-pug

# Manual
git clone https://github.com/JustRoccat/rs-pug
cd rs-pug
cargo build --release
./target/release/rs-pug
```

> [!NOTE]
> Community AUR packaging is not currently maintained. See the note above if you'd like to help.

## Usage

Run `rs-pug` to launch the TUI. The app scans `~/.config/rs-pug/music-local/` for local files by default; you can add more directories from the **Options** tab.

Full guide (keys, remapping, EQ, visualizer, Minimal, IPC, themes, Sonum, storage): [`docs-usage.md`](./docs-usage.md).

Config file: `~/.config/rs-pug/config.toml`. 

## Plugins

Drop `.lua` files into `~/.config/rs-pug/plugins/`, see [`docs.md`](./docs.md) for the full API reference. Lua plugin PRs are especially welcome.

## Works on

Anywhere `mpv` and `yt-dlp` run: Linux and Termux (Android) tested, WSL2 on Windows should also work.

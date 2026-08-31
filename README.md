# rs-pug

[![dependency status](https://deps.rs/repo/github/JustRoccat/rs-pug/status.svg)](https://deps.rs/repo/github/JustRoccat/rs-pug)
[![License: GPL-2.0](https://img.shields.io/badge/license-GPL--2.0-blue.svg)](LICENSE)

> No browser, no ads, no Electron. Search YouTube, SoundCloud, or your own self-hosted [Sonum](https://github.com/JustRoccat/Sonum) server, queue tracks, play local files - all from your terminal.

![demo](https://github.com/user-attachments/assets/d0ee7dcf-a751-4942-adeb-0d738d66095e)

`rs-pug` is a terminal music player built in Rust on top of `mpv`, `yt-dlp`, and `ratatui`. It streams and downloads from YouTube and SoundCloud, can pull tracks from a self-hosted [Sonum](https://github.com/JustRoccat/Sonum) server, manages a local library and playlists, and can be extended with Lua plugins - all without leaving the terminal.

> [!IMPORTANT]
> AUR is no longer maintained by the author. If you'd like to take over as AUR maintainer, please open an issue. `crates.io` continues to be maintained.

> [!TIP]
> Before reporting a bug, make sure you're running the latest `yt-dlp` and `mpv`.

Community plugins, themes, and EQ presets: [all-rspug](https://github.com/JustRoccat/all-rspug/) · [Discord](https://discord.gg/6FcBWwRQBX)

## Features

- Search and stream from YouTube, SoundCloud, or a self-hosted [Sonum](https://github.com/JustRoccat/Sonum) server, or play local files, all from one interface
- Queue management with multi-select bulk-add
- Playlists and library backed by SQLite, with automatic migration from legacy JSON
- Smart Queue: finds similar tracks to keep the music flowing automatically
- Real-time FFT audio spectrum visualizer (with a synthetic fallback)
- 10-band graphic equalizer with savable presets
- Fully remappable keybinds, including modifiers and multi-key sequences
- Built-in and custom themes
- Command palette (`:`) for fuzzy-searching every action
- Control a running instance over IPC, for use in status bars or keybindings
- Extensible with Lua plugins: custom keybinds, live panels, and full UI customization
- Hot reload - configuration and theme changes apply automatically

## Requirements

- [`mpv`](https://mpv.io/) (required)
- [`yt-dlp`](https://github.com/yt-dlp/yt-dlp) (recommended - without it, streaming and downloading are unavailable, but local playback still works)
- MPRIS2 works out of the box, rs-pug ships its own native MPRIS2 daemon, no `mpv-mpris` needed. Disable it with `mpris_enabled = false` in the config if you don't want rs-pug on the session bus (im saying this because before rs-pug needed mpv-mpris)

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

### Keybinds

| Key | Action |
|-----|--------|
| `1`-`5` | Switch tabs: Discover, Albums, Library (playlists), Local, Options |
| `Tab` | Switch panel focus |
| `j` / `k` | Move up / down |
| `/` | Search |
| `Enter` | Play (or add all marked songs to the queue - see [Multi-select](#multi-select--bulk-queue)) |
| `Space` | Pause / Resume |
| `n` / `p` | Next / Previous |
| `m` | Mute |
| `r` | Cycle repeat mode |
| `c` | Context menu |
| `v` | Toggle flat/organized view (Local tab) |
| `Ctrl+V` | Toggle the real FFT spectrum visualizer (needs `parec`) |
| `e` | Edit ID3 tags for selected local file (Local tab) |
| `s` | Cycle local sort mode (Local tab) |
| `g` / `a` | Filter local library by genre / artist (Local tab, Organized view) |
| `b` | Filter by album (Organized view), or mark/unmark for bulk-queue (Flat view / Discover) |
| `F` | Clear local library filters |
| `:` | Open the command palette |
| `?` | Show the full command reference |
| `q` | Quit |

Every keybind above (`n`, `p`, `m`, `r`, `z`, `[`, `]`, and the FFT toggle) can be rebound in `~/.config/rs-pug/config.toml`, using single characters or key sequences:

```toml
[keybinds]
next = "n"
prev = "p"
mute = "m"
repeat = "r"
shuffle = "z"
seek_back = "["
seek_forward = "]"
fft_toggle = "C-v"   # Ctrl+V
```

Modifiers are prefixed with `C-` (Ctrl), `M-` (Alt), and/or `S-` (Shift), e.g. `C-r` or `M-S-n`. Multi-key sequences are space-separated, e.g. `g g` (press `g` twice within 1.5s). The **Options** tab also lets you remap `next`/`prev`/`mute`/`repeat`/`shuffle`/`seek_back`/`seek_forward` directly, though it's currently limited to single characters there.

### Multi-select / bulk queue

Instead of queueing songs one at a time:

1. Press `b` on a song to mark it (in **Discover** results or the **Local** tab's flat view).
2. Keep marking more with `j`/`k`.
3. Press `Enter` to queue every marked song in list order. Playback starts automatically only if nothing was already playing.
4. Press `Esc` to clear marks without queuing anything.

Marks track the song itself, not its list position, so scrolling won't lose them.

### Command palette

`:` opens a fuzzy-searchable palette for playback, volume, repeat/shuffle, seeking, speed, EQ, and tab navigation - use `↑`/`↓` to pick a result and `Enter` to run it. `?` shows the same list read-only, without running anything.

### Playback speed

Available from the **Options** tab's **Speed** row (0.25x-2.00x, `h`/`l` to adjust in 0.05x steps, `Enter` to reset) or the command palette (`speed up` / `speed down` / `speed reset`). The current speed appears as a badge next to "Now Playing" whenever it isn't 1.00x.

### Equalizer

The 10-band graph in **Options** is interactive once selected:

- `h`/`l` - move between bands
- `+`/`-` - adjust gain of the selected band (-12 dB to +12 dB)
- `p` - cycle EQ presets
- `s` - save current settings, including EQ, to `config.toml`

Custom EQ presets are stored as `.json` files in `~/.config/rs-pug/eqpresets/`.

### FFT visualizer

The "Now Playing" bar always shows an animated spectrum - a synthetic wave by default. Press `Ctrl+V` (or your remapped `fft_toggle`) to switch to a **real** spectrum computed from system audio. `rs-pug` tries these in order:

1. `parec` (PulseAudio, or PipeWire's `pipewire-pulse` compatibility layer) - enables precise per-stream capture via `pactl`
2. `pw-cat --record --raw --monitor` - native PipeWire, captures the default sink's output
3. `pw-record --monitor` - native PipeWire fallback

If none are installed, `rs-pug` silently falls back to the synthetic wave. To enable the real visualizer by default at startup:

```toml
[general]
fft_visualizer_default = true
```

## CLI / IPC

Beyond `--source`, `rs-pug` accepts flags that control an **already-running** instance over a local Unix socket - handy for `i3status`, `waybar`, or keybinding scripts:

```bash
rs-pug --toggle-pause         # play/pause the running instance
rs-pug --next                 # skip to next track
rs-pug --prev                 # go to previous track
rs-pug --play <path-or-url>   # queue and play a file or URL
```

Each command connects to the running instance's IPC socket and exits immediately. If no instance is running, an error is printed instead of starting a new one.

Pass `--debug` to write logs to `~/.config/rs-pug/rs-pug.log`, useful when filing a bug report.

## Configuration

Config file: `~/.config/rs-pug/config.toml`.

### Themes

Built-in themes: `dark` (default), `light`, `nord`, `gruvbox`, `mono`.

```toml
[general]
theme = "nord"
```

To create your own, add a `.json` file under `~/.config/rs-pug/themes/` with `[r, g, b]` triples for each color:

```json
{
  "text": [255, 255, 255],
  "dim": [100, 100, 100],
  "muted": [150, 150, 150],
  "info": [0, 255, 255],
  "warn": [255, 255, 0],
  "ok": [0, 255, 0],
  "primary": [255, 0, 255],
  "accent2": [200, 0, 200],
  "accent3": [100, 0, 100],
  "spectrum": [[255, 0, 255], [0, 255, 255], [255, 255, 0]]
}
```

Then reference it by filename (without `.json`):

```toml
[general]
theme = "mytheme"
```

> [!NOTE]
> All nine base colors are required - if one is missing, the file fails to parse and `rs-pug` falls back to the built-in palette. `spectrum` is optional and accepts a list of any length, omit it for the default gradient.

Restart or hot-reload to apply changes. Community themes: [all-rspug](https://github.com/JustRoccat/all-rspug/).

### Local music & storage

`rs-pug` scans `~/.config/rs-pug/music-local/` by default (add more directories from **Options**), with natural sorting and metadata extraction.

Playlists and library data live in a SQLite database at `~/.config/rs-pug/pug.db`. Legacy JSON files are migrated automatically on first run.

- Playlist import: `~/.config/rs-pug/import_playlist.json`
- Playlist export: `~/.config/rs-pug/exports/<playlist_name>.json`



### Sonum (self-hosted music server)

[Sonum](https://github.com/JustRoccat/Sonum) is a lightweight, self-hosted music streaming server (Rust/Axum). It scans a music folder on a machine of your choosing and exposes it over a plain HTTP/JSON API, with metadata, lyrics, and album art extraction, and auto-rescans when files change. `rs-pug` can talk to a Sonum server as a third search source, alongside YouTube and SoundCloud, which is useful for streaming your library from a home server/NAS to any machine running `rs-pug`.

**How it works:** on startup, `rs-pug` writes a default client config to `~/.config/rs-pug/sonumclient.toml` if it doesn't already exist:

```toml
host = "127.0.0.1"
port = 8420
# api_token = "your-secret-token"
```

- `host` / `port` should point at wherever your Sonum server is running (defaults match Sonum's own defaults, `127.0.0.1:8420`).
- `api_token` is optional and only needed if the Sonum server was started with `api_token` set in its own `sonum.conf` - when set, `rs-pug` sends it as `Authorization: Bearer <token>` on every request.
- **Restart `rs-pug` after editing this file** - unlike `config.toml`, it isn't hot-reloaded.

Once configured, switch **Search source** to **Sonum** from the **Options** tab (`h`/`l` to cycle YouTube → SoundCloud → Sonum), or launch with:

```bash
rs-pug --source sonum
```

Searching then queries `GET /tracks?q=<query>&limit=<n>` on the Sonum server; results are mapped into `rs-pug` songs, with playback pointed at the server's `/tracks/:id/stream` endpoint (so `mpv` streams directly from Sonum). The Albums view groups the returned tracks client-side by album/artist, since Sonum's `/tracks` endpoint doesn't have a dedicated album grouping - this means very large libraries may need a higher `limit` to see complete albums in search results.

> [!NOTE]
> This integration only covers *searching and streaming* from Sonum. Local downloads, playlists, and the local library scanner are unaffected and continue to work with `~/.config/rs-pug/music-local/` as usual.

### Smart Playlist

On every startup, rs-pug auto-generates and refreshes a single **Smart Playlist** built from three SQLite-backed rules over your local library: most played, recently added, and not-heard-in-a-while tracks (deduped, capped at ~50 songs). It behaves like a normal playlist otherwise, but its contents are replaced on the next launch, so treat it as a rotating mix rather than something to hand-curate. Disable it with:

```toml
[general]
smart_playlists_enabled = false
```

## Plugins (Lua)

Drop `.lua` files into `~/.config/rs-pug/plugins/` and they're loaded automatically. Plugins can react to keypresses, search queries, and playback events; render live panels; add dynamic tabs; and, opt-in, restructure the stock UI itself (layout, tab bar position, custom sections).

```toml
[lua]
allow-lua-ui-changes = true   # default: false, legacy plugins work either way
```

See [`docs.md`](./docs.md) for the full API reference, including all hooks, UI patch fields, and complete examples. Lua plugin PRs are especially welcome.

## Works on

Anywhere `mpv` and `yt-dlp` run - tested on Linux and Termux (Android); WSL2 on Windows should also work.

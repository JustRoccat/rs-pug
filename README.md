

# Read

Im not gonna maintain aur anymore, if you want to be an aur maintainer please create a issue.

This does not apply to crates.io, im gonna still maintain crates.io

[![dependency status](https://deps.rs/repo/github/JustRoccat/rs-pug/status.svg)](https://deps.rs/repo/github/JustRoccat/rs-pug)

# rs-pug

No browser, no ads, no Electron. Search YouTube and SoundCloud, queue tracks, play local files — all from your terminal.

Built in Rust with `mpv`, `yt-dlp`, and `ratatui`. Requires `mpv` and `yt-dlp` installed.

Plugins, themes and EQ presets from the community: [all-rspug](https://github.com/JustRoccat/all-rspug/) · [Discord](https://discord.gg/6FcBWwRQBX)

![img](https://github.com/user-attachments/assets/d0ee7dcf-a751-4942-adeb-0d738d66095e)

## Installation

```bash
# Arch
yay -S rs-pug-git

# crates.io
cargo install rs-pug

# manual
git clone https://github.com/JustRoccat/rs-pug && cd rs-pug
cargo build --release && ./target/release/rs-pug
```

## Dependencies

Required:

- `mpv`
- `yt-dlp`

Optional:

- `mpv-mpris` (for media key / `playerctl` support)


## Keybinds

| Key | Action |
|-----|--------|
| `1`–`5` | Switch tabs |
| `Tab` | Switch panel focus |
| `j` / `k` | Move up / down |
| `/` | Search |
| `Enter` | Play |
| `Space` | Pause / Resume |
| `n` / `p` | Next / Previous |
| `m` | Mute |
| `r` | Cycle repeat mode |
| `c` | Context menu |
| `v` | Toggle flat/organized view (Local tab) |
| `e` | Edit ID3 tags for selected local file (Local tab) |
| `s` | Cycle local sort mode: title, artist, album, year, date added (Local tab) |
| `g` / `a` / `b` | Filter local library by selected genre / artist / album |
| `F` | Clear local library filters |
| `q` | Quit |


## Contri-pug-ting

1. Fork the repo
2. Install dependencies
3. Smash your head against the keyboard (Rust can be like that)
4. Open a pull request

Lua plugin PRs especially welcome — API reference in [`docs.md`](./docs.md).

## Mouse Support

- **Scroll Wheel**: Scroll through search results, playlists, and the queue.
- **Left Click**: Click on tab icons at the top to switch between views.

## Smart Queue

In the **Options** tab, you can trigger the **Smart Queue**. This feature analyzes the currently playing song and automatically finds similar tracks from the same uploader or with similar titles to keep the music flowing.

## Customization

### Themes

`rs-pug` comes with several built-in themes: `dark` (default), `light`, `nord`, `gruvbox`, and `mono`.

You can add your own themes by creating `.json` files in `~/.config/rs-pug/themes/`.

Example `mytheme.json`:
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
  "accent3": [100, 0, 100]
}
```

### EQ Presets

Your custom EQ presets are stored as `.json` files in `~/.config/rs-pug/eqpresets/`.

## Local Music

By default, `rs-pug` scans `~/.config/rs-pug/music-local/`. You can change this or add more directories in the **Options** tab. The app supports natural sorting and metadata extraction.

## Playlists & Storage

Data is stored in a SQLite database at `~/.config/rs-pug/pug.db`. Legacy JSON files are automatically migrated on first run.

- **Import path**: `~/.config/rs-pug/import_playlist.json`
- **Export path**: `~/.config/rs-pug/exports/<playlist_name>.json`

## Plugins (Lua)

Drop Lua scripts into `~/.config/rs-pug/plugins/`. They are loaded automatically and can react to keys, search queries, and playback events. See `docs.md` for the full API reference.

Lua plugins that change the stock UI are opt-in. Add this to `~/.config/rs-pug/config.toml` to enable the new UI hooks (`on_ui_config`, `on_ui_sections`, `on_ui_update`, and `on_ui_inject`):

```toml
[lua]
allow-lua-ui-changes = true
```

The default is `false`, so legacy plugins using `on_key`, `on_event`, `on_tabs`, or `on_ui_panels` continue to work unchanged. When UI customization is enabled, plugin load/hook/layout issues are captured as non-fatal warnings and shown in the statusbar instead of crashing the app.

## Configuration

Config file path: `~/.config/rs-pug/config.toml`.


## License

MIT

![gif](https://github.com/user-attachments/assets/43e4fc61-cd06-43a3-872c-632059467259)

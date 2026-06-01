# rs-pug

Neovim out of music players. Search, queue, listen — no browser, no ads, no nonsense.

`rs-pug` uses `mpv` + `yt-dlp` under the hood, with a terminal UI built on `ratatui`.

![img](https://github.com/user-attachments/assets/f2934a10-1187-4ebf-9152-021b2c804fc0)
![img](https://github.com/user-attachments/assets/d0ee7dcf-a751-4942-adeb-0d738d66095e)

## Features

- **YouTube & SoundCloud search** and queue playback in TUI.
- **Switchable Search Sources**: Toggle between YouTube and SoundCloud in the Options tab or via CLI.
- **Album search** support (Albums tab).
- **Smart Queue**: Automatically find and play similar tracks.
- **Local files playback** with metadata support (Artist/Album/Title).
- **Organized Local View**: Switch between Flat and Organized (Artist -> Album -> Song) views.
- **Playlist library**: Create, delete, and manage your own playlists.
- **10-band equalizer** with built-in and custom presets.
- **Dynamic, reactive audio visualizer**.
- **Recently played history** (saved to SQLite database).
- **Playlist import/export** from context menu (`c`).
- **Theme switching**: Multiple built-in themes + custom JSON themes.
- **Mouse support**: Scroll through lists and click tabs.
- **Lua plugin system** for extending functionality.
- **MPRIS support**: Media key and playerctl integration.
- **Asynchronous Core**: High responsiveness; the UI remains fluid during searches and library scanning.

## Dependencies

Required:

- `mpv`
- `yt-dlp`

Optional:

- `mpv-mpris` (for media key / `playerctl` support)

## Installation

### From AUR (Arch Linux)

```bash
yay -S rs-pug-git
# or
paru -S rs-pug-git
```

### Manually

```bash
git clone https://github.com/JustRoccat/rs-pug
cd rs-pug
cargo build --release
./target/release/rs-pug
```
### From crates.io
```
cargo install rs-pug
```

## Keybinds

### Navigation & Global

| Key | Action |
|-----|--------|
| `1`-`5` | Switch Tabs (Discover, Albums, Library, Local, Options) |
| `Tab` | Switch focus between panels (Results / Queue) |
| `j` / `Down` | Move selection down |
| `k` / `Up` | Move selection up |
| `PgUp` / `PgDn` | Scroll selection by 10 |
| `/` | Start searching (Discover/Albums tabs) |
| `c` | Open Context Menu |
| `q` | Quit |
| `Ctrl+c` | Force quit |

### Playback

| Key | Action |
|-----|--------|
| `Enter` | Play selected song / Confirm |
| `Space` | Pause / Resume |
| `n` | Next track |
| `p` | Previous track |
| `m` | Mute / Unmute |
| `r` | Cycle Repeat mode (None, Track, Queue) |

### Tab Specific

- **Library Tab**:
  - `a`: Create new playlist.
  - `x`: Delete selected playlist (with confirmation).
  - `e`: Expand / Collapse playlist folders.
- **Local Tab**:
  - `v`: Toggle view mode (**Flat** vs **Organized**).
  - `Backspace` / `Esc`: Go back one level in Organized view.
- **Options Tab**:
  - `f`: While focusing "EQ preset", press to save current EQ as a new preset.

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

## Contributing

Plugin system is Lua-based. If you know Lua, example plugins / plugin PRs are very welcome.

## BTW

Now you can see examples and you can download plugins, themes, eqpresets here: https://github.com/JustRoccat/all-rspug/

We have a discord server now, i will post there updates about the project and news!
https://discord.gg/6FcBWwRQBX

## Please read this if you plan to fork this project
I welcome forks and derivatives. Please link to the original or credit rs-pug. Regardless, please do not misrepresent this as your own work. Even if you don't link the source, never claim you built it from scratch at the very least, retain the original copyright notice. I've already dealt with this, so let’s keep it simple: please don't be an asshole.

![gif](https://github.com/user-attachments/assets/43e4fc61-cd06-43a3-872c-632059467259)

## License

MIT


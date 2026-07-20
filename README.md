[![dependency status](https://deps.rs/repo/github/JustRoccat/rs-pug/status.svg)](https://deps.rs/repo/github/JustRoccat/rs-pug)
## If you find this useful, a star is the only currency I get paid in

# Read

Im not gonna maintain aur anymore, if you want to be an aur maintainer please create a issue.

This does not apply to crates.io, im gonna still maintain crates.io


# Read this too

Always check if you have the latest yt-dlp and mpv before calling this project shit


# rs-pug

No browser, no ads, no Electron. Search YouTube and SoundCloud, queue tracks, play local files - all from your terminal.

Built in Rust with `mpv`, `yt-dlp`, and `ratatui`. Requires `mpv` installed.

YT-DLP IS HIGHLY RECOMMENDED BUT NOT NECESSARY.
rs-pug will work without it but you will lose the ability to stream and download songs through rs-pug.

Plugins, themes and EQ presets from the community: [all-rspug](https://github.com/JustRoccat/all-rspug/) · [Discord](https://discord.gg/6FcBWwRQBX)

![img](https://github.com/user-attachments/assets/d0ee7dcf-a751-4942-adeb-0d738d66095e)

## Installation

```bash
# Arch [Unsupported]
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
| `Ctrl+V` | Toggle the real FFT spectrum visualizer (needs `parec`) — see below |
| `e` | Edit ID3 tags for selected local file (Local tab) |
| `s` | Cycle local sort mode: title, artist, album, year, date added (Local tab) |
| `g` / `a` / `b` | Filter local library by selected genre / artist / album |
| `F` | Clear local library filters |
| `q` | Quit |

Every keybind above (`n`, `p`, `m`, `r`, `z`, `[`, `]`, and the FFT toggle) can be
rebound in `~/.config/rs-pug/config.toml` under `[keybinds]`, and isn't limited
to a single plain character:

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

- Modifiers: prefix with `C-` (Ctrl), `M-` (Alt), and/or `S-` (Shift), e.g. `C-r` or `M-S-n`.
- Multi-key sequences: separate keys with a space, e.g. `g g` — press `g` then `g` again within 1.5s.
`next`/`prev`/`mute`/`repeat`/`shuffle`/`seek_back`/`seek_forward` can also be re-mapped from the **Options** tab with `h`/`l` (currently limited to single characters there; use the config file directly for modifiers or sequences).

## FFT Visualizer

The "Now Playing" bar always shows an animated spectrum. By default it's a
synthetic wave, but you can switch it to a **real** FFT computed from your
system audio:

- Press `Ctrl+V` (or your remapped `fft_toggle` key) at any time to flip
  between the synthetic and real spectrum, this doesn't replace the
  synthetic visualizer, it's an additional mode you can toggle on the fly.
  It tries, in order, whichever of these is installed:
  1. `parec` — PulseAudio, or PipeWire's `pipewire-pulse` compatibility layer
     (most distros with PipeWire ship this, so `parec` just works, and it's
     also what enables the precise per-stream capture above via `pactl`).
  2. `pw-cat --record --raw --monitor`, targeting the default sink, native
     PipeWire, for setups without the PulseAudio compatibility shim. This
     path can't isolate rs-pug's stream specifically (no `pactl` to ask), so
     it captures whatever's coming out of the default output instead.
  3. `pw-record --monitor` native PipeWire fallback if `pw-cat` isn't
     present, same default-sink targeting as above.

  If none of them are installed, `rs-pug` silently falls back to the
  synthetic wave. `pw-cat`/`pw-record` come from the `pipewire` /
  `pipewire-utils` package on most distros, and the precise per-stream mode
  additionally needs `pactl` (`pulseaudio-utils` or `pipewire-pulse`,
  depending on distro).
- To have it on by default at startup instead of toggling it every time, add:

```toml
[general]
fft_visualizer_default = true
```

## CLI / IPC

Beyond `--source`, `rs-pug` finally accepts flags that control an **already-running**
instance over a local Unix socket. handy for `i3status`, `waybar`, or
keybinding scripts, without needing to focus the TUI:

```bash
rs-pug --toggle-pause   # play/pause the running instance
rs-pug --next           # skip to next track
rs-pug --prev           # go to previous track
rs-pug --play <path-or-url>   # queue and play a file or URL
```

Each of these connects to the running instance's IPC socket and exits
immediately; if no instance is running, it prints an error instead of
starting a new one.


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

**Switching to a built-in theme**

Add this to `~/.config/rs-pug/config.toml`:
```toml
[general]
theme = "nord"
```
Restart `rs-pug` (or hot reload, see below) and the new colors apply immediately.

**Creating your own theme**

1. Create a `.json` file in `~/.config/rs-pug/themes/`, e.g. `~/.config/rs-pug/themes/mytheme.json`.
2. Fill in your colors as `[r, g, b]` triples (0-255):

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
  "spectrum": [
    [255, 0, 255],
    [230, 0, 255],
    [200, 0, 255],
    [150, 0, 255],
    [100, 50, 255],
    [0, 150, 255],
    [0, 255, 255],
    [0, 255, 150],
    [100, 255, 0],
    [255, 255, 0],
    [255, 150, 0],
    [255, 0, 0]
  ]
}
```

3. Point `config.toml` at it by filename (without `.json`):
```toml
[general]
theme = "mytheme"
```
4. Restart `rs-pug` / hot reload. The whole UI and the spectrum visualizer under "Now Playing" now uses your colors.

All nine base colors (`text`, `dim`, `muted`, `info`, `warn`, `ok`, `primary`, `accent2`, `accent3`) are required if one is missing the file fails to parse and `rs-pug` silently falls back to the built-in palette instead. `spectrum` is the one optional field: omit it and you get the standard gradient shown above. It accepts a list of any length; the colors are cycled through as the visualizer bars animate.

Community themes: [all-rspug](https://github.com/JustRoccat/all-rspug/).

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

## Hot Reload

so yeah theres hot reload now and it works automaticly so you dont have to do anything, have a good day
## Configuration

Config file path: `~/.config/rs-pug/config.toml`.


## Works on


Runs anywhere mpv and yt-dlp run - tested on Linux and Termux (Android). WSL2 on Windows should work too.

## License

GPL-2.0

![gif](https://github.com/user-attachments/assets/43e4fc61-cd06-43a3-872c-632059467259)

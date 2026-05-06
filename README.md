# rs-pug

Neovim out of music players. Search, queue, listen — no browser, no ads, no nonsense.

`rs-pug` uses `mpv` + `yt-dlp` under the hood, with a terminal UI built on `ratatui`.

![img](https://cdn.discordapp.com/attachments/1473377698478031021/1492564717183696991/image.png?ex=69dbcab7&is=69da7937&hm=87bbbd463f1802f0a254779f4f4810a9eaf9920468c77ba3bda964c850780423&)
![img](https://cdn.discordapp.com/attachments/1473377698478031021/1494436708534714368/image.png?ex=69e29a24&is=69e148a4&hm=3527f1409febfef1ee48485a71b461f50a766f5fc70be93f78221a45f76496bb&)
## Features

- YouTube search and queue playback in TUI
- Local files playback from a configurable directory
- Albums tab + playlist library
- 10-band equalizer with built-in and custom presets
- Dynamic, reactive audio visualizer
- Recently played history (saved to disk)
- Playlist import/export from context menu (`c`)
- Theme switching + custom themes support
- Basic keybind customization in Options
- Lua plugin system

## Dependencies

Required:

- `mpv`
- `yt-dlp`

Optional:

- `mpv-mpris` (if you want `playerctl` / media key support)

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

## Default keybinds

| Key | Action |
|-----|--------|
| `/` | Search |
| `Enter` | Play / confirm |
| `Space` | Pause / resume |
| `n` | Next |
| `p` | Previous |
| `m` | Mute |
| `r` | Repeat mode |
| `Tab` | Focus switch |
| `c` | Context menu |
| `q` | Quit |

> Note: some keys are context-specific (Library, Options, EQ panel).

## Options / EQ quick notes

- In **Options** you can:
  - change search limit, theme, repeat mode
  - edit selected keybind values (`next`, `prev`, `mute`) using `h/l`
  - open EQ controls (10-band gain editing)
- EQ preset selection is available through the Options row (`EQ preset`) via `h/l` or `Enter`.
- **Save custom EQ presets**: While focusing the `EQ preset` option, press `f` to name and save your current settings.

## Customization

### Themes
You can add your own themes by creating `.json` files in:
`~/.config/rs-pug/themes/`

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
Your custom EQ presets are stored as `.json` files in:
`~/.config/rs-pug/eqpresets/`

You can create them directly in the app by pressing `f` while focusing the **EQ preset** option in the Settings tab.

## Local Music

Support for playing local audio files. By default, rs-pug scans:

`~/.config/rs-pug/music-local/`

You can change this directory in the **Options** tab.

## Playlists

- Library tab supports playlist create/delete and song management.
- Context menu (`c`) in **Library + Results** includes:
  - Import playlist
  - Export selected playlist

### Import path

`~/.config/rs-pug/import_playlist.json`

If the file does not exist, rs-pug will create a template automatically.

### Export path

`~/.config/rs-pug/exports/<playlist_name>.json`

## Recently played

- Recently played entries are updated when a song starts.
- Stored at:

`~/.config/rs-pug/recently_played.json`

- Top recent entries are shown in the Library panel.

## Plugins (Lua)

Drop Lua scripts into:

`~/.config/rs-pug/plugins/`

They are loaded automatically. Plugins receive events (song start, search results, keypresses) and can modify app behavior.

Example plugin:

```lua
return {
    on_key = function(key, state)
        if key == "char:x" then
            return { flash = "hey!" }
        end
    end
}
```

## Configuration

Config file path:

`~/.config/rs-pug/config.toml`

## Contributing

Plugin system is Lua-based. If you know Lua, example plugins / plugin PRs are very welcome.

Small, ugly, but there's just something about it.


## BTW

Now you can see examples and you can download plugins, themes, eqpresets here: https://github.com/JustRoccat/all-rspug/

![gif](https://cdn.discordapp.com/attachments/1473377698478031021/1494437452063047691/2026-04-16_22-38-26.gif?ex=69e29ad5&is=69e14955&hm=509349753483c5479c81a58289e83926dc0948cd1f0cdc18d23ec224fd9db4cd&)

## License

MIT

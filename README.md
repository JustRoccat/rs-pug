# rs-pug

Neovim out of music players. Search, queue, listen — no browser, no ads, no nonsense.

`rs-pug` uses `mpv` + `yt-dlp` under the hood, with a terminal UI built on `ratatui`.

![demo placeholder](https://cdn.discordapp.com/attachments/1473377698478031021/1492564717183696991/image.png?ex=69dbcab7&is=69da7937&hm=87bbbd463f1802f0a254779f4f4810a9eaf9920468c77ba3bda964c850780423&)

## Features

- YouTube search and queue playback in TUI
- Albums tab + playlist library
- 10-band equalizer with presets (`Flat`, `Bass Boost`, `Vocal Boost`, `Treble Boost`, `Night`)
- Recently played history (saved to disk)
- Playlist import/export from context menu (`c`)
- Theme switching + basic keybind customization in Options
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

## License

MIT

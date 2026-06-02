[![dependency status](https://deps.rs/repo/github/JustRoccat/rs-pug/status.svg)](https://deps.rs/repo/github/JustRoccat/rs-pug)

# rs-pug

No browser, no ads, no Electron. Search YouTube and SoundCloud, queue tracks, play local files — all from your terminal.

Built in Rust with `mpv`, `yt-dlp`, and `ratatui`. Requires `mpv` and `yt-dlp` installed.

Plugins, themes and EQ presets from the community: [all-rspug](https://github.com/JustRoccat/all-rspug/) · [Discord](https://discord.gg/6FcBWwRQBX)

![img](https://github.com/user-attachments/assets/f2934a10-1187-4ebf-9152-021b2c804fc0)
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
| `q` | Quit |

## Contri-pug-ting

1. Fork the repo
2. Install dependencies
3. Smash your head against the keyboard (Rust can be like that)
4. Open a pull request

Lua plugin PRs especially welcome — API reference in [`docs.md`](./docs.md).

## License

MIT

![gif](https://github.com/user-attachments/assets/43e4fc61-cd06-43a3-872c-632059467259)

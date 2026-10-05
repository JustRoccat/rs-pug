# rs-pug usage guide

Everything that did not fit in the README: keys, config file, Minimal, EQ, visualizer, IPC, themes, Sonum, storage. Lua plugins live in [`docs.md`](./docs.md).

## Keys

| Key | Action |
|-----|--------|
| `1`-`5` | Switch tabs: Discover, Albums, Library (playlists), Local, Options |
| `Tab` | Switch panel focus |
| `j` / `k` | Move up / down |
| `/` | Search |
| `Enter` | Play (or add all marked songs to the queue, see Multi-select below) |
| `Space` | Pause / Resume |
| `n` / `p` | Next / Previous |
| `m` | Mute |
| `r` | Cycle repeat mode |
| `c` | Context menu |
| `v` | Toggle flat/organized view (Local tab) |
| `Ctrl+V` | Toggle the real FFT spectrum visualizer (needs `parec`) |
| `Shift+Z` | Toggle minimal mode |
| `e` | Edit ID3 tags for selected local file (Local tab) |
| `s` | Cycle local sort mode (Local tab) |
| `g` / `a` | Filter local library by genre / artist (Local tab, Organized view) |
| `b` | Filter by album (Organized view), or mark/unmark for bulk-queue (Flat view / Discover) |
| `F` | Clear local library filters |
| `:` | Open the command palette |
| `?` | Show the full command reference |
| `q` | Quit |

### Remapping keys

`n`, `p`, `m`, `r`, `z`, `[`, `]`, the FFT toggle and the Minimal toggle can be rebound in `~/.config/rs-pug/config.toml`:

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
minimal_toggle = "S-Z"   # Shift+Z
```

Modifiers are prefixed with `C-` (Ctrl), `M-` (Alt), and/or `S-` (Shift), e.g. `C-r` or `M-S-n`. Multi-key sequences are space-separated, e.g. `g g` (press `g` twice within 1.5s). The **Options** tab also lets you remap `next`/`prev`/`mute`/`repeat`/`shuffle`/`seek_back`/`seek_forward` directly, though it's currently limited to single characters there.

### Icons

Tab icons, playback state, song bullets, and status markers use Nerd Fonts glyphs. If your terminal has no Nerd Font, flip the **Options** tab's **Icons** row (or `:` → `icons`, or `icons = false` under `[general]`): icon slots fall back to short ASCII (`>`, `||`, `!`) or disappear where the text speaks for itself.

## Multi-select / bulk queue

Instead of queueing songs one at a time:

1. Press `b` on a song to mark it (in **Discover** results or the **Local** tab's flat view).
2. Keep marking more with `j`/`k`.
3. Press `Enter` to queue every marked song in list order. Playback starts automatically only if nothing was already playing.
4. Press `Esc` to clear marks without queuing anything.

Marks track the song itself, not its list position, so scrolling won't lose them.

## Command palette

`:` opens a fuzzy-searchable palette for playback, volume, repeat/shuffle, seeking, speed, EQ, and tab navigation. Use `↑`/`↓` to pick a result and `Enter` to run it. `?` shows the same list read-only, without running anything.

## Playback speed

Available from the **Options** tab's **Speed** row (0.25x-2.00x, `h`/`l` to adjust in 0.05x steps, `Enter` to reset) or the command palette (`speed up` / `speed down` / `speed reset`). The current speed appears as a badge next to "Now Playing" whenever it isn't 1.00x.

## Equalizer

The 10-band graph in **Options** is interactive once selected:

- `h`/`l` - move between bands
- `+`/`-` - adjust gain of the selected band (-12 dB to +12 dB)
- `p` - cycle EQ presets
- `s` - save current settings, including EQ, to `config.toml`

Custom EQ presets are stored as `.json` files in `~/.config/rs-pug/eqpresets/`.

## FFT visualizer

The "Now Playing" bar always shows an animated spectrum, a synthetic wave by default. Press `Ctrl+V` (or your remapped `fft_toggle`) to switch to a **real** spectrum computed from system audio. `rs-pug` tries these in order:

1. `parec` (PulseAudio, or PipeWire's `pipewire-pulse` compatibility layer): precise per-stream capture via `pactl`
2. `pw-cat --record --raw --monitor`: native PipeWire, captures the default sink's output
3. `pw-record --monitor`: native PipeWire fallback

If none are installed, `rs-pug` silently falls back to the synthetic wave. To enable the real visualizer by default at startup:

```toml
[general]
fft_visualizer_default = true
```

## Minimal mode

`Shift+Z` (or your remapped `minimal_toggle`) hides tabs, lists, options, headers, and labels, leaving only:

- cover art in a tight frame without any title: the real embedded picture (local files and downloads embed it automatically) or the stream thumbnail for YouTube/SoundCloud, fetched in the background,
- the track title, a bare spectrum strip, and a single-line progress bar.

Covers render as pixelated halfblocks through `ratatui-image`: plain terminal cells that work in every terminal, with no Kitty/Sixel graphics protocols involved.

Inside Minimal only a few keys do anything: `Shift+Z` or `Esc` to leave, `Space` to pause, `n`/`p` for next/previous track, `←`/`→` to seek, `9`/`0` for volume, `:` for the palette, `?` for help, `q` to quit. Everything else is swallowed so you can't wander the hidden UI by accident.

### Minimal image background

When a cover is showing, Minimal can tint its whole background with the cover's colors: the three most vivid hues of the art blended into a subtle vertical gradient (dark enough that text stays readable, boosted so even near-black covers tint visibly). Three ways to switch it:

- the **Options** tab's **Image background** row (`h`/`l` or `Enter`),
- the command palette (`:`, then `image background`),
- permanently in the config:

```toml
[general]
image_background = true   # default: true
```

## CLI / IPC

Beyond `--source`, `rs-pug` accepts flags that control an **already-running** instance over a local Unix socket. Handy for `i3status`, `waybar`, or keybinding scripts:

```bash
rs-pug --toggle-pause         # play/pause the running instance
rs-pug --next                 # skip to next track
rs-pug --prev                 # go to previous track
rs-pug --play <path-or-url>   # queue and play a file or URL
```

Each command connects to the running instance's IPC socket and exits immediately. If no instance is running, an error is printed instead of starting a new one.

Pass `--debug` to write logs to `~/.config/rs-pug/rs-pug.log`, useful when filing a bug report.

Pass `--validate` to check `config.toml` without starting the app: it prints the active file, the resolved search source, all custom sources, and any warnings, then exits 0 when clean or 1 when something was skipped or fell back:

```bash
rs-pug --validate
```

## Configuration

Config file: `~/.config/rs-pug/config.toml`. It hot-reloads, so most edits apply without a restart (the Sonum client config below is the exception).

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
> All nine base colors are required. If one is missing, the file fails to parse and `rs-pug` falls back to the built-in palette. `spectrum` is optional and accepts a list of any length, omit it for the default gradient.

Restart or hot-reload to apply changes. Community themes: [all-rspug](https://github.com/JustRoccat/all-rspug/).

### Local music and storage

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
- `api_token` is optional and only needed if the Sonum server was started with `api_token` set in its own `sonum.conf`. When set, `rs-pug` sends it as `Authorization: Bearer <token>` on every request.
- **Restart `rs-pug` after editing this file.** Unlike `config.toml`, it isn't hot-reloaded.

Once configured, switch **Search source** to **Sonum** from the **Options** tab (`h`/`l` to cycle YouTube → SoundCloud → Sonum), or launch with:

```bash
rs-pug --source sonum
```

Searching then queries `GET /tracks?q=<query>&limit=<n>` on the Sonum server; results are mapped into `rs-pug` songs, with playback pointed at the server's `/tracks/:id/stream` endpoint (so `mpv` streams directly from Sonum). The Albums view queries Sonum's `GET /albums` endpoint (same `q`/`limit` params, grouping by album and album-artist server-side) and resolves each album's tracks in disc/track-number order. YouTube album search lists actual playlists (playlist-filtered search); a playlist's tracks load on demand with `e`, `Enter`, or `Tab` (30s timeout each), so searching stays fast. SoundCloud has no playlist search in `yt-dlp`, so it keeps single `full album` videos. Custom search sources hide the Albums tab entirely, since album search isn't supported for them.

> [!NOTE]
> This integration covers *searching, streaming, and cover art* from Sonum (covers come from `GET /tracks/:id/art`, honoring `api_token`). Local downloads, playlists, and the local library scanner are unaffected and continue to work with `~/.config/rs-pug/music-local/` as usual.

### Custom search sources

Besides YouTube, SoundCloud, and Sonum, `rs-pug` can search through user-defined sources declared in `~/.config/rs-pug/config.toml`. They appear in the **Options** tab cycle, in the search prompt, and in `--source`.

```toml
[search]
source = "youtube"

[[search.custom_sources]]
name = "YouTube (newest)"
type = "ytdlp"
prefix = "ytsearchdate"          # becomes "{prefix}{limit}:{query}"

[[search.custom_sources]]
name = "Audius"
type = "command"
command = ["/home/you/.config/rs-pug/scripts/audius-search.py", "{query}", "{limit}"]
timeout_secs = 15                # optional, default 15, 0 means default, max 120
```

where `audius-search.py` queries the public Audius search API and prints the matching tracks as a JSON array:

```python
#!/usr/bin/env python3
"""Usage: audius-search.py QUERY LIMIT."""
import json, sys, urllib.parse, urllib.request

query, limit = sys.argv[1], max(1, int(sys.argv[2]))
url = (
    "https://discoveryprovider.audius.co/v1/tracks/search?query="
    + urllib.parse.quote_plus(query)
    + f"&limit={limit}"
)
req = urllib.request.Request(url, headers={"User-Agent": "rs-pug/1.0"})
with urllib.request.urlopen(req, timeout=20) as response:
    data = json.load(response)
songs = []
for track in data.get("data") or []:
    user = track.get("user") or {}
    songs.append({
        "id": str(track.get("id")),
        "title": track.get("title"),
        "webpage_url": "https://audius.co" + track.get("permalink"),
        "uploader": user.get("handle"),
        "duration": track.get("duration"),
    })
print(json.dumps(songs[:limit]))
```

Python above is just an example: `command` can be anything executable (shell, Ruby, Node, a compiled binary, `curl` against an API that already returns the right shape) as long as it prints the JSON array to stdout.

- `ytdlp` reuses the yt-dlp search path with your prefix. Song URLs are built from `webpage_url` with `url` as fallback. Album search appends `full album` like the built-in sources. The prefix must be a search scheme your yt-dlp understands (`ytsearch`, `scsearch`, and a few others; check with `yt-dlp --list-extractors | grep -i search`). Anything else fails at search time with the yt-dlp error. When in doubt, test first: `yt-dlp --flat-playlist --dump-single-json -- "{prefix}3:test"`.
- `command` runs `command` directly as argv, never through a shell. `{query}` and `{limit}` are substituted per argument. Stdout must be a JSON array of songs with `id`, `title`, `webpage_url` (required) and `uploader`, `duration` (optional, `duration` in seconds). Stdout over 8 MiB is rejected, results are truncated to the search limit, entries with empty `id`/`title`/`webpage_url` are skipped, and album search returns a "not supported" error. A non-zero exit, a timeout (the process is killed), or invalid JSON shows an error naming the source.
- Since there is no shell, pipelines need an explicit `sh -c`, with `{limit}`/`{query}` passed as positional parameters, e.g. `command = ["sh", "-c", "yt-dlp ... \"$0\" ... \"$1\" | jq ...", "{limit}", "{query}"]`.
- Names are matched trimmed and case-insensitive (Unicode lowercase). Empty, duplicate, or built-in-clashing names, empty `prefix`/`command`, unknown types, and entries beyond 255 are skipped with a warning. A saved `source` that no longer exists falls back to YouTube with a warning.
- Cycle order in Options is YouTube -> SoundCloud -> Sonum -> customs. `--source` accepts custom names too (unknown names error out listing the available ones), and `rs-pug --validate` checks the whole setup without starting the app. Like the rest of `config.toml`, custom sources hot-reload.

> [!WARNING]
> `command` sources run programs with your privileges. Only point them at scripts you trust.

### Smart Playlist

On every startup, rs-pug auto-generates and refreshes a single **Smart Playlist** built from three SQLite-backed rules over your local library: most played, recently added, and not-heard-in-a-while tracks (deduped, capped at ~50 songs). It behaves like a normal playlist otherwise, but its contents are replaced on the next launch, so treat it as a rotating mix rather than something to hand-curate. Disable it with:

```toml
[general]
smart_playlists_enabled = false
```

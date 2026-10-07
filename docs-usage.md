# rs-pug usage guide

Everything that didn't fit in the README lives here: keys, sound shaping, remote control, themes, and hooking up your own music sources. Lua plugins have a reference of their own:

[docs.md](./docs.md)

## Keys

| Key | What it does |
|-----|--------|
| 1-5 | Switch tabs: Discover, Albums, Library (playlists), Local, Options |
| Tab | Switch panel focus |
| j / k | Move up / down |
| / | Search |
| Enter | Play (or queue everything you marked, see Multi-select) |
| Space | Pause / Resume |
| n / p | Next / Previous |
| m | Mute |
| r | Cycle repeat mode |
| c | Context menu |
| v | Toggle flat/organized view (Local tab) |
| Ctrl+V | Toggle the real FFT spectrum visualizer |
| Shift+Z | Toggle minimal mode |
| e | Edit tags of the selected local file (Local tab) |
| s | Cycle local sort mode (Local tab) |
| g / a | Filter the local library by genre / artist (Local tab, Organized view) |
| b | Filter by album (Organized view), or mark/unmark for bulk-queue (Flat view / Discover) |
| F | Clear local library filters |
| : | Open the command palette |
| ? | Show the full command reference |
| q | Quit |

### Remapping keys

The transport keys, the volume and seek keys, and both toggles can be rebound. They live in the keybinds section of the config file:

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

A modifier goes in front with a dash. C means Ctrl, M means Alt, and S means Shift, so the last two lines above read as Ctrl+V and Shift+Z. You can also chain keys with a space. Press G twice quickly and it counts as one shortcut, as long as both presses land within about a second and a half.

The Options tab lets you remap the transport and seek keys directly too, though there you're limited to single characters.

### Icons

Tabs, playback state, song bullets, and status markers use Nerd Font symbols. If your terminal doesn't have a Nerd Font, turn icons off wherever is closest:

```
Options tab → Icons row (h/l toggles it)
Command palette → icons
Config file → [general] icons = false
```

Without icons, slots fall back to short plain-text marks, or disappear where the text already says enough:

```
>    ||    !
```

## Multi-select / bulk queue

To queue a bunch of songs at once instead of one by one:

1. Press B on a song to mark it (in Discover results or the Local tab's flat view).
2. Keep marking more with J and K.
3. Press Enter to queue every marked song in list order. Playback starts on its own only if nothing was already playing.
4. Press Esc to clear the marks without queueing anything.

Marks stick to the songs themselves, not to their positions, so scrolling around won't lose them.

## Command palette

The colon key opens a fuzzy-searchable palette with playback, volume, repeat and shuffle, seeking, speed, EQ, and tab navigation. Pick with the up and down arrows, run with Enter. The question-mark key shows the same list read-only, without running anything.

## Playback speed

Find it on the Options tab's Speed row: H and L adjust in small steps from quarter speed up to double, Enter resets to normal. The same three moves exist in the palette:

```
speed up / speed down / speed reset
```

Whenever speed isn't normal, a small badge next to Now Playing says so.

## Equalizer

The 10-band graph in Options wakes up once you select its row. H and L move between bands, plus and minus shape the selected band from minus twelve to plus twelve decibels, P flips through presets, and S writes your current setup, EQ included, to the config file.

Your own presets live as JSON files in the presets folder:

```
~/.config/rs-pug/eqpresets/
```

## FFT visualizer

The Now Playing bar always carries an animated spectrum. What you see by default is a made-up wave that just looks alive. Ctrl+V (or your remapped toggle) swaps in a real spectrum computed from the actual system audio. The app tries three ways to listen in, in this order:

```
1. parec: PulseAudio, or PipeWire through its PulseAudio layer (per-stream capture)
2. pw-cat with monitor flags: native PipeWire, default output
3. pw-record with monitor flags: native PipeWire fallback
```

If none of those exist, you silently get the made-up wave back. To start with the real one every time:

```toml
[general]
fft_visualizer_default = true
```

## Minimal mode

Shift+Z (or your remapped toggle) strips the interface down to almost nothing: no tabs, no lists, no headers. What remains is the cover in a tight frame with no title, the track title, a bare spectrum strip, and a single-line progress bar. Local files and downloads show their real embedded picture; streams show the video thumbnail, fetched quietly in the background.

Covers are drawn from plain terminal cells, so they work in every terminal. No special graphics protocols involved.

While Minimal is on, only a handful of keys do anything: Shift+Z or Esc to leave, Space to pause, N and P to skip tracks, left and right arrows to seek, 9 and 0 for volume, colon for the palette, question mark for help, Q to quit. Everything else is swallowed, so you can't wander into the hidden interface by accident.

### Minimal image background

When a cover is showing, Minimal can wash the whole background in the cover's own colors: the three most vivid hues blended into a subtle vertical gradient, kept dark enough to read over and boosted so even near-black covers still tint visibly. Flip it from the Options tab, the palette, or the config file:

```
Options tab → Image background row (h/l or Enter)
Command palette → image background
Config file → [general] image_background = true
```

It defaults to on.

## CLI / IPC

Besides picking a source at startup, rs-pug can drive an already-running instance through a local socket. That's what status bars and hotkey daemons talk to:

```bash
rs-pug --toggle-pause         # play/pause the running instance
rs-pug --next                 # skip to next track
rs-pug --prev                 # go to previous track
rs-pug --play <path-or-url>   # queue and play a file or URL
```

Each command reaches the running instance and exits right away. If nothing is running, you get an error instead of a new window.

The sockets live next to your other runtime files when the system offers a runtime directory, otherwise inside the app's own config folder, and ending up in a per-user temp folder as a last resort:

```
$XDG_RUNTIME_DIR/rs-pug-ipc.sock     # app control
$XDG_RUNTIME_DIR/rs-pug.sock         # mpv control (changeable in the config)
~/.config/rs-pug/rs-pug-ipc.sock     # when no runtime directory exists
```

Both are created readable only by you. If binding one fails, you'll hear about it. Remote control never dies quietly.

For safety, the play command only accepts web addresses and absolute file paths that actually exist. Anything else is refused, and absurdly long input lines are dropped instead of being buffered forever.

When something misbehaves, a debug flag writes a log file, handy when reporting a bug:

```
rs-pug --debug        # logs to ~/.config/rs-pug/rs-pug.log
```

Another flag checks your setup without starting the app. It prints the active file, the resolved search source, all custom sources, and any warnings, then exits zero when clean or one when something was skipped or fell back:

```bash
rs-pug --validate
```

## Configuration

Every setting lives in one file:

```
~/.config/rs-pug/config.toml
```

Edits apply without a restart. The app watches the file. The Sonum client file below is the one exception.

### Themes

Five themes ship with the app, dark by default:

```
dark, light, nord, gruvbox, mono
```

```toml
[general]
theme = "nord"
```

To roll your own, drop a JSON file into the themes folder:

```
~/.config/rs-pug/themes/
```

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

Then point the config at the filename without its extension:

```toml
[general]
theme = "mytheme"
```

> [!NOTE]
> All nine base colors are required. Miss one and the file won't parse, so the app falls back to the built-in palette. The spectrum list is optional, any length. Leave it out for the default gradient.

Restart or just let the hot-reload pick it up. Community themes live at [all-rspug](https://github.com/JustRoccat/all-rspug/).

### Local music and storage

Out of the box the app scans one folder:

```
~/.config/rs-pug/music-local/
```

Add more folders from the Options tab. Sorting feels natural, metadata is read off the files, and these are the formats it picks up:

```
mp3, flac, wav, ogg, opus, m4a
```

A tilde at the start of a folder path means your home folder, everywhere a folder is expected:

```
~/Music   →   /home/you/Music
```

Playlists and library data live in a small database, and anything from the old JSON layout is moved over automatically on first run:

```
~/.config/rs-pug/pug.db               # the database
~/.config/rs-pug/import_playlist.json # drop a playlist here to import it
~/.config/rs-pug/exports/             # playlists land here on export
```

#### Tag editor

Press E over a local file to fix its tags. Tab walks through the fields, Enter writes everything to the file and closes, Esc bails out without touching anything. The year field only accepts numbers. If it complains, you stay on the field so you can fix the value instead of losing it.

### Sonum (self-hosted music server)

[Sonum](https://github.com/JustRoccat/Sonum) is a small self-hosted music server. It watches a folder on a machine of your choice and serves it over a plain JSON API, with metadata, lyrics, and cover art pulled from the files, rescanning when things change. Point rs-pug at it and it becomes another search source next to YouTube and SoundCloud. Handy when your music lives on a home server and you want it on whatever machine you're at.

On first start the app writes a client file for you, readable only by you since it can hold a secret:

```
~/.config/rs-pug/sonumclient.toml
```

```toml
scheme = "http"
host = "127.0.0.1"
port = 8420
# api_token = "your-secret-token"
```

| Setting | Meaning |
|---------|---------|
| scheme | Plain or encrypted connection |
| host / port | Wherever your Sonum server runs (the defaults match Sonum's own) |
| api_token | Only needed if the server itself demands one; sent along as a bearer token on searching, covers, streaming, and downloads |

A token paired with a non-local address over a plain connection earns you a warning in the log. Use the encrypted scheme there.

Unlike the main config, this file needs an app restart after editing. Then flip the Search source row in Options through YouTube and SoundCloud until Sonum shows up, or start straight into it:

```bash
rs-pug --source sonum
```

Four server endpoints do the work:

```
GET /tracks                searching, with query and limit
/tracks/:id/stream         playback, streamed straight into mpv
GET /albums                the Albums view
GET /tracks/:id/art        covers
```

Server-side, albums arrive already grouped with their tracks in disc and track order. YouTube album search works off real playlists, and a playlist opens its tracks on demand with E, Enter, or Tab (each gets half a minute before it times out, so searching stays snappy). SoundCloud has no playlist search to speak of, so it sticks to single full-album uploads. Command-type custom sources never get an Albums tab. yt-dlp ones get it only when you opt in (see below).

> [!NOTE]
> Searching, streaming, covers, and downloads all work against Sonum. Downloading a Sonum track from the context menu grabs the file straight over HTTP with your token; it gets converted to your download format when that differs from the server's file. Playlists and the local scanner ignore all of this and keep using your local folders.

### Multiple Sonum servers

When one library isn't enough, declare the extras below the main settings. Searching asks every server and pools the answers; one being down just logs a warning and the rest carry on. Playback, covers, and downloads always use the song's own server and its own token:

```toml
[[servers]]
name = "NAS"
host = "192.168.1.50"
port = 8420
# scheme = "http"
# api_token = "other-secret-token"
```

One catch: the player itself accepts a single global header, so when several servers carry different tokens, streaming uses the first one (and says so in the log). Downloads and covers always use the right token per server.

### Downloads

The context menu can save a song to disk. Fetching is done by yt-dlp, so audio extraction and thumbnails need ffmpeg installed (it's in the README requirements).

Tune it in the config or straight from the Options tab. The Download format row cycles with H and L, the Download dir row edits with Enter, and clearing the dir falls back to the music folder:

```toml
[general]
download_format = "mp3"   # mp3 (default), best, m4a, opus, flac
# download_dir = "/home/you/Music"   # default: the first music directory
```

| Setting | Meaning |
|---------|---------|
| download_format | Which container the file ends up in; best keeps whatever the source had instead of re-encoding (handy for lossless originals) |
| download_dir | Where files land; unset means the first music folder, and the Options row shows the resolved path with a music-dir marker until you override it |

Sonum downloads skip yt-dlp entirely and pull the server's file directly. If your chosen format matches the server's file, or you picked best, the original lands untouched with its tags. Otherwise the app converts it with ffmpeg, and if conversion fails you still get the original, with the message saying exactly that. Turning a lossy file into flac only makes it bigger. Quality that was never there can't come back.

### Custom search sources

For anything beyond YouTube, SoundCloud, and Sonum, declare your own sources in the main config. They join the Options cycle, the search prompt, and the startup flag:

```toml
[search]
source = "youtube"

[[search.custom_sources]]
name = "YouTube (newest)"
type = "ytdlp"
prefix = "ytsearchdate"          # becomes "{prefix}{limit}:{query}"
albums = true                    # optional, default false: show the Albums tab (ytdlp only)

[[search.custom_sources]]
name = "Audius"
type = "command"
command = ["/home/you/.config/rs-pug/scripts/audius-search.py", "{query}", "{limit}"]
timeout_secs = 15                # optional, default 15, 0 means default, max 120
```

The script above queries the public Audius search API and prints matching tracks as a JSON array:

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

Python is just an example. Anything executable works (shell, Ruby, Node, a compiled binary, curl against an API that already speaks the right shape) as long as it prints a JSON array to standard output. Each entry carries three required fields and two optional ones, duration in seconds:

```json
[
  { "id": "...", "title": "...", "webpage_url": "...", "uploader": "...", "duration": 123 }
]
```

A few things worth knowing about the two flavors:

- A yt-dlp source reuses the normal yt-dlp search with your prefix dropped in front. Song links come from the webpage address, falling back to the plain URL field. Album search tacks on a full-album suffix like the built-ins. The prefix has to be a search scheme your yt-dlp understands. When in doubt, try it by hand first:

```bash
yt-dlp --flat-playlist --dump-single-json -- "{prefix}3:test"
```

Flip the albums flag on and the Albums tab appears for that source; it stays off by default.

- A command source runs your program directly as arguments, never through a shell. The query and the limit are substituted per argument. Output past eight megabytes is refused, results are cut to the search limit, entries missing their id, title, or address are skipped, and album search answers that it isn't supported. A crash, a timeout (the process gets killed), or broken JSON surfaces as an error naming the source. Since there's no shell involved, pipelines need an explicit shell call with the limit and query handed over as positional parameters:

```toml
command = ["sh", "-c", "yt-dlp ... \"$0\" ... \"$1\" | jq ...", "{limit}", "{query}"]
```

- Names match loosely. Surrounding space is ignored, and so is case. Empty names, duplicates, clashes with built-ins, empty prefixes or commands, unknown types, and anything past 255 entries are skipped with a warning. A saved source that no longer exists falls back to YouTube, also with a warning.
- The Options cycle walks YouTube, SoundCloud, Sonum, then your customs. Custom names work with the startup flag too, and an unknown one gets you the full list of what's available. To check the whole setup without starting the app, there's the validate flag. And like everything else in the main config, custom sources hot-reload.

> [!WARNING]
> Command sources run programs with your privileges. Only point them at scripts you trust.

### Smart Playlist

Every startup rebuilds a single Smart Playlist from three rules over your local library: most played, recently added, and not heard in a while. Duplicates are folded away and it caps around fifty songs. It behaves like any playlist otherwise, but the next launch replaces its contents. Treat it as a rotating mix rather than something to curate by hand. To turn it off:

```toml
[general]
smart_playlists_enabled = false
```

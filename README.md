# rs-pug

terminal music player powered by YouTube. search, queue, listen — no browser, no ads, no nonsense.

uses `mpv` and `yt-dlp` under the hood, with a terminal UI built with ratatui.

![demo placeholder](https://cdn.discordapp.com/attachments/1473377698478031021/1492564717183696991/image.png?ex=69dbcab7&is=69da7937&hm=87bbbd463f1802f0a254779f4f4810a9eaf9920468c77ba3bda964c850780423&)

## dependencies

you'll need:

- `mpv`
- `yt-dlp`

optionally `mpv-mpris` if you want playerctl / media keys support.

## installation

**from AUR (arch linux):**

```bash
yay -S rs-pug-git
# or
paru -S rs-pug-git
```

**manually:**

```bash
git clone https://github.com/coldbrxthe/rs-pug
cd rs-pug
cargo build --release
./target/release/rs-pug
```

## keybinds

| key | action |
|-----|--------|
| `/` | search |
| `Enter` | play |
| `a` | add to queue |
| `n` | next |
| `p` | previous |
| `m` | mute |
| `Space` | pause |
| `r` | repeat mode |
| `Tab` | switch tab |
| `q` | quit |

## plugins (lua)

drop lua scripts into `~/.config/rs-pug/plugins/` and they'll be loaded automatically. plugins receive events (song start, search results, keypresses) and can modify the app's behavior.

example plugin:

```lua
return {
    on_key = function(key, state)
        if key == "char:x" then
            return { flash = "hey!" }
        end
    end
}
```

## configuration

config file is created automatically on first launch at `~/.config/rs-pug/config.toml`.

## license

MIT

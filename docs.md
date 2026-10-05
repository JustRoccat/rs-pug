# rs-pug Lua Plugin Documentation

This document describes the Lua plugin API in `rs-pug`: a terminal music player written in Rust. It targets people writing or maintaining plugins: it contains the full hook reference, data types, execution semantics, and ready-to-copy examples.

If you're looking for keybindings, general configuration, EQ, or Sonum, see `docs-usage.md`. This file covers only the plugin system.

## Table of contents

1. [Prerequisites](#prerequisites)
2. [Quick start](#quick-start)
3. [Execution model](#execution-model)
4. [Configuration](#configuration)
5. [Hook reference](#hook-reference)
6. [Data type reference](#data-type-reference)
7. [Key labels](#key-labels)
8. [Merging results from multiple plugins](#merging-results-from-multiple-plugins)
9. [Diagnostics and error handling](#diagnostics-and-error-handling)
10. [Security and trust boundaries](#security-and-trust-boundaries)
11. [Full examples](#full-examples)
12. [Best practices and limitations](#best-practices-and-limitations)

## Prerequisites

- A working `rs-pug` installation.
- Basic familiarity with Lua (5.4) syntax.
- A plugin directory: `~/.config/rs-pug/plugins/` by default (see [Configuration](#configuration)).

`rs-pug` embeds a **Lua 5.4** interpreter via the `mlua` crate (`mlua = { version = "0.12", features = ["lua54", "vendored", "serialize", "send"] }`). You don't need Lua installed on your system: the interpreter is compiled into the binary.

## Quick start

1. Create `~/.config/rs-pug/plugins/hello.lua`:

   ```lua
   plugin = {}

   function plugin.on_key(key, state)
     if key == "char:h" then
       return {
         consume = true,
         flash = "Hello from a plugin! Active tab: " .. state.active_tab,
         flash_seconds = 3,
       }
     end
   end

   return plugin
   ```

2. Start `rs-pug` (or wait for hot-reload if it's already running: changes in `plugins_dir` are detected automatically).
3. Press `h` in the UI. A flash message with the active tab name should appear.

Every `.lua` file in the configured directory is treated as a **separate, independent plugin** with its own Lua state. The file must return a table (directly via `return` at the end of the chunk, or by setting the global variable `plugin`, as in the example above): this table is the plugin's entry point. Functions defined in this table are **hooks**: `rs-pug` calls them at the appropriate points in the application's lifecycle.

## Execution model

### Loading

At startup, and on every reload (`reload`), `rs-pug`:

1. Reads every file with a `.lua` extension in `plugins_dir` (non-recursively, `fs::read_dir`). Iteration order **is not guaranteed to be alphabetical**: it depends on the filesystem. If the execution order of multiple plugins matters (see [Merging results](#merging-results-from-multiple-plugins)), don't assume any particular order.
2. For each file, creates a **new, independent** `Lua` instance (`mlua::Lua::new()`), which by default loads the **full Lua 5.4 standard library** (`string`, `table`, `math`, `os`, `io`, `package`, `coroutine`, `debug`, etc.). Global variables of one plugin are not visible to another.
3. Executes (`eval`) the file's contents under an execution time limit (see below).
4. Reads the resulting value: if the chunk returned a table (`return plugin`), that table is used; otherwise `rs-pug` looks for a global variable named `plugin`. If neither exists, the plugin is rejected with a `missing plugin table` error.
5. Sets two global variables in the plugin's environment:
   - `ALLOW_LUA_UI_CHANGES` (`boolean`): a copy of the `allow-lua-ui-changes` config flag, readable from inside the script.
   - `plugin`: the entry-point table (also available after a reload, useful if you want to reference it via `plugin.on_key = function(...) ... end` instead of a table literal).
6. The plugin name used in logs and warnings is the file name without its extension (`hello.lua` → `hello`).

Load errors (Lua syntax errors, a missing `plugin` table, exceeding the execution time limit while evaluating the top-level chunk) **do not stop** loading the remaining plugins: the offending file is skipped, and the error is added to the warning list (see [Diagnostics](#diagnostics-and-error-handling)).

### Execution time limit

Every hook call (and the chunk evaluation at load time) is subject to a limit:

- **250 ms** of wall-clock time (`PLUGIN_EXEC_TIMEOUT`),
- checked every **10,000 instructions** of the Lua virtual machine (`PLUGIN_HOOK_INSTRUCTION_INTERVAL`, via `HookTriggers::every_nth_instruction`).

If the limit is exceeded, execution is aborted with `"plugin exceeded execution time limit"`. This applies to **each** hook call individually: it is not a shared budget across calls. Avoid infinite loops, heavy computation, and blocking I/O (`os.execute`, unbuffered `io.read`) inside hooks that are called every frame (`on_ui_update`, `on_ui_sections` via state polling: see below).

### Hot-reload

A change in `config.toml` to `general.plugins_enabled`, `general.plugins_dir`, or `lua.allow-lua-ui-changes` triggers a **full reload** of the plugin manager: every Lua instance is destroyed and re-created, and each plugin's state (global variables, closures) is lost. Modifying a `.lua` file itself inside `plugins_dir` is also detected and triggers the same full reload mechanism used for `config.toml`. After a reload, custom tabs, panels, injected UI sections, and warnings are cleared and rebuilt from scratch.

### How often each hook fires

How frequently a given hook is called differs significantly between hooks: see the "Frequency" column in the [Hook reference](#hook-reference). In short:

- Data-transforming hooks (`on_search_query`, `on_search_results`, `on_song_start`): on a specific user/core action.
- `on_key`: on every key press that wasn't already handled by built-in logic (see [Key labels](#key-labels): some UI modes, e.g. an open tag editor or the command palette, intercept keys before they reach plugins).
- `on_event`: on **every** core event (`CoreEvent`), including every playback progress update (`Progress`), i.e. potentially several times per second during playback.
- `on_tabs`, `on_ui_panels`: polled every render frame whenever `PluginUiState` has changed (see [`PluginUiState`](#pluginuistate)).
- `on_ui_config`: called once at startup (if `allow_lua_ui_changes = true`) and again after every plugin hot-reload.
- `on_ui_update`: polled every frame whenever `PluginUiState` has changed, only if `allow_lua_ui_changes = true`.
- `on_ui_sections`, `on_ui_inject`: polled alongside `on_tabs`/`on_ui_panels`, only if `allow_lua_ui_changes = true`.

## Configuration

All plugin settings live in `~/.config/rs-pug/config.toml` and are hot-reloadable.

```toml
[general]
plugins_enabled = true                 # default: true
plugins_dir = "/home/user/.config/rs-pug/plugins"  # default: ~/.config/rs-pug/plugins

[lua]
allow-lua-ui-changes = false           # default: false
```

| Key | Section | Type | Default | Description |
|---|---|---|---|---|
| `plugins_enabled` | `[general]` | `bool` | `true` | Enables/disables the whole plugin system. When `false`, no `.lua` file is loaded and no hook is ever called. |
| `plugins_dir` | `[general]` | `string` | `~/.config/rs-pug/plugins` | Directory scanned for `.lua` files. Non-recursive. |
| `allow-lua-ui-changes` | `[lua]` | `bool` | `false` | Enables the higher-risk UI hooks: `on_ui_config`, `on_ui_sections`, `on_ui_inject`, `on_ui_update`, and the `ui.layout` field returned from `on_key`/`on_event`. TOML alias: `allow_lua_ui_changes` (underscore) is accepted as a synonym. |

> [!NOTE]
> When `allow-lua-ui-changes = false`, the hooks `on_ui_config`, `on_ui_sections`, `on_ui_inject`, and `on_ui_update` are **not called at all** (they don't just receive empty data: the call is skipped entirely). Any `ui.layout` patch returned from `on_key`/`on_event` is silently ignored in that case. `on_key`, `on_event`, `on_tabs`, `on_ui_panels`, and the data-transforming hooks work regardless of this flag.

At startup, `rs-pug` exposes `allow_lua_ui_changes` to every plugin's environment as the global variable `ALLOW_LUA_UI_CHANGES`, so a script can conditionally define UI hooks:

```lua
plugin = {}

if ALLOW_LUA_UI_CHANGES then
  function plugin.on_ui_config(state)
    return { layout = { queue_width_percent = 30 } }
  end
end

return plugin
```

## Hook reference

All hooks are **optional**: define only the ones you need. A hook value that isn't a function (e.g. a string by mistake) results in a `hook is not a function` warning and is treated as if the hook were absent.

Table convention: **Input** describes the Lua call arguments in order; **Returns** describes the expected shape of the return value (`nil` is always allowed and means "no change").

### `on_search_query(query)`

| | |
|---|---|
| **Input** | `query: string`: the raw query typed by the user, before it's sent to the search source. |
| **Returns** | `string`: the modified query, or `nil` to leave it unchanged. |
| **Frequency** | Once, right before a search is executed (Discover/Albums, YouTube/SoundCloud/Sonum source). |
| **Multi-plugin semantics** | Pipeline: one plugin's result becomes the next plugin's input, in load order. |

```lua
function plugin.on_search_query(query)
  return query .. " official audio"
end
```

### `on_search_results(songs)`

| | |
|---|---|
| **Input** | `songs: Song[]`: the list of search results (see [`Song`](#song)). |
| **Returns** | `Song[]`: the modified list (you can filter, sort, add items), or `nil`. |
| **Frequency** | Once, after search results are received, before display. |
| **Multi-plugin semantics** | Pipeline, as above. |

```lua
function plugin.on_search_results(songs)
  local filtered = {}
  for _, song in ipairs(songs) do
    if not song.title:lower():find("live") then
      table.insert(filtered, song)
    end
  end
  return filtered
end
```

### `on_song_start(song)`

| | |
|---|---|
| **Input** | `song: Song`: the track that is about to start playing. |
| **Returns** | `Song`: the modified song (e.g. a different `webpage_url`), or `nil`. |
| **Frequency** | Once, right before playback of that track begins. |
| **Multi-plugin semantics** | Pipeline, as above. |

> [!WARNING]
> If the returned value fails to deserialize into the expected type (e.g. a required `id`/`title`/`webpage_url` field is missing in `on_song_start`/`on_search_results`), or the hook throws an error / exceeds the time limit, that plugin's result is **silently dropped**: the value passed forward is the value from before that plugin ran, **with no entry in the warning list**. See [Diagnostics](#diagnostics-and-error-handling).

### `on_key(key, state)`

| | |
|---|---|
| **Input** | `key: string`: a key label (see [Key labels](#key-labels)); `state: PluginUiState`: the [UI state](#pluginuistate) at the moment of the key press. |
| **Returns** | [`PluginDispatch`](#plugindispatch), or `nil` to do nothing. |
| **Frequency** | On every key press, provided no modal mode (tag editor, option editing, context menu, delete confirmation, help screen, command palette, search-typing mode) already captured it. |
| **Shift aliases** | For alphabetic keys, `rs-pug` first tries the label that was actually pressed (e.g. `char:Z` for Shift+z); if no plugin produced an effect (`consume`, `flash`, or `flash_seconds` set), it then tries the case-toggled variant (`char:z`). See [Key labels](#key-labels). |
| **Multi-plugin semantics** | Every plugin is called; their `PluginDispatch` values are merged: see [Merging results](#merging-results-from-multiple-plugins). |

If the merged `PluginDispatch.consume` is `false` after querying every plugin, the key falls through to `rs-pug`'s built-in key handling (`input::handle_native_key_event`).

```lua
function plugin.on_key(key, state)
  if key == "char:n" and state.active_tab == "discover" then
    return { core_actions = { { type = "next" } }, consume = true }
  end
end
```

### `on_event(event, state)`

| | |
|---|---|
| **Input** | `event: PluginEvent`: the [core event](#pluginevent); `state: PluginUiState`: the UI state at the moment of the event. |
| **Returns** | [`PluginDispatch`](#plugindispatch), or `nil`. |
| **Frequency** | On **every** `CoreEvent` (see the `kind` table under [`PluginEvent`](#pluginevent)): including `progress`, which occurs multiple times per second during playback. Keep this hook's logic lightweight, or filter on `event.kind` at the very start. |
| **Multi-plugin semantics** | Same as `on_key`: every plugin called, results merged. |

```lua
function plugin.on_event(event, state)
  if event.kind == "started" then
    return { flash = "Now playing: " .. event.message, flash_seconds = 3 }
  end
end
```

### `on_tabs(state)`

| | |
|---|---|
| **Input** | `state: PluginUiState`. |
| **Returns** | `PluginTab[]`: a list of extra tabs shown next to the main tabs, or `nil`. |
| **Frequency** | Polled every frame whenever `state` changed since the previous frame. |
| **Note** | Works **regardless** of `allow-lua-ui-changes`: this is a lighter-weight way to add tabs than `on_ui_config`. Tabs from different plugins are simply concatenated (no deduplication of ids on your behalf). |

```lua
function plugin.on_tabs(state)
  return { { id = "stats", title = "Stats", icon = "*" } }
end
```

### `on_ui_panels(state)`

| | |
|---|---|
| **Input** | `state: PluginUiState`. |
| **Returns** | [`PluginPanel[]`](#pluginpanel), or `nil`. |
| **Frequency** | Polled every frame alongside `on_tabs`. |
| **Note** | Works **regardless** of `allow-lua-ui-changes`. Panels with `target` `main`/`results`/`queue` are shown in the results/queue panel **only while a plugin tab or a custom tab is active** (not on stock tabs like Discover). Panels with `target = "overlay"` always render as a floating window in the top-right corner, regardless of the active tab. |

```lua
function plugin.on_ui_panels(state)
  return {
    {
      title = "Info",
      target = "overlay",
      items = {
        { type = "stat", label = "vol", value = tostring(state.volume) },
        { type = "stat", label = "queue", value = tostring(state.queue_len) },
      },
    },
  }
end
```

### `on_ui_config(state)`: requires `allow-lua-ui-changes = true`

| | |
|---|---|
| **Input** | `state: PluginUiState`. |
| **Returns** | [`PluginUiConfig`](#pluginuiconfig) (`{ tabs = {...}, layout = {...} }`), or `nil`. |
| **Frequency** | Once at application startup and again after every plugin hot-reload. **Not** polled every frame: it's a one-time startup configuration of tab structure and layout. For continuous layout updates, use `on_ui_update`. |
| **Effect** | Replaces the full set of main tabs (`app.main_tabs`) and applies `layout` (same as `on_ui_update`). |

```lua
function plugin.on_ui_config(state)
  return {
    tabs = {
      remove = { "local" },
      custom = { { id = "dash", title = "Dashboard", icon = "*", position = 2 } },
    },
    layout = { queue_width_percent = 30, tab_bar_position = "left" },
  }
end
```

### `on_ui_update(state)`: requires `allow-lua-ui-changes = true`

| | |
|---|---|
| **Input** | `state: PluginUiState`. |
| **Returns** | [`PluginLayoutConfig`](#pluginlayoutconfig) **or** `PluginUiConfig` (the function auto-detects the shape: if the table contains a `layout` or `tabs` key, it's treated as a `PluginUiConfig` and only its `.layout` field is used; otherwise it's treated as a flat `PluginLayoutConfig`), or `nil`. |
| **Frequency** | Polled every frame whenever `state` changed: i.e. it reacts to tab changes, volume, repeat mode, search query, etc. in near real time. |
| **Note** | Does not modify tabs (`tabs`): only `layout`. Use `on_ui_config` to change tabs. |

```lua
function plugin.on_ui_update(state)
  if state.active_tab == "queue" then
    return { queue_width_percent = 60 }  -- flat shape
  end
  return { layout = { queue_width_percent = 40 } }  -- on_ui_config-style shape
end
```

### `on_ui_sections(state)`: requires `allow-lua-ui-changes = true`

| | |
|---|---|
| **Input** | `state: PluginUiState`. |
| **Returns** | A table `{ [section_id]: PluginPanelItem[] }`, or `nil`. |
| **Frequency** | Polled every frame alongside `on_ui_inject`, `on_tabs`, `on_ui_panels`. |
| **Effect** | Supplies the **content** displayed inside custom layout sections defined via `layout.custom_sections` (from `on_ui_config`/`on_ui_update`). The table key must match a section's `id`. A section with no matching entry renders as an empty panel titled with its `id`. |

```lua
function plugin.on_ui_sections(state)
  return {
    hello = {
      { type = "header", text = "Hello" },
      { type = "progress", label = "battery", percent = 72 },
    },
  }
end
```

### `on_ui_inject(state)`: requires `allow-lua-ui-changes = true`

| | |
|---|---|
| **Input** | `state: PluginUiState`. |
| **Returns** | [`PluginUiInject`](#pluginuiinject) (any subset of the fields `results_top`, `results_bottom`, `queue_top`, `queue_bottom`, `statusbar_extra`), or `nil`. |
| **Frequency** | Polled every frame alongside `on_ui_sections`. |
| **Effect** | Inserts `PluginPanelItem` elements at the top/bottom of the results list, top/bottom of the queue, or into the status bar: **regardless** of which tab is active (unlike `on_ui_panels` with `target = "main"/"results"/"queue"`, which only render on plugin/custom tabs). |

```lua
function plugin.on_ui_inject(state)
  return {
    statusbar_extra = {
      { type = "keybind", key = "h", action = "say hi" },
    },
  }
end
```

## Data type reference

All types are serialized/deserialized between Rust and Lua through `mlua`/`serde`'s JSON-like data model: in Lua you represent them as plain tables. Field names on the Lua side match the `serde` names exactly (mostly `snake_case`).

### `Song`

| Field | Lua type | Required | Description |
|---|---|---|---|
| `id` | `string` | yes | Track identifier (a URL for streaming sources, a file path for local files). |
| `title` | `string` | yes | Title shown in the UI. |
| `webpage_url` | `string` | yes | URL/path used for playback. |
| `uploader` | `string \| nil` | no | Artist/channel name. |
| `duration` | `number \| nil` | no | Length in seconds. |

### `PluginUiState`

A snapshot of application state passed to **every** hook that takes a `state` argument. Fields are read-only: you make changes by returning a `PluginDispatch`/`PluginUiConfig`/`PluginLayoutConfig`, not by mutating this table.

| Field | Lua type | Description |
|---|---|---|
| `active_tab` | `string` | Id of the active tab: one of `discover`, `albums`, `library`, `local`, `options`, a custom tab id, or a plugin tab id from `on_tabs`. |
| `active_plugin_tab` | `string \| nil` | Id of the active plugin tab (from `on_tabs`), if one is active. |
| `active_custom_tab` | `string \| nil` | Id of the active custom tab (from `on_ui_config`), if one is active. |
| `active_tab_index` | `number` | Position of the active tab in the tab bar (0-based, includes main and plugin tabs). |
| `current_layout` | [`PluginUiLayoutState`](#pluginuilayoutstate) | Current, effective layout values. |
| `visible_sections` | `string[]` | Ids of currently visible (non-hidden) custom sections. |
| `player_state` | `string` | One of the player state labels (e.g. `"playing"`, `"paused"`, `"idle"`, `"searching"`: the exact text comes from `ui_helpers::player_state_label`). |
| `volume` | `number` | Volume, 0–100. |
| `muted` | `boolean` | Whether audio is muted. |
| `repeat_mode` | `string` | `"off"`, `"one"`, or `"all"` (lowercase repeat-mode label). |
| `search_query` | `string` | Current text in the search field (Discover). |
| `album_search_query` | `string` | Current text in the album search field. |
| `queue_len` | `number` | Number of tracks in the queue. |

#### `PluginUiLayoutState`

| Field | Lua type | Description |
|---|---|---|
| `queue_width_percent` | `number` | Current width of the queue panel, in percent. |
| `visualizer_height` | `number` | Current height of the FFT visualizer, in rows. |
| `tab_bar_position` | `string` | `"top"`, `"bottom"`, `"left"`, or `"right"`. |
| `tabs_width` | `number` | Width of the tab bar, in columns (applies to `left`/`right` positions). |
| `queue_position` | `string` | `"left"` or `"right"`. |

### `PluginEvent`

Represents a core event (`CoreEvent`) passed to `on_event`.

| Field | Lua type | Description |
|---|---|---|
| `kind` | `string` | See table below. |
| `message` | `string \| nil` | Context-dependent text message (depends on `kind`). |
| `value` | `number \| nil` | Context-dependent numeric value (depends on `kind`). |

`CoreEvent` → `kind` / `message` / `value` mapping:

| Core `CoreEvent` | `kind` | `message` | `value` |
|---|---|---|---|
| `Started(song)` | `"started"` | track title | - |
| `SearchDone(songs)` | `"search_done"` | - | number of results |
| `AlbumSearchDone(albums)` | `"album_search_done"` | - | number of results |
| `Progress { position, .. }` | `"progress"` | - | playback position, in seconds |
| `Error(msg)` | `"error"` | error text | - |
| any other (`Paused`, `Resumed`, `TrackFinished`, `VolumeChanged`, `MuteChanged`, `SearchFailed`, `AlbumSearchFailed`, `LibraryRefreshDone`, `DownloadFinished`) | `"event"` | `nil` | `nil` |

> [!TIP]
> Events that aren't mapped to a dedicated `kind` still trigger `on_event` (with `kind = "event"` and no extra data): if you need to react specifically to, say, a volume change, check `state.volume` on every call instead of relying on a dedicated `kind`.

### `PluginDispatch`

The value returned from `on_key` and `on_event`. Every field is optional: omit the fields you don't set (or return `nil` for the whole value to do nothing).

| Field | Lua type | Default | Description |
|---|---|---|---|
| `consume` | `boolean` | `false` | When `true` (after merging all plugins), the key/event is not passed to `rs-pug`'s built-in handling. Applies to `on_key` only. |
| `flash` | `string \| nil` | `nil` | Message shown in the status bar. |
| `flash_seconds` | `number \| nil` | `nil` (effectively 4) | How long `flash` is shown, in seconds. |
| `core_actions` | [`PluginCoreAction`](#plugincoreaction)`[]` | `{}` | A list of actions to run against the player core. |
| `ui` | [`PluginUiPatch`](#pluginuipatch) | `{}` | Point changes to UI state. |

```lua
return {
  consume = true,
  flash = "Paused",
  flash_seconds = 2,
  core_actions = { { type = "toggle_pause" } },
  ui = { set_focus = "queue" },
}
```

### `PluginCoreAction`

A tagged union (`{ type = "...", ... }`) representing a command sent to the playback core (`mpv`). The `type` field is required and is `snake_case`.

| `type` | Extra fields | Description |
|---|---|---|
| `search` | `query: string` | Runs a search as if the query were typed and `Enter` pressed. |
| `search_albums` | `query: string` | Same as above, for the albums view. |
| `seek` | `seconds: number` (integer) | Seeks by the given number of seconds (positive = forward). |
| `toggle_pause` | - | Toggles pause/play. |
| `toggle_mute` | - | Toggles mute. |
| `volume_up` | - | Increases volume by the default step. |
| `volume_down` | - | Decreases volume by the default step. |
| `next` | - | Next track. |
| `prev` | - | Previous track. |
| `set_volume` | `value: number` (0–255, effectively 0–100) | Sets volume to a specific value. |
| `play_url` | `url: string`, `title: string \| nil` | Plays any URL/path, optionally with a custom title. |
| `raw_mpv` | `command: any` (any JSON value) | Sends a raw IPC command to `mpv`, unvalidated by `rs-pug`. |

```lua
core_actions = {
  { type = "search", query = "lofi hip hop" },
  { type = "set_volume", value = 50 },
  { type = "raw_mpv", command = { "set_property", "speed", 1.5 } },
}
```

> [!WARNING]
> `raw_mpv` passes a command straight through to `mpv` via IPC, with no validation from `rs-pug`. A malformed command can destabilize playback for the current session. Only use documented `mpv` JSON IPC commands.

### `PluginUiPatch`

Point changes to UI state returned in the `ui` field of `PluginDispatch`. All fields optional.

| Field | Lua type | Description |
|---|---|---|
| `set_tab` | `string \| nil` | Switches the active tab. Accepts a stock tab name (`discover`, `albums`, `library`, `local`, `options`), a custom tab id (requires `allow-lua-ui-changes = true`), or a plugin tab id from `on_tabs`. |
| `set_search_query` | `string \| nil` | Overwrites the search field's contents (Discover). |
| `set_album_search_query` | `string \| nil` | Overwrites the album search field's contents. |
| `set_focus` | `string \| nil` | `"search"`, `"results"`, or `"queue"`. |
| `set_search_mode` | `boolean \| nil` | Turns query-typing mode on/off. |
| `set_selected_result` | `number \| nil` | 0-based index of the selected search result; clamped to the list's range. |
| `set_selected_album_result` | `number \| nil` | Same, for the album list. |
| `set_selected_queue` | `number \| nil` | 0-based index of the selected queue entry. |
| `layout` | [`PluginUiLayoutPatch`](#pluginuilayoutpatch) | A point change to the layout. Only applied when `allow-lua-ui-changes = true`: silently ignored otherwise. |

### `PluginUiLayoutPatch`

A subset of `PluginLayoutConfig` fields available directly from `on_key`/`on_event` (via `PluginUiPatch.layout`). All fields optional; numeric values are clamped to their allowed range (see [`PluginLayoutConfig`](#pluginlayoutconfig)), with a warning emitted when a value is clamped.

| Field | Lua type |
|---|---|
| `queue_width_percent` | `number \| nil` |
| `visualizer_height` | `number \| nil` |
| `tab_bar_position` | `string \| nil` |
| `tabs_width` | `number \| nil` |
| `queue_position` | `string \| nil` |
| `hide_sections` | `string[]` |
| `show_sections` | `string[]` |

### `PluginPanel`

| Field | Lua type | Required | Description |
|---|---|---|---|
| `title` | `string` | yes | Panel/window title. |
| `target` | `string \| nil` | no | One of `"main"`, `"results"`, `"queue"`, `"overlay"`. Missing = treated like `"main"`/`"results"` (shown in the results panel on a plugin/custom tab). |
| `lines` | `string[]` | no | **Deprecated.** A list of plain text lines. If `items` is empty and `lines` isn't, each line is automatically converted to `{ type = "text", text = <line> }`. |
| `items` | [`PluginPanelItem`](#pluginpanelitem)`[]` | no | The preferred way to describe a panel's content. |

### `PluginPanelItem`

A tagged union. Used both in `PluginPanel.items` and in `on_ui_sections`/`on_ui_inject` return values.

| `type` | Extra fields | Rendering |
|---|---|---|
| `text` | `text: string` | A plain text line. |
| `info` | `text: string` | A line in the "info" color. |
| `option` | `key: string`, `value: string` | `"key: value"`, `key` in the warning color. |
| `stat` | `label: string`, `value: string` | `"label value"`, `value` in the "ok" color. |
| `separator` | - | A horizontal separator line. |
| `header` | `text: string` | A bold heading. |
| `keybind` | `key: string`, `action: string` | `"key → action"`. |
| `progress` | `label: string \| nil`, `percent: number` | An ASCII progress bar (10 segments) plus a percentage value. `percent` is clamped to 0–100. |

```lua
items = {
  { type = "header", text = "Stats" },
  { type = "stat", label = "volume", value = tostring(state.volume) },
  { type = "separator" },
  { type = "progress", label = "buffer", percent = 87 },
}
```

### `PluginTab`

| Field | Lua type | Required | Description |
|---|---|---|---|
| `id` | `string` | yes | Unique tab identifier. |
| `title` | `string` | yes | Display name. |
| `icon` | `string \| nil` | no | Icon/glyph shown next to the title. |

### `PluginUiConfig`

Returned from `on_ui_config` (and partly recognized in `on_ui_update`).

```lua
{
  tabs = {
    remove = { "local" },                 -- string[]: main tab ids to remove
    order = { "options", "discover" },     -- string[]: full new tab order (remaining tabs appended at the end)
    rename = {                             -- { [id]: { title?, icon? } }
      discover = { title = "Search", icon = "*" },
    },
    custom = {                             -- PluginCustomTab[]
      { id = "dash", title = "Dashboard", icon = "*", position = 2 },
    },
  },
  layout = { --[[ PluginLayoutConfig, see below ]] },
}
```

`PluginCustomTab`: `id: string` (required, non-empty, unique), `title: string` (required), `icon: string | nil`, `position: number | nil` (1-based position; out-of-range values are clamped, with a warning).

> [!NOTE]
> If `tabs.remove` removes every main tab (and `custom` doesn't replace them), `rs-pug` restores the default tab set and emits a warning: the UI is never left with no tabs at all.

### `PluginLayoutConfig`

Returned directly from `on_ui_update` (flat shape) or as the `.layout` field from `on_ui_config`/`on_ui_update` (nested shape). All fields optional: only fields you set are applied; the rest keep their current value.

| Field | Lua type | Range / clamping | Description |
|---|---|---|---|
| `queue_width_percent` | `number \| nil` | 10–90 | Width of the queue panel, in percent. |
| `visualizer_height` | `number \| nil` | 0–10 | Height of the FFT visualizer, in rows. |
| `show_progress_bar` | `boolean \| nil` | - | Visibility of the playback progress bar. |
| `show_volume_bar` | `boolean \| nil` | - | Visibility of the volume bar. |
| `show_statusbar` | `boolean \| nil` | - | Visibility of the status bar. |
| `show_keybind_hints` | `boolean \| nil` | - | Visibility of keybinding hints. |
| `tab_bar_position` | `string \| nil` | `"top"`, `"bottom"`, `"left"`, `"right"` (other values rejected with a warning) | Position of the tab bar. |
| `tabs_width` | `number \| nil` | 12–40 | Width of the tab bar (for `left`/`right` positions). |
| `queue_position` | `string \| nil` | `"left"`, `"right"` (other values rejected with a warning) | Which side the queue renders on. |
| `hide` | `string[]` | `"visualizer"`, `"progress_bar"`, `"volume_bar"`, `"statusbar"`, `"keybind_hints"` (other values ignored with a warning) | Hides fixed UI elements (a shortcut for the corresponding `show_*`/`visualizer_height` fields). |
| `custom_sections` | [`PluginCustomSection`](#plugincustomsection)`[]` | - | Defines new UI areas, populated by `on_ui_sections`. |
| `hide_sections` | `string[]` | any custom section `id` | Hides the given custom sections. |
| `show_sections` | `string[]` | any custom section `id` | Reveals the given sections (removes them from the hidden list). |

> [!NOTE]
> `hide`/`show_*` on `PluginLayoutConfig` (fixed UI elements: progress bar, volume bar, status bar, hints, visualizer) is a **different mechanism** from `hide_sections`/`show_sections` (your own custom sections from `custom_sections`). Don't mix identifiers between the two.

### `PluginUiInject`

Returned from `on_ui_inject`. All fields are lists of [`PluginPanelItem`](#pluginpanelitem), empty by default.

| Field | Where it renders |
|---|---|
| `results_top` | At the top of the search results list (Discover/Albums), regardless of the active tab. |
| `results_bottom` | At the bottom of the results list. |
| `queue_top` | At the top of the queue panel. |
| `queue_bottom` | At the bottom of the queue panel. |
| `statusbar_extra` | Appended next to the latest warning in the status bar. |

### `PluginCustomSection`

An entry in the `layout.custom_sections` list of `PluginLayoutConfig`.

| Field | Lua type | Required | Default | Description |
|---|---|---|---|---|
| `id` | `string` | yes (non-empty, unique) | - | Section identifier. Also used as the key in `on_ui_sections`'s return value and in `hide_sections`/`show_sections`. |
| `position` | `string` | no | `"below_player"` | One of `"above_player"`, `"below_player"`, `"left"`, `"right"`. Any other value is rejected with a warning. |
| `width` | `number \| nil` | no | - | Width in columns (for `left`/`right`; if omitted, split evenly among sections at the same position). |
| `height` | `number \| nil` | no | 3 | Height in rows. |
| `content` | `string \| nil` | no | - | **Reserved, currently unused by the renderer.** The section's actual content comes exclusively from `on_ui_sections`, returned under a key equal to this section's `id`. |

```lua
layout = {
  custom_sections = {
    { id = "clock", position = "above_player", height = 1 },
  },
}
```

## Key labels

The `key` argument passed to `on_key` is a string generated by `ui_helpers::describe_key_event_with_modifiers`. Full mapping table:

| `KeyCode` | Label | Notes |
|---|---|---|
| Alphanumeric character/symbol `c` | `char:c` | Case reflects the key combination actually pressed (Shift changes the case for letters `a`-`z`). |
| `Enter` | `enter` | |
| `Esc` | `esc` | |
| `Tab` | `tab` | |
| `Backspace` | `backspace` | |
| `←` | `left` | |
| `→` | `right` | |
| `↑` | `up` | |
| `↓` | `down` | |
| `PageUp` | `page_up` | |
| `PageDown` | `page_down` | |
| `F1`–`F12` | `f1`…`f12` | |
| any other key code (e.g. `Home`, `End`, `Delete`) | `other` | Indistinguishable from each other: they all map to the same `"other"` label. |

**Ctrl and Alt modifiers are not encoded in the label passed to `on_key`.** `Ctrl+r` and plain `r` produce the same base label `char:r` (the code path that feeds `on_key` only inspects the Shift modifier, via the letter's case). If your plugin needs to distinguish Ctrl/Alt combinations, that's not possible through `on_key` in the current API: consider using `rs-pug`'s built-in key remapping in `config.toml` (`[keybinds]`, see `docs-usage.md`) instead, which encodes modifiers as `C-`/`M-`/`S-` prefixes at the `rs-pug` level, outside the plugin system.

**Shift alias for letters:** for `KeyCode::Char` values that are ASCII letters, `rs-pug` generates a two-element label list: the primary one (actually pressed) and a "toggled"-case one. `dispatch_key_with_aliases` first tries the primary label against every plugin; if **no** plugin produced an effect (`consume`, `flash`, or `flash_seconds`), it then tries the second label. This means a plugin listening on `char:z` (lowercase) will still fire even in contexts where a bare `z` press is already handled elsewhere as a Shift toggle: in practice, **you usually only need to handle one case variant**, and the system will try the other automatically if the first produced no effect.

## Merging results from multiple plugins

When more than one `.lua` file is present in the directory, `rs-pug` calls the given hook on **every** loaded plugin and merges the results. The merge rules differ per hook: this is a common source of bugs when writing several cooperating plugins.

| Hook | Merge strategy |
|---|---|
| `on_search_query`, `on_search_results`, `on_song_start` | **Pipeline.** Plugin *N*'s result becomes plugin *N+1*'s input, in file-load order. |
| `on_key`, `on_event` (`PluginDispatch` scalar fields: `flash`, `flash_seconds`, `ui.set_tab`, `ui.set_search_query`, `ui.set_album_search_query`, `ui.set_focus`, `ui.set_search_mode`, `ui.set_selected_result`, `ui.set_selected_album_result`, `ui.set_selected_queue`) | **First plugin that sets the field wins** (later calls setting the same field no longer overwrite an already-set value). |
| `on_key`, `on_event` (`consume`) | **Logical OR**: it's enough for one plugin to return `true`. |
| `on_key`, `on_event` (`core_actions`) | **Concatenation**: actions from every plugin are collected and all executed, in load order. |
| `on_key`, `on_event` (`ui.layout`, i.e. [`PluginUiLayoutPatch`](#pluginuilayoutpatch)) | **Last plugin that sets the field wins** (the opposite of the rest of `PluginDispatch`!). The `hide_sections`/`show_sections` lists are concatenated. |
| `on_tabs` | **Concatenation** of every returned list, no `id` deduplication. |
| `on_ui_panels` | **Concatenation** of every returned list. |
| `on_ui_config` (`tabs.remove`) | **Sum** (concatenation) of removals from every plugin. |
| `on_ui_config` (`tabs.order`) | **Last plugin with a non-empty `order` list wins.** |
| `on_ui_config` (`tabs.rename`) | **Map merge**: on a conflict over the same `id`, **the last plugin wins** (`HashMap::extend`). |
| `on_ui_config` (`tabs.custom`) | **Concatenation.** |
| `on_ui_config`/`on_ui_update` (`layout`, scalar fields of [`PluginLayoutConfig`](#pluginlayoutconfig)) | **Last plugin that sets the field wins** (`Some` overwrites a previous `Some`). |
| `layout.hide`, `layout.hide_sections`, `layout.show_sections`, `layout.custom_sections` | **Concatenation.** |
| `on_ui_sections` | **Overwrite by key**: if two plugins return an entry for the same `section_id`, **the last one wins** (the whole entry, not a per-item merge). |
| `on_ui_inject` | **Concatenation** of all five lists across every plugin. |

> [!IMPORTANT]
> Note the asymmetry: scalar fields in `PluginDispatch.ui` (e.g. `set_tab`) follow "first wins", while `PluginDispatch.ui.layout` and `PluginLayoutConfig` follow "last wins". If you're writing several cooperating plugins, assign clear ownership of each field between them instead of relying on load order (which, again, is not guaranteed: see [Execution model](#execution-model)).

## Diagnostics and error handling

`rs-pug` keeps an internal queue of up to 100 warnings (`PluginWarning`), of which only the **most recent** is ever shown in the UI (in the status bar, until replaced by the next one; deduplication: an identical message in a row is not added again, and the queue keeps up to the 20 most recent unique entries visible in `app.plugin_ui.warnings`). Each entry has a level (`WARN`/`ERROR`) and this format:

```
Lua ERROR [plugin_name.hook_name]: error text
```

### Situations that are reported as a warning/error

- A syntax or execution error in the top-level chunk while loading the file (`load failed: ...`).
- No `plugin` table after evaluating the chunk (`missing plugin table: ...`).
- A hook exists but isn't a function (`hook is not a function`).
- A runtime error or exceeding the time limit **inside**: `on_ui_config`, `on_ui_sections`, `on_ui_inject`, `on_ui_update` (`hook call failed: ...`).
- An invalid returned shape (doesn't match the expected type) from: `on_key`/`on_event` (`invalid dispatch return: ...`), `on_ui_config`/`on_ui_sections`/`on_ui_inject`/`on_ui_update` (`invalid return shape: ...`).
- Unknown ids in `tabs.remove`/`tabs.rename`/`tabs.order`, a missing/duplicate custom tab `id`, an out-of-range `custom.position` (`on_ui_config`).
- A numeric value clamped outside its allowed range in the layout layer (`queue_width_percent`, `visualizer_height`, `tabs_width`: see [`PluginLayoutConfig`](#pluginlayoutconfig)).
- An unknown `tab_bar_position`/`queue_position` value.
- An unknown element in `layout.hide`.
- A missing/invalid `position` in `custom_sections`, or a duplicate section `id`.
- Removing every main tab without replacing them with custom tabs.

### Situations that are silently swallowed (no warning entry)

This is an important implementation asymmetry to be aware of while debugging:

- A runtime error or exceeding the time limit **inside** the hooks: `on_search_query`, `on_search_results`, `on_song_start`, `on_key`, `on_event`, `on_tabs`, `on_ui_panels`: in these hooks, an error simply causes that plugin's result to be skipped, **with no message anywhere in the UI or the warning queue**.
- A shape mismatch in the value returned from `on_search_query`/`on_search_results`/`on_song_start`: the result is silently discarded, and the value from before that plugin ran is passed forward unchanged.

> [!TIP]
> If a hook from the second group "isn't working" and no warning ever appears, the most likely cause is a Lua syntax error inside the hook body, a typo in a field name of the returned table, or an infinite loop hitting the 250 ms limit. Run `rs-pug --debug` (logs to `~/.config/rs-pug/rs-pug.log`) and test the hook's logic in a standalone Lua interpreter before wiring it into `rs-pug`, to rule out syntax errors first.

## Security and trust boundaries

- Every plugin runs with **full access to the Lua 5.4 standard library**, including `io` (reading/writing any file the user has permission for) and `os` (including `os.execute`, which runs arbitrary shell commands). `rs-pug` **applies no additional sandbox** beyond the per-call execution time limit.
- Plugins run with the same system privileges as the `rs-pug` process: i.e. whatever user account is running it.
- Only install `.lua` files from sources you trust. A `.lua` file dropped into `plugins_dir` is executed automatically at the next startup or hot-reload, with no user confirmation.
- The 250 ms per-call limit protects the UI from hanging on a runaway script, but it does **not** protect against malicious actions that fit within that window (e.g. a single file write/read, a single `os.execute`).
- `raw_mpv` (see [`PluginCoreAction`](#plugincoreaction)) forwards an arbitrary JSON structure directly to `mpv`'s IPC, unvalidated: treat it as an extension of the trust surface identical to direct access to `mpv`'s IPC socket.

## Full examples

### 1. A search-transforming plugin (pipeline)

```lua
-- ~/.config/rs-pug/plugins/search_boost.lua
plugin = {}

function plugin.on_search_query(query)
  -- Append context to every query on the Discover tab.
  return query .. " lyrics"
end

function plugin.on_search_results(songs)
  -- Drop results longer than 10 minutes (likely mixes/compilations).
  local filtered = {}
  for _, song in ipairs(songs) do
    if not song.duration or song.duration <= 600 then
      table.insert(filtered, song)
    end
  end
  return filtered
end

return plugin
```

### 2. A plugin with custom keybindings and core actions

```lua
-- ~/.config/rs-pug/plugins/quick_seek.lua
plugin = {}

local SEEK_STEP = 15

function plugin.on_key(key, state)
  if key == "char:[" then
    return {
      consume = true,
      core_actions = { { type = "seek", seconds = -SEEK_STEP } },
      flash = "-" .. SEEK_STEP .. "s",
      flash_seconds = 1,
    }
  end
  if key == "char:]" then
    return {
      consume = true,
      core_actions = { { type = "seek", seconds = SEEK_STEP } },
      flash = "+" .. SEEK_STEP .. "s",
      flash_seconds = 1,
    }
  end
end

return plugin
```

### 3. A plugin with an info overlay panel (no `allow-lua-ui-changes` needed)

```lua
-- ~/.config/rs-pug/plugins/now_playing_overlay.lua
plugin = {}

function plugin.on_ui_panels(state)
  return {
    {
      title = "Now",
      target = "overlay",
      items = {
        { type = "stat", label = "tab", value = state.active_tab },
        { type = "stat", label = "state", value = state.player_state },
        { type = "stat", label = "volume", value = tostring(state.volume) .. "%" },
        { type = "separator" },
        { type = "stat", label = "queue", value = tostring(state.queue_len) },
      },
    },
  }
end

return plugin
```

### 4. A plugin that reshapes the layout and adds a tab (requires `allow-lua-ui-changes = true`)

```lua
-- ~/.config/rs-pug/plugins/compact_ui.lua
plugin = {}

if not ALLOW_LUA_UI_CHANGES then
  return plugin  -- UI changes not permitted in config; don't register UI hooks
end

function plugin.on_ui_config(state)
  return {
    tabs = {
      order = { "discover", "queue-stats", "options" },
      custom = {
        { id = "queue-stats", title = "Queue+", icon = "*", position = 2 },
      },
    },
    layout = {
      queue_width_percent = 35,
      show_keybind_hints = false,
      custom_sections = {
        { id = "clock", position = "above_player", height = 1 },
      },
    },
  }
end

function plugin.on_ui_sections(state)
  return {
    clock = {
      { type = "text", text = os.date("%H:%M:%S") },
    },
  }
end

function plugin.on_ui_update(state)
  -- Shrink the queue panel when it's empty, widen it when it has tracks.
  if state.queue_len == 0 then
    return { queue_width_percent = 20 }
  end
  return { queue_width_percent = 35 }
end

return plugin
```

## Best practices and limitations

- **Filter early.** `on_event` fires on every `CoreEvent`, including several times per second during `progress`. Check `event.kind` at the very top of the function before doing anything expensive.
- **Don't assume file order.** Plugin load order within the directory is not guaranteed. If your plugins must cooperate in a specific order, consider merging them into a single `.lua` file.
- **Remember the `layout` merge asymmetry.** `PluginDispatch.ui` scalar fields: first wins; `ui.layout`/`PluginLayoutConfig` scalar fields: last wins. See [Merging results](#merging-results-from-multiple-plugins).
- **Return `nil`, not an empty table, when nothing changes.** For most hooks this is semantically equivalent, but for `on_ui_sections`/`on_ui_config`/`on_ui_update` it more clearly communicates intent and avoids overwriting another plugin's state with an empty value where "last wins" applies.
- **Validate input shapes if you write a transforming hook.** `Song`, `PluginUiState`, etc. have fixed fields: a typo in a field name while building a new `Song` table in `on_song_start` results in the returned value being **silently** dropped (see [Diagnostics](#diagnostics-and-error-handling)).
- **Budget your execution time.** 250 ms per call sounds generous, but hooks polled every frame (`on_ui_update`, `on_ui_sections`, `on_tabs`, `on_ui_panels`) are called many times per second whenever state changes: avoid I/O, `os.execute`, or large table allocations inside them.
- **Keep `content` out of `PluginCustomSection`.** That field is currently unused by the renderer: actual section content must come from `on_ui_sections`.
- **`Ctrl`/`Alt` aren't distinguishable in `on_key`.** If you need to react to modifier combinations, use `rs-pug`'s built-in key remapping in `config.toml` instead of trying to detect them in Lua.
- **Test outside `rs-pug` first.** Since runtime errors in many hooks are swallowed silently, you'll find typos and logic bugs faster by running the code fragment in a standalone `lua5.4` interpreter first.

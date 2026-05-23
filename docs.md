# rs-pug Lua Plugin API

This document explains how to build Lua plugins for `rs-pug`, including live plugin panels (`on_ui_panels`) and dynamic plugin tabs (`on_tabs`).

## Plugin location

Place `.lua` files in:

- `~/.config/rs-pug/plugins/`

A plugin should return a table with hooks:

```lua
plugin = {}

function plugin.on_key(key, state)
  if key == "char:h" then
    return { flash = "Hello from Lua" }
  end
end

return plugin
```

## Available hooks

Implement any subset you need.

### `on_key(key, state) -> PluginDispatch|nil`

Called on every key press.

### `on_event(event, state) -> PluginDispatch|nil`

Called on core events.

Common `event.kind` values:

- `"started"`
- `"search_done"`
- `"album_search_done"`
- `"progress"`
- `"error"`
- `"event"` (fallback)

### `on_search_query(query) -> string|nil`

Modify outgoing search query.

### `on_search_results(songs) -> songs|nil`

Modify search results.

### `on_song_start(song) -> song|nil`

Modify track metadata/URL before playback starts.

### `on_ui_panels(state) -> PluginPanel[]|nil`

Return live panels rendered on the right side of the UI.

### `on_tabs(state) -> PluginTab[]|nil`

Return dynamic plugin tabs.

Each `PluginTab` has:

- `id` (unique identifier)
- `title` (tab label)
- `icon` (optional icon)

## PluginUiState (`state`)

The `state` object passed to `on_key`, `on_event`, `on_ui_panels`, and `on_tabs` includes:

- `active_tab`: `discover | albums | library | local | options`
- `active_plugin_tab`: plugin tab id (string) or `nil` when no plugin tab is active
- `player_state`: `idle | searching | playing | paused`
- `volume`: `0..100`
- `muted`: `true/false`
- `repeat_mode`: `off | one | all`
- `search_query`
- `album_search_query`
- `queue_len`

## PluginDispatch

Optional return from `on_key`/`on_event`:

- `consume` (bool)
- `flash` (string)
- `flash_seconds` (number)
- `core_actions` (array)
- `ui` (UI patch)

### Core actions (`core_actions`)

Each action has `type`:

- `{ type = "search", query = "..." }`
- `{ type = "search_albums", query = "..." }`
- `{ type = "seek", seconds = 10 }`
- `{ type = "toggle_pause" }`
- `{ type = "toggle_mute" }`
- `{ type = "volume_up" }`
- `{ type = "volume_down" }`
- `{ type = "next" }`
- `{ type = "prev" }`
- `{ type = "set_volume", value = 50 }`
- `{ type = "play_url", url = "...", title = "..." }`
- `{ type = "raw_mpv", command = { ... } }`

### UI patch (`ui`)

- `set_tab`: core tab (`discover|albums|library|options`) or plugin tab id
- `set_search_query`
- `set_album_search_query`
- `set_focus`: `search | results | queue`
- `set_search_mode`: bool
- `set_selected_result`: number
- `set_selected_album_result`: number
- `set_selected_queue`: number

## PluginPanel

`on_ui_panels` returns an array of panels:

```lua
{
  {
    title = "My Panel",
    items = {
      { type = "text", text = "hello" },
      { type = "info", text = "network ok" },
      { type = "option", key = "Source", value = "YouTube" },
      { type = "stat", label = "Queue", value = "12" },
      { type = "separator" }
    }
  }
}
```

### Item types

- `text`
- `info`
- `option` (`key/value`)
- `stat` (`label/value`)
- `separator`

### Backward compatibility

Legacy format still works:

```lua
{ title = "Legacy", lines = { "line 1", "line 2" } }
```

`lines` are automatically converted to `items` (`text`).

## Plugin tabs

Example:

```lua
return {
  on_tabs = function(state)
    return {
      { id = "my_settings", title = "My Settings", icon = "★" },
      { id = "diag", title = "Diagnostics", icon = "!" }
    }
  end
}
```

Open a plugin tab from dispatch:

```lua
ui = { set_tab = "my_settings" }
```

Keyboard navigation:

- `1..5` = built-in tabs
- `6,7,8...` = plugin tabs in `on_tabs` order

## Examples

### 1) Keybind + action

```lua
return {
  on_key = function(key, state)
    if key == "char:v" then
      return {
        consume = true,
        flash = "Volume: 50%",
        core_actions = {
          { type = "set_volume", value = 50 }
        }
      }
    end
  end
}
```

### 2) Live panel with options/stats

```lua
return {
  on_ui_panels = function(state)
    return {
      {
        title = "Session",
        items = {
          { type = "option", key = "Tab", value = state.active_tab },
          { type = "option", key = "State", value = state.player_state },
          { type = "stat", label = "Volume", value = tostring(state.volume) .. "%" },
          { type = "stat", label = "Queue", value = tostring(state.queue_len) },
          { type = "separator" },
          { type = "info", text = "Plugin panel live" }
        }
      }
    }
  end
}
```

### 3) Event-driven diagnostics

```lua
local last_error = "none"

return {
  on_event = function(event, state)
    if event.kind == "error" and event.message then
      last_error = event.message
    end
  end,

  on_ui_panels = function(state)
    return {
      {
        title = "Diagnostics",
        items = {
          { type = "text", text = "muted: " .. tostring(state.muted) },
          { type = "info", text = "last error: " .. last_error }
        }
      }
    }
  end
}
```

### 4) Pseudo-tab flow inside Options

```lua
local plugin_tab_open = false
local quality_idx = 1
local qualities = { "low", "medium", "high" }
local normalize_audio = true

local function current_quality()
  return qualities[quality_idx]
end

return {
  on_key = function(key, state)
    if key == "char:t" then
      plugin_tab_open = not plugin_tab_open
      return {
        consume = true,
        ui = { set_tab = "options" },
        flash = plugin_tab_open and "Plugin tab: ON" or "Plugin tab: OFF"
      }
    end

    if not plugin_tab_open then
      return nil
    end

    if key == "left" then
      quality_idx = math.max(1, quality_idx - 1)
      return { consume = true }
    elseif key == "right" then
      quality_idx = math.min(#qualities, quality_idx + 1)
      return { consume = true }
    elseif key == "char:n" then
      normalize_audio = not normalize_audio
      return { consume = true }
    end
  end,

  on_ui_panels = function(state)
    if not plugin_tab_open then
      return nil
    end

    return {
      {
        title = "Plugin Settings",
        items = {
          { type = "info", text = "Pseudo-tab active in Options" },
          { type = "separator" },
          { type = "option", key = "Quality", value = current_quality() },
          { type = "option", key = "Normalize", value = tostring(normalize_audio) },
          { type = "text", text = "left/right: quality" },
          { type = "text", text = "n: toggle normalize" },
          { type = "text", text = "t: close plugin tab" }
        }
      }
    }
  end
}
```

### 5) Full dynamic tab example (tab + options + panel)

Save as `~/.config/rs-pug/plugins/radio_tab.lua`:

```lua
local genre_idx = 1
local genres = { "lofi", "jazz", "synthwave", "ambient" }
local autoplay = true
local tab_id = "radio"

local function genre()
  return genres[genre_idx]
end

return {
  on_tabs = function(state)
    return {
      { id = tab_id, title = "Radio", icon = "📻" }
    }
  end,

  on_key = function(key, state)
    if key == "char:6" then
      return {
        consume = true,
        ui = { set_tab = tab_id },
        flash = "Opened Radio tab"
      }
    end

    if state.active_plugin_tab ~= tab_id then
      return nil
    end

    if key == "left" then
      genre_idx = math.max(1, genre_idx - 1)
      return { consume = true, flash = "Genre: " .. genre() }
    elseif key == "right" then
      genre_idx = math.min(#genres, genre_idx + 1)
      return { consume = true, flash = "Genre: " .. genre() }
    elseif key == "char:a" then
      autoplay = not autoplay
      return { consume = true, flash = "Autoplay: " .. tostring(autoplay) }
    elseif key == "enter" then
      return {
        consume = true,
        flash = "Searching radio: " .. genre(),
        core_actions = {
          { type = "search", query = genre() .. " radio" }
        }
      }
    end
  end,

  on_ui_panels = function(state)
    return {
      {
        title = "Radio Control",
        items = {
          { type = "option", key = "Genre", value = genre() },
          { type = "option", key = "Autoplay", value = tostring(autoplay) },
          { type = "stat", label = "Queue", value = tostring(state.queue_len) },
          { type = "separator" },
          { type = "text", text = "left/right: change genre" },
          { type = "text", text = "a: toggle autoplay" },
          { type = "text", text = "enter: search station" }
        }
      }
    }
  end
}
```


## Panel placement

By default, plugin panels are rendered in the normal plugin tab content area (left/main pane) when your plugin tab is active.

If you want a floating top-right panel, set:

```lua
{
  title = "My Overlay",
  target = "overlay",
  items = { { type = "text", text = "hello" } }
}
```

Supported targets:

- `main` or `results` (default/main list area in active plugin tab)
- `queue` (right pane list area in active plugin tab)
- `overlay` (optional floating panel in the top-right corner)

Example (render in right pane instead of overlay):

```lua
{
  title = "Plugin Help",
  target = "queue",
  items = { { type = "text", text = "Use ↑/↓ and Enter" } }
}
```


## Plugin manager example (without floating overlay window)

This example renders everything in normal tab panes (left/right lists), not in the top-right floating overlay.

Save as `~/.config/rs-pug/plugins/plugin_manager.lua`:

```lua
local plugin_list = {
  {
    name = "discord_rich_presence.lua",
    url = "https://raw.githubusercontent.com/JustRoccat/all-rspug/main/plugins/discord_rich_presence.lua",
  },
  {
    name = "hq.lua",
    url = "https://raw.githubusercontent.com/JustRoccat/all-rspug/main/plugins/hq.lua",
  },
}

local selected_idx = 1
local tab_id = "plugin_manager"
local install_path = (os.getenv("HOME") or "") .. "/.config/rs-pug/plugins/"

local function install_plugin(plugin)
  local cmd = string.format("curl -L -s -o '%s%s' '%s'", install_path, plugin.name, plugin.url)
  os.execute(cmd)
end

local function build_results_items()
  local items = {
    { type = "info", text = "--- RS-PUG PLUGIN MANAGER ---" },
    { type = "separator" },
    { type = "text", text = "Available plugins:" },
    { type = "separator" },
  }

  for i, p in ipairs(plugin_list) do
    local label = (i == selected_idx) and ("[▶] " .. p.name) or ("    " .. p.name)
    table.insert(items, { type = "text", text = label })
  end

  return items
end

local function build_queue_items()
  return {
    { type = "text", text = "Controls:" },
    { type = "text", text = "↑/↓ : Select plugin" },
    { type = "text", text = "ENTER: Install plugin" },
    { type = "text", text = "6: Open Plugins tab" },
  }
end

return {
  on_tabs = function(state)
    return {
      { id = tab_id, title = "Plugins", icon = "🔌" },
    }
  end,

  on_key = function(key, state)
    if key == "char:6" then
      return {
        consume = true,
        ui = { set_tab = tab_id },
        flash = "Plugin Manager",
      }
    end

    if state.active_plugin_tab ~= tab_id then
      return nil
    end

    if key == "up" then
      selected_idx = math.max(1, selected_idx - 1)
      return { consume = true }
    elseif key == "down" then
      selected_idx = math.min(#plugin_list, selected_idx + 1)
      return { consume = true }
    elseif key == "enter" then
      local plugin = plugin_list[selected_idx]
      install_plugin(plugin)
      return {
        consume = true,
        flash = "Installed " .. plugin.name .. ". Restart app to load it.",
      }
    end
  end,

  on_ui_panels = function(state)
    if state.active_plugin_tab ~= tab_id then
      return nil
    end

    return {
      {
        title = "Plugin List",
        target = "results",
        items = build_results_items(),
      },
      {
        title = "Help",
        target = "queue",
        items = build_queue_items(),
      },
    }
  end,
}
```

Important: this example uses `target = "results"` and `target = "queue"`, so it renders in normal tab panes. It does **not** use `target = "overlay"`.

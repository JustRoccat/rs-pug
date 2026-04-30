# rs-pug Lua Plugin System

The `rs-pug` music player features a powerful and flexible plugin system powered by Lua. This allows users to extend the application's functionality, modify its behavior, and integrate it with external tools without modifying the core Rust codebase.

## Getting Started

To create a plugin, simply write a Lua script and place it in the `~/.config/rs-pug/plugins/` directory. The application automatically loads all `.lua` files found in this directory upon startup.

Each plugin script must return a Lua table containing specific hook functions. These functions are called by `rs-pug` when certain events occur within the application.

```lua
-- Example: ~/.config/rs-pug/plugins/hello_world.lua
return {
    on_key = function(key, state)
        if key == "char:h" then
            return { flash = "Hello from Lua!" }
        end
    end
}
```

## Available Hooks

The plugin system provides several hooks that you can implement in your returned table. You only need to define the hooks you actually intend to use.

### `on_key(key, state)`

This hook is triggered whenever the user presses a key. It is the primary way to add custom keybindings or override existing ones.

*   **`key` (string):** The string representation of the pressed key. Examples include `"char:a"`, `"ctrl:c"`, `"enter"`, `"tab"`, `"space"`, `"up"`, `"down"`, etc.
*   **`state` (table):** A table containing the current state of the user interface. See the [PluginUiState](#pluginuistate) section for details.
*   **Returns:** A `PluginDispatch` table (optional). If you return a table, it dictates what actions the application should take. See the [PluginDispatch](#plugindispatch) section.

### `on_event(event, state)`

This hook is triggered by various application events, such as when a song starts playing or when the player state changes.

*   **`event` (table):** A table describing the event. It typically contains a `kind` field (string) and optional `message` (string) or `value` (number) fields.
*   **`state` (table):** The current UI state.
*   **Returns:** A `PluginDispatch` table (optional).

### `on_search_query(query)`

This hook allows you to intercept and modify the search query before it is sent to YouTube.

*   **`query` (string):** The original search query entered by the user.
*   **Returns:** A string representing the modified search query.

### `on_search_results(songs)`

This hook allows you to filter, sort, or modify the list of songs returned from a search.

*   **`songs` (table):** An array-like table of song objects.
*   **Returns:** A modified array-like table of song objects.

### `on_song_start(song)`

This hook is called right before a song begins playing. You can use it to modify the song's metadata or URL.

*   **`song` (table):** The song object about to be played.
*   **Returns:** The modified song object.

## Data Structures

When interacting with the `rs-pug` API, you will frequently use the following data structures.

### PluginUiState

The `state` parameter passed to `on_key` and `on_event` provides a snapshot of the application's current status. It contains the following fields:

*   `active_tab` (string): The currently active tab (e.g., `"discover"`, `"albums"`, `"library"`, `"options"`).
*   `player_state` (string): The current state of the media player (e.g., `"playing"`, `"paused"`, `"stopped"`).
*   `volume` (number): The current volume level, from 0 to 100.
*   `muted` (boolean): Whether the player is currently muted.
*   `repeat_mode` (string): The current repeat mode (`"none"`, `"all"`, `"one"`).
*   `search_query` (string): The text currently in the search input field.
*   `album_search_query` (string): The text currently in the album search input field.
*   `queue_len` (number): The number of items currently in the playback queue.

### PluginDispatch

The `PluginDispatch` table is what your `on_key` and `on_event` hooks return to instruct `rs-pug` to perform actions. All fields are optional.

*   `consume` (boolean): If set to `true`, `rs-pug` will stop processing this event. This prevents default keybindings from triggering if your plugin handles the key.
*   `flash` (string): A message to display briefly on the screen.
*   `flash_seconds` (number): The duration (in seconds) to display the flash message.
*   `core_actions` (table): An array of action tables to execute. See [Core Actions](#core-actions).
*   `ui` (table): A table of UI state changes to apply. See [UI Patches](#ui-patches).

#### Core Actions

You can trigger core application functions by adding action tables to the `core_actions` array in your `PluginDispatch` return value. Each action table must have a `type` field.

*   `{ type = "search", query = "..." }`: Initiates a search.
*   `{ type = "search_albums", query = "..." }`: Initiates an album search.
*   `{ type = "seek", seconds = 120 }`: Seeks to a specific position in the current song.
*   `{ type = "toggle_pause" }`: Toggles playback pause state.
*   `{ type = "toggle_mute" }`: Toggles audio mute state.
*   `{ type = "volume_up" }`: Increases the volume.
*   `{ type = "volume_down" }`: Decreases the volume.
*   `{ type = "next" }`: Skips to the next song in the queue.
*   `{ type = "prev" }`: Returns to the previous song.
*   `{ type = "set_volume", value = 50 }`: Sets the volume to a specific level (0-100).
*   `{ type = "play_url", url = "...", title = "..." }`: Plays a specific URL directly.
*   `{ type = "raw_mpv", command = { ... } }`: Sends a raw JSON IPC command directly to the underlying `mpv` instance.

#### UI Patches

You can modify the user interface state by providing a `ui` table in your `PluginDispatch` return value.

*   `set_tab` (string): Switches to the specified tab (`"discover"`, `"albums"`, `"library"`, `"options"`).
*   `set_search_query` (string): Updates the text in the search input.
*   `set_album_search_query` (string): Updates the text in the album search input.
*   `set_focus` (string): Changes the currently focused UI element.
*   `set_search_mode` (boolean): Toggles search input mode.
*   `set_selected_result` (number): Changes the selected index in the search results list.
*   `set_selected_album_result` (number): Changes the selected index in the album search results list.
*   `set_selected_queue` (number): Changes the selected index in the playback queue.

## Example Plugins

Here are a few practical examples of what you can build with the `rs-pug` Lua API.

### 1. Custom Keybind: Quick Volume Set

This plugin adds a custom keybinding (`v`) that instantly sets the volume to 50% and flashes a confirmation message.

```lua
-- ~/.config/rs-pug/plugins/quick_volume.lua
return {
    on_key = function(key, state)
        if key == "char:v" then
            return {
                consume = true,
                flash = "Volume set to 50%",
                core_actions = {
                    { type = "set_volume", value = 50 }
                }
            }
        end
    end
}
```

### 2. System Notifications on Song Start

This plugin uses the `os.execute` function (available in standard Lua) to trigger a system notification whenever a new song starts playing.

```lua
-- ~/.config/rs-pug/plugins/notify.lua
return {
    on_event = function(event, state)
        if event.kind == "song_start" and event.message then
            -- Escape single quotes to prevent shell injection
            local safe_title = event.message:gsub("'", "'\\''")
            os.execute("notify-send 'rs-pug' 'Now playing: " .. safe_title .. "'")
        end
    end
}
```

### 3. Auto-Append "Live" to Searches

This plugin intercepts all search queries and automatically appends the word "live" to them, ensuring you always get live performance results.

```lua
-- ~/.config/rs-pug/plugins/always_live.lua
return {
    on_search_query = function(query)
        -- Only append if it's not already there
        if not query:lower():match("live") then
            return query .. " live"
        end
        return query
    end
}
```

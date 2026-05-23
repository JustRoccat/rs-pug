use std::{fs, path::Path};

use mlua::{Function, Lua, LuaSerdeExt, Table, Value};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::model::{RepeatMode, Song, Tab};

struct LuaPlugin {
    name: String,
    lua: Lua,
}

pub struct PluginManager {
    plugins: Vec<LuaPlugin>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum PluginCoreAction {
    Search { query: String },
    SearchAlbums { query: String },
    Seek { seconds: i32 },
    TogglePause,
    ToggleMute,
    VolumeUp,
    VolumeDown,
    Next,
    Prev,
    SetVolume { value: u8 },
    PlayUrl { url: String, title: Option<String> },
    RawMpv { command: JsonValue },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginUiPatch {
    #[serde(default)]
    pub set_tab: Option<String>,
    #[serde(default)]
    pub set_search_query: Option<String>,
    #[serde(default)]
    pub set_album_search_query: Option<String>,
    #[serde(default)]
    pub set_focus: Option<String>,
    #[serde(default)]
    pub set_search_mode: Option<bool>,
    #[serde(default)]
    pub set_selected_result: Option<usize>,
    #[serde(default)]
    pub set_selected_album_result: Option<usize>,
    #[serde(default)]
    pub set_selected_queue: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginDispatch {
    #[serde(default)]
    pub consume: bool,
    #[serde(default)]
    pub flash: Option<String>,
    #[serde(default)]
    pub flash_seconds: Option<u64>,
    #[serde(default)]
    pub core_actions: Vec<PluginCoreAction>,
    #[serde(default)]
    pub ui: PluginUiPatch,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginEvent {
    pub kind: String,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub value: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum PluginPanelItem {
    Text { text: String },
    Info { text: String },
    Option { key: String, value: String },
    Stat { label: String, value: String },
    Separator,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginPanel {
    pub title: String,
    #[serde(default)]
    pub lines: Vec<String>,
    #[serde(default)]
    pub items: Vec<PluginPanelItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginTab {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginUiState {
    pub active_tab: String,
    pub player_state: String,
    pub volume: u8,
    pub muted: bool,
    pub repeat_mode: String,
    pub search_query: String,
    pub album_search_query: String,
    pub queue_len: usize,
}

impl PluginUiState {
    pub fn from_runtime(
        tab: Tab,
        player_state: &str,
        volume: u8,
        muted: bool,
        repeat_mode: RepeatMode,
        search_query: String,
        album_search_query: String,
        queue_len: usize,
    ) -> Self {
        Self {
            active_tab: match tab {
                Tab::Discover => "discover",
                Tab::Albums => "albums",
                Tab::Library => "library",
                Tab::Local => "local",
                Tab::Options => "options",
            }
            .to_owned(),
            player_state: player_state.to_owned(),
            volume,
            muted,
            repeat_mode: repeat_mode.label().to_lowercase(),
            search_query,
            album_search_query,
            queue_len,
        }
    }
}

impl PluginManager {
    pub fn load(enabled: bool, configured_dir: &str) -> Self {
        if !enabled {
            return Self {
                plugins: Vec::new(),
            };
        }

        let mut plugins = Vec::new();
        let path = Path::new(configured_dir);
        let Ok(entries) = fs::read_dir(path) else {
            return Self { plugins };
        };

        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().and_then(|e| e.to_str()) != Some("lua") {
                continue;
            }
            let Ok(src) = fs::read_to_string(&p) else {
                continue;
            };

            let lua = Lua::new();
            let plugin_name = p.to_string_lossy().into_owned();
            let chunk = lua.load(&src).set_name(plugin_name.as_str());
            let Ok(value) = chunk.eval::<Value>() else {
                continue;
            };

            let plugin_table = match value {
                Value::Table(table) => table,
                _ => match lua.globals().get::<Table>("plugin") {
                    Ok(table) => table,
                    Err(_) => continue,
                },
            };
            let _ = lua.globals().set("plugin", plugin_table);

            plugins.push(LuaPlugin {
                name: p
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("plugin")
                    .to_owned(),
                lua,
            });
        }

        Self { plugins }
    }

    #[allow(dead_code)]
    pub fn plugin_count(&self) -> usize {
        self.plugins.len()
    }

    pub fn transform_search_query(&self, query: String) -> String {
        self.run_fold(query, "on_search_query")
    }

    pub fn transform_search_results(&self, songs: Vec<Song>) -> Vec<Song> {
        self.run_fold(songs, "on_search_results")
    }

    pub fn transform_song_start(&self, song: Song) -> Song {
        self.run_fold(song, "on_song_start")
    }

    pub fn dispatch_key(&self, key: &str, state: &PluginUiState) -> PluginDispatch {
        let mut merged = PluginDispatch::default();
        for plugin in &self.plugins {
            let Some(func) = self.read_hook(plugin, "on_key") else {
                continue;
            };

            let Ok(key_lua) = plugin.lua.to_value(key) else {
                continue;
            };
            let Ok(state_lua) = plugin.lua.to_value(state) else {
                continue;
            };
            let Ok(output) = func.call::<Value>((key_lua, state_lua)) else {
                continue;
            };
            self.merge_dispatch(&mut merged, plugin, output);
        }
        merged
    }

    pub fn dispatch_event(&self, event: &PluginEvent, state: &PluginUiState) -> PluginDispatch {
        let mut merged = PluginDispatch::default();
        for plugin in &self.plugins {
            let Some(func) = self.read_hook(plugin, "on_event") else {
                continue;
            };
            let Ok(event_lua) = plugin.lua.to_value(event) else {
                continue;
            };
            let Ok(state_lua) = plugin.lua.to_value(state) else {
                continue;
            };
            let Ok(output) = func.call::<Value>((event_lua, state_lua)) else {
                continue;
            };
            self.merge_dispatch(&mut merged, plugin, output);
        }
        merged
    }

    pub fn collect_tabs(&self, state: &PluginUiState) -> Vec<PluginTab> {
        let mut tabs = Vec::new();
        for plugin in &self.plugins {
            let Some(func) = self.read_hook(plugin, "on_tabs") else {
                continue;
            };
            let Ok(state_lua) = plugin.lua.to_value(state) else {
                continue;
            };
            let Ok(output) = func.call::<Value>(state_lua) else {
                continue;
            };
            if output.is_nil() {
                continue;
            }
            if let Ok(mut plugin_tabs) = plugin.lua.from_value::<Vec<PluginTab>>(output) {
                tabs.append(&mut plugin_tabs);
            }
        }
        tabs
    }

    pub fn collect_ui_panels(&self, state: &PluginUiState) -> Vec<PluginPanel> {
        let mut panels = Vec::new();
        for plugin in &self.plugins {
            let Some(func) = self.read_hook(plugin, "on_ui_panels") else {
                continue;
            };
            let Ok(state_lua) = plugin.lua.to_value(state) else {
                continue;
            };
            let Ok(output) = func.call::<Value>(state_lua) else {
                continue;
            };
            if output.is_nil() {
                continue;
            }
            if let Ok(mut plugin_panels) = plugin.lua.from_value::<Vec<PluginPanel>>(output) {
                for panel in &mut plugin_panels {
                    if panel.items.is_empty() && !panel.lines.is_empty() {
                        panel.items = panel
                            .lines
                            .iter()
                            .cloned()
                            .map(|text| PluginPanelItem::Text { text })
                            .collect();
                    }
                }
                panels.append(&mut plugin_panels);
            }
        }
        panels
    }
    fn merge_dispatch(&self, merged: &mut PluginDispatch, plugin: &LuaPlugin, output: Value) {
        if output.is_nil() {
            return;
        }
        let Ok(dispatch) = plugin.lua.from_value::<PluginDispatch>(output) else {
            return;
        };

        merged.consume |= dispatch.consume;
        if merged.flash.is_none() {
            merged.flash = dispatch.flash;
        }
        if merged.flash_seconds.is_none() {
            merged.flash_seconds = dispatch.flash_seconds;
        }
        merged.core_actions.extend(dispatch.core_actions);
        if merged.ui.set_tab.is_none() {
            merged.ui.set_tab = dispatch.ui.set_tab;
        }
        if merged.ui.set_search_query.is_none() {
            merged.ui.set_search_query = dispatch.ui.set_search_query;
        }
        if merged.ui.set_album_search_query.is_none() {
            merged.ui.set_album_search_query = dispatch.ui.set_album_search_query;
        }
        if merged.ui.set_focus.is_none() {
            merged.ui.set_focus = dispatch.ui.set_focus;
        }
        if merged.ui.set_search_mode.is_none() {
            merged.ui.set_search_mode = dispatch.ui.set_search_mode;
        }
        if merged.ui.set_selected_result.is_none() {
            merged.ui.set_selected_result = dispatch.ui.set_selected_result;
        }
        if merged.ui.set_selected_album_result.is_none() {
            merged.ui.set_selected_album_result = dispatch.ui.set_selected_album_result;
        }
        if merged.ui.set_selected_queue.is_none() {
            merged.ui.set_selected_queue = dispatch.ui.set_selected_queue;
        }
    }

    fn read_hook(&self, plugin: &LuaPlugin, hook: &str) -> Option<Function> {
        let Ok(root) = plugin.lua.globals().get::<Table>("plugin") else {
            return None;
        };
        root.get::<Function>(hook).ok()
    }

    fn run_fold<T>(&self, mut value: T, hook: &str) -> T
    where
        T: Clone + serde::Serialize + serde::de::DeserializeOwned,
    {
        for plugin in &self.plugins {
            let Some(func) = self.read_hook(plugin, hook) else {
                continue;
            };
            let Ok(input) = plugin.lua.to_value(&value) else {
                continue;
            };
            let Ok(output) = func.call::<Value>(input) else {
                continue;
            };
            if output.is_nil() {
                continue;
            }

            if let Ok(next) = plugin.lua.from_value::<T>(output) {
                value = next;
            } else {
                let _ = &plugin.name;
            }
        }
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn collects_ui_panels_from_lua_hook() {
        let dir = tempfile::tempdir().expect("tempdir");
        let plugin_path = dir.path().join("panel.lua");
        fs::write(
            &plugin_path,
            r#"
plugin = {}
function plugin.on_ui_panels(state)
  return {
    {
      title = "Stats",
      items = {
        { type = "text", text = "tab:" .. state.active_tab },
        { type = "option", key = "source", value = "youtube" },
        { type = "stat", label = "vol", value = tostring(state.volume) }
      }
    }
  }
end
return plugin
"#,
        )
        .expect("write plugin");

        let manager = PluginManager::load(true, dir.path().to_str().expect("utf8"));
        let state = PluginUiState::from_runtime(
            Tab::Discover,
            "playing",
            70,
            false,
            RepeatMode::Off,
            String::new(),
            String::new(),
            1,
        );

        let panels = manager.collect_ui_panels(&state);
        assert_eq!(panels.len(), 1);
        assert_eq!(panels[0].title, "Stats");
        assert!(matches!(panels[0].items[0], PluginPanelItem::Text { .. }));
        assert!(matches!(panels[0].items[1], PluginPanelItem::Option { .. }));
        assert!(matches!(panels[0].items[2], PluginPanelItem::Stat { .. }));
    }

    #[test]
    fn collects_legacy_lines_as_text_items() {
        let dir = tempfile::tempdir().expect("tempdir");
        let plugin_path = dir.path().join("panel.lua");
        fs::write(
            &plugin_path,
            r#"
plugin = {}
function plugin.on_ui_panels(state)
  return {
    { title = "Legacy", lines = {"a", "b"} }
  }
end
return plugin
"#,
        )
        .expect("write plugin");
        let manager = PluginManager::load(true, dir.path().to_str().expect("utf8"));
        let state = PluginUiState::from_runtime(
            Tab::Discover,
            "idle",
            10,
            false,
            RepeatMode::Off,
            String::new(),
            String::new(),
            0,
        );
        let panels = manager.collect_ui_panels(&state);
        assert_eq!(panels[0].items.len(), 2);
        assert!(matches!(panels[0].items[0], PluginPanelItem::Text { .. }));
    }
}

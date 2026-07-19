use std::{collections::VecDeque, fs, path::Path, sync::Mutex, time::{Duration, Instant}};
use mlua::{Function, HookTriggers, Lua, LuaSerdeExt, Table, Value, VmState};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
const PLUGIN_EXEC_TIMEOUT: Duration = Duration::from_millis(250);
const PLUGIN_HOOK_INSTRUCTION_INTERVAL: u32 = 10_000;
fn with_exec_timeout<T>(
    lua: &Lua,
    f: impl FnOnce() -> mlua::Result<T>,
) -> mlua::Result<T> {
    let start = Instant::now();
    lua.set_hook(
        HookTriggers::new().every_nth_instruction(PLUGIN_HOOK_INSTRUCTION_INTERVAL),
        move |_lua, _debug| {
            if start.elapsed() > PLUGIN_EXEC_TIMEOUT {
                Err(
                    mlua::Error::RuntimeError(
                        "plugin exceeded execution time limit".to_string(),
                    ),
                )
            } else {
                Ok(VmState::Continue)
            }
        },
    );
    let result = f();
    lua.remove_hook();
    result
}
#[cfg(test)]
use crate::model::RepeatMode;
use crate::model::{App, MainTabKind, Song, Tab};
struct LuaPlugin {
    name: String,
    lua: Lua,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginWarningLevel {
    Warning,
    Error,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginWarning {
    pub level: PluginWarningLevel,
    pub plugin: Option<String>,
    pub hook: Option<String>,
    pub message: String,
}
impl PluginWarning {
    pub fn label(&self) -> String {
        let level = match self.level {
            PluginWarningLevel::Warning => "WARN",
            PluginWarningLevel::Error => "ERROR",
        };
        match (&self.plugin, &self.hook) {
            (Some(plugin), Some(hook)) => {
                format!("Lua {level} [{plugin}.{hook}]: {}", self.message)
            }
            (Some(plugin), None) => format!("Lua {level} [{plugin}]: {}", self.message),
            (None, Some(hook)) => format!("Lua {level} [{hook}]: {}", self.message),
            (None, None) => format!("Lua {level}: {}", self.message),
        }
    }
}
pub struct PluginManager {
    plugins: Vec<LuaPlugin>,
    enabled: bool,
    configured_dir: String,
    allow_lua_ui_changes: bool,
    warnings: Mutex<VecDeque<PluginWarning>>,
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
pub struct PluginUiLayoutPatch {
    #[serde(default)]
    pub queue_width_percent: Option<u16>,
    #[serde(default)]
    pub visualizer_height: Option<u16>,
    #[serde(default)]
    pub tab_bar_position: Option<String>,
    #[serde(default)]
    pub tabs_width: Option<u16>,
    #[serde(default)]
    pub queue_position: Option<String>,
    #[serde(default)]
    pub hide_sections: Vec<String>,
    #[serde(default)]
    pub show_sections: Vec<String>,
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
    #[serde(default)]
    pub layout: PluginUiLayoutPatch,
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
    Header { text: String },
    Keybind { key: String, action: String },
    Progress { label: Option<String>, percent: f64 },
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PluginPanelTarget {
    Main,
    Results,
    Queue,
    Overlay,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginPanel {
    pub title: String,
    #[serde(default)]
    pub target: Option<PluginPanelTarget>,
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PluginUiLayoutState {
    pub queue_width_percent: u16,
    pub visualizer_height: u16,
    pub tab_bar_position: String,
    pub tabs_width: u16,
    pub queue_position: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginUiConfig {
    #[serde(default)]
    pub tabs: PluginTabsConfig,
    #[serde(default)]
    pub layout: PluginLayoutConfig,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginTabsConfig {
    #[serde(default)]
    pub remove: Vec<String>,
    #[serde(default)]
    pub order: Vec<String>,
    #[serde(default)]
    pub rename: std::collections::HashMap<String, PluginTabRename>,
    #[serde(default)]
    pub custom: Vec<PluginCustomTab>,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginTabRename {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCustomTab {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub position: Option<usize>,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginLayoutConfig {
    #[serde(default)]
    pub queue_width_percent: Option<u16>,
    #[serde(default)]
    pub visualizer_height: Option<u16>,
    #[serde(default)]
    pub show_progress_bar: Option<bool>,
    #[serde(default)]
    pub show_volume_bar: Option<bool>,
    #[serde(default)]
    pub show_statusbar: Option<bool>,
    #[serde(default)]
    pub show_keybind_hints: Option<bool>,
    #[serde(default)]
    pub tab_bar_position: Option<String>,
    #[serde(default)]
    pub tabs_width: Option<u16>,
    #[serde(default)]
    pub queue_position: Option<String>,
    #[serde(default)]
    pub hide: Vec<String>,
    #[serde(default)]
    pub custom_sections: Vec<PluginCustomSection>,
    #[serde(default)]
    pub hide_sections: Vec<String>,
    #[serde(default)]
    pub show_sections: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCustomSection {
    pub id: String,
    #[serde(default = "default_section_position")]
    pub position: String,
    #[serde(default)]
    pub width: Option<u16>,
    #[serde(default)]
    pub height: Option<u16>,
    #[serde(default)]
    pub content: Option<String>,
}
fn default_section_position() -> String {
    "below_player".to_owned()
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginUiInject {
    #[serde(default)]
    pub results_top: Vec<PluginPanelItem>,
    #[serde(default)]
    pub results_bottom: Vec<PluginPanelItem>,
    #[serde(default)]
    pub queue_top: Vec<PluginPanelItem>,
    #[serde(default)]
    pub queue_bottom: Vec<PluginPanelItem>,
    #[serde(default)]
    pub statusbar_extra: Vec<PluginPanelItem>,
}
pub type PluginUiSections = std::collections::HashMap<String, Vec<PluginPanelItem>>;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginUiState {
    pub active_tab: String,
    pub active_plugin_tab: Option<String>,
    pub active_custom_tab: Option<String>,
    pub active_tab_index: usize,
    pub current_layout: PluginUiLayoutState,
    pub visible_sections: Vec<String>,
    pub player_state: String,
    pub volume: u8,
    pub muted: bool,
    pub repeat_mode: String,
    pub search_query: String,
    pub album_search_query: String,
    pub queue_len: usize,
}
impl PluginUiState {
    fn tab_id(tab: Tab) -> &'static str {
        match tab {
            Tab::Discover => "discover",
            Tab::Albums => "albums",
            Tab::Library => "library",
            Tab::Local => "local",
            Tab::Options => "options",
        }
    }
    #[cfg(test)]
    pub fn from_runtime(
        tab: Tab,
        active_plugin_tab: Option<String>,
        player_state: &str,
        volume: u8,
        muted: bool,
        repeat_mode: RepeatMode,
        search_query: String,
        album_search_query: String,
        queue_len: usize,
    ) -> Self {
        Self {
            active_tab: Self::tab_id(tab).to_owned(),
            active_plugin_tab,
            active_custom_tab: None,
            active_tab_index: match tab {
                Tab::Discover => 1,
                Tab::Albums => 2,
                Tab::Library => 3,
                Tab::Local => 4,
                Tab::Options => 5,
            },
            current_layout: PluginUiLayoutState {
                queue_width_percent: 40,
                visualizer_height: 5,
                tab_bar_position: "top".to_owned(),
                tabs_width: 22,
                queue_position: "right".to_owned(),
            },
            visible_sections: Vec::new(),
            player_state: player_state.to_owned(),
            volume,
            muted,
            repeat_mode: repeat_mode.label().to_lowercase(),
            search_query,
            album_search_query,
            queue_len,
        }
    }
    pub fn from_app(app: &App) -> Self {
        let active_tab_index = app.active_tab_index();
        let active_tab = if let Some(custom) = &app.plugin_ui.active_custom_tab {
            custom.clone()
        } else if let Some(tab) = app.main_tabs.get(active_tab_index.saturating_sub(1)) {
            match &tab.kind {
                MainTabKind::Stock(_) => tab.id.clone(),
                MainTabKind::Custom(id) => id.clone(),
            }
        } else {
            Self::tab_id(app.active_tab).to_owned()
        };
        Self {
            active_tab,
            active_plugin_tab: app.plugin_ui.active_tab.clone(),
            active_custom_tab: app.plugin_ui.active_custom_tab.clone(),
            active_tab_index,
            current_layout: app.current_layout_state(),
            visible_sections: app.visible_section_ids(),
            player_state: crate::ui_helpers::player_state_label(app.player_state)
                .to_owned(),
            volume: app.volume,
            muted: app.muted,
            repeat_mode: app.repeat_mode.label().to_lowercase(),
            search_query: app.search.query.clone(),
            album_search_query: app.albums.search_query.clone(),
            queue_len: app.queue.len(),
        }
    }
}
impl PluginManager {
    pub fn load(
        enabled: bool,
        configured_dir: &str,
        allow_lua_ui_changes: bool,
    ) -> Self {
        if !enabled {
            return Self {
                plugins: Vec::new(),
                enabled,
                configured_dir: configured_dir.to_owned(),
                allow_lua_ui_changes,
                warnings: Mutex::new(VecDeque::new()),
            };
        }
        let mut plugins = Vec::new();
        let mut warnings = VecDeque::new();
        let path = Path::new(configured_dir);
        let Ok(entries) = fs::read_dir(path) else {
            warnings
                .push_back(PluginWarning {
                    level: PluginWarningLevel::Warning,
                    plugin: None,
                    hook: None,
                    message: format!("cannot read plugin directory {}", path.display()),
                });
            return Self {
                plugins,
                enabled,
                configured_dir: configured_dir.to_owned(),
                allow_lua_ui_changes,
                warnings: Mutex::new(warnings),
            };
        };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().and_then(|e| e.to_str()) != Some("lua") {
                continue;
            }
            let plugin_file = p.to_string_lossy().into_owned();
            let Ok(src) = fs::read_to_string(&p) else {
                warnings
                    .push_back(PluginWarning {
                        level: PluginWarningLevel::Error,
                        plugin: Some(plugin_file),
                        hook: None,
                        message: "cannot read plugin file".to_owned(),
                    });
                continue;
            };
            let lua = Lua::new();
            let plugin_name = p.to_string_lossy().into_owned();
            let chunk = lua.load(&src).set_name(plugin_name.as_str());
            let value = match with_exec_timeout(&lua, || chunk.eval::<Value>()) {
                Ok(value) => value,
                Err(err) => {
                    warnings
                        .push_back(PluginWarning {
                            level: PluginWarningLevel::Error,
                            plugin: Some(plugin_name.clone()),
                            hook: None,
                            message: format!("load failed: {err}"),
                        });
                    continue;
                }
            };
            let plugin_table = match value {
                Value::Table(table) => table,
                _ => {
                    match lua.globals().get::<Table>("plugin") {
                        Ok(table) => table,
                        Err(err) => {
                            warnings
                                .push_back(PluginWarning {
                                    level: PluginWarningLevel::Error,
                                    plugin: Some(plugin_name.clone()),
                                    hook: None,
                                    message: format!("missing plugin table: {err}"),
                                });
                            continue;
                        }
                    }
                }
            };
            if let Err(err) = lua
                .globals()
                .set("ALLOW_LUA_UI_CHANGES", allow_lua_ui_changes)
            {
                warnings
                    .push_back(PluginWarning {
                        level: PluginWarningLevel::Warning,
                        plugin: Some(plugin_name.clone()),
                        hook: None,
                        message: format!("cannot expose ALLOW_LUA_UI_CHANGES: {err}"),
                    });
            }
            if let Err(err) = lua.globals().set("plugin", plugin_table) {
                warnings
                    .push_back(PluginWarning {
                        level: PluginWarningLevel::Error,
                        plugin: Some(plugin_name.clone()),
                        hook: None,
                        message: format!("cannot expose plugin table: {err}"),
                    });
                continue;
            }
            plugins
                .push(LuaPlugin {
                    name: p
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("plugin")
                        .to_owned(),
                    lua,
                });
        }
        Self {
            plugins,
            enabled,
            configured_dir: configured_dir.to_owned(),
            allow_lua_ui_changes,
            warnings: Mutex::new(warnings),
        }
    }
    pub fn reload(
        &mut self,
        enabled: bool,
        configured_dir: &str,
        allow_lua_ui_changes: bool,
    ) {
        let next = Self::load(enabled, configured_dir, allow_lua_ui_changes);
        if let Ok(mut warnings) = self.warnings.lock() {
            warnings
                .push_back(PluginWarning {
                    level: PluginWarningLevel::Warning,
                    plugin: None,
                    hook: None,
                    message: "plugins hot-reloaded".to_owned(),
                });
            warnings.extend(next.drain_warnings());
        }
        self.plugins = next.plugins;
        self.enabled = next.enabled;
        self.configured_dir = next.configured_dir;
        self.allow_lua_ui_changes = next.allow_lua_ui_changes;
    }
    #[allow(dead_code)]
    pub fn plugin_count(&self) -> usize {
        self.plugins.len()
    }
    pub fn drain_warnings(&self) -> Vec<PluginWarning> {
        let Ok(mut warnings) = self.warnings.lock() else {
            return Vec::new();
        };
        warnings.drain(..).collect()
    }
    fn warn(
        &self,
        level: PluginWarningLevel,
        plugin: Option<&LuaPlugin>,
        hook: Option<&str>,
        message: impl Into<String>,
    ) {
        if let Ok(mut warnings) = self.warnings.lock() {
            if warnings.len() >= 100 {
                warnings.pop_front();
            }
            warnings
                .push_back(PluginWarning {
                    level,
                    plugin: plugin.map(|plugin| plugin.name.clone()),
                    hook: hook.map(str::to_owned),
                    message: message.into(),
                });
        }
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
            let Ok(output) = with_exec_timeout(
                &plugin.lua,
                || func.call::<Value>((key_lua, state_lua)),
            ) else {
                continue;
            };
            self.merge_dispatch(&mut merged, plugin, output);
        }
        merged
    }
    pub fn dispatch_event(
        &self,
        event: &PluginEvent,
        state: &PluginUiState,
    ) -> PluginDispatch {
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
            let Ok(output) = with_exec_timeout(
                &plugin.lua,
                || func.call::<Value>((event_lua, state_lua)),
            ) else {
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
            let Ok(output) = with_exec_timeout(
                &plugin.lua,
                || func.call::<Value>(state_lua),
            ) else {
                continue;
            };
            if output.is_nil() {
                continue;
            }
            if let Ok(mut plugin_tabs) = plugin.lua.from_value::<Vec<PluginTab>>(output)
            {
                tabs.append(&mut plugin_tabs);
            }
        }
        tabs
    }
    pub fn collect_ui_config(&self, state: &PluginUiState) -> PluginUiConfig {
        if !self.allow_lua_ui_changes {
            return PluginUiConfig::default();
        }
        let mut config = PluginUiConfig::default();
        for plugin in &self.plugins {
            let hook = "on_ui_config";
            let Some(func) = self.read_hook(plugin, hook) else {
                continue;
            };
            let state_lua = match plugin.lua.to_value(state) {
                Ok(value) => value,
                Err(err) => {
                    self.warn(
                        PluginWarningLevel::Error,
                        Some(plugin),
                        Some(hook),
                        format!("state serialization failed: {err}"),
                    );
                    continue;
                }
            };
            let output = match with_exec_timeout(
                &plugin.lua,
                || func.call::<Value>(state_lua),
            ) {
                Ok(value) => value,
                Err(err) => {
                    self.warn(
                        PluginWarningLevel::Error,
                        Some(plugin),
                        Some(hook),
                        format!("hook call failed: {err}"),
                    );
                    continue;
                }
            };
            if output.is_nil() {
                continue;
            }
            match plugin.lua.from_value::<PluginUiConfig>(output) {
                Ok(next) => merge_ui_config(&mut config, next),
                Err(err) => {
                    self.warn(
                        PluginWarningLevel::Error,
                        Some(plugin),
                        Some(hook),
                        format!("invalid return shape: {err}"),
                    )
                }
            }
        }
        config
    }
    pub fn collect_ui_sections(&self, state: &PluginUiState) -> PluginUiSections {
        if !self.allow_lua_ui_changes {
            return PluginUiSections::default();
        }
        let mut sections = PluginUiSections::default();
        for plugin in &self.plugins {
            let hook = "on_ui_sections";
            let Some(func) = self.read_hook(plugin, hook) else {
                continue;
            };
            let state_lua = match plugin.lua.to_value(state) {
                Ok(value) => value,
                Err(err) => {
                    self.warn(
                        PluginWarningLevel::Error,
                        Some(plugin),
                        Some(hook),
                        format!("state serialization failed: {err}"),
                    );
                    continue;
                }
            };
            let output = match with_exec_timeout(
                &plugin.lua,
                || func.call::<Value>(state_lua),
            ) {
                Ok(value) => value,
                Err(err) => {
                    self.warn(
                        PluginWarningLevel::Error,
                        Some(plugin),
                        Some(hook),
                        format!("hook call failed: {err}"),
                    );
                    continue;
                }
            };
            if output.is_nil() {
                continue;
            }
            match plugin.lua.from_value::<PluginUiSections>(output) {
                Ok(plugin_sections) => sections.extend(plugin_sections),
                Err(err) => {
                    self.warn(
                        PluginWarningLevel::Error,
                        Some(plugin),
                        Some(hook),
                        format!("invalid return shape: {err}"),
                    )
                }
            }
        }
        sections
    }
    pub fn collect_ui_inject(&self, state: &PluginUiState) -> PluginUiInject {
        if !self.allow_lua_ui_changes {
            return PluginUiInject::default();
        }
        let mut inject = PluginUiInject::default();
        for plugin in &self.plugins {
            let hook = "on_ui_inject";
            let Some(func) = self.read_hook(plugin, hook) else {
                continue;
            };
            let state_lua = match plugin.lua.to_value(state) {
                Ok(value) => value,
                Err(err) => {
                    self.warn(
                        PluginWarningLevel::Error,
                        Some(plugin),
                        Some(hook),
                        format!("state serialization failed: {err}"),
                    );
                    continue;
                }
            };
            let output = match with_exec_timeout(
                &plugin.lua,
                || func.call::<Value>(state_lua),
            ) {
                Ok(value) => value,
                Err(err) => {
                    self.warn(
                        PluginWarningLevel::Error,
                        Some(plugin),
                        Some(hook),
                        format!("hook call failed: {err}"),
                    );
                    continue;
                }
            };
            if output.is_nil() {
                continue;
            }
            match plugin.lua.from_value::<PluginUiInject>(output) {
                Ok(plugin_inject) => {
                    inject.results_top.extend(plugin_inject.results_top);
                    inject.results_bottom.extend(plugin_inject.results_bottom);
                    inject.queue_top.extend(plugin_inject.queue_top);
                    inject.queue_bottom.extend(plugin_inject.queue_bottom);
                    inject.statusbar_extra.extend(plugin_inject.statusbar_extra);
                }
                Err(err) => {
                    self.warn(
                        PluginWarningLevel::Error,
                        Some(plugin),
                        Some(hook),
                        format!("invalid return shape: {err}"),
                    )
                }
            }
        }
        inject
    }
    pub fn collect_ui_update(&self, state: &PluginUiState) -> PluginLayoutConfig {
        if !self.allow_lua_ui_changes {
            return PluginLayoutConfig::default();
        }
        let mut layout = PluginLayoutConfig::default();
        for plugin in &self.plugins {
            let hook = "on_ui_update";
            let Some(func) = self.read_hook(plugin, hook) else {
                continue;
            };
            let state_lua = match plugin.lua.to_value(state) {
                Ok(value) => value,
                Err(err) => {
                    self.warn(
                        PluginWarningLevel::Error,
                        Some(plugin),
                        Some(hook),
                        format!("state serialization failed: {err}"),
                    );
                    continue;
                }
            };
            let output = match with_exec_timeout(
                &plugin.lua,
                || func.call::<Value>(state_lua),
            ) {
                Ok(value) => value,
                Err(err) => {
                    self.warn(
                        PluginWarningLevel::Error,
                        Some(plugin),
                        Some(hook),
                        format!("hook call failed: {err}"),
                    );
                    continue;
                }
            };
            if output.is_nil() {
                continue;
            }
            match self.layout_config_from_value(plugin, hook, output) {
                Some(config) => merge_layout_config(&mut layout, config),
                None => continue,
            }
        }
        layout
    }
    fn layout_config_from_value(
        &self,
        plugin: &LuaPlugin,
        hook: &str,
        output: Value,
    ) -> Option<PluginLayoutConfig> {
        let prefer_ui_config = match &output {
            Value::Table(table) => {
                lua_table_has_key(table, "layout") || lua_table_has_key(table, "tabs")
            }
            _ => true,
        };
        let ui_result = plugin.lua.from_value::<PluginUiConfig>(output.clone());
        let layout_result = plugin.lua.from_value::<PluginLayoutConfig>(output);
        if prefer_ui_config {
            match ui_result {
                Ok(config) => Some(config.layout),
                Err(ui_err) => {
                    match layout_result {
                        Ok(config) => Some(config),
                        Err(layout_err) => {
                            self.warn_invalid_layout_shape(
                                plugin,
                                hook,
                                ui_err,
                                layout_err,
                            );
                            None
                        }
                    }
                }
            }
        } else {
            match layout_result {
                Ok(config) => Some(config),
                Err(layout_err) => {
                    match ui_result {
                        Ok(config) => Some(config.layout),
                        Err(ui_err) => {
                            self.warn_invalid_layout_shape(
                                plugin,
                                hook,
                                ui_err,
                                layout_err,
                            );
                            None
                        }
                    }
                }
            }
        }
    }
    fn warn_invalid_layout_shape(
        &self,
        plugin: &LuaPlugin,
        hook: &str,
        ui_err: mlua::Error,
        layout_err: mlua::Error,
    ) {
        self.warn(
            PluginWarningLevel::Error,
            Some(plugin),
            Some(hook),
            format!(
                "invalid return shape: expected PluginUiConfig ({ui_err}) or PluginLayoutConfig ({layout_err})"
            ),
        );
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
            let Ok(output) = with_exec_timeout(
                &plugin.lua,
                || func.call::<Value>(state_lua),
            ) else {
                continue;
            };
            if output.is_nil() {
                continue;
            }
            if let Ok(mut plugin_panels) = plugin
                .lua
                .from_value::<Vec<PluginPanel>>(output)
            {
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
    fn merge_dispatch(
        &self,
        merged: &mut PluginDispatch,
        plugin: &LuaPlugin,
        output: Value,
    ) {
        if output.is_nil() {
            return;
        }
        let dispatch = match plugin.lua.from_value::<PluginDispatch>(output) {
            Ok(dispatch) => dispatch,
            Err(err) => {
                self.warn(
                    PluginWarningLevel::Error,
                    Some(plugin),
                    None,
                    format!("invalid dispatch return: {err}"),
                );
                return;
            }
        };
        merged.consume |= dispatch.consume;
        merged.flash = merged.flash.take().or(dispatch.flash);
        merged.flash_seconds = merged.flash_seconds.or(dispatch.flash_seconds);
        merged.core_actions.extend(dispatch.core_actions);
        merged.ui.set_tab = merged.ui.set_tab.take().or(dispatch.ui.set_tab);
        merged.ui.set_search_query = merged
            .ui
            .set_search_query
            .take()
            .or(dispatch.ui.set_search_query);
        merged.ui.set_album_search_query = merged
            .ui
            .set_album_search_query
            .take()
            .or(dispatch.ui.set_album_search_query);
        merged.ui.set_focus = merged.ui.set_focus.take().or(dispatch.ui.set_focus);
        merged.ui.set_search_mode = merged
            .ui
            .set_search_mode
            .or(dispatch.ui.set_search_mode);
        merged.ui.set_selected_result = merged
            .ui
            .set_selected_result
            .or(dispatch.ui.set_selected_result);
        merged.ui.set_selected_album_result = merged
            .ui
            .set_selected_album_result
            .or(dispatch.ui.set_selected_album_result);
        merged.ui.set_selected_queue = merged
            .ui
            .set_selected_queue
            .or(dispatch.ui.set_selected_queue);
        if self.allow_lua_ui_changes {
            merge_ui_layout_patch(&mut merged.ui.layout, dispatch.ui.layout);
        }
    }
    fn read_hook(&self, plugin: &LuaPlugin, hook: &str) -> Option<Function> {
        let root = match plugin.lua.globals().get::<Table>("plugin") {
            Ok(root) => root,
            Err(err) => {
                self.warn(
                    PluginWarningLevel::Error,
                    Some(plugin),
                    Some(hook),
                    format!("plugin table unavailable: {err}"),
                );
                return None;
            }
        };
        match root.get::<Value>(hook) {
            Ok(Value::Function(func)) => Some(func),
            Ok(Value::Nil) => None,
            Ok(_) => {
                self.warn(
                    PluginWarningLevel::Warning,
                    Some(plugin),
                    Some(hook),
                    "hook is not a function",
                );
                None
            }
            Err(err) => {
                self.warn(
                    PluginWarningLevel::Error,
                    Some(plugin),
                    Some(hook),
                    format!("hook lookup failed: {err}"),
                );
                None
            }
        }
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
            let Ok(output) = with_exec_timeout(&plugin.lua, || func.call::<Value>(input))
            else {
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
        let manager = PluginManager::load(
            true,
            dir.path().to_str().expect("utf8"),
            false,
        );
        let state = PluginUiState::from_runtime(
            Tab::Discover,
            None,
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
    fn ignores_new_ui_hooks_when_flag_is_false() {
        let dir = tempfile::tempdir().expect("tempdir");
        let plugin_path = dir.path().join("ui.lua");
        fs::write(
                &plugin_path,
                r#"
plugin = {}
function plugin.on_ui_config(state)
  return { layout = { queue_width_percent = 25 } }
end
function plugin.on_ui_sections(state)
  return { hello = { { type = "header", text = "Hello" } } }
end
return plugin
"#,
            )
            .expect("write plugin");
        let manager = PluginManager::load(
            true,
            dir.path().to_str().expect("utf8"),
            false,
        );
        let state = PluginUiState::from_runtime(
            Tab::Discover,
            None,
            "idle",
            10,
            false,
            RepeatMode::Off,
            String::new(),
            String::new(),
            0,
        );
        assert!(manager.collect_ui_config(& state).layout.queue_width_percent.is_none());
        assert!(manager.collect_ui_sections(& state).is_empty());
    }
    #[test]
    fn key_dispatch_deserializes_ui_layout_patch() {
        let dir = tempfile::tempdir().expect("tempdir");
        fs::write(
                dir.path().join("layout_key.lua"),
                r#"
plugin = {}
function plugin.on_key(key, state)
  if key == "char:Z" then
    return {
      consume = true,
      flash = "layout patch",
      ui = {
        layout = {
          queue_width_percent = 65,
          queue_position = "left",
          visualizer_height = 4,
          tab_bar_position = "top"
        }
      }
    }
  end
end
return plugin
"#,
            )
            .expect("write plugin");
        let manager = PluginManager::load(
            true,
            dir.path().to_str().expect("utf8"),
            true,
        );
        let state = PluginUiState::from_runtime(
            Tab::Discover,
            None,
            "idle",
            10,
            false,
            RepeatMode::Off,
            String::new(),
            String::new(),
            0,
        );
        let dispatch = manager.dispatch_key("char:Z", &state);
        assert!(dispatch.consume);
        assert_eq!(dispatch.flash.as_deref(), Some("layout patch"));
        assert_eq!(dispatch.ui.layout.queue_width_percent, Some(65));
        assert_eq!(dispatch.ui.layout.queue_position.as_deref(), Some("left"));
        assert_eq!(dispatch.ui.layout.visualizer_height, Some(4));
        assert_eq!(dispatch.ui.layout.tab_bar_position.as_deref(), Some("top"));
    }
    #[test]
    fn ui_update_accepts_layout_and_full_config_shapes() {
        let dir = tempfile::tempdir().expect("tempdir");
        fs::write(
                dir.path().join("layout.lua"),
                r#"
plugin = {}
function plugin.on_ui_update(state)
  if state.active_tab == "discover" then
    return { queue_width_percent = 33 }
  end
  return { layout = { visualizer_height = 7 } }
end
return plugin
"#,
            )
            .expect("write plugin");
        let manager = PluginManager::load(
            true,
            dir.path().to_str().expect("utf8"),
            true,
        );
        let mut state = PluginUiState::from_runtime(
            Tab::Discover,
            None,
            "idle",
            10,
            false,
            RepeatMode::Off,
            String::new(),
            String::new(),
            0,
        );
        let layout = manager.collect_ui_update(&state);
        assert_eq!(layout.queue_width_percent, Some(33));
        state.active_tab = "library".to_owned();
        let layout = manager.collect_ui_update(&state);
        assert_eq!(layout.visualizer_height, Some(7));
    }
    #[test]
    fn collects_new_ui_hooks_when_flag_is_true() {
        let dir = tempfile::tempdir().expect("tempdir");
        let plugin_path = dir.path().join("ui.lua");
        fs::write(
                &plugin_path,
                r#"
plugin = {}
function plugin.on_ui_config(state)
  return {
    tabs = { custom = { { id = "dash", title = "Dash", icon = "D", position = 2 } } },
    layout = {
      queue_width_percent = 25,
      tab_bar_position = "right",
      tabs_width = 24,
      queue_position = "left",
      custom_sections = { { id = "hello", position = "below_player", height = 3, content = "lua" } }
    }
  }
end
function plugin.on_ui_sections(state)
  return { hello = { { type = "header", text = "Hello" }, { type = "progress", percent = 50 } } }
end
function plugin.on_ui_inject(state)
  return { statusbar_extra = { { type = "keybind", key = "x", action = "action" } } }
end
return plugin
"#,
            )
            .expect("write plugin");
        let manager = PluginManager::load(
            true,
            dir.path().to_str().expect("utf8"),
            true,
        );
        let state = PluginUiState::from_runtime(
            Tab::Discover,
            None,
            "idle",
            10,
            false,
            RepeatMode::Off,
            String::new(),
            String::new(),
            0,
        );
        let config = manager.collect_ui_config(&state);
        assert_eq!(config.layout.queue_width_percent, Some(25));
        assert_eq!(config.layout.tab_bar_position.as_deref(), Some("right"));
        assert_eq!(config.layout.tabs_width, Some(24));
        assert_eq!(config.layout.queue_position.as_deref(), Some("left"));
        assert_eq!(config.tabs.custom[0].id, "dash");
        let sections = manager.collect_ui_sections(&state);
        assert_eq!(sections["hello"].len(), 2);
        assert_eq!(manager.collect_ui_inject(& state).statusbar_extra.len(), 1);
    }
    #[test]
    fn records_lua_load_and_hook_warnings() {
        let dir = tempfile::tempdir().expect("tempdir");
        fs::write(dir.path().join("broken.lua"), "plugin = { }
function nope")
            .expect("write plugin");
        fs::write(
                dir.path().join("bad_hook.lua"),
                r#"
plugin = {}
function plugin.on_ui_config(state)
  error("bad config")
end
return plugin
"#,
            )
            .expect("write plugin");
        let manager = PluginManager::load(
            true,
            dir.path().to_str().expect("utf8"),
            true,
        );
        let initial = manager.drain_warnings();
        assert!(initial.iter().any(| warning | warning.message.contains("load failed")));
        let state = PluginUiState::from_runtime(
            Tab::Discover,
            None,
            "idle",
            10,
            false,
            RepeatMode::Off,
            String::new(),
            String::new(),
            0,
        );
        let _ = manager.collect_ui_config(&state);
        let hook_warnings = manager.drain_warnings();
        assert!(
            hook_warnings.iter().any(| warning | { warning.hook.as_deref() ==
            Some("on_ui_config") && warning.message.contains("hook call failed") })
        );
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
        let manager = PluginManager::load(
            true,
            dir.path().to_str().expect("utf8"),
            false,
        );
        let state = PluginUiState::from_runtime(
            Tab::Discover,
            None,
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
fn lua_table_has_key(table: &Table, key: &str) -> bool {
    matches!(table.get::< Value > (key), Ok(value) if ! value.is_nil())
}
fn merge_ui_config(target: &mut PluginUiConfig, source: PluginUiConfig) {
    target.tabs.remove.extend(source.tabs.remove);
    if !source.tabs.order.is_empty() {
        target.tabs.order = source.tabs.order;
    }
    target.tabs.rename.extend(source.tabs.rename);
    target.tabs.custom.extend(source.tabs.custom);
    merge_layout_config(&mut target.layout, source.layout);
}
fn merge_layout_config(target: &mut PluginLayoutConfig, source: PluginLayoutConfig) {
    if source.queue_width_percent.is_some() {
        target.queue_width_percent = source.queue_width_percent;
    }
    if source.visualizer_height.is_some() {
        target.visualizer_height = source.visualizer_height;
    }
    if source.show_progress_bar.is_some() {
        target.show_progress_bar = source.show_progress_bar;
    }
    if source.show_volume_bar.is_some() {
        target.show_volume_bar = source.show_volume_bar;
    }
    if source.show_statusbar.is_some() {
        target.show_statusbar = source.show_statusbar;
    }
    if source.show_keybind_hints.is_some() {
        target.show_keybind_hints = source.show_keybind_hints;
    }
    if source.tab_bar_position.is_some() {
        target.tab_bar_position = source.tab_bar_position;
    }
    if source.tabs_width.is_some() {
        target.tabs_width = source.tabs_width;
    }
    if source.queue_position.is_some() {
        target.queue_position = source.queue_position;
    }
    target.hide.extend(source.hide);
    target.custom_sections.extend(source.custom_sections);
    target.hide_sections.extend(source.hide_sections);
    target.show_sections.extend(source.show_sections);
}
fn merge_ui_layout_patch(target: &mut PluginUiLayoutPatch, source: PluginUiLayoutPatch) {
    if source.queue_width_percent.is_some() {
        target.queue_width_percent = source.queue_width_percent;
    }
    if source.visualizer_height.is_some() {
        target.visualizer_height = source.visualizer_height;
    }
    if source.tab_bar_position.is_some() {
        target.tab_bar_position = source.tab_bar_position;
    }
    if source.tabs_width.is_some() {
        target.tabs_width = source.tabs_width;
    }
    if source.queue_position.is_some() {
        target.queue_position = source.queue_position;
    }
    target.hide_sections.extend(source.hide_sections);
    target.show_sections.extend(source.show_sections);
}

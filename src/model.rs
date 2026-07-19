#[derive(Debug, Default)]
pub struct SearchState {
    pub query: String,
    pub results: Vec<Song>,
    pub selected_result: usize,
}
#[derive(Debug, Default)]
pub struct AlbumState {
    pub search_query: String,
    pub results: Vec<Album>,
    pub selected_result: usize,
    pub expanded: Vec<bool>,
}
#[derive(Debug, Default)]
pub struct PlaylistState {
    pub playlists: Vec<Playlist>,
    pub selected_playlist: usize,
    pub context_open: bool,
    pub context_index: usize,
    pub expanded: Vec<bool>,
    pub selected_song: usize,
    pub confirm_delete: bool,
    pub delete_name: String,
    pub adding_song: bool,
}
#[derive(Debug, Default)]
pub struct EqState {
    pub enabled: bool,
    pub bands: [f32; 10],
    pub focus_band: usize,
    pub preset_index: usize,
    pub custom_presets: Vec<crate::config::EqPreset>,
}
#[derive(Debug, Default)]
pub struct PluginUiState {
    pub panels: Vec<PluginPanel>,
    pub tabs: Vec<PluginTab>,
    pub active_tab: Option<String>,
    pub active_custom_tab: Option<String>,
    pub warnings: std::collections::VecDeque<String>,
    pub allow_lua_ui_changes: bool,
    pub custom_sections: Vec<PluginCustomSection>,
    pub hidden_sections: Vec<String>,
    pub section_items: std::collections::HashMap<String, Vec<PluginPanelItem>>,
    pub inject: PluginUiInject,
}
#[derive(Debug)]
pub struct LocalLibraryState {
    pub scanning: bool,
    pub window: Vec<LocalSong>,
    pub offset: usize,
    pub total: usize,
    pub view_mode: LocalViewMode,
    pub sort_mode: LocalSortMode,
    pub filter_genre: Option<String>,
    pub filter_artist: Option<String>,
    pub filter_album: Option<String>,
    pub tag_editor_open: bool,
    pub tag_editor_field: LocalTagField,
    pub tag_editor_song: Option<LocalSong>,
    pub tag_edit_buffer: String,
    pub selected_song: usize,
    pub nav_level: LocalNavLevel,
    pub nav_artist: Option<String>,
    pub nav_album: Option<String>,
    pub selected_nav_idx: usize,
}
impl Default for LocalLibraryState {
    fn default() -> Self {
        Self {
            scanning: false,
            window: Vec::new(),
            offset: 0,
            total: 0,
            view_mode: LocalViewMode::Flat,
            sort_mode: LocalSortMode::Title,
            filter_genre: None,
            filter_artist: None,
            filter_album: None,
            tag_editor_open: false,
            tag_editor_field: LocalTagField::Title,
            tag_editor_song: None,
            tag_edit_buffer: String::new(),
            selected_song: 0,
            nav_level: LocalNavLevel::Artists,
            nav_artist: None,
            nav_album: None,
            selected_nav_idx: 0,
        }
    }
}
use std::time::{Duration, Instant};
use crate::config::{
    Config, GeneralConfig, KeybindsConfig, LuaConfig, MpvConfig, SearchConfig, Theme,
};
use crate::plugins::{
    PluginCustomSection, PluginPanel, PluginPanelItem, PluginTab, PluginUiInject,
    PluginUiLayoutState,
};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Song {
    pub id: String,
    pub title: String,
    pub webpage_url: String,
    #[serde(default)]
    pub uploader: Option<String>,
    #[serde(default)]
    pub duration: Option<f64>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Album {
    pub name: String,
    pub artist: String,
    pub songs: Vec<Song>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Playlist {
    pub name: String,
    pub songs: Vec<Song>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LocalSong {
    pub path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    #[serde(default)]
    pub genre: String,
    #[serde(default)]
    pub year: Option<u32>,
    pub duration: f64,
    pub mtime: u64,
    #[serde(default)]
    pub added_at: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
pub enum LocalViewMode {
    Flat,
    Organized,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalSortMode {
    Title,
    Artist,
    Album,
    Year,
    DateAdded,
}
impl LocalSortMode {
    pub fn next(self) -> Self {
        match self {
            LocalSortMode::Title => LocalSortMode::Artist,
            LocalSortMode::Artist => LocalSortMode::Album,
            LocalSortMode::Album => LocalSortMode::Year,
            LocalSortMode::Year => LocalSortMode::DateAdded,
            LocalSortMode::DateAdded => LocalSortMode::Title,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            LocalSortMode::Title => "title",
            LocalSortMode::Artist => "artist",
            LocalSortMode::Album => "album",
            LocalSortMode::Year => "year",
            LocalSortMode::DateAdded => "date added",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalTagField {
    Title,
    Artist,
    Album,
    Genre,
    Year,
}
impl LocalTagField {
    pub fn next(self) -> Self {
        match self {
            LocalTagField::Title => LocalTagField::Artist,
            LocalTagField::Artist => LocalTagField::Album,
            LocalTagField::Album => LocalTagField::Genre,
            LocalTagField::Genre => LocalTagField::Year,
            LocalTagField::Year => LocalTagField::Title,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            LocalTagField::Title => "title",
            LocalTagField::Artist => "artist",
            LocalTagField::Album => "album",
            LocalTagField::Genre => "genre",
            LocalTagField::Year => "year",
        }
    }
}
impl From<&LocalSong> for Song {
    fn from(ls: &LocalSong) -> Self {
        Self {
            id: ls.path.clone(),
            title: ls.title.clone(),
            webpage_url: ls.path.clone(),
            uploader: Some(ls.artist.clone()),
            duration: Some(ls.duration),
        }
    }
}
impl Song {
    pub fn subtitle(&self) -> String {
        let artist = self
            .uploader
            .clone()
            .unwrap_or_else(|| "Unknown channel".to_owned());
        let duration = self
            .duration
            .map(format_duration)
            .unwrap_or_else(|| "--:--".to_owned());
        format!("{artist} • {duration}")
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerState {
    Idle,
    Searching,
    Playing,
    Paused,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum LocalNavLevel {
    Artists,
    Albums,
    Songs,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Search,
    Results,
    Queue,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Discover,
    Albums,
    Library,
    Local,
    Options,
}
#[derive(Debug, Clone)]
pub enum MainTabKind {
    Stock(Tab),
    Custom(String),
}
#[derive(Debug, Clone)]
pub struct MainTab {
    pub id: String,
    pub title: String,
    pub icon: String,
    pub kind: MainTabKind,
}
#[derive(Debug, Clone)]
pub struct UiLayout {
    pub queue_width_percent: u16,
    pub visualizer_height: u16,
    pub show_progress_bar: bool,
    pub show_volume_bar: bool,
    pub show_statusbar: bool,
    pub show_keybind_hints: bool,
    pub tab_bar_position: String,
    pub tabs_width: u16,
    pub queue_position: String,
}
impl Default for UiLayout {
    fn default() -> Self {
        Self {
            queue_width_percent: 40,
            visualizer_height: 5,
            show_progress_bar: true,
            show_volume_bar: true,
            show_statusbar: true,
            show_keybind_hints: true,
            tab_bar_position: "top".to_owned(),
            tabs_width: 22,
            queue_position: "right".to_owned(),
        }
    }
}
pub fn default_main_tabs() -> Vec<MainTab> {
    vec![
        MainTab { id : "discover".to_owned(), title : "DISCOVER".to_owned(), icon : "♫"
        .to_owned(), kind : MainTabKind::Stock(Tab::Discover), }, MainTab { id : "albums"
        .to_owned(), title : "ALBUMS".to_owned(), icon : "◈".to_owned(), kind :
        MainTabKind::Stock(Tab::Albums), }, MainTab { id : "library".to_owned(), title :
        "LIBRARY".to_owned(), icon : "◉".to_owned(), kind :
        MainTabKind::Stock(Tab::Library), }, MainTab { id : "local".to_owned(), title :
        "LOCAL".to_owned(), icon : "🗀".to_owned(), kind :
        MainTabKind::Stock(Tab::Local), }, MainTab { id : "options".to_owned(), title :
        "OPTIONS".to_owned(), icon : "⚙".to_owned(), kind :
        MainTabKind::Stock(Tab::Options), },
    ]
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepeatMode {
    Off,
    One,
    All,
}
pub const EQ_PRESET_NAMES: [&str; 5] = [
    "Flat",
    "Bass Boost",
    "Vocal Boost",
    "Treble Boost",
    "Night",
];
pub fn eq_preset_bands(app: &App, index: usize) -> [f32; 10] {
    let total = EQ_PRESET_NAMES.len() + app.eq.custom_presets.len();
    let idx = index % total;
    if idx < EQ_PRESET_NAMES.len() {
        match idx {
            0 => [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            1 => [6.0, 5.0, 4.0, 2.0, 1.0, 0.0, -1.0, -2.0, -2.0, -2.0],
            2 => [-2.0, -1.0, 0.0, 2.0, 3.0, 4.0, 4.0, 3.0, 1.0, 0.0],
            3 => [-3.0, -2.0, -1.0, 0.0, 1.0, 2.0, 3.0, 5.0, 6.0, 6.0],
            _ => [3.0, 2.0, 1.0, 0.0, -1.0, -2.0, -2.0, -2.0, -3.0, -3.0],
        }
    } else {
        app.eq.custom_presets[idx - EQ_PRESET_NAMES.len()].bands
    }
}
pub fn eq_preset_name(app: &App, index: usize) -> String {
    let total = EQ_PRESET_NAMES.len() + app.eq.custom_presets.len();
    let idx = index % total;
    if idx < EQ_PRESET_NAMES.len() {
        EQ_PRESET_NAMES[idx].to_string()
    } else {
        app.eq.custom_presets[idx - EQ_PRESET_NAMES.len()].name.clone()
    }
}
impl RepeatMode {
    pub fn next(self) -> Self {
        match self {
            RepeatMode::Off => RepeatMode::One,
            RepeatMode::One => RepeatMode::All,
            RepeatMode::All => RepeatMode::Off,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            RepeatMode::Off => "OFF",
            RepeatMode::One => "ONE",
            RepeatMode::All => "ALL",
        }
    }
}
#[derive(Debug)]
pub struct App {
    pub fft_state: Option<std::sync::Arc<std::sync::Mutex<crate::fft::FftState>>>,
    pub show_fft: bool,
    pub player_state: PlayerState,
    pub focus: Focus,
    pub search_mode: bool,
    pub queue: std::collections::VecDeque<Song>,
    pub selected_queue: usize,
    pub current_song: Option<Song>,
    pub flash_message: String,
    pub flash_until: std::time::Instant,
    pub volume: u8,
    pub muted: bool,
    pub playback_pos: f64,
    pub playback_duration: f64,
    pub repeat_mode: RepeatMode,
    pub theme: Theme,
    pub active_tab: Tab,
    pub options_index: usize,
    pub opt_search_limit: u8,
    pub opt_source: crate::config::SearchSource,
    pub opt_socket: String,
    pub opt_theme: Theme,
    pub opt_mpris_enabled: bool,
    pub opt_mpris_command: Option<String>,
    pub opt_plugins_enabled: bool,
    pub opt_plugins_dir: String,
    pub opt_music_dirs: Vec<String>,
    pub opt_editing: bool,
    pub opt_edit_buffer: String,
    pub key_next: String,
    pub key_prev: String,
    pub key_mute: String,
    pub key_repeat: String,
    pub key_shuffle: String,
    pub key_seek_back: String,
    pub key_seek_forward: String,
    pub key_fft_toggle: String,
    pub key_sequence: String,
    pub key_sequence_started: Option<Instant>,
    pub anim_tick: u64,
    pub recently_played: std::collections::VecDeque<Song>,
    pub main_tabs: Vec<MainTab>,
    pub ui_layout: UiLayout,
    pub storage: crate::storage::Storage,
    pub search: SearchState,
    pub albums: AlbumState,
    pub playlists: PlaylistState,
    pub eq: EqState,
    pub plugin_ui: PluginUiState,
    pub local: LocalLibraryState,
}
impl App {
    pub fn new(storage: crate::storage::Storage) -> Self {
        Self {
            fft_state: None,
            show_fft: false,
            player_state: PlayerState::Idle,
            focus: Focus::Results,
            search_mode: false,
            queue: std::collections::VecDeque::new(),
            selected_queue: 0,
            current_song: None,
            flash_message: "Press / to search YouTube".to_owned(),
            flash_until: std::time::Instant::now() + std::time::Duration::from_secs(4),
            volume: 70,
            muted: false,
            playback_pos: 0.0,
            playback_duration: 0.0,
            repeat_mode: RepeatMode::Off,
            theme: Theme::Dark,
            active_tab: Tab::Discover,
            options_index: 0,
            opt_search_limit: 20,
            opt_source: crate::config::SearchSource::YouTube,
            opt_socket: "/tmp/rs-pug.sock".to_owned(),
            opt_theme: Theme::Dark,
            opt_mpris_enabled: true,
            opt_mpris_command: None,
            opt_plugins_enabled: true,
            opt_plugins_dir: crate::config::GeneralConfig::default().plugins_dir,
            opt_music_dirs: Vec::new(),
            opt_editing: false,
            opt_edit_buffer: String::new(),
            key_next: "n".to_string(),
            key_prev: "p".to_string(),
            key_mute: "m".to_string(),
            key_repeat: "r".to_string(),
            key_shuffle: "z".to_string(),
            key_seek_back: "[".to_string(),
            key_seek_forward: "]".to_string(),
            key_fft_toggle: "C-v".to_string(),
            key_sequence: String::new(),
            key_sequence_started: None,
            anim_tick: 0,
            recently_played: std::collections::VecDeque::new(),
            main_tabs: default_main_tabs(),
            ui_layout: UiLayout::default(),
            storage,
            search: SearchState::default(),
            albums: AlbumState::default(),
            playlists: PlaylistState::default(),
            eq: EqState::default(),
            plugin_ui: PluginUiState::default(),
            local: LocalLibraryState::default(),
        }
    }
    pub fn set_flash(&mut self, msg: impl Into<String>, seconds: u64) {
        self.flash_message = msg.into();
        self.flash_until = Instant::now() + Duration::from_secs(seconds);
    }
    pub fn push_plugin_warning(&mut self, warning: String) {
        if self.plugin_ui.warnings.back() == Some(&warning) {
            return;
        }
        if self.plugin_ui.warnings.len() >= 20 {
            self.plugin_ui.warnings.pop_front();
        }
        self.plugin_ui.warnings.push_back(warning);
    }
    pub fn shown_message(&self) -> &str {
        if Instant::now() <= self.flash_until { &self.flash_message } else { "" }
    }
    pub fn current_selection(&self) -> Option<&Song> {
        self.search.results.get(self.search.selected_result)
    }
    pub fn queue_selection(&self) -> Option<&Song> {
        self.queue.get(self.selected_queue)
    }
    pub fn selected_song_for_context(&self) -> Option<Song> {
        match self.active_tab {
            Tab::Discover => {
                match self.focus {
                    Focus::Results => self.current_selection().cloned(),
                    Focus::Queue => self.queue_selection().cloned(),
                    Focus::Search => None,
                }
            }
            Tab::Albums => {
                let mut current_flat_idx = 0;
                for (i, album) in self.albums.results.iter().enumerate() {
                    let expanded = self.albums.expanded.get(i).copied().unwrap_or(false);
                    let album_size = 1 + if expanded { album.songs.len() } else { 0 };
                    if self.albums.selected_result < current_flat_idx + album_size {
                        if self.albums.selected_result == current_flat_idx {
                            return None;
                        } else {
                            let song_idx = self.albums.selected_result - current_flat_idx
                                - 1;
                            return album.songs.get(song_idx).cloned();
                        }
                    }
                    current_flat_idx += album_size;
                }
                None
            }
            Tab::Library => {
                self.playlists
                    .playlists
                    .get(self.playlists.selected_playlist)
                    .and_then(|p| p.songs.get(self.playlists.selected_song).cloned())
            }
            Tab::Local => {
                match self.focus {
                    Focus::Results => {
                        let relative_idx = self
                            .local
                            .selected_song
                            .saturating_sub(self.local.offset);
                        self.local.window.get(relative_idx).map(Song::from)
                    }
                    Focus::Queue => self.queue_selection().cloned(),
                    Focus::Search => None,
                }
            }
            Tab::Options => None,
        }
    }
    pub fn apply_config(&mut self, cfg: &Config) {
        self.opt_search_limit = cfg.search.limit.max(1);
        self.opt_source = cfg.search.source;
        self.opt_socket = cfg.mpv.socket.clone();
        self.opt_theme = cfg.general.theme.clone();
        self.opt_mpris_enabled = cfg.general.mpris_enabled;
        self.opt_mpris_command = cfg.general.mpris_command.clone();
        self.opt_plugins_enabled = cfg.general.plugins_enabled;
        self.opt_plugins_dir = cfg.general.plugins_dir.clone();
        self.opt_music_dirs = cfg.general.music_directories.clone();
        self.theme = cfg.general.theme.clone();
        self.plugin_ui.allow_lua_ui_changes = cfg.lua.allow_lua_ui_changes;
        self.apply_keybinds(&cfg.keybinds);
    }
    pub fn build_config(&self) -> Config {
        Config {
            general: GeneralConfig {
                mpris_enabled: self.opt_mpris_enabled,
                mpris_command: self.opt_mpris_command.clone(),
                theme: self.opt_theme.clone(),
                plugins_enabled: self.opt_plugins_enabled,
                plugins_dir: self.opt_plugins_dir.clone(),
                music_directories: self.opt_music_dirs.clone(),
                fft_visualizer_default: self.show_fft,
            },
            search: SearchConfig {
                limit: self.opt_search_limit.max(1),
                source: self.opt_source,
            },
            mpv: MpvConfig {
                socket: self.opt_socket.clone(),
            },
            keybinds: KeybindsConfig {
                next: self.key_next.clone(),
                prev: self.key_prev.clone(),
                mute: self.key_mute.clone(),
                repeat: self.key_repeat.clone(),
                shuffle: self.key_shuffle.clone(),
                seek_back: self.key_seek_back.clone(),
                seek_forward: self.key_seek_forward.clone(),
                fft_toggle: self.key_fft_toggle.clone(),
            },
            lua: LuaConfig {
                allow_lua_ui_changes: self.plugin_ui.allow_lua_ui_changes,
            },
        }
    }
    pub fn current_layout_state(&self) -> PluginUiLayoutState {
        PluginUiLayoutState {
            queue_width_percent: self.ui_layout.queue_width_percent,
            visualizer_height: self.ui_layout.visualizer_height,
            tab_bar_position: self.ui_layout.tab_bar_position.clone(),
            tabs_width: self.ui_layout.tabs_width,
            queue_position: self.ui_layout.queue_position.clone(),
        }
    }
    pub fn visible_section_ids(&self) -> Vec<String> {
        self.plugin_ui
            .custom_sections
            .iter()
            .filter(|s| !self.plugin_ui.hidden_sections.iter().any(|id| id == &s.id))
            .map(|s| s.id.clone())
            .collect()
    }
    pub fn active_tab_index(&self) -> usize {
        self.main_tabs
            .iter()
            .position(|tab| match &tab.kind {
                MainTabKind::Stock(t) => {
                    self.plugin_ui.active_custom_tab.is_none() && *t == self.active_tab
                }
                MainTabKind::Custom(id) => {
                    self.plugin_ui.active_custom_tab.as_ref() == Some(id)
                }
            })
            .map(|i| i + 1)
            .unwrap_or(1)
    }
    fn apply_keybinds(&mut self, keybinds: &KeybindsConfig) {
        self.key_next = keybinds.next.clone();
        self.key_prev = keybinds.prev.clone();
        self.key_mute = keybinds.mute.clone();
        self.key_repeat = keybinds.repeat.clone();
        self.key_shuffle = keybinds.shuffle.clone();
        self.key_seek_back = keybinds.seek_back.clone();
        self.key_seek_forward = keybinds.seek_forward.clone();
        self.key_fft_toggle = keybinds.fft_toggle.clone();
    }
}
fn format_duration(seconds: f64) -> String {
    let secs = seconds.round() as u64;
    let m = secs / 60;
    let s = secs % 60;
    format!("{m:02}:{s:02}")
}

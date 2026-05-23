use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use crate::config::{Config, GeneralConfig, KeybindsConfig, MpvConfig, SearchConfig, Theme};
use crate::plugins::{PluginPanel, PluginTab};
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
    pub duration: f64,
    pub mtime: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
pub enum LocalViewMode {
    Flat,
    Organized,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepeatMode {
    Off,
    One,
    All,
}

pub const EQ_PRESET_NAMES: [&str; 5] =
    ["Flat", "Bass Boost", "Vocal Boost", "Treble Boost", "Night"];

pub fn eq_preset_bands(app: &App, index: usize) -> [f32; 10] {
    let total = EQ_PRESET_NAMES.len() + app.custom_eq_presets.len();
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
        app.custom_eq_presets[idx - EQ_PRESET_NAMES.len()].bands
    }
}

pub fn eq_preset_name(app: &App, index: usize) -> String {
    let total = EQ_PRESET_NAMES.len() + app.custom_eq_presets.len();
    let idx = index % total;
    if idx < EQ_PRESET_NAMES.len() {
        EQ_PRESET_NAMES[idx].to_string()
    } else {
        app.custom_eq_presets[idx - EQ_PRESET_NAMES.len()]
            .name
            .clone()
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
    pub player_state: PlayerState,
    pub focus: Focus,
    pub search_mode: bool,
    pub search_query: String,
    pub search_results: Vec<Song>,
    pub selected_result: usize,
    pub album_search_query: String,
    pub album_results: Vec<Album>,
    pub selected_album_result: usize,
    pub album_expanded: Vec<bool>,
    pub queue: VecDeque<Song>,
    pub selected_queue: usize,
    pub current_song: Option<Song>,
    pub flash_message: String,
    pub flash_until: Instant,
    pub volume: u8,
    pub muted: bool,
    pub playback_pos: f64,
    pub playback_duration: f64,
    pub repeat_mode: RepeatMode,
    pub theme: Theme,
    pub active_tab: Tab,
    pub playlists: Vec<Playlist>,
    pub selected_playlist: usize,
    pub context_open: bool,
    pub context_index: usize,
    pub playlist_expanded: Vec<bool>,
    pub selected_playlist_song: usize,
    pub options_index: usize,
    pub opt_search_limit: u8,
    pub opt_source: crate::config::SearchSource,
    pub opt_socket: String,
    pub opt_theme: Theme,
    pub opt_music_dirs: Vec<String>,
    pub opt_editing: bool,
    pub opt_edit_buffer: String,
    pub key_next: char,
    pub key_prev: char,
    pub key_mute: char,
    pub key_repeat: char,
    pub key_shuffle: char,
    pub key_seek_back: char,
    pub key_seek_forward: char,
    pub anim_tick: u64,
    pub confirm_delete_playlist: bool,
    pub delete_playlist_name: String,
    pub eq_enabled: bool,
    pub eq_bands: [f32; 10],
    pub eq_focus_band: usize,
    pub eq_preset_index: usize,
    pub custom_eq_presets: Vec<crate::config::EqPreset>,
    pub recently_played: VecDeque<Song>,
    pub scanning: bool,
    pub local_library_window: Vec<LocalSong>,
    pub local_library_offset: usize,
    pub local_library_total: usize,
    pub local_view_mode: LocalViewMode,
    pub selected_local_song: usize,
    pub local_nav_level: LocalNavLevel,
    pub local_nav_artist: Option<String>,
    pub local_nav_album: Option<String>,
    pub selected_local_nav_idx: usize,
    pub adding_song_to_playlist: bool,
    pub plugin_panels: Vec<PluginPanel>,
    pub plugin_tabs: Vec<PluginTab>,
    pub active_plugin_tab: Option<String>,
    pub storage: crate::storage::Storage,
}

impl App {
    pub fn new(storage: crate::storage::Storage) -> Self {
        Self {
            player_state: PlayerState::Idle,
            focus: Focus::Results,
            search_mode: false,
            search_query: String::new(),
            search_results: Vec::new(),
            selected_result: 0,
            album_search_query: String::new(),
            album_results: Vec::new(),
            selected_album_result: 0,
            album_expanded: Vec::new(),
            queue: VecDeque::new(),
            selected_queue: 0,
            current_song: None,
            flash_message: "Press / to search YouTube".to_owned(),
            flash_until: Instant::now() + Duration::from_secs(4),
            volume: 70,
            muted: false,
            playback_pos: 0.0,
            playback_duration: 0.0,
            repeat_mode: RepeatMode::Off,
            theme: Theme::Dark,
            active_tab: Tab::Discover,
            playlists: Vec::new(),
            selected_playlist: 0,
            context_open: false,
            context_index: 0,
            playlist_expanded: Vec::new(),
            selected_playlist_song: 0,
            options_index: 0,
            opt_search_limit: 20,
            opt_source: crate::config::SearchSource::YouTube,
            opt_socket: "/tmp/rs-pug.sock".to_owned(),
            opt_theme: Theme::Dark,
            opt_music_dirs: Vec::new(),
            opt_editing: false,
            opt_edit_buffer: String::new(),
            key_next: 'n',
            key_prev: 'p',
            key_mute: 'm',
            key_repeat: 'r',
            key_shuffle: 'z',
            key_seek_back: '[',
            key_seek_forward: ']',
            anim_tick: 0,
            confirm_delete_playlist: false,
            delete_playlist_name: String::new(),
            eq_enabled: false,
            eq_bands: [0.0f32; 10],
            eq_focus_band: 0,
            eq_preset_index: 0,
            custom_eq_presets: Vec::new(),
            recently_played: VecDeque::new(),
            scanning: false,
            local_library_window: Vec::new(),
            local_library_offset: 0,
            local_library_total: 0,
            local_view_mode: LocalViewMode::Flat,
            selected_local_song: 0,
            local_nav_level: LocalNavLevel::Artists,
            local_nav_artist: None,
            local_nav_album: None,
            selected_local_nav_idx: 0,
            adding_song_to_playlist: false,
            plugin_panels: Vec::new(),
            plugin_tabs: Vec::new(),
            active_plugin_tab: None,
            storage,
        }
    }

    pub fn set_flash(&mut self, msg: impl Into<String>, seconds: u64) {
        self.flash_message = msg.into();
        self.flash_until = Instant::now() + Duration::from_secs(seconds);
    }

    pub fn shown_message(&self) -> &str {
        if Instant::now() <= self.flash_until {
            &self.flash_message
        } else {
            ""
        }
    }

    pub fn current_selection(&self) -> Option<&Song> {
        self.search_results.get(self.selected_result)
    }

    pub fn queue_selection(&self) -> Option<&Song> {
        self.queue.get(self.selected_queue)
    }

    pub fn selected_song_for_context(&self) -> Option<Song> {
        match self.active_tab {
            Tab::Discover => match self.focus {
                Focus::Results => self.current_selection().cloned(),
                Focus::Queue => self.queue_selection().cloned(),
                Focus::Search => None,
            },
            Tab::Albums => {
                let mut current_flat_idx = 0;
                for (i, album) in self.album_results.iter().enumerate() {
                    let expanded = self.album_expanded.get(i).copied().unwrap_or(false);
                    let album_size = 1 + if expanded { album.songs.len() } else { 0 };
                    if self.selected_album_result < current_flat_idx + album_size {
                        if self.selected_album_result == current_flat_idx {
                            return None;
                        } else {
                            let song_idx = self.selected_album_result - current_flat_idx - 1;
                            return album.songs.get(song_idx).cloned();
                        }
                    }
                    current_flat_idx += album_size;
                }
                None
            }
            Tab::Library => self
                .playlists
                .get(self.selected_playlist)
                .and_then(|p| p.songs.get(self.selected_playlist_song).cloned()),
            Tab::Local => match self.focus {
                Focus::Results => {
                    let relative_idx = self
                        .selected_local_song
                        .saturating_sub(self.local_library_offset);
                    self.local_library_window.get(relative_idx).map(Song::from)
                }
                Focus::Queue => self.queue_selection().cloned(),
                Focus::Search => None,
            },
            Tab::Options => None,
        }
    }

    pub fn apply_config(&mut self, cfg: &Config) {
        self.opt_search_limit = cfg.search.limit.max(1);
        self.opt_source = cfg.search.source;
        self.opt_socket = cfg.mpv.socket.clone();
        self.opt_theme = cfg.general.theme.clone();
        self.opt_music_dirs = cfg.general.music_directories.clone();
        self.theme = cfg.general.theme.clone();
        self.apply_keybinds(&cfg.keybinds);
    }

    pub fn build_config(&self) -> Config {
        Config {
            general: GeneralConfig {
                mpris_enabled: true,
                mpris_command: None,
                theme: self.opt_theme.clone(),
                plugins_enabled: true,
                plugins_dir: GeneralConfig::default().plugins_dir,
                music_directories: self.opt_music_dirs.clone(),
            },
            search: SearchConfig {
                limit: self.opt_search_limit.max(1),
                source: self.opt_source,
            },
            mpv: MpvConfig {
                socket: self.opt_socket.clone(),
            },
            keybinds: KeybindsConfig {
                next: self.key_next,
                prev: self.key_prev,
                mute: self.key_mute,
                repeat: self.key_repeat,
                shuffle: self.key_shuffle,
                seek_back: self.key_seek_back,
                seek_forward: self.key_seek_forward,
            },
        }
    }

    fn apply_keybinds(&mut self, keybinds: &KeybindsConfig) {
        self.key_next = keybinds.next;
        self.key_prev = keybinds.prev;
        self.key_mute = keybinds.mute;
        self.key_repeat = keybinds.repeat;
        self.key_shuffle = keybinds.shuffle;
        self.key_seek_back = keybinds.seek_back;
        self.key_seek_forward = keybinds.seek_forward;
    }
}

fn format_duration(seconds: f64) -> String {
    let secs = seconds.round() as u64;
    let m = secs / 60;
    let s = secs % 60;
    format!("{m:02}:{s:02}")
}

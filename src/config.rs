use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub search: SearchConfig,
    #[serde(default)]
    pub mpv: MpvConfig,
    #[serde(default)]
    pub keybinds: KeybindsConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            search: SearchConfig::default(),
            mpv: MpvConfig::default(),
            keybinds: KeybindsConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Dark,
    Light,
    Custom,
    Nord,
    Gruvbox,
    Mono,
}

impl Default for Theme {
    fn default() -> Self {
        Theme::Dark
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GeneralConfig {
    #[serde(default = "default_true")]
    pub mpris_enabled: bool,
    #[serde(default)]
    pub mpris_command: Option<String>,
    #[serde(default)]
    pub theme: Theme,
    #[serde(default = "default_true")]
    pub plugins_enabled: bool,
    #[serde(default = "default_plugins_dir")]
    pub plugins_dir: String,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            mpris_enabled: true,
            mpris_command: None,
            theme: Theme::Dark,
            plugins_enabled: true,
            plugins_dir: default_plugins_dir(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KeybindsConfig {
    #[serde(default = "default_next_key")]
    pub next: char,
    #[serde(default = "default_prev_key")]
    pub prev: char,
    #[serde(default = "default_mute_key")]
    pub mute: char,
    #[serde(default = "default_repeat_key")]
    pub repeat: char,
    #[serde(default = "default_shuffle_key")]
    pub shuffle: char,
    #[serde(default = "default_seek_back_key")]
    pub seek_back: char,
    #[serde(default = "default_seek_forward_key")]
    pub seek_forward: char,
}

impl Default for KeybindsConfig {
    fn default() -> Self {
        Self {
            next: default_next_key(),
            prev: default_prev_key(),
            mute: default_mute_key(),
            repeat: default_repeat_key(),
            shuffle: default_shuffle_key(),
            seek_back: default_seek_back_key(),
            seek_forward: default_seek_forward_key(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearchConfig {
    #[serde(default = "default_limit")]
    pub limit: u8,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            limit: default_limit(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MpvConfig {
    #[serde(default = "default_socket")]
    pub socket: String,
}

impl Default for MpvConfig {
    fn default() -> Self {
        Self {
            socket: default_socket(),
        }
    }
}

pub fn load_config() -> Config {
    let paths = config_paths();

    for path in paths {
        if let Ok(raw) = fs::read_to_string(&path) {
            if let Ok(cfg) = toml::from_str::<Config>(&raw) {
                return cfg;
            }
        }
    }

    Config::default()
}

pub fn save_config(config: &Config) {
    let path = user_config_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(raw) = toml::to_string_pretty(config) {
        let _ = fs::write(path, raw);
    }
}

fn config_paths() -> Vec<PathBuf> {
    let mut paths = vec![PathBuf::from("rs-pug.toml"), PathBuf::from("pug.toml")];
    paths.push(user_config_path());
    paths
}

fn user_config_path() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".config/rs-pug/config.toml")
    } else {
        PathBuf::from("rs-pug.toml")
    }
}

fn default_true() -> bool {
    true
}

fn default_limit() -> u8 {
    20
}

fn default_socket() -> String {
    "/tmp/rs-pug.sock".to_owned()
}

fn default_next_key() -> char {
    'n'
}

fn default_prev_key() -> char {
    'p'
}

fn default_mute_key() -> char {
    'm'
}

fn default_repeat_key() -> char {
    'r'
}

fn default_shuffle_key() -> char {
    'z'
}

fn default_seek_back_key() -> char {
    '['
}

fn default_seek_forward_key() -> char {
    ']'
}

fn default_plugins_dir() -> String {
    if let Ok(home) = std::env::var("HOME") {
        format!("{home}/.config/rs-pug/plugins")
    } else {
        ".config/rs-pug/plugins".to_owned()
    }
}

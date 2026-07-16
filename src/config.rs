use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EqPreset {
    pub name: String,
    pub bands: [f32; 10],
}

impl Default for EqPreset {
    fn default() -> Self {
        Self {
            name: "Default".to_string(),
            bands: [0.0; 10],
        }
    }
}

fn sanitize_preset_filename(name: &str) -> Result<String, std::io::Error> {
    let sanitized = name.replace(['/', '\\'], "_");
    if sanitized.trim().is_empty() || sanitized.contains("..") {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "invalid preset name",
        ));
    }
    Ok(sanitized)
}

pub fn save_eq_preset(preset: &EqPreset) -> Result<(), std::io::Error> {
    let home = std::env::var("HOME")
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::NotFound, "HOME not set"))?;
    let safe_name = sanitize_preset_filename(&preset.name)?;
    let path = PathBuf::from(home).join(format!(".config/rs-pug/eqpresets/{}.json", safe_name));
    let raw = serde_json::to_string_pretty(preset)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    fs::write(path, raw)
}

pub fn load_eq_presets() -> Vec<EqPreset> {
    let mut presets = Vec::new();
    let home = std::env::var("HOME").ok();
    if let Some(home_dir) = home {
        let dir = PathBuf::from(home_dir).join(".config/rs-pug/eqpresets");
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Ok(raw) = fs::read_to_string(path) {
                        if let Ok(preset) = serde_json::from_str::<EqPreset>(&raw) {
                            presets.push(preset);
                        }
                    }
                }
            }
        }
    }
    presets
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Palette {
    pub text: [u8; 3],
    pub dim: [u8; 3],
    pub muted: [u8; 3],
    pub info: [u8; 3],
    pub warn: [u8; 3],
    pub ok: [u8; 3],
    pub primary: [u8; 3],
    pub accent2: [u8; 3],
    pub accent3: [u8; 3],
    #[serde(default = "default_spectrum")]
    pub spectrum: Vec<[u8; 3]>,
}

impl Palette {
    pub fn get_color(&self, field: &str) -> ratatui::style::Color {
        let rgb = match field {
            "text" => self.text,
            "dim" => self.dim,
            "muted" => self.muted,
            "info" => self.info,
            "warn" => self.warn,
            "ok" => self.ok,
            "primary" => self.primary,
            "accent2" => self.accent2,
            "accent3" => self.accent3,
            _ => self.primary,
        };
        ratatui::style::Color::Rgb(rgb[0], rgb[1], rgb[2])
    }

    pub fn spectrum_colors(&self) -> Vec<ratatui::style::Color> {
        let source = if self.spectrum.is_empty() {
            default_spectrum()
        } else {
            self.spectrum.clone()
        };
        source
            .into_iter()
            .map(|rgb| ratatui::style::Color::Rgb(rgb[0], rgb[1], rgb[2]))
            .collect()
    }
}

fn default_spectrum() -> Vec<[u8; 3]> {
    vec![
        [255, 62, 205],
        [230, 72, 255],
        [175, 82, 255],
        [118, 108, 255],
        [72, 168, 255],
        [38, 222, 255],
        [0, 255, 198],
        [0, 255, 138],
        [112, 255, 82],
        [255, 235, 48],
        [255, 158, 38],
        [255, 78, 78],
    ]
}

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
    #[serde(default)]
    pub lua: LuaConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            search: SearchConfig::default(),
            mpv: MpvConfig::default(),
            keybinds: KeybindsConfig::default(),
            lua: LuaConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Dark,
    Light,
    Nord,
    Gruvbox,
    Mono,
    #[serde(untagged)]
    Custom(String),
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
    #[serde(default = "default_music_directories")]
    pub music_directories: Vec<String>,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            mpris_enabled: true,
            mpris_command: None,
            theme: Theme::Dark,
            plugins_enabled: true,
            plugins_dir: default_plugins_dir(),
            music_directories: default_music_directories(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LuaConfig {
    #[serde(default, rename = "allow-lua-ui-changes", alias = "allow_lua_ui_changes")]
    pub allow_lua_ui_changes: bool,
}

impl Default for LuaConfig {
    fn default() -> Self {
        Self {
            allow_lua_ui_changes: false,
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

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SearchSource {
    YouTube,
    SoundCloud,
}

impl Default for SearchSource {
    fn default() -> Self {
        SearchSource::YouTube
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearchConfig {
    #[serde(default = "default_limit")]
    pub limit: u8,
    #[serde(default)]
    pub source: SearchSource,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            limit: default_limit(),
            source: SearchSource::default(),
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

pub fn ensure_default_dirs() {
    let home = std::env::var("HOME").ok();
    if let Some(home_dir) = home {
        let music_local = PathBuf::from(&home_dir).join(".config/rs-pug/music-local");
        let _ = fs::create_dir_all(music_local);

        let plugins_dir = PathBuf::from(&home_dir).join(".config/rs-pug/plugins");
        let _ = fs::create_dir_all(plugins_dir);

        let themes_dir = PathBuf::from(&home_dir).join(".config/rs-pug/themes");
        let _ = fs::create_dir_all(themes_dir);

        let eq_presets_dir = PathBuf::from(&home_dir).join(".config/rs-pug/eqpresets");
        let _ = fs::create_dir_all(eq_presets_dir);
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

pub fn config_paths() -> Vec<PathBuf> {
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

fn default_music_directories() -> Vec<String> {
    vec!["~/.config/rs-pug/music-local/".to_string()]
}

pub fn theme_to_str(theme: &Theme) -> String {
    match theme {
        Theme::Dark => "dark".to_string(),
        Theme::Light => "light".to_string(),
        Theme::Nord => "nord".to_string(),
        Theme::Gruvbox => "gruvbox".to_string(),
        Theme::Mono => "mono".to_string(),
        Theme::Custom(name) => name.clone(),
    }
}

pub fn theme_from_str(s: &str) -> Theme {
    match s {
        "dark" => Theme::Dark,
        "light" => Theme::Light,
        "nord" => Theme::Nord,
        "gruvbox" => Theme::Gruvbox,
        "mono" => Theme::Mono,
        name => Theme::Custom(name.to_string()),
    }
}

pub fn get_available_themes() -> Vec<String> {
    let mut themes = vec![
        "dark".to_string(),
        "light".to_string(),
        "nord".to_string(),
        "gruvbox".to_string(),
        "mono".to_string(),
    ];

    let home = std::env::var("HOME").ok();
    if let Some(home_dir) = home {
        let themes_dir = PathBuf::from(&home_dir).join(".config/rs-pug/themes");
        if let Ok(entries) = fs::read_dir(themes_dir) {
            for entry in entries.flatten() {
                if let Ok(name) = entry.file_name().into_string() {
                    if name.ends_with(".json") {
                        let theme_name = name.trim_end_matches(".json").to_string();
                        if !themes.contains(&theme_name) {
                            themes.push(theme_name);
                        }
                    }
                }
            }
        }
    }
    themes
}

pub fn load_palette(theme: &Theme) -> Palette {
    let theme_name = theme_to_str(theme);

    let home = std::env::var("HOME").ok();
    if let Some(home_dir) = home {
        let path = PathBuf::from(&home_dir).join(format!(".config/rs-pug/themes/{}.json", theme_name));
        if let Ok(raw) = fs::read_to_string(path) {
            if let Ok(pal) = serde_json::from_str::<Palette>(&raw) {
                return pal;
            }
        }
    }

    match theme {
        Theme::Light => Palette {
            text: [20, 20, 35],
            dim: [90, 90, 115],
            muted: [140, 135, 158],
            info: [0, 120, 210],
            warn: [185, 128, 0],
            ok: [0, 158, 88],
            primary: [20, 120, 220],
            accent2: [110, 10, 210],
            accent3: [0, 158, 210],
            spectrum: vec![
                [20, 120, 220],
                [40, 130, 210],
                [70, 120, 220],
                [110, 10, 210],
                [140, 60, 200],
                [0, 158, 210],
                [0, 158, 150],
                [0, 158, 88],
                [90, 170, 60],
                [185, 128, 0],
                [210, 100, 30],
                [200, 50, 60],
            ],
        },
        Theme::Nord => Palette {
            text: [216, 222, 233],
            dim: [76, 86, 106],
            muted: [129, 161, 193],
            info: [136, 192, 208],
            warn: [235, 203, 139],
            ok: [163, 190, 140],
            primary: [94, 129, 172],
            accent2: [129, 161, 193],
            accent3: [136, 192, 208],
            spectrum: vec![
                [94, 129, 172],
                [110, 145, 180],
                [129, 161, 193],
                [136, 192, 208],
                [143, 188, 187],
                [163, 190, 140],
                [180, 195, 130],
                [235, 203, 139],
                [208, 135, 112],
                [191, 97, 106],
                [180, 142, 173],
                [129, 161, 193],
            ],
        },
        Theme::Gruvbox => Palette {
            text: [235, 219, 178],
            dim: [102, 92, 84],
            muted: [168, 153, 132],
            info: [131, 165, 152],
            warn: [250, 189, 47],
            ok: [184, 187, 38],
            primary: [215, 153, 33],
            accent2: [211, 134, 155],
            accent3: [104, 157, 106],
            spectrum: vec![
                [251, 73, 52],
                [254, 128, 25],
                [250, 189, 47],
                [184, 187, 38],
                [142, 192, 124],
                [104, 157, 106],
                [131, 165, 152],
                [69, 133, 136],
                [211, 134, 155],
                [214, 93, 14],
                [215, 153, 33],
                [204, 36, 29],
            ],
        },
        Theme::Mono => Palette {
            text: [230, 230, 230],
            dim: [90, 90, 90],
            muted: [150, 150, 150],
            info: [190, 190, 190],
            warn: [220, 220, 220],
            ok: [200, 200, 200],
            primary: [245, 245, 245],
            accent2: [210, 210, 210],
            accent3: [175, 175, 175],
            spectrum: vec![[255, 255, 255]],
        },
        _ => Palette {
            text: [225, 218, 248],
            dim: [68, 62, 102],
            muted: [108, 100, 140],
            info: [82, 216, 255],
            warn: [255, 205, 52],
            ok: [52, 255, 162],
            primary: [255, 62, 205],
            accent2: [152, 82, 255],
            accent3: [0, 228, 255],
            spectrum: default_spectrum(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_preset_filename_keeps_plain_names() {
        assert_eq!(sanitize_preset_filename("My Preset").unwrap(), "My Preset");
    }

    #[test]
    fn sanitize_preset_filename_strips_path_separators() {
        assert_eq!(
            sanitize_preset_filename("foo/bar\\baz").unwrap(),
            "foo_bar_baz"
        );
    }

    #[test]
    fn sanitize_preset_filename_rejects_traversal() {
        assert!(sanitize_preset_filename("../../etc/passwd").is_err());
        assert!(sanitize_preset_filename("..").is_err());
    }

    #[test]
    fn sanitize_preset_filename_rejects_empty() {
        assert!(sanitize_preset_filename("").is_err());
        assert!(sanitize_preset_filename("   ").is_err());
    }

    #[test]
    fn save_eq_preset_does_not_escape_presets_dir() {
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("HOME", dir.path());

        let preset = EqPreset {
            name: "../../evil".to_string(),
            bands: [0.0; 10],
        };
        let result = save_eq_preset(&preset);
        assert!(result.is_err());

        let escaped = dir.path().join("evil.json");
        assert!(!escaped.exists());
        let presets_dir = dir.path().join(".config/rs-pug/eqpresets");
        if presets_dir.exists() {
            assert_eq!(fs::read_dir(presets_dir).unwrap().count(), 0);
        }
    }
}

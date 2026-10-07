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
        return Err(
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid preset name"),
        );
    }
    Ok(sanitized)
}
pub fn save_eq_preset(preset: &EqPreset) -> Result<(), std::io::Error> {
    let home = std::env::var("HOME")
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::NotFound, "HOME not set"))?;
    let safe_name = sanitize_preset_filename(&preset.name)?;
    let path = PathBuf::from(home)
        .join(format!(".config/rs-pug/eqpresets/{}.json", safe_name));
    let raw = serde_json::to_string_pretty(preset)
        .map_err(std::io::Error::other)?;
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
        [255, 62, 205], [230, 72, 255], [175, 82, 255], [118, 108, 255], [72, 168, 255],
        [38, 222, 255], [0, 255, 198], [0, 255, 138], [112, 255, 82], [255, 235, 48],
        [255, 158, 38], [255, 78, 78],
    ]
}
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
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
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    Dark,
    Light,
    Nord,
    Gruvbox,
    Mono,
    #[serde(untagged)]
    Custom(String),
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GeneralConfig {
    #[serde(default = "default_true")]
    pub mpris_enabled: bool,
    // Legacy compat
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
    #[serde(default = "default_download_format")]
    pub download_format: String,
    #[serde(default)]
    pub download_dir: Option<String>,
    #[serde(default)]
    pub fft_visualizer_default: bool,
    #[serde(default = "default_true")]
    pub smart_playlists_enabled: bool,
    #[serde(default = "default_true")]
    pub image_background: bool,
    #[serde(default = "default_true")]
    pub icons: bool,
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
            download_format: default_download_format(),
            download_dir: None,
            fft_visualizer_default: false,
            smart_playlists_enabled: true,
            image_background: true,
            icons: true,
        }
    }
}
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct LuaConfig {
    #[serde(default, rename = "allow-lua-ui-changes", alias = "allow_lua_ui_changes")]
    pub allow_lua_ui_changes: bool,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KeybindsConfig {
    #[serde(default = "default_next_key")]
    pub next: String,
    #[serde(default = "default_prev_key")]
    pub prev: String,
    #[serde(default = "default_mute_key")]
    pub mute: String,
    #[serde(default = "default_repeat_key")]
    pub repeat: String,
    #[serde(default = "default_shuffle_key")]
    pub shuffle: String,
    #[serde(default = "default_seek_back_key")]
    pub seek_back: String,
    #[serde(default = "default_seek_forward_key")]
    pub seek_forward: String,
    #[serde(default = "default_fft_toggle_key")]
    pub fft_toggle: String,
    #[serde(default = "default_minimal_toggle_key")]
    pub minimal_toggle: String,
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
            fft_toggle: default_fft_toggle_key(),
            minimal_toggle: default_minimal_toggle_key(),
        }
    }
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SearchSource {
    #[default]
    YouTube,
    SoundCloud,
    Sonum,
    Custom(u8),
}
pub const MAX_CUSTOM_SOURCES: usize = 255;
pub const DEFAULT_COMMAND_TIMEOUT_SECS: u64 = 15;
pub const MAX_COMMAND_TIMEOUT_SECS: u64 = 120;
fn default_command_timeout() -> u64 {
    DEFAULT_COMMAND_TIMEOUT_SECS
}
pub fn normalize_command_timeout(value: Option<u64>) -> u64 {
    match value {
        None | Some(0) => DEFAULT_COMMAND_TIMEOUT_SECS,
        Some(n) => n.min(MAX_COMMAND_TIMEOUT_SECS),
    }
}
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct CustomSource {
    pub name: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub albums: bool,
    #[serde(flatten)]
    pub kind: CustomSourceKind,
}
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum CustomSourceKind {
    Ytdlp {
        prefix: String,
    },
    Command {
        command: Vec<String>,
        #[serde(default = "default_command_timeout")]
        timeout_secs: u64,
    },
}
#[derive(Debug, Clone, Deserialize, Default)]
struct RawCustomSource {
    #[serde(default)]
    name: Option<toml::Value>,
    #[serde(default, rename = "type")]
    kind: Option<toml::Value>,
    #[serde(default)]
    prefix: Option<toml::Value>,
    #[serde(default)]
    command: Option<toml::Value>,
    #[serde(default)]
    timeout_secs: Option<toml::Value>,
    #[serde(default)]
    albums: Option<toml::Value>,
}
#[derive(Debug, Clone)]
pub struct SearchConfig {
    pub limit: u8,
    pub source: SearchSource,
    pub custom_sources: Vec<CustomSource>,
}
impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            limit: default_limit(),
            source: SearchSource::default(),
            custom_sources: Vec::new(),
        }
    }
}
pub fn normalize_source_name(name: &str) -> String {
    name.trim().to_lowercase()
}
pub fn source_names_equal(a: &str, b: &str) -> bool {
    normalize_source_name(a) == normalize_source_name(b)
}
fn is_builtin_name(normalized: &str) -> bool {
    matches!(normalized, "youtube" | "soundcloud" | "sonum")
}
pub fn available_source_names(customs: &[CustomSource]) -> Vec<String> {
    let mut names = vec![
        "youtube".to_owned(),
        "soundcloud".to_owned(),
        "sonum".to_owned(),
    ];
    names.extend(customs.iter().map(|c| c.name.clone()));
    names
}
pub fn resolve_source_name(raw: Option<&str>, customs: &[CustomSource]) -> SearchSource {
    let Some(name) = raw else {
        return SearchSource::YouTube;
    };
    let normalized = normalize_source_name(name);
    if normalized.is_empty() {
        return SearchSource::YouTube;
    }
    if normalized == "youtube" {
        return SearchSource::YouTube;
    }
    if normalized == "soundcloud" {
        return SearchSource::SoundCloud;
    }
    if normalized == "sonum" {
        return SearchSource::Sonum;
    }
    customs
        .iter()
        .position(|c| source_names_equal(&c.name, name))
        .map(|i| SearchSource::Custom(i as u8))
        .unwrap_or(SearchSource::YouTube)
}
pub fn source_persist_name(source: SearchSource, customs: &[CustomSource]) -> String {
    match source {
        SearchSource::YouTube => "youtube".to_owned(),
        SearchSource::SoundCloud => "soundcloud".to_owned(),
        SearchSource::Sonum => "sonum".to_owned(),
        SearchSource::Custom(i) => customs
            .get(i as usize)
            .map(|c| c.name.clone())
            .unwrap_or_else(|| "youtube".to_owned()),
    }
}
pub fn source_label(source: SearchSource, customs: &[CustomSource]) -> String {
    match source {
        SearchSource::YouTube => "YouTube".to_owned(),
        SearchSource::SoundCloud => "SoundCloud".to_owned(),
        SearchSource::Sonum => "Sonum".to_owned(),
        SearchSource::Custom(i) => customs
            .get(i as usize)
            .map(|c| c.name.clone())
            .unwrap_or_else(|| "Unknown".to_owned()),
    }
}
pub fn next_source(current: SearchSource, len: usize) -> SearchSource {
    match current {
        SearchSource::YouTube => SearchSource::SoundCloud,
        SearchSource::SoundCloud => SearchSource::Sonum,
        SearchSource::Sonum => {
            if len > 0 {
                SearchSource::Custom(0)
            } else {
                SearchSource::YouTube
            }
        }
        SearchSource::Custom(i) => {
            let next = i as usize + 1;
            if next < len {
                SearchSource::Custom(next as u8)
            } else {
                SearchSource::YouTube
            }
        }
    }
}
pub fn custom_source_for(source: SearchSource, customs: &[CustomSource]) -> Option<&CustomSource> {
    match source {
        SearchSource::Custom(i) => customs.get(i as usize),
        _ => None,
    }
}
fn validate_one_raw(
    raw: &RawCustomSource,
    seen: &mut std::collections::HashSet<String>,
    warnings: &mut Vec<String>,
) -> Option<CustomSource> {
    let name = raw
        .name
        .as_ref()
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .trim()
        .to_owned();
    if name.is_empty() {
        warnings.push("skipping custom source with empty name".to_owned());
        return None;
    }
    let normalized = normalize_source_name(&name);
    if is_builtin_name(&normalized) {
        warnings.push(format!(
            "skipping custom source '{name}': name clashes with built-in"
        ));
        return None;
    }
    if !seen.insert(normalized) {
        warnings.push(format!("skipping duplicate custom source '{name}'"));
        return None;
    }
    let kind = raw
        .kind
        .as_ref()
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .trim()
        .to_lowercase();
    match kind.as_str() {
        "ytdlp" => {
            let prefix = raw
                .prefix
                .as_ref()
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .trim()
                .to_owned();
            if prefix.is_empty() {
                warnings.push(format!("skipping custom source '{name}': empty prefix"));
                return None;
            }
            let albums = raw
                .albums
                .as_ref()
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            Some(CustomSource {
                name,
                albums,
                kind: CustomSourceKind::Ytdlp { prefix },
            })
        }
        "command" => {
            let command = raw
                .command
                .as_ref()
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(str::to_owned))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            if command.is_empty() {
                warnings.push(format!("skipping custom source '{name}': empty command"));
                return None;
            }
            let timeout = raw
                .timeout_secs
                .as_ref()
                .and_then(|v| v.as_integer())
                .and_then(|n| u64::try_from(n).ok());
            Some(CustomSource {
                name,
                albums: false,
                kind: CustomSourceKind::Command {
                    command,
                    timeout_secs: normalize_command_timeout(timeout),
                },
            })
        }
        _ => {
            warnings.push(format!("skipping custom source '{name}': unknown type"));
            None
        }
    }
}
pub fn validate_raw_custom_sources(
    value: Option<&toml::Value>,
) -> (Vec<CustomSource>, Vec<String>) {
    let mut warnings = Vec::new();
    let Some(value) = value else {
        return (Vec::new(), warnings);
    };
    let Some(arr) = value.as_array() else {
        warnings.push("ignoring search.custom_sources: expected array".to_owned());
        return (Vec::new(), warnings);
    };
    let mut customs = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for entry in arr {
        if customs.len() >= MAX_CUSTOM_SOURCES {
            warnings.push(format!(
                "ignoring custom sources beyond {MAX_CUSTOM_SOURCES}"
            ));
            break;
        }
        let raw: RawCustomSource = match entry.clone().try_into() {
            Ok(raw) => raw,
            Err(_) => {
                warnings.push("skipping custom source with invalid shape".to_owned());
                continue;
            }
        };
        if let Some(custom) = validate_one_raw(&raw, &mut seen, &mut warnings) {
            customs.push(custom);
        }
    }
    (customs, warnings)
}
fn normalize_timeout_in_place(customs: &mut [CustomSource]) {
    for custom in customs {
        if let CustomSourceKind::Command { timeout_secs, .. } = &mut custom.kind {
            *timeout_secs = normalize_command_timeout(Some(*timeout_secs));
        }
    }
}
impl<'de> serde::Deserialize<'de> for SearchConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Helper {
            #[serde(default = "default_limit")]
            limit: u8,
            #[serde(default)]
            source: Option<toml::Value>,
            #[serde(default)]
            custom_sources: Option<toml::Value>,
        }
        let helper = Helper::deserialize(deserializer)?;
        let (customs, _) = validate_raw_custom_sources(helper.custom_sources.as_ref());
        let source_str = helper.source.as_ref().and_then(|v| v.as_str());
        Ok(Self {
            limit: helper.limit,
            source: resolve_source_name(source_str, &customs),
            custom_sources: customs,
        })
    }
}
impl serde::Serialize for SearchConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Serialize)]
        struct Helper<'a> {
            limit: u8,
            source: String,
            #[serde(skip_serializing_if = "Vec::is_empty")]
            custom_sources: &'a Vec<CustomSource>,
        }
        Helper {
            limit: self.limit,
            source: source_persist_name(self.source, &self.custom_sources),
            custom_sources: &self.custom_sources,
        }
        .serialize(serializer)
    }
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MpvConfig {
    #[serde(default = "default_socket")]
    pub socket: String,
}
impl Default for MpvConfig {
    fn default() -> Self {
        Self { socket: default_socket() }
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
pub fn load_config_with_diagnostics() -> (Config, Option<String>, Option<PathBuf>) {
    let paths = config_paths();
    let mut warning: Option<String> = None;
    for path in &paths {
        let raw = match fs::read_to_string(path) {
            Ok(raw) => raw,
            Err(_) => continue,
        };
        match toml::from_str::<Config>(&raw) {
            Ok(mut cfg) => {
                normalize_timeout_in_place(&mut cfg.search.custom_sources);
                let search_warnings = collect_search_warnings(&raw, &mut cfg);
                for w in &search_warnings {
                    log::warn!("config at {}: {w}", path.display());
                }
                let message = if search_warnings.is_empty() {
                    None
                } else {
                    Some(search_warnings.join("; "))
                };
                return (cfg, message, Some(path.clone()));
            }
            Err(err) => {
                log::warn!("failed to parse config at {}: {err}", path.display());
                if warning.is_none() {
                    warning = Some(format!(
                        "Config at {} is invalid, using defaults: {err}",
                        path.display()
                    ));
                }
            }
        }
    }
    (Config::default(), warning, None)
}
fn collect_search_warnings(raw: &str, cfg: &mut Config) -> Vec<String> {
    let value: toml::Value = match toml::from_str(raw) {
        Ok(value) => value,
        Err(_) => return Vec::new(),
    };
    let search = value.get("search");
    let customs_value = search.and_then(|s| s.get("custom_sources"));
    let (customs, mut warnings) = validate_raw_custom_sources(customs_value);
    cfg.search.custom_sources = customs.clone();
    let source_str = search
        .and_then(|s| s.get("source"))
        .and_then(|v| v.as_str());
    if let Some(name) = source_str {
        let normalized = normalize_source_name(name);
        let known = is_builtin_name(&normalized)
            || customs.iter().any(|c| source_names_equal(&c.name, name));
        if !known && !normalized.is_empty() {
            warnings.push(format!(
                "unknown search source '{name}', falling back to YouTube"
            ));
        }
        cfg.search.source = resolve_source_name(Some(name), &customs);
    }
    cfg.search.custom_sources = customs;
    if let SearchSource::Custom(i) = cfg.search.source {
        if cfg.search.custom_sources.get(i as usize).is_none() {
            warnings
                .push("saved search source no longer exists, falling back to YouTube".to_owned());
            cfg.search.source = SearchSource::YouTube;
        }
    }
    warnings
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
pub fn log_path() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".config/rs-pug/rs-pug.log")
    } else {
        PathBuf::from("rs-pug.log")
    }
}
fn default_true() -> bool {
    true
}
fn is_false(value: &bool) -> bool {
    !value
}
fn default_limit() -> u8 {
    20
}
fn default_socket() -> String {
    default_runtime_socket("rs-pug.sock")
}
pub fn default_ipc_socket() -> String {
    default_runtime_socket("rs-pug-ipc.sock")
}
fn default_runtime_socket(name: &str) -> String {
    if let Ok(dir) = std::env::var("XDG_RUNTIME_DIR") {
        if !dir.trim().is_empty() {
            return format!("{}/{name}", dir.trim_end_matches('/'));
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        if !home.trim().is_empty() {
            return format!("{home}/.config/rs-pug/{name}");
        }
    }
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("LOGNAME"))
        .unwrap_or_else(|_| "default".to_owned());
    let safe: String = user
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    format!("/tmp/rs-pug-{safe}/{name}")
}
pub fn ensure_socket_parent(path: &str) {
    let path = std::path::Path::new(path);
    if let Some(parent) = path.parent() {
        if parent.as_os_str().is_empty() {
            return;
        }
        let _ = fs::create_dir_all(parent);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if parent.to_string_lossy().starts_with("/tmp/rs-pug-") {
                let _ = fs::set_permissions(parent, fs::Permissions::from_mode(0o700));
            }
        }
    }
}
fn default_next_key() -> String {
    "n".to_string()
}
fn default_prev_key() -> String {
    "p".to_string()
}
fn default_mute_key() -> String {
    "m".to_string()
}
fn default_repeat_key() -> String {
    "r".to_string()
}
fn default_shuffle_key() -> String {
    "z".to_string()
}
fn default_seek_back_key() -> String {
    "[".to_string()
}
fn default_seek_forward_key() -> String {
    "]".to_string()
}
fn default_fft_toggle_key() -> String {
    "C-v".to_string()
}
fn default_minimal_toggle_key() -> String {
    "S-Z".to_string()
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
fn default_download_format() -> String {
    "mp3".to_owned()
}
pub fn normalize_download_format(raw: &str) -> String {
    match raw.trim().to_lowercase().as_str() {
        "best" | "m4a" | "opus" | "flac" | "mp3" => raw.trim().to_lowercase(),
        _ => "mp3".to_owned(),
    }
}
pub const DOWNLOAD_FORMATS: [&str; 5] = ["mp3", "best", "m4a", "opus", "flac"];
pub fn cycle_download_format(current: &str, delta: isize) -> String {
    let pos = DOWNLOAD_FORMATS
        .iter()
        .position(|f| *f == current.trim().to_lowercase().as_str())
        .unwrap_or(0) as isize;
    DOWNLOAD_FORMATS
        [(pos + delta).rem_euclid(DOWNLOAD_FORMATS.len() as isize) as usize]
        .to_owned()
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
        "dark".to_string(), "light".to_string(), "nord".to_string(), "gruvbox"
        .to_string(), "mono".to_string(),
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
        let path = PathBuf::from(&home_dir)
            .join(format!(".config/rs-pug/themes/{}.json", theme_name));
        if let Ok(raw) = fs::read_to_string(path) {
            if let Ok(pal) = serde_json::from_str::<Palette>(&raw) {
                return pal;
            }
        }
    }
    match theme {
        Theme::Light => {
            Palette {
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
                    [20, 120, 220], [40, 130, 210], [70, 120, 220], [110, 10, 210], [140,
                    60, 200], [0, 158, 210], [0, 158, 150], [0, 158, 88], [90, 170, 60],
                    [185, 128, 0], [210, 100, 30], [200, 50, 60],
                ],
            }
        }
        Theme::Nord => {
            Palette {
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
                    [94, 129, 172], [110, 145, 180], [129, 161, 193], [136, 192, 208],
                    [143, 188, 187], [163, 190, 140], [180, 195, 130], [235, 203, 139],
                    [208, 135, 112], [191, 97, 106], [180, 142, 173], [129, 161, 193],
                ],
            }
        }
        Theme::Gruvbox => {
            Palette {
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
                    [251, 73, 52], [254, 128, 25], [250, 189, 47], [184, 187, 38], [142,
                    192, 124], [104, 157, 106], [131, 165, 152], [69, 133, 136], [211,
                    134, 155], [214, 93, 14], [215, 153, 33], [204, 36, 29],
                ],
            }
        }
        Theme::Mono => {
            Palette {
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
            }
        }
        _ => {
            Palette {
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
            }
        }
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
        assert_eq!(sanitize_preset_filename("foo/bar\\baz").unwrap(), "foo_bar_baz");
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
    fn download_format_accepts_known_values_and_falls_back() {
        assert_eq!(normalize_download_format("mp3"), "mp3");
        assert_eq!(normalize_download_format(" BEST "), "best");
        assert_eq!(normalize_download_format("Flac"), "flac");
        assert_eq!(normalize_download_format("m4a"), "m4a");
        assert_eq!(normalize_download_format("opus"), "opus");
        assert_eq!(normalize_download_format("wav"), "mp3");
        assert_eq!(normalize_download_format(""), "mp3");
    }
    #[test]
    fn download_format_cycles_through_all_choices() {
        assert_eq!(cycle_download_format("mp3", 1), "best");
        assert_eq!(cycle_download_format("mp3", -1), "flac");
        assert_eq!(cycle_download_format("flac", 1), "mp3");
        assert_eq!(cycle_download_format("nonsense", 1), "best");
    }
    #[test]
    fn runtime_socket_prefers_xdg_dir() {
        let dir = tempfile::tempdir().unwrap();
        let key = "XDG_RUNTIME_DIR";
        let old = std::env::var(key).ok();
        unsafe {
            std::env::set_var(key, dir.path());
        }
        let sock = default_ipc_socket();
        assert_eq!(sock, format!("{}/rs-pug-ipc.sock", dir.path().display()));
        unsafe {
            match old {
                Some(v) => std::env::set_var(key, v),
                None => std::env::remove_var(key),
            }
        }
    }
    #[test]
    fn config_roundtrips_download_options() {
        let cfg = Config::default();
        let raw = toml::to_string_pretty(&cfg).expect("config must serialize");
        assert!(raw.contains("download_format"));
        let back: Config = toml::from_str(&raw).expect("config must parse");
        assert_eq!(back.general.download_format, "mp3");
        assert_eq!(back.general.download_dir, None);
    }
    #[test]
    fn custom_source_albums_opt_in_only_for_ytdlp() {
        let (customs, _) = custom_sources_from_toml(
            r#"
[search]
[[search.custom_sources]]
name = "PeerTube"
type = "ytdlp"
prefix = "peertube"
albums = true
[[search.custom_sources]]
name = "Script"
type = "command"
command = ["/bin/echo"]
albums = true
"#,
        );
        assert_eq!(customs.len(), 2);
        assert!(customs[0].albums);
        assert!(!customs[1].albums);
    }
    fn custom_sources_from_toml(toml: &str) -> (Vec<CustomSource>, Vec<String>) {
        let value: toml::Value = toml::from_str(toml).unwrap();
        validate_raw_custom_sources(value.get("search").and_then(|s| s.get("custom_sources")))
    }
    #[test]
    fn custom_source_names_match_unicode_case_insensitive_trimmed() {
        assert!(source_names_equal("  Привет ", "привет"));
        assert!(source_names_equal("YouTube (Newest)", "youtube (newest)"));
        assert!(!source_names_equal("a", "b"));
    }
    #[test]
    fn resolve_source_name_finds_builtins_and_customs() {
        let customs = vec![CustomSource {
            name: "My Script".to_owned(),
            albums: false,
            kind: CustomSourceKind::Ytdlp {
                prefix: "ytsearchdate".to_owned(),
            },
        }];
        assert_eq!(
            resolve_source_name(Some("youtube"), &customs),
            SearchSource::YouTube
        );
        assert_eq!(
            resolve_source_name(Some("  SOUNDCLOUD "), &customs),
            SearchSource::SoundCloud
        );
        assert_eq!(
            resolve_source_name(Some("my script"), &customs),
            SearchSource::Custom(0)
        );
        assert_eq!(
            resolve_source_name(Some("  MY SCRIPT "), &customs),
            SearchSource::Custom(0)
        );
        assert_eq!(
            resolve_source_name(Some("nope"), &customs),
            SearchSource::YouTube
        );
        assert_eq!(resolve_source_name(None, &customs), SearchSource::YouTube);
    }
    #[test]
    fn validate_custom_sources_skips_bad_entries() {
        let (customs, warnings) = custom_sources_from_toml(
            r#"
[search]
[[search.custom_sources]]
name = ""
type = "ytdlp"
prefix = "ytsearchdate"
[[search.custom_sources]]
name = "YouTube"
type = "ytdlp"
prefix = "x"
[[search.custom_sources]]
name = "dup"
type = "ytdlp"
prefix = "a"
[[search.custom_sources]]
name = "DUP"
type = "ytdlp"
prefix = "b"
[[search.custom_sources]]
name = "empty-prefix"
type = "ytdlp"
prefix = "  "
[[search.custom_sources]]
name = "empty-cmd"
type = "command"
command = []
[[search.custom_sources]]
name = "ok"
type = "command"
command = ["/bin/echo", "{query}"]
"#,
        );
        assert_eq!(customs.len(), 2);
        assert_eq!(customs[0].name, "dup");
        assert_eq!(customs[1].name, "ok");
        assert_eq!(warnings.len(), 5);
    }
    #[test]
    fn normalize_command_timeout_defaults_and_clamps() {
        assert_eq!(normalize_command_timeout(None), 15);
        assert_eq!(normalize_command_timeout(Some(0)), 15);
        assert_eq!(normalize_command_timeout(Some(5)), 5);
        assert_eq!(normalize_command_timeout(Some(500)), 120);
    }
    #[test]
    fn next_source_cycles_through_customs() {
        assert_eq!(
            next_source(SearchSource::YouTube, 0),
            SearchSource::SoundCloud
        );
        assert_eq!(next_source(SearchSource::Sonum, 0), SearchSource::YouTube);
        assert_eq!(next_source(SearchSource::Sonum, 2), SearchSource::Custom(0));
        assert_eq!(
            next_source(SearchSource::Custom(0), 2),
            SearchSource::Custom(1)
        );
        assert_eq!(
            next_source(SearchSource::Custom(1), 2),
            SearchSource::YouTube
        );
        assert_eq!(
            next_source(SearchSource::Custom(9), 2),
            SearchSource::YouTube
        );
    }
    #[test]
    fn source_persist_name_uses_declared_name() {
        let customs = vec![CustomSource {
            name: "My Script".to_owned(),
            albums: false,
            kind: CustomSourceKind::Ytdlp {
                prefix: "x".to_owned(),
            },
        }];
        assert_eq!(
            source_persist_name(SearchSource::Custom(0), &customs),
            "My Script"
        );
        assert_eq!(
            source_persist_name(SearchSource::Custom(9), &customs),
            "youtube"
        );
        assert_eq!(
            source_persist_name(SearchSource::SoundCloud, &customs),
            "soundcloud"
        );
    }
    #[test]
    fn search_config_round_trips_custom_source() {
        let raw = r#"
[search]
source = "My Script"
[[search.custom_sources]]
name = "My Script"
type = "ytdlp"
prefix = "ytsearchdate"
"#;
        let cfg: Config = toml::from_str(raw).unwrap();
        assert_eq!(cfg.search.source, SearchSource::Custom(0));
        let serialized = toml::to_string(&cfg.search).unwrap();
        assert!(serialized.contains("My Script"));
        let reparsed: SearchConfig = toml::from_str(&serialized).unwrap();
        assert_eq!(reparsed.source, SearchSource::Custom(0));
    }
    #[test]
    fn save_eq_preset_does_not_escape_presets_dir() {        let dir = tempfile::tempdir().unwrap();
        unsafe {
            std::env::set_var("HOME", dir.path());
        }
        let preset = EqPreset {
            name: "../../evil".to_string(),
            bands: [0.0; 10],
        };
        let result = save_eq_preset(&preset);
        assert!(result.is_err());
        let escaped = dir.path().join("evil.json");
        assert!(! escaped.exists());
        let presets_dir = dir.path().join(".config/rs-pug/eqpresets");
        if presets_dir.exists() {
            assert_eq!(fs::read_dir(presets_dir).unwrap().count(), 0);
        }
    }
}

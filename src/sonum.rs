use crate::model::{Album, Song};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf, time::Duration};

/// pulls in the Sonum client config from sonumclient.toml
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SonumConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub api_token: Option<String>,
}

impl Default for SonumConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            api_token: None,
        }
    }
}

impl SonumConfig {
    pub fn base_url(&self) -> String {
        format!("http://{}:{}", self.host, self.port)
    }
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    8420
}

pub fn sonum_config_path() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".config/rs-pug/sonumclient.toml")
    } else {
        PathBuf::from("sonumclient.toml")
    }
}

fn default_conf_contents() -> String {
    format!(
        r#"# Sonum client config for rs-pug.
# Sonum is a local HTTP/JSON server hosting your music library. Once configured,
# switch "Search source" to "Sonum" under Options (h/l keys) so search and
# playback hit this server.
# Restart rs-pug after editing for changes to take effect.

# Host / IP address of the Sonum server (e.g. "127.0.0.1" or "192.168.1.50").
host = "{}"

# Port the Sonum server is listening on.
port = {}

# Optional Bearer token if your Sonum server requires auth
# (see `api_token` in server's sonum.conf).
# api_token = "your-secret-token"
"#,
        default_host(),
        default_port()
    )
}

pub fn ensure_sonum_config() {
    let path = sonum_config_path();
    if path.exists() {
        return;
    }
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(&path, default_conf_contents());
}

/// reads sonumclient.toml defaults to localhost:8420 (no token) if missing or bad
pub fn load_sonum_config() -> SonumConfig {
    let path = sonum_config_path();
    match fs::read_to_string(&path) {
        Ok(raw) => toml::from_str(&raw).unwrap_or_else(|err| {
            log::warn!("failed to parse {}: {err}", path.display());
            SonumConfig::default()
        }),
        Err(_) => SonumConfig::default(),
    }
}

#[derive(Debug, Deserialize)]
struct SonumTrackDto {
    id: String,
    title: String,
    artist: String,
    #[serde(default)]
    album: String,
    #[serde(default)]
    duration_seconds: Option<u64>,
    stream_url: String,
}

fn fetch_tracks(config: &SonumConfig, query: &str, limit: u8) -> Result<Vec<SonumTrackDto>> {
    let url = format!("{}/tracks", config.base_url());
    let mut request = ureq::get(&url)
        .query("limit", &limit.to_string())
        .timeout(Duration::from_secs(8));
    if !query.trim().is_empty() {
        request = request.query("q", query);
    }
    if let Some(token) = &config.api_token {
        request = request.set("Authorization", &format!("Bearer {token}"));
    }
    let response = request.call().map_err(|err| {
        anyhow::anyhow!(
            "failed to reach Sonum server at {} (check host/port in {}): {err}",
            config.base_url(),
            sonum_config_path().display()
        )
    })?;
    response
        .into_json::<Vec<SonumTrackDto>>()
        .context("invalid JSON response from Sonum server")
}

fn track_to_song(config: &SonumConfig, track: SonumTrackDto) -> Song {
    Song {
        id: track.id,
        title: track.title,
        webpage_url: format!("{}{}", config.base_url(), track.stream_url),
        uploader: Some(track.artist),
        duration: track.duration_seconds.map(|d| d as f64),
    }
}

pub async fn search_songs(limit: u8, query: String) -> Result<Vec<Song>> {
    tokio::task::spawn_blocking(move || {
        let config = load_sonum_config();
        let tracks = fetch_tracks(&config, &query, limit)?;
        Ok(tracks
            .into_iter()
            .map(|t| track_to_song(&config, t))
            .collect())
    })
    .await
    .context("Sonum search task failed")?
}

pub async fn search_albums(limit: u8, query: String) -> Result<Vec<Album>> {
    tokio::task::spawn_blocking(move || {
        let config = load_sonum_config();
        let tracks = fetch_tracks(&config, &query, limit)?;
        let mut albums: Vec<Album> = Vec::new();
        for track in tracks {
            let album_name = if track.album.trim().is_empty() {
                "Unknown Album".to_string()
            } else {
                track.album.clone()
            };
            let artist = track.artist.clone();
            let song = track_to_song(&config, track);
            if let Some(existing) = albums
                .iter_mut()
                .find(|a| a.name == album_name && a.artist == artist)
            {
                existing.songs.push(song);
            } else {
                albums.push(Album {
                    name: album_name,
                    artist,
                    songs: vec![song],
                });
            }
        }
        albums.retain(|a| a.songs.len() > 1);
        Ok(albums)
    })
    .await
    .context("Sonum album search task failed")?
}

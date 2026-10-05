use crate::model::{Album, Song};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf, time::Duration};

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
    duration_seconds: Option<u64>,
    stream_url: String,
    #[serde(default)]
    track_number: Option<u32>,
    #[serde(default)]
    disc_number: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct SonumAlbumDto {
    name: String,
    artist: String,
    #[serde(default)]
    track_ids: Vec<String>,
}

// sonum sorts an albums track ids by id string, so order the fetched tracks here
fn sort_album_tracks(tracks: &mut [SonumTrackDto]) {
    tracks.sort_by(|a, b| {
        (
            a.disc_number.unwrap_or(u32::MAX),
            a.track_number.unwrap_or(u32::MAX),
        )
            .cmp(&(
                b.disc_number.unwrap_or(u32::MAX),
                b.track_number.unwrap_or(u32::MAX),
            ))
            .then_with(|| a.title.cmp(&b.title))
    });
}

fn fetch_tracks(config: &SonumConfig, query: &str, limit: u8) -> Result<Vec<SonumTrackDto>> {
    let url = format!("{}/tracks", config.base_url());
    let mut request = ureq::get(&url)
        .query("limit", limit.to_string())
        .config()
        .timeout_global(Some(Duration::from_secs(8)))
        .build();
    if !query.trim().is_empty() {
        request = request.query("q", query);
    }
    if let Some(token) = &config.api_token {
        request = request.header("Authorization", &format!("Bearer {token}"));
    }
    let mut response = request.call().map_err(|err| {
        anyhow::anyhow!(
            "failed to reach Sonum server at {} (check host/port in {}): {err}",
            config.base_url(),
            sonum_config_path().display()
        )
    })?;
    response
        .body_mut()
        .read_json::<Vec<SonumTrackDto>>()
        .context("invalid JSON response from Sonum server")
}

fn fetch_albums(config: &SonumConfig, query: &str, limit: u8) -> Result<Vec<SonumAlbumDto>> {
    let url = format!("{}/albums", config.base_url());
    let mut request = ureq::get(&url)
        .query("limit", limit.to_string())
        .config()
        .timeout_global(Some(Duration::from_secs(8)))
        .build();
    if !query.trim().is_empty() {
        request = request.query("q", query);
    }
    if let Some(token) = &config.api_token {
        request = request.header("Authorization", &format!("Bearer {token}"));
    }
    let mut response = request.call().map_err(|err| {
        anyhow::anyhow!(
            "failed to reach Sonum server at {} (check host/port in {}): {err}",
            config.base_url(),
            sonum_config_path().display()
        )
    })?;
    response
        .body_mut()
        .read_json::<Vec<SonumAlbumDto>>()
        .context("invalid JSON response from Sonum server (/albums)")
}

fn fetch_track_by_id(config: &SonumConfig, id: &str) -> Result<SonumTrackDto> {
    let url = format!("{}/tracks/{id}", config.base_url());
    let mut request = ureq::get(&url)
        .config()
        .timeout_global(Some(Duration::from_secs(8)))
        .build();
    if let Some(token) = &config.api_token {
        request = request.header("Authorization", &format!("Bearer {token}"));
    }
    let mut response = request
        .call()
        .map_err(|err| anyhow::anyhow!("failed to fetch Sonum track {id}: {err}"))?;
    response
        .body_mut()
        .read_json::<SonumTrackDto>()
        .with_context(|| format!("invalid JSON response from Sonum server (/tracks/{id})"))
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
        let summaries = fetch_albums(&config, &query, limit)?;
        let mut albums: Vec<Album> = Vec::new();
        for summary in summaries {
            let mut tracks = Vec::new();
            for id in &summary.track_ids {
                match fetch_track_by_id(&config, id) {
                    Ok(track) => tracks.push(track),
                    Err(err) => log::warn!("skipping Sonum track {id}: {err:#}"),
                }
            }
            if tracks.is_empty() {
                continue;
            }
            sort_album_tracks(&mut tracks);
            albums.push(Album {
                name: summary.name,
                artist: summary.artist,
                songs: tracks
                    .into_iter()
                    .map(|t| track_to_song(&config, t))
                    .collect(),
                playlist_url: None,
            });
        }
        Ok(albums)
    })
    .await
    .context("Sonum album search task failed")?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn album_track(id: &str, title: &str, disc: Option<u32>, number: Option<u32>) -> SonumTrackDto {
        SonumTrackDto {
            id: id.to_owned(),
            title: title.to_owned(),
            artist: "Night Drive".to_owned(),
            duration_seconds: Some(180),
            stream_url: format!("/tracks/{id}/stream"),
            track_number: number,
            disc_number: disc,
        }
    }

    #[test]
    fn album_tracks_sort_by_disc_then_number() {
        let mut tracks = vec![
            album_track("c", "Outro", Some(1), Some(9)),
            album_track("a", "Intro", Some(1), Some(1)),
            album_track("b", "Interlude", Some(2), Some(1)),
            album_track("d", "Untagged", None, None),
        ];
        sort_album_tracks(&mut tracks);
        let order: Vec<&str> = tracks.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(order, vec!["a", "c", "b", "d"]);
    }
}

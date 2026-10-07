use crate::model::{Album, Song};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{fs, io::Read, path::PathBuf, time::Duration};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SonumServer {
    #[serde(default)]
    pub name: String,
    #[serde(default = "default_scheme")]
    pub scheme: String,
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub api_token: Option<String>,
}

impl SonumServer {
    pub fn base_url(&self) -> String {
        format!(
            "{}://{}:{}",
            normalize_scheme(&self.scheme),
            self.host,
            self.port
        )
    }
    pub fn display_name(&self) -> String {
        if self.name.trim().is_empty() {
            format!("{}:{}", self.host, self.port)
        } else {
            self.name.clone()
        }
    }
    pub fn is_loopback_host(&self) -> bool {
        matches!(self.host.as_str(), "127.0.0.1" | "::1" | "localhost")
    }
    pub fn token_over_plaintext(&self) -> bool {
        self.api_token
            .as_ref()
            .is_some_and(|t| !t.trim().is_empty())
            && normalize_scheme(&self.scheme) == "http"
            && !self.is_loopback_host()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SonumConfig {
    #[serde(default = "default_scheme")]
    pub scheme: String,
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub api_token: Option<String>,
    #[serde(default)]
    pub servers: Vec<SonumServer>,
}

impl Default for SonumConfig {
    fn default() -> Self {
        Self {
            scheme: default_scheme(),
            host: default_host(),
            port: default_port(),
            api_token: None,
            servers: Vec::new(),
        }
    }
}

impl SonumConfig {
    pub fn primary(&self) -> SonumServer {
        SonumServer {
            name: String::new(),
            scheme: self.scheme.clone(),
            host: self.host.clone(),
            port: self.port,
            api_token: self.api_token.clone(),
        }
    }
    pub fn all_servers(&self) -> Vec<SonumServer> {
        let mut servers = vec![self.primary()];
        servers.extend(self.servers.iter().cloned());
        servers
    }
}

fn default_scheme() -> String {
    "http".to_string()
}

pub fn normalize_scheme(raw: &str) -> String {
    match raw.trim().to_lowercase().as_str() {
        "https" => "https".to_owned(),
        _ => "http".to_owned(),
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
# Run `chmod 600 ~/.config/rs-pug/sonumclient.toml` to restrict this file,
# it can contain your api_token.

# URL scheme for the Sonum server ("http" or "https").
scheme = "{}"

# Host / IP address of the Sonum server (e.g. "127.0.0.1" or "192.168.1.50").
host = "{}"

# Port the Sonum server is listening on.
port = {}

# Optional Bearer token if your Sonum server requires auth
# (see `api_token` in server's sonum.conf).
# api_token = "your-secret-token"

# Additional Sonum servers. Searching queries every server and merges
# the results; playback, covers and downloads use each song's own server
# (matched by URL) with its own token.
# [[servers]]
# name = "NAS"
# host = "192.168.1.50"
# port = 8420
# scheme = "http"
# api_token = "other-secret-token"
"#,
        default_scheme(),
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
    #[cfg(unix)]
    {
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        if let Ok(file) = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
        {
            use std::io::Write;
            let mut file = file;
            let _ = file.write_all(default_conf_contents().as_bytes());
            let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
            return;
        }
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

fn fetch_tracks(server: &SonumServer, query: &str, limit: u8) -> Result<Vec<SonumTrackDto>> {
    let url = format!("{}/tracks", server.base_url());
    let mut request = ureq::get(&url)
        .query("limit", limit.to_string())
        .config()
        .timeout_global(Some(Duration::from_secs(8)))
        .build();
    if !query.trim().is_empty() {
        request = request.query("q", query);
    }
    if let Some(token) = &server.api_token {
        request = request.header("Authorization", &format!("Bearer {token}"));
    }
    let mut response = request.call().map_err(|err| {
        anyhow::anyhow!(
            "failed to reach Sonum server '{}' at {} (check host/port in {}): {err}",
            server.display_name(),
            server.base_url(),
            sonum_config_path().display()
        )
    })?;
    response
        .body_mut()
        .read_json::<Vec<SonumTrackDto>>()
        .context("invalid JSON response from Sonum server")
}

fn fetch_albums(server: &SonumServer, query: &str, limit: u8) -> Result<Vec<SonumAlbumDto>> {
    let url = format!("{}/albums", server.base_url());
    let mut request = ureq::get(&url)
        .query("limit", limit.to_string())
        .config()
        .timeout_global(Some(Duration::from_secs(8)))
        .build();
    if !query.trim().is_empty() {
        request = request.query("q", query);
    }
    if let Some(token) = &server.api_token {
        request = request.header("Authorization", &format!("Bearer {token}"));
    }
    let mut response = request.call().map_err(|err| {
        anyhow::anyhow!(
            "failed to reach Sonum server '{}' at {} (check host/port in {}): {err}",
            server.display_name(),
            server.base_url(),
            sonum_config_path().display()
        )
    })?;
    response
        .body_mut()
        .read_json::<Vec<SonumAlbumDto>>()
        .context("invalid JSON response from Sonum server (/albums)")
}

fn fetch_track_by_id(server: &SonumServer, id: &str) -> Result<SonumTrackDto> {
    let url = format!("{}/tracks/{id}", server.base_url());
    let mut request = ureq::get(&url)
        .config()
        .timeout_global(Some(Duration::from_secs(8)))
        .build();
    if let Some(token) = &server.api_token {
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

fn track_to_song(server: &SonumServer, track: SonumTrackDto) -> Song {
    Song {
        id: track.id,
        title: track.title,
        webpage_url: format!("{}{}", server.base_url(), track.stream_url),
        uploader: Some(track.artist),
        duration: track.duration_seconds.map(|d| d as f64),
    }
}

pub fn sonum_config_for_url(url: &str) -> Option<SonumServer> {
    load_sonum_config()
        .all_servers()
        .into_iter()
        .find(|server| {
            let base = server.base_url();
            url == base || url.starts_with(&format!("{base}/"))
        })
}

pub fn sanitize_download_filename(title: &str) -> String {
    let cleaned: String = title
        .chars()
        .map(|c| match c {
            '/' | '\\' | '\0' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        "track".to_owned()
    } else {
        trimmed.to_owned()
    }
}

pub fn stream_extension(url: &str) -> String {
    let path = url.split(['?', '#']).next().unwrap_or(url);
    let ext = path.rsplit('.').next().unwrap_or("");
    if ext.is_empty() || ext.contains('/') || ext.len() > 8 {
        return "mp3".to_owned();
    }
    ext.to_lowercase()
}

const SONUM_DOWNLOAD_LIMIT_BYTES: usize = 300 * 1024 * 1024;

pub fn fetch_sonum_bytes(url: &str) -> Result<Vec<u8>> {
    let server = sonum_config_for_url(url)
        .ok_or_else(|| anyhow::anyhow!("Sonum download failed: unknown server for {url}"))?;
    let mut request = ureq::get(url)
        .config()
        .timeout_global(Some(Duration::from_secs(300)))
        .build();
    if let Some(token) = &server.api_token {
        if !token.trim().is_empty() {
            request = request.header("Authorization", &format!("Bearer {token}"));
        }
    }
    let mut response = request
        .call()
        .map_err(|err| anyhow::anyhow!("Sonum download failed: {err}"))?;
    let body = response.body_mut().as_reader();
    let mut buf = Vec::new();
    body.take(SONUM_DOWNLOAD_LIMIT_BYTES as u64 + 1)
        .read_to_end(&mut buf)
        .context("failed to read Sonum download")?;
    if buf.len() > SONUM_DOWNLOAD_LIMIT_BYTES {
        anyhow::bail!("Sonum download exceeds size limit");
    }
    if buf.is_empty() {
        anyhow::bail!("Sonum download returned no data");
    }
    Ok(buf)
}
pub async fn search_songs(limit: u8, query: String) -> Result<Vec<Song>> {
    tokio::task::spawn_blocking(move || {
        let mut songs = Vec::new();
        for server in load_sonum_config().all_servers() {
            match fetch_tracks(&server, &query, limit) {
                Ok(tracks) => songs.extend(tracks.into_iter().map(|t| track_to_song(&server, t))),
                Err(err) => {
                    log::warn!("skipping Sonum server '{}': {err:#}", server.display_name())
                }
            }
        }
        Ok(songs)
    })
    .await
    .context("Sonum search task failed")?
}

pub async fn search_albums(limit: u8, query: String) -> Result<Vec<Album>> {
    tokio::task::spawn_blocking(move || {
        let mut albums: Vec<Album> = Vec::new();
        for server in load_sonum_config().all_servers() {
            let summaries = match fetch_albums(&server, &query, limit) {
                Ok(summaries) => summaries,
                Err(err) => {
                    log::warn!("skipping Sonum server '{}': {err:#}", server.display_name());
                    continue;
                }
            };
            for summary in summaries {
                let mut tracks = Vec::new();
                for id in &summary.track_ids {
                    match fetch_track_by_id(&server, id) {
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
                        .map(|t| track_to_song(&server, t))
                        .collect(),
                    playlist_url: None,
                });
            }
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
    #[test]
    fn base_url_uses_scheme_and_warns_on_plaintext_token() {
        let plain = SonumConfig {
            scheme: "http".to_owned(),
            host: "192.168.1.50".to_owned(),
            port: 8420,
            api_token: Some("secret".to_owned()),
            servers: Vec::new(),
        };
        assert_eq!(plain.primary().base_url(), "http://192.168.1.50:8420");
        assert!(plain.primary().token_over_plaintext());
        let tls = SonumConfig {
            scheme: "https".to_owned(),
            ..plain.clone()
        };
        assert_eq!(tls.primary().base_url(), "https://192.168.1.50:8420");
        assert!(!tls.primary().token_over_plaintext());
        let local = SonumConfig {
            scheme: "http".to_owned(),
            host: "127.0.0.1".to_owned(),
            port: 8420,
            api_token: Some("secret".to_owned()),
            servers: Vec::new(),
        };
        assert!(!local.primary().token_over_plaintext());
        assert_eq!(normalize_scheme("HTTPS"), "https");
        assert_eq!(normalize_scheme("gopher"), "http");
    }
    #[test]
    fn extra_servers_parse_and_match_by_url() {
        let config: SonumConfig = toml::from_str(
            r#"
host = "127.0.0.1"
port = 8420
[[servers]]
name = "NAS"
host = "192.168.1.50"
port = 8420
scheme = "https"
api_token = "nas-token"
[[servers]]
host = "192.168.1.60"
port = 9000
"#,
        )
        .unwrap();
        let servers = config.all_servers();
        assert_eq!(servers.len(), 3);
        assert_eq!(servers[0].base_url(), "http://127.0.0.1:8420");
        assert_eq!(servers[1].display_name(), "NAS");
        assert_eq!(servers[1].base_url(), "https://192.168.1.50:8420");
        assert_eq!(servers[2].display_name(), "192.168.1.60:9000");
        assert!(
            servers.iter().all(|s| !s.token_over_plaintext()),
            "no server sends a token over plaintext http"
        );
        let tls_only: SonumConfig = toml::from_str(
            r#"
scheme = "https"
host = "192.168.1.50"
port = 8420
api_token = "secret"
"#,
        )
        .unwrap();
        assert!(
            tls_only
                .all_servers()
                .iter()
                .all(|s| !s.token_over_plaintext())
        );
    }
    #[test]
    fn stream_extension_and_filename_are_safe() {
        assert_eq!(stream_extension("http://x/tracks/1/stream?token=a"), "mp3");
        assert_eq!(stream_extension("http://x/song.flac"), "flac");
        assert_eq!(sanitize_download_filename("a/b\\c"), "a_b_c");
        assert_eq!(sanitize_download_filename("   "), "track");
    }
}

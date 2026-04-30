use std::{collections::VecDeque, process::Stdio, time::Duration};

use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::UnixStream,
    process::{Child, Command},
    sync::mpsc,
    time,
};

use crate::{config::Config, model::{Song, LocalSong}, plugins::PluginManager};
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::tag::Accessor;

#[derive(Debug)]
pub enum CoreCmd {
    Search(String),
    SearchAlbums(String),
    Play(Song),
    SmartQueue(Song),
    TogglePause,
    VolumeUp,
    VolumeDown,
    SetVolume(u8),
    SeekBy(i32),
    PlayUrl { url: String, title: Option<String> },
    RawMpv(Value),
    ToggleMute,
    Next,
    Prev,
    Quit,
}

#[derive(Debug)]
pub enum CoreEvent {
    SearchDone(Vec<Song>),
    AlbumSearchDone(Vec<Song>),
    SearchFailed(String),
    AlbumSearchFailed(String),
    Started(Song),
    Paused,
    Resumed,
    TrackFinished,
    Progress { position: f64, duration: f64 },
    VolumeChanged(u8),
    MuteChanged(bool),
    Error(String),
}

pub struct Core {
    config: Config,
    mpv_child: Child,
    mpris_child: Option<Child>,
    history: VecDeque<Song>,
    volume: u8,
    muted: bool,
    was_playing: bool,
    plugins: PluginManager,
}

impl Core {
    pub async fn new(config: Config) -> Result<Self> {
        let mpv_child = Command::new("mpv")
            .arg("--idle")
            .arg("--no-video")
            .arg("--profile=high-quality")
            .arg("--audio-display=no")
            .arg("--volume=70")
            .arg(format!("--input-ipc-server={}", config.mpv.socket))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|err| anyhow::anyhow!("failed to start mpv (is `mpv` installed?): {err}"))?;

        tokio::time::sleep(Duration::from_millis(180)).await;

        let mut core = Self {
            plugins: PluginManager::load(
                config.general.plugins_enabled,
                config.general.plugins_dir.as_str(),
            ),
            config,
            mpv_child,
            mpris_child: None,
            history: VecDeque::new(),
            volume: 70,
            muted: false,
            was_playing: false,
        };
        let _ = core.plugins.plugin_count();
        core.try_start_mpris();
        Ok(core)
    }

    pub async fn run(
        mut self,
        mut rx: mpsc::UnboundedReceiver<CoreCmd>,
        tx: mpsc::UnboundedSender<CoreEvent>,
    ) {
        let mut tick = time::interval(Duration::from_millis(700));
        loop {
            tokio::select! {
                _ = tick.tick() => {
                    if let Err(err) = self.poll_playback_finished(&tx).await {
                        let _ = tx.send(CoreEvent::Error(format!("{err:#}")));
                    }
                }
                maybe_cmd = rx.recv() => {
                    let Some(cmd) = maybe_cmd else { break };
                    let res = match cmd {
                        CoreCmd::Search(query) => {
                            let limit = self.config.search.limit.max(1);
                            let query = self.plugins.transform_search_query(query);
                            match search_songs(limit, query).await {
                                Ok(songs) => {
                                    let songs = self.plugins.transform_search_results(songs);
                                    let _ = tx.send(CoreEvent::SearchDone(songs));
                                }
                                Err(err) => {
                                    let _ = tx.send(CoreEvent::SearchFailed(format!("{err:#}")));
                                }
                            }
                            Ok(())
                        }
                        CoreCmd::SearchAlbums(query) => {
                            let limit = self.config.search.limit.max(1);
                            let query = self.plugins.transform_search_query(query);
                            match search_albums(limit, query).await {
                                Ok(songs) => {
                                    let songs = self.plugins.transform_search_results(songs);
                                    let _ = tx.send(CoreEvent::AlbumSearchDone(songs));
                                }
                                Err(err) => {
                                    let _ = tx.send(CoreEvent::AlbumSearchFailed(format!("{err:#}")));
                                }
                            }
                            Ok(())
                        }
                        CoreCmd::Play(song) => self.play(song, &tx).await,
                        CoreCmd::SmartQueue(song) => self.smart_queue(song, &tx).await,
                        CoreCmd::TogglePause => self.toggle_pause(&tx).await,
                        CoreCmd::VolumeUp => self.change_volume(5, &tx).await,
                        CoreCmd::VolumeDown => self.change_volume(-5, &tx).await,
                        CoreCmd::SetVolume(value) => self.set_volume(value, &tx).await,
                        CoreCmd::SeekBy(seconds) => self.seek_by(seconds, &tx).await,
                        CoreCmd::PlayUrl { url, title } => {
                            let song = Song {
                                id: url.clone(),
                                title: title.unwrap_or_else(|| url.clone()),
                                webpage_url: url,
                                uploader: None,
                                duration: None,
                            };
                            self.play(song, &tx).await
                        }
                        CoreCmd::RawMpv(command) => {
                            self.send_mpv(json!({"command": command})).await
                        }
                        CoreCmd::ToggleMute => self.toggle_mute(&tx).await,
                        CoreCmd::Next => self.next(&tx).await,
                        CoreCmd::Prev => self.prev(&tx).await,
                        CoreCmd::Quit => break,
                    };

                    if let Err(err) = res {
                        let _ = tx.send(CoreEvent::Error(format!("{err:#}")));
                    }
                }
            }
        }

        let _ = self.stop_mpris().await;
        let _ = self.mpv_child.kill().await;
    }

    async fn play(&mut self, song: Song, tx: &mpsc::UnboundedSender<CoreEvent>) -> Result<()> {
        let song = self.plugins.transform_song_start(song);
        self.send_mpv(json!({"command": ["loadfile", song.webpage_url, "replace"]}))
            .await?;
        self.history.push_front(song.clone());
        if self.history.len() > 128 {
            self.history.pop_back();
        }
        self.was_playing = true;
        let _ = tx.send(CoreEvent::Started(song));
        Ok(())
    }

    async fn smart_queue(
        &mut self,
        current: Song,
        tx: &mpsc::UnboundedSender<CoreEvent>,
    ) -> Result<()> {
        let query = current
            .uploader
            .as_ref()
            .map(|u| format!("{u} {}", current.title))
            .unwrap_or_else(|| current.title.clone());
        let candidates = search_songs(8, query).await?;
        let maybe_next = candidates
            .into_iter()
            .find(|song| song.id != current.id && song.title != current.title);

        if let Some(next_song) = maybe_next {
            self.play(next_song, tx).await?;
            Ok(())
        } else {
            anyhow::bail!("smart queue: no similar song found")
        }
    }

    async fn toggle_pause(&self, tx: &mpsc::UnboundedSender<CoreEvent>) -> Result<()> {
        self.send_mpv(json!({"command": ["cycle", "pause"]}))
            .await?;
        let _ = tx.send(CoreEvent::Paused);
        Ok(())
    }

    async fn next(&self, tx: &mpsc::UnboundedSender<CoreEvent>) -> Result<()> {
        self.send_mpv(json!({"command": ["playlist-next", "force"]}))
            .await?;
        let _ = tx.send(CoreEvent::Resumed);
        Ok(())
    }

    async fn prev(&self, tx: &mpsc::UnboundedSender<CoreEvent>) -> Result<()> {
        self.send_mpv(json!({"command": ["playlist-prev", "force"]}))
            .await?;
        let _ = tx.send(CoreEvent::Resumed);
        Ok(())
    }

    async fn stop_mpris(&mut self) -> Result<()> {
        if let Some(mut child) = self.mpris_child.take() {
            let _ = child.kill().await;
        }
        Ok(())
    }

    async fn send_mpv(&self, message: Value) -> Result<()> {
        let mut stream = UnixStream::connect(self.config.mpv.socket.as_str())
            .await
            .context("failed to connect to mpv ipc socket")?;
        let mut payload = serde_json::to_vec(&message)?;
        payload.push(b'\n');
        stream.write_all(&payload).await?;
        Ok(())
    }

    async fn change_volume(
        &mut self,
        delta: i8,
        tx: &mpsc::UnboundedSender<CoreEvent>,
    ) -> Result<()> {
        let next = (self.volume as i16 + delta as i16).clamp(0, 130) as u8;
        self.send_mpv(json!({"command": ["set_property", "volume", next]}))
            .await?;
        self.volume = next;
        let _ = tx.send(CoreEvent::VolumeChanged(next));
        Ok(())
    }

    async fn set_volume(&mut self, value: u8, tx: &mpsc::UnboundedSender<CoreEvent>) -> Result<()> {
        let next = value.min(130);
        self.send_mpv(json!({"command": ["set_property", "volume", next]}))
            .await?;
        self.volume = next;
        let _ = tx.send(CoreEvent::VolumeChanged(next));
        Ok(())
    }

    async fn seek_by(&self, seconds: i32, tx: &mpsc::UnboundedSender<CoreEvent>) -> Result<()> {
        self.send_mpv(json!({"command": ["seek", seconds, "relative"]}))
            .await?;
        let _ = tx.send(CoreEvent::Resumed);
        Ok(())
    }

    async fn toggle_mute(&mut self, tx: &mpsc::UnboundedSender<CoreEvent>) -> Result<()> {
        self.send_mpv(json!({"command": ["cycle", "mute"]})).await?;
        self.muted = !self.muted;
        let _ = tx.send(CoreEvent::MuteChanged(self.muted));
        Ok(())
    }

    async fn poll_playback_finished(
        &mut self,
        tx: &mpsc::UnboundedSender<CoreEvent>,
    ) -> Result<()> {
        let is_playing = !self.read_mpv_bool_property("idle-active").await?;
        if self.was_playing && !is_playing {
            let _ = tx.send(CoreEvent::TrackFinished);
        }
        if is_playing {
            let position = self.read_mpv_number_property("time-pos").await?;
            let duration = self.read_mpv_number_property("duration").await?;
            if let Some(position) = position {
                let _ = tx.send(CoreEvent::Progress {
                    position,
                    duration: duration.unwrap_or(0.0),
                });
            }
        }
        self.was_playing = is_playing;
        Ok(())
    }

    fn try_start_mpris(&mut self) {
        if !self.config.general.mpris_enabled {
            return;
        }

        for (bin, args) in self.mpris_candidates() {
            match Command::new(&bin)
                .args(args)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
            {
                Ok(c) => {
                    self.mpris_child = Some(c);
                    break;
                }
                Err(_) => continue,
            }
        }
    }

    fn mpris_candidates(&self) -> Vec<(String, Vec<String>)> {
        if let Some(cmd) = self
            .config
            .general
            .mpris_command
            .as_ref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
        {
            return vec![parse_command(cmd)];
        }

        vec![
            (
                "mpv-mpris".to_owned(),
                vec!["--socket".to_owned(), self.config.mpv.socket.clone()],
            ),
            (
                "mpv-mpris".to_owned(),
                vec!["--mpv-socket".to_owned(), self.config.mpv.socket.clone()],
            ),
            ("mpv-mpris".to_owned(), vec![self.config.mpv.socket.clone()]),
            ("mpv-mpris".to_owned(), vec![]),
        ]
    }
}


fn parse_command(raw: &str) -> (String, Vec<String>) {
    let mut parts = raw.split_whitespace();
    let bin = parts.next().unwrap_or("mpv-mpris").to_owned();
    let args = parts.map(|s| s.to_owned()).collect();
    (bin, args)
}

#[derive(Debug, Deserialize)]
struct MpvBoolResponse {
    data: bool,
}

#[derive(Debug, Deserialize)]
struct MpvNumberResponse {
    data: Option<f64>,
}

impl Core {
    async fn read_mpv_bool_property(&self, property: &str) -> Result<bool> {
        let mut stream = UnixStream::connect(self.config.mpv.socket.as_str())
            .await
            .context("failed to connect to mpv ipc socket")?;
        let mut payload = serde_json::to_vec(&json!({"command": ["get_property", property]}))?;
        payload.push(b'\n');
        stream.write_all(&payload).await?;

        let mut line = String::new();
        let mut reader = BufReader::new(stream);
        reader.read_line(&mut line).await?;
        let parsed: MpvBoolResponse =
            serde_json::from_str(line.trim()).context("failed to parse mpv bool response")?;
        Ok(parsed.data)
    }

    async fn read_mpv_number_property(&self, property: &str) -> Result<Option<f64>> {
        let mut stream = UnixStream::connect(self.config.mpv.socket.as_str())
            .await
            .context("failed to connect to mpv ipc socket")?;
        let mut payload = serde_json::to_vec(&json!({"command": ["get_property", property]}))?;
        payload.push(b'\n');
        stream.write_all(&payload).await?;

        let mut line = String::new();
        let mut reader = BufReader::new(stream);
        reader.read_line(&mut line).await?;
        let parsed: MpvNumberResponse =
            serde_json::from_str(line.trim()).context("failed to parse mpv numeric response")?;
        Ok(parsed.data)
    }
}

#[derive(Debug, Deserialize)]
struct FlatSearch {
    #[serde(default)]
    entries: Vec<FlatEntry>,
}

#[derive(Debug, Deserialize)]
struct FlatEntry {
    id: String,
    title: String,
    #[serde(default)]
    uploader: Option<String>,
}

async fn search_songs(limit: u8, query: String) -> Result<Vec<Song>> {
    let needle = format!("ytsearch{limit}:{query}");
    let output = Command::new("yt-dlp")
        .arg("--flat-playlist")
        .arg("--dump-single-json")
        .arg(needle)
        .output()
        .await
        .map_err(|err| anyhow::anyhow!("failed to run yt-dlp (is `yt-dlp` installed?): {err}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "yt-dlp returned non-zero status: {}",
            stderr.trim().chars().take(240).collect::<String>()
        );
    }

    let parsed: FlatSearch =
        serde_json::from_slice(&output.stdout).context("failed parsing yt-dlp flat json output")?;

    let songs = parsed
        .entries
        .into_iter()
        .map(|e| Song {
            id: e.id.clone(),
            title: e.title,
            webpage_url: format!("https://www.youtube.com/watch?v={}", e.id),
            uploader: e.uploader,
            duration: None,
        })
        .collect();

    Ok(songs)
}

async fn search_albums(limit: u8, query: String) -> Result<Vec<Song>> {
    let album_query = format!("{query} album");
    search_songs(limit, album_query).await
}

pub fn scan_local_library(config: &Config) -> Vec<LocalSong> {
    let mut songs = Vec::new();
    let extensions = ["mp3", "flac", "wav", "ogg", "m4a"];

    for dir in &config.general.music_directories {
        let path_str = if dir.starts_with('~') {
            if let Ok(home) = std::env::var("HOME") {
                dir.replacen('~', &home, 1)
            } else {
                dir.clone()
            }
        } else {
            dir.clone()
        };

        let path = std::path::Path::new(&path_str);
        if !path.exists() {
            continue;
        }

        for entry in walkdir::WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                if let Some(ext) = entry.path().extension().and_then(|s| s.to_str()) {
                    if extensions.contains(&ext.to_lowercase().as_str()) {
                        let song = extract_metadata(entry.path());
                        songs.push(song);
                    }
                }
            }
        }
    }
    songs
}

fn extract_metadata(path: &std::path::Path) -> LocalSong {
    let path_str = path.to_string_lossy().to_string();
    let filename = path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown")
        .to_string();

    if let Ok(tagged_file) = lofty::read_from_path(path) {
        let properties = tagged_file.properties();
        let tag = tagged_file.primary_tag();

        let title = tag.and_then(|t| t.title()).map(|s| s.to_string()).unwrap_or_else(|| filename.clone());
        let artist = tag.and_then(|t| t.artist()).map(|s| s.to_string()).unwrap_or_else(|| "Unknown".to_string());
        let album = tag.and_then(|t| t.album()).map(|s| s.to_string()).unwrap_or_else(|| "Unknown".to_string());
        let duration = properties.duration().as_secs() as f64;

        LocalSong {
            path: path_str,
            title,
            artist,
            album,
            duration,
        }
    } else {
        LocalSong {
            path: path_str,
            title: filename,
            artist: "Unknown".to_string(),
            album: "Unknown".to_string(),
            duration: 0.0,
        }
    }
}

pub fn refresh_library(config: &Config) -> Vec<LocalSong> {
    let songs = scan_local_library(config);
    crate::storage::save_local_library(&songs);
    songs
}

pub fn check_and_refresh_library(config: &Config) -> Option<Vec<LocalSong>> {
    let last_dirs = crate::storage::load_last_scanned_dirs();
    if last_dirs != config.general.music_directories {
        let songs = refresh_library(config);
        crate::storage::save_last_scanned_dirs(&config.general.music_directories);
        Some(songs)
    } else {
        None
    }
}


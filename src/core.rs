use crate::{
    config::Config,
    model::{LocalSong, Song},
    plugins::PluginManager,
};
use anyhow::{Context, Result};
use lofty::config::WriteOptions;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::tag::{Accessor, ItemKey, Tag};
use serde::Deserialize;
use serde_json::{Value, json};
use std::os::unix::fs::MetadataExt;
use std::{
    collections::{HashSet, VecDeque},
    process::Stdio,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::UnixStream,
    process::{Child, Command},
    sync::mpsc,
    time,
};

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
    PlayUrl {
        url: String,
        title: Option<String>,
    },
    RawMpv(Value),
    ToggleMute,
    Next,
    Prev,
    UpdateSearchSource(
        crate::config::SearchSource,
        std::sync::Arc<Vec<crate::config::CustomSource>>,
    ),
    DownloadSong {
        song: Song,
        path: String,
    },
    Quit,
    HandleSearchDone(Vec<Song>),
    HandleAlbumSearchDone(Vec<crate::model::Album>),
    ExpandAlbum {
        index: usize,
        url: String,
        artist: String,
    },
    HandleAlbumExpanded {
        index: usize,
        songs: Vec<Song>,
    },
}

#[derive(Debug)]
pub enum CoreEvent {
    SearchDone(Vec<Song>),
    AlbumSearchDone(Vec<crate::model::Album>),
    SearchFailed(String),
    AlbumSearchFailed(String),
    AlbumExpanded { index: usize, songs: Vec<Song> },
    AlbumExpandFailed(String),
    Started(Song),
    Paused,
    Resumed,
    TrackFinished,
    Progress { position: f64, duration: f64 },
    VolumeChanged(u8),
    MuteChanged(bool),
    Error(String),
    LibraryRefreshDone,
    DownloadFinished(Result<String, String>),
}

pub struct Core {
    config: Config,
    custom_sources: std::sync::Arc<Vec<crate::config::CustomSource>>,
    mpv_child: Child,
    history: VecDeque<Song>,
    volume: u8,
    muted: bool,
    was_playing: bool,
    plugins: Arc<Mutex<PluginManager>>,
}

impl Core {
    pub async fn new(config: Config, plugins: Arc<Mutex<PluginManager>>) -> Result<Self> {
        let mpv_child = Command::new("mpv")
            .arg("--idle")
            .arg("--no-video")
            .arg("--profile=high-quality")
            .arg("--audio-display=no")
            .arg("--volume=70")
            .arg("--audio-client-name=rs-pug")
            .arg(format!("--input-ipc-server={}", config.mpv.socket))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|err| anyhow::anyhow!("failed to start mpv (is `mpv` installed?): {err}"))?;
        wait_for_mpv_socket(config.mpv.socket.as_str()).await?;
        let custom_sources = std::sync::Arc::new(config.search.custom_sources.clone());
        let core = Self {
            plugins,
            config,
            custom_sources,
            mpv_child,
            history: VecDeque::new(),
            volume: 70,
            muted: false,
            was_playing: false,
        };
        Ok(core)
    }

    pub async fn run(
        mut self,
        mut rx: mpsc::UnboundedReceiver<CoreCmd>,
        tx: mpsc::UnboundedSender<CoreEvent>,
        cmd_tx: mpsc::UnboundedSender<CoreCmd>,
    ) {
        let mut tick = time::interval(Duration::from_millis(700));
        loop {
            tokio::select! {
                _ = tick.tick() => {
                    if let Err(err) = self.poll_playback_finished(&tx).await {
                        if !err.to_string().contains("failed to connect to mpv ipc socket") {
                            let _ = tx.send(CoreEvent::Error(format!("{err:#}")));
                        }
                    }
                }
                maybe_cmd = rx.recv() => {
                    let Some(cmd) = maybe_cmd else { break };
                    let res = match cmd {
                        CoreCmd::Search(query) => {
                            let limit = self.config.search.limit.max(1);
                            let query = self.transform_search_query(query);
                            let source = self.config.search.source;
                            let customs = std::sync::Arc::clone(&self.custom_sources);
                            let cmd_tx = cmd_tx.clone();
                            let tx = tx.clone();
                            tokio::spawn(async move {
                                match search_songs(limit, query, source, &customs).await {
                                    Ok(songs) => { let _ = cmd_tx.send(CoreCmd::HandleSearchDone(songs)); }
                                    Err(err) => { let _ = tx.send(CoreEvent::SearchFailed(format!("{err:#}"))); }
                                }
                            });
                            Ok(())
                        }
                        CoreCmd::SearchAlbums(query) => {
                            let limit = self.config.search.limit.max(1);
                            let query = self.transform_search_query(query);
                            let source = self.config.search.source;
                            let customs = std::sync::Arc::clone(&self.custom_sources);
                            let cmd_tx = cmd_tx.clone();
                            let tx = tx.clone();
                            tokio::spawn(async move {
                                match search_albums(limit, query, source, &customs).await {
                                    Ok(albums) => { let _ = cmd_tx.send(CoreCmd::HandleAlbumSearchDone(albums)); }
                                    Err(err) => { let _ = tx.send(CoreEvent::AlbumSearchFailed(format!("{err:#}"))); }
                                }
                            });
                            Ok(())
                        }
                        CoreCmd::ExpandAlbum { index, url, artist } => {
                            let cmd_tx = cmd_tx.clone();
                            let tx = tx.clone();
                            tokio::spawn(async move {
                                match expand_playlist(&url, &artist).await {
                                    Ok(songs) => { let _ = cmd_tx.send(CoreCmd::HandleAlbumExpanded { index, songs }); }
                                    Err(err) => { let _ = tx.send(CoreEvent::AlbumExpandFailed(format!("{err:#}"))); }
                                }
                            });
                            Ok(())
                        }
                        CoreCmd::Play(song) => self.play(song, &tx).await,
                        CoreCmd::SmartQueue(song) => {
                            let source = self.config.search.source;
                            let customs = std::sync::Arc::clone(&self.custom_sources);
                            let tx = tx.clone();
                            let cmd_tx = cmd_tx.clone();
                            tokio::spawn(async move {
                                if let Err(err) = perform_smart_queue(song, source, &customs, cmd_tx).await {
                                    let _ = tx.send(CoreEvent::Error(format!("{err:#}")));
                                }
                            });
                            Ok(())
                        }
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
                            self.send_mpv(json!({ "command" : command })).await
                        }
                        CoreCmd::ToggleMute => self.toggle_mute(&tx).await,
                        CoreCmd::Next => self.next(&tx).await,
                        CoreCmd::Prev => self.prev(&tx).await,
                        CoreCmd::DownloadSong { song, path } => {
                            let tx = tx.clone();
                            tokio::spawn(async move {
                                let output_template = format!("{}/%(title)s.%(ext)s", path);
                                let result = Command::new("yt-dlp")
                                    .arg("--no-playlist")
                                    .arg("-x")
                                    .arg("--audio-format")
                                    .arg("mp3")
                                    .arg("--audio-quality")
                                    .arg("0")
                                    .arg("--embed-metadata")
                                    .arg("--embed-thumbnail")
                                    .arg("--convert-thumbnails")
                                    .arg("jpg")
                                    .arg("--parse-metadata")
                                    .arg("uploader:%(artist)s")
                                    .arg("--parse-metadata")
                                    .arg("channel:%(artist)s")
                                    .arg("--parse-metadata")
                                    .arg("title:%(title)s")
                                    .arg("-o")
                                    .arg(output_template)
                                    .arg("--")
                                    .arg(&song.webpage_url)
                                    .output()
                                    .await;
                                let res = match result {
                                    Ok(output) if output.status.success() => {
                                        Ok(format!("Downloaded: {}", song.title))
                                    }
                                    Ok(output) => {
                                        let stderr = String::from_utf8_lossy(&output.stderr);
                                        Err(format!("yt-dlp failed: {}", stderr.trim()))
                                    }
                                    Err(err) => Err(format!("failed to run yt-dlp: {err}")),
                                };
                                let _ = tx.send(CoreEvent::DownloadFinished(res));
                            });
                            Ok(())
                        }
                        CoreCmd::UpdateSearchSource(source, customs) => {
                            self.config.search.source = source;
                            self.custom_sources = customs;
                            Ok(())
                        }
                        CoreCmd::HandleSearchDone(songs) => {
                            let songs = self.transform_search_results(songs);
                            let _ = tx.send(CoreEvent::SearchDone(songs));
                            Ok(())
                        }
                        CoreCmd::HandleAlbumSearchDone(albums) => {
                            let _ = tx.send(CoreEvent::AlbumSearchDone(albums));
                            Ok(())
                        }
                        CoreCmd::HandleAlbumExpanded { index, songs } => {
                            let _ = tx.send(CoreEvent::AlbumExpanded { index, songs });
                            Ok(())
                        }
                        CoreCmd::Quit => break,
                    };
                    if let Err(err) = res {
                        let _ = tx.send(CoreEvent::Error(format!("{err:#}")));
                    }
                }
            }
        }
        let _ = self.mpv_child.kill().await;
    }
    fn transform_search_query(&self, query: String) -> String {
        self.plugins
            .lock()
            .map(|plugins| plugins.transform_search_query(query.clone()))
            .unwrap_or(query)
    }
    fn transform_search_results(&self, songs: Vec<Song>) -> Vec<Song> {
        self.plugins
            .lock()
            .map(|plugins| plugins.transform_search_results(songs.clone()))
            .unwrap_or(songs)
    }
    fn transform_song_start(&self, song: Song) -> Song {
        self.plugins
            .lock()
            .map(|plugins| plugins.transform_song_start(song.clone()))
            .unwrap_or(song)
    }
    async fn play(&mut self, song: Song, tx: &mpsc::UnboundedSender<CoreEvent>) -> Result<()> {
        let song = self.transform_song_start(song);
        self.send_mpv(json!({ "command" : ["loadfile", song.webpage_url, "replace"] }))
            .await?;
        self.history.push_front(song.clone());
        if self.history.len() > 128 {
            self.history.pop_back();
        }
        self.was_playing = true;
        let _ = tx.send(CoreEvent::Started(song));
        Ok(())
    }
    async fn toggle_pause(&self, tx: &mpsc::UnboundedSender<CoreEvent>) -> Result<()> {
        self.send_mpv(json!({ "command" : ["cycle", "pause"] }))
            .await?;
        let _ = tx.send(CoreEvent::Paused);
        Ok(())
    }
    async fn next(&self, tx: &mpsc::UnboundedSender<CoreEvent>) -> Result<()> {
        self.send_mpv(json!({ "command" : ["playlist-next", "force"] }))
            .await?;
        let _ = tx.send(CoreEvent::Resumed);
        Ok(())
    }
    async fn prev(&self, tx: &mpsc::UnboundedSender<CoreEvent>) -> Result<()> {
        self.send_mpv(json!({ "command" : ["playlist-prev", "force"] }))
            .await?;
        let _ = tx.send(CoreEvent::Resumed);
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
        self.send_mpv(json!({ "command" : ["set_property", "volume", next] }))
            .await?;
        self.volume = next;
        let _ = tx.send(CoreEvent::VolumeChanged(next));
        Ok(())
    }
    async fn set_volume(&mut self, value: u8, tx: &mpsc::UnboundedSender<CoreEvent>) -> Result<()> {
        let next = value.min(130);
        self.send_mpv(json!({ "command" : ["set_property", "volume", next] }))
            .await?;
        self.volume = next;
        let _ = tx.send(CoreEvent::VolumeChanged(next));
        Ok(())
    }
    async fn seek_by(&self, seconds: i32, tx: &mpsc::UnboundedSender<CoreEvent>) -> Result<()> {
        self.send_mpv(json!({ "command" : ["seek", seconds, "relative"] }))
            .await?;
        let _ = tx.send(CoreEvent::Resumed);
        Ok(())
    }
    async fn toggle_mute(&mut self, tx: &mpsc::UnboundedSender<CoreEvent>) -> Result<()> {
        self.send_mpv(json!({ "command" : ["cycle", "mute"] }))
            .await?;
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
        let mut payload = serde_json::to_vec(&json!({ "command" : ["get_property", property] }))?;
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
        let mut payload = serde_json::to_vec(&json!({ "command" : ["get_property", property] }))?;
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
    url: String,
    webpage_url: Option<String>,
    #[serde(default)]
    uploader: Option<String>,
    #[serde(default)]
    duration: Option<f64>,
    #[serde(default)]
    ie_key: Option<String>,
}
async fn wait_for_mpv_socket(socket: &str) -> Result<()> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    loop {
        match UnixStream::connect(socket).await {
            Ok(_) => return Ok(()),
            Err(err) if tokio::time::Instant::now() < deadline => {
                let _ = err;
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
            Err(err) => {
                return Err(anyhow::anyhow!(
                    "mpv ipc socket did not become ready at {socket}: {err}"
                ));
            }
        }
    }
}
const COMMAND_STDOUT_CAP: usize = 8 * 1024 * 1024;
fn truncate_stderr(output: &[u8]) -> String {
    String::from_utf8_lossy(output)
        .trim()
        .chars()
        .take(240)
        .collect()
}
async fn run_ytdlp_flat(needle: &str) -> Result<FlatSearch> {
    let output = Command::new("yt-dlp")
        .arg("--flat-playlist")
        .arg("--dump-single-json")
        .arg("--")
        .arg(needle)
        .output()
        .await
        .map_err(|err| anyhow::anyhow!("failed to run yt-dlp (is `yt-dlp` installed?): {err}"))?;
    if !output.status.success() {
        anyhow::bail!(
            "yt-dlp returned non-zero status: {}",
            truncate_stderr(&output.stderr)
        );
    }
    serde_json::from_slice(&output.stdout).context("failed parsing yt-dlp flat json output")
}
fn youtube_watch_url(id: &str) -> String {
    format!("https://www.youtube.com/watch?v={id}")
}
fn flat_entry_url(entry: &FlatEntry) -> String {
    entry
        .webpage_url
        .clone()
        .unwrap_or_else(|| entry.url.clone())
}
fn substitute_arg(arg: &str, query: &str, limit: u8) -> String {
    arg.replace("{query}", query)
        .replace("{limit}", &limit.to_string())
}
fn filter_command_songs(songs: Vec<Song>, limit: u8) -> Vec<Song> {
    let mut kept = Vec::new();
    let mut skipped = 0;
    for song in songs {
        if song.id.trim().is_empty()
            || song.title.trim().is_empty()
            || song.webpage_url.trim().is_empty()
        {
            skipped += 1;
            continue;
        }
        kept.push(song);
        if kept.len() >= limit.max(1) as usize {
            break;
        }
    }
    if skipped > 0 {
        log::warn!("command source skipped {skipped} entries with empty id/title/url");
    }
    kept
}
async fn run_command_source(
    name: &str,
    command: &[String],
    timeout_secs: u64,
    limit: u8,
    query: &str,
) -> Result<Vec<Song>> {
    let substituted = command
        .iter()
        .map(|arg| substitute_arg(arg, query, limit))
        .collect::<Vec<_>>();
    let Some((program, args)) = substituted.split_first() else {
        anyhow::bail!("source '{name}': empty command");
    };
    let child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|err| anyhow::anyhow!("source '{name}': failed to run command: {err}"))?;
    let timeout = Duration::from_secs(timeout_secs.max(1));
    let output = tokio::time::timeout(timeout, child.wait_with_output())
        .await
        .map_err(|_| anyhow::anyhow!("source '{name}': command timed out after {timeout_secs}s"))?
        .map_err(|err| anyhow::anyhow!("source '{name}': failed reading command output: {err}"))?;
    if !output.status.success() {
        anyhow::bail!(
            "source '{name}': command returned non-zero status: {}",
            truncate_stderr(&output.stderr)
        );
    }
    if output.stdout.len() > COMMAND_STDOUT_CAP {
        anyhow::bail!("source '{name}': command output exceeded 8 MiB");
    }
    let songs: Vec<Song> = serde_json::from_slice(&output.stdout)
        .with_context(|| format!("source '{name}': invalid JSON from command"))?;
    Ok(filter_command_songs(songs, limit))
}
async fn perform_smart_queue(
    current: Song,
    source: crate::config::SearchSource,
    customs: &std::sync::Arc<Vec<crate::config::CustomSource>>,
    cmd_tx: mpsc::UnboundedSender<CoreCmd>,
) -> Result<()> {
    let query = current
        .uploader
        .as_ref()
        .map(|u| format!("{u} {}", current.title))
        .unwrap_or_else(|| current.title.clone());
    let candidates = search_songs(8, query, source, customs).await?;
    let maybe_next = candidates
        .into_iter()
        .find(|song| song.id != current.id && song.title != current.title);
    if let Some(next_song) = maybe_next {
        let _ = cmd_tx.send(CoreCmd::Play(next_song));
        Ok(())
    } else {
        anyhow::bail!("smart queue: no similar song found")
    }
}
async fn search_songs(
    limit: u8,
    query: String,
    source: crate::config::SearchSource,
    customs: &std::sync::Arc<Vec<crate::config::CustomSource>>,
) -> Result<Vec<Song>> {
    use crate::config::{CustomSourceKind, SearchSource};
    match source {
        SearchSource::Sonum => crate::sonum::search_songs(limit, query).await,
        SearchSource::YouTube => {
            let parsed = run_ytdlp_flat(&format!("ytsearch{limit}:{query}")).await?;
            Ok(parsed
                .entries
                .into_iter()
                .map(|e| Song {
                    id: e.id.clone(),
                    title: e.title,
                    webpage_url: youtube_watch_url(&e.id),
                    uploader: e.uploader,
                    duration: None,
                })
                .collect())
        }
        SearchSource::SoundCloud => {
            let parsed = run_ytdlp_flat(&format!("scsearch{limit}:{query}")).await?;
            Ok(parsed
                .entries
                .into_iter()
                .map(|e| {
                    let webpage_url = flat_entry_url(&e);
                    Song {
                        id: e.id.clone(),
                        title: e.title,
                        webpage_url,
                        uploader: e.uploader,
                        duration: None,
                    }
                })
                .collect())
        }
        SearchSource::Custom(i) => {
            let Some(custom) = customs.get(i as usize) else {
                anyhow::bail!("unknown custom source");
            };
            match &custom.kind {
                CustomSourceKind::Ytdlp { prefix } => {
                    let parsed = run_ytdlp_flat(&format!("{prefix}{limit}:{query}"))
                        .await
                        .with_context(|| format!("source '{}'", custom.name))?;
                    Ok(parsed
                        .entries
                        .into_iter()
                        .map(|e| {
                            let webpage_url = flat_entry_url(&e);
                            Song {
                                id: e.id.clone(),
                                title: e.title,
                                webpage_url,
                                uploader: e.uploader,
                                duration: None,
                            }
                        })
                        .collect())
                }
                CustomSourceKind::Command {
                    command,
                    timeout_secs,
                } => run_command_source(&custom.name, command, *timeout_secs, limit, &query).await,
            }
        }
    }
}
async fn search_albums(
    limit: u8,
    query: String,
    source: crate::config::SearchSource,
    customs: &std::sync::Arc<Vec<crate::config::CustomSource>>,
) -> Result<Vec<crate::model::Album>> {
    use crate::config::{CustomSourceKind, SearchSource};
    match source {
        SearchSource::Sonum => crate::sonum::search_albums(limit, query).await,
        SearchSource::YouTube => search_youtube_playlist_albums(limit, &query).await,
        // scsearch yields tracks only cause soundcloud exposes no playlist search to ytdlp, ytdlp stupid
        SearchSource::SoundCloud => search_video_albums("scsearch", limit, &query, false).await,
        SearchSource::Custom(i) => {
            let Some(custom) = customs.get(i as usize) else {
                anyhow::bail!("unknown custom source");
            };
            match &custom.kind {
                CustomSourceKind::Ytdlp { prefix } => {
                    search_video_albums(prefix, limit, &query, false)
                        .await
                        .with_context(|| format!("source '{}'", custom.name))
                }
                CustomSourceKind::Command { .. } => {
                    anyhow::bail!("album search is not supported for source '{}'", custom.name);
                }
            }
        }
    }
}

async fn search_video_albums(
    prefix: &str,
    limit: u8,
    query: &str,
    youtube_style: bool,
) -> Result<Vec<crate::model::Album>> {
    let parsed = run_ytdlp_flat(&format!("{prefix}{limit}:{query} full album")).await?;
    let mut albums = Vec::new();
    for entry in parsed.entries {
        if !is_full_album_title(&entry.title) {
            continue;
        }
        let artist = entry
            .uploader
            .clone()
            .unwrap_or_else(|| "Unknown Artist".to_string());
        let name = entry.title.clone();
        albums.push(crate::model::Album {
            name,
            artist: artist.clone(),
            songs: vec![flat_entry_to_song(entry, youtube_style, &artist)],
            playlist_url: None,
        });
    }
    Ok(albums)
}

const PLAYLIST_EXPAND_TIMEOUT_SECS: u64 = 30;

async fn search_youtube_playlist_albums(
    limit: u8,
    query: &str,
) -> Result<Vec<crate::model::Album>> {
    // super secret mysterious link, why? without it only single vidoes come back, why? secret...
    let needle = format!(
        "https://www.youtube.com/results?search_query={}+full+album&sp=EgIQAw%3D%3D",
        percent_encode_query(query)
    );
    let searched = run_ytdlp_flat(&needle).await?;
    let mut albums = Vec::new();
    for entry in searched.entries.into_iter().take(limit.max(1) as usize) {
        let artist = entry
            .uploader
            .clone()
            .unwrap_or_else(|| "Unknown Artist".to_string());
        let name = entry.title.clone();
        if !is_playlist_entry(&entry) {
            if !is_full_album_title(&name) {
                continue;
            }
            albums.push(crate::model::Album {
                name,
                artist: artist.clone(),
                songs: vec![flat_entry_to_song(entry, true, &artist)],
                playlist_url: None,
            });
            continue;
        }
        albums.push(crate::model::Album {
            name,
            artist,
            songs: Vec::new(),
            playlist_url: Some(absolute_youtube_url(&entry.url)),
        });
    }
    Ok(albums)
}

async fn expand_playlist(url: &str, artist: &str) -> Result<Vec<Song>> {
    let list = tokio::time::timeout(
        Duration::from_secs(PLAYLIST_EXPAND_TIMEOUT_SECS),
        run_ytdlp_flat(url),
    )
    .await
    .map_err(|_| {
        anyhow::anyhow!("playlist expand timed out after {PLAYLIST_EXPAND_TIMEOUT_SECS}s")
    })?
    .with_context(|| format!("playlist expand failed for {url}"))?;
    if list.entries.is_empty() {
        anyhow::bail!("playlist at {url} returned no tracks");
    }
    Ok(list
        .entries
        .into_iter()
        .map(|sub| flat_entry_to_song(sub, true, artist))
        .collect())
}

fn is_full_album_title(title: &str) -> bool {
    let lower = title.to_lowercase();
    lower.contains("full album") || lower.contains("complete album")
}

fn is_playlist_entry(entry: &FlatEntry) -> bool {
    entry.ie_key.as_deref() == Some("YoutubeTab") || entry.url.contains("playlist?list=")
}

fn absolute_youtube_url(url: &str) -> String {
    if url.starts_with("http://") || url.starts_with("https://") {
        url.to_owned()
    } else {
        format!("https://www.youtube.com/{}", url.trim_start_matches('/'))
    }
}

fn percent_encode_query(query: &str) -> String {
    let mut out = String::new();
    for byte in query.bytes() {
        match byte {
            b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn flat_entry_to_song(entry: FlatEntry, youtube_style: bool, fallback_artist: &str) -> Song {
    let artist = entry
        .uploader
        .clone()
        .unwrap_or_else(|| fallback_artist.to_string());
    let webpage_url = if youtube_style {
        youtube_watch_url(&entry.id)
    } else {
        flat_entry_url(&entry)
    };
    Song {
        id: entry.id,
        title: entry.title,
        webpage_url,
        uploader: Some(artist),
        duration: entry.duration,
    }
}
pub fn scan_local_library(config: &Config) -> Vec<LocalSong> {
    let mut songs = Vec::new();
    let mut seen_files = HashSet::new();
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
        for entry in walkdir::WalkDir::new(path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                let p = entry.path();
                if let Some(ext) = p.extension().and_then(|s| s.to_str()) {
                    if extensions.contains(&ext.to_lowercase().as_str()) {
                        if let Ok(meta) = entry.metadata() {
                            let dev_ino = (meta.dev(), meta.ino());
                            if !seen_files.insert(dev_ino) {
                                continue;
                            }
                        }
                        let song = extract_metadata(p);
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
    let filename = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown")
        .to_string();
    let mtime = std::fs::metadata(path)
        .and_then(|m| m.modified())
        .map(|t| {
            t.duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        })
        .unwrap_or(0);
    if let Ok(tagged_file) = lofty::read_from_path(path) {
        let properties = tagged_file.properties();
        let tag = tagged_file.primary_tag();
        let title = tag
            .and_then(|t| t.title())
            .map(|s| s.to_string())
            .unwrap_or_else(|| filename.clone());
        let artist = tag
            .and_then(|t| t.artist())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Unknown".to_string());
        let album = tag
            .and_then(|t| t.album())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Unknown".to_string());
        let genre = tag
            .and_then(|t| t.genre())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Unknown".to_string());
        let year = tag
            .and_then(|t| t.get_string(ItemKey::Year))
            .and_then(|s| s.get(..4).unwrap_or(s).parse::<u32>().ok());
        let duration = properties.duration().as_secs() as f64;
        LocalSong {
            path: path_str,
            title,
            artist,
            album,
            genre,
            year,
            duration,
            mtime,
            added_at: mtime,
            play_count: 0,
            last_played: None,
        }
    } else {
        LocalSong {
            path: path_str,
            title: filename,
            artist: "Unknown".to_string(),
            album: "Unknown".to_string(),
            genre: "Unknown".to_string(),
            year: None,
            duration: 0.0,
            mtime,
            added_at: mtime,
            play_count: 0,
            last_played: None,
        }
    }
}
pub fn refresh_library(config: &Config, storage: &crate::storage::Storage) -> Vec<LocalSong> {
    let songs = scan_local_library(config);
    let _ = storage.save_local_library(&songs);
    songs
}
pub fn check_and_refresh_library(
    config: &Config,
    storage: &crate::storage::Storage,
) -> Option<Vec<LocalSong>> {
    let songs = refresh_library(config, storage);
    storage.save_last_scanned_dirs(&config.general.music_directories);
    Some(songs)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use std::fs;
    use std::os::unix::fs::symlink;
    #[test]
    fn test_scan_local_library_follows_symlinks() {
        let tmp_dir = std::env::temp_dir().join("rs_pug_test_symlinks");
        if tmp_dir.exists() {
            let _ = fs::remove_dir_all(&tmp_dir);
        }
        fs::create_dir_all(&tmp_dir).unwrap();
        let source_dir = tmp_dir.join("source");
        fs::create_dir_all(&source_dir).unwrap();
        let song_path = source_dir.join("test.mp3");
        fs::write(&song_path, "dummy content").unwrap();
        let scan_dir = tmp_dir.join("scan");
        fs::create_dir_all(&scan_dir).unwrap();
        let link_path = scan_dir.join("music_link");
        symlink(&source_dir, &link_path).unwrap();
        let mut config = Config::default();
        config.general.music_directories = vec![scan_dir.to_str().unwrap().to_string()];
        let songs = scan_local_library(&config);
        let result = !songs.is_empty();
        let _ = fs::remove_dir_all(&tmp_dir);
        assert!(
            result,
            "Should have found songs in symlinked directory. Found: {}",
            songs.len()
        );
    }
    #[test]
    fn substitute_arg_replaces_query_and_limit() {
        assert_eq!(substitute_arg("{query}", "hello world", 20), "hello world");
        assert_eq!(substitute_arg("n={limit}", "x", 8), "n=8");
        assert_eq!(substitute_arg("{query}-{query}-{limit}", "a", 3), "a-a-3");
        assert_eq!(substitute_arg("plain", "a", 3), "plain");
    }
    #[test]
    fn filter_command_songs_skips_empty_and_truncates() {
        let songs = vec![
            Song {
                id: "".to_owned(),
                title: "t".to_owned(),
                webpage_url: "u".to_owned(),
                uploader: None,
                duration: None,
            },
            Song {
                id: "1".to_owned(),
                title: "  ".to_owned(),
                webpage_url: "u".to_owned(),
                uploader: None,
                duration: None,
            },
            Song {
                id: "1".to_owned(),
                title: "a".to_owned(),
                webpage_url: "http://x".to_owned(),
                uploader: Some("u".to_owned()),
                duration: Some(12.0),
            },
            Song {
                id: "2".to_owned(),
                title: "b".to_owned(),
                webpage_url: "http://y".to_owned(),
                uploader: None,
                duration: None,
            },
        ];
        let kept = filter_command_songs(songs, 1);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].id, "1");
    }
    #[test]
    fn percent_encode_query_keeps_search_readable() {
        assert_eq!(percent_encode_query("rust in peace"), "rust+in+peace");
        assert_eq!(percent_encode_query("megadeth"), "megadeth");
        assert_eq!(percent_encode_query("a&b=c"), "a%26b%3Dc");
    }
    #[test]
    fn playlist_entries_detected_by_extractor_or_url() {
        let playlist = FlatEntry {
            id: "PL123".to_owned(),
            title: "Rust In Peace".to_owned(),
            url: "https://www.youtube.com/playlist?list=PL123".to_owned(),
            webpage_url: None,
            uploader: None,
            duration: None,
            ie_key: Some("YoutubeTab".to_owned()),
        };
        assert!(is_playlist_entry(&playlist));
        let video = FlatEntry {
            id: "FiFIGN5084Y".to_owned(),
            title: "Holy Wars".to_owned(),
            url: "FiFIGN5084Y".to_owned(),
            webpage_url: None,
            uploader: None,
            duration: Some(397.0),
            ie_key: Some("Youtube".to_owned()),
        };
        assert!(!is_playlist_entry(&video));
    }
    #[test]
    fn full_album_title_matches_case_insensitive() {
        assert!(is_full_album_title("Rust In Peace (Full Album)"));
        assert!(is_full_album_title("COMPLETE ALBUM 1990"));
        assert!(!is_full_album_title("Rust In Peace"));
    }
    #[test]
    fn absolute_youtube_url_prefixes_relative_paths() {
        assert_eq!(
            absolute_youtube_url("https://www.youtube.com/playlist?list=PL1"),
            "https://www.youtube.com/playlist?list=PL1"
        );
        assert_eq!(
            absolute_youtube_url("/playlist?list=PL1"),
            "https://www.youtube.com/playlist?list=PL1"
        );
    }
    #[test]
    fn command_stdout_parses_as_songs() {
        let raw = br#"[{"id":"1","title":"a","webpage_url":"http://x","uploader":"u","duration":11.5},{"id":"2","title":"b","webpage_url":"http://y"}]"#;
        let songs: Vec<Song> = serde_json::from_slice(raw).unwrap();
        assert_eq!(songs.len(), 2);
        assert_eq!(songs[0].uploader.as_deref(), Some("u"));
        assert_eq!(songs[1].duration, None);
        let bad = br#"[{"title":"missing id"}]"#;
        assert!(serde_json::from_slice::<Vec<Song>>(bad).is_err());
    }
    #[test]
    fn test_scan_local_library_deduplicates_symlinks() {
        let tmp_dir = std::env::temp_dir().join("rs_pug_test_dedup");
        if tmp_dir.exists() {
            let _ = fs::remove_dir_all(&tmp_dir);
        }
        fs::create_dir_all(&tmp_dir).unwrap();
        let source_dir = tmp_dir.join("source");
        fs::create_dir_all(&source_dir).unwrap();
        let song_path = source_dir.join("test.mp3");
        fs::write(&song_path, "dummy content").unwrap();
        let scan_dir = tmp_dir.join("scan");
        fs::create_dir_all(&scan_dir).unwrap();
        let link1 = scan_dir.join("link1.mp3");
        let link2 = scan_dir.join("link2.mp3");
        symlink(&song_path, &link1).unwrap();
        symlink(&song_path, &link2).unwrap();
        let mut config = Config::default();
        config.general.music_directories = vec![scan_dir.to_str().unwrap().to_string()];
        let songs = scan_local_library(&config);
        let _ = fs::remove_dir_all(&tmp_dir);
        assert_eq!(
            songs.len(),
            1,
            "Should have deduplicated symlinks to the same file. Found: {}",
            songs.len()
        );
    }
}
pub fn write_local_tags(song: &LocalSong) -> Result<()> {
    let path = std::path::Path::new(&song.path);
    let mut tagged_file = lofty::read_from_path(path)
        .with_context(|| format!("failed to read tags from {}", song.path))?;
    let tag_type = tagged_file.primary_tag_type();
    if tagged_file.primary_tag_mut().is_none() {
        tagged_file.insert_tag(Tag::new(tag_type));
    }
    let tag = tagged_file
        .primary_tag_mut()
        .ok_or_else(|| anyhow::anyhow!("failed to create tag for {}", song.path))?;
    tag.set_title(song.title.clone());
    tag.set_artist(song.artist.clone());
    tag.set_album(song.album.clone());
    tag.set_genre(song.genre.clone());
    if let Some(year) = song.year {
        tag.insert_text(ItemKey::Year, year.to_string());
    } else {
        tag.remove_key(ItemKey::Year);
    }
    tagged_file
        .save_to_path(path, WriteOptions::default())
        .with_context(|| format!("failed to write tags to {}", song.path))
}

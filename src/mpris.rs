use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};

use tokio::sync::mpsc;
use zbus::object_server::InterfaceRef;
use zbus::zvariant::{ObjectPath, OwnedValue, Value};
use zbus::{Connection, SignalContext, interface};

use crate::model::{RepeatMode, Song};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MprisPlaybackStatus {
    Playing,
    Paused,
    Stopped,
}

impl MprisPlaybackStatus {
    fn as_str(self) -> &'static str {
        match self {
            MprisPlaybackStatus::Playing => "Playing",
            MprisPlaybackStatus::Paused => "Paused",
            MprisPlaybackStatus::Stopped => "Stopped",
        }
    }
}

#[derive(Debug, Clone)]
pub struct MprisTrack {
    pub id: String,
    pub title: String,
    pub artist: Option<String>,
    pub length_micros: i64,
}

pub fn track_from_song(song: &Song) -> MprisTrack {
    MprisTrack {
        id: song.id.clone(),
        title: song.title.clone(),
        artist: song.uploader.clone(),
        length_micros: song
            .duration
            .map(|d| (d.max(0.0) * 1_000_000.0) as i64)
            .unwrap_or(0),
    }
}

#[derive(Debug, Clone)]
struct MprisSharedState {
    playback_status: MprisPlaybackStatus,
    track: Option<MprisTrack>,
    position_micros: i64,
    volume: f64,
    loop_status: String,
}

impl Default for MprisSharedState {
    fn default() -> Self {
        Self {
            playback_status: MprisPlaybackStatus::Stopped,
            track: None,
            position_micros: 0,
            volume: 0.7,
            loop_status: "None".to_owned(),
        }
    }
}

type SharedState = Arc<Mutex<MprisSharedState>>;

/// Stuff coming from media keys, playerctl, or widgets to control the app
/// We just drain it right in the main loop, same as hotreoad and plugins.
#[derive(Debug)]
pub enum MprisAction {
    Next,
    Previous,
    Play,
    Pause,
    PlayPause,
    Stop,
    SeekRelative(i64),
    SetPositionAbsolute(i64),
    OpenUri(String),
    SetVolume(f64),
    SetLoopStatus(String),
}

fn track_object_path(id: &str) -> ObjectPath<'static> {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    id.hash(&mut hasher);
    let digest = hasher.finish();
    ObjectPath::try_from(format!("/org/mpris/MediaPlayer2/Track/{digest:x}")).unwrap_or_else(|_| {
        ObjectPath::try_from("/org/mpris/MediaPlayer2/Track/0").expect("valid path")
    })
}

struct RootIface;

#[interface(name = "org.mpris.MediaPlayer2")]
impl RootIface {
    async fn raise(&self) -> zbus::fdo::Result<()> {
        Ok(())
    }

    async fn quit(&self) -> zbus::fdo::Result<()> {
        Ok(())
    }

    #[zbus(property)]
    async fn can_quit(&self) -> bool {
        false
    }

    #[zbus(property)]
    async fn can_raise(&self) -> bool {
        false
    }

    #[zbus(property)]
    async fn has_track_list(&self) -> bool {
        false
    }

    #[zbus(property)]
    async fn identity(&self) -> String {
        "rs-pug".to_owned()
    }

    #[zbus(property)]
    async fn desktop_entry(&self) -> String {
        "rs-pug".to_owned()
    }

    #[zbus(property)]
    async fn supported_uri_schemes(&self) -> Vec<String> {
        vec!["http".to_owned(), "https".to_owned(), "file".to_owned()]
    }

    #[zbus(property)]
    async fn supported_mime_types(&self) -> Vec<String> {
        Vec::new()
    }
}

struct PlayerIface {
    state: SharedState,
    action_tx: mpsc::UnboundedSender<MprisAction>,
}

#[interface(name = "org.mpris.MediaPlayer2.Player")]
impl PlayerIface {
    async fn next(&self) -> zbus::fdo::Result<()> {
        let _ = self.action_tx.send(MprisAction::Next);
        Ok(())
    }

    async fn previous(&self) -> zbus::fdo::Result<()> {
        let _ = self.action_tx.send(MprisAction::Previous);
        Ok(())
    }

    async fn pause(&self) -> zbus::fdo::Result<()> {
        let _ = self.action_tx.send(MprisAction::Pause);
        Ok(())
    }

    #[zbus(name = "PlayPause")]
    async fn play_pause(&self) -> zbus::fdo::Result<()> {
        let _ = self.action_tx.send(MprisAction::PlayPause);
        Ok(())
    }

    async fn stop(&self) -> zbus::fdo::Result<()> {
        let _ = self.action_tx.send(MprisAction::Stop);
        Ok(())
    }

    async fn play(&self) -> zbus::fdo::Result<()> {
        let _ = self.action_tx.send(MprisAction::Play);
        Ok(())
    }

    async fn seek(&self, offset: i64) -> zbus::fdo::Result<()> {
        let _ = self.action_tx.send(MprisAction::SeekRelative(offset));
        Ok(())
    }

    #[zbus(name = "SetPosition")]
    async fn set_position(
        &self,
        _track_id: ObjectPath<'_>,
        position: i64,
    ) -> zbus::fdo::Result<()> {
        let _ = self
            .action_tx
            .send(MprisAction::SetPositionAbsolute(position));
        Ok(())
    }

    #[zbus(name = "OpenUri")]
    async fn open_uri(&self, uri: String) -> zbus::fdo::Result<()> {
        let _ = self.action_tx.send(MprisAction::OpenUri(uri));
        Ok(())
    }

    #[zbus(signal)]
    async fn seeked(ctxt: &SignalContext<'_>, position: i64) -> zbus::Result<()>;

    #[zbus(property)]
    async fn playback_status(&self) -> String {
        self.state
            .lock()
            .unwrap()
            .playback_status
            .as_str()
            .to_owned()
    }

    #[zbus(property)]
    async fn loop_status(&self) -> String {
        self.state.lock().unwrap().loop_status.clone()
    }

    #[zbus(property)]
    async fn set_loop_status(&self, value: String) -> zbus::Result<()> {
        self.state.lock().unwrap().loop_status = value.clone();
        let _ = self.action_tx.send(MprisAction::SetLoopStatus(value));
        Ok(())
    }

    #[zbus(property)]
    async fn rate(&self) -> f64 {
        1.0
    }

    #[zbus(property)]
    async fn set_rate(&self, _value: f64) -> zbus::Result<()> {
        Ok(())
    }

    #[zbus(property)]
    async fn metadata(&self) -> HashMap<String, OwnedValue> {
        let state = self.state.lock().unwrap();
        let mut map = HashMap::new();
        if let Some(track) = &state.track {
            let path = track_object_path(&track.id);
            if let Ok(value) = OwnedValue::try_from(Value::from(path)) {
                map.insert("mpris:trackid".to_owned(), value);
            }
            if let Ok(value) = OwnedValue::try_from(Value::from(track.length_micros)) {
                map.insert("mpris:length".to_owned(), value);
            }
            if let Ok(value) = OwnedValue::try_from(Value::from(track.title.clone())) {
                map.insert("xesam:title".to_owned(), value);
            }
            if let Some(artist) = &track.artist {
                if let Ok(value) = OwnedValue::try_from(Value::from(vec![artist.clone()])) {
                    map.insert("xesam:artist".to_owned(), value);
                }
            }
        }
        map
    }

    #[zbus(property)]
    async fn volume(&self) -> f64 {
        self.state.lock().unwrap().volume
    }

    #[zbus(property)]
    async fn set_volume(&self, value: f64) -> zbus::Result<()> {
        let clamped = value.clamp(0.0, 1.3);
        self.state.lock().unwrap().volume = clamped;
        let _ = self.action_tx.send(MprisAction::SetVolume(clamped));
        Ok(())
    }

    #[zbus(property(emits_changed_signal = "false"))]
    async fn position(&self) -> i64 {
        self.state.lock().unwrap().position_micros
    }

    #[zbus(property)]
    async fn minimum_rate(&self) -> f64 {
        1.0
    }

    #[zbus(property)]
    async fn maximum_rate(&self) -> f64 {
        1.0
    }

    #[zbus(property)]
    async fn can_go_next(&self) -> bool {
        true
    }

    #[zbus(property)]
    async fn can_go_previous(&self) -> bool {
        true
    }

    #[zbus(property)]
    async fn can_play(&self) -> bool {
        true
    }

    #[zbus(property)]
    async fn can_pause(&self) -> bool {
        true
    }

    #[zbus(property)]
    async fn can_seek(&self) -> bool {
        true
    }

    #[zbus(property)]
    async fn can_control(&self) -> bool {
        true
    }
}

/// MPRIS2 daemon handle. Safe to hold onto forever, if theres no session bus, sync and notify calls just quietly do nothing lmao
pub struct MprisServer {
    state: SharedState,
    iface: Option<InterfaceRef<PlayerIface>>,

    _conn: Option<Connection>,
}

impl MprisServer {
    /// Tries to hook up MPRIS2 Never fails outright, errors just log a warning and return a dummy server as if it were disabled
    pub async fn start(enabled: bool) -> (Self, mpsc::UnboundedReceiver<MprisAction>) {
        let (action_tx, action_rx) = mpsc::unbounded_channel();
        let state: SharedState = Arc::new(Mutex::new(MprisSharedState::default()));
        if !enabled {
            return (
                Self {
                    state,
                    iface: None,
                    _conn: None,
                },
                action_rx,
            );
        }
        match Self::try_start(state.clone(), action_tx).await {
            Ok((conn, iface)) => {
                log::info!("native MPRIS2 daemon registered on the session bus");
                (
                    Self {
                        state,
                        iface: Some(iface),
                        _conn: Some(conn),
                    },
                    action_rx,
                )
            }
            Err(err) => {
                log::warn!("MPRIS2 daemon not started: {err:#}");
                (
                    Self {
                        state,
                        iface: None,
                        _conn: None,
                    },
                    action_rx,
                )
            }
        }
    }

    async fn try_start(
        state: SharedState,
        action_tx: mpsc::UnboundedSender<MprisAction>,
    ) -> anyhow::Result<(Connection, InterfaceRef<PlayerIface>)> {
        let conn = Connection::session().await?;
        let path = "/org/mpris/MediaPlayer2";
        conn.object_server()
            .at(
                path,
                PlayerIface {
                    state: state.clone(),
                    action_tx,
                },
            )
            .await?;
        conn.object_server().at(path, RootIface).await?;
        let base_name = "org.mpris.MediaPlayer2.rs_pug";
        if conn.request_name(base_name).await.is_err() {
            // Another instance is running, so MPRIS gives this one a unique suffix to avoid clashing with the main player name
            let fallback = format!("{base_name}.instance{}", std::process::id());
            conn.request_name(fallback.as_str()).await?;
        }
        let iface = conn
            .object_server()
            .interface::<_, PlayerIface>(path)
            .await?;
        Ok((conn, iface))
    }

    pub fn sync(
        &self,
        playback_status: MprisPlaybackStatus,
        track: Option<MprisTrack>,
        position_seconds: f64,
        volume_percent: u8,
    ) {
        let Some(iface) = self.iface.clone() else {
            return;
        };
        let position_micros = (position_seconds.max(0.0) * 1_000_000.0) as i64;
        let volume = (volume_percent as f64 / 100.0).clamp(0.0, 1.3);
        let mut changed_status = false;
        let mut changed_track = false;
        let mut changed_volume = false;
        {
            let mut guard = self.state.lock().unwrap();
            guard.position_micros = position_micros;
            if guard.playback_status != playback_status {
                guard.playback_status = playback_status;
                changed_status = true;
            }
            let track_id_changed =
                guard.track.as_ref().map(|t| &t.id) != track.as_ref().map(|t| &t.id);
            if track_id_changed {
                guard.track = track;
                changed_track = true;
            }
            if (guard.volume - volume).abs() > f64::EPSILON {
                guard.volume = volume;
                changed_volume = true;
            }
        }
        if changed_status || changed_track || changed_volume {
            tokio::spawn(async move {
                let ctxt = iface.signal_context();
                let player = iface.get().await;
                if changed_status {
                    let _ = player.playback_status_changed(ctxt).await;
                }
                if changed_track {
                    let _ = player.metadata_changed(ctxt).await;
                }
                if changed_volume {
                    let _ = player.volume_changed(ctxt).await;
                }
            });
        }
    }

    pub fn sync_loop_status(&self, mode: RepeatMode) {
        let Some(iface) = self.iface.clone() else {
            return;
        };
        let value = match mode {
            RepeatMode::Off => "None",
            RepeatMode::One => "Track",
            RepeatMode::All => "Playlist",
        }
        .to_owned();
        let changed = {
            let mut guard = self.state.lock().unwrap();
            if guard.loop_status != value {
                guard.loop_status = value.clone();
                true
            } else {
                false
            }
        };
        if changed {
            tokio::spawn(async move {
                let ctxt = iface.signal_context();
                let player = iface.get().await;
                let _ = player.loop_status_changed(ctxt).await;
            });
        }
    }

    pub fn notify_seeked(&self, position_seconds: f64) {
        let Some(iface) = self.iface.clone() else {
            return;
        };
        let position_micros = (position_seconds.max(0.0) * 1_000_000.0) as i64;
        {
            let mut guard = self.state.lock().unwrap();
            guard.position_micros = position_micros;
        }
        tokio::spawn(async move {
            let ctxt = iface.signal_context();
            let _ = PlayerIface::seeked(ctxt, position_micros).await;
        });
    }
}

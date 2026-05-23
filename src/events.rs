use crate::core::{CoreCmd, CoreEvent};
use crate::model::{App, Focus, PlayerState, RepeatMode, Tab};
use crate::plugins::{PluginCoreAction, PluginDispatch, PluginEvent};
use crate::ui_helpers;
use tokio::sync::mpsc;

pub fn apply_event(app: &mut App, event: CoreEvent) -> Option<CoreCmd> {
    let cmd = match event {
        CoreEvent::SearchDone(songs) => {
            app.search_results = songs;
            app.selected_result = 0;
            app.focus = Focus::Results;
            app.player_state = if app.current_song.is_some() {
                PlayerState::Playing
            } else {
                PlayerState::Idle
            };
            app.set_flash(format!("Loaded {} result(s)", app.search_results.len()), 4);
            None
        }
        CoreEvent::AlbumSearchDone(songs) => {
            app.album_results = songs;
            app.selected_album_result = 0;
            app.focus = Focus::Results;
            app.player_state = if app.current_song.is_some() {
                PlayerState::Playing
            } else {
                PlayerState::Idle
            };
            app.set_flash(
                format!("Loaded {} album result(s)", app.album_results.len()),
                4,
            );
            None
        }
        CoreEvent::SearchFailed(msg) => {
            app.player_state = PlayerState::Idle;
            app.set_flash(msg, 6);
            None
        }
        CoreEvent::AlbumSearchFailed(msg) => {
            app.player_state = PlayerState::Idle;
            app.set_flash(msg, 6);
            None
        }
        CoreEvent::Started(song) => {
            app.current_song = Some(song.clone());
            app.player_state = PlayerState::Playing;
            app.playback_pos = 0.0;
            app.playback_duration = song.duration.unwrap_or(0.0);
            app.recently_played.retain(|s| s.id != song.id);
            app.recently_played.push_front(song.clone());
            while app.recently_played.len() > 40 {
                let _ = app.recently_played.pop_back();
            }
            let history: Vec<_> = app.recently_played.iter().cloned().collect();
            app.storage
                .save_recently_played(&history)
                .expect("Failed to save recently played");
            app.set_flash(format!("Now playing: {} ({})", song.title, song.id), 4);
            None
        }
        CoreEvent::Paused => {
            app.player_state = if app.player_state == PlayerState::Paused {
                PlayerState::Playing
            } else {
                PlayerState::Paused
            };
            app.set_flash("Toggled pause", 2);
            None
        }
        CoreEvent::Resumed => {
            app.player_state = PlayerState::Playing;
            None
        }
        CoreEvent::TrackFinished => {
            if app.repeat_mode == RepeatMode::One {
                if let Some(song) = app.current_song.clone() {
                    return Some(CoreCmd::Play(song));
                }
            }
            let next_song = if let Some(current) = app.current_song.as_ref() {
                if let Some(pos) = app.queue.iter().position(|s| s.id == current.id) {
                    app.queue.remove(pos);
                    app.queue.get(pos).cloned().or_else(|| {
                        if app.repeat_mode == RepeatMode::All {
                            app.queue.front().cloned()
                        } else {
                            None
                        }
                    })
                } else {
                    app.queue.front().cloned()
                }
            } else {
                app.queue.front().cloned()
            };

            if let Some(next_song) = next_song {
                app.selected_queue = app
                    .queue
                    .iter()
                    .position(|s| s.id == next_song.id)
                    .unwrap_or(0);
                app.set_flash(format!("Autoplay next: {}", next_song.title), 3);
                Some(CoreCmd::Play(next_song))
            } else {
                app.current_song = None;
                app.player_state = PlayerState::Idle;
                app.playback_pos = 0.0;
                app.playback_duration = 0.0;
                app.set_flash("Queue ended", 3);
                None
            }
        }
        CoreEvent::Progress { position, duration } => {
            app.playback_pos = position;
            app.playback_duration = duration;
            None
        }
        CoreEvent::VolumeChanged(volume) => {
            app.volume = volume;
            app.set_flash(format!("Volume: {volume}%"), 2);
            None
        }
        CoreEvent::MuteChanged(muted) => {
            app.muted = muted;
            app.set_flash(if muted { "Muted" } else { "Unmuted" }, 2);
            None
        }
        CoreEvent::Error(msg) => {
            app.set_flash(msg, 6);
            None
        }
        CoreEvent::LibraryRefreshDone => {
            app.scanning = false;
            if let Ok((window, offset, total)) = app
                .storage
                .fetch_local_songs_window(app.selected_local_song, 200)
            {
                app.local_library_window = window;
                app.local_library_offset = offset;
                app.local_library_total = total;
            }
            app.set_flash("Library refreshed", 3);
            None
        }
    };
    if app.active_tab == Tab::Local {
        ui_helpers::update_local_library_window(app);
    }
    cmd
}

pub fn map_plugin_action(action: PluginCoreAction) -> Option<CoreCmd> {
    match action {
        PluginCoreAction::Search { query } => Some(CoreCmd::Search(query)),
        PluginCoreAction::SearchAlbums { query } => Some(CoreCmd::SearchAlbums(query)),
        PluginCoreAction::Seek { seconds } => Some(CoreCmd::SeekBy(seconds)),
        PluginCoreAction::TogglePause => Some(CoreCmd::TogglePause),
        PluginCoreAction::ToggleMute => Some(CoreCmd::ToggleMute),
        PluginCoreAction::VolumeUp => Some(CoreCmd::VolumeUp),
        PluginCoreAction::VolumeDown => Some(CoreCmd::VolumeDown),
        PluginCoreAction::Next => Some(CoreCmd::Next),
        PluginCoreAction::Prev => Some(CoreCmd::Prev),
        PluginCoreAction::SetVolume { value } => Some(CoreCmd::SetVolume(value)),
        PluginCoreAction::PlayUrl { url, title } => Some(CoreCmd::PlayUrl { url, title }),
        PluginCoreAction::RawMpv { command } => Some(CoreCmd::RawMpv(command)),
    }
}

pub fn apply_plugin_dispatch(
    app: &mut App,
    cmd_tx: &mpsc::UnboundedSender<CoreCmd>,
    dispatch: PluginDispatch,
) -> bool {
    if let Some(tab) = dispatch.ui.set_tab {
        if let Some(core_tab) = parse_tab_name(&tab) {
            app.active_tab = core_tab;
            app.active_plugin_tab = None;
        } else if app.plugin_tabs.iter().any(|t| t.id == tab) {
            app.active_tab = Tab::Options;
            app.active_plugin_tab = Some(tab);
        }
    }
    if let Some(query) = dispatch.ui.set_search_query {
        app.search_query = query;
    }
    if let Some(query) = dispatch.ui.set_album_search_query {
        app.album_search_query = query;
    }
    if let Some(mode) = dispatch.ui.set_search_mode {
        app.search_mode = mode;
    }
    if let Some(focus) = dispatch.ui.set_focus {
        app.focus = parse_focus_name(&focus).unwrap_or(app.focus);
    }
    if let Some(index) = dispatch.ui.set_selected_result {
        app.selected_result = index.min(app.search_results.len().saturating_sub(1));
    }
    if let Some(index) = dispatch.ui.set_selected_album_result {
        let total_items: usize = app
            .album_results
            .iter()
            .enumerate()
            .map(|(i, a)| {
                1 + if app.album_expanded.get(i).copied().unwrap_or(false) {
                    a.songs.len()
                } else {
                    0
                }
            })
            .sum();
        app.selected_album_result = index.min(total_items.saturating_sub(1));
    }
    if let Some(index) = dispatch.ui.set_selected_queue {
        app.selected_queue = index.min(app.queue.len().saturating_sub(1));
    }
    if let Some(msg) = dispatch.flash {
        app.set_flash(msg, dispatch.flash_seconds.unwrap_or(4));
    }
    for action in dispatch.core_actions {
        if let Some(cmd) = map_plugin_action(action) {
            let _ = cmd_tx.send(cmd);
        }
    }
    dispatch.consume
}

pub fn plugin_event_from_core_event(event: &CoreEvent) -> PluginEvent {
    match event {
        CoreEvent::Started(song) => PluginEvent {
            kind: "started".to_owned(),
            message: Some(song.title.clone()),
            value: None,
        },
        CoreEvent::SearchDone(items) => PluginEvent {
            kind: "search_done".to_owned(),
            message: None,
            value: Some(items.len() as f64),
        },
        CoreEvent::AlbumSearchDone(items) => PluginEvent {
            kind: "album_search_done".to_owned(),
            message: None,
            value: Some(items.len() as f64),
        },
        CoreEvent::Progress { position, .. } => PluginEvent {
            kind: "progress".to_owned(),
            message: None,
            value: Some(*position),
        },
        CoreEvent::Error(msg) => PluginEvent {
            kind: "error".to_owned(),
            message: Some(msg.clone()),
            value: None,
        },
        _ => PluginEvent {
            kind: "event".to_owned(),
            message: None,
            value: None,
        },
    }
}

pub fn parse_tab_name(raw: &str) -> Option<Tab> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "discover" => Some(Tab::Discover),
        "albums" => Some(Tab::Albums),
        "library" => Some(Tab::Library),
        "options" => Some(Tab::Options),
        _ => None,
    }
}

pub fn parse_focus_name(raw: &str) -> Option<Focus> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "search" => Some(Focus::Search),
        "results" => Some(Focus::Results),
        "queue" => Some(Focus::Queue),
        _ => None,
    }
}

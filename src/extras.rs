use crate::core::CoreCmd;
use crate::model::{App, Song};
use serde_json::json;
use tokio::sync::mpsc;

const SPEED_MIN: f32 = 0.25;
const SPEED_MAX: f32 = 2.0;
const SPEED_STEP: f32 = 0.05;

pub fn set_speed(app: &mut App, cmd_tx: &mpsc::UnboundedSender<CoreCmd>, speed: f32) {
    let clamped = speed.clamp(SPEED_MIN, SPEED_MAX);
    app.playback_speed = clamped;
    let _ = cmd_tx.send(CoreCmd::RawMpv(json!(["set_property", "speed", clamped])));
}

pub fn nudge_speed(app: &mut App, cmd_tx: &mpsc::UnboundedSender<CoreCmd>, delta_steps: i32) {
    let next = app.playback_speed + delta_steps as f32 * SPEED_STEP;
    set_speed(app, cmd_tx, next);
    app.set_flash(format!("Speed {:.2}x", app.playback_speed), 2);
}

pub fn reset_speed(app: &mut App, cmd_tx: &mpsc::UnboundedSender<CoreCmd>) {
    set_speed(app, cmd_tx, 1.0);
    app.set_flash("Speed reset to 1.00x", 2);
}

pub fn toggle_mark(app: &mut App, key: String) {
    if !app.multi_select.remove(&key) {
        app.multi_select.insert(key);
    }
    if app.multi_select.is_empty() {
        app.set_flash("Selection cleared", 2);
    } else {
        app.set_flash(
            format!(
                "{} marked  (b mark · Enter add all · Esc clear)",
                app.multi_select.len()
            ),
            3,
        );
    }
}

pub fn bulk_queue_marked_discover(app: &mut App, cmd_tx: &mpsc::UnboundedSender<CoreCmd>) {
    if app.multi_select.is_empty() {
        return;
    }
    let songs: Vec<Song> = app
        .search
        .results
        .iter()
        .filter(|s| app.multi_select.contains(&s.id))
        .cloned()
        .collect();
    queue_songs(app, cmd_tx, songs);
}

pub fn bulk_queue_marked_local(app: &mut App, cmd_tx: &mpsc::UnboundedSender<CoreCmd>) {
    if app.multi_select.is_empty() {
        return;
    }
    let keys: Vec<String> = app.multi_select.iter().cloned().collect();
    let songs: Vec<Song> = app
        .storage
        .load_songs_by_paths(&keys)
        .unwrap_or_default()
        .iter()
        .map(Song::from)
        .collect();
    queue_songs(app, cmd_tx, songs);
}

pub fn queue_play_next(app: &mut App, cmd_tx: &mpsc::UnboundedSender<CoreCmd>, song: Song) {
    let was_idle = app.current_song.is_none() && app.queue.is_empty();
    if was_idle {
        app.queue.push_back(song.clone());
        app.selected_queue = 0;
        let _ = cmd_tx.send(CoreCmd::Play(song));
        app.set_flash("Playing next", 2);
        return;
    }
    let pos = app
        .current_song
        .as_ref()
        .and_then(|current| app.queue.iter().position(|s| s.id == current.id))
        .map(|idx| idx + 1)
        .unwrap_or(1)
        .min(app.queue.len());
    app.queue.insert(pos, song);
    app.set_flash(format!("Queued next: {}", app.queue[pos].title), 2);
}

pub fn move_queue_item(app: &mut App, delta: isize) {
    if app.queue.is_empty() {
        return;
    }
    let len = app.queue.len() as isize;
    let from = app.selected_queue as isize;
    let to = (from + delta).clamp(0, len - 1) as usize;
    if from as usize == to {
        return;
    }
    if let Some(song) = app.queue.remove(from as usize) {
        app.queue.insert(to, song);
        app.selected_queue = to;
    }
}

pub fn save_queue_as_playlist(app: &mut App) {
    if app.queue.is_empty() {
        app.set_flash("Queue is empty", 2);
        return;
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let name = format!("Queue {stamp}");
    let songs: Vec<Song> = app.queue.iter().cloned().collect();
    app.playlists.playlists.push(crate::model::Playlist {
        name: name.clone(),
        songs,
    });
    app.playlists.expanded.push(false);
    app.playlists.selected_playlist = app.playlists.playlists.len().saturating_sub(1);
    crate::playlist::save_playlists(app);
    app.set_flash(format!("Saved queue as {name}"), 3);
}

fn queue_songs(app: &mut App, cmd_tx: &mpsc::UnboundedSender<CoreCmd>, songs: Vec<Song>) {
    let was_idle = app.current_song.is_none();
    let mut added = 0usize;
    for song in songs {
        app.queue.push_back(song.clone());
        added += 1;
        if was_idle && added == 1 {
            let _ = cmd_tx.send(CoreCmd::Play(song));
        }
    }
    app.selected_queue = app.queue.len().saturating_sub(1);
    app.multi_select.clear();
    app.set_flash(format!("Added {added} tracks to queue"), 3);
}

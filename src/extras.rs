use serde_json::json;
use tokio::sync::mpsc;
use crate::core::CoreCmd;
use crate::model::{App, Song};

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
    let library = app.storage.load_local_library().unwrap_or_default();
    let songs: Vec<Song> = library
        .iter()
        .filter(|ls| app.multi_select.contains(&ls.path))
        .map(Song::from)
        .collect();
    queue_songs(app, cmd_tx, songs);
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

use tokio::sync::mpsc;
use crate::core::CoreCmd;
use crate::model::{App, Tab};

#[derive(Debug, Clone, Copy)]
pub enum Action {
    TogglePause,
    Next,
    Prev,
    ToggleMute,
    VolumeUp,
    VolumeDown,
    CycleRepeat,
    ShuffleQueue,
    SeekForward,
    SeekBack,
    SpeedUp,
    SpeedDown,
    SpeedReset,
    ToggleEq,
    GoToTab(Tab),
    Quit,
}

pub struct PaletteCommand {
    pub name: &'static str,
    pub hint: &'static str,
    pub action: Action,
}

pub fn all_commands() -> Vec<PaletteCommand> {
    vec![
        PaletteCommand { name: "play / pause", hint: "toggle playback", action: Action::TogglePause },
        PaletteCommand { name: "next track", hint: "skip to next in queue", action: Action::Next },
        PaletteCommand { name: "previous track", hint: "go to previous", action: Action::Prev },
        PaletteCommand { name: "mute", hint: "toggle mute", action: Action::ToggleMute },
        PaletteCommand { name: "volume up", hint: "+5", action: Action::VolumeUp },
        PaletteCommand { name: "volume down", hint: "-5", action: Action::VolumeDown },
        PaletteCommand { name: "repeat mode", hint: "cycle off / one / all", action: Action::CycleRepeat },
        PaletteCommand { name: "shuffle queue", hint: "keep current, shuffle rest", action: Action::ShuffleQueue },
        PaletteCommand { name: "seek forward", hint: "+10s", action: Action::SeekForward },
        PaletteCommand { name: "seek back", hint: "-10s", action: Action::SeekBack },
        PaletteCommand { name: "speed up", hint: "+0.05x playback speed", action: Action::SpeedUp },
        PaletteCommand { name: "speed down", hint: "-0.05x playback speed", action: Action::SpeedDown },
        PaletteCommand { name: "speed reset", hint: "back to 1.00x", action: Action::SpeedReset },
        PaletteCommand { name: "equalizer", hint: "toggle on/off", action: Action::ToggleEq },
        PaletteCommand { name: "go to discover", hint: "switch tab", action: Action::GoToTab(Tab::Discover) },
        PaletteCommand { name: "go to albums", hint: "switch tab", action: Action::GoToTab(Tab::Albums) },
        PaletteCommand { name: "go to library", hint: "playlists", action: Action::GoToTab(Tab::Library) },
        PaletteCommand { name: "go to local", hint: "local library", action: Action::GoToTab(Tab::Local) },
        PaletteCommand { name: "go to options", hint: "settings", action: Action::GoToTab(Tab::Options) },
        PaletteCommand { name: "quit", hint: "close rs-pug", action: Action::Quit },
    ]
}

pub fn filter_commands(query: &str) -> Vec<PaletteCommand> {
    let q = query.trim().to_lowercase();
    let mut cmds = all_commands();
    if q.is_empty() {
        return cmds;
    }
    cmds.retain(|c| {
        c.name.to_lowercase().contains(&q) || c.hint.to_lowercase().contains(&q)
    });
    cmds.sort_by_key(|c| !c.name.to_lowercase().starts_with(&q));
    cmds
}

pub fn dispatch(app: &mut App, cmd_tx: &mpsc::UnboundedSender<CoreCmd>, action: Action) -> bool {
    match action {
        Action::TogglePause => {
            let _ = cmd_tx.send(CoreCmd::TogglePause);
        }
        Action::Next => {
            if app.queue.len() > 1 {
                if let Some(song) = app.queue.pop_front() {
                    app.queue.push_back(song);
                }
                if let Some(next_song) = app.queue.front().cloned() {
                    let _ = cmd_tx.send(CoreCmd::Play(next_song));
                }
            } else {
                let _ = cmd_tx.send(CoreCmd::Next);
            }
        }
        Action::Prev => {
            if app.queue.len() > 1 {
                if let Some(song) = app.queue.pop_back() {
                    app.queue.push_front(song);
                }
                if let Some(song) = app.queue.front().cloned() {
                    let _ = cmd_tx.send(CoreCmd::Play(song));
                }
            } else {
                let _ = cmd_tx.send(CoreCmd::Prev);
            }
        }
        Action::ToggleMute => {
            let _ = cmd_tx.send(CoreCmd::ToggleMute);
        }
        Action::VolumeUp => {
            let _ = cmd_tx.send(CoreCmd::VolumeUp);
        }
        Action::VolumeDown => {
            let _ = cmd_tx.send(CoreCmd::VolumeDown);
        }
        Action::CycleRepeat => {
            app.repeat_mode = app.repeat_mode.next();
            app.set_flash(format!("Repeat mode: {}", app.repeat_mode.label()), 2);
        }
        Action::ShuffleQueue => {
            crate::ui_helpers::shuffle_queue_keep_current(app);
        }
        Action::SeekForward => {
            let _ = cmd_tx.send(CoreCmd::SeekBy(10));
        }
        Action::SeekBack => {
            let _ = cmd_tx.send(CoreCmd::SeekBy(-10));
        }
        Action::SpeedUp => crate::extras::nudge_speed(app, cmd_tx, 1),
        Action::SpeedDown => crate::extras::nudge_speed(app, cmd_tx, -1),
        Action::SpeedReset => crate::extras::reset_speed(app, cmd_tx),
        Action::ToggleEq => {
            app.eq.enabled = !app.eq.enabled;
            if app.eq.enabled {
                crate::eq::send_eq_update(cmd_tx, app.eq.bands);
                app.set_flash("Equalizer ON", 2);
            } else {
                crate::eq::send_eq_update(cmd_tx, [0.0f32; 10]);
                app.set_flash("Equalizer OFF", 2);
            }
        }
        Action::GoToTab(tab) => {
            app.active_tab = tab;
            app.plugin_ui.active_tab = None;
            app.plugin_ui.active_custom_tab = None;
        }
        Action::Quit => {
            let _ = cmd_tx.send(CoreCmd::Quit);
            return false;
        }
    }
    true
}

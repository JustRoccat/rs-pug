use crate::core::CoreCmd;
use crate::model::{App, Tab};
use tokio::sync::mpsc;

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
    MoveQueueUp,
    MoveQueueDown,
    SaveQueueAsPlaylist,
    SeekForward,
    SeekBack,
    SpeedUp,
    SpeedDown,
    SpeedReset,
    ToggleEq,
    ToggleMinimal,
    ToggleImageBackground,
    ToggleIcons,
    ToggleNormalize,
    CycleReplaygain,
    ToggleGapless,
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
        PaletteCommand {
            name: "play / pause",
            hint: "toggle playback",
            action: Action::TogglePause,
        },
        PaletteCommand {
            name: "next track",
            hint: "skip to next in queue",
            action: Action::Next,
        },
        PaletteCommand {
            name: "previous track",
            hint: "go to previous",
            action: Action::Prev,
        },
        PaletteCommand {
            name: "mute",
            hint: "toggle mute",
            action: Action::ToggleMute,
        },
        PaletteCommand {
            name: "volume up",
            hint: "+5",
            action: Action::VolumeUp,
        },
        PaletteCommand {
            name: "volume down",
            hint: "-5",
            action: Action::VolumeDown,
        },
        PaletteCommand {
            name: "repeat mode",
            hint: "cycle off / one / all",
            action: Action::CycleRepeat,
        },
        PaletteCommand {
            name: "shuffle queue",
            hint: "keep current, shuffle rest",
            action: Action::ShuffleQueue,
        },
        PaletteCommand {
            name: "move queue up",
            hint: "move selected up",
            action: Action::MoveQueueUp,
        },
        PaletteCommand {
            name: "move queue down",
            hint: "move selected down",
            action: Action::MoveQueueDown,
        },
        PaletteCommand {
            name: "save queue as playlist",
            hint: "store queue",
            action: Action::SaveQueueAsPlaylist,
        },
        PaletteCommand {
            name: "seek forward",
            hint: "+10s",
            action: Action::SeekForward,
        },
        PaletteCommand {
            name: "seek back",
            hint: "-10s",
            action: Action::SeekBack,
        },
        PaletteCommand {
            name: "speed up",
            hint: "+0.05x playback speed",
            action: Action::SpeedUp,
        },
        PaletteCommand {
            name: "speed down",
            hint: "-0.05x playback speed",
            action: Action::SpeedDown,
        },
        PaletteCommand {
            name: "speed reset",
            hint: "back to 1.00x",
            action: Action::SpeedReset,
        },
        PaletteCommand {
            name: "equalizer",
            hint: "toggle on/off",
            action: Action::ToggleEq,
        },
        PaletteCommand {
            name: "normalize audio",
            hint: "toggle loudnorm filter",
            action: Action::ToggleNormalize,
        },
        PaletteCommand {
            name: "replaygain",
            hint: "cycle off / track / album",
            action: Action::CycleReplaygain,
        },
        PaletteCommand {
            name: "gapless",
            hint: "toggle gapless playback",
            action: Action::ToggleGapless,
        },
        PaletteCommand {
            name: "minimal",
            hint: "toggle minimal view (Shift+Z)",
            action: Action::ToggleMinimal,
        },
        PaletteCommand {
            name: "image background",
            hint: "toggle minimal cover tint",
            action: Action::ToggleImageBackground,
        },
        PaletteCommand {
            name: "icons",
            hint: "toggle nerdfonts icons",
            action: Action::ToggleIcons,
        },
        PaletteCommand {
            name: "go to discover",
            hint: "switch tab",
            action: Action::GoToTab(Tab::Discover),
        },
        PaletteCommand {
            name: "go to albums",
            hint: "switch tab",
            action: Action::GoToTab(Tab::Albums),
        },
        PaletteCommand {
            name: "go to library",
            hint: "playlists",
            action: Action::GoToTab(Tab::Library),
        },
        PaletteCommand {
            name: "go to local",
            hint: "local library",
            action: Action::GoToTab(Tab::Local),
        },
        PaletteCommand {
            name: "go to lyrics",
            hint: "lyrics view",
            action: Action::GoToTab(Tab::Lyrics),
        },
        PaletteCommand {
            name: "go to options",
            hint: "settings",
            action: Action::GoToTab(Tab::Options),
        },
        PaletteCommand {
            name: "quit",
            hint: "close rs-pug",
            action: Action::Quit,
        },
    ]
}

pub fn filter_commands(query: &str) -> Vec<PaletteCommand> {
    let q = query.trim().to_lowercase();
    let mut cmds = all_commands();
    if q.is_empty() {
        return cmds;
    }
    cmds.retain(|c| c.name.to_lowercase().contains(&q) || c.hint.to_lowercase().contains(&q));
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
        Action::MoveQueueUp => {
            crate::extras::move_queue_item(app, -1);
        }
        Action::MoveQueueDown => {
            crate::extras::move_queue_item(app, 1);
        }
        Action::SaveQueueAsPlaylist => {
            crate::extras::save_queue_as_playlist(app);
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
            crate::eq::send_audio_update(app, cmd_tx);
            app.set_flash(
                if app.eq.enabled {
                    "Equalizer ON"
                } else {
                    "Equalizer OFF"
                },
                2,
            );
        }
        Action::ToggleNormalize => {
            app.opt_audio_normalize = !app.opt_audio_normalize;
            crate::eq::send_audio_update(app, cmd_tx);
            app.set_flash(
                format!(
                    "Normalize: {}",
                    if app.opt_audio_normalize { "ON" } else { "OFF" }
                ),
                2,
            );
        }
        Action::CycleReplaygain => {
            app.opt_replaygain_mode = crate::config::cycle_replaygain(&app.opt_replaygain_mode, 1);
            let _ = cmd_tx.send(crate::core::CoreCmd::RawMpv(serde_json::json!([
                "set_property",
                "replaygain",
                crate::config::mpv_replaygain(&app.opt_replaygain_mode)
            ])));
            app.set_flash(format!("Replaygain: {}", app.opt_replaygain_mode), 2);
        }
        Action::ToggleGapless => {
            app.opt_gapless = !app.opt_gapless;
            let _ = cmd_tx.send(crate::core::CoreCmd::RawMpv(serde_json::json!([
                "set_property",
                "gapless-audio",
                app.opt_gapless
            ])));
            app.set_flash(
                format!("Gapless: {}", if app.opt_gapless { "ON" } else { "OFF" }),
                2,
            );
        }
        Action::ToggleMinimal => {
            app.toggle_minimal();
        }
        Action::ToggleImageBackground => {
            app.opt_image_background = !app.opt_image_background;
            app.set_flash(
                format!(
                    "Image background: {}",
                    if app.opt_image_background {
                        "ON"
                    } else {
                        "OFF"
                    }
                ),
                2,
            );
        }
        Action::ToggleIcons => {
            app.opt_icons = !app.opt_icons;
            app.set_flash(
                format!(
                    "Icons: {}",
                    if app.opt_icons {
                        "ON (nerdfonts)"
                    } else {
                        "OFF (plain text)"
                    }
                ),
                2,
            );
        }
        Action::GoToTab(tab) => {
            if tab == Tab::Albums && !app.albums_available() {
                app.set_flash("Albums are unavailable for custom search sources", 3);
                return true;
            }
            app.active_tab = tab;
            app.plugin_ui.active_tab = None;
            app.plugin_ui.active_custom_tab = None;
            app.plugin_ui.pending_plugin_tab = None;
        }
        Action::Quit => {
            let _ = cmd_tx.send(CoreCmd::Quit);
            return false;
        }
    }
    true
}

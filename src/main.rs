use std::io::{self, Stdout};
use std::time::Duration;

use anyhow::Result;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
        MouseButton, MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{prelude::*, Terminal};
use serde_json::json;
use tokio::sync::mpsc;

mod config;
mod core;
mod model;
mod plugins;
mod storage;
mod tui;

use config::{load_config, save_config};
use core::{Core, CoreCmd, CoreEvent};
use model::{
    eq_preset_bands, eq_preset_name, App, Focus, PlayerState, Playlist, RepeatMode, Tab,
    EQ_PRESET_NAMES,
};
use plugins::{PluginCoreAction, PluginDispatch, PluginEvent, PluginManager, PluginUiState};
use storage::{
    export_playlist_to_default, import_playlist_from_default, load_playlists, load_recently_played,
    save_playlists, save_recently_played,
};

#[tokio::main]
async fn main() -> Result<()> {
    let config = load_config();
    let plugin_manager = PluginManager::load(
        config.general.plugins_enabled,
        config.general.plugins_dir.as_str(),
    );

    let mut terminal = setup_terminal()?;
    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
    let (evt_tx, mut evt_rx) = mpsc::unbounded_channel();

    let core = Core::new(config.clone()).await?;
    tokio::spawn(core.run(cmd_rx, evt_tx));

    let mut app = App::new();
    app.apply_config(&config);
    app.playlists = load_playlists();
    app.playlist_expanded = vec![false; app.playlists.len()];
    app.recently_played = load_recently_played().into();

    let tick_rate = Duration::from_millis(20);
    let mut running = true;

    while running {
        ensure_playlist_state(&mut app);
        app.anim_tick = app.anim_tick.wrapping_add(1);
        terminal.draw(|frame| tui::draw(frame, &app))?;

        while let Ok(event) = evt_rx.try_recv() {
            let plugin_event = plugin_event_from_core_event(&event);
            if let Some(cmd) = apply_event(&mut app, event) {
                let _ = cmd_tx.send(cmd);
            }
            let dispatch = plugin_manager.dispatch_event(
                &plugin_event,
                &PluginUiState::from_runtime(
                    app.active_tab,
                    player_state_label(app.player_state),
                    app.volume,
                    app.muted,
                    app.repeat_mode,
                    app.search_query.clone(),
                    app.album_search_query.clone(),
                    app.queue.len(),
                ),
            );
            if apply_plugin_dispatch(&mut app, &cmd_tx, dispatch) {
                continue;
            }
        }

        if event::poll(tick_rate)? {
            match event::read()? {
                Event::Mouse(mouse) => match mouse.kind {
                    MouseEventKind::ScrollDown => scroll_selection(&mut app, 3),
                    MouseEventKind::ScrollUp => scroll_selection(&mut app, -3),
                    MouseEventKind::Down(MouseButton::Left) => {
                        if mouse.row <= 2 {
                            app.active_tab = if mouse.column < 18 {
                                Tab::Discover
                            } else if mouse.column < 30 {
                                Tab::Albums
                            } else if mouse.column < 44 {
                                Tab::Library
                            } else {
                                Tab::Options
                            };
                        }
                    }
                    _ => {}
                },
                Event::Key(key) => {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }

                    if key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        break;
                    }

                    if app.context_open {
                        match key.code {
                            KeyCode::Esc => app.context_open = false,
                            KeyCode::Char('j') | KeyCode::Down => {
                                app.context_index = (app.context_index + 1)
                                    .min(context_menu_len(&app).saturating_sub(1));
                            }
                            KeyCode::Char('k') | KeyCode::Up => {
                                app.context_index = app.context_index.saturating_sub(1);
                            }
                            KeyCode::Enter => {
                                let idx = app.context_index;
                                execute_context_action(&mut app, idx);
                                app.context_open = false;
                            }
                            _ => {}
                        }
                        continue;
                    }
                    if app.confirm_delete_playlist {
                        match key.code {
                            KeyCode::Char('y') | KeyCode::Enter => {
                                if app.selected_playlist < app.playlists.len() {
                                    let deleted = app.playlists.remove(app.selected_playlist);
                                    if app.selected_playlist < app.playlist_expanded.len() {
                                        app.playlist_expanded.remove(app.selected_playlist);
                                    }
                                    app.selected_playlist = app
                                        .selected_playlist
                                        .min(app.playlists.len().saturating_sub(1));
                                    save_playlists(&app.playlists);
                                    app.set_flash(format!("Deleted {}", deleted.name), 3);
                                }
                                app.confirm_delete_playlist = false;
                            }
                            KeyCode::Char('n') | KeyCode::Esc => {
                                app.confirm_delete_playlist = false;
                                app.set_flash("Delete canceled", 2);
                            }
                            _ => {}
                        }
                        continue;
                    }

                    if app.search_mode {
                        match key.code {
                            KeyCode::Esc => app.search_mode = false,
                            KeyCode::Enter => {
                                let q = if app.active_tab == Tab::Albums {
                                    app.album_search_query.trim().to_owned()
                                } else {
                                    app.search_query.trim().to_owned()
                                };
                                if !q.is_empty() {
                                    app.player_state = PlayerState::Searching;
                                    let _ = if app.active_tab == Tab::Albums {
                                        cmd_tx.send(CoreCmd::SearchAlbums(q))
                                    } else {
                                        cmd_tx.send(CoreCmd::Search(q))
                                    };
                                }
                                app.search_mode = false;
                            }
                            KeyCode::Backspace => {
                                if app.active_tab == Tab::Albums {
                                    app.album_search_query.pop();
                                } else {
                                    app.search_query.pop();
                                }
                            }
                            KeyCode::Char(c) => {
                                if app.active_tab == Tab::Albums {
                                    app.album_search_query.push(c);
                                } else {
                                    app.search_query.push(c);
                                }
                            }
                            _ => {}
                        }
                        continue;
                    }

                    let key_label = describe_key_event(&key.code);
                    let plugin_dispatch = plugin_manager.dispatch_key(
                        key_label.as_str(),
                        &PluginUiState::from_runtime(
                            app.active_tab,
                            player_state_label(app.player_state),
                            app.volume,
                            app.muted,
                            app.repeat_mode,
                            app.search_query.clone(),
                            app.album_search_query.clone(),
                            app.queue.len(),
                        ),
                    );
                    if apply_plugin_dispatch(&mut app, &cmd_tx, plugin_dispatch) {
                        continue;
                    }

                    match key.code {
                        KeyCode::Char('1') => app.active_tab = Tab::Discover,
                        KeyCode::Char('2') => app.active_tab = Tab::Albums,
                        KeyCode::Char('3') => app.active_tab = Tab::Library,
                        KeyCode::Char('4') => app.active_tab = Tab::Options,
                        KeyCode::Char('a') if app.active_tab == Tab::Library => {
                            let name = format!("Playlist {}", app.playlists.len() + 1);
                            app.playlists.push(Playlist {
                                name: name.clone(),
                                songs: Vec::new(),
                            });
                            app.playlist_expanded.push(true);
                            app.selected_playlist = app.playlists.len().saturating_sub(1);
                            save_playlists(&app.playlists);
                            app.set_flash(format!("Created {name}"), 3);
                        }
                        KeyCode::Char('x') if app.active_tab == Tab::Library => {
                            if app.selected_playlist < app.playlists.len() {
                                app.confirm_delete_playlist = true;
                                app.delete_playlist_name =
                                    app.playlists[app.selected_playlist].name.clone();
                            }
                        }
                        KeyCode::Char('e') if app.active_tab == Tab::Library => {
                            if let Some(open) = app.playlist_expanded.get_mut(app.selected_playlist)
                            {
                                *open = !*open;
                            }
                        }
                        KeyCode::Char('c') => {
                            let can_open =
                                if app.active_tab == Tab::Library && app.focus == Focus::Results {
                                    true
                                } else {
                                    app.selected_song_for_context().is_some()
                                };
                            if can_open {
                                app.context_open = true;
                                app.context_index = 0;
                            }
                        }
                        KeyCode::Char('/') => {
                            if !matches!(app.active_tab, Tab::Discover | Tab::Albums) {
                                continue;
                            }
                            app.search_mode = true;
                            app.focus = Focus::Search;
                        }
                        KeyCode::Char('j') | KeyCode::Down => match app.focus {
                            Focus::Results => {
                                if app.active_tab == Tab::Options {
                                    app.options_index = (app.options_index + 1).min(9);
                                } else if app.active_tab == Tab::Library {
                                    if !app.playlists.is_empty() {
                                        app.selected_playlist = (app.selected_playlist + 1)
                                            .min(app.playlists.len().saturating_sub(1));
                                        app.selected_playlist_song = 0;
                                    }
                                } else if app.active_tab == Tab::Albums {
                                    if !app.album_results.is_empty() {
                                        app.selected_album_result = (app.selected_album_result + 1)
                                            .min(app.album_results.len().saturating_sub(1));
                                    }
                                } else if !app.search_results.is_empty() {
                                    app.selected_result = (app.selected_result + 1)
                                        .min(app.search_results.len().saturating_sub(1));
                                }
                            }
                            Focus::Queue => {
                                if app.active_tab == Tab::Library {
                                    if let Some(playlist) = app.playlists.get(app.selected_playlist)
                                    {
                                        if !playlist.songs.is_empty() {
                                            app.selected_playlist_song =
                                                (app.selected_playlist_song + 1)
                                                    .min(playlist.songs.len().saturating_sub(1));
                                        }
                                    }
                                } else if !app.queue.is_empty() {
                                    app.selected_queue = (app.selected_queue + 1)
                                        .min(app.queue.len().saturating_sub(1));
                                }
                            }
                            Focus::Search => app.focus = Focus::Results,
                        },
                        KeyCode::Char('k') | KeyCode::Up => match app.focus {
                            Focus::Results => {
                                if app.active_tab == Tab::Options {
                                    app.options_index = app.options_index.saturating_sub(1);
                                } else if app.active_tab == Tab::Library {
                                    app.selected_playlist = app.selected_playlist.saturating_sub(1);
                                    app.selected_playlist_song = 0;
                                } else if app.active_tab == Tab::Albums {
                                    app.selected_album_result =
                                        app.selected_album_result.saturating_sub(1);
                                } else {
                                    app.selected_result = app.selected_result.saturating_sub(1);
                                }
                            }
                            Focus::Queue => {
                                if app.active_tab == Tab::Library {
                                    app.selected_playlist_song =
                                        app.selected_playlist_song.saturating_sub(1);
                                } else {
                                    app.selected_queue = app.selected_queue.saturating_sub(1);
                                }
                            }
                            Focus::Search => app.focus = Focus::Results,
                        },
                        KeyCode::PageDown => scroll_selection(&mut app, 10),
                        KeyCode::PageUp => scroll_selection(&mut app, -10),
                        KeyCode::Tab => {
                            if app.active_tab == Tab::Options {
                                continue;
                            }
                            app.focus = match app.focus {
                                Focus::Search | Focus::Results => Focus::Queue,
                                Focus::Queue => Focus::Results,
                            };
                        }
                        KeyCode::Enter => {
                            if app.active_tab == Tab::Options {
                                if app.options_index == 2 {
                                    if let Some(current) = app.current_song.clone() {
                                        app.player_state = PlayerState::Searching;
                                        app.set_flash("Smart Queue: searching similar song...", 3);
                                        let _ = cmd_tx.send(CoreCmd::SmartQueue(current));
                                    } else {
                                        app.set_flash(
                                            "Smart Queue needs a currently playing song",
                                            3,
                                        );
                                    }
                                } else if app.options_index == 5 {
                                    app.eq_enabled = !app.eq_enabled;
                                    if app.eq_enabled {
                                        send_eq_update(&cmd_tx, app.eq_bands);
                                        app.set_flash("Equalizer ON", 2);
                                    } else {
                                        send_eq_update(&cmd_tx, [0.0f32; 10]);
                                        app.set_flash("Equalizer OFF", 2);
                                    }
                                } else if app.options_index == 6 {
                                    cycle_eq_preset(&mut app, &cmd_tx, 1);
                                }
                                continue;
                            }
                            match app.focus {
                                Focus::Results if app.active_tab == Tab::Discover => {
                                    if let Some(song) = app.current_selection().cloned() {
                                        app.queue.push_back(song.clone());
                                        app.selected_queue = app.queue.len().saturating_sub(1);
                                        let _ = cmd_tx.send(CoreCmd::Play(song));
                                    }
                                }
                                Focus::Results if app.active_tab == Tab::Albums => {
                                    if let Some(song) =
                                        app.album_results.get(app.selected_album_result).cloned()
                                    {
                                        app.queue.push_back(song.clone());
                                        app.selected_queue = app.queue.len().saturating_sub(1);
                                        let _ = cmd_tx.send(CoreCmd::Play(song));
                                    }
                                }
                                Focus::Results if app.active_tab == Tab::Library => {
                                    if let Some(playlist) = app.playlists.get(app.selected_playlist)
                                    {
                                        app.queue.clear();
                                        for s in &playlist.songs {
                                            app.queue.push_back(s.clone());
                                        }
                                        if let Some(first) = app.queue.front().cloned() {
                                            let _ = cmd_tx.send(CoreCmd::Play(first));
                                        }
                                    }
                                }
                                Focus::Queue => {
                                    if app.active_tab == Tab::Library {
                                        if let Some(playlist) =
                                            app.playlists.get(app.selected_playlist)
                                        {
                                            if !playlist.songs.is_empty() {
                                                app.queue.clear();
                                                let start = app
                                                    .selected_playlist_song
                                                    .min(playlist.songs.len().saturating_sub(1));
                                                for s in playlist.songs.iter().skip(start) {
                                                    app.queue.push_back(s.clone());
                                                }
                                                if let Some(song) = app.queue.front().cloned() {
                                                    app.selected_queue = 0;
                                                    let _ = cmd_tx.send(CoreCmd::Play(song));
                                                }
                                            }
                                        }
                                    } else if let Some(song) = app.queue_selection().cloned() {
                                        if app.selected_queue < app.queue.len() {
                                            app.queue.rotate_left(app.selected_queue);
                                            app.selected_queue = 0;
                                        }
                                        let _ = cmd_tx.send(CoreCmd::Play(song));
                                    }
                                }
                                Focus::Results => {}
                                Focus::Search => {}
                            }
                        }
                        KeyCode::Char(' ') => {
                            let _ = cmd_tx.send(CoreCmd::TogglePause);
                        }
                        KeyCode::Left if app.active_tab != Tab::Options => {
                            let _ = cmd_tx.send(CoreCmd::SeekBy(-10));
                        }
                        KeyCode::Right if app.active_tab != Tab::Options => {
                            let _ = cmd_tx.send(CoreCmd::SeekBy(10));
                        }
                        KeyCode::Char('0')
                            if !(app.active_tab == Tab::Options && app.options_index == 5) =>
                        {
                            let _ = cmd_tx.send(CoreCmd::VolumeUp);
                        }
                        KeyCode::Char('9') => {
                            let _ = cmd_tx.send(CoreCmd::VolumeDown);
                        }
                        KeyCode::Char(c) if c == app.key_next => {
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
                        KeyCode::Char(c) if c == app.key_prev => {
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
                        KeyCode::Char(c) if c == app.key_mute => {
                            let _ = cmd_tx.send(CoreCmd::ToggleMute);
                        }
                        KeyCode::Char(c) if c == app.key_repeat => {
                            app.repeat_mode = app.repeat_mode.next();
                            app.set_flash(format!("Repeat mode: {}", app.repeat_mode.label()), 2);
                        }
                        KeyCode::Char(c) if c == app.key_shuffle => {
                            shuffle_queue_keep_current(&mut app);
                        }
                        KeyCode::Char(c) if c == app.key_seek_back => {
                            let _ = cmd_tx.send(CoreCmd::SeekBy(-10));
                        }
                        KeyCode::Char(c) if c == app.key_seek_forward => {
                            let _ = cmd_tx.send(CoreCmd::SeekBy(10));
                        }
                        KeyCode::Char('q') => {
                            running = false;
                        }
                        KeyCode::Char('d') => {
                            if app.active_tab == Tab::Library && app.focus == Focus::Queue {
                                remove_selected_playlist_song(&mut app);
                                save_playlists(&app.playlists);
                            } else if app.focus == Focus::Queue {
                                remove_selected_queue_song(&mut app);
                            }
                        }
                        KeyCode::Char('i')
                            if app.active_tab == Tab::Library && app.focus == Focus::Results =>
                        {
                            import_playlist_action(&mut app);
                        }
                        KeyCode::Char('e')
                            if app.active_tab == Tab::Library && app.focus == Focus::Results =>
                        {
                            export_selected_playlist_action(&mut app);
                        }
                        KeyCode::Char('h') | KeyCode::Left if app.active_tab == Tab::Options => {
                            match app.options_index {
                                0 => {
                                    app.opt_search_limit =
                                        app.opt_search_limit.saturating_sub(1).max(1)
                                }
                                1 => app.opt_socket = "/tmp/rs-pug.sock".to_owned(),
                                3 => app.opt_theme = prev_theme(app.opt_theme),
                                4 => {
                                    app.repeat_mode = prev_repeat_mode(app.repeat_mode);
                                    app.set_flash(
                                        format!("Repeat mode: {}", app.repeat_mode.label()),
                                        2,
                                    );
                                }
                                5 => {
                                    if app.eq_focus_band > 0 {
                                        app.eq_focus_band -= 1;
                                    }
                                }
                                6 => cycle_eq_preset(&mut app, &cmd_tx, -1),
                                7 => app.key_next = cycle_keybind_char(app.key_next, -1),
                                8 => app.key_prev = cycle_keybind_char(app.key_prev, -1),
                                9 => app.key_mute = cycle_keybind_char(app.key_mute, -1),
                                _ => {}
                            }
                        }
                        KeyCode::Char('l') | KeyCode::Right if app.active_tab == Tab::Options => {
                            match app.options_index {
                                0 => app.opt_search_limit = (app.opt_search_limit + 1).min(50),
                                1 => app.opt_socket = "/tmp/rs-pug.sock".to_owned(),
                                3 => app.opt_theme = next_theme(app.opt_theme),
                                4 => {
                                    app.repeat_mode = app.repeat_mode.next();
                                    app.set_flash(
                                        format!("Repeat mode: {}", app.repeat_mode.label()),
                                        2,
                                    );
                                }
                                5 => {
                                    if app.eq_focus_band < 9 {
                                        app.eq_focus_band += 1;
                                    }
                                }
                                6 => cycle_eq_preset(&mut app, &cmd_tx, 1),
                                7 => app.key_next = cycle_keybind_char(app.key_next, 1),
                                8 => app.key_prev = cycle_keybind_char(app.key_prev, 1),
                                9 => app.key_mute = cycle_keybind_char(app.key_mute, 1),
                                _ => {}
                            }
                        }
                        KeyCode::Char('p') if app.active_tab == Tab::Options => {
                            cycle_eq_preset(&mut app, &cmd_tx, 1);
                        }
                        KeyCode::Char('s') if app.active_tab == Tab::Options => {
                            save_config(&app.build_config());
                            app.theme = app.opt_theme;
                            app.set_flash("Saved settings to ~/.config/rs-pug/config.toml", 4);
                        }
                        KeyCode::Char('+') | KeyCode::Char('=')
                            if app.active_tab == Tab::Options && app.options_index == 5 =>
                        {
                            let b = app.eq_focus_band;
                            app.eq_bands[b] = (app.eq_bands[b] + 1.0).min(12.0);
                            if app.eq_enabled {
                                send_eq_update(&cmd_tx, app.eq_bands);
                            }
                        }
                        KeyCode::Char('-')
                            if app.active_tab == Tab::Options && app.options_index == 5 =>
                        {
                            let b = app.eq_focus_band;
                            app.eq_bands[b] = (app.eq_bands[b] - 1.0).max(-12.0);
                            if app.eq_enabled {
                                send_eq_update(&cmd_tx, app.eq_bands);
                            }
                        }
                        KeyCode::Char('0')
                            if app.active_tab == Tab::Options && app.options_index == 5 =>
                        {
                            app.eq_bands = [0.0f32; 10];
                            app.eq_preset_index = 0;
                            if app.eq_enabled {
                                send_eq_update(&cmd_tx, app.eq_bands);
                            }
                            app.set_flash("Equalizer reset to Flat", 2);
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }

    let _ = cmd_tx.send(CoreCmd::Quit);
    restore_terminal(terminal)?;
    Ok(())
}

fn add_to_named_playlist(app: &mut App, song: model::Song, name: &str) {
    if let Some(p) = app.playlists.iter_mut().find(|p| p.name == name) {
        p.songs.push(song);
    } else {
        app.playlists.push(Playlist {
            name: name.to_owned(),
            songs: vec![song],
        });
        app.playlist_expanded.push(true);
        app.selected_playlist = app.playlists.len().saturating_sub(1);
    }
    app.set_flash(format!("Added to playlist: {name}"), 3);
}

fn shuffle_queue_keep_current(app: &mut App) {
    if app.queue.len() < 2 {
        return;
    }
    let keep = app.current_song.as_ref().map(|s| s.id.clone());
    let mut items: Vec<_> = app.queue.iter().cloned().collect();
    pseudo_shuffle(&mut items);
    if let Some(id) = keep {
        if let Some(pos) = items.iter().position(|s| s.id == id) {
            let current = items.remove(pos);
            items.insert(0, current);
        }
    }
    app.queue = items.into();
    app.selected_queue = 0;
    app.set_flash("Queue shuffled", 2);
}

fn pseudo_shuffle<T>(items: &mut [T]) {
    if items.len() < 2 {
        return;
    }
    let mut seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9E3779B97F4A7C15);
    for i in (1..items.len()).rev() {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let j = (seed as usize) % (i + 1);
        items.swap(i, j);
    }
}

fn cycle_eq_preset(app: &mut App, cmd_tx: &mpsc::UnboundedSender<CoreCmd>, delta: isize) {
    let total = EQ_PRESET_NAMES.len() as isize;
    let next = (app.eq_preset_index as isize + delta).rem_euclid(total) as usize;
    app.eq_preset_index = next;
    app.eq_bands = eq_preset_bands(next);
    if app.eq_enabled {
        send_eq_update(cmd_tx, app.eq_bands);
    }
    app.set_flash(format!("EQ preset: {}", eq_preset_name(next)), 2);
}

fn send_eq_update(cmd_tx: &mpsc::UnboundedSender<CoreCmd>, bands: [f32; 10]) {
    let freqs = [32, 64, 125, 250, 500, 1000, 2000, 4000, 8000, 16000];
    let parts: Vec<String> = bands
        .iter()
        .zip(freqs.iter())
        .map(|(gain, freq)| {
            format!(
                "equalizer=frequency={}:gain={}:width_type=o:width=1.5",
                freq, gain
            )
        })
        .collect();
    let filter = parts.join(",");
    let _ = cmd_tx.send(CoreCmd::RawMpv(
        json!({"command": ["set_property", "af", filter]}),
    ));
}

fn cycle_keybind_char(current: char, delta: isize) -> char {
    const POOL: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789[]-=/;',.";
    let pos = POOL
        .iter()
        .position(|c| *c as char == current.to_ascii_lowercase())
        .unwrap_or(0) as isize;
    let next = (pos + delta).rem_euclid(POOL.len() as isize) as usize;
    POOL[next] as char
}

fn import_playlist_action(app: &mut App) {
    match import_playlist_from_default() {
        Ok(mut imported) => {
            if imported.name.trim().is_empty() {
                imported.name = "Imported".to_owned();
            }
            if let Some(existing) = app.playlists.iter_mut().find(|p| p.name == imported.name) {
                let before = existing.songs.len();
                existing.songs.extend(imported.songs);
                let merged_name = existing.name.clone();
                let added = existing.songs.len().saturating_sub(before);
                app.set_flash(
                    format!("Imported into {} (+{} tracks)", merged_name, added),
                    4,
                );
            } else {
                app.playlists.push(imported.clone());
                app.playlist_expanded.push(true);
                app.selected_playlist = app.playlists.len().saturating_sub(1);
                app.set_flash(format!("Imported playlist {}", imported.name), 4);
            }
            save_playlists(&app.playlists);
        }
        Err(err) => app.set_flash(err, 5),
    }
}

fn export_selected_playlist_action(app: &mut App) {
    if let Some(playlist) = app.playlists.get(app.selected_playlist) {
        match export_playlist_to_default(playlist) {
            Ok(path) => app.set_flash(format!("Exported to {}", path.display()), 5),
            Err(err) => app.set_flash(err, 5),
        }
    } else {
        app.set_flash("No playlist selected", 3);
    }
}

fn context_menu_len(app: &App) -> usize {
    if app.active_tab == Tab::Library && app.focus == Focus::Results {
        2
    } else {
        4
    }
}

fn execute_context_action(app: &mut App, index: usize) {
    if app.active_tab == Tab::Library && app.focus == Focus::Results {
        match index {
            0 => import_playlist_action(app),
            1 => export_selected_playlist_action(app),
            _ => {}
        }
        return;
    }

    if let Some(song) = app.selected_song_for_context() {
        match index {
            0 => add_to_selected_playlist(app, song),
            1 => {
                let name = format!("Playlist {}", app.playlists.len() + 1);
                add_to_named_playlist(app, song, &name);
            }
            2 => remove_selected_queue_song(app),
            3 => remove_selected_playlist_song(app),
            _ => {}
        }
        save_playlists(&app.playlists);
    }
}

fn add_to_selected_playlist(app: &mut App, song: model::Song) {
    if app.playlists.is_empty() {
        add_to_named_playlist(app, song, "Favorites");
        return;
    }
    if let Some(pl) = app.playlists.get_mut(app.selected_playlist) {
        let name = pl.name.clone();
        pl.songs.push(song);
        app.set_flash(format!("Added to {}", name), 3);
    }
}

fn remove_selected_queue_song(app: &mut App) {
    if app.selected_queue < app.queue.len() {
        let removed = app.queue.remove(app.selected_queue);
        app.selected_queue = app.selected_queue.min(app.queue.len().saturating_sub(1));
        if let Some(song) = removed {
            app.set_flash(format!("Removed from queue: {}", song.title), 3);
        }
    }
}

fn remove_selected_playlist_song(app: &mut App) {
    if let Some(playlist) = app.playlists.get_mut(app.selected_playlist) {
        if app.selected_playlist_song < playlist.songs.len() {
            let removed = playlist.songs.remove(app.selected_playlist_song);
            let playlist_name = playlist.name.clone();
            app.selected_playlist_song = app
                .selected_playlist_song
                .min(playlist.songs.len().saturating_sub(1));
            app.set_flash(
                format!("Removed from {}: {}", playlist_name, removed.title),
                3,
            );
        }
    }
}

fn ensure_playlist_state(app: &mut App) {
    if app.playlist_expanded.len() < app.playlists.len() {
        app.playlist_expanded.extend(
            std::iter::repeat(false).take(app.playlists.len() - app.playlist_expanded.len()),
        );
    } else if app.playlist_expanded.len() > app.playlists.len() {
        app.playlist_expanded.truncate(app.playlists.len());
    }
    if let Some(p) = app.playlists.get(app.selected_playlist) {
        app.selected_playlist_song = app
            .selected_playlist_song
            .min(p.songs.len().saturating_sub(1));
    } else {
        app.selected_playlist_song = 0;
    }
}

fn apply_event(app: &mut App, event: CoreEvent) -> Option<CoreCmd> {
    match event {
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
            save_recently_played(&history);
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
    }
}

fn map_plugin_action(action: PluginCoreAction) -> Option<CoreCmd> {
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

fn apply_plugin_dispatch(
    app: &mut App,
    cmd_tx: &mpsc::UnboundedSender<CoreCmd>,
    dispatch: PluginDispatch,
) -> bool {
    if let Some(tab) = dispatch.ui.set_tab {
        app.active_tab = parse_tab_name(&tab).unwrap_or(app.active_tab);
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
        app.selected_album_result = index.min(app.album_results.len().saturating_sub(1));
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

fn plugin_event_from_core_event(event: &CoreEvent) -> PluginEvent {
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

fn parse_tab_name(raw: &str) -> Option<Tab> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "discover" => Some(Tab::Discover),
        "albums" => Some(Tab::Albums),
        "library" => Some(Tab::Library),
        "options" => Some(Tab::Options),
        _ => None,
    }
}

fn parse_focus_name(raw: &str) -> Option<Focus> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "search" => Some(Focus::Search),
        "results" => Some(Focus::Results),
        "queue" => Some(Focus::Queue),
        _ => None,
    }
}

fn player_state_label(state: PlayerState) -> &'static str {
    match state {
        PlayerState::Idle => "idle",
        PlayerState::Searching => "searching",
        PlayerState::Playing => "playing",
        PlayerState::Paused => "paused",
    }
}

fn describe_key_event(code: &KeyCode) -> String {
    match code {
        KeyCode::Char(c) => format!("char:{c}"),
        KeyCode::Enter => "enter".to_owned(),
        KeyCode::Esc => "esc".to_owned(),
        KeyCode::Tab => "tab".to_owned(),
        KeyCode::Backspace => "backspace".to_owned(),
        KeyCode::Left => "left".to_owned(),
        KeyCode::Right => "right".to_owned(),
        KeyCode::Up => "up".to_owned(),
        KeyCode::Down => "down".to_owned(),
        KeyCode::PageUp => "page_up".to_owned(),
        KeyCode::PageDown => "page_down".to_owned(),
        KeyCode::F(n) => format!("f{n}"),
        _ => "other".to_owned(),
    }
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore_terminal(mut terminal: Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

fn scroll_selection(app: &mut App, delta: isize) {
    match app.active_tab {
        Tab::Discover => match app.focus {
            Focus::Results => {
                let len = app.search_results.len();
                if len > 0 {
                    app.selected_result = ((app.selected_result as isize + delta)
                        .clamp(0, len as isize - 1))
                        as usize;
                }
            }
            Focus::Queue => {
                let len = app.queue.len();
                if len > 0 {
                    app.selected_queue =
                        ((app.selected_queue as isize + delta).clamp(0, len as isize - 1)) as usize;
                }
            }
            Focus::Search => {}
        },
        Tab::Albums => match app.focus {
            Focus::Results => {
                let len = app.album_results.len();
                if len > 0 {
                    app.selected_album_result = ((app.selected_album_result as isize + delta)
                        .clamp(0, len as isize - 1))
                        as usize;
                }
            }
            Focus::Queue => {
                let len = app.queue.len();
                if len > 0 {
                    app.selected_queue =
                        ((app.selected_queue as isize + delta).clamp(0, len as isize - 1)) as usize;
                }
            }
            Focus::Search => {}
        },
        Tab::Library => match app.focus {
            Focus::Results => {
                let len = app.playlists.len();
                if len > 0 {
                    app.selected_playlist = ((app.selected_playlist as isize + delta)
                        .clamp(0, len as isize - 1))
                        as usize;
                    app.selected_playlist_song = 0;
                }
            }
            Focus::Queue => {
                if let Some(p) = app.playlists.get(app.selected_playlist) {
                    let len = p.songs.len();
                    if len > 0 {
                        app.selected_playlist_song = ((app.selected_playlist_song as isize + delta)
                            .clamp(0, len as isize - 1))
                            as usize;
                    }
                }
            }
            Focus::Search => {}
        },
        Tab::Options => {
            app.options_index = ((app.options_index as isize + delta).clamp(0, 9)) as usize;
        }
    }
}

fn next_theme(theme: config::Theme) -> config::Theme {
    match theme {
        config::Theme::Dark => config::Theme::Light,
        config::Theme::Light => config::Theme::Custom,
        config::Theme::Custom => config::Theme::Nord,
        config::Theme::Nord => config::Theme::Gruvbox,
        config::Theme::Gruvbox => config::Theme::Mono,
        config::Theme::Mono => config::Theme::Dark,
    }
}

fn prev_theme(theme: config::Theme) -> config::Theme {
    match theme {
        config::Theme::Dark => config::Theme::Mono,
        config::Theme::Light => config::Theme::Dark,
        config::Theme::Custom => config::Theme::Light,
        config::Theme::Nord => config::Theme::Custom,
        config::Theme::Gruvbox => config::Theme::Nord,
        config::Theme::Mono => config::Theme::Gruvbox,
    }
}

fn prev_repeat_mode(mode: RepeatMode) -> RepeatMode {
    match mode {
        RepeatMode::Off => RepeatMode::All,
        RepeatMode::One => RepeatMode::Off,
        RepeatMode::All => RepeatMode::One,
    }
}

use crossterm::event::{KeyEvent, MouseEvent, KeyEventKind, KeyCode, KeyModifiers, MouseButton, MouseEventKind};
use tokio::sync::mpsc;
use crate::model::{App, Focus, LocalNavLevel, LocalViewMode, PlayerState, Tab, Song};
use crate::plugins::{PluginManager, PluginUiState};
use crate::core::CoreCmd;
use crate::ui_helpers;
use crate::playlist;
use crate::eq;
use crate::config::{save_config, SearchSource, EqPreset};
use crate::events;

pub fn handle_mouse_event(app: &mut App, mouse: MouseEvent) {
    match mouse.kind {
        MouseEventKind::ScrollDown => {
            let local_nav_len = get_local_nav_len(app);
            ui_helpers::scroll_selection(app, 3, local_nav_len)
        }
        MouseEventKind::ScrollUp => {
            let local_nav_len = get_local_nav_len(app);
            ui_helpers::scroll_selection(app, -3, local_nav_len)
        }
        MouseEventKind::Down(MouseButton::Left) => {
            if mouse.row <= 2 {
                app.active_tab = if mouse.column < 16 {
                    Tab::Discover
                } else if mouse.column < 32 {
                    Tab::Albums
                } else if mouse.column < 48 {
                    Tab::Library
                } else if mouse.column < 64 {
                    Tab::Local
                } else {
                    Tab::Options
                };
            }
        }
        _ => {}
    }
}

pub fn handle_key_event(
    app: &mut App,
    key: KeyEvent,
    ui_state: &PluginUiState,
    plugin_manager: &PluginManager,
    cmd_tx: &mpsc::UnboundedSender<CoreCmd>,
) -> bool {
    if key.kind != KeyEventKind::Press {
        return true;
    }

    if key.code == KeyCode::Char('c')
        && key.modifiers.contains(KeyModifiers::CONTROL)
    {
        return false;
    }

    if app.opt_editing {
        if let KeyCode::Char(c) = key.code {
            app.opt_edit_buffer.push(c);
            return true;
        } else if key.code == KeyCode::Backspace {
            app.opt_edit_buffer.pop();
            return true;
        } else if key.code == KeyCode::Enter {
        } else {
            return true;
        }
    }
    if app.context_open {
        match key.code {
            KeyCode::Esc => app.context_open = false,
            KeyCode::Char('j') | KeyCode::Down => {
                app.context_index = (app.context_index + 1)
                    .min(context_menu_len(app).saturating_sub(1));
            }
            KeyCode::Char('k') | KeyCode::Up => {
                app.context_index = app.context_index.saturating_sub(1);
            }
            KeyCode::Enter => {
                let idx = app.context_index;
                playlist::execute_context_action(app, idx);
                playlist::ensure_playlist_state(app);
                app.context_open = false;
            }
            _ => {}
        }
        return true;
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
                    app.storage.save_playlists(&app.playlists).expect("Failed to save playlists");
                    app.set_flash(format!("Deleted {}", deleted.name), 3);
                    playlist::ensure_playlist_state(app);
                }
                app.confirm_delete_playlist = false;
            }
            KeyCode::Char('n') | KeyCode::Esc => {
                app.confirm_delete_playlist = false;
                app.set_flash("Delete canceled", 2);
            }
            _ => {}
        }
        return true;
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
        return true;
    }

    let key_label = ui_helpers::describe_key_event(&key.code);
    let plugin_dispatch = plugin_manager.dispatch_key(
        key_label.as_str(),
        ui_state,
    );
    if events::apply_plugin_dispatch(app, cmd_tx, plugin_dispatch) {
        return true;
    }

    match key.code {
        KeyCode::Char('1') => app.active_tab = Tab::Discover,
        KeyCode::Char('2') => app.active_tab = Tab::Albums,
        KeyCode::Char('3') => app.active_tab = Tab::Library,
        KeyCode::Char('4') => app.active_tab = Tab::Local,
        KeyCode::Char('5') => app.active_tab = Tab::Options,
        KeyCode::Char('a') if app.active_tab == Tab::Library => {
            let name = format!("Playlist {}", app.playlists.len() + 1);
            playlist::create_empty_playlist(app, &name);
            app.storage.save_playlists(&app.playlists).expect("Failed to save playlists");
            app.set_flash(format!("Created {name}"), 3);
            playlist::ensure_playlist_state(app);
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
                return true;
            }
            app.search_mode = true;
            app.focus = Focus::Search;
        }
        KeyCode::Char('v') if app.active_tab == Tab::Local => {
            app.local_view_mode = match app.local_view_mode {
                LocalViewMode::Flat => LocalViewMode::Organized,
                LocalViewMode::Organized => LocalViewMode::Flat,
            };
            app.set_flash(
                format!(
                    "View mode: {}",
                    match app.local_view_mode {
                        LocalViewMode::Flat => "Flat",
                        LocalViewMode::Organized => "Organized",
                    }
                ),
                2,
            );
        }
        KeyCode::Char('f') if app.active_tab == Tab::Options && app.options_index == 8 => {
            app.opt_editing = true;
            app.opt_edit_buffer = "New Preset".to_string();
            app.set_flash("Editing EQ Preset Name... (Enter to save)", 3);
        }
        KeyCode::Char('j') | KeyCode::Down => match app.focus {
            Focus::Results => {
                if app.active_tab == Tab::Options {
                    app.options_index = (app.options_index + 1).min(ui_helpers::MAX_OPTIONS_INDEX);
                } else if app.active_tab == Tab::Library {
                    if !app.playlists.is_empty() {
                        app.selected_playlist = (app.selected_playlist + 1)
                            .min(app.playlists.len().saturating_sub(1));
                        app.selected_playlist_song = 0;
                        playlist::ensure_playlist_state(app);
                    }
                } else if app.active_tab == Tab::Albums {
                    if !app.album_results.is_empty() {
                        app.selected_album_result = (app.selected_album_result + 1).min(app.album_results.len().saturating_sub(1));
                    }
                } else if app.active_tab == Tab::Local {
                    if app.local_library_total > 0 {
                        app.selected_local_song = (app.selected_local_song + 1)
                            .min(app.local_library_total.saturating_sub(1));
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
                            app.selected_playlist_song = (app.selected_playlist_song + 1)
                                .min(playlist.songs.len().saturating_sub(1));
                            playlist::ensure_playlist_state(app);
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
                    playlist::ensure_playlist_state(app);
                } else if app.active_tab == Tab::Albums {
                    app.selected_album_result = app.selected_album_result.saturating_sub(1);
                } else if app.active_tab == Tab::Local {
                    app.selected_local_song = app.selected_local_song.saturating_sub(1);
                } else {
                    app.selected_result = app.selected_result.saturating_sub(1);
                }
            }
            Focus::Queue => {
                if app.active_tab == Tab::Library {
                    app.selected_playlist_song =
                        app.selected_playlist_song.saturating_sub(1);
                    playlist::ensure_playlist_state(app);
                } else {
                    app.selected_queue = app.selected_queue.saturating_sub(1);
                }
            }
            Focus::Search => app.focus = Focus::Results,
        },
        KeyCode::PageDown => {
            let local_nav_len = get_local_nav_len(app);
            ui_helpers::scroll_selection(app, 10, local_nav_len)
        }
        KeyCode::PageUp => {
            let local_nav_len = get_local_nav_len(app);
            ui_helpers::scroll_selection(app, -10, local_nav_len)
        }
        KeyCode::Tab => {
            if app.active_tab == Tab::Options {
                return true;
            }
            app.focus = match app.focus {
                Focus::Search | Focus::Results => Focus::Queue,
                Focus::Queue => Focus::Results,
            };
        }
        KeyCode::Enter => {
            if app.active_tab == Tab::Options {
                if app.options_index == 3 {
                    if app.opt_editing {
                        let new_dir = app.opt_edit_buffer.clone();
                        app.opt_music_dirs = vec![new_dir];
                        save_config(&app.build_config());
                        app.opt_editing = false;
                        app.set_flash("Music directory updated", 3);
                    } else {
                        app.opt_editing = true;
                        app.opt_edit_buffer = app.opt_music_dirs.first().cloned().unwrap_or_default();
                        app.set_flash("Editing Music Directory... (Enter to save)", 3);
                    }
                } else if app.options_index == 8 && app.opt_editing {
                    let name = app.opt_edit_buffer.clone();
                    let preset = EqPreset {
                        name: name.clone(),
                        bands: app.eq_bands,
                    };
                    if let Err(e) = crate::config::save_eq_preset(&preset) {
                        app.set_flash(format!("Error saving preset: {e}"), 4);
                    } else {
                        app.custom_eq_presets.push(preset);
                        app.set_flash(format!("Saved preset: {name}"), 3);
                    }
                    app.opt_editing = false;
                } else if app.options_index == 4 {
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
                } else if app.options_index == 7 {
                    app.eq_enabled = !app.eq_enabled;
                    if app.eq_enabled {
                        eq::send_eq_update(cmd_tx, app.eq_bands);
                        app.set_flash("Equalizer ON", 2);
                    } else {
                        eq::send_eq_update(cmd_tx, [0.0f32; 10]);
                        app.set_flash("Equalizer OFF", 2);
                    }
                } else if app.options_index == 8 {
                    eq::cycle_eq_preset(app, cmd_tx, 1);
                }
                return true;
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
                    if let Some(album) = app.album_results.get(app.selected_album_result) {
                        if let Some(song) = album.songs.first() {
                            app.queue.push_back(song.clone());
                            app.selected_queue = app.queue.len().saturating_sub(1);
                            let _ = cmd_tx.send(CoreCmd::Play(song.clone()));
                        }
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
                Focus::Results => {
                    if app.active_tab == Tab::Local {
                        if app.local_view_mode == LocalViewMode::Flat {
                            if let Some(ls) = app.local_library_window.get(app.selected_local_song.saturating_sub(app.local_library_offset)) {
                                let song = Song::from(ls);
                                app.queue.push_back(song.clone());
                                app.selected_queue = app.queue.len().saturating_sub(1);
                                let _ = cmd_tx.send(CoreCmd::Play(song));
                            }
                        } else {
                            let items = ui_helpers::get_local_nav_items(app);
                            match items {
                                ui_helpers::LocalNavItems::Artists(artists) => {
                                    if let Some(artist) = artists.get(app.selected_local_nav_idx) {
                                        app.local_nav_artist = Some(artist.clone());
                                        app.local_nav_level = LocalNavLevel::Albums;
                                        app.selected_local_nav_idx = 0;
                                    }
                                }
                                ui_helpers::LocalNavItems::Albums(albums) => {
                                    if let Some(album) = albums.get(app.selected_local_nav_idx) {
                                        app.local_nav_album = Some(album.clone());
                                        app.local_nav_level = LocalNavLevel::Songs;
                                        app.selected_local_nav_idx = 0;
                                    }
                                }
                                ui_helpers::LocalNavItems::Songs(songs) => {
                                    if let Some(ls) = songs.get(app.selected_local_nav_idx) {
                                        let song = Song::from(*ls);
                                        app.queue.push_back(song.clone());
                                        app.selected_queue = app.queue.len().saturating_sub(1);
                                        let _ = cmd_tx.send(CoreCmd::Play(song));
                                    }
                                }
                            }
                        }
                    }
                }
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
            ui_helpers::shuffle_queue_keep_current(app);
        }
        KeyCode::Char(c) if c == app.key_seek_back => {
            let _ = cmd_tx.send(CoreCmd::SeekBy(-10));
        }
        KeyCode::Char(c) if c == app.key_seek_forward => {
            let _ = cmd_tx.send(CoreCmd::SeekBy(10));
        }
        KeyCode::Char('q') => {
            return false;
        }
        KeyCode::Char('d') => {
            if app.active_tab == Tab::Library && app.focus == Focus::Queue {
                playlist::remove_selected_playlist_song(app);
                playlist::ensure_playlist_state(app);
                app.storage.save_playlists(&app.playlists).expect("Failed to save playlists");
            } else if app.focus == Focus::Queue {
                playlist::remove_selected_queue_song(app);
            }
        }
        KeyCode::Char('i')
            if app.active_tab == Tab::Library && app.focus == Focus::Results =>
        {
            playlist::import_playlist_action(app);
            playlist::ensure_playlist_state(app);
        }
        KeyCode::Char('e')
            if app.active_tab == Tab::Library && app.focus == Focus::Results =>
        {
            playlist::export_selected_playlist_action(app);
        }
        KeyCode::Char('h') | KeyCode::Left if app.active_tab == Tab::Options => {
            match app.options_index {
                0 => {
                    toggle_search_source(app, cmd_tx);
                }
                1 => {
                    app.opt_search_limit =
                        app.opt_search_limit.saturating_sub(1).max(1);
                }
                2 => app.opt_socket = "/tmp/rs-pug.sock".to_owned(),
                5 => {
                    app.opt_theme = ui_helpers::prev_theme(app.opt_theme.clone());
                }
                6 => {
                    app.repeat_mode = ui_helpers::prev_repeat_mode(app.repeat_mode);
                    app.set_flash(
                        format!("Repeat mode: {}", app.repeat_mode.label()),
                        2,
                    );
                }
                7 => {
                    if app.eq_focus_band > 0 {
                        app.eq_focus_band -= 1;
                    }
                }
                8 => eq::cycle_eq_preset(app, cmd_tx, -1),
                9 => app.key_next = ui_helpers::cycle_keybind_char(app.key_next, -1),
                10 => app.key_prev = ui_helpers::cycle_keybind_char(app.key_prev, -1),
                ui_helpers::MAX_OPTIONS_INDEX => app.key_mute = ui_helpers::cycle_keybind_char(app.key_mute, -1),
                _ => {}
            }
        }
        KeyCode::Char('l') | KeyCode::Right if app.active_tab == Tab::Options => {
            match app.options_index {
                0 => {
                    toggle_search_source(app, cmd_tx);
                }
                1 => app.opt_search_limit = (app.opt_search_limit + 1).min(50),
                2 => app.opt_socket = "/tmp/rs-pug.sock".to_owned(),
                5 => {
                    app.opt_theme = ui_helpers::next_theme(app.opt_theme.clone());
                }
                6 => {
                    app.repeat_mode = app.repeat_mode.next();
                    app.set_flash(
                        format!("Repeat mode: {}", app.repeat_mode.label()),
                        2,
                    );
                }
                7 => {
                    if app.eq_focus_band < 9 {
                        app.eq_focus_band += 1;
                    }
                }
                8 => eq::cycle_eq_preset(app, cmd_tx, 1),
                9 => app.key_next = ui_helpers::cycle_keybind_char(app.key_next, 1),
                10 => app.key_prev = ui_helpers::cycle_keybind_char(app.key_prev, 1),
                ui_helpers::MAX_OPTIONS_INDEX => app.key_mute = ui_helpers::cycle_keybind_char(app.key_mute, 1),
                _ => {}
            }
        }
        KeyCode::Char('p') if app.active_tab == Tab::Options => {
            eq::cycle_eq_preset(app, cmd_tx, 1);
        }
        KeyCode::Char('s') if app.active_tab == Tab::Options => {
            save_config(&app.build_config());
            app.theme = app.opt_theme.clone();
            app.set_flash("Saved settings to ~/.config/rs-pug/config.toml", 4);
        }
        KeyCode::Char('+') | KeyCode::Char('=')
            if app.active_tab == Tab::Options && app.options_index == 7 =>
        {
            let b = app.eq_focus_band;
            app.eq_bands[b] = (app.eq_bands[b] + 1.0).min(12.0);
            if app.eq_enabled {
                eq::send_eq_update(cmd_tx, app.eq_bands);
            }
        }
        KeyCode::Char('-')
            if app.active_tab == Tab::Options && app.options_index == 7 =>
        {
            let b = app.eq_focus_band;
            app.eq_bands[b] = (app.eq_bands[b] - 1.0).max(-12.0);
            if app.eq_enabled {
                eq::send_eq_update(cmd_tx, app.eq_bands);
            }
        }
        KeyCode::Char('0')
            if app.active_tab == Tab::Options && app.options_index == 7 =>
        {
            app.eq_bands = [0.0f32; 10];
            app.eq_preset_index = 0;
            if app.eq_enabled {
                eq::send_eq_update(cmd_tx, [0.0f32; 10]);
            }
            app.set_flash("Equalizer reset to Flat", 2);
        }
        _ => {}
    }
    true
}

fn get_local_nav_len(app: &App) -> usize {
    if app.active_tab == Tab::Local && app.local_view_mode == LocalViewMode::Organized {
        ui_helpers::get_local_nav_items(app).len()
    } else {
        0
    }
}

fn toggle_search_source(app: &mut App, cmd_tx: &mpsc::UnboundedSender<CoreCmd>) {
    app.opt_source = match app.opt_source {
        SearchSource::YouTube => SearchSource::SoundCloud,
        SearchSource::SoundCloud => SearchSource::YouTube,
    };
    let _ = cmd_tx.send(CoreCmd::UpdateSearchSource(app.opt_source));
    save_config(&app.build_config());
    app.set_flash(
        format!(
            "Search source: {}",
            if matches!(app.opt_source, SearchSource::YouTube) {
                "YouTube"
            } else {
                "SoundCloud"
            }
        ),
        2,
    );
}

fn context_menu_len(app: &App) -> usize {
    if app.active_tab == Tab::Library && app.focus == Focus::Results {
        2
    } else {
        4
    }
}

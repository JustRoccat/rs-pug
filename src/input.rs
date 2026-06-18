use crate::config::{save_config, EqPreset, SearchSource};
use crate::core::CoreCmd;
use crate::eq;
use crate::events;
use crate::model::{
    App, Focus, LocalNavLevel, LocalTagField, LocalViewMode, PlayerState, Song, Tab,
};
use crate::playlist;
use crate::ui_helpers;
use crossterm::{
    event::{
        KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
    },
    terminal,
};
use tokio::sync::mpsc;

pub enum KeyPluginAction {
    Handled(bool),
    Dispatch { labels: Vec<String> },
}

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
            if let Some(tab_idx) = tab_index_from_mouse(app, mouse) {
                activate_tab_by_render_index(app, tab_idx);
            }
        }
        _ => {}
    }
}

pub fn handle_key_event_pre_plugin(
    app: &mut App,
    key: KeyEvent,
    cmd_tx: &mpsc::UnboundedSender<CoreCmd>,
) -> KeyPluginAction {
    if key.kind != KeyEventKind::Press {
        return KeyPluginAction::Handled(true);
    }

    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return KeyPluginAction::Handled(false);
    }

    if app.local_tag_editor_open {
        match key.code {
            KeyCode::Esc => {
                app.local_tag_editor_open = false;
                app.local_tag_editor_song = None;
            }
            KeyCode::Tab => cycle_tag_field(app),
            KeyCode::Backspace => {
                app.local_tag_edit_buffer.pop();
            }
            KeyCode::Enter => save_tag_field_or_close(app),
            KeyCode::Char(c) => app.local_tag_edit_buffer.push(c),
            _ => {}
        }
        return KeyPluginAction::Handled(true);
    }

    if app.opt_editing {
        if let KeyCode::Char(c) = key.code {
            app.opt_edit_buffer.push(c);
            return KeyPluginAction::Handled(true);
        } else if key.code == KeyCode::Backspace {
            app.opt_edit_buffer.pop();
            return KeyPluginAction::Handled(true);
        } else if key.code == KeyCode::Enter {
        } else {
            return KeyPluginAction::Handled(true);
        }
    }
    if app.context_open {
        match key.code {
            KeyCode::Esc => app.context_open = false,
            KeyCode::Char('j') | KeyCode::Down => {
                app.context_index =
                    (app.context_index + 1).min(context_menu_len(app).saturating_sub(1));
            }
            KeyCode::Char('k') | KeyCode::Up => {
                app.context_index = app.context_index.saturating_sub(1);
            }
            KeyCode::Enter => {
                let idx = app.context_index;
                playlist::execute_context_action(app, idx, cmd_tx);
                playlist::ensure_playlist_state(app);
                app.context_open = false;
            }
            _ => {}
        }
        return KeyPluginAction::Handled(true);
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
                    app.storage
                        .save_playlists(&app.playlists)
                        .expect("Failed to save playlists");
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
        return KeyPluginAction::Handled(true);
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
                if !q.is_empty() && app.active_tab != Tab::Local {
                    app.player_state = PlayerState::Searching;
                    let _ = if app.active_tab == Tab::Albums {
                        cmd_tx.send(CoreCmd::SearchAlbums(q))
                    } else {
                        cmd_tx.send(CoreCmd::Search(q))
                    };
                }
                app.search_mode = false;
                if app.active_tab == Tab::Local {
                    if let Ok((window, offset, total)) = app
                        .storage
                        .fetch_local_songs_window(app.selected_local_song, 200)
                    {
                        app.local_library_window = window;
                        app.local_library_offset = offset;
                        app.local_library_total = total;
                    }
                }
            }
            KeyCode::Backspace => {
                if app.active_tab == Tab::Albums {
                    app.album_search_query.pop();
                } else {
                    app.search_query.pop();
                }
                if app.active_tab == Tab::Local {
                    let q = app.search_query.to_lowercase();
                    let _ = q;
                    ui_helpers::refresh_local_visible_window(app);
                }
            }
            KeyCode::Char(c) => {
                if app.active_tab == Tab::Albums {
                    app.album_search_query.push(c);
                } else {
                    app.search_query.push(c);
                }
                if app.active_tab == Tab::Local {
                    let q = app.search_query.to_lowercase();
                    let _ = q;
                    ui_helpers::refresh_local_visible_window(app);
                }
            }
            _ => {}
        }
        return KeyPluginAction::Handled(true);
    }

    KeyPluginAction::Dispatch {
        labels: ui_helpers::describe_key_event_labels(&key.code, key.modifiers),
    }
}

pub fn handle_native_key_event(
    app: &mut App,
    key: KeyEvent,
    cmd_tx: &mpsc::UnboundedSender<CoreCmd>,
) -> bool {
    let is_core_options =
        |app: &App| app.active_tab == Tab::Options && app.active_plugin_tab.is_none();

    match key.code {
        KeyCode::Char(c @ '1'..='8') => {
            activate_numbered_tab(app, c);
        }
        KeyCode::Char('9') => {
            let _ = cmd_tx.send(CoreCmd::VolumeDown);
        }
        KeyCode::Char('0') => {
            let _ = cmd_tx.send(CoreCmd::VolumeUp);
        }
        KeyCode::Char('a') if app.active_tab == Tab::Library => {
            let name = format!("Playlist {}", app.playlists.len() + 1);
            playlist::create_empty_playlist(app, &name);
            app.storage
                .save_playlists(&app.playlists)
                .expect("Failed to save playlists");
            app.set_flash(format!("Created {name}"), 3);
            playlist::ensure_playlist_state(app);
        }
        KeyCode::Char('x') if app.active_tab == Tab::Library => {
            if app.selected_playlist < app.playlists.len() {
                app.confirm_delete_playlist = true;
                app.delete_playlist_name = app.playlists[app.selected_playlist].name.clone();
            }
        }
        KeyCode::Char('e') if app.active_tab == Tab::Library => {
            if let Some(open) = app.playlist_expanded.get_mut(app.selected_playlist) {
                *open = !*open;
            }
        }
        KeyCode::Char('c') => {
            let can_open = if app.active_tab == Tab::Library && app.focus == Focus::Results {
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
            if !matches!(app.active_tab, Tab::Discover | Tab::Albums | Tab::Local) {
                return true;
            }
            app.search_mode = true;
            app.focus = Focus::Search;
        }
        KeyCode::Backspace | KeyCode::Esc
            if app.active_tab == Tab::Local
                && app.local_view_mode == LocalViewMode::Organized
                && app.focus == Focus::Results =>
        {
            local_nav_back(app);
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

        KeyCode::Char('s') if app.active_tab == Tab::Local => {
            app.local_sort_mode = app.local_sort_mode.next();
            ui_helpers::refresh_local_visible_window(app);
            app.set_flash(format!("Local sort: {}", app.local_sort_mode.label()), 2);
        }
        KeyCode::Char('f')
            if app.active_tab == Tab::Local && key.modifiers.contains(KeyModifiers::CONTROL) =>
        {
            clear_local_filters(app);
        }
        KeyCode::Char('F') if app.active_tab == Tab::Local => {
            clear_local_filters(app);
        }
        KeyCode::Char('g') if app.active_tab == Tab::Local => set_filter_from_selected(app, 'g'),
        KeyCode::Char('a') if app.active_tab == Tab::Local => set_filter_from_selected(app, 'a'),
        KeyCode::Char('b') if app.active_tab == Tab::Local => set_filter_from_selected(app, 'b'),
        KeyCode::Char('e')
            if app.active_tab == Tab::Local && app.local_view_mode == LocalViewMode::Flat =>
        {
            open_tag_editor(app)
        }
        KeyCode::Char('f') if is_core_options(app) && app.options_index == 8 => {
            app.opt_editing = true;
            app.opt_edit_buffer = "New Preset".to_string();
            app.set_flash("Editing EQ Preset Name... (Enter to save)", 3);
        }
        KeyCode::Char('j') | KeyCode::Down => match app.focus {
            Focus::Results => {
                if is_core_options(app) {
                    app.options_index = (app.options_index + 1).min(ui_helpers::MAX_OPTIONS_INDEX);
                } else if app.active_tab == Tab::Library {
                    if !app.playlists.is_empty() {
                        app.selected_playlist =
                            (app.selected_playlist + 1).min(app.playlists.len().saturating_sub(1));
                        app.selected_playlist_song = 0;
                        playlist::ensure_playlist_state(app);
                    }
                } else if app.active_tab == Tab::Albums {
                    if !app.album_results.is_empty() {
                        app.selected_album_result = (app.selected_album_result + 1)
                            .min(app.album_results.len().saturating_sub(1));
                    }
                } else if app.active_tab == Tab::Local {
                    if app.local_view_mode == LocalViewMode::Flat {
                        if app.local_library_total > 0 {
                            app.selected_local_song = (app.selected_local_song + 1)
                                .min(app.local_library_total.saturating_sub(1));
                        }
                    } else {
                        let local_nav_len = get_local_nav_len(app);
                        if local_nav_len > 0 {
                            app.selected_local_nav_idx = (app.selected_local_nav_idx + 1)
                                .min(local_nav_len.saturating_sub(1));
                        }
                    }
                } else if !app.search_results.is_empty() {
                    app.selected_result =
                        (app.selected_result + 1).min(app.search_results.len().saturating_sub(1));
                }
            }
            Focus::Queue => {
                if app.active_tab == Tab::Library {
                    if let Some(playlist) = app.playlists.get(app.selected_playlist) {
                        if !playlist.songs.is_empty() {
                            app.selected_playlist_song = (app.selected_playlist_song + 1)
                                .min(playlist.songs.len().saturating_sub(1));
                            playlist::ensure_playlist_state(app);
                        }
                    }
                } else if !app.queue.is_empty() {
                    app.selected_queue =
                        (app.selected_queue + 1).min(app.queue.len().saturating_sub(1));
                }
            }
            Focus::Search => app.focus = Focus::Results,
        },
        KeyCode::Char('k') | KeyCode::Up => match app.focus {
            Focus::Results => {
                if is_core_options(app) {
                    app.options_index = app.options_index.saturating_sub(1);
                } else if app.active_tab == Tab::Library {
                    app.selected_playlist = app.selected_playlist.saturating_sub(1);
                    app.selected_playlist_song = 0;
                    playlist::ensure_playlist_state(app);
                } else if app.active_tab == Tab::Albums {
                    app.selected_album_result = app.selected_album_result.saturating_sub(1);
                } else if app.active_tab == Tab::Local {
                    if app.local_view_mode == LocalViewMode::Flat {
                        app.selected_local_song = app.selected_local_song.saturating_sub(1);
                    } else {
                        app.selected_local_nav_idx = app.selected_local_nav_idx.saturating_sub(1);
                    }
                } else {
                    app.selected_result = app.selected_result.saturating_sub(1);
                }
            }
            Focus::Queue => {
                if app.active_tab == Tab::Library {
                    app.selected_playlist_song = app.selected_playlist_song.saturating_sub(1);
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
            if is_core_options(app) {
                return true;
            }
            app.focus = match app.focus {
                Focus::Search | Focus::Results => Focus::Queue,
                Focus::Queue => Focus::Results,
            };
        }
        KeyCode::Enter => {
            if is_core_options(app) {
                if app.options_index == 3 {
                    if app.opt_editing {
                        let new_dir = app.opt_edit_buffer.clone();
                        app.opt_music_dirs = vec![new_dir];
                        save_config(&app.build_config());
                        app.opt_editing = false;
                        app.set_flash("Music directory updated", 3);
                    } else {
                        app.opt_editing = true;
                        app.opt_edit_buffer =
                            app.opt_music_dirs.first().cloned().unwrap_or_default();
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
                        app.set_flash("Smart Queue needs a currently playing song", 3);
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
                    if let Some(playlist) = app.playlists.get(app.selected_playlist) {
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
                        if let Some(playlist) = app.playlists.get(app.selected_playlist) {
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
                            if let Some(ls) = app.local_library_window.get(
                                app.selected_local_song
                                    .saturating_sub(app.local_library_offset),
                            ) {
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
        KeyCode::Left if !is_core_options(app) => {
            let _ = cmd_tx.send(CoreCmd::SeekBy(-10));
        }
        KeyCode::Right if !is_core_options(app) => {
            let _ = cmd_tx.send(CoreCmd::SeekBy(10));
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
                app.storage
                    .save_playlists(&app.playlists)
                    .expect("Failed to save playlists");
            } else if app.focus == Focus::Queue {
                playlist::remove_selected_queue_song(app);
            }
        }
        KeyCode::Char('i') if app.active_tab == Tab::Library && app.focus == Focus::Results => {
            playlist::import_playlist_action(app);
            playlist::ensure_playlist_state(app);
        }
        KeyCode::Char('e') if app.active_tab == Tab::Library && app.focus == Focus::Results => {
            playlist::export_selected_playlist_action(app);
        }
        KeyCode::Char('h') | KeyCode::Left if is_core_options(app) => match app.options_index {
            0 => {
                toggle_search_source(app, cmd_tx);
            }
            1 => {
                app.opt_search_limit = app.opt_search_limit.saturating_sub(1).max(1);
            }
            2 => app.opt_socket = "/tmp/rs-pug.sock".to_owned(),
            5 => {
                app.opt_theme = ui_helpers::prev_theme(app.opt_theme.clone());
            }
            6 => {
                app.repeat_mode = ui_helpers::prev_repeat_mode(app.repeat_mode);
                app.set_flash(format!("Repeat mode: {}", app.repeat_mode.label()), 2);
            }
            7 => {
                if app.eq_focus_band > 0 {
                    app.eq_focus_band -= 1;
                }
            }
            8 => eq::cycle_eq_preset(app, cmd_tx, -1),
            9 => app.key_next = ui_helpers::cycle_keybind_char(app.key_next, -1),
            10 => app.key_prev = ui_helpers::cycle_keybind_char(app.key_prev, -1),
            ui_helpers::MAX_OPTIONS_INDEX => {
                app.key_mute = ui_helpers::cycle_keybind_char(app.key_mute, -1)
            }
            _ => {}
        },
        KeyCode::Char('l') | KeyCode::Right if is_core_options(app) => match app.options_index {
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
                app.set_flash(format!("Repeat mode: {}", app.repeat_mode.label()), 2);
            }
            7 => {
                if app.eq_focus_band < 9 {
                    app.eq_focus_band += 1;
                }
            }
            8 => eq::cycle_eq_preset(app, cmd_tx, 1),
            9 => app.key_next = ui_helpers::cycle_keybind_char(app.key_next, 1),
            10 => app.key_prev = ui_helpers::cycle_keybind_char(app.key_prev, 1),
            ui_helpers::MAX_OPTIONS_INDEX => {
                app.key_mute = ui_helpers::cycle_keybind_char(app.key_mute, 1)
            }
            _ => {}
        },
        KeyCode::Char('p') if is_core_options(app) => {
            eq::cycle_eq_preset(app, cmd_tx, 1);
        }
        KeyCode::Char('s') if is_core_options(app) => {
            save_config(&app.build_config());
            app.theme = app.opt_theme.clone();
            app.set_flash("Saved settings to ~/.config/rs-pug/config.toml", 4);
        }
        KeyCode::Char('+') | KeyCode::Char('=')
            if is_core_options(app) && app.options_index == 7 =>
        {
            let b = app.eq_focus_band;
            app.eq_bands[b] = (app.eq_bands[b] + 1.0).min(12.0);
            if app.eq_enabled {
                eq::send_eq_update(cmd_tx, app.eq_bands);
            }
        }
        KeyCode::Char('-') if is_core_options(app) && app.options_index == 7 => {
            let b = app.eq_focus_band;
            app.eq_bands[b] = (app.eq_bands[b] - 1.0).max(-12.0);
            if app.eq_enabled {
                eq::send_eq_update(cmd_tx, app.eq_bands);
            }
        }
        _ => {}
    }
    true
}

fn activate_numbered_tab(app: &mut App, key: char) {
    let Some(digit) = key.to_digit(10).map(|value| value as usize) else {
        return;
    };
    if !(1..=8).contains(&digit) {
        return;
    }

    activate_tab_by_render_index(app, digit - 1);
}

fn activate_tab_by_render_index(app: &mut App, tab_index: usize) {
    if tab_index < app.main_tabs.len() {
        events::activate_main_tab(app, tab_index);
        return;
    }

    let plugin_index = tab_index.saturating_sub(app.main_tabs.len());
    if let Some(tab) = app.plugin_tabs.get(plugin_index) {
        app.active_tab = Tab::Options;
        app.active_plugin_tab = Some(tab.id.clone());
        app.active_custom_tab = None;
    }
}

fn tab_index_from_mouse(app: &App, mouse: MouseEvent) -> Option<usize> {
    let tab_count = app.main_tabs.len() + app.plugin_tabs.len();
    if tab_count == 0 {
        return None;
    }
    let (terminal_width, terminal_height) = terminal::size().unwrap_or((0, 0));
    if terminal_width == 0 || terminal_height == 0 {
        return None;
    }

    match app.ui_layout.tab_bar_position.as_str() {
        "left" => {
            let tabs_width = side_tabs_width(app, terminal_width);
            if mouse.column >= tabs_width || mouse.row == 0 {
                return None;
            }
            vertical_tab_index(mouse.row, tab_count)
        }
        "right" => {
            let tabs_width = side_tabs_width(app, terminal_width);
            let start_col = terminal_width.saturating_sub(tabs_width);
            if mouse.column < start_col || mouse.row == 0 {
                return None;
            }
            vertical_tab_index(mouse.row, tab_count)
        }
        "bottom" => {
            let start_row = terminal_height.saturating_sub(3);
            if mouse.row < start_row || mouse.row >= terminal_height {
                return None;
            }
            horizontal_tab_index(app, mouse.column, terminal_width)
        }
        _ => {
            if mouse.row > 2 {
                return None;
            }
            horizontal_tab_index(app, mouse.column, terminal_width)
        }
    }
}

fn side_tabs_width(app: &App, terminal_width: u16) -> u16 {
    app.ui_layout
        .tabs_width
        .min(terminal_width.saturating_sub(20))
        .max(1)
        .min(terminal_width)
}

fn vertical_tab_index(row: u16, tab_count: usize) -> Option<usize> {
    (row as usize).checked_sub(1).filter(|idx| *idx < tab_count)
}

fn horizontal_tab_index(app: &App, column: u16, terminal_width: u16) -> Option<usize> {
    if column == 0 || column >= terminal_width.saturating_sub(1) {
        return None;
    }

    let inner_col = column.saturating_sub(1) as usize;
    let mut start = 0usize;
    for (idx, (icon, label)) in tab_defs_for_input(app).into_iter().enumerate() {
        let width = icon.chars().count() + 1 + label.chars().count();
        if inner_col >= start && inner_col < start + width {
            return Some(idx);
        }
        start += width + 1;
    }
    None
}

fn tab_defs_for_input(app: &App) -> Vec<(String, String)> {
    let mut defs: Vec<(String, String)> = app
        .main_tabs
        .iter()
        .map(|tab| (tab.icon.clone(), tab.title.clone()))
        .collect();
    defs.extend(app.plugin_tabs.iter().map(|tab| {
        (
            tab.icon.clone().unwrap_or_else(|| "◌".to_owned()),
            tab.title.to_uppercase(),
        )
    }));
    defs
}

fn local_nav_back(app: &mut App) {
    match app.local_nav_level {
        LocalNavLevel::Artists => {}
        LocalNavLevel::Albums => {
            app.local_nav_level = LocalNavLevel::Artists;
            app.local_nav_artist = None;
            app.local_nav_album = None;
            app.selected_local_nav_idx = 0;
        }
        LocalNavLevel::Songs => {
            app.local_nav_level = LocalNavLevel::Albums;
            app.local_nav_album = None;
            app.selected_local_nav_idx = 0;
        }
    }
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
        5
    }
}

fn selected_local_song_clone(app: &App) -> Option<crate::model::LocalSong> {
    app.local_library_window
        .get(
            app.selected_local_song
                .saturating_sub(app.local_library_offset),
        )
        .cloned()
}

fn tag_value(song: &crate::model::LocalSong, field: LocalTagField) -> String {
    match field {
        LocalTagField::Title => song.title.clone(),
        LocalTagField::Artist => song.artist.clone(),
        LocalTagField::Album => song.album.clone(),
        LocalTagField::Genre => song.genre.clone(),
        LocalTagField::Year => song.year.map(|y| y.to_string()).unwrap_or_default(),
    }
}

fn apply_tag_value(
    song: &mut crate::model::LocalSong,
    field: LocalTagField,
    value: String,
) -> Result<(), String> {
    match field {
        LocalTagField::Title => song.title = value,
        LocalTagField::Artist => song.artist = value,
        LocalTagField::Album => song.album = value,
        LocalTagField::Genre => song.genre = value,
        LocalTagField::Year => {
            song.year = if value.trim().is_empty() {
                None
            } else {
                Some(
                    value
                        .trim()
                        .parse::<u32>()
                        .map_err(|_| "year must be a number".to_string())?,
                )
            };
        }
    }
    Ok(())
}

fn open_tag_editor(app: &mut App) {
    if let Some(song) = selected_local_song_clone(app) {
        app.local_tag_editor_field = LocalTagField::Title;
        app.local_tag_edit_buffer = tag_value(&song, app.local_tag_editor_field);
        app.local_tag_editor_song = Some(song);
        app.local_tag_editor_open = true;
        app.set_flash(
            "Editing local tags: Tab switches field, Enter saves field, Esc cancels",
            3,
        );
    }
}

fn cycle_tag_field(app: &mut App) {
    if let Some(mut song) = app.local_tag_editor_song.take() {
        if let Err(err) = apply_tag_value(
            &mut song,
            app.local_tag_editor_field,
            app.local_tag_edit_buffer.clone(),
        ) {
            app.set_flash(err, 3);
        }
        app.local_tag_editor_field = app.local_tag_editor_field.next();
        app.local_tag_edit_buffer = tag_value(&song, app.local_tag_editor_field);
        app.local_tag_editor_song = Some(song);
    }
}

fn save_tag_field_or_close(app: &mut App) {
    let Some(mut song) = app.local_tag_editor_song.take() else {
        return;
    };
    if let Err(err) = apply_tag_value(
        &mut song,
        app.local_tag_editor_field,
        app.local_tag_edit_buffer.clone(),
    ) {
        app.set_flash(err, 3);
        app.local_tag_editor_song = Some(song);
        return;
    }
    match crate::core::write_local_tags(&song)
        .map_err(|e| format!("{e:#}"))
        .and_then(|_| app.storage.update_local_song(&song))
    {
        Ok(()) => {
            app.local_tag_editor_open = false;
            app.set_flash("Local tags saved", 3);
            ui_helpers::refresh_local_visible_window(app);
        }
        Err(err) => {
            app.set_flash(format!("Tag save failed: {err}"), 5);
            app.local_tag_editor_song = Some(song);
        }
    }
}

fn clear_local_filters(app: &mut App) {
    app.local_filter_genre = None;
    app.local_filter_artist = None;
    app.local_filter_album = None;
    ui_helpers::refresh_local_visible_window(app);
    app.set_flash("Local filters cleared", 2);
}

fn selected_local_filter_value(app: &App, kind: char) -> Option<String> {
    if app.local_view_mode == LocalViewMode::Flat {
        let song = selected_local_song_clone(app)?;
        return match kind {
            'g' => Some(song.genre),
            'a' => Some(song.artist),
            'b' => Some(song.album),
            _ => None,
        };
    }

    match ui_helpers::get_local_nav_items(app) {
        ui_helpers::LocalNavItems::Artists(artists) => match kind {
            'a' => artists.get(app.selected_local_nav_idx).cloned(),
            _ => None,
        },
        ui_helpers::LocalNavItems::Albums(albums) => match kind {
            'a' => app.local_nav_artist.clone(),
            'b' => albums.get(app.selected_local_nav_idx).cloned(),
            _ => None,
        },
        ui_helpers::LocalNavItems::Songs(songs) => {
            let song = songs.get(app.selected_local_nav_idx)?;
            match kind {
                'g' => Some(song.genre.clone()),
                'a' => Some(song.artist.clone()),
                'b' => Some(song.album.clone()),
                _ => None,
            }
        }
    }
}

fn set_filter_from_selected(app: &mut App, kind: char) {
    if let Some(value) = selected_local_filter_value(app, kind) {
        match kind {
            'g' => app.local_filter_genre = Some(value),
            'a' => app.local_filter_artist = Some(value),
            'b' => app.local_filter_album = Some(value),
            _ => {}
        }
        ui_helpers::refresh_local_visible_window(app);
        app.set_flash("Local filter applied (Ctrl+F or Shift+F clears)", 2);
    } else {
        app.set_flash("No matching selected field to filter", 2);
    }
}

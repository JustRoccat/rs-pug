use crate::core::CoreCmd;
use crate::model::{App, Playlist, Tab, Focus, Song};
use tokio::sync::mpsc;

pub fn create_empty_playlist(app: &mut App, name: &str) {
    app.playlists.push(Playlist {
        name: name.to_owned(),
        songs: Vec::new(),
    });
    app.playlist_expanded.push(true);
    app.selected_playlist = app.playlists.len().saturating_sub(1);
}

pub fn add_to_named_playlist(app: &mut App, song: Song, name: &str) {
    if let Some(p) = app.playlists.iter_mut().find(|p| p.name == name) {
        if p.songs.iter().any(|s| s.id == song.id) {
            app.set_flash(format!("Song already in playlist {name}"), 3);
            return;
        }
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

pub fn import_playlist_action(app: &mut App) {
    match app.storage.import_playlist_from_default() {
        Ok(mut imported) => {
            if imported.name.trim().is_empty() {
                imported.name = "Imported".to_owned();
            }
            if let Some(existing) = app.playlists.iter_mut().find(|p| p.name == imported.name) {
                let before = existing.songs.len();
                for song in imported.songs {
                    if !existing.songs.iter().any(|s| s.id == song.id) {
                        existing.songs.push(song);
                    }
                }
                let merged_name = existing.name.clone();
                let added = existing.songs.len().saturating_sub(before);
                app.set_flash(
                    format!("Imported into {} (+{} tracks)", merged_name, added),
                    4,
                );
            } else {
                let name = imported.name.clone();
                app.playlists.push(imported);
                app.playlist_expanded.push(true);
                app.selected_playlist = app.playlists.len().saturating_sub(1);
                app.set_flash(format!("Imported playlist {}", name), 4);
            }
            app.storage.save_playlists(&app.playlists).expect("Failed to save playlists");
        }
        Err(err) => app.set_flash(err, 5),
    }
}

pub fn export_selected_playlist_action(app: &mut App) {
    if let Some(playlist) = app.playlists.get(app.selected_playlist) {
        match app.storage.export_playlist_to_default(playlist) {
            Ok(path) => app.set_flash(format!("Exported to {}", path.display()), 5),
            Err(err) => app.set_flash(err, 5),
        }
    } else {
        app.set_flash("No playlist selected", 3);
    }
}

pub fn execute_context_action(app: &mut App, index: usize, cmd_tx: &mpsc::UnboundedSender<CoreCmd>) {
    if app.active_tab == Tab::Library && app.focus == Focus::Results {
        match index {
            0 => import_playlist_action(app),
            1 => export_selected_playlist_action(app),
            _ => {}
        }
        return;
    }

    if let Some(song) = app.selected_song_for_context() {
        if app.active_tab == Tab::Local {
            match index {
                0 => add_to_selected_playlist(app, song),
                1 => {
                    let name = format!("Playlist {}", app.playlists.len() + 1);
                    add_to_named_playlist(app, song, &name);
                }
                2 => remove_selected_queue_song(app),
                _ => {}
            }
        } else {
            match index {
                0 => add_to_selected_playlist(app, song),
                1 => {
                    let name = format!("Playlist {}", app.playlists.len() + 1);
                    add_to_named_playlist(app, song, &name);
                }
                2 => {
                    if let Some(dir) = app.opt_music_dirs.first() {
                        let _ = cmd_tx.send(CoreCmd::DownloadSong {
                            song: song.clone(),
                            path: dir.clone(),
                        });
                        app.set_flash(format!("Downloading {}...", song.title), 3);
                    } else {
                        app.set_flash("Please set a music directory in settings!", 3);
                    }
                }
                3 => remove_selected_queue_song(app),
                4 => remove_selected_playlist_song(app),
                _ => {}
            }
        }
        app.storage.save_playlists(&app.playlists).expect("Failed to save playlists");
    }
}

pub fn add_to_selected_playlist(app: &mut App, song: Song) {
    if app.playlists.is_empty() {
        add_to_named_playlist(app, song, "Favorites");
        return;
    }

    if let Some(pl) = app.playlists.get_mut(app.selected_playlist) {
        if pl.songs.iter().any(|s| s.id == song.id) {
            let name = pl.name.clone();
            app.set_flash(format!("Song already in {}", name), 3);
            return;
        }
        pl.songs.push(song);
        let name = pl.name.clone();
        app.set_flash(format!("Added to {}", name), 3);
    }
}

pub fn remove_selected_queue_song(app: &mut App) {
    if app.selected_queue < app.queue.len() {
        let removed = app.queue.remove(app.selected_queue);
        app.selected_queue = app.selected_queue.min(app.queue.len().saturating_sub(1));
        if let Some(song) = removed {
            app.set_flash(format!("Removed from queue: {}", song.title), 3);
        }
    }
}

pub fn remove_selected_playlist_song(app: &mut App) {
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

pub fn ensure_playlist_state(app: &mut App) {
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

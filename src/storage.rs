use std::{fs, path::PathBuf};

use crate::model::Playlist;

fn playlist_path() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        let dir = PathBuf::from(home).join(".config/rs-pug");
        let _ = fs::create_dir_all(&dir);
        return dir.join("playlists.json");
    }

    PathBuf::from("playlists.json")
}

pub fn load_playlists() -> Vec<Playlist> {
    let path = playlist_path();
    if let Ok(raw) = fs::read_to_string(path) {
        if let Ok(data) = serde_json::from_str::<Vec<Playlist>>(&raw) {
            return data;
        }
    }
    Vec::new()
}

pub fn save_playlists(playlists: &[Playlist]) {
    if let Ok(raw) = serde_json::to_string_pretty(playlists) {
        let _ = fs::write(playlist_path(), raw);
    }
}

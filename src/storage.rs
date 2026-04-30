use std::{fs, path::PathBuf};

use crate::model::{Playlist, Song, LocalSong};

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

fn recently_played_path() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        let dir = PathBuf::from(home).join(".config/rs-pug");
        let _ = fs::create_dir_all(&dir);
        return dir.join("recently_played.json");
    }

    PathBuf::from("recently_played.json")
}

pub fn load_recently_played() -> Vec<Song> {
    let path = recently_played_path();
    if let Ok(raw) = fs::read_to_string(path) {
        if let Ok(data) = serde_json::from_str::<Vec<Song>>(&raw) {
            return data;
        }
    }
    Vec::new()
}

pub fn save_recently_played(songs: &[Song]) {
    if let Ok(raw) = serde_json::to_string_pretty(songs) {
        let _ = fs::write(recently_played_path(), raw);
    }
}

pub fn import_playlist_from_default() -> Result<Playlist, String> {
    let path = if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".config/rs-pug/import_playlist.json")
    } else {
        PathBuf::from("import_playlist.json")
    };
    let raw = fs::read_to_string(&path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let template = Playlist {
                name: "Imported".to_owned(),
                songs: Vec::new(),
            };
            match serde_json::to_string_pretty(&template)
                .ok()
                .and_then(|raw| fs::write(&path, raw).ok())
            {
                Some(_) => format!(
                    "Created import template: {} (fill songs and import again)",
                    path.display()
                ),
                None => format!(
                    "Import file not found and could not create template: {}",
                    path.display()
                ),
            }
        } else {
            format!("Cannot read {}: {e}", path.display())
        }
    })?;
    serde_json::from_str::<Playlist>(&raw)
        .map_err(|e| format!("Invalid playlist JSON in {}: {e}", path.display()))
}

pub fn export_playlist_to_default(playlist: &Playlist) -> Result<PathBuf, String> {
    let base = if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".config/rs-pug/exports")
    } else {
        PathBuf::from("exports")
    };
    fs::create_dir_all(&base)
        .map_err(|e| format!("Cannot create export dir {}: {e}", base.display()))?;
    let safe_name: String = playlist
        .name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let out = base.join(format!("{safe_name}.json"));
    let raw = serde_json::to_string_pretty(playlist)
        .map_err(|e| format!("Cannot serialize playlist: {e}"))?;
    fs::write(&out, raw).map_err(|e| format!("Cannot write {}: {e}", out.display()))?;
    Ok(out)
}

fn local_library_path() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        let dir = PathBuf::from(home).join(".config/rs-pug");
        let _ = fs::create_dir_all(&dir);
        return dir.join("local_library.json");
    }

    PathBuf::from("local_library.json")
}

fn last_scanned_dirs_path() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        let dir = PathBuf::from(home).join(".config/rs-pug");
        let _ = fs::create_dir_all(&dir);
        return dir.join("last_scanned_dirs.json");
    }

    PathBuf::from("last_scanned_dirs.json")
}

pub fn load_last_scanned_dirs() -> Vec<String> {
    let path = last_scanned_dirs_path();
    if let Ok(raw) = fs::read_to_string(path) {
        if let Ok(data) = serde_json::from_str::<Vec<String>>(&raw) {
            return data;
        }
    }
    Vec::new()
}

pub fn save_last_scanned_dirs(dirs: &[String]) {
    if let Ok(raw) = serde_json::to_string_pretty(dirs) {
        let _ = fs::write(last_scanned_dirs_path(), raw);
    }
}

pub fn load_local_library() -> Vec<LocalSong> {

    let path = local_library_path();
    if let Ok(raw) = fs::read_to_string(path) {
        if let Ok(data) = serde_json::from_str::<Vec<LocalSong>>(&raw) {
            return data;
        }
    }
    Vec::new()
}

pub fn save_local_library(songs: &[LocalSong]) {
    if let Ok(raw) = serde_json::to_string_pretty(songs) {
        let _ = fs::write(local_library_path(), raw);
    }
}

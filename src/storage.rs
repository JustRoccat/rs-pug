use crate::db::DbStorage;
use crate::model::{LocalSong, Playlist, Song};
use std::{
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};

#[derive(Clone)]
pub struct Storage {
    db: Arc<Mutex<DbStorage>>,
}

impl std::fmt::Debug for Storage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Storage").finish()
    }
}

impl Storage {
    pub fn init() -> Result<Self, String> {
        let dir = Self::config_dir().map_err(|e| e.to_string())?;
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

        let db_path = dir.join("pug.db");
        let mut db = DbStorage::open(db_path).map_err(|e| e.to_string())?;

        Self::migrate_from_json(&mut db)?;
        Self::migrate_last_scanned_dirs(&mut db);

        Ok(Self {
            db: Arc::new(Mutex::new(db)),
        })
    }

    fn config_dir() -> Result<PathBuf, std::io::Error> {
        if let Ok(home) = std::env::var("HOME") {
            Ok(PathBuf::from(home).join(".config/rs-pug"))
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "HOME not set",
            ))
        }
    }

    fn json_path(filename: &str) -> PathBuf {
        Self::config_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(filename)
    }

    fn migrate_from_json(db: &mut DbStorage) -> Result<(), String> {
        let local_path = Self::json_path("local_library.json");
        let playlist_path = Self::json_path("playlists.json");
        let recent_path = Self::json_path("recently_played.json");

        let mut local_library = Vec::new();
        if let Ok(raw) = fs::read_to_string(&local_path) {
            if let Ok(data) = serde_json::from_str::<Vec<LocalSong>>(&raw) {
                local_library = data;
            }
        }

        let mut playlists = Vec::new();
        if let Ok(raw) = fs::read_to_string(&playlist_path) {
            if let Ok(data) = serde_json::from_str::<Vec<Playlist>>(&raw) {
                playlists = data;
            }
        }

        let mut recently_played = Vec::new();
        if let Ok(raw) = fs::read_to_string(&recent_path) {
            if let Ok(data) = serde_json::from_str::<Vec<Song>>(&raw) {
                recently_played = data;
            }
        }

        if !local_library.is_empty() || !playlists.is_empty() || !recently_played.is_empty() {
            db.migrate_from_json(local_library, playlists, recently_played)
                .map_err(|e| e.to_string())?;

            // Delete legacy JSON files to prevent re-migration on every startup
            let _ = fs::remove_file(Self::json_path("local_library.json"));
            let _ = fs::remove_file(Self::json_path("playlists.json"));
            let _ = fs::remove_file(Self::json_path("recently_played.json"));
        }

        Ok(())
    }

    pub fn load_local_library(&self) -> Result<Vec<LocalSong>, String> {
        self.db
            .lock()
            .unwrap()
            .load_local_songs()
            .map_err(|e| e.to_string())
    }

    pub fn fetch_local_songs_window(
        &self,
        target_index: usize,
        window_size: usize,
    ) -> Result<(Vec<LocalSong>, usize, usize), String> {
        let db = self.db.lock().unwrap();
        let total = db.get_local_songs_count().map_err(|e| e.to_string())?;

        // Center the target_index in the window
        let offset = target_index
            .saturating_sub(window_size / 2)
            .min(total.saturating_sub(window_size));

        let songs = db
            .load_local_songs_paginated(window_size, offset)
            .map_err(|e| e.to_string())?;

        Ok((songs, offset, total))
    }

    pub fn save_local_library(&self, songs: &[LocalSong]) -> Result<(), String> {
        self.db
            .lock()
            .unwrap()
            .save_local_songs_bulk(songs)
            .map_err(|e| e.to_string())
    }

    pub fn load_playlists(&self) -> Result<Vec<Playlist>, String> {
        self.db
            .lock()
            .unwrap()
            .load_playlists()
            .map_err(|e| e.to_string())
    }

    pub fn save_playlists(&self, playlists: &[Playlist]) -> Result<(), String> {
        self.db
            .lock()
            .unwrap()
            .save_playlists(playlists)
            .map_err(|e| e.to_string())
    }

    pub fn load_recently_played(&self) -> Result<Vec<Song>, String> {
        self.db
            .lock()
            .unwrap()
            .load_recently_played()
            .map_err(|e| e.to_string())
    }

    pub fn save_recently_played(&self, songs: &[Song]) -> Result<(), String> {
        self.db
            .lock()
            .unwrap()
            .save_recently_played(songs)
            .map_err(|e| e.to_string())
    }

    fn migrate_last_scanned_dirs(db: &mut DbStorage) {
        let path = Self::json_path("last_scanned_dirs.json");
        if let Ok(raw) = fs::read_to_string(&path) {
            if let Ok(data) = serde_json::from_str::<Vec<String>>(&raw) {
                let _ = db.save_last_scanned_dirs(&data);
                let _ = fs::remove_file(path);
            }
        }
    }

    pub fn save_last_scanned_dirs(&self, dirs: &[String]) {
        let _ = self.db.lock().unwrap().save_last_scanned_dirs(dirs);
    }

    pub fn import_playlist_from_default(&self) -> Result<crate::model::Playlist, String> {
        let path = Self::json_path("import_playlist.json");
        if !path.exists() {
            let template = crate::model::Playlist {
                name: "Imported".to_string(),
                songs: Vec::new(),
            };
            let raw = serde_json::to_string_pretty(&template).map_err(|e| e.to_string())?;
            fs::write(&path, raw).map_err(|e| e.to_string())?;
        }
        let raw = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        serde_json::from_str(&raw).map_err(|e| e.to_string())
    }

    pub fn export_playlist_to_default(
        &self,
        playlist: &crate::model::Playlist,
    ) -> Result<std::path::PathBuf, String> {
        let config_dir = Self::config_dir().map_err(|e| e.to_string())?;
        let exports_dir = config_dir.join("exports");
        fs::create_dir_all(&exports_dir).map_err(|e| e.to_string())?;
        let sanitized_name = playlist.name.replace(|c: char| !c.is_alphanumeric(), "_");
        let filename = format!("{}.json", sanitized_name);
        let path = exports_dir.join(filename);
        let raw = serde_json::to_string_pretty(playlist).map_err(|e| e.to_string())?;
        fs::write(&path, raw).map_err(|e| e.to_string())?;
        Ok(path)
    }
}

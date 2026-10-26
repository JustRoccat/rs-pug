use rusqlite::{Connection, Result};
use std::collections::HashMap;
use std::path::PathBuf;
const LOCAL_SONG_SELECT: &str = "SELECT path, title, artist, album, COALESCE(genre, 'Unknown'), year, duration, mtime, COALESCE(added_at, mtime), play_count, last_played FROM local_songs";
pub struct LocalSearch<'a> {
    pub query: &'a str,
    pub genre: Option<&'a str>,
    pub artist: Option<&'a str>,
    pub album: Option<&'a str>,
    pub sort: crate::model::LocalSortMode,
    pub limit: usize,
    pub offset: usize,
}
fn map_local_song_row(row: &rusqlite::Row<'_>) -> Result<crate::model::LocalSong> {
    Ok(crate::model::LocalSong {
        path: row.get(0)?,
        title: row.get(1)?,
        artist: row.get(2)?,
        album: row.get(3)?,
        genre: row.get(4)?,
        year: row.get(5)?,
        duration: row.get(6)?,
        mtime: row.get(7)?,
        added_at: row.get(8)?,
        play_count: row.get(9)?,
        last_played: row.get(10)?,
    })
}
pub struct DbStorage {
    conn: Connection,
}
impl std::fmt::Debug for DbStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DbStorage").finish()
    }
}
impl DbStorage {
    pub fn open(path: PathBuf) -> Result<Self> {
        let conn = Connection::open(path)?;
        let mut storage = Self { conn };
        storage.init_db()?;
        Ok(storage)
    }
    fn init_db(&mut self) -> Result<()> {
        self.conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;",
        )?;
        let user_version: i32 = self
            .conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if user_version == 0 {
            self.create_schema()?;
            self.ensure_search_indexes()?;
            self.rebuild_fts()?;
            self.conn.execute("PRAGMA user_version = 5", [])?;
        } else if user_version < 2 {
            self.create_app_settings_schema()?;
            self.migrate_local_metadata_columns()?;
            self.ensure_search_indexes()?;
            self.rebuild_fts()?;
            self.conn.execute("PRAGMA user_version = 5", [])?;
        } else if user_version < 4 {
            self.migrate_local_metadata_columns()?;
            self.ensure_search_indexes()?;
            self.rebuild_fts()?;
            self.conn.execute("PRAGMA user_version = 5", [])?;
        } else if user_version < 5 {
            self.ensure_search_indexes()?;
            self.rebuild_fts()?;
            self.conn.execute("PRAGMA user_version = 5", [])?;
        }
        Ok(())
    }
    fn migrate_local_metadata_columns(&self) -> Result<()> {
        let mut stmt = self.conn.prepare("PRAGMA table_info(local_songs)")?;
        let cols: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<Result<Vec<_>>>()?;
        if !cols.iter().any(|c| c == "genre") {
            self.conn.execute(
                "ALTER TABLE local_songs ADD COLUMN genre TEXT DEFAULT 'Unknown'",
                [],
            )?;
        }
        if !cols.iter().any(|c| c == "year") {
            self.conn
                .execute("ALTER TABLE local_songs ADD COLUMN year INTEGER", [])?;
        }
        if !cols.iter().any(|c| c == "added_at") {
            self.conn.execute(
                "ALTER TABLE local_songs ADD COLUMN added_at INTEGER DEFAULT 0",
                [],
            )?;
            self.conn.execute(
                "UPDATE local_songs SET added_at = COALESCE(mtime, 0) WHERE added_at = 0",
                [],
            )?;
        }
        if !cols.iter().any(|c| c == "play_count") {
            self.conn.execute(
                "ALTER TABLE local_songs ADD COLUMN play_count INTEGER NOT NULL DEFAULT 0",
                [],
            )?;
        }
        if !cols.iter().any(|c| c == "last_played") {
            self.conn
                .execute("ALTER TABLE local_songs ADD COLUMN last_played INTEGER", [])?;
        }
        Ok(())
    }
    fn create_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE local_songs (
                path TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                artist TEXT,
                album TEXT,
                genre TEXT DEFAULT 'Unknown',
                year INTEGER,
                duration REAL,
                mtime INTEGER,
                added_at INTEGER,
                play_count INTEGER NOT NULL DEFAULT 0,
                last_played INTEGER
            );
            CREATE INDEX idx_local_artist ON local_songs(artist);
            CREATE INDEX idx_local_album ON local_songs(album);
            CREATE INDEX idx_local_play_count ON local_songs(play_count DESC);
            CREATE INDEX idx_local_last_played ON local_songs(last_played);
            CREATE TABLE network_songs (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                webpage_url TEXT NOT NULL,
                uploader TEXT,
                duration REAL
            );
            CREATE TABLE playlists (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE
            );
            CREATE TABLE playlist_songs (
                playlist_id INTEGER NOT NULL,
                song_id TEXT NOT NULL,
                song_type TEXT CHECK (song_type IN ('local', 'network')),
                position INTEGER,
                PRIMARY KEY (playlist_id, song_id),
                FOREIGN KEY (playlist_id) REFERENCES playlists(id) ON DELETE CASCADE
            );
            CREATE INDEX idx_playlist_pos ON playlist_songs(playlist_id, position);
            CREATE TABLE recently_played (
                song_id TEXT NOT NULL,
                song_type TEXT CHECK (song_type IN ('local', 'network')),
                played_at INTEGER NOT NULL
            );
            CREATE INDEX idx_recent_time ON recently_played(played_at);
            CREATE TABLE app_settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );",
        )
    }
    fn ensure_search_indexes(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_local_title ON local_songs(title);
             CREATE INDEX IF NOT EXISTS idx_local_genre ON local_songs(genre);
             CREATE INDEX IF NOT EXISTS idx_local_year ON local_songs(year);
             CREATE VIRTUAL TABLE IF NOT EXISTS local_songs_fts USING fts5(title, artist, album, genre, tokenize='unicode61 remove_diacritics 1');
             DROP TRIGGER IF EXISTS local_songs_ai;
             DROP TRIGGER IF EXISTS local_songs_ad;
             DROP TRIGGER IF EXISTS local_songs_au;
             CREATE TRIGGER local_songs_ai AFTER INSERT ON local_songs BEGIN
                 INSERT INTO local_songs_fts(rowid, title, artist, album, genre) VALUES (new.rowid, new.title, new.artist, new.album, new.genre);
             END;
             CREATE TRIGGER local_songs_ad AFTER DELETE ON local_songs BEGIN
                 DELETE FROM local_songs_fts WHERE rowid = old.rowid;
             END;
             CREATE TRIGGER local_songs_au AFTER UPDATE ON local_songs BEGIN
                 DELETE FROM local_songs_fts WHERE rowid = old.rowid;
                 INSERT INTO local_songs_fts(rowid, title, artist, album, genre) VALUES (new.rowid, new.title, new.artist, new.album, new.genre);
             END;",
        )
    }
    fn rebuild_fts(&self) -> Result<()> {
        self.conn.execute_batch(
            "DELETE FROM local_songs_fts;
             INSERT INTO local_songs_fts(rowid, title, artist, album, genre) SELECT rowid, title, artist, album, genre FROM local_songs;",
        )
    }
    fn create_app_settings_schema(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS app_settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        )?;
        Ok(())
    }
    pub fn migrate_from_json(
        &mut self,
        local_library: Vec<crate::model::LocalSong>,
        playlists: Vec<crate::model::Playlist>,
        recently_played: Vec<crate::model::Song>,
    ) -> Result<()> {
        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx
                .prepare(
                    "INSERT OR IGNORE INTO local_songs (path, title, artist, album, genre, year, duration, mtime, added_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
                )?;
            for song in local_library {
                stmt.execute((
                    song.path,
                    song.title,
                    song.artist,
                    song.album,
                    song.genre,
                    song.year,
                    song.duration,
                    song.mtime,
                    song.added_at,
                ))?;
            }
        }
        {
            let mut stmt_song = tx
                .prepare(
                    "INSERT OR IGNORE INTO playlist_songs (playlist_id, song_id, song_type, position) VALUES (?, ?, ?, ?)",
                )?;
            for playlist in playlists.iter() {
                tx.execute(
                    "INSERT OR IGNORE INTO playlists (name) VALUES (?)",
                    [&playlist.name],
                )?;
                let playlist_id: i64 = tx.query_row(
                    "SELECT id FROM playlists WHERE name = ?",
                    [&playlist.name],
                    |row| row.get(0),
                )?;
                for (song_pos, song) in playlist.songs.iter().enumerate() {
                    let song_type = if song.id.contains('/') || song.id.contains('\\') {
                        "local"
                    } else {
                        "network"
                    };
                    stmt_song.execute((playlist_id, &song.id, song_type, song_pos as i64))?;
                }
            }
        }
        {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            let mut stmt = tx.prepare(
                "INSERT INTO recently_played (song_id, song_type, played_at) VALUES (?, ?, ?)",
            )?;
            for song in recently_played {
                let song_type = if song.id.contains('/') || song.id.contains('\\') {
                    "local"
                } else {
                    "network"
                };
                stmt.execute((song.id, song_type, now))?;
            }
        }
        tx.commit()?;
        Ok(())
    }
    pub fn get_local_songs_count(&self) -> Result<usize> {
        let count: usize = self
            .conn
            .query_row("SELECT COUNT(*) FROM local_songs", [], |row| row.get(0))?;
        Ok(count)
    }
    #[cfg(test)]
    pub fn load_local_songs_paginated(
        &self,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<crate::model::LocalSong>> {
        let sql = format!("{LOCAL_SONG_SELECT} ORDER BY path LIMIT ? OFFSET ?");
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map([limit as i64, offset as i64], map_local_song_row)?;
        rows.collect()
    }
    pub fn top_played_local_songs(&self, limit: usize) -> Result<Vec<crate::model::LocalSong>> {
        let sql = format!(
            "{LOCAL_SONG_SELECT} WHERE play_count > 0 ORDER BY play_count DESC, last_played DESC LIMIT ?"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map([limit as i64], map_local_song_row)?;
        rows.collect()
    }
    pub fn recently_added_local_songs(&self, limit: usize) -> Result<Vec<crate::model::LocalSong>> {
        let sql = format!("{LOCAL_SONG_SELECT} ORDER BY added_at DESC LIMIT ?");
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map([limit as i64], map_local_song_row)?;
        rows.collect()
    }
    pub fn stale_local_songs(
        &self,
        stale_before: i64,
        limit: usize,
    ) -> Result<Vec<crate::model::LocalSong>> {
        let sql = format!(
            "{LOCAL_SONG_SELECT} WHERE last_played IS NULL OR last_played < ?1
             ORDER BY COALESCE(last_played, 0) ASC LIMIT ?2"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map((stale_before, limit as i64), map_local_song_row)?;
        rows.collect()
    }
    pub fn record_local_play(&self, path: &str) -> Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        self.conn.execute(
            "UPDATE local_songs SET play_count = play_count + 1, last_played = ?2 WHERE path = ?1",
            (path, now),
        )?;
        Ok(())
    }
    pub fn update_local_song(&mut self, song: &crate::model::LocalSong) -> Result<()> {
        self.conn
            .execute(
                "UPDATE local_songs SET title = ?2, artist = ?3, album = ?4, genre = ?5, year = ?6, duration = ?7, mtime = ?8, added_at = ?9 WHERE path = ?1",
                (
                    &song.path,
                    &song.title,
                    &song.artist,
                    &song.album,
                    &song.genre,
                    song.year,
                    song.duration,
                    song.mtime,
                    song.added_at,
                ),
            )?;
        Ok(())
    }
    pub fn save_local_songs_bulk(&mut self, songs: &[crate::model::LocalSong]) -> Result<()> {
        let tx = self.conn.transaction()?;
        // Keep play counts
        let preserved: HashMap<String, (u32, Option<i64>)> = {
            let mut stmt = tx.prepare("SELECT path, play_count, last_played FROM local_songs")?;
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, u32>(1)?,
                    row.get::<_, Option<i64>>(2)?,
                ))
            })?;
            rows.collect::<Result<Vec<_>>>()?
                .into_iter()
                .map(|(path, play_count, last_played)| (path, (play_count, last_played)))
                .collect()
        };
        tx.execute("DELETE FROM local_songs", [])?;
        {
            let mut stmt = tx
                .prepare(
                    "INSERT INTO local_songs (path, title, artist, album, genre, year, duration, mtime, added_at, play_count, last_played)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                )?;
            for s in songs {
                let (play_count, last_played) =
                    preserved.get(&s.path).copied().unwrap_or((0, None));
                stmt.execute((
                    &s.path,
                    &s.title,
                    &s.artist,
                    &s.album,
                    &s.genre,
                    s.year,
                    s.duration,
                    s.mtime,
                    s.added_at,
                    play_count,
                    last_played,
                ))?;
            }
        }
        tx.commit()?;
        Ok(())
    }
    pub fn upsert_playlist(&mut self, playlist: &crate::model::Playlist) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO playlists (name) VALUES (?1) ON CONFLICT(name) DO NOTHING",
            [&playlist.name],
        )?;
        let playlist_id: i64 = tx.query_row(
            "SELECT id FROM playlists WHERE name = ?1",
            [&playlist.name],
            |row| row.get(0),
        )?;
        tx.execute(
            "DELETE FROM playlist_songs WHERE playlist_id = ?1",
            [playlist_id],
        )?;
        {
            let mut stmt_network = tx.prepare(
                "INSERT INTO network_songs (id, title, webpage_url, uploader, duration)
                 VALUES (?, ?, ?, ?, ?)
                 ON CONFLICT(id) DO UPDATE SET
                    title = excluded.title,
                    webpage_url = excluded.webpage_url,
                    uploader = excluded.uploader,
                    duration = excluded.duration",
            )?;
            let mut stmt_song = tx.prepare(
                "INSERT INTO playlist_songs (playlist_id, song_id, song_type, position) VALUES (?, ?, ?, ?)",
            )?;
            for (pos, song) in playlist.songs.iter().enumerate() {
                let song_type = if song.id.contains('/') || song.id.contains('\\') {
                    "local"
                } else {
                    "network"
                };
                if song_type == "network" {
                    stmt_network.execute((
                        &song.id,
                        &song.title,
                        &song.webpage_url,
                        &song.uploader,
                        song.duration,
                    ))?;
                }
                stmt_song.execute((playlist_id, &song.id, song_type, pos as i64))?;
            }
        }
        tx.commit()?;
        Ok(())
    }
    pub fn load_playlists(&self) -> Result<Vec<crate::model::Playlist>> {
        let mut stmt = self.conn.prepare("SELECT id, name FROM playlists")?;
        let playlist_rows = stmt.query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut playlists = Vec::new();
        for p_row in playlist_rows {
            let (p_id, name) = p_row?;
            let mut s_stmt = self
                .conn
                .prepare(
                    "SELECT song_id, song_type FROM playlist_songs WHERE playlist_id = ? ORDER BY position",
                )?;
            let s_rows = s_stmt.query_map([p_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            let mut songs = Vec::new();
            for s_row in s_rows {
                let (s_id, s_type) = s_row?;
                if s_type == "local" {
                    if let Ok(song) = self.conn.query_row(
                        "SELECT title, artist, album, duration FROM local_songs WHERE path = ?",
                        [s_id.clone()],
                        |row| {
                            Ok(crate::model::Song {
                                id: s_id.clone(),
                                title: row.get(0)?,
                                webpage_url: s_id.clone(),
                                uploader: Some(row.get(1)?),
                                duration: row.get(3)?,
                            })
                        },
                    ) {
                        songs.push(song);
                    }
                } else {
                    if let Ok(song) = self
                        .conn
                        .query_row(
                            "SELECT title, webpage_url, uploader, duration FROM network_songs WHERE id = ?",
                            [s_id.clone()],
                            |row| {
                                Ok(crate::model::Song {
                                    id: s_id.clone(),
                                    title: row.get(0)?,
                                    webpage_url: row.get(1)?,
                                    uploader: row.get(2)?,
                                    duration: row.get(3)?,
                                })
                            },
                        )
                    {
                        songs.push(song);
                    }
                }
            }
            playlists.push(crate::model::Playlist { name, songs });
        }
        Ok(playlists)
    }
    pub fn save_playlists(&mut self, playlists: &[crate::model::Playlist]) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM playlists", [])?;
        for playlist in playlists {
            tx.execute("INSERT INTO playlists (name) VALUES (?)", [&playlist.name])?;
            let p_id: i64 = tx.query_row(
                "SELECT id FROM playlists WHERE name = ?",
                [&playlist.name],
                |row| row.get(0),
            )?;
            let mut s_stmt = tx
                .prepare(
                    "INSERT INTO playlist_songs (playlist_id, song_id, song_type, position) VALUES (?, ?, ?, ?)",
                )?;
            for (pos, song) in playlist.songs.iter().enumerate() {
                let s_type = if song.id.contains('/') || song.id.contains('\\') {
                    "local"
                } else {
                    "network"
                };
                if s_type == "network" {
                    tx.execute(
                        "INSERT INTO network_songs (id, title, webpage_url, uploader, duration)
                         VALUES (?, ?, ?, ?, ?)
                         ON CONFLICT(id) DO UPDATE SET
                            title = excluded.title,
                            webpage_url = excluded.webpage_url,
                            uploader = excluded.uploader,
                            duration = excluded.duration",
                        (
                            &song.id,
                            &song.title,
                            &song.webpage_url,
                            &song.uploader,
                            song.duration,
                        ),
                    )?;
                }
                s_stmt.execute((p_id, &song.id, s_type, pos as i64))?;
            }
        }
        tx.commit()?;
        Ok(())
    }
    pub fn load_recently_played(&self) -> Result<Vec<crate::model::Song>> {
        let mut stmt = self
            .conn
            .prepare("SELECT song_id, song_type FROM recently_played ORDER BY played_at DESC")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut songs = Vec::new();
        for row in rows {
            let (s_id, s_type) = row?;
            if s_type == "local" {
                if let Ok(song) = self.conn.query_row(
                    "SELECT title, artist, album, duration FROM local_songs WHERE path = ?",
                    [s_id.clone()],
                    |row| {
                        Ok(crate::model::Song {
                            id: s_id.clone(),
                            title: row.get(0)?,
                            webpage_url: s_id.clone(),
                            uploader: Some(row.get(1)?),
                            duration: row.get(3)?,
                        })
                    },
                ) {
                    songs.push(song);
                }
            } else {
                if let Ok(song) = self.conn.query_row(
                    "SELECT title, webpage_url, uploader, duration FROM network_songs WHERE id = ?",
                    [s_id.clone()],
                    |row| {
                        Ok(crate::model::Song {
                            id: s_id.clone(),
                            title: row.get(0)?,
                            webpage_url: row.get(1)?,
                            uploader: row.get(2)?,
                            duration: row.get(3)?,
                        })
                    },
                ) {
                    songs.push(song);
                }
            }
        }
        Ok(songs)
    }
    pub fn save_last_scanned_dirs(&mut self, dirs: &[String]) -> Result<()> {
        let raw = serde_json::to_string(dirs).unwrap_or_else(|_| "[]".to_owned());
        self.conn.execute(
            "INSERT INTO app_settings (key, value) VALUES ('last_scanned_dirs', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [raw],
        )?;
        Ok(())
    }
    pub fn save_recently_played(&mut self, songs: &[crate::model::Song]) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM recently_played", [])?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO recently_played (song_id, song_type, played_at) VALUES (?, ?, ?)",
            )?;
            for song in songs {
                let s_type = if song.id.contains('/') || song.id.contains('\\') {
                    "local"
                } else {
                    "network"
                };
                stmt.execute((&song.id, s_type, now))?;
            }
        }
        tx.commit()?;
        Ok(())
    }
    pub fn save_session(&self, raw: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO app_settings (key, value) VALUES ('session', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [raw],
        )?;
        Ok(())
    }
    pub fn load_session(&self) -> Result<Option<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT value FROM app_settings WHERE key = 'session'")?;
        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            let raw: String = row.get(0)?;
            Ok(Some(raw))
        } else {
            Ok(None)
        }
    }
    pub fn load_songs_by_paths(&self, paths: &[String]) -> Result<Vec<crate::model::LocalSong>> {
        if paths.is_empty() {
            return Ok(Vec::new());
        }
        let mut out = Vec::new();
        for chunk in paths.chunks(400) {
            let placeholders = chunk.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            let sql = format!("{LOCAL_SONG_SELECT} WHERE path IN ({placeholders})");
            let mut stmt = self.conn.prepare(&sql)?;
            let params: Vec<&dyn rusqlite::ToSql> =
                chunk.iter().map(|p| p as &dyn rusqlite::ToSql).collect();
            let rows = stmt.query_map(params.as_slice(), map_local_song_row)?;
            out.extend(rows.collect::<Result<Vec<_>>>()?);
        }
        Ok(out)
    }
    pub fn search_local_songs(
        &self,
        search: LocalSearch<'_>,
    ) -> Result<(Vec<crate::model::LocalSong>, usize)> {
        let order = match search.sort {
            crate::model::LocalSortMode::Title => "title COLLATE NOCASE, path",
            crate::model::LocalSortMode::Artist => "artist COLLATE NOCASE, title COLLATE NOCASE",
            crate::model::LocalSortMode::Album => "album COLLATE NOCASE, title COLLATE NOCASE",
            crate::model::LocalSortMode::Year => "year, title COLLATE NOCASE",
            crate::model::LocalSortMode::DateAdded => "added_at DESC, title COLLATE NOCASE",
        };
        let trimmed = search.query.trim();
        if trimmed.is_empty()
            && search.genre.is_none()
            && search.artist.is_none()
            && search.album.is_none()
        {
            let total = self.get_local_songs_count()?;
            let sql = format!("{LOCAL_SONG_SELECT} ORDER BY {order} LIMIT ? OFFSET ?");
            let mut stmt = self.conn.prepare(&sql)?;
            let rows = stmt.query_map(
                [search.limit as i64, search.offset as i64],
                map_local_song_row,
            )?;
            return Ok((rows.collect::<Result<Vec<_>>>()?, total));
        }
        if !trimmed.is_empty() {
            if let Ok(result) = self.search_local_songs_fts(&search, order) {
                return Ok(result);
            }
        }
        self.search_local_songs_like(&search, order)
    }
    fn search_local_songs_fts(
        &self,
        search: &LocalSearch<'_>,
        order: &str,
    ) -> Result<(Vec<crate::model::LocalSong>, usize)> {
        let fts_query = fts_escape(search.query);
        let mut filters = Vec::new();
        if search.genre.is_some() {
            filters.push("local_songs.genre = ?");
        }
        if search.artist.is_some() {
            filters.push("local_songs.artist = ?");
        }
        if search.album.is_some() {
            filters.push("local_songs.album = ?");
        }
        let extra = if filters.is_empty() {
            String::new()
        } else {
            format!(" AND {}", filters.join(" AND "))
        };
        let select = format!(
            "{LOCAL_SONG_SELECT} JOIN local_songs_fts ON local_songs.rowid = local_songs_fts.rowid WHERE local_songs_fts MATCH ?{extra} ORDER BY {order} LIMIT ? OFFSET ?"
        );
        let count_sql = format!(
            "SELECT COUNT(*) FROM local_songs JOIN local_songs_fts ON local_songs.rowid = local_songs_fts.rowid WHERE local_songs_fts MATCH ?{extra}"
        );
        let genre_owned = search.genre.map(str::to_owned);
        let artist_owned = search.artist.map(str::to_owned);
        let album_owned = search.album.map(str::to_owned);
        let filter_values: Vec<String> = [&genre_owned, &artist_owned, &album_owned]
            .iter()
            .filter_map(|v| (*v).clone())
            .collect();
        let count_params: Vec<&dyn rusqlite::ToSql> = {
            let mut params: Vec<&dyn rusqlite::ToSql> = vec![&fts_query];
            for value in &filter_values {
                params.push(value);
            }
            params
        };
        let total: usize = self
            .conn
            .prepare(&count_sql)?
            .query_row(count_params.as_slice(), |row| row.get(0))?;
        let mut stmt = self.conn.prepare(&select)?;
        let mut params: Vec<&dyn rusqlite::ToSql> = vec![&fts_query];
        for value in &filter_values {
            params.push(value);
        }
        let limit_i = search.limit as i64;
        let offset_i = search.offset as i64;
        params.push(&limit_i);
        params.push(&offset_i);
        let rows = stmt.query_map(params.as_slice(), map_local_song_row)?;
        Ok((rows.collect::<Result<Vec<_>>>()?, total))
    }
    fn search_local_songs_like(
        &self,
        search: &LocalSearch<'_>,
        order: &str,
    ) -> Result<(Vec<crate::model::LocalSong>, usize)> {
        let mut conditions = Vec::new();
        let mut values: Vec<String> = Vec::new();
        if !search.query.trim().is_empty() {
            let pattern = format!("%{}%", search.query.trim());
            conditions.push(
                "(local_songs.title LIKE ? ESCAPE '\\' OR local_songs.artist LIKE ? ESCAPE '\\' OR local_songs.album LIKE ? ESCAPE '\\' OR local_songs.genre LIKE ? ESCAPE '\\' OR CAST(local_songs.year AS TEXT) LIKE ? ESCAPE '\\')".to_owned(),
            );
            for _ in 0..5 {
                values.push(pattern.clone());
            }
        }
        if let Some(value) = search.genre {
            conditions.push("local_songs.genre = ?".to_owned());
            values.push(value.to_owned());
        }
        if let Some(value) = search.artist {
            conditions.push("local_songs.artist = ?".to_owned());
            values.push(value.to_owned());
        }
        if let Some(value) = search.album {
            conditions.push("local_songs.album = ?".to_owned());
            values.push(value.to_owned());
        }
        let filter = if conditions.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", conditions.join(" AND "))
        };
        let count_sql = format!("SELECT COUNT(*) FROM local_songs{filter}");
        let count_params: Vec<&dyn rusqlite::ToSql> =
            values.iter().map(|v| v as &dyn rusqlite::ToSql).collect();
        let total: usize = self
            .conn
            .prepare(&count_sql)?
            .query_row(count_params.as_slice(), |row| row.get(0))?;
        let select = format!("{LOCAL_SONG_SELECT}{filter} ORDER BY {order} LIMIT ? OFFSET ?");
        let mut stmt = self.conn.prepare(&select)?;
        let mut params: Vec<&dyn rusqlite::ToSql> =
            values.iter().map(|v| v as &dyn rusqlite::ToSql).collect();
        let limit_i = search.limit as i64;
        let offset_i = search.offset as i64;
        params.push(&limit_i);
        params.push(&offset_i);
        let rows = stmt.query_map(params.as_slice(), map_local_song_row)?;
        Ok((rows.collect::<Result<Vec<_>>>()?, total))
    }
}

fn fts_escape(query: &str) -> String {
    let tokens: Vec<String> = query
        .split_whitespace()
        .map(|token| {
            let cleaned: String = token
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
                .collect();
            if cleaned.is_empty() {
                String::new()
            } else {
                format!("\"{}\"", cleaned.replace('"', "\"\""))
            }
        })
        .filter(|token| token.len() > 2)
        .collect();
    if tokens.is_empty() {
        format!(
            "\"{}\"",
            query
                .replace('"', "\"\"")
                .trim()
                .chars()
                .take(64)
                .collect::<String>()
        )
    } else {
        tokens.join(" AND ")
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::LocalSong;
    use tempfile::NamedTempFile;
    fn setup_db() -> (DbStorage, NamedTempFile) {
        let file = NamedTempFile::new().unwrap();
        let db = DbStorage::open(file.path().to_path_buf()).unwrap();
        (db, file)
    }
    #[test]
    fn test_pagination() {
        let (db, _file) = setup_db();
        let songs = vec![
            LocalSong {
                path: "/1".to_string(),
                title: "T1".to_string(),
                artist: "A1".to_string(),
                album: "Al1".to_string(),
                genre: "G1".to_string(),
                year: Some(2001),
                duration: 100.0,
                mtime: 1,
                added_at: 1,
                play_count: 0,
                last_played: None,
            },
            LocalSong {
                path: "/2".to_string(),
                title: "T2".to_string(),
                artist: "A2".to_string(),
                album: "Al2".to_string(),
                genre: "G2".to_string(),
                year: Some(2002),
                duration: 200.0,
                mtime: 2,
                added_at: 2,
                play_count: 0,
                last_played: None,
            },
            LocalSong {
                path: "/3".to_string(),
                title: "T3".to_string(),
                artist: "A3".to_string(),
                album: "Al3".to_string(),
                genre: "G3".to_string(),
                year: Some(2003),
                duration: 300.0,
                mtime: 3,
                added_at: 3,
                play_count: 0,
                last_played: None,
            },
            LocalSong {
                path: "/4".to_string(),
                title: "T4".to_string(),
                artist: "A4".to_string(),
                album: "Al4".to_string(),
                genre: "G4".to_string(),
                year: Some(2004),
                duration: 400.0,
                mtime: 4,
                added_at: 4,
                play_count: 0,
                last_played: None,
            },
            LocalSong {
                path: "/5".to_string(),
                title: "T5".to_string(),
                artist: "A5".to_string(),
                album: "Al5".to_string(),
                genre: "G5".to_string(),
                year: Some(2005),
                duration: 500.0,
                mtime: 5,
                added_at: 5,
                play_count: 0,
                last_played: None,
            },
        ];
        let mut db = db;
        db.save_local_songs_bulk(&songs).unwrap();
        assert_eq!(db.get_local_songs_count().unwrap(), 5);
        let page1 = db.load_local_songs_paginated(2, 0).unwrap();
        assert_eq!(page1.len(), 2);
        assert_eq!(page1[0].path, "/1");
        assert_eq!(page1[1].path, "/2");
        let page2 = db.load_local_songs_paginated(2, 2).unwrap();
        assert_eq!(page2.len(), 2);
        assert_eq!(page2[0].path, "/3");
        assert_eq!(page2[1].path, "/4");
        let page3 = db.load_local_songs_paginated(2, 4).unwrap();
        assert_eq!(page3.len(), 1);
        assert_eq!(page3[0].path, "/5");
        let page4 = db.load_local_songs_paginated(2, 6).unwrap();
        assert_eq!(page4.len(), 0);
    }
    #[test]
    fn test_empty_db_pagination() {
        let (db, _file) = setup_db();
        assert_eq!(db.get_local_songs_count().unwrap(), 0);
        let songs = db.load_local_songs_paginated(10, 0).unwrap();
        assert_eq!(songs.len(), 0);
    }
}

use crate::config;
use crate::model::{
    App, Focus, LocalNavLevel, LocalSortMode, LocalViewMode, PlayerState, RepeatMode, Tab,
};
use crossterm::event::{KeyCode, KeyModifiers};
pub fn local_visible_songs(app: &App) -> Vec<crate::model::LocalSong> {
    let mut songs = app.storage.load_local_library().unwrap_or_default();
    let q = app.search.query.to_lowercase();
    songs
        .retain(|s| {
            (q.is_empty() || s.title.to_lowercase().contains(&q)
                || s.artist.to_lowercase().contains(&q)
                || s.album.to_lowercase().contains(&q)
                || s.genre.to_lowercase().contains(&q)
                || s.year.map(|y| y.to_string().contains(&q)).unwrap_or(false))
                && app.local.filter_genre.as_ref().map(|v| &s.genre == v).unwrap_or(true)
                && app
                    .local
                    .filter_artist
                    .as_ref()
                    .map(|v| &s.artist == v)
                    .unwrap_or(true)
                && app.local.filter_album.as_ref().map(|v| &s.album == v).unwrap_or(true)
        });
    match app.local.sort_mode {
        LocalSortMode::Title => {
            songs.sort_by(|a, b| crate::utils::natural_compare(&a.title, &b.title))
        }
        LocalSortMode::Artist => {
            songs
                .sort_by(|a, b| {
                    crate::utils::natural_compare(&a.artist, &b.artist)
                        .then_with(|| crate::utils::natural_compare(&a.title, &b.title))
                })
        }
        LocalSortMode::Album => {
            songs
                .sort_by(|a, b| {
                    crate::utils::natural_compare(&a.album, &b.album)
                        .then_with(|| crate::utils::natural_compare(&a.title, &b.title))
                })
        }
        LocalSortMode::Year => {
            songs.sort_by_key(|s| (s.year.unwrap_or(0), s.title.clone()))
        }
        LocalSortMode::DateAdded => {
            songs
                .sort_by(|a, b| {
                    b.added_at
                        .cmp(&a.added_at)
                        .then_with(|| crate::utils::natural_compare(&a.title, &b.title))
                })
        }
    }
    songs
}
pub fn refresh_local_visible_window(app: &mut App) {
    let songs = local_visible_songs(app);
    app.local.total = songs.len();
    app.local.offset = 0;
    app.local.window = songs;
    app.local.selected_song = app
        .local
        .selected_song
        .min(app.local.total.saturating_sub(1));
    app.local.selected_nav_idx = 0;
}
pub enum LocalNavItems<'a> {
    Artists(Vec<String>),
    Albums(Vec<String>),
    Songs(Vec<&'a crate::model::LocalSong>),
}
pub const MAX_OPTIONS_INDEX: usize = 11;
impl<'a> LocalNavItems<'a> {
    pub fn len(&self) -> usize {
        match self {
            LocalNavItems::Artists(v) => v.len(),
            LocalNavItems::Albums(v) => v.len(),
            LocalNavItems::Songs(v) => v.len(),
        }
    }
}
pub fn get_local_nav_items(app: &App) -> LocalNavItems<'_> {
    match app.local.nav_level {
        LocalNavLevel::Artists => {
            let mut artists: Vec<String> = app
                .local
                .window
                .iter()
                .map(|s| s.artist.clone())
                .collect();
            artists.sort_by(|a, b| crate::utils::natural_compare(a, b));
            artists.dedup();
            LocalNavItems::Artists(artists)
        }
        LocalNavLevel::Albums => {
            let artist = app.local.nav_artist.as_deref().unwrap_or("Unknown");
            let mut albums: Vec<String> = app
                .local
                .window
                .iter()
                .filter(|s| s.artist == artist)
                .map(|s| s.album.clone())
                .collect();
            albums.sort_by(|a, b| crate::utils::natural_compare(a, b));
            albums.dedup();
            LocalNavItems::Albums(albums)
        }
        LocalNavLevel::Songs => {
            let artist = app.local.nav_artist.as_deref().unwrap_or("Unknown");
            let album = app.local.nav_album.as_deref().unwrap_or("Unknown");
            let mut songs: Vec<&crate::model::LocalSong> = app
                .local
                .window
                .iter()
                .filter(|s| s.artist == artist && s.album == album)
                .collect();
            songs.sort_by(|a, b| crate::utils::natural_compare(&a.title, &b.title));
            LocalNavItems::Songs(songs)
        }
    }
}
pub fn player_state_label(state: PlayerState) -> &'static str {
    match state {
        PlayerState::Idle => "idle",
        PlayerState::Searching => "searching",
        PlayerState::Playing => "playing",
        PlayerState::Paused => "paused",
    }
}
pub fn describe_key_event_with_modifiers(
    code: &KeyCode,
    modifiers: KeyModifiers,
) -> String {
    match code {
        KeyCode::Char(c) => {
            let c = if modifiers.contains(KeyModifiers::SHIFT) {
                c.to_ascii_uppercase()
            } else {
                *c
            };
            format!("char:{c}")
        }
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
pub fn describe_key_event_labels(
    code: &KeyCode,
    modifiers: KeyModifiers,
) -> Vec<String> {
    let primary = describe_key_event_with_modifiers(code, modifiers);
    let Some(alias) = toggled_ascii_char_label(code, primary.as_str()) else {
        return vec![primary];
    };
    if alias == primary { vec![primary] } else { vec![primary, alias] }
}
fn toggled_ascii_char_label(code: &KeyCode, primary: &str) -> Option<String> {
    let KeyCode::Char(c) = code else {
        return None;
    };
    if !c.is_ascii_alphabetic() {
        return None;
    }
    let current = primary.strip_prefix("char:")?.chars().next()?;
    let toggled = if current.is_ascii_uppercase() {
        current.to_ascii_lowercase()
    } else {
        current.to_ascii_uppercase()
    };
    Some(format!("char:{toggled}"))
}
pub fn scroll_selection(app: &mut App, delta: isize, local_nav_len: usize) {
    match app.active_tab {
        Tab::Discover => {
            match app.focus {
                Focus::Results => {
                    let len = app.search.results.len();
                    if len > 0 {
                        app.search.selected_result = ((app.search.selected_result
                            as isize + delta)
                            .clamp(0, len as isize - 1)) as usize;
                    }
                }
                Focus::Queue => {
                    let len = app.queue.len();
                    if len > 0 {
                        app.selected_queue = ((app.selected_queue as isize + delta)
                            .clamp(0, len as isize - 1)) as usize;
                    }
                }
                Focus::Search => {}
            }
        }
        Tab::Albums => {
            match app.focus {
                Focus::Results => {
                    let total_items: usize = app
                        .albums
                        .results
                        .iter()
                        .enumerate()
                        .map(|(i, a)| {
                            1
                                + if app.albums.expanded.get(i).copied().unwrap_or(false) {
                                    a.songs.len()
                                } else {
                                    0
                                }
                        })
                        .sum();
                    if total_items > 0 {
                        app.albums.selected_result = ((app.albums.selected_result
                            as isize + delta)
                            .clamp(0, total_items as isize - 1)) as usize;
                    }
                }
                Focus::Queue => {
                    let len = app.queue.len();
                    if len > 0 {
                        app.selected_queue = ((app.selected_queue as isize + delta)
                            .clamp(0, len as isize - 1)) as usize;
                    }
                }
                Focus::Search => {}
            }
        }
        Tab::Library => {
            match app.focus {
                Focus::Results => {
                    let len = app.playlists.playlists.len();
                    if len > 0 {
                        app.playlists.selected_playlist = ((app
                            .playlists
                            .selected_playlist as isize + delta)
                            .clamp(0, len as isize - 1)) as usize;
                        app.playlists.selected_song = 0;
                    }
                }
                Focus::Queue => {
                    if let Some(p) = app
                        .playlists
                        .playlists
                        .get(app.playlists.selected_playlist)
                    {
                        let len = p.songs.len();
                        if len > 0 {
                            app.playlists.selected_song = ((app.playlists.selected_song
                                as isize + delta)
                                .clamp(0, len as isize - 1)) as usize;
                        }
                    }
                }
                Focus::Search => {}
            }
        }
        Tab::Local => {
            match app.focus {
                Focus::Results => {
                    if app.local.view_mode == LocalViewMode::Flat {
                        let len = app.local.total;
                        if len > 0 {
                            app.local.selected_song = ((app.local.selected_song as isize
                                + delta)
                                .clamp(0, len as isize - 1)) as usize;
                        }
                    } else {
                        let len = local_nav_len;
                        if len > 0 {
                            app.local.selected_nav_idx = ((app.local.selected_nav_idx
                                as isize + delta)
                                .clamp(0, len as isize - 1)) as usize;
                        }
                    }
                }
                Focus::Queue => {
                    let len = app.queue.len();
                    if len > 0 {
                        app.selected_queue = ((app.selected_queue as isize + delta)
                            .clamp(0, len as isize - 1)) as usize;
                    }
                }
                Focus::Search => {}
            }
        }
        Tab::Options => {
            app.options_index = ((app.options_index as isize + delta)
                .clamp(0, MAX_OPTIONS_INDEX as isize)) as usize;
        }
    }
    if app.active_tab == Tab::Local {
        update_local_library_window(app);
    }
}
pub fn update_local_library_window(app: &mut App) {
    let start = app.local.offset;
    let end = start + app.local.window.len();
    if app.local.selected_song < start || app.local.selected_song >= end {
        if app.search.query.is_empty() && app.local.filter_genre.is_none()
            && app.local.filter_artist.is_none() && app.local.filter_album.is_none()
            && app.local.sort_mode == LocalSortMode::Title
        {
            if let Ok((window, offset, _total)) = app
                .storage
                .fetch_local_songs_window(app.local.selected_song, 200)
            {
                app.local.window = window;
                app.local.offset = offset;
            }
        } else {
            refresh_local_visible_window(app);
        }
    }
}
pub fn next_theme(theme: config::Theme) -> config::Theme {
    let available = config::get_available_themes();
    let current_str = config::theme_to_str(&theme);
    let pos = available.iter().position(|s| s == &current_str).unwrap_or(0);
    let next_pos = (pos + 1) % available.len();
    config::theme_from_str(&available[next_pos])
}
pub fn prev_theme(theme: config::Theme) -> config::Theme {
    let available = config::get_available_themes();
    let current_str = config::theme_to_str(&theme);
    let pos = available.iter().position(|s| s == &current_str).unwrap_or(0);
    let prev_pos = (pos + available.len() - 1) % available.len();
    config::theme_from_str(&available[prev_pos])
}
pub fn prev_repeat_mode(mode: RepeatMode) -> RepeatMode {
    match mode {
        RepeatMode::Off => RepeatMode::All,
        RepeatMode::One => RepeatMode::Off,
        RepeatMode::All => RepeatMode::One,
    }
}
pub fn pseudo_shuffle<T>(items: &mut [T]) {
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
pub fn shuffle_queue_keep_current(app: &mut App) {
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
pub fn cycle_keybind_char(current: &str, delta: isize) -> String {
    const POOL: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789[]-=/;',.";
    let c = current.chars().next().unwrap_or('a');
    let pos = POOL
        .iter()
        .position(|ch| *ch as char == c.to_ascii_lowercase())
        .unwrap_or(0) as isize;
    let next = (pos + delta).rem_euclid(POOL.len() as isize) as usize;
    (POOL[next] as char).to_string()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn key_labels_include_case_alias_for_letters() {
        assert_eq!(
            describe_key_event_labels(& KeyCode::Char('z'), KeyModifiers::empty()),
            vec!["char:z".to_owned(), "char:Z".to_owned()]
        );
        assert_eq!(
            describe_key_event_labels(& KeyCode::Char('z'), KeyModifiers::SHIFT),
            vec!["char:Z".to_owned(), "char:z".to_owned()]
        );
    }
}

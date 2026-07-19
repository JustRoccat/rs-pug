use crate::core::{CoreCmd, CoreEvent};
use crate::model::{
    default_main_tabs, App, Focus, MainTab, MainTabKind, PlayerState, RepeatMode, Tab,
};
use crate::plugins::{
    PluginCoreAction, PluginDispatch, PluginEvent, PluginLayoutConfig, PluginUiConfig,
    PluginUiLayoutPatch,
};
use crate::ui_helpers;
use tokio::sync::mpsc;
pub fn apply_event(app: &mut App, event: CoreEvent) -> Option<CoreCmd> {
    let cmd = match event {
        CoreEvent::SearchDone(songs) => {
            app.search.results = songs;
            app.search.selected_result = 0;
            app.focus = Focus::Results;
            app.player_state = if app.current_song.is_some() {
                PlayerState::Playing
            } else {
                PlayerState::Idle
            };
            app.set_flash(format!("Loaded {} result(s)", app.search.results.len()), 4);
            None
        }
        CoreEvent::AlbumSearchDone(songs) => {
            app.albums.results = songs;
            app.albums.selected_result = 0;
            app.focus = Focus::Results;
            app.player_state = if app.current_song.is_some() {
                PlayerState::Playing
            } else {
                PlayerState::Idle
            };
            app.set_flash(
                format!("Loaded {} album result(s)", app.albums.results.len()),
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
            let _ = app.storage.save_recently_played(&history);
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
                    app.queue
                        .get(pos)
                        .cloned()
                        .or_else(|| {
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
        CoreEvent::LibraryRefreshDone => {
            app.local.scanning = false;
            if let Ok((window, offset, total)) = app
                .storage
                .fetch_local_songs_window(app.local.selected_song, 200)
            {
                app.local.window = window;
                app.local.offset = offset;
                app.local.total = total;
            }
            app.set_flash("Library refreshed", 3);
            None
        }
        CoreEvent::DownloadFinished(result) => {
            match result {
                Ok(msg) => app.set_flash(msg, 5),
                Err(err) => app.set_flash(format!("Download failed: {err}"), 5),
            }
            None
        }
    };
    if app.active_tab == Tab::Local {
        ui_helpers::update_local_library_window(app);
    }
    cmd
}
pub fn map_plugin_action(action: PluginCoreAction) -> Option<CoreCmd> {
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
pub fn apply_plugin_dispatch(
    app: &mut App,
    cmd_tx: &mpsc::UnboundedSender<CoreCmd>,
    dispatch: PluginDispatch,
) -> bool {
    if let Some(tab) = dispatch.ui.set_tab {
        if let Some(core_tab) = parse_tab_name(&tab) {
            app.active_tab = core_tab;
            app.plugin_ui.active_tab = None;
            app.plugin_ui.active_custom_tab = None;
        } else if app.plugin_ui.allow_lua_ui_changes
            && app
                .main_tabs
                .iter()
                .any(|t| matches!(& t.kind, MainTabKind::Custom(id) if id == & tab))
        {
            app.plugin_ui.active_custom_tab = Some(tab);
            app.plugin_ui.active_tab = None;
        } else if app.plugin_ui.tabs.iter().any(|t| t.id == tab) {
            app.active_tab = Tab::Options;
            app.plugin_ui.active_tab = Some(tab);
        }
    }
    if let Some(query) = dispatch.ui.set_search_query {
        app.search.query = query;
    }
    if let Some(query) = dispatch.ui.set_album_search_query {
        app.albums.search_query = query;
    }
    if let Some(mode) = dispatch.ui.set_search_mode {
        app.search_mode = mode;
    }
    if let Some(focus) = dispatch.ui.set_focus {
        app.focus = parse_focus_name(&focus).unwrap_or(app.focus);
    }
    if let Some(index) = dispatch.ui.set_selected_result {
        app.search.selected_result = index
            .min(app.search.results.len().saturating_sub(1));
    }
    if let Some(index) = dispatch.ui.set_selected_album_result {
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
        app.albums.selected_result = index.min(total_items.saturating_sub(1));
    }
    if let Some(index) = dispatch.ui.set_selected_queue {
        app.selected_queue = index.min(app.queue.len().saturating_sub(1));
    }
    if app.plugin_ui.allow_lua_ui_changes {
        apply_layout_patch(app, dispatch.ui.layout);
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
pub fn plugin_event_from_core_event(event: &CoreEvent) -> PluginEvent {
    match event {
        CoreEvent::Started(song) => {
            PluginEvent {
                kind: "started".to_owned(),
                message: Some(song.title.clone()),
                value: None,
            }
        }
        CoreEvent::SearchDone(items) => {
            PluginEvent {
                kind: "search_done".to_owned(),
                message: None,
                value: Some(items.len() as f64),
            }
        }
        CoreEvent::AlbumSearchDone(items) => {
            PluginEvent {
                kind: "album_search_done".to_owned(),
                message: None,
                value: Some(items.len() as f64),
            }
        }
        CoreEvent::Progress { position, .. } => {
            PluginEvent {
                kind: "progress".to_owned(),
                message: None,
                value: Some(*position),
            }
        }
        CoreEvent::Error(msg) => {
            PluginEvent {
                kind: "error".to_owned(),
                message: Some(msg.clone()),
                value: None,
            }
        }
        _ => {
            PluginEvent {
                kind: "event".to_owned(),
                message: None,
                value: None,
            }
        }
    }
}
pub fn parse_tab_name(raw: &str) -> Option<Tab> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "discover" => Some(Tab::Discover),
        "albums" => Some(Tab::Albums),
        "library" => Some(Tab::Library),
        "options" => Some(Tab::Options),
        "local" => Some(Tab::Local),
        _ => None,
    }
}
pub fn parse_focus_name(raw: &str) -> Option<Focus> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "search" => Some(Focus::Search),
        "results" => Some(Focus::Results),
        "queue" => Some(Focus::Queue),
        _ => None,
    }
}
pub fn apply_ui_config(app: &mut App, config: PluginUiConfig) {
    if !app.plugin_ui.allow_lua_ui_changes {
        return;
    }
    let mut tabs = default_main_tabs();
    for id in &config.tabs.remove {
        if !tabs.iter().any(|tab| &tab.id == id) {
            app.push_plugin_warning(
                format!("Lua WARN [on_ui_config]: unknown tab in tabs.remove: {id}"),
            );
        }
    }
    tabs.retain(|tab| !config.tabs.remove.iter().any(|id| id == &tab.id));
    for (id, rename) in &config.tabs.rename {
        if !tabs.iter().any(|tab| &tab.id == id) {
            app.push_plugin_warning(
                format!("Lua WARN [on_ui_config]: unknown tab in tabs.rename: {id}"),
            );
            continue;
        }
        if let Some(tab) = tabs.iter_mut().find(|tab| &tab.id == id) {
            if let Some(title) = &rename.title {
                tab.title = title.to_uppercase();
            }
            if let Some(icon) = &rename.icon {
                tab.icon = icon.clone();
            }
        }
    }
    if !config.tabs.order.is_empty() {
        let mut ordered = Vec::new();
        for id in &config.tabs.order {
            if let Some(pos) = tabs.iter().position(|tab| &tab.id == id) {
                ordered.push(tabs.remove(pos));
            } else {
                app.push_plugin_warning(
                    format!("Lua WARN [on_ui_config]: unknown tab in tabs.order: {id}"),
                );
            }
        }
        ordered.extend(tabs);
        tabs = ordered;
    }
    for custom in config.tabs.custom {
        if custom.id.trim().is_empty() {
            app.push_plugin_warning(
                "Lua WARN [on_ui_config]: custom tab without id ignored".to_owned(),
            );
            continue;
        }
        if tabs.iter().any(|tab| tab.id == custom.id) {
            app.push_plugin_warning(
                format!(
                    "Lua WARN [on_ui_config]: duplicate custom tab id ignored: {}",
                    custom.id
                ),
            );
            continue;
        }
        let tab = MainTab {
            id: custom.id.clone(),
            title: custom.title.to_uppercase(),
            icon: custom.icon.unwrap_or_else(|| "◌".to_owned()),
            kind: MainTabKind::Custom(custom.id),
        };
        let requested = custom.position.unwrap_or(tabs.len() + 1);
        let pos = requested.saturating_sub(1).min(tabs.len());
        if requested == 0 || requested > tabs.len() + 1 {
            app.push_plugin_warning(
                format!(
                    "Lua WARN [on_ui_config]: custom tab position clamped: {requested}"
                ),
            );
        }
        tabs.insert(pos, tab);
    }
    app.main_tabs = tabs;
    if app.main_tabs.is_empty() {
        app.push_plugin_warning(
            "Lua WARN [on_ui_config]: all stock tabs removed; defaults restored"
                .to_owned(),
        );
        app.main_tabs = default_main_tabs();
    }
    if !app
        .main_tabs
        .iter()
        .any(|tab| match &tab.kind {
            MainTabKind::Stock(stock) => {
                app.plugin_ui.active_custom_tab.is_none() && *stock == app.active_tab
            }
            MainTabKind::Custom(id) => {
                app.plugin_ui.active_custom_tab.as_ref() == Some(id)
            }
        })
    {
        activate_main_tab(app, 0);
    }
    apply_layout_config(app, config.layout);
}
pub fn apply_layout_config(app: &mut App, layout: PluginLayoutConfig) {
    if !app.plugin_ui.allow_lua_ui_changes {
        return;
    }
    apply_layout_dimensions(
        app,
        layout.queue_width_percent,
        layout.visualizer_height,
        layout.tab_bar_position.as_deref(),
        layout.tabs_width,
        layout.queue_position.as_deref(),
        "layout",
    );
    if let Some(value) = layout.show_progress_bar {
        app.ui_layout.show_progress_bar = value;
    }
    if let Some(value) = layout.show_volume_bar {
        app.ui_layout.show_volume_bar = value;
    }
    if let Some(value) = layout.show_statusbar {
        app.ui_layout.show_statusbar = value;
    }
    if let Some(value) = layout.show_keybind_hints {
        app.ui_layout.show_keybind_hints = value;
    }
    apply_layout_hide(app, layout.hide);
    apply_custom_sections(app, layout.custom_sections);
    update_section_visibility(app, layout.hide_sections, layout.show_sections);
}
pub fn apply_layout_patch(app: &mut App, patch: PluginUiLayoutPatch) {
    if !app.plugin_ui.allow_lua_ui_changes {
        return;
    }
    apply_layout_dimensions(
        app,
        patch.queue_width_percent,
        patch.visualizer_height,
        patch.tab_bar_position.as_deref(),
        patch.tabs_width,
        patch.queue_position.as_deref(),
        "ui.layout",
    );
    update_section_visibility(app, patch.hide_sections, patch.show_sections);
}
fn apply_layout_dimensions(
    app: &mut App,
    queue_width_percent: Option<u16>,
    visualizer_height: Option<u16>,
    tab_bar_position: Option<&str>,
    tabs_width: Option<u16>,
    queue_position: Option<&str>,
    source: &str,
) {
    if let Some(value) = queue_width_percent {
        let clamped = value.clamp(10, 90);
        if clamped != value {
            app.push_plugin_warning(
                format!(
                    "Lua WARN [{source}]: queue_width_percent clamped from {value} to {clamped}"
                ),
            );
        }
        app.ui_layout.queue_width_percent = clamped;
    }
    if let Some(value) = visualizer_height {
        let clamped = value.clamp(0, 10);
        if clamped != value {
            app.push_plugin_warning(
                format!(
                    "Lua WARN [{source}]: visualizer_height clamped from {value} to {clamped}"
                ),
            );
        }
        app.ui_layout.visualizer_height = clamped;
    }
    if let Some(value) = tab_bar_position {
        apply_tab_bar_position(app, value, &format!("{source}.tab_bar_position"));
    }
    if let Some(value) = tabs_width {
        apply_tabs_width(app, value, &format!("{source}.tabs_width"));
    }
    if let Some(value) = queue_position {
        apply_queue_position(app, value, &format!("{source}.queue_position"));
    }
}
fn apply_layout_hide(app: &mut App, hidden_items: Vec<String>) {
    for item in hidden_items {
        match item.as_str() {
            "visualizer" => app.ui_layout.visualizer_height = 0,
            "progress_bar" => app.ui_layout.show_progress_bar = false,
            "volume_bar" => app.ui_layout.show_volume_bar = false,
            "statusbar" => app.ui_layout.show_statusbar = false,
            "keybind_hints" => app.ui_layout.show_keybind_hints = false,
            _ => {
                app.push_plugin_warning(
                    format!("Lua WARN [layout.hide]: unknown UI element: {item}"),
                )
            }
        }
    }
}
fn apply_custom_sections(
    app: &mut App,
    sections: Vec<crate::plugins::PluginCustomSection>,
) {
    for section in sections {
        if section.id.trim().is_empty() {
            app.push_plugin_warning(
                "Lua WARN [layout.plugin_ui.custom_sections]: section without id ignored"
                    .to_owned(),
            );
            continue;
        }
        if !matches!(
            section.position.as_str(), "above_player" | "below_player" | "left" | "right"
        ) {
            app.push_plugin_warning(
                format!(
                    "Lua WARN [layout.plugin_ui.custom_sections]: invalid position for {}: {}",
                    section.id, section.position
                ),
            );
            continue;
        }
        if !app.plugin_ui.custom_sections.iter().any(|s| s.id == section.id) {
            app.plugin_ui.custom_sections.push(section);
        } else {
            app.push_plugin_warning(
                format!(
                    "Lua WARN [layout.plugin_ui.custom_sections]: duplicate section id ignored: {}",
                    section.id
                ),
            );
        }
    }
}
fn apply_tab_bar_position(app: &mut App, raw: &str, source: &str) {
    match raw.trim().to_ascii_lowercase().as_str() {
        "top" | "bottom" | "left" | "right" => {
            app.ui_layout.tab_bar_position = raw.trim().to_ascii_lowercase();
        }
        other => {
            app.push_plugin_warning(
                format!("Lua WARN [{source}]: unknown tab_bar_position: {other}"),
            )
        }
    }
}
fn apply_queue_position(app: &mut App, raw: &str, source: &str) {
    match raw.trim().to_ascii_lowercase().as_str() {
        "left" | "right" => {
            app.ui_layout.queue_position = raw.trim().to_ascii_lowercase();
        }
        other => {
            app.push_plugin_warning(
                format!("Lua WARN [{source}]: unknown queue_position: {other}"),
            )
        }
    }
}
fn apply_tabs_width(app: &mut App, value: u16, source: &str) {
    let clamped = value.clamp(12, 40);
    if clamped != value {
        app.push_plugin_warning(
            format!("Lua WARN [{source}]: tabs_width clamped from {value} to {clamped}"),
        );
    }
    app.ui_layout.tabs_width = clamped;
}
pub fn activate_main_tab(app: &mut App, index: usize) {
    if let Some(tab) = app.main_tabs.get(index) {
        match &tab.kind {
            MainTabKind::Stock(stock) => {
                app.active_tab = *stock;
                app.plugin_ui.active_custom_tab = None;
            }
            MainTabKind::Custom(id) => {
                app.plugin_ui.active_custom_tab = Some(id.clone());
            }
        }
        app.plugin_ui.active_tab = None;
    }
}
fn update_section_visibility(app: &mut App, hide: Vec<String>, show: Vec<String>) {
    for id in hide {
        if !app.plugin_ui.hidden_sections.iter().any(|hidden| hidden == &id) {
            app.plugin_ui.hidden_sections.push(id);
        }
    }
    for id in show {
        app.plugin_ui.hidden_sections.retain(|hidden| hidden != &id);
    }
}

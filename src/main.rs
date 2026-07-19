use std::{
    collections::VecDeque, fs, path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, SystemTime},
};
use anyhow::Result;
use clap::Parser;
use crossterm::event::{self, Event, KeyEvent};
use tokio::sync::mpsc;
mod cli;
mod config;
mod core;
mod db;
mod eq;
mod fft;
mod events;
mod input;
mod model;
mod playlist;
mod plugins;
mod storage;
mod terminal;
mod tui;
mod ui_helpers;
mod utils;
use config::{load_config, SearchSource};
use core::{Core, CoreCmd, CoreEvent};
use input::KeyPluginAction;
use model::App;
use plugins::{
    PluginDispatch, PluginManager, PluginPanel, PluginTab, PluginUiConfig,
    PluginUiInject, PluginUiSections, PluginUiState,
};
use storage::Storage;
enum PluginTaskResult {
    UiConfig(PluginUiConfig),
    UiUpdate { state: PluginUiState, layout: plugins::PluginLayoutConfig },
    UiSurface {
        state: PluginUiState,
        tabs: Vec<PluginTab>,
        panels: Vec<PluginPanel>,
        sections: PluginUiSections,
        inject: PluginUiInject,
    },
    EventDispatch(PluginDispatch),
    KeyDispatch { key: KeyEvent, dispatch: PluginDispatch },
}
struct PendingPluginKey {
    key: KeyEvent,
    labels: Vec<String>,
    state: PluginUiState,
}
#[tokio::main]
async fn main() -> Result<()> {
    let args = cli::Args::parse();
    let ipc_sock_path = "/tmp/rs-pug-ipc.sock";
    let is_client_cmd = args.toggle_pause || args.next || args.prev
        || args.play.is_some();
    if is_client_cmd {
        if let Ok(mut stream) = tokio::net::UnixStream::connect(ipc_sock_path).await {
            use tokio::io::AsyncWriteExt;
            if args.toggle_pause {
                let _ = stream.write_all(b"TOGGLE_PAUSE\n").await;
            }
            if args.next {
                let _ = stream.write_all(b"NEXT\n").await;
            }
            if args.prev {
                let _ = stream.write_all(b"PREV\n").await;
            }
            if let Some(play) = args.play {
                let _ = stream.write_all(format!("PLAY {play}\n").as_bytes()).await;
            }
        } else {
            eprintln!(
                "Failed to connect to rs-pug instance at {ipc_sock_path}. Is it running?"
            );
            std::process::exit(1);
        }
        return Ok(());
    }
    terminal::install_panic_hook();
    config::ensure_default_dirs();
    let mut config = load_config();
    if let Some(source_arg) = args.source {
        config.search.source = SearchSource::from(source_arg);
    }
    let plugin_manager = Arc::new(
        Mutex::new(
            PluginManager::load(
                config.general.plugins_enabled,
                config.general.plugins_dir.as_str(),
                config.lua.allow_lua_ui_changes,
            ),
        ),
    );
    let mut terminal = terminal::setup_terminal()?;
    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
    let (evt_tx, mut evt_rx) = mpsc::unbounded_channel();
    let (plugin_tx, mut plugin_rx) = mpsc::unbounded_channel();
    let core = Core::new(config.clone(), Arc::clone(&plugin_manager)).await?;
    tokio::spawn(core.run(cmd_rx, evt_tx.clone(), cmd_tx.clone()));
    let ipc_cmd_tx = cmd_tx.clone();
    tokio::spawn(async move {
        let _ = std::fs::remove_file(ipc_sock_path);
        if let Ok(listener) = tokio::net::UnixListener::bind(ipc_sock_path) {
            use tokio::io::{AsyncBufReadExt, BufReader};
            loop {
                if let Ok((mut stream, _)) = listener.accept().await {
                    let tx = ipc_cmd_tx.clone();
                    tokio::spawn(async move {
                        let (read, _) = stream.split();
                        let mut reader = BufReader::new(read);
                        let mut line = String::new();
                        while let Ok(n) = reader.read_line(&mut line).await {
                            if n == 0 {
                                break;
                            }
                            let cmd = line.trim();
                            if cmd == "TOGGLE_PAUSE" {
                                let _ = tx.send(CoreCmd::TogglePause);
                            } else if cmd == "NEXT" {
                                let _ = tx.send(CoreCmd::Next);
                            } else if cmd == "PREV" {
                                let _ = tx.send(CoreCmd::Prev);
                            } else if cmd.starts_with("PLAY ") {
                                let path = cmd.strip_prefix("PLAY ").unwrap().to_string();
                                let song = crate::model::Song {
                                    id: path.clone(),
                                    title: path.clone(),
                                    webpage_url: path,
                                    uploader: Some("IPC".to_string()),
                                    duration: None,
                                };
                                let _ = tx.send(CoreCmd::Play(song));
                            }
                            line.clear();
                        }
                    });
                }
            }
        }
    });
    let storage = match Storage::init() {
        Ok(storage) => storage,
        Err(err) => {
            terminal::restore_terminal(terminal)?;
            anyhow::bail!("Failed to init storage: {err}");
        }
    };
    let mut app = App::new(storage);
    app.apply_config(&config);
    if config.general.fft_visualizer_default {
        app.show_fft = true;
        app.fft_state = Some(fft::start_fft_monitor());
    }
    if app.plugin_ui.allow_lua_ui_changes {
        app.set_flash("Lua UI changes enabled", 4);
    }
    if let Ok(plugins) = plugin_manager.try_lock() {
        for warning in plugins.drain_warnings() {
            app.push_plugin_warning(warning.label());
        }
    }
    match app.storage.load_playlists() {
        Ok(playlists) => {
            app.playlists.playlists = playlists;
            app.playlists.expanded = vec![false; app.playlists.playlists.len()];
        }
        Err(e) => app.set_flash(format!("Error loading playlists: {e}"), 5),
    }
    match app.storage.load_recently_played() {
        Ok(recent) => app.recently_played = recent.into(),
        Err(e) => app.set_flash(format!("Error loading recent: {e}"), 5),
    }
    match app.storage.fetch_local_songs_window(0, 200) {
        Ok((window, offset, total)) => {
            app.local.window = window;
            app.local.offset = offset;
            app.local.total = total;
        }
        Err(e) => app.set_flash(format!("Error loading library window: {e}"), 5),
    }
    app.eq.custom_presets = config::load_eq_presets();
    app.local.scanning = true;
    let storage_clone = app.storage.clone();
    let config_clone = config.clone();
    let evt_tx_clone = evt_tx.clone();
    tokio::spawn(async move {
        let _ = tokio::task::spawn_blocking(move || {
                core::check_and_refresh_library(&config_clone, &storage_clone)
            })
            .await;
        let _ = evt_tx_clone.send(CoreEvent::LibraryRefreshDone);
    });
    let tick_rate = Duration::from_millis(33);
    let (hr_result_tx, mut hr_result_rx) = mpsc::unbounded_channel::<HotReloadResult>();
    let (hr_paths_tx, hr_paths_rx) = mpsc::unbounded_channel::<HotReloadPaths>();
    spawn_hot_reload_task(config.clone(), hr_result_tx, hr_paths_rx);
    let mut startup_ui_config_scheduled = false;
    let mut startup_ui_config_done = !app.plugin_ui.allow_lua_ui_changes;
    let mut ui_update_pending = false;
    let mut ui_surface_pending = false;
    let mut key_hook_pending = false;
    let mut queued_plugin_keys = VecDeque::new();
    let mut last_ui_state: Option<PluginUiState> = None;
    let mut last_ui_surface_state: Option<PluginUiState> = None;
    loop {
        while let Ok(hr) = hr_result_rx.try_recv() {
            let mut should_reload_plugins = hr.plugins_changed;
            if hr.config_changed {
                let old = config.clone();
                let prev_opt_theme = app.opt_theme.clone();
                config = load_config();
                app.apply_config(&config);
                app.opt_theme = prev_opt_theme;
                let _ = hr_paths_tx
                    .send(HotReloadPaths {
                        plugins_dir: config.general.plugins_dir.clone(),
                        music_dirs: config.general.music_directories.clone(),
                    });
                if config.general.plugins_enabled != old.general.plugins_enabled
                    || config.general.plugins_dir != old.general.plugins_dir
                    || config.lua.allow_lua_ui_changes != old.lua.allow_lua_ui_changes
                {
                    should_reload_plugins = true;
                }
            }
            if hr.eq_changed {
                app.eq.custom_presets = config::load_eq_presets();
            }
            if should_reload_plugins {
                if let Ok(mut plugins) = plugin_manager.lock() {
                    plugins
                        .reload(
                            config.general.plugins_enabled,
                            &config.general.plugins_dir,
                            config.lua.allow_lua_ui_changes,
                        );
                }
                startup_ui_config_scheduled = false;
                startup_ui_config_done = !app.plugin_ui.allow_lua_ui_changes;
                ui_update_pending = false;
                ui_surface_pending = false;
                last_ui_state = None;
                last_ui_surface_state = None;
                app.plugin_ui.tabs.clear();
                app.plugin_ui.panels.clear();
                app.plugin_ui.section_items.clear();
                app.plugin_ui.inject = PluginUiInject::default();
            }
            if hr.music_changed && !app.local.scanning {
                app.local.scanning = true;
                let storage_clone = app.storage.clone();
                let config_clone = config.clone();
                let evt_tx_clone = evt_tx.clone();
                tokio::spawn(async move {
                    let _ = tokio::task::spawn_blocking(move || {
                            core::check_and_refresh_library(
                                &config_clone,
                                &storage_clone,
                            )
                        })
                        .await;
                    let _ = evt_tx_clone.send(CoreEvent::LibraryRefreshDone);
                });
            }
        }
        if let Ok(plugins) = plugin_manager.try_lock() {
            for warning in plugins.drain_warnings() {
                app.push_plugin_warning(warning.label());
            }
        }
        let mut keep_running = true;
        while let Ok(result) = plugin_rx.try_recv() {
            match result {
                PluginTaskResult::UiConfig(config) => {
                    events::apply_ui_config(&mut app, config);
                    startup_ui_config_done = true;
                    last_ui_state = None;
                    last_ui_surface_state = None;
                }
                PluginTaskResult::UiUpdate { state, layout } => {
                    if state == PluginUiState::from_app(&app) {
                        events::apply_layout_config(&mut app, layout);
                        last_ui_state = Some(state);
                    }
                    ui_update_pending = false;
                }
                PluginTaskResult::UiSurface {
                    state,
                    tabs,
                    panels,
                    sections,
                    inject,
                } => {
                    if state == PluginUiState::from_app(&app) {
                        app.plugin_ui.tabs = tabs;
                        if let Some(active) = app.plugin_ui.active_tab.clone() {
                            if !app.plugin_ui.tabs.iter().any(|t| t.id == active) {
                                app.plugin_ui.active_tab = None;
                            }
                        }
                        app.plugin_ui.panels = panels;
                        if app.plugin_ui.allow_lua_ui_changes {
                            app.plugin_ui.section_items = sections;
                            app.plugin_ui.inject = inject;
                        }
                        last_ui_surface_state = Some(state);
                    }
                    ui_surface_pending = false;
                }
                PluginTaskResult::EventDispatch(dispatch) => {
                    let _ = events::apply_plugin_dispatch(&mut app, &cmd_tx, dispatch);
                    last_ui_state = None;
                    last_ui_surface_state = None;
                }
                PluginTaskResult::KeyDispatch { key, dispatch } => {
                    key_hook_pending = false;
                    if !events::apply_plugin_dispatch(&mut app, &cmd_tx, dispatch) {
                        keep_running = input::handle_native_key_event(
                            &mut app,
                            key,
                            &cmd_tx,
                        );
                    }
                    last_ui_state = None;
                    last_ui_surface_state = None;
                    if keep_running {
                        start_next_plugin_key(
                            &plugin_manager,
                            &plugin_tx,
                            &mut queued_plugin_keys,
                            &mut key_hook_pending,
                        );
                    }
                }
            }
            if !keep_running {
                break;
            }
        }
        if !keep_running {
            break;
        }
        let ui_state = PluginUiState::from_app(&app);
        if app.plugin_ui.allow_lua_ui_changes && !startup_ui_config_scheduled {
            spawn_ui_config_task(&plugin_manager, &plugin_tx, ui_state.clone());
            startup_ui_config_scheduled = true;
        }
        let ui_state = PluginUiState::from_app(&app);
        if startup_ui_config_done && app.plugin_ui.allow_lua_ui_changes
            && !ui_update_pending && last_ui_state.as_ref() != Some(&ui_state)
        {
            spawn_ui_update_task(&plugin_manager, &plugin_tx, ui_state.clone());
            ui_update_pending = true;
        }
        if startup_ui_config_done && !ui_surface_pending
            && last_ui_surface_state.as_ref() != Some(&ui_state)
        {
            spawn_ui_surface_task(
                &plugin_manager,
                &plugin_tx,
                ui_state.clone(),
                app.plugin_ui.allow_lua_ui_changes,
            );
            ui_surface_pending = true;
        }
        app.anim_tick = app.anim_tick.wrapping_add(1);
        terminal.draw(|frame| tui::draw(frame, &app))?;
        while let Ok(event) = evt_rx.try_recv() {
            let plugin_event = events::plugin_event_from_core_event(&event);
            if let Some(cmd) = events::apply_event(&mut app, event) {
                let _ = cmd_tx.send(cmd);
            }
            let ui_state = PluginUiState::from_app(&app);
            spawn_event_dispatch_task(
                &plugin_manager,
                &plugin_tx,
                plugin_event,
                ui_state,
            );
        }
        if event::poll(tick_rate)? {
            match event::read()? {
                Event::Mouse(mouse) => {
                    input::handle_mouse_event(&mut app, mouse);
                }
                Event::Key(key) => {
                    match input::handle_key_event_pre_plugin(&mut app, key, &cmd_tx) {
                        KeyPluginAction::Handled(next) => {
                            if !next {
                                break;
                            }
                        }
                        KeyPluginAction::Dispatch { labels } => {
                            let request = PendingPluginKey {
                                key,
                                labels,
                                state: PluginUiState::from_app(&app),
                            };
                            queued_plugin_keys.push_back(request);
                            start_next_plugin_key(
                                &plugin_manager,
                                &plugin_tx,
                                &mut queued_plugin_keys,
                                &mut key_hook_pending,
                            );
                        }
                    }
                }
                _ => {}
            }
        }
    }
    let _ = cmd_tx.send(CoreCmd::Quit);
    terminal::restore_terminal(terminal)?;
    Ok(())
}
fn spawn_ui_config_task(
    plugin_manager: &Arc<Mutex<PluginManager>>,
    plugin_tx: &mpsc::UnboundedSender<PluginTaskResult>,
    state: PluginUiState,
) {
    let plugins = Arc::clone(plugin_manager);
    let tx = plugin_tx.clone();
    tokio::task::spawn_blocking(move || {
        let config = plugins
            .lock()
            .map(|plugins| plugins.collect_ui_config(&state))
            .unwrap_or_default();
        let _ = tx.send(PluginTaskResult::UiConfig(config));
    });
}
fn spawn_ui_update_task(
    plugin_manager: &Arc<Mutex<PluginManager>>,
    plugin_tx: &mpsc::UnboundedSender<PluginTaskResult>,
    state: PluginUiState,
) {
    let plugins = Arc::clone(plugin_manager);
    let tx = plugin_tx.clone();
    tokio::task::spawn_blocking(move || {
        let layout = plugins
            .lock()
            .map(|plugins| plugins.collect_ui_update(&state))
            .unwrap_or_default();
        let _ = tx
            .send(PluginTaskResult::UiUpdate {
                state,
                layout,
            });
    });
}
fn spawn_ui_surface_task(
    plugin_manager: &Arc<Mutex<PluginManager>>,
    plugin_tx: &mpsc::UnboundedSender<PluginTaskResult>,
    state: PluginUiState,
    allow_lua_ui_changes: bool,
) {
    let plugins = Arc::clone(plugin_manager);
    let tx = plugin_tx.clone();
    tokio::task::spawn_blocking(move || {
        let (tabs, panels, sections, inject) = plugins
            .lock()
            .map(|plugins| {
                let sections = if allow_lua_ui_changes {
                    plugins.collect_ui_sections(&state)
                } else {
                    PluginUiSections::default()
                };
                let inject = if allow_lua_ui_changes {
                    plugins.collect_ui_inject(&state)
                } else {
                    PluginUiInject::default()
                };
                (
                    plugins.collect_tabs(&state),
                    plugins.collect_ui_panels(&state),
                    sections,
                    inject,
                )
            })
            .unwrap_or_default();
        let _ = tx
            .send(PluginTaskResult::UiSurface {
                state,
                tabs,
                panels,
                sections,
                inject,
            });
    });
}
fn spawn_event_dispatch_task(
    plugin_manager: &Arc<Mutex<PluginManager>>,
    plugin_tx: &mpsc::UnboundedSender<PluginTaskResult>,
    event: plugins::PluginEvent,
    state: PluginUiState,
) {
    let plugins = Arc::clone(plugin_manager);
    let tx = plugin_tx.clone();
    tokio::task::spawn_blocking(move || {
        let dispatch = plugins
            .lock()
            .map(|plugins| plugins.dispatch_event(&event, &state))
            .unwrap_or_default();
        let _ = tx.send(PluginTaskResult::EventDispatch(dispatch));
    });
}
fn start_next_plugin_key(
    plugin_manager: &Arc<Mutex<PluginManager>>,
    plugin_tx: &mpsc::UnboundedSender<PluginTaskResult>,
    queued_plugin_keys: &mut VecDeque<PendingPluginKey>,
    key_hook_pending: &mut bool,
) {
    if *key_hook_pending {
        return;
    }
    let Some(request) = queued_plugin_keys.pop_front() else {
        return;
    };
    *key_hook_pending = true;
    let plugins = Arc::clone(plugin_manager);
    let tx = plugin_tx.clone();
    tokio::task::spawn_blocking(move || {
        let dispatch = plugins
            .lock()
            .map(|plugins| dispatch_key_with_aliases(
                &plugins,
                &request.labels,
                &request.state,
            ))
            .unwrap_or_default();
        let _ = tx
            .send(PluginTaskResult::KeyDispatch {
                key: request.key,
                dispatch,
            });
    });
}
fn dispatch_key_with_aliases(
    plugins: &PluginManager,
    labels: &[String],
    state: &PluginUiState,
) -> PluginDispatch {
    for label in labels {
        let dispatch = plugins.dispatch_key(label.as_str(), state);
        if plugin_dispatch_has_effect(&dispatch) {
            return dispatch;
        }
    }
    PluginDispatch::default()
}
fn plugin_dispatch_has_effect(dispatch: &PluginDispatch) -> bool {
    dispatch.consume || dispatch.flash.is_some() || dispatch.flash_seconds.is_some()
        || !dispatch.core_actions.is_empty() || dispatch.ui.set_tab.is_some()
        || dispatch.ui.set_search_query.is_some()
        || dispatch.ui.set_album_search_query.is_some()
        || dispatch.ui.set_focus.is_some() || dispatch.ui.set_search_mode.is_some()
        || dispatch.ui.set_selected_result.is_some()
        || dispatch.ui.set_selected_album_result.is_some()
        || dispatch.ui.set_selected_queue.is_some()
        || layout_patch_has_effect(&dispatch.ui.layout)
}
fn layout_patch_has_effect(layout: &plugins::PluginUiLayoutPatch) -> bool {
    layout.queue_width_percent.is_some() || layout.visualizer_height.is_some()
        || layout.tab_bar_position.is_some() || layout.tabs_width.is_some()
        || layout.queue_position.is_some() || !layout.hide_sections.is_empty()
        || !layout.show_sections.is_empty()
}
struct HotReloadResult {
    config_changed: bool,
    eq_changed: bool,
    #[allow(dead_code)]
    theme_changed: bool,
    plugins_changed: bool,
    music_changed: bool,
}
struct HotReloadPaths {
    plugins_dir: String,
    music_dirs: Vec<String>,
}
struct HotReloadState {
    config_snapshot: PathsSnapshot,
    theme_snapshot: DirSnapshot,
    eq_snapshot: DirSnapshot,
    plugin_snapshot: DirSnapshot,
    music_dirs_snapshot: MusicDirsSnapshot,
}
impl HotReloadState {
    fn new(config: &config::Config) -> Self {
        Self {
            config_snapshot: PathsSnapshot::capture(config::config_paths()),
            theme_snapshot: DirSnapshot::capture(config_resource_dir("themes"), "json"),
            eq_snapshot: DirSnapshot::capture(config_resource_dir("eqpresets"), "json"),
            plugin_snapshot: DirSnapshot::capture(&config.general.plugins_dir, "lua"),
            music_dirs_snapshot: MusicDirsSnapshot::capture(
                &config.general.music_directories,
            ),
        }
    }
    fn refresh_all(&mut self) -> HotReloadResult {
        HotReloadResult {
            config_changed: self.config_snapshot.refresh(),
            theme_changed: self.theme_snapshot.refresh(),
            eq_changed: self.eq_snapshot.refresh(),
            plugins_changed: self.plugin_snapshot.refresh(),
            music_changed: self.music_dirs_snapshot.refresh(),
        }
    }
    fn update_paths(&mut self, paths: HotReloadPaths) {
        self.plugin_snapshot = DirSnapshot::capture(&paths.plugins_dir, "lua");
        self.music_dirs_snapshot = MusicDirsSnapshot::capture(&paths.music_dirs);
    }
}
fn spawn_hot_reload_task(
    config: config::Config,
    result_tx: mpsc::UnboundedSender<HotReloadResult>,
    mut paths_rx: mpsc::UnboundedReceiver<HotReloadPaths>,
) {
    tokio::spawn(async move {
        let mut state = match tokio::task::spawn_blocking(move || HotReloadState::new(
                &config,
            ))
            .await
        {
            Ok(state) => state,
            Err(_) => return,
        };
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        interval.tick().await;
        loop {
            interval.tick().await;
            let mut latest_paths: Option<HotReloadPaths> = None;
            while let Ok(p) = paths_rx.try_recv() {
                latest_paths = Some(p);
            }
            let result = tokio::task::spawn_blocking(move || {
                    if let Some(paths) = latest_paths {
                        state.update_paths(paths);
                    }
                    let result = state.refresh_all();
                    (state, result)
                })
                .await;
            let result = match result {
                Ok(result) => result,
                Err(_) => break,
            };
            state = result.0;
            if result_tx.send(result.1).is_err() {
                break;
            }
        }
    });
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct FileFingerprint {
    path: PathBuf,
    modified: Option<SystemTime>,
    len: Option<u64>,
}
impl FileFingerprint {
    fn capture(path: PathBuf) -> Self {
        let metadata = fs::metadata(&path).ok();
        Self {
            path,
            modified: metadata.as_ref().and_then(|m| m.modified().ok()),
            len: metadata.as_ref().map(|m| m.len()),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct PathsSnapshot {
    files: Vec<FileFingerprint>,
}
impl PathsSnapshot {
    fn capture(mut paths: Vec<PathBuf>) -> Self {
        paths.sort();
        paths.dedup();
        Self {
            files: paths.into_iter().map(FileFingerprint::capture).collect(),
        }
    }
    fn refresh(&mut self) -> bool {
        let next = Self::capture(
            self.files.iter().map(|file| file.path.clone()).collect(),
        );
        if *self != next {
            *self = next;
            true
        } else {
            false
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct DirSnapshot {
    dir: PathBuf,
    extension: String,
    files: Vec<FileFingerprint>,
}
impl DirSnapshot {
    fn capture(dir: impl AsRef<Path>, extension: &str) -> Self {
        let dir = PathBuf::from(dir.as_ref());
        let mut paths = Vec::new();
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|ext| ext.to_str()) == Some(extension) {
                    paths.push(path);
                }
            }
        }
        paths.sort();
        Self {
            dir,
            extension: extension.to_owned(),
            files: paths.into_iter().map(FileFingerprint::capture).collect(),
        }
    }
    fn refresh(&mut self) -> bool {
        let next = Self::capture(&self.dir.clone(), &self.extension.clone());
        if *self != next {
            *self = next;
            true
        } else {
            false
        }
    }
}
fn config_resource_dir(kind: &str) -> PathBuf {
    if let Some(home) = std::env::var_os("HOME") {
        PathBuf::from(home).join(".config/rs-pug").join(kind)
    } else {
        PathBuf::from(".config/rs-pug").join(kind)
    }
}
const AUDIO_EXTENSIONS: &[&str] = &["mp3", "flac", "wav", "ogg", "m4a"];
#[derive(Debug, Clone, PartialEq, Eq)]
struct MusicDirsSnapshot {
    dirs: Vec<String>,
    files: Vec<FileFingerprint>,
}
impl MusicDirsSnapshot {
    fn capture(dirs: &[String]) -> Self {
        let mut paths = Vec::new();
        for dir in dirs {
            let path_str = if dir.starts_with('~') {
                if let Ok(home) = std::env::var("HOME") {
                    dir.replacen('~', &home, 1)
                } else {
                    dir.clone()
                }
            } else {
                dir.clone()
            };
            let path = Path::new(&path_str);
            if !path.exists() {
                continue;
            }
            for entry in walkdir::WalkDir::new(path)
                .follow_links(true)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if entry.file_type().is_file() {
                    let p = entry.path();
                    if let Some(ext) = p.extension().and_then(|s| s.to_str()) {
                        if AUDIO_EXTENSIONS.contains(&ext.to_lowercase().as_str()) {
                            paths.push(p.to_path_buf());
                        }
                    }
                }
            }
        }
        paths.sort();
        Self {
            dirs: dirs.to_vec(),
            files: paths.into_iter().map(FileFingerprint::capture).collect(),
        }
    }
    fn refresh(&mut self) -> bool {
        let next = Self::capture(&self.dirs.clone());
        if *self != next {
            *self = next;
            true
        } else {
            false
        }
    }
}

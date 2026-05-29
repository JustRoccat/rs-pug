use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Duration,
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
    PluginDispatch, PluginManager, PluginPanel, PluginTab, PluginUiConfig, PluginUiInject,
    PluginUiSections, PluginUiState,
};
use storage::Storage;

enum PluginTaskResult {
    UiConfig(PluginUiConfig),
    UiUpdate {
        state: PluginUiState,
        layout: plugins::PluginLayoutConfig,
    },
    UiSurface {
        state: PluginUiState,
        tabs: Vec<PluginTab>,
        panels: Vec<PluginPanel>,
        sections: PluginUiSections,
        inject: PluginUiInject,
    },
    EventDispatch(PluginDispatch),
    KeyDispatch {
        key: KeyEvent,
        dispatch: PluginDispatch,
    },
}

struct PendingPluginKey {
    key: KeyEvent,
    label: String,
    state: PluginUiState,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = cli::Args::parse();
    config::ensure_default_dirs();
    let mut config = load_config();

    if let Some(source_arg) = args.source {
        config.search.source = SearchSource::from(source_arg);
    }

    let plugin_manager = Arc::new(Mutex::new(PluginManager::load(
        config.general.plugins_enabled,
        config.general.plugins_dir.as_str(),
        config.lua.allow_lua_ui_changes,
    )));

    let mut terminal = terminal::setup_terminal()?;
    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
    let (evt_tx, mut evt_rx) = mpsc::unbounded_channel();
    let (plugin_tx, mut plugin_rx) = mpsc::unbounded_channel();

    let core = Core::new(config.clone(), Arc::clone(&plugin_manager)).await?;
    tokio::spawn(core.run(cmd_rx, evt_tx.clone(), cmd_tx.clone()));

    let storage = Storage::init().expect("Failed to init storage");
    let mut app = App::new(storage);
    app.apply_config(&config);
    if app.allow_lua_ui_changes {
        app.set_flash("Lua UI changes enabled", 4);
    }
    if let Ok(plugins) = plugin_manager.try_lock() {
        for warning in plugins.drain_warnings() {
            app.push_plugin_warning(warning.label());
        }
    }

    match app.storage.load_playlists() {
        Ok(playlists) => {
            app.playlists = playlists;
            app.playlist_expanded = vec![false; app.playlists.len()];
        }
        Err(e) => app.set_flash(format!("Error loading playlists: {e}"), 5),
    }

    match app.storage.load_recently_played() {
        Ok(recent) => app.recently_played = recent.into(),
        Err(e) => app.set_flash(format!("Error loading recent: {e}"), 5),
    }

    match app.storage.fetch_local_songs_window(0, 200) {
        Ok((window, offset, total)) => {
            app.local_library_window = window;
            app.local_library_offset = offset;
            app.local_library_total = total;
        }
        Err(e) => app.set_flash(format!("Error loading library window: {e}"), 5),
    }
    app.custom_eq_presets = config::load_eq_presets();

    app.scanning = true;
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

    let tick_rate = Duration::from_millis(20);
    let mut startup_ui_config_scheduled = false;
    let mut ui_update_pending = false;
    let mut ui_surface_pending = false;
    let mut key_hook_pending = false;
    let mut queued_plugin_keys = VecDeque::new();
    let mut last_ui_state: Option<PluginUiState> = None;
    let mut last_ui_surface_state: Option<PluginUiState> = None;

    loop {
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
                }
                PluginTaskResult::UiUpdate { state, layout } => {
                    events::apply_layout_config(&mut app, layout);
                    last_ui_state = Some(state);
                    ui_update_pending = false;
                }
                PluginTaskResult::UiSurface {
                    state,
                    tabs,
                    panels,
                    sections,
                    inject,
                } => {
                    app.plugin_tabs = tabs;
                    if let Some(active) = app.active_plugin_tab.clone() {
                        if !app.plugin_tabs.iter().any(|t| t.id == active) {
                            app.active_plugin_tab = None;
                        }
                    }
                    app.plugin_panels = panels;
                    if app.allow_lua_ui_changes {
                        app.ui_section_items = sections;
                        app.ui_inject = inject;
                    }
                    last_ui_surface_state = Some(state);
                    ui_surface_pending = false;
                }
                PluginTaskResult::EventDispatch(dispatch) => {
                    let _ = events::apply_plugin_dispatch(&mut app, &cmd_tx, dispatch);
                }
                PluginTaskResult::KeyDispatch { key, dispatch } => {
                    key_hook_pending = false;
                    if !events::apply_plugin_dispatch(&mut app, &cmd_tx, dispatch) {
                        keep_running = input::handle_native_key_event(&mut app, key, &cmd_tx);
                    }
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
        if app.allow_lua_ui_changes && !startup_ui_config_scheduled {
            spawn_ui_config_task(&plugin_manager, &plugin_tx, ui_state.clone());
            startup_ui_config_scheduled = true;
        }
        let ui_state = PluginUiState::from_app(&app);
        if app.allow_lua_ui_changes
            && !ui_update_pending
            && last_ui_state.as_ref() != Some(&ui_state)
        {
            spawn_ui_update_task(&plugin_manager, &plugin_tx, ui_state.clone());
            ui_update_pending = true;
        }
        if !ui_surface_pending && last_ui_surface_state.as_ref() != Some(&ui_state) {
            spawn_ui_surface_task(
                &plugin_manager,
                &plugin_tx,
                ui_state.clone(),
                app.allow_lua_ui_changes,
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
            spawn_event_dispatch_task(&plugin_manager, &plugin_tx, plugin_event, ui_state);
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
                        KeyPluginAction::Dispatch { label } => {
                            let request = PendingPluginKey {
                                key,
                                label,
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
        let _ = tx.send(PluginTaskResult::UiUpdate { state, layout });
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
        let _ = tx.send(PluginTaskResult::UiSurface {
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
            .map(|plugins| plugins.dispatch_key(request.label.as_str(), &request.state))
            .unwrap_or_default();
        let _ = tx.send(PluginTaskResult::KeyDispatch {
            key: request.key,
            dispatch,
        });
    });
}

/* todo:
 * add update reminder*/

use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use crossterm::event::{self, Event};
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
use model::App;
use plugins::{PluginManager, PluginUiState};
use storage::Storage;

#[tokio::main]
async fn main() -> Result<()> {
    let args = cli::Args::parse();
    config::ensure_default_dirs();
    let mut config = load_config();

    if let Some(source_arg) = args.source {
        config.search.source = SearchSource::from(source_arg);
    }

    let plugin_manager = PluginManager::load(
        config.general.plugins_enabled,
        config.general.plugins_dir.as_str(),
    );

    let mut terminal = terminal::setup_terminal()?;
    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
    let (evt_tx, mut evt_rx) = mpsc::unbounded_channel();

    let core = Core::new(config.clone()).await?;
    tokio::spawn(core.run(cmd_rx, evt_tx.clone(), cmd_tx.clone()));

    let storage = Storage::init().expect("Failed to init storage");
    let mut app = App::new(storage);
    app.apply_config(&config);

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
    let running = true;

    while running {
        let ui_state = PluginUiState::from_runtime(
            app.active_tab,
            app.active_plugin_tab.clone(),
            ui_helpers::player_state_label(app.player_state),
            app.volume,
            app.muted,
            app.repeat_mode,
            app.search_query.clone(),
            app.album_search_query.clone(),
            app.queue.len(),
        );
        app.plugin_tabs = plugin_manager.collect_tabs(&ui_state);
        if let Some(active) = app.active_plugin_tab.clone() {
            if !app.plugin_tabs.iter().any(|t| t.id == active) {
                app.active_plugin_tab = None;
            }
        }
        app.plugin_panels = plugin_manager.collect_ui_panels(&ui_state);
        app.anim_tick = app.anim_tick.wrapping_add(1);
        terminal.draw(|frame| tui::draw(frame, &app))?;

        while let Ok(event) = evt_rx.try_recv() {
            let plugin_event = events::plugin_event_from_core_event(&event);
            if let Some(cmd) = events::apply_event(&mut app, event) {
                let _ = cmd_tx.send(cmd);
            }
            let dispatch = plugin_manager.dispatch_event(&plugin_event, &ui_state);
            if events::apply_plugin_dispatch(&mut app, &cmd_tx, dispatch) {
                continue;
            }
        }

        if event::poll(tick_rate)? {
            match event::read()? {
                Event::Mouse(mouse) => {
                    input::handle_mouse_event(&mut app, mouse);
                }
                Event::Key(key) => {
                    if !input::handle_key_event(&mut app, key, &ui_state, &plugin_manager, &cmd_tx)
                    {
                        break;
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

/* todo:
 * add update reminder*/

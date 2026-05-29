/* WARNING: Adding a function here is like playing Jenga with a live grenade. Change at your own risk. */
use ratatui::{
    prelude::*,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, List, ListItem, ListState, Paragraph, Tabs},
};

use crate::{
    config::{Palette, Theme},
    model::{eq_preset_name, App, Focus, PlayerState, RepeatMode, Song, Tab},
    plugins::PluginPanelItem,
    utils::natural_compare,
};

fn palette(theme: Theme) -> Palette {
    crate::config::load_palette(&theme)
}

fn search_source_label(source: &crate::config::SearchSource) -> String {
    match source {
        crate::config::SearchSource::YouTube => "YouTube".to_string(),
        crate::config::SearchSource::SoundCloud => "SoundCloud".to_string(),
    }
}

const VOLT_BLOCKS: [&str; 8] = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];

const SPECTRUM_COLORS: [Color; 12] = [
    Color::Rgb(255, 62, 205),
    Color::Rgb(230, 72, 255),
    Color::Rgb(175, 82, 255),
    Color::Rgb(118, 108, 255),
    Color::Rgb(72, 168, 255),
    Color::Rgb(38, 222, 255),
    Color::Rgb(0, 255, 198),
    Color::Rgb(0, 255, 138),
    Color::Rgb(112, 255, 82),
    Color::Rgb(255, 235, 48),
    Color::Rgb(255, 158, 38),
    Color::Rgb(255, 78, 78),
];

fn spectrum_spans(app: &App, width: usize) -> Vec<Span<'static>> {
    if width == 0 {
        return vec![];
    }

    let playing = app.player_state == PlayerState::Playing;
    let vol_factor = (app.volume as f32 / 100.0).clamp(0.2, 1.0);
    let tick = app.anim_tick as f64;

    (0..width)
        .map(|col| {
            let c = col as f64;

            let wave1 = (c * 0.1 + tick * 0.02).sin() * 0.3;

            let wave2 = (c * 0.4 + tick * 0.08).sin() * 0.2;

            let wave3 = (c * 1.2 + tick * 0.2).sin() * 0.1;

            let combined = (wave1 + wave2 + wave3 + 1.0) / 2.0;

            let level = if playing {
                (combined * 6.0 * vol_factor as f64).clamp(1.0, 7.0) as usize
            } else {
                (combined * 2.0 * 0.3).clamp(0.0, 3.0) as usize
            };

            let nc = SPECTRUM_COLORS.len();
            let idx = (col * nc / width + tick as usize / 15) % nc;

            Span::styled(
                VOLT_BLOCKS[level],
                Style::default().fg(SPECTRUM_COLORS[idx]),
            )
        })
        .collect()
}

pub fn draw(frame: &mut Frame, app: &App) {
    let pal = palette(app.theme.clone());
    let anim = pal.get_color("primary");
    let anim2 = pal.get_color("accent2");
    let size = frame.size();

    let tab_position = app.ui_layout.tab_bar_position.as_str();
    let tabs_width = app
        .ui_layout
        .tabs_width
        .min(size.width.saturating_sub(20))
        .max(1)
        .min(size.width);
    let (main_area, tab_area, vertical_tabs) = match tab_position {
        "left" => {
            let cols = Layout::horizontal([Constraint::Length(tabs_width), Constraint::Min(20)])
                .split(size);
            (cols[1], cols[0], true)
        }
        "right" => {
            let cols = Layout::horizontal([Constraint::Min(20), Constraint::Length(tabs_width)])
                .split(size);
            (cols[0], cols[1], true)
        }
        "bottom" => {
            let rows = Layout::vertical([Constraint::Min(8), Constraint::Length(3)]).split(size);
            (rows[0], rows[1], false)
        }
        _ => {
            let rows = Layout::vertical([Constraint::Length(3), Constraint::Min(8)]).split(size);
            (rows[1], rows[0], false)
        }
    };

    if vertical_tabs {
        draw_tabs_vertical(frame, app, &pal, anim, tab_area);
    } else {
        draw_tabs(frame, app, &pal, anim, anim2, tab_area);
    }

    let above_height = custom_sections_height(app, "above_player");
    let below_height = custom_sections_height(app, "below_player");
    let mut constraints = Vec::new();
    constraints.push(Constraint::Length(3));
    if above_height > 0 {
        constraints.push(Constraint::Length(above_height));
    }
    constraints.push(Constraint::Min(8));
    if app.ui_layout.visualizer_height > 0 {
        constraints.push(Constraint::Length(app.ui_layout.visualizer_height.max(3)));
    }
    if below_height > 0 {
        constraints.push(Constraint::Length(below_height));
    }
    if app.ui_layout.show_progress_bar {
        constraints.push(Constraint::Length(3));
    }
    if app.ui_layout.show_statusbar || app.ui_layout.show_keybind_hints {
        constraints.push(Constraint::Length(3));
    }
    let vertical = Layout::vertical(constraints).split(main_area);
    let mut row = 0;

    draw_search(frame, app, &pal, vertical[row]);
    row += 1;
    if above_height > 0 {
        draw_custom_sections(frame, app, &pal, anim, "above_player", vertical[row]);
        row += 1;
    }
    draw_content(frame, app, &pal, anim, vertical[row]);
    draw_custom_sections(frame, app, &pal, anim, "left", vertical[row]);
    draw_custom_sections(frame, app, &pal, anim, "right", vertical[row]);
    row += 1;
    if app.ui_layout.visualizer_height > 0 {
        draw_now_playing(frame, app, &pal, anim, vertical[row]);
        row += 1;
    }
    if below_height > 0 {
        draw_custom_sections(frame, app, &pal, anim, "below_player", vertical[row]);
        row += 1;
    }
    if app.ui_layout.show_progress_bar {
        draw_progress(frame, app, &pal, anim, vertical[row]);
        row += 1;
    }
    if app.ui_layout.show_statusbar || app.ui_layout.show_keybind_hints {
        draw_help(frame, app, &pal, vertical[row]);
    }
    draw_overlays(frame, app, &pal, anim, size);
}

fn custom_sections_height(app: &App, position: &str) -> u16 {
    if !app.allow_lua_ui_changes {
        return 0;
    }
    app.custom_sections
        .iter()
        .filter(|section| {
            section.position == position && !app.hidden_sections.iter().any(|id| id == &section.id)
        })
        .map(|section| section.height.unwrap_or(3))
        .max()
        .unwrap_or(0)
}

fn tab_defs_and_active(app: &App) -> (Vec<(String, String)>, usize) {
    let mut defs: Vec<(String, String)> = app
        .main_tabs
        .iter()
        .map(|tab| (tab.icon.clone(), tab.title.clone()))
        .collect();
    for t in &app.plugin_tabs {
        defs.push((
            t.icon.clone().unwrap_or_else(|| "◌".to_string()),
            t.title.to_uppercase(),
        ));
    }
    let active = if let Some(active_id) = &app.active_plugin_tab {
        app.plugin_tabs
            .iter()
            .position(|t| &t.id == active_id)
            .map(|i| i + app.main_tabs.len())
            .unwrap_or_else(|| app.active_tab_index().saturating_sub(1))
    } else {
        app.active_tab_index().saturating_sub(1)
    };
    (defs, active)
}

fn draw_tabs(frame: &mut Frame, app: &App, pal: &Palette, anim: Color, _anim2: Color, area: Rect) {
    let (defs, active) = tab_defs_and_active(app);

    let tab_lines: Vec<Line> = defs
        .iter()
        .enumerate()
        .map(|(i, (icon, label))| {
            if i == active {
                Line::from(vec![
                    Span::styled(
                        icon.to_string(),
                        Style::default().fg(anim).add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        label.to_string(),
                        Style::default().fg(anim).add_modifier(Modifier::BOLD),
                    ),
                ])
            } else {
                Line::from(vec![
                    Span::styled(icon.to_string(), Style::default().fg(pal.get_color("dim"))),
                    Span::raw(" "),
                    Span::styled(
                        label.to_string(),
                        Style::default().fg(pal.get_color("muted")),
                    ),
                ])
            }
        })
        .collect();

    let tabs = Tabs::new(tab_lines)
        .select(active)
        .block(
            Block::default()
                .title(Span::styled(
                    " ♪  R S - P U G  ♪ ",
                    Style::default().fg(anim).add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(anim)),
        )
        .style(Style::default().fg(pal.get_color("muted")))
        .highlight_style(Style::default().fg(anim).add_modifier(Modifier::BOLD))
        .divider(Span::styled("│", Style::default().fg(pal.get_color("dim"))));

    frame.render_widget(tabs, area);
}

fn draw_tabs_vertical(frame: &mut Frame, app: &App, pal: &Palette, anim: Color, area: Rect) {
    let (defs, active) = tab_defs_and_active(app);
    let items: Vec<ListItem> = defs
        .iter()
        .enumerate()
        .map(|(i, (icon, label))| {
            let number = i + 1;
            let shortcut = if number <= 8 {
                format!("{number:>2} ")
            } else {
                " · ".to_owned()
            };
            if i == active {
                ListItem::new(Line::from(vec![
                    Span::styled(
                        shortcut.clone(),
                        Style::default()
                            .fg(pal.get_color("warn"))
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        icon.clone(),
                        Style::default().fg(anim).add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        label.clone(),
                        Style::default().fg(anim).add_modifier(Modifier::BOLD),
                    ),
                ]))
            } else {
                ListItem::new(Line::from(vec![
                    Span::styled(shortcut, Style::default().fg(pal.get_color("dim"))),
                    Span::styled(icon.clone(), Style::default().fg(pal.get_color("dim"))),
                    Span::raw(" "),
                    Span::styled(label.clone(), Style::default().fg(pal.get_color("muted"))),
                ]))
            }
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(Span::styled(
                " ♪  TABS ",
                Style::default().fg(anim).add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(anim)),
    );
    frame.render_widget(list, area);
}

fn draw_search(frame: &mut Frame, app: &App, pal: &Palette, area: Rect) {
    let active_query = if app.active_tab == Tab::Albums {
        app.album_search_query.as_str()
    } else {
        app.search_query.as_str()
    };

    let (border_color, title_str) = if app.search_mode {
        (
            pal.get_color("info"),
            " ⌨  SEARCHING — type and press Enter ",
        )
    } else {
        (pal.get_color("dim"), " ⌕  SEARCH — press / to start ")
    };

    let content = if active_query.is_empty() && !app.search_mode {
        Line::from(Span::styled(
            format!("  search {}...", search_source_label(&app.opt_source)),
            Style::default()
                .fg(pal.get_color("dim"))
                .add_modifier(Modifier::ITALIC),
        ))
    } else {
        let cursor = if app.search_mode { "█" } else { "" };
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                format!("{}{}", active_query, cursor),
                Style::default()
                    .fg(pal.get_color("info"))
                    .add_modifier(Modifier::BOLD),
            ),
        ])
    };

    let widget = Paragraph::new(content).block(
        Block::default()
            .title(Span::styled(
                title_str,
                Style::default()
                    .fg(border_color)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color)),
    );
    frame.render_widget(widget, area);
}

fn panel_item_line(item: &PluginPanelItem, pal: &Palette) -> Line<'static> {
    match item {
        PluginPanelItem::Text { text } => Line::from(Span::styled(
            text.clone(),
            Style::default().fg(pal.get_color("text")),
        )),
        PluginPanelItem::Info { text } => Line::from(Span::styled(
            text.clone(),
            Style::default().fg(pal.get_color("info")),
        )),
        PluginPanelItem::Option { key, value } => Line::from(vec![
            Span::styled(
                format!("{}: ", key),
                Style::default().fg(pal.get_color("warn")),
            ),
            Span::styled(value.clone(), Style::default().fg(pal.get_color("text"))),
        ]),
        PluginPanelItem::Stat { label, value } => Line::from(vec![
            Span::styled(
                format!("{} ", label),
                Style::default().fg(pal.get_color("muted")),
            ),
            Span::styled(value.clone(), Style::default().fg(pal.get_color("ok"))),
        ]),
        PluginPanelItem::Separator => Line::from(Span::styled(
            "─".repeat(24),
            Style::default().fg(pal.get_color("dim")),
        )),
        PluginPanelItem::Header { text } => Line::from(Span::styled(
            text.clone(),
            Style::default()
                .fg(pal.get_color("accent2"))
                .add_modifier(Modifier::BOLD),
        )),
        PluginPanelItem::Keybind { key, action } => Line::from(vec![
            Span::styled(
                key.clone(),
                Style::default()
                    .fg(pal.get_color("warn"))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" → ", Style::default().fg(pal.get_color("dim"))),
            Span::styled(action.clone(), Style::default().fg(pal.get_color("text"))),
        ]),
        PluginPanelItem::Progress { label, percent } => {
            let pct = percent.clamp(0.0, 100.0).round() as usize;
            let filled = pct / 10;
            let bar = format!("{}{}", "█".repeat(filled), "░".repeat(10 - filled));
            Line::from(vec![
                Span::styled(
                    label.clone().unwrap_or_else(|| "progress".to_owned()),
                    Style::default().fg(pal.get_color("muted")),
                ),
                Span::raw(" "),
                Span::styled(bar, Style::default().fg(pal.get_color("ok"))),
                Span::styled(
                    format!(" {pct}%"),
                    Style::default().fg(pal.get_color("text")),
                ),
            ])
        }
    }
}

fn plugin_panel_lines(panel: &crate::plugins::PluginPanel, pal: &Palette) -> Vec<Line<'static>> {
    if !panel.items.is_empty() {
        panel
            .items
            .iter()
            .map(|item| panel_item_line(item, pal))
            .collect()
    } else {
        panel
            .lines
            .iter()
            .map(|line| {
                Line::from(Span::styled(
                    line.clone(),
                    Style::default().fg(pal.get_color("text")),
                ))
            })
            .collect()
    }
}

fn draw_content(frame: &mut Frame, app: &App, pal: &Palette, anim: Color, area: Rect) {
    let queue = if app.active_tab == Tab::Library {
        36
    } else {
        app.ui_layout.queue_width_percent.clamp(10, 90)
    };
    let split = if app.ui_layout.queue_position == "left" {
        [
            Constraint::Percentage(queue),
            Constraint::Percentage(100 - queue),
        ]
    } else {
        [
            Constraint::Percentage(100 - queue),
            Constraint::Percentage(queue),
        ]
    };
    let cols = Layout::horizontal(split).split(area);

    if app.ui_layout.queue_position == "left" {
        draw_queue_panel(frame, app, pal, anim, cols[0]);
        draw_results_panel(frame, app, pal, anim, cols[1]);
    } else {
        draw_results_panel(frame, app, pal, anim, cols[0]);
        draw_queue_panel(frame, app, pal, anim, cols[1]);
    }
}

fn draw_results_panel(frame: &mut Frame, app: &App, pal: &Palette, anim: Color, area: Rect) {
    let focused = app.focus == Focus::Results;

    let mut items: Vec<ListItem> = if app.active_plugin_tab.is_some()
        || app.active_custom_tab.is_some()
    {
        let plugin_items: Vec<ListItem> = app
            .plugin_panels
            .iter()
            .filter(|p| {
                matches!(
                    p.target,
                    None | Some(crate::plugins::PluginPanelTarget::Main)
                        | Some(crate::plugins::PluginPanelTarget::Results)
                )
            })
            .flat_map(|p| {
                let mut lines = vec![ListItem::new(Line::from(Span::styled(
                    format!("[{}]", p.title),
                    Style::default().fg(anim).add_modifier(Modifier::BOLD),
                )))];
                lines.extend(plugin_panel_lines(p, pal).into_iter().map(ListItem::new));
                lines
            })
            .collect();
        if plugin_items.is_empty() {
            vec![dim_item(
                "Plugin/custom tab active. Provide on_ui_panels().",
                pal,
            )]
        } else {
            plugin_items
        }
    } else if app.active_tab == Tab::Options {
        let eq_label = if app.eq_enabled {
            format!(
                "Equalizer      ON  ·  band {}/10  ·  {:.0} dB",
                app.eq_focus_band + 1,
                app.eq_bands[app.eq_focus_band]
            )
        } else {
            "Equalizer      OFF  (Enter to enable)".to_owned()
        };
        let rows: Vec<(&str, String)> = vec![
            (
                "⊞",
                format!("Search source  {}", search_source_label(&app.opt_source)),
            ),
            ("⊞", format!("Search limit   {}", app.opt_search_limit)),
            ("⊞", format!("MPV socket     {}", app.opt_socket)),
            (
                "⊞",
                if app.opt_editing && app.options_index == 2 {
                    format!("Music Dir      {}", app.opt_edit_buffer)
                } else {
                    format!(
                        "Music Dir      {}",
                        app.opt_music_dirs
                            .first()
                            .map(|s| s.as_str())
                            .unwrap_or("none")
                    )
                },
            ),
            ("⊞", "Smart Queue    press Enter".to_owned()),
            (
                "⊞",
                format!("Theme          {}", theme_label(app.opt_theme.clone())),
            ),
            ("⊞", format!("Repeat mode    {}", app.repeat_mode.label())),
            ("⊞", eq_label),
            (
                "⊞",
                if app.opt_editing && app.options_index == 7 {
                    format!("EQ preset      {}", app.opt_edit_buffer)
                } else {
                    format!(
                        "EQ preset      {}",
                        eq_preset_name(app, app.eq_preset_index)
                    )
                },
            ),
            ("⊞", format!("Key next       {}", app.key_next)),
            ("⊞", format!("Key prev       {}", app.key_prev)),
            ("⊞", format!("Key mute       {}", app.key_mute)),
        ];
        rows.into_iter()
            .enumerate()
            .map(|(i, (icon, label))| {
                if i == app.options_index {
                    ListItem::new(Line::from(vec![
                        Span::styled("▶ ", Style::default().fg(anim)),
                        Span::styled(icon.to_string(), Style::default().fg(anim)),
                        Span::raw(" "),
                        Span::styled(
                            label,
                            Style::default()
                                .fg(pal.get_color("text"))
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]))
                } else {
                    ListItem::new(Line::from(vec![
                        Span::styled("  ", Style::default()),
                        Span::styled(icon.to_string(), Style::default().fg(pal.get_color("dim"))),
                        Span::raw(" "),
                        Span::styled(label, Style::default().fg(pal.get_color("muted"))),
                    ]))
                }
            })
            .collect()
    } else if app.active_tab == Tab::Library {
        app.playlists
            .iter()
            .enumerate()
            .flat_map(|(idx, p)| {
                let is_sel = idx == app.selected_playlist;
                let open = app.playlist_expanded.get(idx).copied().unwrap_or(false);
                let arrow = if open { "▾" } else { "▸" };
                let mut items = vec![if is_sel {
                    ListItem::new(Line::from(vec![
                        Span::styled("▶ ", Style::default().fg(anim)),
                        Span::styled(arrow.to_string(), Style::default().fg(anim)),
                        Span::raw(" "),
                        Span::styled(
                            p.name.clone(),
                            Style::default()
                                .fg(pal.get_color("text"))
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!("  ·  {} tracks", p.songs.len()),
                            Style::default().fg(pal.get_color("muted")),
                        ),
                    ]))
                } else {
                    ListItem::new(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(
                            arrow.to_string(),
                            Style::default().fg(pal.get_color("muted")),
                        ),
                        Span::raw(" "),
                        Span::styled(p.name.clone(), Style::default().fg(pal.get_color("text"))),
                        Span::styled(
                            format!("  ·  {} tracks", p.songs.len()),
                            Style::default().fg(pal.get_color("dim")),
                        ),
                    ]))
                }];
                if open {
                    items.extend(p.songs.iter().map(|song| {
                        ListItem::new(Line::from(vec![
                            Span::styled(
                                "      ♪  ".to_string(),
                                Style::default().fg(pal.get_color("accent2")),
                            ),
                            Span::styled(
                                song.title.clone(),
                                Style::default().fg(pal.get_color("text")),
                            ),
                        ]))
                    }));
                }
                items
            })
            .collect()
    } else if app.active_tab == Tab::Albums {
        let mut current_flat_idx = 0;
        app.album_results
            .iter()
            .enumerate()
            .flat_map(|(idx, album)| {
                let is_album_sel = current_flat_idx == app.selected_album_result;
                let open = app.album_expanded.get(idx).copied().unwrap_or(false);
                let arrow = if open { "▾" } else { "▸" };
                let mut items = vec![if is_album_sel {
                    ListItem::new(Line::from(vec![
                        Span::styled("▶ ", Style::default().fg(anim)),
                        Span::styled(arrow.to_string(), Style::default().fg(anim)),
                        Span::raw(" "),
                        Span::styled(
                            album.name.clone(),
                            Style::default()
                                .fg(pal.get_color("text"))
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!("  ·  {} songs", album.songs.len()),
                            Style::default().fg(pal.get_color("muted")),
                        ),
                    ]))
                } else {
                    ListItem::new(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(
                            arrow.to_string(),
                            Style::default().fg(pal.get_color("muted")),
                        ),
                        Span::raw(" "),
                        Span::styled(
                            album.name.clone(),
                            Style::default().fg(pal.get_color("text")),
                        ),
                        Span::styled(
                            format!("  ·  {} songs", album.songs.len()),
                            Style::default().fg(pal.get_color("dim")),
                        ),
                    ]))
                }];
                current_flat_idx += 1;
                if open {
                    items.extend(album.songs.iter().enumerate().map(|(_s_idx, song)| {
                        let is_song_sel = current_flat_idx == app.selected_album_result;
                        current_flat_idx += 1;
                        ListItem::new(Line::from(vec![
                            if is_song_sel {
                                Span::styled("▶ ", Style::default().fg(anim))
                            } else {
                                Span::styled(
                                    "      ♪  ".to_string(),
                                    Style::default().fg(pal.get_color("accent2")),
                                )
                            },
                            Span::styled(
                                song.title.clone(),
                                if is_song_sel {
                                    Style::default().fg(anim).add_modifier(Modifier::BOLD)
                                } else {
                                    Style::default().fg(pal.get_color("text"))
                                },
                            ),
                        ]))
                    }));
                }
                items
            })
            .collect()
    } else if app.active_tab == Tab::Local {
        if app.local_view_mode == crate::model::LocalViewMode::Flat {
            build_local_song_list(
                &app.local_library_window,
                app.selected_local_song,
                focused,
                pal,
                anim,
            )
        } else {
            let (list, _) = build_organized_local_list(app, focused, pal, anim);
            list
        }
    } else {
        build_song_list(&app.search_results, app.selected_result, focused, pal, anim)
    };

    if app.allow_lua_ui_changes {
        let mut top: Vec<ListItem> = app
            .ui_inject
            .results_top
            .iter()
            .map(|item| ListItem::new(panel_item_line(item, pal)))
            .collect();
        if !top.is_empty() {
            top.extend(items);
            items = top;
        }
        items.extend(
            app.ui_inject
                .results_bottom
                .iter()
                .map(|item| ListItem::new(panel_item_line(item, pal))),
        );
    }

    let custom_title = app.active_custom_tab.as_ref().and_then(|id| {
        app.main_tabs
            .iter()
            .find(|tab| &tab.id == id)
            .map(|tab| format!(" ✦  {} {} ", tab.icon, tab.title))
    });
    let title = if let Some(title) = custom_title {
        title
    } else {
        match app.active_tab {
            Tab::Discover => " ♫  RESULTS ".to_owned(),
            Tab::Albums => " ◈  ALBUM RESULTS ".to_owned(),
            Tab::Library => " ◉  PLAYLISTS ".to_owned(),
            Tab::Options => {
                if let Some(active_id) = &app.active_plugin_tab {
                    if let Some(tab) = app.plugin_tabs.iter().find(|t| &t.id == active_id) {
                        let icon = tab.icon.clone().unwrap_or_else(|| "◌".to_string());
                        format!(" ⚙  SETTINGS — {} {} ", icon, tab.title.to_uppercase())
                    } else {
                        " ⚙  SETTINGS ".to_owned()
                    }
                } else {
                    " ⚙  SETTINGS ".to_owned()
                }
            }
            Tab::Local => {
                let mut t = " 🗀  LOCAL LIBRARY ".to_owned();
                if app.local_view_mode == crate::model::LocalViewMode::Organized {
                    match app.local_nav_level {
                        crate::model::LocalNavLevel::Artists => t.push_str(" ❯ Artists"),
                        crate::model::LocalNavLevel::Albums => {
                            if let Some(artist) = &app.local_nav_artist {
                                t = format!(" 🗀  LOCAL LIBRARY ❯ {} ❯ Albums", artist);
                            }
                        }
                        crate::model::LocalNavLevel::Songs => {
                            if let Some(artist) = &app.local_nav_artist {
                                if let Some(album) = &app.local_nav_album {
                                    t = format!(
                                        " 🗀  LOCAL LIBRARY ❯ {} ❯ {} ❯ Songs",
                                        artist, album
                                    );
                                }
                            }
                        }
                    }
                }
                t
            }
        }
    };
    let border_color = match app.active_tab {
        Tab::Options => pal.get_color("accent2"),
        _ => {
            if focused {
                anim
            } else {
                pal.get_color("dim")
            }
        }
    };

    let mut state = ListState::default();
    let selected_idx = match app.active_tab {
        Tab::Options => app.options_index,
        Tab::Library => app.selected_playlist,
        Tab::Albums => app.selected_album_result,
        Tab::Local => {
            if app.local_view_mode == crate::model::LocalViewMode::Flat {
                app.selected_local_song
            } else {
                app.selected_local_nav_idx
            }
        }
        _ => app.selected_result,
    };
    state.select(Some(selected_idx));

    let list = List::new(items)
        .block(
            Block::default()
                .title(Span::styled(
                    title,
                    Style::default()
                        .fg(border_color)
                        .add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color)),
        )
        .highlight_spacing(ratatui::widgets::HighlightSpacing::Always);

    frame.render_stateful_widget(list, area, &mut state);
}

fn build_song_list(
    songs: &[Song],
    selected: usize,
    focused: bool,
    pal: &Palette,
    anim: Color,
) -> Vec<ListItem<'static>> {
    songs
        .iter()
        .enumerate()
        .map(|(idx, song)| {
            let is_sel = idx == selected && focused;
            let title_line = if is_sel {
                Line::from(vec![
                    Span::styled("▶ ", Style::default().fg(anim)),
                    Span::styled(
                        song.title.clone(),
                        Style::default().fg(anim).add_modifier(Modifier::BOLD),
                    ),
                ])
            } else {
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        song.title.clone(),
                        Style::default().fg(pal.get_color("text")),
                    ),
                ])
            };
            let sub_color = if is_sel {
                pal.get_color("accent3")
            } else {
                pal.get_color("muted")
            };
            let sub_line = Line::from(vec![
                Span::styled(
                    "    ◦ ".to_string(),
                    Style::default().fg(pal.get_color("dim")),
                ),
                Span::styled(song.subtitle(), Style::default().fg(sub_color)),
            ]);
            ListItem::new(vec![title_line, sub_line])
        })
        .collect()
}

fn build_local_song_list<'a>(
    songs: &'a [crate::model::LocalSong],
    selected: usize,
    focused: bool,
    pal: &'a Palette,
    anim: Color,
) -> Vec<ListItem<'a>> {
    songs
        .iter()
        .enumerate()
        .map(|(idx, song)| {
            let is_sel = idx == selected && focused;
            let title_line = if is_sel {
                Line::from(vec![
                    Span::styled("▶ ", Style::default().fg(anim)),
                    Span::styled(
                        song.title.clone(),
                        Style::default().fg(anim).add_modifier(Modifier::BOLD),
                    ),
                ])
            } else {
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        song.title.clone(),
                        Style::default().fg(pal.get_color("text")),
                    ),
                ])
            };
            let sub_color = if is_sel {
                pal.get_color("accent3")
            } else {
                pal.get_color("muted")
            };
            let subtitle = format!(
                "{} • {} • {}",
                song.artist,
                song.album,
                format_time(song.duration)
            );
            let sub_line = Line::from(vec![
                Span::styled(
                    "    ◦ ".to_string(),
                    Style::default().fg(pal.get_color("dim")),
                ),
                Span::styled(subtitle, Style::default().fg(sub_color)),
            ]);
            ListItem::new(vec![title_line, sub_line])
        })
        .collect()
}

fn build_organized_local_list<'a>(
    app: &'a App,
    focused: bool,
    pal: &'a Palette,
    anim: Color,
) -> (Vec<ListItem<'a>>, Option<usize>) {
    match app.local_nav_level {
        crate::model::LocalNavLevel::Artists => {
            let mut artists: Vec<String> = app
                .local_library_window
                .iter()
                .map(|s| s.artist.clone())
                .collect();
            artists.sort_by(|a, b| natural_compare(a, b));
            artists.dedup();

            let items = artists
                .iter()
                .enumerate()
                .map(|(idx, artist)| {
                    let is_sel = idx == app.selected_local_nav_idx && focused;
                    let line = if is_sel {
                        Line::from(vec![
                            Span::styled("▶ ", Style::default().fg(anim)),
                            Span::styled(
                                artist.clone(),
                                Style::default().fg(anim).add_modifier(Modifier::BOLD),
                            ),
                        ])
                    } else {
                        Line::from(vec![
                            Span::raw("  "),
                            Span::styled(
                                artist.clone(),
                                Style::default().fg(pal.get_color("text")),
                            ),
                        ])
                    };
                    ListItem::new(line)
                })
                .collect();
            (items, Some(app.selected_local_nav_idx))
        }
        crate::model::LocalNavLevel::Albums => {
            let artist = app.local_nav_artist.as_deref().unwrap_or("Unknown");
            let mut albums: Vec<String> = app
                .local_library_window
                .iter()
                .filter(|s| s.artist == artist)
                .map(|s| s.album.clone())
                .collect();
            albums.sort_by(|a, b| natural_compare(a, b));
            albums.dedup();

            let items = albums
                .iter()
                .enumerate()
                .map(|(idx, album)| {
                    let is_sel = idx == app.selected_local_nav_idx && focused;
                    let line = if is_sel {
                        Line::from(vec![
                            Span::styled("▶ ", Style::default().fg(anim)),
                            Span::styled(
                                album.clone(),
                                Style::default().fg(anim).add_modifier(Modifier::BOLD),
                            ),
                        ])
                    } else {
                        Line::from(vec![
                            Span::raw("  "),
                            Span::styled(album.clone(), Style::default().fg(pal.get_color("text"))),
                        ])
                    };
                    ListItem::new(line)
                })
                .collect();
            (items, Some(app.selected_local_nav_idx))
        }
        crate::model::LocalNavLevel::Songs => {
            let artist = app.local_nav_artist.as_deref().unwrap_or("Unknown");
            let album = app.local_nav_album.as_deref().unwrap_or("Unknown");
            let mut songs: Vec<&crate::model::LocalSong> = app
                .local_library_window
                .iter()
                .filter(|s| s.artist == artist && s.album == album)
                .collect();
            songs.sort_by(|a, b| natural_compare(&a.title, &b.title));

            let items = songs
                .iter()
                .enumerate()
                .map(|(idx, song)| {
                    let is_sel = idx == app.selected_local_nav_idx && focused;
                    let title_line = if is_sel {
                        Line::from(vec![
                            Span::styled("▶ ", Style::default().fg(anim)),
                            Span::styled(
                                song.title.clone(),
                                Style::default().fg(anim).add_modifier(Modifier::BOLD),
                            ),
                        ])
                    } else {
                        Line::from(vec![
                            Span::raw("  "),
                            Span::styled(
                                song.title.clone(),
                                Style::default().fg(pal.get_color("text")),
                            ),
                        ])
                    };
                    let sub_color = if is_sel {
                        pal.get_color("accent3")
                    } else {
                        pal.get_color("dim")
                    };
                    let subtitle = format!(
                        "{} • {} • {}",
                        song.artist,
                        song.album,
                        format_time(song.duration)
                    );
                    let sub_line = Line::from(vec![
                        Span::styled(
                            "    ◦ ".to_string(),
                            Style::default().fg(pal.get_color("dim")),
                        ),
                        Span::styled(subtitle, Style::default().fg(sub_color)),
                    ]);
                    ListItem::new(vec![title_line, sub_line])
                })
                .collect();
            (items, Some(app.selected_local_nav_idx))
        }
    }
}

fn draw_queue_panel(frame: &mut Frame, app: &App, pal: &Palette, anim: Color, area: Rect) {
    let focused = app.focus == Focus::Queue;

    let chunks = if app.ui_layout.show_volume_bar {
        Layout::vertical([Constraint::Min(3), Constraint::Length(3)]).split(area)
    } else {
        Layout::vertical([Constraint::Min(3)]).split(area)
    };

    let mut items: Vec<ListItem> =
        if app.active_plugin_tab.is_some() || app.active_custom_tab.is_some() {
            {
                let plugin_items: Vec<ListItem> = app
                    .plugin_panels
                    .iter()
                    .filter(|p| p.target == Some(crate::plugins::PluginPanelTarget::Queue))
                    .flat_map(|p| {
                        let mut lines = vec![ListItem::new(Line::from(Span::styled(
                            format!("[{}]", p.title),
                            Style::default().fg(anim).add_modifier(Modifier::BOLD),
                        )))];
                        lines.extend(plugin_panel_lines(p, pal).into_iter().map(ListItem::new));
                        lines
                    })
                    .collect();
                if plugin_items.is_empty() {
                    vec![
                        dim_item("Plugin/custom tab controls", pal),
                        dim_item("Use plugin-defined keys", pal),
                        dim_item("Set panel target='queue' for side pane", pal),
                    ]
                } else {
                    plugin_items
                }
            }
        } else if app.active_tab == Tab::Options {
            if app.options_index == 7 || app.options_index == 8 {
                draw_eq_panel(frame, app, pal, anim, chunks[0]);

                if !app.ui_layout.show_volume_bar {
                    return;
                }

                let vol_ratio = (app.volume as f64 / 100.0).min(1.0);
                let vol_label = if app.muted {
                    format!(" MUTED  ({}%) ", app.volume)
                } else {
                    format!(" VOL  {}% ", app.volume)
                };
                let vol_color = if app.muted {
                    pal.get_color("muted")
                } else {
                    pal.get_color("ok")
                };
                let gauge = Gauge::default()
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(pal.get_color("dim"))),
                    )
                    .gauge_style(Style::default().fg(vol_color))
                    .ratio(vol_ratio)
                    .label(Span::styled(
                        vol_label,
                        Style::default()
                            .fg(pal.get_color("text"))
                            .add_modifier(Modifier::BOLD),
                    ));
                frame.render_widget(gauge, chunks[1]);
                return;
            }
            vec![
                dim_item("j / k     navigate options", pal),
                dim_item("h / l     change value", pal),
                dim_item("Enter     run action", pal),
                dim_item("On key rows: h/l cycle key", pal),
                dim_item("s         save config", pal),
                dim_item("r         toggle repeat", pal),
                dim_item("", pal),
                dim_item("Restart after socket changes.", pal),
            ]
        } else if app.active_tab == Tab::Library {
            let mut items: Vec<ListItem> = app
                .playlists
                .get(app.selected_playlist)
                .map(|p| {
                    p.songs
                        .iter()
                        .enumerate()
                        .map(|(idx, song)| {
                            let is_sel = idx == app.selected_playlist_song && focused;
                            if is_sel {
                                ListItem::new(Line::from(vec![
                                    Span::styled("▶ ", Style::default().fg(anim)),
                                    Span::styled(
                                        song.title.clone(),
                                        Style::default().fg(anim).add_modifier(Modifier::BOLD),
                                    ),
                                ]))
                            } else {
                                ListItem::new(Line::from(vec![
                                    Span::styled(
                                        "  ♪ ".to_string(),
                                        Style::default().fg(pal.get_color("dim")),
                                    ),
                                    Span::styled(
                                        song.title.clone(),
                                        Style::default().fg(pal.get_color("text")),
                                    ),
                                ]))
                            }
                        })
                        .collect()
                })
                .unwrap_or_default();
            items.push(dim_item("", pal));
            items.push(dim_item("Enter play  d delete  c menu", pal));
            if !app.recently_played.is_empty() {
                items.push(dim_item("", pal));
                items.push(ListItem::new(Span::styled(
                    "Recently played:",
                    Style::default()
                        .fg(pal.get_color("accent3"))
                        .add_modifier(Modifier::BOLD),
                )));
                items.extend(app.recently_played.iter().take(5).map(|song| {
                    ListItem::new(Line::from(vec![
                        Span::styled(
                            "  ↺ ".to_string(),
                            Style::default().fg(pal.get_color("dim")),
                        ),
                        Span::styled(
                            song.title.clone(),
                            Style::default().fg(pal.get_color("muted")),
                        ),
                    ]))
                }));
            }
            items
        } else {
            app.queue
                .iter()
                .enumerate()
                .map(|(idx, song)| {
                    let is_sel = idx == app.selected_queue && focused;
                    let num = format!("{:>2}.", idx + 1);
                    if is_sel {
                        ListItem::new(Line::from(vec![
                            Span::styled("▶ ", Style::default().fg(anim)),
                            Span::styled(num, Style::default().fg(pal.get_color("dim"))),
                            Span::raw(" "),
                            Span::styled(
                                song.title.clone(),
                                Style::default().fg(anim).add_modifier(Modifier::BOLD),
                            ),
                        ]))
                    } else {
                        ListItem::new(Line::from(vec![
                            Span::styled("   ", Style::default()),
                            Span::styled(num, Style::default().fg(pal.get_color("dim"))),
                            Span::raw(" "),
                            Span::styled(
                                song.title.clone(),
                                Style::default().fg(pal.get_color("muted")),
                            ),
                        ]))
                    }
                })
                .collect()
        };

    if app.allow_lua_ui_changes {
        let mut top: Vec<ListItem> = app
            .ui_inject
            .queue_top
            .iter()
            .map(|item| ListItem::new(panel_item_line(item, pal)))
            .collect();
        if !top.is_empty() {
            top.extend(items);
            items = top;
        }
        items.extend(
            app.ui_inject
                .queue_bottom
                .iter()
                .map(|item| ListItem::new(panel_item_line(item, pal))),
        );
    }

    let queue_title = match app.active_tab {
        Tab::Library => " PLAYLIST SONGS ",
        Tab::Options => " HELP ",
        _ => " QUEUE ",
    };
    let border_color = if focused {
        pal.get_color("warn")
    } else {
        pal.get_color("dim")
    };

    let mut state = ListState::default();
    let selected_idx = if app.active_tab == Tab::Library {
        app.selected_playlist_song
    } else {
        app.selected_queue
    };
    state.select(Some(selected_idx));

    let list = List::new(items).block(
        Block::default()
            .title(Span::styled(
                queue_title,
                Style::default()
                    .fg(border_color)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color)),
    );
    frame.render_stateful_widget(list, chunks[0], &mut state);
    if !app.ui_layout.show_volume_bar {
        return;
    }

    let vol_ratio = (app.volume as f64 / 100.0).min(1.0);
    let vol_label = if app.muted {
        format!(" MUTED  ({}%) ", app.volume)
    } else {
        format!(" VOL  {}% ", app.volume)
    };
    let vol_color = if app.muted {
        pal.get_color("muted")
    } else {
        pal.get_color("ok")
    };
    let gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(pal.get_color("dim"))),
        )
        .gauge_style(Style::default().fg(vol_color))
        .ratio(vol_ratio)
        .label(Span::styled(
            vol_label,
            Style::default()
                .fg(pal.get_color("text"))
                .add_modifier(Modifier::BOLD),
        ));
    frame.render_widget(gauge, chunks[1]);
}

fn dim_item(text: &'static str, pal: &Palette) -> ListItem<'static> {
    ListItem::new(Span::styled(
        text,
        Style::default().fg(pal.get_color("muted")),
    ))
}

fn draw_now_playing(frame: &mut Frame, app: &App, pal: &Palette, anim: Color, area: Rect) {
    let inner_w = area.width.saturating_sub(2) as usize;

    let (song_str, artist_str) = if let Some(song) = &app.current_song {
        (song.title.clone(), song.subtitle())
    } else {
        (
            "Nothing playing".to_owned(),
            "Press / to search  ·  Tab to move focus".to_owned(),
        )
    };

    let state_icon = match app.player_state {
        PlayerState::Playing => "▶",
        PlayerState::Paused => "⏸",
        PlayerState::Searching => "⌛",
        PlayerState::Idle => "⏹",
    };

    let repeat_badge = match app.repeat_mode {
        RepeatMode::Off => String::new(),
        RepeatMode::One => "  ↺¹ ONE".to_owned(),
        RepeatMode::All => "  ↺∞ ALL".to_owned(),
    };
    let mute_badge = if app.muted {
        "  [MUTED]".to_owned()
    } else {
        String::new()
    };

    let line1 = Line::from(vec![
        Span::styled(
            format!("{state_icon}  "),
            Style::default().fg(anim).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            song_str,
            Style::default()
                .fg(pal.get_color("text"))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{}{}", repeat_badge, mute_badge),
            Style::default()
                .fg(pal.get_color("warn"))
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    let line2 = Line::from(vec![
        Span::styled(
            "   ◦  ".to_string(),
            Style::default().fg(pal.get_color("dim")),
        ),
        Span::styled(artist_str, Style::default().fg(pal.get_color("accent3"))),
    ]);

    let playing = app.player_state == PlayerState::Playing;
    let spec_w = inner_w.saturating_sub(3);
    let mut spec = vec![Span::styled("  ".to_string(), Style::default())];
    spec.extend(spectrum_spans(app, spec_w));
    let line3 = Line::from(spec);

    let border_color = if playing { anim } else { pal.get_color("dim") };

    let widget = Paragraph::new(vec![line1, line2, line3]).block(
        Block::default()
            .title(Span::styled(
                " ♫  NOW PLAYING ",
                Style::default()
                    .fg(border_color)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color)),
    );
    frame.render_widget(widget, area);
}

fn draw_progress(frame: &mut Frame, app: &App, pal: &Palette, anim: Color, area: Rect) {
    let ratio = if app.playback_duration > 0.0 {
        (app.playback_pos / app.playback_duration).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let label = if app.playback_duration > 0.0 {
        format!(
            "  {}  ─  {}  ({:.0}%)",
            format_time(app.playback_pos),
            format_time(app.playback_duration),
            ratio * 100.0,
        )
    } else if app.current_song.is_some() {
        format!("  {}  ─  loading...", format_time(app.playback_pos))
    } else {
        "  ─  no track loaded".to_owned()
    };

    let gauge_color = match app.player_state {
        PlayerState::Playing => anim,
        PlayerState::Paused => pal.get_color("warn"),
        _ => pal.get_color("dim"),
    };

    let gauge = Gauge::default()
        .block(
            Block::default()
                .title(Span::styled(
                    " PROGRESS ",
                    Style::default().fg(pal.get_color("muted")),
                ))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(pal.get_color("dim"))),
        )
        .gauge_style(Style::default().fg(gauge_color))
        .ratio(ratio)
        .label(Span::styled(
            label,
            Style::default()
                .fg(pal.get_color("text"))
                .add_modifier(Modifier::BOLD),
        ));
    frame.render_widget(gauge, area);
}

fn tab_key_hint(app: &App) -> String {
    let count = (app.main_tabs.len() + app.plugin_tabs.len()).clamp(1, 8);
    format!("1-{count}")
}

fn key_span(text: impl Into<String>, pal: &Palette) -> Span<'static> {
    Span::styled(
        text.into(),
        Style::default()
            .fg(pal.get_color("warn"))
            .add_modifier(Modifier::BOLD),
    )
}

fn draw_help(frame: &mut Frame, app: &App, pal: &Palette, area: Rect) {
    macro_rules! key {
        ($text:expr) => {
            key_span($text, pal)
        };
    }
    let sep = || -> Span<'static> { Span::styled(":", Style::default().fg(pal.get_color("dim"))) };
    let act = |a: &'static str| -> Span<'static> {
        Span::styled(a, Style::default().fg(pal.get_color("muted")))
    };
    let gap = || -> Span<'static> { Span::raw("  ") };

    let mut spans = if app.ui_layout.show_keybind_hints {
        vec![
            key!(tab_key_hint(app)),
            sep(),
            act("tabs"),
            gap(),
            key!("/"),
            sep(),
            act("search"),
            gap(),
            key!("Tab"),
            sep(),
            act("focus"),
            gap(),
            key!("Enter"),
            sep(),
            act("play"),
            gap(),
            key!("Space"),
            sep(),
            act("pause"),
            gap(),
            key!("n"),
            sep(),
            act("next"),
            gap(),
            key!("d"),
            sep(),
            act("remove"),
            gap(),
            key!("9/0"),
            sep(),
            act("vol"),
            gap(),
            key!("c"),
            sep(),
            act("menu"),
            gap(),
            key!("a/x"),
            sep(),
            act("playlists"),
            gap(),
            key!("q"),
            sep(),
            act("quit"),
        ]
    } else {
        Vec::new()
    };
    if app.ui_layout.show_statusbar {
        if let Some(warning) = app.plugin_warnings.back() {
            spans.push(gap());
            spans.push(Span::styled(
                "⚠",
                Style::default()
                    .fg(pal.get_color("warn"))
                    .add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::raw(" "));
            spans.push(Span::styled(
                warning.clone(),
                Style::default().fg(pal.get_color("warn")),
            ));
        }
        for item in &app.ui_inject.statusbar_extra {
            spans.push(gap());
            spans.extend(panel_item_line(item, pal).spans);
        }
    }
    let widget = Paragraph::new(Line::from(spans)).block(
        Block::default()
            .title(Span::styled(
                " KEYBINDS ",
                Style::default().fg(pal.get_color("dim")),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(pal.get_color("dim"))),
    );
    frame.render_widget(widget, area);
}

fn draw_custom_sections(
    frame: &mut Frame,
    app: &App,
    pal: &Palette,
    anim: Color,
    position: &str,
    area: Rect,
) {
    if !app.allow_lua_ui_changes {
        return;
    }
    let sections: Vec<_> = app
        .custom_sections
        .iter()
        .filter(|section| {
            section.position == position && !app.hidden_sections.iter().any(|id| id == &section.id)
        })
        .collect();
    if sections.is_empty() {
        return;
    }
    let height = sections
        .iter()
        .map(|section| section.height.unwrap_or(3))
        .max()
        .unwrap_or(3)
        .min(area.height);
    if height == 0 {
        return;
    }
    let y = match position {
        "above_player" => area.y,
        "below_player" => area.y + area.height.saturating_sub(height),
        _ => area.y,
    };
    let count = sections.len() as u16;
    let each_width = (area.width / count.max(1)).max(1);
    for (idx, section) in sections.iter().enumerate() {
        let width = section.width.unwrap_or(each_width).min(area.width);
        let x = match position {
            "right" => area.x + area.width.saturating_sub(width),
            "left" => area.x,
            _ => area.x + (idx as u16).saturating_mul(each_width),
        };
        let section_area = Rect::new(
            x,
            y,
            width.min(area.width.saturating_sub(x.saturating_sub(area.x))),
            height,
        );
        if section_area.width == 0 || section_area.height == 0 {
            continue;
        }
        let rows: Vec<Line> = app
            .ui_section_items
            .get(&section.id)
            .map(|items| {
                items
                    .iter()
                    .map(|item| panel_item_line(item, pal))
                    .collect()
            })
            .unwrap_or_else(Vec::new);
        frame.render_widget(Clear, section_area);
        frame.render_widget(
            Paragraph::new(rows).block(
                Block::default()
                    .title(Span::styled(
                        format!(" {} ", section.id),
                        Style::default().fg(anim).add_modifier(Modifier::BOLD),
                    ))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(anim)),
            ),
            section_area,
        );
    }
}

fn draw_eq_panel(frame: &mut Frame, app: &App, pal: &Palette, anim: Color, area: Rect) {
    const BAND_LABELS: [&str; 10] = [
        "32", "64", "125", "250", "500", "1k", "2k", "4k", "8k", "16k",
    ];
    const MAX_DB: f32 = 12.0;
    const EQ_BLOCKS: [&str; 8] = [" ", "▁", "▂", "▃", "▄", "▅", "▆", "█"];

    let inner = area.inner(&ratatui::layout::Margin {
        horizontal: 1,
        vertical: 1,
    });
    let title_color = if app.eq_enabled {
        anim
    } else {
        pal.get_color("muted")
    };

    let block = Block::default()
        .title(Span::styled(
            if app.eq_enabled {
                " ▶ EQUALIZER  (ON) "
            } else {
                " ⏹ EQUALIZER  (OFF) "
            },
            Style::default()
                .fg(title_color)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(title_color));
    frame.render_widget(block, area);

    if inner.width < 10 || inner.height < 4 {
        return;
    }

    let band_count = 10usize;
    let bar_h = inner.height.saturating_sub(4) as usize;
    let bar_w = (inner.width as usize / band_count).max(2);

    for (i, &gain) in app.eq_bands.iter().enumerate() {
        let col_x = inner.x + (i * bar_w) as u16;
        if col_x >= inner.x + inner.width {
            break;
        }

        let focused = i == app.eq_focus_band && app.options_index == 7;
        let band_color = if focused {
            anim
        } else if app.eq_enabled {
            SPECTRUM_COLORS[i % SPECTRUM_COLORS.len()]
        } else {
            pal.get_color("dim")
        };
        let bg_color = if app.eq_enabled {
            SPECTRUM_COLORS[(i + 4) % SPECTRUM_COLORS.len()]
        } else {
            pal.get_color("dim")
        };

        let norm = (gain / MAX_DB).clamp(-1.0, 1.0);
        let mid_row = inner.y + (bar_h / 2) as u16;
        let max_half = (bar_h / 2).max(1);
        let cells_from_mid = (norm.abs() * max_half as f32 * 8.0).round() as usize;

        for row in inner.y..(inner.y + bar_h as u16) {
            let cell_dist = if row <= mid_row {
                (mid_row - row) as usize
            } else {
                (row - mid_row) as usize
            };
            let units_start = cell_dist * 8;
            let units_end = units_start + 8;
            let fill_units = cells_from_mid
                .saturating_sub(units_start)
                .min(units_end - units_start);

            let is_upper_half = row < mid_row;
            let should_fill = if norm >= 0.0 {
                is_upper_half
            } else {
                row > mid_row
            };

            let ch = if row == mid_row {
                "─".to_owned()
            } else if should_fill {
                EQ_BLOCKS[fill_units.min(7)].to_owned()
            } else {
                "·".to_owned()
            };

            let style = if row == mid_row {
                Style::default().fg(pal.get_color("dim"))
            } else if should_fill && fill_units > 0 {
                Style::default().fg(band_color).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(bg_color)
            };
            let cell_w = bar_w.min((inner.x + inner.width).saturating_sub(col_x) as usize) as u16;
            if cell_w == 0 {
                break;
            }
            let cell_area = Rect::new(col_x, row, cell_w, 1);
            frame.render_widget(
                Paragraph::new(ch.repeat(cell_area.width as usize))
                    .alignment(Alignment::Center)
                    .style(style),
                cell_area,
            );
        }

        let db_str = if gain == 0.0 {
            " 0".to_owned()
        } else {
            format!("{:+.0}", gain)
        };
        let db_area = Rect::new(col_x, inner.y + bar_h as u16 + 1, bar_w as u16, 1);
        if db_area.y < inner.y + inner.height {
            frame.render_widget(
                Paragraph::new(db_str).alignment(Alignment::Center).style(
                    Style::default()
                        .fg(if focused {
                            anim
                        } else {
                            pal.get_color("muted")
                        })
                        .add_modifier(if focused {
                            Modifier::BOLD
                        } else {
                            Modifier::empty()
                        }),
                ),
                db_area,
            );
        }

        let lbl_area = Rect::new(col_x, inner.y + bar_h as u16 + 2, bar_w as u16, 1);
        if lbl_area.y < inner.y + inner.height {
            let lbl = BAND_LABELS[i];
            frame.render_widget(
                Paragraph::new(lbl)
                    .alignment(Alignment::Center)
                    .style(Style::default().fg(if focused { anim } else { pal.get_color("dim") })),
                lbl_area,
            );
        }
    }

    let info_y = inner.y + inner.height.saturating_sub(2);
    if info_y > inner.y {
        let focus_hint = match app.eq_focus_band {
            0 | 1 => "Sub-bass and body",
            2 | 3 => "Warmth and kick",
            4 | 5 => "Mids and vocals",
            6 | 7 => "Presence and attack",
            _ => "Air and detail",
        };
        let info = format!(
            "Band {} ({})  •  {}",
            app.eq_focus_band + 1,
            BAND_LABELS[app.eq_focus_band],
            focus_hint
        );
        frame.render_widget(
            Paragraph::new(info)
                .alignment(Alignment::Center)
                .style(Style::default().fg(pal.get_color("muted"))),
            Rect::new(inner.x, info_y, inner.width, 1),
        );
    }

    let hint_y = inner.y + inner.height.saturating_sub(1);
    if hint_y >= inner.y {
        let hint = "h/l: band  +/-: gain  Enter: on/off  0: reset  f: save preset";

        let hint_area = Rect::new(inner.x, hint_y, inner.width, 1);
        frame.render_widget(
            Paragraph::new(hint)
                .alignment(Alignment::Center)
                .style(Style::default().fg(pal.get_color("dim"))),
            hint_area,
        );
    }
}

fn draw_plugin_panels(frame: &mut Frame, app: &App, pal: &Palette, anim: Color, size: Rect) {
    if app.plugin_panels.is_empty() {
        return;
    }
    let mut y = size.y + 1;
    for panel in app
        .plugin_panels
        .iter()
        .filter(|p| p.target == Some(crate::plugins::PluginPanelTarget::Overlay))
    {
        let item_lines: Vec<Line> = if panel.items.is_empty() {
            panel
                .lines
                .iter()
                .map(|line| {
                    Line::from(Span::styled(
                        line.clone(),
                        Style::default().fg(pal.get_color("text")),
                    ))
                })
                .collect()
        } else {
            panel
                .items
                .iter()
                .map(|item| panel_item_line(item, pal))
                .collect()
        };
        let lines = item_lines.len().clamp(1, 10) as u16;
        let content_width = item_lines
            .iter()
            .map(|line| line.width() as u16)
            .max()
            .unwrap_or(0)
            .max(panel.title.chars().count() as u16)
            .min(size.width.saturating_sub(8));
        let w = (content_width + 4)
            .max(20)
            .min(size.width.saturating_sub(2));
        let h = (lines + 2).min(size.height.saturating_sub(y.saturating_sub(size.y) + 1));
        if h < 3 {
            break;
        }
        let x = size.x + size.width.saturating_sub(w + 1);
        let area = Rect::new(x, y, w, h);
        let rows: Vec<Line> = item_lines.into_iter().take(lines as usize).collect();
        frame.render_widget(Clear, area);
        frame.render_widget(
            Paragraph::new(rows).block(
                Block::default()
                    .title(Span::styled(
                        format!(" {} ", panel.title),
                        Style::default().fg(anim).add_modifier(Modifier::BOLD),
                    ))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(anim)),
            ),
            area,
        );
        y = y.saturating_add(h + 1);
        if y >= size.y + size.height.saturating_sub(3) {
            break;
        }
    }
}

fn draw_overlays(frame: &mut Frame, app: &App, pal: &Palette, anim: Color, size: Rect) {
    draw_plugin_panels(frame, app, pal, anim, size);
    let msg = app.shown_message();
    if !msg.is_empty() {
        let area = centered_rect(68, 22, size);
        frame.render_widget(Clear, area);
        frame.render_widget(
            Paragraph::new(msg)
                .block(
                    Block::default()
                        .title(Span::styled(
                            " ✦  NOTICE  ✦ ",
                            Style::default().fg(anim).add_modifier(Modifier::BOLD),
                        ))
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(anim)),
                )
                .style(Style::default().fg(pal.get_color("ok")))
                .alignment(Alignment::Center),
            area,
        );
    }

    if app.context_open {
        let menu_w = if app.active_tab == Tab::Library && app.focus == Focus::Results {
            74
        } else {
            44
        };
        let area = centered_rect(menu_w, 32, size);
        frame.render_widget(Clear, area);
        let options: &[&str] = if app.active_tab == Tab::Library && app.focus == Focus::Results {
            &[
                "⇪  Import playlist  (~/.config/rs-pug/import_playlist.json)",
                "⇩  Export selected playlist",
            ]
        } else if app.active_tab == Tab::Local {
            &[
                "◈  Add to Playlist",
                "✦  Create new playlist",
                "✕  Remove from queue",
            ]
        } else {
            &[
                "◈  Add to selected playlist",
                "✦  Create new playlist",
                "✕  Remove from queue",
                "✕  Remove from playlist",
            ]
        };
        let items: Vec<ListItem> = options
            .iter()
            .enumerate()
            .map(|(idx, o)| {
                if idx == app.context_index {
                    ListItem::new(Line::from(vec![
                        Span::styled("▶ ", Style::default().fg(anim)),
                        Span::styled(
                            o.to_string(),
                            Style::default()
                                .fg(pal.get_color("text"))
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]))
                } else {
                    ListItem::new(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(o.to_string(), Style::default().fg(pal.get_color("muted"))),
                    ]))
                }
            })
            .collect();
        let menu = List::new(items).block(
            Block::default()
                .title(Span::styled(
                    if app.adding_song_to_playlist {
                        " ✦  SELECT PLAYLIST  (Enter / Esc) "
                    } else {
                        " ✦  SONG MENU  (Enter / Esc) "
                    },
                    Style::default().fg(anim).add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(anim)),
        );
        frame.render_widget(menu, area);
    }

    if app.confirm_delete_playlist {
        let area = centered_rect(54, 26, size);
        frame.render_widget(Clear, area);
        let text = format!(
            "\n  Delete playlist \"{}\"?\n\n  y / Enter  →  confirm\n  n / Esc    →  cancel",
            app.delete_playlist_name,
        );
        frame.render_widget(
            Paragraph::new(text)
                .block(
                    Block::default()
                        .title(Span::styled(
                            " ⚠  CONFIRM DELETE ",
                            Style::default()
                                .fg(pal.get_color("warn"))
                                .add_modifier(Modifier::BOLD),
                        ))
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(pal.get_color("warn"))),
                )
                .style(Style::default().fg(pal.get_color("text"))),
            area,
        );
    }

    if app.scanning {
        let area = centered_rect(40, 14, size);
        frame.render_widget(Clear, area);
        frame.render_widget(
            Paragraph::new(" ⚙  Scanning library... ")
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(anim)),
                )
                .style(
                    Style::default()
                        .fg(pal.get_color("text"))
                        .add_modifier(Modifier::BOLD),
                )
                .alignment(Alignment::Center),
            area,
        );
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(r);
    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(popup[1])[1]
}

fn format_time(seconds: f64) -> String {
    let secs = seconds.max(0.0).round() as u64;
    format!("{:02}:{:02}", secs / 60, secs % 60)
}

fn theme_label(theme: Theme) -> String {
    match theme {
        Theme::Dark => "dark".to_string(),
        Theme::Light => "light".to_string(),
        Theme::Nord => "nord".to_string(),
        Theme::Gruvbox => "gruvbox".to_string(),
        Theme::Mono => "mono".to_string(),
        Theme::Custom(name) => name,
    }
}

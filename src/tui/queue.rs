use super::*;
use super::eq::draw_eq_panel;

pub(super) fn draw_queue_panel(frame: &mut Frame, app: &App, pal: &Palette, anim: Color, area: Rect) {
    let focused = app.focus == Focus::Queue;
    let chunks = if app.ui_layout.show_volume_bar {
        Layout::vertical([Constraint::Min(3), Constraint::Length(3)]).split(area)
    } else {
        Layout::vertical([Constraint::Min(3)]).split(area)
    };
    let mut items: Vec<ListItem> =
        if app.plugin_ui.active_tab.is_some() || app.plugin_ui.active_custom_tab.is_some() {
            {
                let plugin_items: Vec<ListItem> = app
                    .plugin_ui
                    .panels
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
                .playlists
                .get(app.playlists.selected_playlist)
                .map(|p| {
                    p.songs
                        .iter()
                        .enumerate()
                        .map(|(idx, song)| {
                            let is_sel = idx == app.playlists.selected_song && focused;
                            if is_sel {
                                ListItem::new(Line::from(vec![
                                    Span::styled("▶ ", Style::default().fg(anim)),
                                    Span::styled(
                                        song.title.as_str(),
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
                                        song.title.as_str(),
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
                            song.title.as_str(),
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
                                song.title.as_str(),
                                Style::default().fg(anim).add_modifier(Modifier::BOLD),
                            ),
                        ]))
                    } else {
                        ListItem::new(Line::from(vec![
                            Span::styled("   ", Style::default()),
                            Span::styled(num, Style::default().fg(pal.get_color("dim"))),
                            Span::raw(" "),
                            Span::styled(
                                song.title.as_str(),
                                Style::default().fg(pal.get_color("muted")),
                            ),
                        ]))
                    }
                })
                .collect()
        };
    if app.plugin_ui.allow_lua_ui_changes {
        let mut top: Vec<ListItem> = app
            .plugin_ui
            .inject
            .queue_top
            .iter()
            .map(|item| ListItem::new(panel_item_line(item, pal)))
            .collect();
        if !top.is_empty() {
            top.extend(items);
            items = top;
        }
        items.extend(
            app.plugin_ui
                .inject
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
        app.playlists.selected_song
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

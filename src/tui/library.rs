use super::*;
use super::queue::draw_queue_panel;

pub(super) fn draw_content(frame: &mut Frame, app: &App, pal: &Palette, anim: Color, area: Rect) {
    let queue = app.ui_layout.queue_width_percent.clamp(10, 90);
    let queue_width = Constraint::Ratio(queue as u32, 100);
    let results_width = Constraint::Ratio((100 - queue) as u32, 100);
    let split = if app.ui_layout.queue_position == "left" {
        [queue_width, results_width]
    } else {
        [results_width, queue_width]
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
    let mut items: Vec<ListItem> = if app.plugin_ui.active_tab.is_some()
        || app.plugin_ui.active_custom_tab.is_some()
    {
        let plugin_items: Vec<ListItem> = app
            .plugin_ui
            .panels
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
        let eq_label = if app.eq.enabled {
            format!(
                "Equalizer      ON  ·  band {}/10  ·  {:.0} dB",
                app.eq.focus_band + 1,
                app.eq.bands[app.eq.focus_band]
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
                if app.opt_editing && app.options_index == 3 {
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
                format!("Theme          {}", theme_label(&app.opt_theme)),
            ),
            ("⊞", format!("Repeat mode    {}", app.repeat_mode.label())),
            ("⊞", eq_label),
            (
                "⊞",
                if app.opt_editing && app.options_index == 8 {
                    format!("EQ preset      {}", app.opt_edit_buffer)
                } else {
                    format!(
                        "EQ preset      {}",
                        eq_preset_name(app, app.eq.preset_index)
                    )
                },
            ),
            ("⊞", format!("Key next       {}", app.key_next)),
            ("⊞", format!("Key prev       {}", app.key_prev)),
            ("⊞", format!("Key mute       {}", app.key_mute)),
            (
                "⊞",
                format!(
                    "Speed          {:.2}x  (h/l adjust · Enter reset)",
                    app.playback_speed
                ),
            ),
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
            .playlists
            .iter()
            .enumerate()
            .flat_map(|(idx, p)| {
                let is_sel = idx == app.playlists.selected_playlist;
                let open = app.playlists.expanded.get(idx).copied().unwrap_or(false);
                let arrow = if open { "▾" } else { "▸" };
                let mut items = vec![if is_sel {
                    ListItem::new(Line::from(vec![
                        Span::styled("▶ ", Style::default().fg(anim)),
                        Span::styled(arrow.to_string(), Style::default().fg(anim)),
                        Span::raw(" "),
                        Span::styled(
                            p.name.as_str(),
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
                        Span::styled(p.name.as_str(), Style::default().fg(pal.get_color("text"))),
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
                                song.title.as_str(),
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
        app.albums
            .results
            .iter()
            .enumerate()
            .flat_map(|(idx, album)| {
                let is_album_sel = current_flat_idx == app.albums.selected_result;
                let open = app.albums.expanded.get(idx).copied().unwrap_or(false);
                let arrow = if open { "▾" } else { "▸" };
                let mut items = vec![if is_album_sel {
                    ListItem::new(Line::from(vec![
                        Span::styled("▶ ", Style::default().fg(anim)),
                        Span::styled(arrow.to_string(), Style::default().fg(anim)),
                        Span::raw(" "),
                        Span::styled(
                            album.name.as_str(),
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
                            album.name.as_str(),
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
                    items.extend(album.songs.iter().map(|song| {
                        let is_song_sel = current_flat_idx == app.albums.selected_result;
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
                                song.title.as_str(),
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
        if app.local.view_mode == crate::model::LocalViewMode::Flat {
            build_local_song_list(
                &app.local.window,
                app.local.selected_song,
                focused,
                pal,
                anim,
                &app.multi_select,
            )
        } else {
            let (list, _) = build_organized_local_list(app, focused, pal, anim);
            list
        }
    } else {
        build_song_list(
            &app.search.results,
            app.search.selected_result,
            focused,
            pal,
            anim,
            &app.multi_select,
        )
    };
    if app.plugin_ui.allow_lua_ui_changes {
        let mut top: Vec<ListItem> = app
            .plugin_ui
            .inject
            .results_top
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
                .results_bottom
                .iter()
                .map(|item| ListItem::new(panel_item_line(item, pal))),
        );
    }
    let custom_title = app.plugin_ui.active_custom_tab.as_ref().and_then(|id| {
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
                if let Some(active_id) = &app.plugin_ui.active_tab {
                    if let Some(tab) = app.plugin_ui.tabs.iter().find(|t| &t.id == active_id) {
                        let icon = tab.icon.as_deref().unwrap_or("◌");
                        format!(" ⚙  SETTINGS — {} {} ", icon, tab.title.to_uppercase())
                    } else {
                        " ⚙  SETTINGS ".to_owned()
                    }
                } else {
                    " ⚙  SETTINGS ".to_owned()
                }
            }
            Tab::Local => {
                let mut t = format!(" 🗀  LOCAL LIBRARY — sort: {} ", app.local.sort_mode.label());
                let filters: Vec<String> = [
                    app.local
                        .filter_genre
                        .as_ref()
                        .map(|v| format!("genre={v}")),
                    app.local
                        .filter_artist
                        .as_ref()
                        .map(|v| format!("artist={v}")),
                    app.local
                        .filter_album
                        .as_ref()
                        .map(|v| format!("album={v}")),
                ]
                .into_iter()
                .flatten()
                .collect();
                if !filters.is_empty() {
                    t.push_str(&format!(" — filters: {}", filters.join(", ")));
                }
                if app.local.view_mode == crate::model::LocalViewMode::Organized {
                    match app.local.nav_level {
                        crate::model::LocalNavLevel::Artists => t.push_str(" ❯ Artists"),
                        crate::model::LocalNavLevel::Albums => {
                            if let Some(artist) = &app.local.nav_artist {
                                t = format!(" 🗀  LOCAL LIBRARY ❯ {} ❯ Albums", artist);
                            }
                        }
                        crate::model::LocalNavLevel::Songs => {
                            if let Some(artist) = &app.local.nav_artist {
                                if let Some(album) = &app.local.nav_album {
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
        Tab::Library => app.playlists.selected_playlist,
        Tab::Albums => app.albums.selected_result,
        Tab::Local => {
            if app.local.view_mode == crate::model::LocalViewMode::Flat {
                app.local.selected_song
            } else {
                app.local.selected_nav_idx
            }
        }
        _ => app.search.selected_result,
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
    marked: &std::collections::HashSet<String>,
) -> Vec<ListItem<'static>> {
    songs
        .iter()
        .enumerate()
        .map(|(idx, song)| {
            let is_sel = idx == selected && focused;
            let is_marked = marked.contains(&song.id);
            let cursor = if is_sel { "▶ " } else { "  " };
            let mark = if is_marked { "✓" } else { " " };
            let mark_color = if is_marked {
                pal.get_color("ok")
            } else {
                pal.get_color("dim")
            };
            let title_line = if is_sel {
                Line::from(vec![
                    Span::styled(cursor, Style::default().fg(anim)),
                    Span::styled(mark, Style::default().fg(mark_color).add_modifier(Modifier::BOLD)),
                    Span::raw(" "),
                    Span::styled(
                        song.title.clone(),
                        Style::default().fg(anim).add_modifier(Modifier::BOLD),
                    ),
                ])
            } else {
                Line::from(vec![
                    Span::styled(cursor, Style::default()),
                    Span::styled(mark, Style::default().fg(mark_color).add_modifier(Modifier::BOLD)),
                    Span::raw(" "),
                    Span::styled(
                        song.title.clone(),
                        Style::default().fg(if is_marked {
                            pal.get_color("ok")
                        } else {
                            pal.get_color("text")
                        }),
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
    marked: &std::collections::HashSet<String>,
) -> Vec<ListItem<'a>> {
    songs
        .iter()
        .enumerate()
        .map(|(idx, song)| {
            let is_sel = idx == selected && focused;
            let is_marked = marked.contains(&song.path);
            let cursor = if is_sel { "▶ " } else { "  " };
            let mark = if is_marked { "✓" } else { " " };
            let mark_color = if is_marked {
                pal.get_color("ok")
            } else {
                pal.get_color("dim")
            };
            let title_line = if is_sel {
                Line::from(vec![
                    Span::styled(cursor, Style::default().fg(anim)),
                    Span::styled(mark, Style::default().fg(mark_color).add_modifier(Modifier::BOLD)),
                    Span::raw(" "),
                    Span::styled(
                        song.title.as_str(),
                        Style::default().fg(anim).add_modifier(Modifier::BOLD),
                    ),
                ])
            } else {
                Line::from(vec![
                    Span::styled(cursor, Style::default()),
                    Span::styled(mark, Style::default().fg(mark_color).add_modifier(Modifier::BOLD)),
                    Span::raw(" "),
                    Span::styled(
                        song.title.as_str(),
                        Style::default().fg(if is_marked {
                            pal.get_color("ok")
                        } else {
                            pal.get_color("text")
                        }),
                    ),
                ])
            };
            let sub_color = if is_sel {
                pal.get_color("accent3")
            } else {
                pal.get_color("muted")
            };
            let subtitle = format!(
                "{} • {} • {} • {} • {}",
                song.artist,
                song.album,
                song.genre,
                song.year
                    .map(|y| y.to_string())
                    .unwrap_or_else(|| "----".to_owned()),
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
    match app.local.nav_level {
        crate::model::LocalNavLevel::Artists => {
            let mut artists: Vec<String> =
                app.local.window.iter().map(|s| s.artist.clone()).collect();
            artists.sort_by(|a, b| natural_compare(a, b));
            artists.dedup();
            let items = artists
                .into_iter()
                .enumerate()
                .map(|(idx, artist)| {
                    let is_sel = idx == app.local.selected_nav_idx && focused;
                    let line = if is_sel {
                        Line::from(vec![
                            Span::styled("▶ ", Style::default().fg(anim)),
                            Span::styled(
                                artist,
                                Style::default().fg(anim).add_modifier(Modifier::BOLD),
                            ),
                        ])
                    } else {
                        Line::from(vec![
                            Span::raw("  "),
                            Span::styled(artist, Style::default().fg(pal.get_color("text"))),
                        ])
                    };
                    ListItem::new(line)
                })
                .collect();
            (items, Some(app.local.selected_nav_idx))
        }
        crate::model::LocalNavLevel::Albums => {
            let artist = app.local.nav_artist.as_deref().unwrap_or("Unknown");
            let mut albums: Vec<String> = app
                .local
                .window
                .iter()
                .filter(|s| s.artist == artist)
                .map(|s| s.album.clone())
                .collect();
            albums.sort_by(|a, b| natural_compare(a, b));
            albums.dedup();
            let items = albums
                .into_iter()
                .enumerate()
                .map(|(idx, album)| {
                    let is_sel = idx == app.local.selected_nav_idx && focused;
                    let line = if is_sel {
                        Line::from(vec![
                            Span::styled("▶ ", Style::default().fg(anim)),
                            Span::styled(
                                album,
                                Style::default().fg(anim).add_modifier(Modifier::BOLD),
                            ),
                        ])
                    } else {
                        Line::from(vec![
                            Span::raw("  "),
                            Span::styled(album, Style::default().fg(pal.get_color("text"))),
                        ])
                    };
                    ListItem::new(line)
                })
                .collect();
            (items, Some(app.local.selected_nav_idx))
        }
        crate::model::LocalNavLevel::Songs => {
            let artist = app.local.nav_artist.as_deref().unwrap_or("Unknown");
            let album = app.local.nav_album.as_deref().unwrap_or("Unknown");
            let mut songs: Vec<&crate::model::LocalSong> = app
                .local
                .window
                .iter()
                .filter(|s| s.artist == artist && s.album == album)
                .collect();
            songs.sort_by(|a, b| natural_compare(&a.title, &b.title));
            let items = songs
                .iter()
                .enumerate()
                .map(|(idx, song)| {
                    let is_sel = idx == app.local.selected_nav_idx && focused;
                    let title_line = if is_sel {
                        Line::from(vec![
                            Span::styled("▶ ", Style::default().fg(anim)),
                            Span::styled(
                                song.title.as_str(),
                                Style::default().fg(anim).add_modifier(Modifier::BOLD),
                            ),
                        ])
                    } else {
                        Line::from(vec![
                            Span::raw("  "),
                            Span::styled(
                                song.title.as_str(),
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
            (items, Some(app.local.selected_nav_idx))
        }
    }
}

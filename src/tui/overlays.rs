use super::*;
use super::plugins_panel::draw_plugin_panels;
use crate::actions;

pub(super) fn draw_overlays(frame: &mut Frame, app: &App, pal: &Palette, anim: Color, size: Rect) {
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
    if app.local.tag_editor_open {
        let area = centered_rect(70, 32, size);
        frame.render_widget(Clear, area);
        let song = app.local.tag_editor_song.as_ref();
        let lines = vec![
            Line::from(vec![
                Span::styled("Field: ", Style::default().fg(pal.get_color("muted"))),
                Span::styled(
                    app.local.tag_editor_field.label(),
                    Style::default().fg(anim).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("> ", Style::default().fg(anim)),
                Span::styled(
                    app.local.tag_edit_buffer.as_str(),
                    Style::default().fg(pal.get_color("text")),
                ),
            ]),
            Line::from(""),
            Line::from(
                song.map(|s| {
                    format!(
                        "{} • {} • {} • {} • {}",
                        s.title,
                        s.artist,
                        s.album,
                        s.genre,
                        s.year
                            .map(|y| y.to_string())
                            .unwrap_or_else(|| "----".to_owned())
                    )
                })
                .unwrap_or_default(),
            ),
            Line::from(""),
            Line::from("Tab: next field  Enter: write tags  Esc: cancel"),
        ];
        frame.render_widget(
            Paragraph::new(lines).block(
                Block::default()
                    .title(Span::styled(
                        " ID3 TAG EDITOR ",
                        Style::default().fg(anim).add_modifier(Modifier::BOLD),
                    ))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(anim)),
            ),
            area,
        );
    }
    if app.playlists.context_open {
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
                "⇩  Download Song",
                "✕  Remove from queue",
                "✕  Remove from playlist",
            ]
        };
        let items: Vec<ListItem> = options
            .iter()
            .enumerate()
            .map(|(idx, o)| {
                if idx == app.playlists.context_index {
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
                    if app.playlists.adding_song {
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
    if app.playlists.confirm_delete {
        let area = centered_rect(54, 26, size);
        frame.render_widget(Clear, area);
        let text = format!(
            "\n  Delete playlist \"{}\"?\n\n  y / Enter  →  confirm\n  n / Esc    →  cancel",
            app.playlists.delete_name,
        );
        frame.render_widget(
            Paragraph::new(text)
                .block(
                    Block::default()
                        .title(Span::styled(
                            "   CONFIRM DELETE ",
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
    if app.local.scanning {
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
    if app.help_open {
        let area = centered_rect(74, 80, size);
        frame.render_widget(Clear, area);
        let commands = actions::all_commands();
        let items: Vec<ListItem> = commands
            .iter()
            .map(|cmd| {
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("{:<18}", cmd.name),
                        Style::default()
                            .fg(pal.get_color("text"))
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(cmd.hint, Style::default().fg(pal.get_color("muted"))),
                ]))
            })
            .collect();
        let list = List::new(items).block(
            Block::default()
                .title(Span::styled(
                    " COMMANDS  (also reachable by typing them after `:`) ",
                    Style::default().fg(anim).add_modifier(Modifier::BOLD),
                ))
                .title_bottom(Span::styled(
                    " q / Esc / ?  close ",
                    Style::default().fg(pal.get_color("dim")),
                ))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(anim)),
        );
        frame.render_widget(list, area);
    }
    if app.palette_open {
        let area = centered_rect(60, 50, size);
        frame.render_widget(Clear, area);
        let matches = actions::filter_commands(&app.palette_query);
        let chunks = Layout::vertical([Constraint::Length(3), Constraint::Min(3)]).split(area);
        let query_line = Line::from(vec![
            Span::styled(":", Style::default().fg(anim).add_modifier(Modifier::BOLD)),
            Span::raw(" "),
            Span::styled(
                app.palette_query.as_str(),
                Style::default().fg(pal.get_color("text")),
            ),
            Span::styled("▏", Style::default().fg(anim)),
        ]);
        frame.render_widget(
            Paragraph::new(query_line).block(
                Block::default()
                    .title(Span::styled(
                        " COMMAND PALETTE ",
                        Style::default().fg(anim).add_modifier(Modifier::BOLD),
                    ))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(anim)),
            ),
            chunks[0],
        );
        let items: Vec<ListItem> = if matches.is_empty() {
            vec![dim_item("No matching commands", pal)]
        } else {
            matches
                .iter()
                .enumerate()
                .map(|(idx, cmd)| {
                    if idx == app.palette_selected {
                        ListItem::new(Line::from(vec![
                            Span::styled("▶ ", Style::default().fg(anim)),
                            Span::styled(
                                cmd.name,
                                Style::default()
                                    .fg(pal.get_color("text"))
                                    .add_modifier(Modifier::BOLD),
                            ),
                            Span::raw("  "),
                            Span::styled(cmd.hint, Style::default().fg(pal.get_color("accent3"))),
                        ]))
                    } else {
                        ListItem::new(Line::from(vec![
                            Span::raw("  "),
                            Span::styled(cmd.name, Style::default().fg(pal.get_color("muted"))),
                            Span::raw("  "),
                            Span::styled(cmd.hint, Style::default().fg(pal.get_color("dim"))),
                        ]))
                    }
                })
                .collect()
        };
        let mut state = ListState::default();
        if !matches.is_empty() {
            state.select(Some(app.palette_selected.min(matches.len().saturating_sub(1))));
        }
        let list = List::new(items).block(
            Block::default()
                .title(Span::styled(
                    " ↑↓ navigate · Enter run · Esc close ",
                    Style::default().fg(pal.get_color("dim")),
                ))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(pal.get_color("dim"))),
        );
        frame.render_stateful_widget(list, chunks[1], &mut state);
    }
}

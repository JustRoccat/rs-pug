use ratatui::{
    prelude::*,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, List, ListItem, Paragraph, Tabs, ListState},
};

use crate::{
    config::Theme,
    model::{eq_preset_name, App, Focus, PlayerState, RepeatMode, Song, Tab},
};

struct Palette {
    text: Color,
    dim: Color,
    muted: Color,
    info: Color,
    warn: Color,
    ok: Color,
    primary: Color,
    accent2: Color,
    accent3: Color,
}

fn palette(theme: Theme) -> Palette {
    match theme {
        Theme::Light => Palette {
            text: Color::Rgb(20, 20, 35),
            dim: Color::Rgb(90, 90, 115),
            muted: Color::Rgb(140, 135, 158),
            info: Color::Rgb(0, 120, 210),
            warn: Color::Rgb(185, 128, 0),
            ok: Color::Rgb(0, 158, 88),
            primary: Color::Rgb(20, 120, 220),
            accent2: Color::Rgb(110, 10, 210),
            accent3: Color::Rgb(0, 158, 210),
        },
        Theme::Nord => Palette {
            text: Color::Rgb(216, 222, 233),
            dim: Color::Rgb(76, 86, 106),
            muted: Color::Rgb(129, 161, 193),
            info: Color::Rgb(136, 192, 208),
            warn: Color::Rgb(235, 203, 139),
            ok: Color::Rgb(163, 190, 140),
            primary: Color::Rgb(94, 129, 172),
            accent2: Color::Rgb(129, 161, 193),
            accent3: Color::Rgb(136, 192, 208),
        },
        Theme::Gruvbox => Palette {
            text: Color::Rgb(235, 219, 178),
            dim: Color::Rgb(102, 92, 84),
            muted: Color::Rgb(168, 153, 132),
            info: Color::Rgb(131, 165, 152),
            warn: Color::Rgb(250, 189, 47),
            ok: Color::Rgb(184, 187, 38),
            primary: Color::Rgb(215, 153, 33),
            accent2: Color::Rgb(211, 134, 155),
            accent3: Color::Rgb(104, 157, 106),
        },
        Theme::Mono => Palette {
            text: Color::Rgb(230, 230, 230),
            dim: Color::Rgb(90, 90, 90),
            muted: Color::Rgb(150, 150, 150),
            info: Color::Rgb(190, 190, 190),
            warn: Color::Rgb(220, 220, 220),
            ok: Color::Rgb(200, 200, 200),
            primary: Color::Rgb(245, 245, 245),
            accent2: Color::Rgb(210, 210, 210),
            accent3: Color::Rgb(175, 175, 175),
        },
        _ => Palette {
            // Deep space neon palette
            text: Color::Rgb(225, 218, 248),
            dim: Color::Rgb(68, 62, 102),
            muted: Color::Rgb(108, 100, 140),
            info: Color::Rgb(82, 216, 255),
            warn: Color::Rgb(255, 205, 52),
            ok: Color::Rgb(52, 255, 162),
            primary: Color::Rgb(255, 62, 205),
            accent2: Color::Rgb(152, 82, 255),
            accent3: Color::Rgb(0, 228, 255),
        },
    }
}

fn animated_accent(tick: u64) -> Color {
    use std::f64::consts::TAU;
    let t = tick as f64 * 0.042;
    let r = ((t.sin() * 0.5 + 0.5) * 205.0 + 50.0) as u8;
    let g = (((t + TAU / 3.0).sin() * 0.5 + 0.5) * 35.0) as u8;
    let b = (((t + TAU * 2.0 / 3.0).sin() * 0.5 + 0.5) * 205.0 + 50.0) as u8;
    Color::Rgb(r, g, b)
}

fn animated_secondary(tick: u64) -> Color {
    use std::f64::consts::TAU;
    let t = tick as f64 * 0.042 + TAU / 4.0;
    let r = ((t.sin() * 0.5 + 0.5) * 150.0 + 50.0) as u8;
    let g = (((t + TAU / 3.0).sin() * 0.5 + 0.5) * 30.0) as u8;
    let b = (((t + TAU * 2.0 / 3.0).sin() * 0.5 + 0.5) * 205.0 + 50.0) as u8;
    Color::Rgb(r, g, b)
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

fn spectrum_spans(tick: u64, width: usize, playing: bool) -> Vec<Span<'static>> {
    if width == 0 {
        return vec![];
    }
    if !playing {
        return (0..width)
            .map(|col| {
                let h = (tick as f64 * 0.025 + col as f64 * 0.38).sin() * 0.28 + 0.35;
                let level = (h * 2.5).clamp(0.0, 7.0) as usize;
                Span::styled(
                    VOLT_BLOCKS[level],
                    Style::default().fg(Color::Rgb(68, 58, 105)),
                )
            })
            .collect();
    }

    (0..width)
        .map(|col| {
            let t = tick as f64;
            let c = col as f64;

            let w1 = (c * 0.33 + t * 0.132).sin();
            let w2 = (c * 0.69 + t * 0.188).sin() * 0.58;
            let w3 = (c * 0.17 + t * 0.072).sin() * 0.38;
            let w4 = (c * 1.12 + t * 0.295).sin() * 0.18;
            let w5 = (c * 0.52 + t * 0.215).sin() * 0.12;

            let fp = (col
                .wrapping_mul(6271)
                .wrapping_add(col.wrapping_mul(col).wrapping_mul(104723))
                % 100) as f64
                / 400.0;

            let combined = ((w1 + w2 + w3 + w4 + w5) / 2.26 + 1.0) / 2.0 * 0.82 + fp;
            let level = (combined.clamp(0.0, 1.0) * 7.0) as usize;

            let nc = SPECTRUM_COLORS.len();
            let idx = (col * nc / width + tick as usize / 10) % nc;

            Span::styled(
                VOLT_BLOCKS[level],
                Style::default().fg(SPECTRUM_COLORS[idx]),
            )
        })
        .collect()
}

pub fn draw(frame: &mut Frame, app: &App) {
    let pal = palette(app.theme);
    let anim = if matches!(app.theme, Theme::Custom) {
        animated_accent(app.anim_tick)
    } else {
        pal.primary
    };
    let anim2 = if matches!(app.theme, Theme::Custom) {
        animated_secondary(app.anim_tick)
    } else {
        pal.accent2
    };
    let size = frame.size();

    let vertical = Layout::vertical([
        Constraint::Length(3), // tab bar
        Constraint::Length(3), // search
        Constraint::Min(8),    // results + queue
        Constraint::Length(5), // now playing
        Constraint::Length(3), // progress
        Constraint::Length(3), // help
    ])
    .split(size);

    draw_tabs(frame, app, &pal, anim, anim2, vertical[0]);
    draw_search(frame, app, &pal, vertical[1]);
    draw_content(frame, app, &pal, anim, vertical[2]);
    draw_now_playing(frame, app, &pal, anim, vertical[3]);
    draw_progress(frame, app, &pal, anim, vertical[4]);
    draw_help(frame, &pal, vertical[5]);
    draw_overlays(frame, app, &pal, anim, size);
}

fn draw_tabs(frame: &mut Frame, app: &App, pal: &Palette, anim: Color, _anim2: Color, area: Rect) {
    let active = match app.active_tab {
        Tab::Discover => 0,
        Tab::Albums => 1,
        Tab::Library => 2,
        Tab::Options => 3,
    };

    let defs: &[(&str, &str)] = &[
        ("♫", "DISCOVER"),
        ("◈", "ALBUMS"),
        ("◉", "LIBRARY"),
        ("⚙", "OPTIONS"),
    ];

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
                    Span::styled(icon.to_string(), Style::default().fg(pal.dim)),
                    Span::raw(" "),
                    Span::styled(label.to_string(), Style::default().fg(pal.muted)),
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
        .style(Style::default().fg(pal.muted))
        .highlight_style(Style::default().fg(anim).add_modifier(Modifier::BOLD))
        .divider(Span::styled("│", Style::default().fg(pal.dim)));

    frame.render_widget(tabs, area);
}

fn draw_search(frame: &mut Frame, app: &App, pal: &Palette, area: Rect) {
    let active_query = if app.active_tab == Tab::Albums {
        app.album_search_query.as_str()
    } else {
        app.search_query.as_str()
    };

    let (border_color, title_str) = if app.search_mode {
        (pal.info, " ⌨  SEARCHING — type and press Enter ")
    } else {
        (pal.dim, " ⌕  SEARCH — press / to start ")
    };

    let content = if active_query.is_empty() && !app.search_mode {
        Line::from(Span::styled(
            "  search YouTube, Bandcamp, SoundCloud...",
            Style::default().fg(pal.dim).add_modifier(Modifier::ITALIC),
        ))
    } else {
        let cursor = if app.search_mode { "█" } else { "" };
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                format!("{}{}", active_query, cursor),
                Style::default().fg(pal.info).add_modifier(Modifier::BOLD),
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

fn draw_content(frame: &mut Frame, app: &App, pal: &Palette, anim: Color, area: Rect) {
    let split = if app.active_tab == Tab::Library {
        [Constraint::Percentage(64), Constraint::Percentage(36)]
    } else {
        [Constraint::Percentage(60), Constraint::Percentage(40)]
    };
    let cols = Layout::horizontal(split).split(area);

    draw_results_panel(frame, app, pal, anim, cols[0]);
    draw_queue_panel(frame, app, pal, anim, cols[1]);
}

fn draw_results_panel(frame: &mut Frame, app: &App, pal: &Palette, anim: Color, area: Rect) {
    let focused = app.focus == Focus::Results;

    let items: Vec<ListItem> = if app.active_tab == Tab::Options {
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
            ("⊞", format!("Search limit   {}", app.opt_search_limit)),
            ("⊞", format!("MPV socket     {}", app.opt_socket)),
            ("⊞", "Smart Queue    press Enter".to_owned()),
            (
                "⊞",
                format!("Theme          {}", theme_label(app.opt_theme)),
            ),
            ("⊞", format!("Repeat mode    {}", app.repeat_mode.label())),
            ("⊞", eq_label),
            (
                "⊞",
                format!("EQ preset      {}", eq_preset_name(app.eq_preset_index)),
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
                            Style::default().fg(pal.text).add_modifier(Modifier::BOLD),
                        ),
                    ]))
                } else {
                    ListItem::new(Line::from(vec![
                        Span::styled("  ", Style::default()),
                        Span::styled(icon.to_string(), Style::default().fg(pal.dim)),
                        Span::raw(" "),
                        Span::styled(label, Style::default().fg(pal.muted)),
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
                            Style::default().fg(pal.text).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!("  ·  {} tracks", p.songs.len()),
                            Style::default().fg(pal.muted),
                        ),
                    ]))
                } else {
                    ListItem::new(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(arrow.to_string(), Style::default().fg(pal.muted)),
                        Span::raw(" "),
                        Span::styled(p.name.clone(), Style::default().fg(pal.text)),
                        Span::styled(
                            format!("  ·  {} tracks", p.songs.len()),
                            Style::default().fg(pal.dim),
                        ),
                    ]))
                }];
                if open {
                    items.extend(p.songs.iter().map(|song| {
                        ListItem::new(Line::from(vec![
                            Span::styled("      ♪  ".to_string(), Style::default().fg(pal.accent2)),
                            Span::styled(song.title.clone(), Style::default().fg(pal.text)),
                        ]))
                    }));
                }
                items
            })
            .collect()
    } else if app.active_tab == Tab::Albums {
        build_song_list(
            &app.album_results,
            app.selected_album_result,
            focused,
            pal,
            anim,
        )
    } else {
        build_song_list(&app.search_results, app.selected_result, focused, pal, anim)
    };

    let (panel_title, border_color) = match app.active_tab {
        Tab::Discover => (" ♫  RESULTS ", if focused { anim } else { pal.dim }),
        Tab::Albums => (" ◈  ALBUM RESULTS ", if focused { anim } else { pal.dim }),
        Tab::Library => (" ◉  PLAYLISTS ", if focused { anim } else { pal.dim }),
        Tab::Options => (" ⚙  SETTINGS ", pal.accent2),
    };

    let mut state = ListState::default();
    let selected_idx = match app.active_tab {
        Tab::Options => app.options_index,
        Tab::Library => app.selected_playlist,
        Tab::Albums => app.selected_album_result,
        _ => app.selected_result,
    };
    state.select(Some(selected_idx));

    let list = List::new(items)
        .block(
            Block::default()
                .title(Span::styled(
                    panel_title,
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
                    Span::styled(song.title.clone(), Style::default().fg(pal.text)),
                ])
            };
            let sub_color = if is_sel { pal.accent3 } else { pal.muted };
            let sub_line = Line::from(vec![
                Span::styled("    ◦ ".to_string(), Style::default().fg(pal.dim)),
                Span::styled(song.subtitle(), Style::default().fg(sub_color)),
            ]);
            ListItem::new(vec![title_line, sub_line])
        })
        .collect()
}

fn draw_queue_panel(frame: &mut Frame, app: &App, pal: &Palette, anim: Color, area: Rect) {
    let focused = app.focus == Focus::Queue;

    let chunks = Layout::vertical([Constraint::Min(3), Constraint::Length(3)]).split(area);

    // ── Queue list ──────────────────────────────────────────────────
    let items: Vec<ListItem> = if app.active_tab == Tab::Options {
        if app.options_index == 5 {
            // EQ visualizer as list items
            draw_eq_panel(frame, app, pal, anim, chunks[0]);
            // skip normal list rendering by returning early after volume
            let vol_ratio = (app.volume as f64 / 100.0).min(1.0);
            let vol_label = if app.muted {
                format!(" MUTED  ({}%) ", app.volume)
            } else {
                format!(" VOL  {}% ", app.volume)
            };
            let vol_color = if app.muted { pal.muted } else { pal.ok };
            let gauge = Gauge::default()
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(pal.dim)),
                )
                .gauge_style(Style::default().fg(vol_color))
                .ratio(vol_ratio)
                .label(Span::styled(
                    vol_label,
                    Style::default().fg(pal.text).add_modifier(Modifier::BOLD),
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
                                Span::styled("  ♪ ".to_string(), Style::default().fg(pal.dim)),
                                Span::styled(song.title.clone(), Style::default().fg(pal.text)),
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
                    .fg(pal.accent3)
                    .add_modifier(Modifier::BOLD),
            )));
            items.extend(app.recently_played.iter().take(5).map(|song| {
                ListItem::new(Line::from(vec![
                    Span::styled("  ↺ ".to_string(), Style::default().fg(pal.dim)),
                    Span::styled(song.title.clone(), Style::default().fg(pal.muted)),
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
                        Span::styled(num, Style::default().fg(pal.dim)),
                        Span::raw(" "),
                        Span::styled(
                            song.title.clone(),
                            Style::default().fg(anim).add_modifier(Modifier::BOLD),
                        ),
                    ]))
                } else {
                    ListItem::new(Line::from(vec![
                        Span::styled("   ", Style::default()),
                        Span::styled(num, Style::default().fg(pal.dim)),
                        Span::raw(" "),
                        Span::styled(song.title.clone(), Style::default().fg(pal.muted)),
                    ]))
                }
            })
            .collect()
    };

    let queue_title = match app.active_tab {
        Tab::Library => " PLAYLIST SONGS ",
        Tab::Options => " HELP ",
        _ => " QUEUE ",
    };
    let border_color = if focused { pal.warn } else { pal.dim };

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

    // ── Volume gauge ─────────────────────────────────────────────────
    let vol_ratio = (app.volume as f64 / 100.0).min(1.0);
    let vol_label = if app.muted {
        format!(" MUTED  ({}%) ", app.volume)
    } else {
        format!(" VOL  {}% ", app.volume)
    };
    let vol_color = if app.muted { pal.muted } else { pal.ok };
    let gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(pal.dim)),
        )
        .gauge_style(Style::default().fg(vol_color))
        .ratio(vol_ratio)
        .label(Span::styled(
            vol_label,
            Style::default().fg(pal.text).add_modifier(Modifier::BOLD),
        ));
    frame.render_widget(gauge, chunks[1]);
}

fn dim_item(text: &'static str, pal: &Palette) -> ListItem<'static> {
    ListItem::new(Span::styled(text, Style::default().fg(pal.muted)))
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
            Style::default().fg(pal.text).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{}{}", repeat_badge, mute_badge),
            Style::default().fg(pal.warn).add_modifier(Modifier::BOLD),
        ),
    ]);

    let line2 = Line::from(vec![
        Span::styled("   ◦  ".to_string(), Style::default().fg(pal.dim)),
        Span::styled(artist_str, Style::default().fg(pal.accent3)),
    ]);

    let playing = app.player_state == PlayerState::Playing;
    let spec_w = inner_w.saturating_sub(3);
    let mut spec = vec![Span::styled("  ".to_string(), Style::default())];
    spec.extend(spectrum_spans(app.anim_tick, spec_w, playing));
    let line3 = Line::from(spec);

    let border_color = if playing { anim } else { pal.dim };

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
        PlayerState::Paused => pal.warn,
        _ => pal.dim,
    };

    let gauge = Gauge::default()
        .block(
            Block::default()
                .title(Span::styled(" PROGRESS ", Style::default().fg(pal.muted)))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(pal.dim)),
        )
        .gauge_style(Style::default().fg(gauge_color))
        .ratio(ratio)
        .label(Span::styled(
            label,
            Style::default().fg(pal.text).add_modifier(Modifier::BOLD),
        ));
    frame.render_widget(gauge, area);
}

fn draw_help(frame: &mut Frame, pal: &Palette, area: Rect) {
    let key = |k: &'static str| -> Span<'static> {
        Span::styled(
            k,
            Style::default().fg(pal.warn).add_modifier(Modifier::BOLD),
        )
    };
    let sep = || -> Span<'static> { Span::styled(":", Style::default().fg(pal.dim)) };
    let act =
        |a: &'static str| -> Span<'static> { Span::styled(a, Style::default().fg(pal.muted)) };
    let gap = || -> Span<'static> { Span::raw("  ") };

    let line = Line::from(vec![
        key("1-4"),
        sep(),
        act("tabs"),
        gap(),
        key("/"),
        sep(),
        act("search"),
        gap(),
        key("Tab"),
        sep(),
        act("focus"),
        gap(),
        key("Enter"),
        sep(),
        act("play"),
        gap(),
        key("Space"),
        sep(),
        act("pause"),
        gap(),
        key("n"),
        sep(),
        act("next"),
        gap(),
        key("d"),
        sep(),
        act("remove"),
        gap(),
        key("9/0"),
        sep(),
        act("vol"),
        gap(),
        key("c"),
        sep(),
        act("menu"),
        gap(),
        key("a/x"),
        sep(),
        act("playlists"),
        gap(),
        key("q"),
        sep(),
        act("quit"),
    ]);

    let widget = Paragraph::new(line).block(
        Block::default()
            .title(Span::styled(" KEYBINDS ", Style::default().fg(pal.dim)))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(pal.dim)),
    );
    frame.render_widget(widget, area);
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
    let title_color = if app.eq_enabled { anim } else { pal.muted };

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
    let bar_h = inner.height.saturating_sub(4) as usize; // leave room for value + label + hint
    let bar_w = (inner.width as usize / band_count).max(2);

    // draw each band
    for (i, &gain) in app.eq_bands.iter().enumerate() {
        let col_x = inner.x + (i * bar_w) as u16;
        if col_x >= inner.x + inner.width {
            break;
        }

        let focused = i == app.eq_focus_band && app.options_index == 5;
        let band_color = if focused {
            anim
        } else if app.eq_enabled {
            SPECTRUM_COLORS[i % SPECTRUM_COLORS.len()]
        } else {
            pal.dim
        };
        let bg_color = if app.eq_enabled {
            SPECTRUM_COLORS[(i + 4) % SPECTRUM_COLORS.len()]
        } else {
            pal.dim
        };

        // normalise gain to -1..1 and map to cell count around 0dB center line
        let norm = (gain / MAX_DB).clamp(-1.0, 1.0);
        let mid_row = inner.y + (bar_h / 2) as u16;
        let max_half = (bar_h / 2).max(1);
        let cells_from_mid = (norm.abs() * max_half as f32 * 8.0).round() as usize;

        // draw bar cells with partial-height glyphs
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
                Style::default().fg(pal.dim)
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

        // dB value
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
                        .fg(if focused { anim } else { pal.muted })
                        .add_modifier(if focused {
                            Modifier::BOLD
                        } else {
                            Modifier::empty()
                        }),
                ),
                db_area,
            );
        }

        // freq label
        let lbl_area = Rect::new(col_x, inner.y + bar_h as u16 + 2, bar_w as u16, 1);
        if lbl_area.y < inner.y + inner.height {
            let lbl = BAND_LABELS[i];
            frame.render_widget(
                Paragraph::new(lbl)
                    .alignment(Alignment::Center)
                    .style(Style::default().fg(if focused { anim } else { pal.dim })),
                lbl_area,
            );
        }
    }

    // keybind hint at bottom
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
                .style(Style::default().fg(pal.muted)),
            Rect::new(inner.x, info_y, inner.width, 1),
        );
    }

    let hint_y = inner.y + inner.height.saturating_sub(1);
    if hint_y >= inner.y {
        let hint = "h/l: band  +/-: gain  Enter: on/off  0: reset";
        let hint_area = Rect::new(inner.x, hint_y, inner.width, 1);
        frame.render_widget(
            Paragraph::new(hint)
                .alignment(Alignment::Center)
                .style(Style::default().fg(pal.dim)),
            hint_area,
        );
    }
}

fn draw_overlays(frame: &mut Frame, app: &App, pal: &Palette, anim: Color, size: Rect) {
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
                .style(Style::default().fg(pal.ok))
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
                            Style::default().fg(pal.text).add_modifier(Modifier::BOLD),
                        ),
                    ]))
                } else {
                    ListItem::new(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(o.to_string(), Style::default().fg(pal.muted)),
                    ]))
                }
            })
            .collect();
        let menu = List::new(items).block(
            Block::default()
                .title(Span::styled(
                    " ✦  SONG MENU  (Enter / Esc) ",
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
                            Style::default().fg(pal.warn).add_modifier(Modifier::BOLD),
                        ))
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(pal.warn)),
                )
                .style(Style::default().fg(pal.text)),
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

fn theme_label(theme: Theme) -> &'static str {
    match theme {
        Theme::Dark => "dark",
        Theme::Light => "light",
        Theme::Custom => "custom",
        Theme::Nord => "nord",
        Theme::Gruvbox => "gruvbox",
        Theme::Mono => "mono",
    }
}

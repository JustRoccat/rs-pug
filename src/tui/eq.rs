use super::*;

pub(super) fn draw_eq_panel(frame: &mut Frame, app: &App, pal: &Palette, anim: Color, area: Rect) {
    const BAND_LABELS: [&str; 10] = [
        "32", "64", "125", "250", "500", "1k", "2k", "4k", "8k", "16k",
    ];
    const MAX_DB: f32 = 12.0;
    const EQ_BLOCKS: [&str; 8] = [" ", "▁", "▂", "▃", "▄", "▅", "▆", "█"];
    let inner = area.inner(&ratatui::layout::Margin {
        horizontal: 1,
        vertical: 1,
    });
    let title_color = if app.eq.enabled {
        anim
    } else {
        pal.get_color("muted")
    };
    let block = Block::default()
        .title(Span::styled(
            if app.eq.enabled {
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
    for (i, &gain) in app.eq.bands.iter().enumerate() {
        let col_x = inner.x + (i * bar_w) as u16;
        if col_x >= inner.x + inner.width {
            break;
        }
        let focused = i == app.eq.focus_band && app.options_index == 7;
        let spectrum = pal.spectrum_colors();
        let band_color = if focused {
            anim
        } else if app.eq.enabled {
            spectrum[i % spectrum.len()]
        } else {
            pal.get_color("dim")
        };
        let bg_color = if app.eq.enabled {
            spectrum[(i + 4) % spectrum.len()]
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
        let focus_hint = match app.eq.focus_band {
            0 | 1 => "Sub-bass and body",
            2 | 3 => "Warmth and kick",
            4 | 5 => "Mids and vocals",
            6 | 7 => "Presence and attack",
            _ => "Air and detail",
        };
        let info = format!(
            "Band {} ({})  •  {}",
            app.eq.focus_band + 1,
            BAND_LABELS[app.eq.focus_band],
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

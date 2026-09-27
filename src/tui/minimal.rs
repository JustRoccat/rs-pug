use super::*;
use ratatui_image::Image;

pub(crate) struct MinimalAreas {
    pub title: Rect,
    pub cover: Rect,
    pub spectrum: Rect,
    pub progress: Rect,
}

// Layout rows
pub(crate) fn minimal_layout(area: Rect) -> MinimalAreas {
    let rows = Layout::vertical([
        Constraint::Length(4),
        Constraint::Min(8),
        Constraint::Length(5),
        Constraint::Length(1),
    ])
    .split(area);
    MinimalAreas {
        title: rows[0],
        cover: rows[1],
        spectrum: rows[2],
        progress: rows[3],
    }
}

pub(super) fn draw_minimal(frame: &mut Frame, app: &App, pal: &Palette, anim: Color, area: Rect) {
    if area.width < 20 || area.height < 12 {
        let msg = Paragraph::new("minimal: terminal too small")
            .alignment(Alignment::Center)
            .style(Style::default().fg(pal.get_color("warn")));
        frame.render_widget(msg, area);
        return;
    }

    if app.opt_image_background {
        if let Some(stops) = app.cover_palette {
            paint_minimal_backdrop(frame, area, stops);
        }
    }

    let layout = minimal_layout(area);
    draw_minimal_title(frame, app, pal, anim, layout.title);
    draw_minimal_cover(frame, app, pal, anim, layout.cover);
    draw_minimal_spectrum(frame, app, pal, layout.spectrum);
    draw_minimal_progress(frame, app, pal, anim, layout.progress);
}

// Backdrop row color
fn minimal_backdrop_color(row: u16, height: u16, stops: [[u8; 3]; 3]) -> Color {
    fn lerp(a: u8, b: u8, t: f32) -> f32 {
        a as f32 + (b as f32 - a as f32) * t
    }
    let t = if height <= 1 {
        0.0
    } else {
        row.min(height - 1) as f32 / (height - 1) as f32
    };
    let base = if t < 0.5 {
        let k = t * 2.0;
        [
            lerp(stops[0][0], stops[1][0], k),
            lerp(stops[0][1], stops[1][1], k),
            lerp(stops[0][2], stops[1][2], k),
        ]
    } else {
        let k = (t - 0.5) * 2.0;
        [
            lerp(stops[1][0], stops[2][0], k),
            lerp(stops[1][1], stops[2][1], k),
            lerp(stops[1][2], stops[2][2], k),
        ]
    };
    // Brightness fade
    let env = 0.5 + (0.25 - 0.5) * t;
    Color::Rgb(
        (base[0] * env) as u8,
        (base[1] * env) as u8,
        (base[2] * env) as u8,
    )
}

// Paint backdrop
fn paint_minimal_backdrop(frame: &mut Frame, area: Rect, stops: [[u8; 3]; 3]) {
    let buf = frame.buffer_mut();
    for y in area.y..area.y.saturating_add(area.height) {
        let bg = minimal_backdrop_color(y.saturating_sub(area.y), area.height, stops);
        for x in area.x..area.x.saturating_add(area.width) {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_bg(bg);
            }
        }
    }
}

fn draw_minimal_title(frame: &mut Frame, app: &App, pal: &Palette, anim: Color, area: Rect) {
    let (song_str, artist_str) = if let Some(song) = &app.current_song {
        (song.title.clone(), song.subtitle())
    } else {
        (
            "Nothing playing".to_owned(),
            "Press Shift+Z to leave minimal  ·  / to search when back".to_owned(),
        )
    };
    let state_icon = crate::icons::state(app.player_state, app.opt_icons);
    let mut badges = String::new();
    match app.repeat_mode {
        RepeatMode::Off => {}
        RepeatMode::One => badges.push_str("  ↺¹ ONE"),
        RepeatMode::All => badges.push_str("  ↺∞ ALL"),
    }
    if app.muted {
        badges.push_str("  [MUTED]");
    } else {
        badges.push_str(&format!("  vol {}%", app.volume));
    }
    if (app.playback_speed - 1.0).abs() > 0.001 {
        badges.push_str(&format!("  {:.2}x", app.playback_speed));
    }
    let w = area.width.saturating_sub(4) as usize;
    let song_str = truncate_center(&song_str, w.max(8));
    let mut sub = format!("{state_icon}  {artist_str}{badges}");
    if sub.chars().count() > w && w > 8 {
        sub = truncate_center(&sub, w);
    }
    let lines = vec![
        Line::from(Span::styled(
            song_str,
            Style::default()
                .fg(pal.get_color("text"))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("◦  ", Style::default().fg(pal.get_color("dim"))),
            Span::styled(sub, Style::default().fg(pal.get_color("accent3"))),
        ]),
    ];
    let border = if app.player_state == PlayerState::Playing {
        anim
    } else {
        pal.get_color("dim")
    };
    let w = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border)),
        );
    frame.render_widget(w, area);
}

fn draw_minimal_cover(frame: &mut Frame, app: &App, pal: &Palette, anim: Color, area: Rect) {
    let frame_style = Style::default().fg(if app.player_state == PlayerState::Playing {
        anim
    } else {
        pal.get_color("dim")
    });
    if let Some(proto) = &app.cover_protocol {
        let fitted = proto.0.size();
        let (iw, ih) = (fitted.width, fitted.height);
        if iw > 0 && ih > 0 {
            let x = area.x + (area.width.saturating_sub(iw)) / 2;
            let y = area.y + (area.height.saturating_sub(ih)) / 2;
            let rect = Rect::new(x, y, iw.min(area.width), ih.min(area.height));
            let frame_rect = Rect::new(
                rect.x.saturating_sub(1),
                rect.y.saturating_sub(1),
                rect.width.saturating_add(2),
                rect.height.saturating_add(2),
            )
            .intersection(area);
            frame.render_widget(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(frame_style),
                frame_rect,
            );
            frame.render_widget(Image::new(&proto.0).allow_clipping(true), rect);
            return;
        }
    }
    let body: Vec<Line> = if app.current_song.is_none() {
        placeholder_lines(pal, "Nothing playing", "", "")
    } else if app.cover_loading || app.cover_bitmap.is_some() {
        placeholder_lines(pal, "Loading cover…", "", "")
    } else {
        placeholder_lines(pal, "no cover", "", "")
    };
    // Center placeholder
    let inner_h = area.height.saturating_sub(2) as usize;
    let body = if !body.is_empty() && body.len() < inner_h {
        let pad = (inner_h - body.len()) / 2;
        let mut padded = vec![Line::from(""); pad];
        padded.extend(body);
        padded
    } else {
        body
    };
    let w = Paragraph::new(body).alignment(Alignment::Center);
    frame.render_widget(w, area);
}

fn placeholder_lines(pal: &Palette, head: &str, sub1: &str, sub2: &str) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from(Span::styled(
        head.to_owned(),
        Style::default()
            .fg(pal.get_color("muted"))
            .add_modifier(Modifier::BOLD),
    ))];
    for sub in [sub1, sub2] {
        if !sub.is_empty() {
            lines.push(Line::from(Span::styled(
                sub.to_owned(),
                Style::default().fg(pal.get_color("dim")),
            )));
        }
    }
    lines
}

fn draw_minimal_spectrum(frame: &mut Frame, app: &App, pal: &Palette, area: Rect) {
    let w = area.width as usize;
    let h = area.height as usize;
    if w == 0 || h == 0 {
        return;
    }
    let levels = minimal_levels(app, w);
    let colors = pal.spectrum_colors();
    let nc = colors.len().max(1);
    let tick = app.anim_tick as usize;
    let mut rows: Vec<Line> = Vec::with_capacity(h);
    for row in 0..h {
        let threshold = (h - row) as f64 / h as f64;
        let mut spans: Vec<Span<'static>> = Vec::with_capacity(w);
        for (col, lvl) in levels.iter().enumerate() {
            let idx = (col * nc / w.max(1) + tick / 15) % nc;
            if *lvl >= threshold {
                spans.push(Span::styled("█", Style::default().fg(colors[idx])));
            } else if row == h - 1 {
                spans.push(Span::styled(
                    "▁",
                    Style::default().fg(pal.get_color("dim")),
                ));
            } else {
                spans.push(Span::raw(" "));
            }
        }
        rows.push(Line::from(spans));
    }
    frame.render_widget(Paragraph::new(rows), area);
}

// Spectrum levels
fn minimal_levels(app: &App, width: usize) -> Vec<f64> {
    if width == 0 {
        return vec![];
    }
    if app.show_fft {
        if let Some(state) = app.fft_state.as_ref() {
            if let Ok(s) = state.lock() {
                if s.running && !s.bands.is_empty() {
                    let bands = s.bands.clone();
                    return (0..width)
                        .map(|col| {
                            let idx =
                                (col * bands.len() / width).min(bands.len().saturating_sub(1));
                            bands[idx].clamp(0.0, 1.0)
                        })
                        .collect();
                }
            }
        }
    }
    let playing = app.player_state == PlayerState::Playing;
    let vol = (app.volume as f64 / 100.0).clamp(0.2, 1.0);
    let tick = app.anim_tick as f64;
    (0..width)
        .map(|col| {
            let c = col as f64;
            let w1 = (c * 0.10 + tick * 0.03).sin() * 0.35;
            let w2 = (c * 0.35 + tick * 0.09).sin() * 0.25;
            let w3 = (c * 1.10 + tick * 0.22).sin() * 0.15;
            let combined = (w1 + w2 + w3 + 1.0) / 2.0;
            if playing {
                (combined * vol).clamp(0.02, 1.0)
            } else {
                (combined * 0.25).clamp(0.02, 0.4)
            }
        })
        .collect()
}

fn draw_minimal_progress(frame: &mut Frame, app: &App, pal: &Palette, anim: Color, area: Rect) {
    let ratio = if app.playback_duration > 0.0 {
        (app.playback_pos / app.playback_duration).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let label = if app.playback_duration > 0.0 {
        format!(
            "  {} ─ {} ({:.0}%)",
            format_time(app.playback_pos),
            format_time(app.playback_duration),
            ratio * 100.0,
        )
    } else if app.current_song.is_some() {
        format!("  {} ─ loading...", format_time(app.playback_pos))
    } else {
        "  ─  no track loaded".to_owned()
    };
    let gauge_color = match app.player_state {
        PlayerState::Playing => anim,
        PlayerState::Paused => pal.get_color("warn"),
        _ => pal.get_color("dim"),
    };
    let gauge = Gauge::default()
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

fn truncate_center(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_owned();
    }
    if max <= 1 {
        return "…".to_owned();
    }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_app() -> App {
        let file = tempfile::NamedTempFile::new().expect("tempfile");
        let db_path = file.path().to_path_buf();
        std::mem::forget(file);
        let storage = crate::storage::Storage::open_at(db_path).expect("open test db");
        App::new(storage)
    }

    #[test]
    fn minimal_layout_stacks_cover_above_small_spectrum() {
        let area = Rect::new(0, 0, 100, 40);
        let l = minimal_layout(area);
        assert!(l.cover.y > l.title.y, "cover below title");
        assert!(l.spectrum.y > l.cover.y, "spectrum below cover");
        assert!(l.progress.y > l.spectrum.y, "progress at bottom");
        assert_eq!(l.spectrum.height, 5, "spectrum is a small bare strip");
        assert_eq!(l.progress.height, 1, "progress is a single line");
        assert!(
            l.cover.height > l.spectrum.height * 2,
            "cover dominates: {} vs {}",
            l.cover.height,
            l.spectrum.height
        );
    }

    #[test]
    fn minimal_levels_match_width_and_range() {
        let mut app = test_app();
        app.player_state = PlayerState::Playing;
        app.anim_tick = 42;
        let levels = minimal_levels(&app, 24);
        assert_eq!(levels.len(), 24);
        assert!(levels.iter().all(|v| (0.0..=1.0).contains(v)));
    }

    #[test]
    fn minimal_backdrop_paints_buffer_cells() {
        use ratatui::{backend::TestBackend, Terminal};
        use ratatui::style::Color;

        fn tinted_cells(palette: Option<[[u8; 3]; 3]>, background: bool) -> usize {
            let mut app = test_app();
            app.opt_image_background = background;
            app.cover_palette = palette;
            let backend = TestBackend::new(100, 40);
            let mut terminal = Terminal::new(backend).unwrap();
            terminal
                .draw(|frame| {
                    let pal = crate::config::load_palette(&crate::config::Theme::Dark);
                    let anim = pal.get_color("primary");
                    draw_minimal(frame, &app, &pal, anim, frame.area());
                })
                .unwrap();
            terminal
                .backend()
                .buffer()
                .content()
                .iter()
                .filter(|c| !matches!(c.bg, Color::Reset))
                .count()
        }
        assert!(
            tinted_cells(Some([[200, 0, 0], [0, 200, 0], [0, 0, 200]]), true) > 3000,
            "backdrop tints the full area"
        );
        assert_eq!(
            tinted_cells(Some([[200, 0, 0], [0, 200, 0], [0, 0, 200]]), false),
            0,
            "toggle off paints nothing"
        );
        assert_eq!(
            tinted_cells(None, true),
            0,
            "no accent paints nothing"
        );
    }

    #[test]
    fn minimal_backdrop_blends_three_stops_downwards() {
        use ratatui::style::Color;
        let rgb = |c: Color| match c {
            Color::Rgb(r, g, b) => (r, g, b),
            other => panic!("expected rgb bg, got {other:?}"),
        };
        let stops = [[200, 0, 0], [0, 200, 0], [0, 0, 200]];
        let top = rgb(minimal_backdrop_color(0, 10, stops));
        let bottom = rgb(minimal_backdrop_color(9, 10, stops));
        assert!(top.0 > 60 && top.1 < 30 && top.2 < 30, "red on top: {top:?}");
        assert!(bottom.2 > 20 && bottom.0 < 20, "blue at bottom: {bottom:?}");
        let mid = rgb(minimal_backdrop_color(5, 10, stops));
        assert!(mid.1 >= mid.0 && mid.1 >= mid.2, "green mid: {mid:?}");
        let _ = rgb(minimal_backdrop_color(0, 1, stops));
        let _ = rgb(minimal_backdrop_color(0, 0, stops));
    }

    #[test]
    fn minimal_renders_pixelated_cover_cells() {
        use ratatui::{backend::TestBackend, Terminal};
        use ratatui_image::{picker::Picker, Resize};

        let mut app = test_app();
        app.current_song = Some(Song {
            id: "x".to_owned(),
            title: "Test".to_owned(),
            webpage_url: "u".to_owned(),
            uploader: Some("A".to_owned()),
            duration: Some(60.0),
        });
        let mut raw = image::RgbaImage::new(512, 512);
        for (x, y, px) in raw.enumerate_pixels_mut() {
            *px = image::Rgba([(x / 2) as u8, (y / 2) as u8, 128, 255]);
        }
        let bitmap = image::DynamicImage::ImageRgba8(raw);
        let picker = Picker::halfblocks();
        let proto = picker
            .new_protocol(
                bitmap.clone(),
                ratatui::layout::Size::new(40, 20),
                Resize::Fit(None),
            )
            .expect("halfblocks protocol");
        app.cover_bitmap = Some(bitmap);
        app.cover_protocol = Some(crate::model::CoverProtocol(proto));

        let backend = TestBackend::new(80, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let pal = crate::config::load_palette(&crate::config::Theme::Dark);
                let anim = pal.get_color("primary");
                draw_minimal(frame, &app, &pal, anim, frame.area());
            })
            .unwrap();
        let buf = terminal.backend().buffer().clone();
        let halfblocks = buf
            .content()
            .iter()
            .filter(|c| c.symbol() == "▀" || c.symbol() == "▄")
            .count();
        assert!(halfblocks > 100, "cover renders as halfblock pixels");
        let text: String = buf
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();
        for banned in ["COVER", "SPECTRUM", "PROGRESS"] {
            assert!(!text.contains(banned), "no {banned} label");
        }
    }

    #[test]
    fn cover_frame_touches_art_evenly() {
        use ratatui::{backend::TestBackend, Terminal};
        use ratatui_image::{picker::Picker, Resize};

        let mut app = test_app();
        app.current_song = Some(Song {
            id: "x".to_owned(),
            title: "Test".to_owned(),
            webpage_url: "u".to_owned(),
            uploader: Some("A".to_owned()),
            duration: Some(60.0),
        });
        let mut raw = image::RgbaImage::new(512, 512);
        for (x, y, px) in raw.enumerate_pixels_mut() {
            *px = image::Rgba([
                128 + ((x * 7 + y * 13) % 128) as u8,
                128 + ((x * 3 + y * 11) % 128) as u8,
                200,
                255,
            ]);
        }
        let bitmap = image::DynamicImage::ImageRgba8(raw);
        let picker = Picker::halfblocks();
        let proto = picker
            .new_protocol(
                bitmap.clone(),
                ratatui::layout::Size::new(60, 20),
                Resize::Fit(None),
            )
            .expect("halfblocks protocol");
        app.cover_bitmap = Some(bitmap);
        app.cover_protocol = Some(crate::model::CoverProtocol(proto));

        let backend = TestBackend::new(100, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let pal = crate::config::load_palette(&crate::config::Theme::Dark);
                let anim = pal.get_color("primary");
                draw_minimal(frame, &app, &pal, anim, frame.area());
            })
            .unwrap();
        let buf = terminal.backend().buffer();
        let is_art = |x: u16, y: u16| {
            buf.cell((x, y))
                .map(|c| c.symbol() == "▀" || c.symbol() == "▄")
                .unwrap_or(false)
        };
        let (mut x0, mut y0, mut x1, mut y1) = (u16::MAX, u16::MAX, 0, 0);
        for y in 0..40 {
            for x in 0..100 {
                if is_art(x, y) {
                    x0 = x0.min(x);
                    y0 = y0.min(y);
                    x1 = x1.max(x);
                    y1 = y1.max(y);
                }
            }
        }
        assert!(x1 > x0 + 10 && y1 > y0 + 5, "art rendered sizable");
        for x in x0..=x1 {
            assert_eq!(
                buf.cell((x, y0 - 1)).map(|c| c.symbol()),
                Some("─"),
                "top border touches art at x={x}"
            );
            assert_eq!(
                buf.cell((x, y1 + 1)).map(|c| c.symbol()),
                Some("─"),
                "bottom border touches art at x={x}"
            );
        }
        for y in y0..=y1 {
            assert_eq!(
                buf.cell((x0 - 1, y)).map(|c| c.symbol()),
                Some("│"),
                "left border touches art at y={y}"
            );
            assert_eq!(
                buf.cell((x1 + 1, y)).map(|c| c.symbol()),
                Some("│"),
                "right border touches art at y={y}"
            );
        }
    }
}

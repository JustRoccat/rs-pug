use super::*;

pub(super) fn draw_now_playing(frame: &mut Frame, app: &App, pal: &Palette, anim: Color, area: Rect) {
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
    let speed_badge = if (app.playback_speed - 1.0).abs() > 0.001 {
        format!("  {:.2}x", app.playback_speed)
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
        Span::styled(
            speed_badge,
            Style::default()
                .fg(pal.get_color("accent3"))
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
    let spectrum_rows = area.height.saturating_sub(4).max(1) as usize;
    let mut rows = vec![line1, line2];
    rows.extend((0..spectrum_rows).map(|_| {
        let mut spec = vec![Span::styled("  ".to_string(), Style::default())];
        spec.extend(spectrum_spans(app, pal, spec_w));
        Line::from(spec)
    }));
    let border_color = if playing { anim } else { pal.get_color("dim") };
    let widget = Paragraph::new(rows).block(
        Block::default()
            .title(Span::styled(
                "   NOW PLAYING ",
                Style::default()
                    .fg(border_color)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color)),
    );
    frame.render_widget(widget, area);
}
pub(super) fn draw_progress(frame: &mut Frame, app: &App, pal: &Palette, anim: Color, area: Rect) {
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

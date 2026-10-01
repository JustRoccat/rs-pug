use super::*;

pub(super) fn draw_search(frame: &mut Frame, app: &App, pal: &Palette, area: Rect) {
    let active_query = if app.active_tab == Tab::Albums {
        app.albums.search_query.as_str()
    } else {
        app.search.query.as_str()
    };
    let on = app.opt_icons;
    let (border_color, title_str): (_, String) = if app.search_mode {
        (
            pal.get_color("info"),
            if on {
                format!(" {}  SEARCHING — type and press Enter ", crate::icons::nf::KEYBOARD)
            } else {
                " SEARCHING — type and press Enter ".to_owned()
            },
        )
    } else {
        (
            pal.get_color("dim"),
            if app.active_tab == Tab::Local {
                if on {
                    format!(" {}  SEARCH LOCAL — press / to start ", crate::icons::nf::SEARCH)
                } else {
                    " SEARCH LOCAL — press / to start ".to_owned()
                }
            } else if on {
                format!(" {}  SEARCH — press / to start ", crate::icons::nf::SEARCH)
            } else {
                " SEARCH — press / to start ".to_owned()
            },
        )
    };
    let content = if active_query.is_empty() && !app.search_mode {
        let prompt = if app.active_tab == Tab::Local {
            "search local files...".to_string()
        } else {
            format!(
                "search {}...",
                search_source_label(&app.opt_source, &app.opt_custom_sources)
            )
        };
        Line::from(Span::styled(
            prompt,
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

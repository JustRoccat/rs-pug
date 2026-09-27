use super::*;

// Status row
pub(super) fn has_status_content(app: &App) -> bool {
    app.ui_layout.show_statusbar
        && (!app.plugin_ui.warnings.is_empty()
            || !app.plugin_ui.inject.statusbar_extra.is_empty())
}

pub(super) fn draw_help(frame: &mut Frame, app: &App, pal: &Palette, area: Rect) {
    let gap = || -> Span<'static> { Span::raw("  ") };
    let mut spans = Vec::new();
    if let Some(warning) = app.plugin_ui.warnings.back() {
        spans.push(Span::styled(
            crate::icons::warn(app.opt_icons),
            Style::default()
                .fg(pal.get_color("warn"))
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw(" "));
        spans.push(Span::styled(
            warning.as_str(),
            Style::default().fg(pal.get_color("warn")),
        ));
    }
    for item in &app.plugin_ui.inject.statusbar_extra {
        if !spans.is_empty() {
            spans.push(gap());
        }
        spans.extend(panel_item_line(item, pal).spans);
    }
    let widget = Paragraph::new(Line::from(spans)).block(
        Block::default()
            .title(Span::styled(
                " STATUS ",
                Style::default().fg(pal.get_color("dim")),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(pal.get_color("dim"))),
    );
    frame.render_widget(widget, area);
}
pub(super) fn draw_custom_sections(
    frame: &mut Frame,
    app: &App,
    pal: &Palette,
    anim: Color,
    position: &str,
    area: Rect,
) {
    if !app.plugin_ui.allow_lua_ui_changes {
        return;
    }
    let sections: Vec<_> = app
        .plugin_ui
        .custom_sections
        .iter()
        .filter(|section| {
            section.position == position
                && !app
                    .plugin_ui
                    .hidden_sections
                    .iter()
                    .any(|id| id == &section.id)
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
            .plugin_ui
            .section_items
            .get(&section.id)
            .map(|items| {
                items
                    .iter()
                    .map(|item| panel_item_line(item, pal))
                    .collect()
            })
            .unwrap_or_default();
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

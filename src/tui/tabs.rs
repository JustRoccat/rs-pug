use super::*;

pub(super) fn draw_tabs(frame: &mut Frame, app: &App, pal: &Palette, anim: Color, _anim2: Color, area: Rect) {
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
                    "   R S - P U G   ",
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
pub(super) fn draw_tabs_vertical(frame: &mut Frame, app: &App, pal: &Palette, anim: Color, area: Rect) {
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
                        shortcut,
                        Style::default()
                            .fg(pal.get_color("warn"))
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        icon.as_str(),
                        Style::default().fg(anim).add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        label.as_str(),
                        Style::default().fg(anim).add_modifier(Modifier::BOLD),
                    ),
                ]))
            } else {
                ListItem::new(Line::from(vec![
                    Span::styled(shortcut, Style::default().fg(pal.get_color("dim"))),
                    Span::styled(icon.as_str(), Style::default().fg(pal.get_color("dim"))),
                    Span::raw(" "),
                    Span::styled(label.as_str(), Style::default().fg(pal.get_color("muted"))),
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

use super::*;

pub(super) fn draw_tabs(frame: &mut Frame, app: &App, pal: &Palette, anim: Color, _anim2: Color, area: Rect) {
    let (defs, active) = tab_defs_and_active(app);
    let on = app.opt_icons;
    let tab_lines: Vec<Line> = defs
        .iter()
        .enumerate()
        .map(|(i, (icon, label))| {
            if i == active {
                Line::from(crate::icons::tab_row(
                    icon,
                    label,
                    on,
                    Style::default().fg(anim).add_modifier(Modifier::BOLD),
                    Style::default().fg(anim).add_modifier(Modifier::BOLD),
                ))
            } else {
                Line::from(crate::icons::tab_row(
                    icon,
                    label,
                    on,
                    Style::default().fg(pal.get_color("dim")),
                    Style::default().fg(pal.get_color("muted")),
                ))
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
    let on = app.opt_icons;
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
                let mut row = vec![Span::styled(
                    shortcut,
                    Style::default()
                        .fg(pal.get_color("warn"))
                        .add_modifier(Modifier::BOLD),
                )];
                row.extend(crate::icons::tab_row(
                    icon,
                    label,
                    on,
                    Style::default().fg(anim).add_modifier(Modifier::BOLD),
                    Style::default().fg(anim).add_modifier(Modifier::BOLD),
                ));
                ListItem::new(Line::from(row))
            } else {
                let mut row = vec![Span::styled(
                    shortcut,
                    Style::default().fg(pal.get_color("dim")),
                )];
                row.extend(crate::icons::tab_row(
                    icon,
                    label,
                    on,
                    Style::default().fg(pal.get_color("dim")),
                    Style::default().fg(pal.get_color("muted")),
                ));
                ListItem::new(Line::from(row))
            }
        })
        .collect();
    let tabs_title = if app.opt_icons {
        format!(" {}  TABS ", crate::icons::nf::MUSIC)
    } else {
        " TABS ".to_owned()
    };
    let list = List::new(items).block(
        Block::default()
            .title(Span::styled(
                tabs_title,
                Style::default().fg(anim).add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(anim)),
    );
    frame.render_widget(list, area);
}

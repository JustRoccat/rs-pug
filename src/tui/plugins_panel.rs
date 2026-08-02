use super::*;

pub(super) fn draw_plugin_panels(frame: &mut Frame, app: &App, pal: &Palette, anim: Color, size: Rect) {
    if app.plugin_ui.panels.is_empty() {
        return;
    }
    let mut y = size.y + 1;
    for panel in app
        .plugin_ui
        .panels
        .iter()
        .filter(|p| p.target == Some(crate::plugins::PluginPanelTarget::Overlay))
    {
        let item_lines: Vec<Line> = if panel.items.is_empty() {
            panel
                .lines
                .iter()
                .map(|line| {
                    Line::from(Span::styled(
                        line.as_str(),
                        Style::default().fg(pal.get_color("text")),
                    ))
                })
                .collect()
        } else {
            panel
                .items
                .iter()
                .map(|item| panel_item_line(item, pal))
                .collect()
        };
        let lines = item_lines.len().clamp(1, 10) as u16;
        let content_width = item_lines
            .iter()
            .map(|line| line.width() as u16)
            .max()
            .unwrap_or(0)
            .max(panel.title.chars().count() as u16)
            .min(size.width.saturating_sub(8));
        let w = (content_width + 4)
            .max(20)
            .min(size.width.saturating_sub(2));
        let h = (lines + 2).min(size.height.saturating_sub(y.saturating_sub(size.y) + 1));
        if h < 3 {
            break;
        }
        let x = size.x + size.width.saturating_sub(w + 1);
        let area = Rect::new(x, y, w, h);
        let rows: Vec<Line> = item_lines.into_iter().take(lines as usize).collect();
        frame.render_widget(Clear, area);
        frame.render_widget(
            Paragraph::new(rows).block(
                Block::default()
                    .title(Span::styled(
                        format!(" {} ", panel.title),
                        Style::default().fg(anim).add_modifier(Modifier::BOLD),
                    ))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(anim)),
            ),
            area,
        );
        y = y.saturating_add(h + 1);
        if y >= size.y + size.height.saturating_sub(3) {
            break;
        }
    }
}

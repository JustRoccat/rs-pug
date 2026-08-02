use crate::{
    config::{Palette, Theme},
    model::{eq_preset_name, App, Focus, PlayerState, RepeatMode, Song, Tab},
    plugins::PluginPanelItem,
    utils::natural_compare,
};
use ratatui::{
    prelude::*,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, List, ListItem, ListState, Paragraph, Tabs},
};
mod eq;
mod help;
mod library;
mod overlays;
mod playback;
mod plugins_panel;
mod queue;
mod search;
mod tabs;
use help::{draw_custom_sections, draw_help};
use library::draw_content;
use overlays::draw_overlays;
use playback::{draw_now_playing, draw_progress};
use search::draw_search;
use tabs::{draw_tabs, draw_tabs_vertical};
fn palette(theme: &Theme) -> Palette {
    crate::config::load_palette(theme)
}
fn search_source_label(source: &crate::config::SearchSource) -> String {
    match source {
        crate::config::SearchSource::YouTube => "YouTube".to_string(),
        crate::config::SearchSource::SoundCloud => "SoundCloud".to_string(),
    }
}
const VOLT_BLOCKS: [&str; 8] = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];
fn spectrum_spans(app: &App, pal: &Palette, width: usize) -> Vec<Span<'static>> {
    if width == 0 {
        return vec![];
    }
    let playing = app.player_state == PlayerState::Playing;
    let vol_factor = (app.volume as f32 / 100.0).clamp(0.2, 1.0);
    let tick = app.anim_tick as f64;
    let colors = pal.spectrum_colors();
    let fft_data = if app.show_fft {
        app.fft_state.as_ref().and_then(|s| {
            if let Ok(state) = s.lock() {
                if state.running {
                    Some(state.bands.clone())
                } else {
                    None
                }
            } else {
                None
            }
        })
    } else {
        None
    };
    (0..width)
        .map(|col| {
            let level = if let Some(ref bands) = fft_data {
                let band_idx = (col * bands.len() / width).min(bands.len().saturating_sub(1));
                let val = bands[band_idx] * 7.0;
                val.clamp(0.0, 7.0) as usize
            } else {
                let c = col as f64;
                let wave1 = (c * 0.1 + tick * 0.02).sin() * 0.3;
                let wave2 = (c * 0.4 + tick * 0.08).sin() * 0.2;
                let wave3 = (c * 1.2 + tick * 0.2).sin() * 0.1;
                let combined = (wave1 + wave2 + wave3 + 1.0) / 2.0;
                if playing {
                    (combined * 6.0 * vol_factor as f64).clamp(1.0, 7.0) as usize
                } else {
                    (combined * 2.0 * 0.3).clamp(0.0, 3.0) as usize
                }
            };
            let nc = colors.len();
            let idx = (col * nc / width + tick as usize / 15) % nc;
            Span::styled(
                VOLT_BLOCKS[level.clamp(0, 7)],
                Style::default().fg(colors[idx]),
            )
        })
        .collect()
}
pub fn draw(frame: &mut Frame, app: &App) {
    let pal = palette(&app.theme);
    let anim = pal.get_color("primary");
    let anim2 = pal.get_color("accent2");
    let size = frame.size();
    let tab_position = app.ui_layout.tab_bar_position.as_str();
    let tabs_width = app
        .ui_layout
        .tabs_width
        .min(size.width.saturating_sub(20))
        .max(1)
        .min(size.width);
    let (main_area, tab_area, vertical_tabs) = match tab_position {
        "left" => {
            let cols = Layout::horizontal([Constraint::Length(tabs_width), Constraint::Min(20)])
                .split(size);
            (cols[1], cols[0], true)
        }
        "right" => {
            let cols = Layout::horizontal([Constraint::Min(20), Constraint::Length(tabs_width)])
                .split(size);
            (cols[0], cols[1], true)
        }
        "bottom" => {
            let rows = Layout::vertical([Constraint::Min(8), Constraint::Length(3)]).split(size);
            (rows[0], rows[1], false)
        }
        _ => {
            let rows = Layout::vertical([Constraint::Length(3), Constraint::Min(8)]).split(size);
            (rows[1], rows[0], false)
        }
    };
    if vertical_tabs {
        draw_tabs_vertical(frame, app, &pal, anim, tab_area);
    } else {
        draw_tabs(frame, app, &pal, anim, anim2, tab_area);
    }
    let above_height = custom_sections_height(app, "above_player");
    let below_height = custom_sections_height(app, "below_player");
    let mut constraints = Vec::new();
    constraints.push(Constraint::Length(3));
    if above_height > 0 {
        constraints.push(Constraint::Length(above_height));
    }
    constraints.push(Constraint::Min(8));
    if app.ui_layout.visualizer_height > 0 {
        constraints.push(Constraint::Length(app.ui_layout.visualizer_height));
    }
    if below_height > 0 {
        constraints.push(Constraint::Length(below_height));
    }
    if app.ui_layout.show_progress_bar {
        constraints.push(Constraint::Length(3));
    }
    if app.ui_layout.show_statusbar || app.ui_layout.show_keybind_hints {
        constraints.push(Constraint::Length(3));
    }
    let vertical = Layout::vertical(constraints).split(main_area);
    let mut row = 0;
    draw_search(frame, app, &pal, vertical[row]);
    row += 1;
    if above_height > 0 {
        draw_custom_sections(frame, app, &pal, anim, "above_player", vertical[row]);
        row += 1;
    }
    draw_content(frame, app, &pal, anim, vertical[row]);
    draw_custom_sections(frame, app, &pal, anim, "left", vertical[row]);
    draw_custom_sections(frame, app, &pal, anim, "right", vertical[row]);
    row += 1;
    if app.ui_layout.visualizer_height > 0 {
        draw_now_playing(frame, app, &pal, anim, vertical[row]);
        row += 1;
    }
    if below_height > 0 {
        draw_custom_sections(frame, app, &pal, anim, "below_player", vertical[row]);
        row += 1;
    }
    if app.ui_layout.show_progress_bar {
        draw_progress(frame, app, &pal, anim, vertical[row]);
        row += 1;
    }
    if app.ui_layout.show_statusbar || app.ui_layout.show_keybind_hints {
        draw_help(frame, app, &pal, vertical[row]);
    }
    draw_overlays(frame, app, &pal, anim, size);
}
fn custom_sections_height(app: &App, position: &str) -> u16 {
    if !app.plugin_ui.allow_lua_ui_changes {
        return 0;
    }
    app.plugin_ui
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
        .map(|section| section.height.unwrap_or(3))
        .max()
        .unwrap_or(0)
}
fn tab_defs_and_active(app: &App) -> (Vec<(String, String)>, usize) {
    let mut defs: Vec<(String, String)> = app
        .main_tabs
        .iter()
        .map(|tab| (tab.icon.clone(), tab.title.clone()))
        .collect();
    for t in &app.plugin_ui.tabs {
        defs.push((
            t.icon.clone().unwrap_or_else(|| "◌".to_string()),
            t.title.to_uppercase(),
        ));
    }
    let active = if let Some(active_id) = &app.plugin_ui.active_tab {
        app.plugin_ui
            .tabs
            .iter()
            .position(|t| &t.id == active_id)
            .map(|i| i + app.main_tabs.len())
            .unwrap_or_else(|| app.active_tab_index().saturating_sub(1))
    } else {
        app.active_tab_index().saturating_sub(1)
    };
    (defs, active)
}
fn panel_item_line(item: &PluginPanelItem, pal: &Palette) -> Line<'static> {
    match item {
        PluginPanelItem::Text { text } => Line::from(Span::styled(
            text.clone(),
            Style::default().fg(pal.get_color("text")),
        )),
        PluginPanelItem::Info { text } => Line::from(Span::styled(
            text.clone(),
            Style::default().fg(pal.get_color("info")),
        )),
        PluginPanelItem::Option { key, value } => Line::from(vec![
            Span::styled(
                format!("{}: ", key),
                Style::default().fg(pal.get_color("warn")),
            ),
            Span::styled(value.clone(), Style::default().fg(pal.get_color("text"))),
        ]),
        PluginPanelItem::Stat { label, value } => Line::from(vec![
            Span::styled(
                format!("{} ", label),
                Style::default().fg(pal.get_color("muted")),
            ),
            Span::styled(value.clone(), Style::default().fg(pal.get_color("ok"))),
        ]),
        PluginPanelItem::Separator => Line::from(Span::styled(
            "─".repeat(24),
            Style::default().fg(pal.get_color("dim")),
        )),
        PluginPanelItem::Header { text } => Line::from(Span::styled(
            text.clone(),
            Style::default()
                .fg(pal.get_color("accent2"))
                .add_modifier(Modifier::BOLD),
        )),
        PluginPanelItem::Keybind { key, action } => Line::from(vec![
            Span::styled(
                key.clone(),
                Style::default()
                    .fg(pal.get_color("warn"))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" → ", Style::default().fg(pal.get_color("dim"))),
            Span::styled(action.clone(), Style::default().fg(pal.get_color("text"))),
        ]),
        PluginPanelItem::Progress { label, percent } => {
            let pct = percent.clamp(0.0, 100.0).round() as usize;
            let filled = pct / 10;
            let bar = format!("{}{}", "█".repeat(filled), "░".repeat(10 - filled));
            Line::from(vec![
                Span::styled(
                    label.clone().unwrap_or_else(|| "progress".to_owned()),
                    Style::default().fg(pal.get_color("muted")),
                ),
                Span::raw(" "),
                Span::styled(bar, Style::default().fg(pal.get_color("ok"))),
                Span::styled(
                    format!(" {pct}%"),
                    Style::default().fg(pal.get_color("text")),
                ),
            ])
        }
    }
}
fn plugin_panel_lines(panel: &crate::plugins::PluginPanel, pal: &Palette) -> Vec<Line<'static>> {
    if !panel.items.is_empty() {
        panel
            .items
            .iter()
            .map(|item| panel_item_line(item, pal))
            .collect()
    } else {
        panel
            .lines
            .iter()
            .map(|line| {
                Line::from(Span::styled(
                    line.clone(),
                    Style::default().fg(pal.get_color("text")),
                ))
            })
            .collect()
    }
}
fn dim_item(text: &'static str, pal: &Palette) -> ListItem<'static> {
    ListItem::new(Span::styled(
        text,
        Style::default().fg(pal.get_color("muted")),
    ))
}
fn tab_key_hint(app: &App) -> String {
    let count = (app.main_tabs.len() + app.plugin_ui.tabs.len()).clamp(1, 8);
    format!("1-{count}")
}
fn key_span(text: impl Into<String>, pal: &Palette) -> Span<'static> {
    Span::styled(
        text.into(),
        Style::default()
            .fg(pal.get_color("warn"))
            .add_modifier(Modifier::BOLD),
    )
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
fn theme_label(theme: &Theme) -> String {
    match theme {
        Theme::Dark => "dark".to_string(),
        Theme::Light => "light".to_string(),
        Theme::Nord => "nord".to_string(),
        Theme::Gruvbox => "gruvbox".to_string(),
        Theme::Mono => "mono".to_string(),
        Theme::Custom(name) => name.clone(),
    }
}

pub mod nf {
    pub const MUSIC: &str = "\u{f001}";
    pub const GRID: &str = "\u{f009}";
    pub const LIST: &str = "\u{f0ca}";
    pub const FOLDER: &str = "\u{f07b}";
    pub const GEAR: &str = "\u{f013}";
    pub const PLAY: &str = "\u{f04b}";
    pub const PAUSE: &str = "\u{f04c}";
    pub const STOP: &str = "\u{f04d}";
    pub const SPINNER: &str = "\u{f110}";
    pub const WARN: &str = "\u{f071}";
    pub const KEYBOARD: &str = "\u{f11c}";
    pub const SEARCH: &str = "\u{f002}";
}

// Tab icon or nothing
pub fn tab(icon: &str, enabled: bool) -> String {
    if enabled { icon.to_owned() } else { String::new() }
}

// Icon row or title
pub fn tab_row(
    icon: &str,
    label: &str,
    enabled: bool,
    icon_style: ratatui::style::Style,
    label_style: ratatui::style::Style,
) -> Vec<ratatui::text::Span<'static>> {
    use ratatui::text::Span;
    let mut row = Vec::with_capacity(3);
    if enabled && !icon.is_empty() {
        row.push(Span::styled(icon.to_owned(), icon_style));
        row.push(Span::raw(" "));
    }
    row.push(Span::styled(label.to_owned(), label_style));
    row
}

// Player state icon
pub fn state(state: crate::model::PlayerState, enabled: bool) -> &'static str {
    use crate::model::PlayerState;
    if enabled {
        match state {
            PlayerState::Playing => nf::PLAY,
            PlayerState::Paused => nf::PAUSE,
            PlayerState::Searching => nf::SPINNER,
            PlayerState::Idle => nf::STOP,
        }
    } else {
        match state {
            PlayerState::Playing => ">",
            PlayerState::Paused => "||",
            PlayerState::Searching => "...",
            PlayerState::Idle => "-",
        }
    }
}

// Titled section
pub fn section(title: &str, nf_icon: &str, enabled: bool) -> String {
    if enabled {
        format!(" {nf_icon}  {title} ")
    } else {
        format!(" {title} ")
    }
}

// Song bullet
pub fn song_bullet(enabled: bool) -> &'static str {
    if enabled { nf::MUSIC } else { " " }
}

// Warning marker
pub fn warn(enabled: bool) -> &'static str {
    if enabled { nf::WARN } else { "!" }
}

// Gear or nothing
pub fn gear(enabled: bool) -> &'static str {
    if enabled { nf::GEAR } else { "" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tab_icon_hides_without_nerdfonts() {
        assert_eq!(tab(nf::MUSIC, true), nf::MUSIC);
        assert_eq!(tab(nf::MUSIC, false), "");
        assert_eq!(tab("◌", false), "");
    }

    #[test]
    fn state_icons_have_ascii_fallbacks() {
        use crate::model::PlayerState;
        assert_eq!(state(PlayerState::Playing, true), nf::PLAY);
        assert_eq!(state(PlayerState::Playing, false), ">");
        assert_eq!(state(PlayerState::Paused, false), "||");
        assert_eq!(state(PlayerState::Searching, false), "...");
        assert_eq!(state(PlayerState::Idle, false), "-");
        for s in [
            PlayerState::Playing,
            PlayerState::Paused,
            PlayerState::Searching,
            PlayerState::Idle,
        ] {
            assert!(state(s, false).is_ascii());
        }
    }

    #[test]
    fn bullets_and_markers_degrade_cleanly() {
        assert_eq!(song_bullet(true), nf::MUSIC);
        assert_eq!(song_bullet(false), " ");
        assert_eq!(warn(true), nf::WARN);
        assert_eq!(warn(false), "!");
        assert_eq!(gear(true), nf::GEAR);
        assert_eq!(gear(false), "");
    }

    #[test]
    fn tab_row_skips_dangling_space() {
        use ratatui::style::Style;
        let on = tab_row("X", "TABS", true, Style::default(), Style::default());
        assert_eq!(on.len(), 3);
        let off = tab_row("X", "TABS", false, Style::default(), Style::default());
        assert_eq!(off.len(), 1);
    }

    #[test]
    fn stock_tabs_ship_toggleable_icons() {
        use ratatui::style::Style;
        let tabs = crate::model::default_main_tabs();
        assert_eq!(tabs.len(), 5);
        for tab in &tabs {
            assert!(!tab.icon.is_empty(), "tab {} has an icon", tab.id);
            assert!(
                tab.icon.chars().all(|c| ('\u{e000}'..='\u{f8ff}').contains(&c)),
                "tab {} icon is NF-only: {:?}",
                tab.id,
                tab.icon
            );
            let rendered =
                tab_row(&tab.icon, &tab.title, false, Style::default(), Style::default());
            assert_eq!(rendered.len(), 1, "icons off leaves just the title");
        }
        assert!(crate::config::GeneralConfig::default().icons);
    }
}

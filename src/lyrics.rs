use std::path::{Path, PathBuf};
use std::time::Duration;

use lofty::file::TaggedFileExt;
use serde::Deserialize;

use crate::model::Song;

#[derive(Debug, Clone, Default)]
pub struct Lyrics {
    pub lines: Vec<(f64, String)>,
    pub text: Option<String>,
    pub source: String,
}

#[derive(Debug, Deserialize)]
struct SonumLyricsDto {
    #[serde(default)]
    lines: Vec<SonumLyricLine>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    source: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SonumLyricLine {
    #[serde(default)]
    time_ms: Option<u64>,
    #[serde(default)]
    text: Option<String>,
}

pub fn fetch_lyrics(song: &Song) -> Option<Lyrics> {
    if let Some(found) = fetch_sonum_lyrics(song) {
        return Some(found);
    }
    fetch_local_lyrics(song)
}

fn fetch_sonum_lyrics(song: &Song) -> Option<Lyrics> {
    let server = crate::sonum::sonum_config_for_url(&song.webpage_url)?;
    let id = song.id.trim();
    if id.is_empty() || id.contains('/') || id.contains('\\') || id.contains("..") {
        return None;
    }
    let url = format!("{}/tracks/{id}/lyrics", server.base_url());
    let mut request = ureq::get(&url)
        .config()
        .timeout_global(Some(Duration::from_secs(10)))
        .build();
    if let Some(token) = server.api_token.as_deref() {
        let token = token.trim();
        if !token.is_empty() {
            request = request.header("Authorization", &format!("Bearer {token}"));
        }
    }
    let mut response = request.call().ok()?;
    let dto: SonumLyricsDto = response.body_mut().read_json().ok()?;
    let mut lines: Vec<(f64, String)> = dto
        .lines
        .into_iter()
        .filter_map(|line| {
            let text = line.text.unwrap_or_default();
            if text.trim().is_empty() {
                return None;
            }
            Some((line.time_ms.unwrap_or(0) as f64 / 1000.0, text))
        })
        .collect();
    lines.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    let has_content = !lines.is_empty() || dto.text.as_ref().is_some_and(|t| !t.trim().is_empty());
    if !has_content {
        return None;
    }
    Some(Lyrics {
        lines,
        text: dto.text.filter(|t| !t.trim().is_empty()),
        source: dto.source.unwrap_or_else(|| "sonum".to_owned()),
    })
}

fn fetch_local_lyrics(song: &Song) -> Option<Lyrics> {
    let path = crate::cover::local_path_for(song)?;
    if let Some(lrc_path) = sibling_lrc(&path) {
        if let Ok(raw) = std::fs::read_to_string(&lrc_path) {
            if let Some(parsed) = parse_lrc(&raw) {
                return Some(parsed);
            }
        }
    }
    embedded_lyrics(&path)
}

fn sibling_lrc(path: &Path) -> Option<PathBuf> {
    let stem = path.file_stem()?.to_str()?;
    let lrc = path.with_file_name(format!("{stem}.lrc"));
    if lrc.is_file() {
        return Some(lrc);
    }
    None
}

fn embedded_lyrics(path: &Path) -> Option<Lyrics> {
    let tagged = lofty::read_from_path(path).ok()?;
    let tag = tagged.primary_tag()?;
    let raw = tag
        .get_string(lofty::tag::ItemKey::Lyrics)
        .or_else(|| tag.get_string(lofty::tag::ItemKey::Lyricist))?;
    if raw.trim().is_empty() {
        return None;
    }
    if let Some(parsed) = parse_lrc(raw) {
        return Some(parsed);
    }
    Some(Lyrics {
        lines: Vec::new(),
        text: Some(raw.to_owned()),
        source: "embedded".to_owned(),
    })
}

pub fn parse_lrc(raw: &str) -> Option<Lyrics> {
    let mut lines: Vec<(f64, String)> = Vec::new();
    let mut plain: Vec<String> = Vec::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let stamps = collect_stamps(trimmed);
        let text = strip_stamps(trimmed).trim().to_owned();
        if text.is_empty() {
            continue;
        }
        if stamps.is_empty() {
            plain.push(text);
        } else {
            for stamp in stamps {
                lines.push((stamp, text.clone()));
            }
        }
    }
    if lines.is_empty() && plain.is_empty() {
        return None;
    }
    lines.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    let text = if lines.is_empty() {
        Some(plain.join("\n"))
    } else {
        None
    };
    Some(Lyrics {
        lines,
        text,
        source: "lrc".to_owned(),
    })
}

fn collect_stamps(line: &str) -> Vec<f64> {
    let mut stamps = Vec::new();
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'[' {
            i += 1;
            continue;
        }
        if let Some(end) = line[i..].find(']') {
            let inner = &line[i + 1..i + end];
            if let Some(secs) = parse_stamp(inner) {
                stamps.push(secs);
            }
            i += end + 1;
        } else {
            break;
        }
    }
    stamps
}

fn strip_stamps(line: &str) -> String {
    let mut out = String::new();
    let mut i = 0;
    let bytes = line.as_bytes();
    while i < bytes.len() {
        if bytes[i] == b'[' {
            if let Some(end) = line[i..].find(']') {
                i += end + 1;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn parse_stamp(raw: &str) -> Option<f64> {
    let mut parts = raw.split(':');
    let minutes: f64 = parts.next()?.trim().parse().ok()?;
    let rest = parts.next()?.trim();
    if parts.next().is_some() {
        return None;
    }
    let seconds: f64 = if rest.contains('.') {
        rest.parse().ok()?
    } else if rest.contains(',') {
        rest.replace(',', ".").parse().ok()?
    } else {
        rest.parse().ok()?
    };
    if minutes < 0.0 || seconds < 0.0 || seconds >= 100.0 {
        return None;
    }
    Some(minutes * 60.0 + seconds)
}

pub fn current_line_index(lines: &[(f64, String)], position: f64) -> Option<usize> {
    let mut current = None;
    for (idx, (time, _)) in lines.iter().enumerate() {
        if *time <= position + 0.05 {
            current = Some(idx);
        } else {
            break;
        }
    }
    current
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lrc_parses_synced_lines() {
        let raw = "[00:12.50] First\n[00:16.00] Second\n[01:02.10] Third";
        let parsed = parse_lrc(raw).expect("parsed");
        assert_eq!(parsed.lines.len(), 3);
        assert!((parsed.lines[0].0 - 12.5).abs() < 0.01);
        assert_eq!(parsed.lines[0].1, "First");
        assert!(parsed.text.is_none());
    }
    #[test]
    fn lrc_handles_plain_text() {
        let parsed = parse_lrc("Line one\nLine two").expect("parsed");
        assert!(parsed.lines.is_empty());
        assert_eq!(parsed.text.as_deref(), Some("Line one\nLine two"));
    }
    #[test]
    fn lrc_supports_multiple_stamps() {
        let parsed = parse_lrc("[00:10.00][00:20.00] Repeat").expect("parsed");
        assert_eq!(parsed.lines.len(), 2);
    }
    #[test]
    fn lrc_rejects_empty() {
        assert!(parse_lrc("   \n  ").is_none());
    }
    #[test]
    fn current_line_tracks_position() {
        let lines = vec![(10.0, "a".to_owned()), (20.0, "b".to_owned())];
        assert_eq!(current_line_index(&lines, 5.0), None);
        assert_eq!(current_line_index(&lines, 10.0), Some(0));
        assert_eq!(current_line_index(&lines, 25.0), Some(1));
    }
}

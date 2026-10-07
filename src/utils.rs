pub fn natural_compare(a: &str, b: &str) -> std::cmp::Ordering {
    let mut a_chars = a.chars().peekable();
    let mut b_chars = b.chars().peekable();
    loop {
        match (a_chars.peek(), b_chars.peek()) {
            (Some(&a_char), Some(&b_char)) => {
                if a_char.is_ascii_digit() && b_char.is_ascii_digit() {
                    let mut a_num = 0u64;
                    while let Some(&c) = a_chars.peek() {
                        if !c.is_ascii_digit() {
                            break;
                        }
                        a_num = a_num
                            .saturating_mul(10)
                            .saturating_add(c.to_digit(10).unwrap_or(0) as u64);
                        a_chars.next();
                    }
                    let mut b_num = 0u64;
                    while let Some(&c) = b_chars.peek() {
                        if !c.is_ascii_digit() {
                            break;
                        }
                        b_num = b_num
                            .saturating_mul(10)
                            .saturating_add(c.to_digit(10).unwrap_or(0) as u64);
                        b_chars.next();
                    }
                    if a_num != b_num {
                        return a_num.cmp(&b_num);
                    }
                } else {
                    if a_char != b_char {
                        return a_char.cmp(&b_char);
                    }
                    a_chars.next();
                    b_chars.next();
                }
            }
            (None, Some(_)) => return std::cmp::Ordering::Less,
            (Some(_), None) => return std::cmp::Ordering::Greater,
            (None, None) => return std::cmp::Ordering::Equal,
        }
    }
}
pub const AUDIO_EXTENSIONS: &[&str] = &["mp3", "flac", "wav", "ogg", "opus", "m4a"];
pub fn expand_tilde(path: &str) -> String {
    if let Some(rest) = path.strip_prefix('~') {
        match std::env::var("HOME") {
            Ok(home) => format!("{home}{rest}"),
            Err(_) => path.to_owned(),
        }
    } else {
        path.to_owned()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn audio_extensions_cover_common_lossy_and_lossless() {
        for ext in ["mp3", "flac", "wav", "ogg", "opus", "m4a"] {
            assert!(AUDIO_EXTENSIONS.contains(&ext), "missing {ext}");
        }
    }
    #[test]
    fn expand_tilde_leaves_plain_paths_alone() {
        assert_eq!(expand_tilde("/home/you/Music"), "/home/you/Music");
        assert_eq!(expand_tilde("relative/dir"), "relative/dir");
    }
    #[test]
    fn expand_tilde_resolves_home_prefix() {
        let dir = tempfile::tempdir().unwrap();
        let key = "HOME";
        let old = std::env::var(key).ok();
        unsafe {
            std::env::set_var(key, dir.path());
        }
        assert_eq!(
            expand_tilde("~/Music"),
            format!("{}/Music", dir.path().display())
        );
        unsafe {
            match old {
                Some(v) => std::env::set_var(key, v),
                None => std::env::remove_var(key),
            }
        }
    }
}

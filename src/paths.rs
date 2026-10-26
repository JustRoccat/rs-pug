use std::path::PathBuf;

pub fn config_home() -> PathBuf {
    if let Ok(dir) = std::env::var("XDG_CONFIG_HOME") {
        let trimmed = dir.trim().to_owned();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        if !home.trim().is_empty() {
            return PathBuf::from(home).join(".config");
        }
    }
    PathBuf::from(".config")
}

pub fn rs_pug_dir() -> PathBuf {
    config_home().join("rs-pug")
}

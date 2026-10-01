use clap::Parser;
#[derive(Parser, Debug)]
#[command(author, version, about = "rs-pug music player", long_about = None)]
pub struct Args {
    #[arg(
        short,
        long,
        help = "Search source: youtube, soundcloud, sonum, or a custom source name from config"
    )]
    pub source: Option<String>,
    #[arg(long)]
    pub play: Option<String>,
    #[arg(long)]
    pub toggle_pause: bool,
    #[arg(long)]
    pub next: bool,
    #[arg(long)]
    pub prev: bool,
    #[arg(long)]
    pub debug: bool,
    #[arg(long, help = "Validate config.toml and exit")]
    pub validate: bool,
}
pub fn validate_report(
    config: &crate::config::Config,
    warning: Option<&str>,
    path: Option<&std::path::Path>,
) -> (String, i32) {
    use crate::config::{source_label, source_persist_name, CustomSourceKind};
    let mut out = String::new();
    match path {
        Some(path) => out.push_str(&format!("Config file: {}\n", path.display())),
        None => out.push_str("Config file: not found, using defaults\n"),
    }
    out.push_str(&format!(
        "Search source: {} ({})\n",
        source_label(config.search.source, &config.search.custom_sources),
        source_persist_name(config.search.source, &config.search.custom_sources),
    ));
    out.push_str(&format!("Custom sources ({}):\n", config.search.custom_sources.len()));
    for custom in &config.search.custom_sources {
        match &custom.kind {
            CustomSourceKind::Ytdlp { prefix } => {
                out.push_str(&format!("  - {} [ytdlp] prefix=\"{prefix}\"\n", custom.name));
            }
            CustomSourceKind::Command {
                command,
                timeout_secs,
            } => {
                out.push_str(&format!(
                    "  - {} [command] argv=[{}] timeout={timeout_secs}s\n",
                    custom.name,
                    command.join(" "),
                ));
            }
        }
    }
    match warning {
        Some(warning) => {
            out.push_str(&format!("Warnings: {warning}\nResult: INVALID\n"));
            (out, 1)
        }
        None => {
            out.push_str("Result: OK\n");
            (out, 0)
        }
    }
}
pub fn resolve_cli_source(
    name: &str,
    customs: &[crate::config::CustomSource],
) -> Result<crate::config::SearchSource, String> {
    let source = crate::config::resolve_source_name(Some(name), customs);
    let known = crate::config::normalize_source_name(name);
    let is_known = matches!(known.as_str(), "youtube" | "soundcloud" | "sonum")
        || customs
            .iter()
            .any(|c| crate::config::source_names_equal(&c.name, name));
    if is_known {
        Ok(source)
    } else {
        Err(format!(
            "invalid value '{name}' for '--source <SOURCE>'\n  [possible values: {}]",
            crate::config::available_source_names(customs).join(", ")
        ))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{CustomSource, CustomSourceKind, SearchSource};
    fn customs() -> Vec<CustomSource> {
        vec![CustomSource {
            name: "My Script".to_owned(),
            kind: CustomSourceKind::Command {
                command: vec!["/bin/echo".to_owned()],
                timeout_secs: 15,
            },
        }]
    }
    #[test]
    fn resolves_builtins_and_customs_case_insensitive() {
        let customs = customs();
        assert_eq!(
            resolve_cli_source("youtube", &customs).unwrap(),
            SearchSource::YouTube
        );
        assert_eq!(
            resolve_cli_source("  MY SCRIPT ", &customs).unwrap(),
            SearchSource::Custom(0)
        );
    }
    #[test]
    fn rejects_unknown_with_available_list() {
        let customs = customs();
        let err = resolve_cli_source("nope", &customs).unwrap_err();
        assert!(err.contains("My Script"));
        assert!(err.contains("youtube"));
    }
    #[test]
    fn validate_report_marks_warnings_invalid() {
        use crate::config::Config;
        let cfg = Config::default();
        let (ok, code) = validate_report(&cfg, None, None);
        assert!(ok.contains("Result: OK"));
        assert_eq!(code, 0);
        let (bad, code) = validate_report(&cfg, Some("something skipped"), None);
        assert!(bad.contains("Result: INVALID"));
        assert!(bad.contains("something skipped"));
        assert_eq!(code, 1);
    }
    #[test]
    fn validate_report_lists_custom_sources() {
        use crate::config::Config;
        let mut cfg = Config::default();
        cfg.search.custom_sources = customs();
        let (report, _) = validate_report(&cfg, None, None);
        assert!(report.contains("Custom sources (1)"));
        assert!(report.contains("My Script"));
        assert!(report.contains("timeout=15s"));
    }
}

use clap::{Parser, ValueEnum};
use crate::config::SearchSource;
#[derive(Parser, Debug)]
#[command(author, version, about = "rs-pug music player", long_about = None)]
pub struct Args {
    #[arg(short, long)]
    pub source: Option<SourceArg>,
    #[arg(long)]
    pub play: Option<String>,
    #[arg(long)]
    pub toggle_pause: bool,
    #[arg(long)]
    pub next: bool,
    #[arg(long)]
    pub prev: bool,
}
#[derive(ValueEnum, Clone, Debug)]
pub enum SourceArg {
    Youtube,
    Soundcloud,
}
impl From<SourceArg> for SearchSource {
    fn from(arg: SourceArg) -> Self {
        match arg {
            SourceArg::Youtube => SearchSource::YouTube,
            SourceArg::Soundcloud => SearchSource::SoundCloud,
        }
    }
}

use clap::{Parser, ValueEnum};
use crate::config::SearchSource;

#[derive(Parser, Debug)]
#[command(author, version, about = "rs-pug music player", long_about = None)]
pub struct Args {
    /// Search source: youtube or soundcloud
    #[arg(short, long)]
    pub source: Option<SourceArg>,
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

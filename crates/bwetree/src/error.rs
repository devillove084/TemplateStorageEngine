use snafu::prelude::*;
use std::{fs, io, path::PathBuf};

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Unable to read configuration from {}", path.display()))]
    ReadConfiguration { source: io::Error, path: PathBuf },

    #[snafu(display("Unable to write result to {}", path.display()))]
    WriteResult { source: io::Error, path: PathBuf },

    #[snafu(display("Unable to read key"))]
    ReadResult {source: io::Error,}
}

#[derive(Debug, Snafu)]
pub enum PageIOError {}

pub type Result<T, E = Error> = std::result::Result<T, E>;
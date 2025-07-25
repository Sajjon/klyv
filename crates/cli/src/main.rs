mod fixtures;
mod init_logging;
mod test;

use clap::Parser;
use init_logging::init_logging;
use klyv_core::prelude::*;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = BINARY_NAME, about = "Splitting files with multiple types into separate files")]
#[command(version = env!("CARGO_PKG_VERSION"))]
pub struct CliArgs {
    #[arg(long, short)]
    source: PathBuf,
    /// If None, same dir as `source` will be used
    #[arg(long, short)]
    out: Option<PathBuf>,
}

/// Input for the CLI, containing the source path and optional output path
#[derive(Clone, Debug, Builder, Getters)]
pub struct Input {
    #[getset(get = "pub")]
    source: PathBuf,
    /// If None, same dir as `source` will be used
    #[getset(get = "pub")]
    out: Option<PathBuf>,
}
impl TryFrom<CliArgs> for Input {
    type Error = Error;

    fn try_from(args: CliArgs) -> Result<Self, Self::Error> {
        Ok(Input::builder()
            .source(args.source)
            .maybe_out(args.out)
            .build())
    }
}

pub trait ResultExt<T, E>: Sized {
    fn map_to_void(self) -> Result<(), E>;
}

impl<T, E> ResultExt<T, E> for Result<T, E> {
    fn map_to_void(self) -> Result<(), E> {
        self.map(|_| ())
    }
}

pub fn run(input: Input) -> Result<FileSystemNode> {
    info!("Splitting files at {}", input.source().display());
    let node = find_in().path(input.source()).call()?;
    write()
        .node(node.clone())
        .out(input.out().as_ref().unwrap_or(input.source()))
        .call()?;
    Ok(node)
}

fn run_cli() -> Result<()> {
    let args = CliArgs::parse();
    let input = Input::try_from(args)?;
    run(input).map_to_void()
}

fn main() {
    init_logging();
    info!("Starting klyv");
    match run_cli() {
        Ok(_) => debug!("Run completed successfully."),
        Err(e) => error!("Error: {}", e),
    }
}

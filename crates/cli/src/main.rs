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
    path: PathBuf,
}

#[derive(Clone, Debug, Builder, Getters)]
pub struct Input {
    #[getset(get = "pub")]
    path: PathBuf,
}
impl TryFrom<CliArgs> for Input {
    type Error = Error;

    fn try_from(args: CliArgs) -> Result<Self, Self::Error> {
        Ok(Input::builder().path(args.path).build())
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
    info!("Splitting files at {}", input.path().display());
    find_in().path(input.path()).call()
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

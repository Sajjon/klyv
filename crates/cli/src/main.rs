mod init_logging;
use std::path::PathBuf;

use init_logging::init_logging;

use clap::Parser;
use klyv_core::prelude::*;

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

fn run(input: Input) -> Result<()> {
    debug!("Splitting files at {}", input.path().display());
    Ok(())
}

fn run_cli() -> Result<()> {
    let args = CliArgs::parse();
    let input = Input::try_from(args)?;
    run(input)
}

fn main() {
    init_logging();
    info!("Starting klyv");
    match run_cli() {
        Ok(_) => debug!("Run completed successfully."),
        Err(e) => error!("Error: {}", e),
    }
}

use anyhow::Result;
use clap::Parser;
use clap::Subcommand;
use clap_verbosity_flag::Verbosity;
use spectool::command::test::Args as TestArgs;

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Performs conformance tests on the WDL specification.
    Test(TestArgs),
}

/// A command-line tool for working with the WDL specification.
#[derive(Parser, Debug)]
pub struct Args {
    #[command(subcommand)]
    command: Command,

    /// The verbosity arguments.
    #[command(flatten)]
    verbosity: Verbosity,
}

fn main() -> Result<()> {
    let args = Args::parse();

    tracing_subscriber::fmt()
        .with_max_level(args.verbosity)
        .init();

    match args.command {
        Command::Test(args) => spectool::command::test::main(args)?,
    };

    Ok(())
}

use clap::{command, Args, Parser, Subcommand};

#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Run(RunArgs),
    DryRun(DryRunArgs),
}

#[derive(Args)]
pub struct RunArgs {
    #[arg(long)]
    pub mock: bool,
}

#[derive(Args)]
pub struct DryRunArgs {
    #[arg(long)]
    pub calldata: Option<String>,
    #[arg(long)]
    pub bytecode: Option<String>,
    #[arg(long)]
    pub file: Option<String>,
}

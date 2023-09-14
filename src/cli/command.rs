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

    #[arg(long, env = "TRACE_PATH", default_value = "./traces/calculation.json")]
    pub trace_path: Vec<String>,

    #[arg(long)]
    pub batch_dir: Option<String>,

    #[arg(long, default_value = "output")]
    pub output_dir: String,

    #[arg(long, default_value = "vk_chunk_0.vkey")]
    pub chunk_vk_filename: String,

    #[arg(long, default_value = "./test_params")]
    pub chunk_params_dir: String,

    #[arg(long, default_value = "./test_assets")]
    pub chunk_assets_dir: String,

    #[arg(long, default_value = "./configs")]
    pub scroll_prover_assets_dir: String,
}

#[derive(Args)]
pub struct DryRunArgs {
    #[arg(short, long)]
    pub calldata: Option<String>,
    #[arg(short, long)]
    pub bytecode: Option<String>,
    #[arg(short = 'd', long)]
    pub hardcode: Option<String>,
    #[arg(short, long)]
    pub file: Option<String>,
}

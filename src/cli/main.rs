use clap::Parser;
use lambda_zkevm::cli::command::Cli;
use lambda_zkevm::cli::exec;

fn main() {
    let cli = Cli::parse();
    exec::match_operation(&cli);
}


use lambda_zkevm::cli::command;
use lambda_zkevm::cli::exec;


#[tokio::main]
async fn main() {
    let matches = command::parse_arguments();

    if let Some((subcommand, sub_matches)) = matches.subcommand() {
        exec::match_operation(subcommand, sub_matches).await;
    }
}

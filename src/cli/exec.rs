use super::command::{Cli, Commands};

mod dry_run;
mod run;

pub fn match_operation(cli: &Cli) {
    match &cli.command {
        Commands::DryRun(args) => {
            dry_run::exec_dry_run(
                args.calldata.as_ref(),
                args.bytecode.as_ref(),
                args.file.as_ref(),
            );
        }
        Commands::Run(args) => {
            if args.mock {
                run::run_mock_prove();
            } else {
                run::run_chunk_prove_verify();
            }
        }
    }
}

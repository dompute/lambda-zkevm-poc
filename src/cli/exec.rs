use std::path::Path;

use chrono::Utc;
use log::info;
use prover::{
    utils::{get_block_trace_from_file, init_env_and_log},
    BlockTrace,
};

use crate::run::load_batch_traces;

use super::command::{Cli, Commands, RunArgs};

mod dry_run;
mod run;

pub fn match_operation(cli: &Cli) {
    match &cli.command {
        Commands::DryRun(args) => {
            dry_run::exec_dry_run(
                args.calldata.as_deref(),
                args.bytecode.as_deref(),
                args.hardcode.as_deref(),
                args.file.as_deref(),
            );
        }
        Commands::Run(args) => {
            let output_dir = args.init();
            if args.mock {
                run::run_mock_prove(args.get_block_traces());
            } else {
                run::run_chunk_prove_verify(
                    args.get_block_traces(),
                    &output_dir,
                    &args.chunk_params_dir,
                    &args.chunk_assets_dir,
                    &args.chunk_vk_filename,
                );
            }
        }
    }
}

impl RunArgs {
    pub fn get_block_traces(&self) -> Vec<BlockTrace> {
        if let Some(batch_dir) = &self.batch_dir {
            info!("use batch chunk files under dir: {:?}", batch_dir);
            load_batch_traces(batch_dir).1
        } else {
            info!("use block traces files: {:?}", self.trace_path);
            self.trace_path
                .iter()
                .map(get_block_trace_from_file)
                .collect()
        }
    }

    pub fn id(&self) -> &str {
        if self.mock {
            "mock"
        } else {
            "chunk"
        }
    }

    pub fn mode(&self) -> &str {
        if self.batch_dir.is_some() || self.trace_path.len() > 1 {
            "multi"
        } else {
            "single"
        }
    }

    pub fn init(&self) -> String {
        let output_name = format!(
            "{}_output_{}_{}",
            self.id(),
            self.mode(),
            Utc::now().format("%Y%m%d_%H%M%S")
        );
        let output_dir = Path::new(&self.output_dir).join(output_name);
        std::fs::create_dir_all(&output_dir).unwrap();

        std::env::set_var("OUTPUT_DIR", output_dir.to_str().unwrap());
        std::env::set_var("SCROLL_PROVER_ASSETS_DIR", &self.scroll_prover_assets_dir);
        init_env_and_log(self.id())
    }
}

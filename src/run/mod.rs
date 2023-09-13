use glob::glob;
use prover::{utils::get_block_trace_from_file, BlockTrace};

pub mod mock_plonk;
mod proof;

pub use proof::{
    gen_and_verify_batch_proofs, gen_and_verify_chunk_proofs, gen_and_verify_normal_and_evm_proofs,
    gen_and_verify_normal_proof,
};

pub fn load_batch_traces(batch_dir: &str) -> (Vec<String>, Vec<BlockTrace>) {
    let file_names: Vec<String> = glob(&format!("{batch_dir}/**/*.json"))
        .unwrap()
        .map(|p| p.unwrap().to_str().unwrap().to_string())
        .collect();
    log::info!("test batch with {:?}", file_names);
    let mut names_and_traces = file_names
        .into_iter()
        .map(|trace_path| {
            let trace: BlockTrace = get_block_trace_from_file(trace_path.clone());
            (
                trace_path,
                trace.clone(),
                trace.header.number.unwrap().as_u64(),
            )
        })
        .collect::<Vec<_>>();
    names_and_traces.sort_by(|a, b| a.2.cmp(&b.2));
    log::info!(
        "sorted: {:?}",
        names_and_traces
            .iter()
            .map(|(f, _, _)| f.clone())
            .collect::<Vec<String>>()
    );
    names_and_traces.into_iter().map(|(f, t, _)| (f, t)).unzip()
}

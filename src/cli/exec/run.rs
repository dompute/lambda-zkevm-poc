use crate::run::gen_and_verify_chunk_proofs;
use prover::{utils::chunk_trace_to_witness_block, zkevm::circuit::SuperCircuit, BlockTrace};
use std::env;

pub(crate) fn run_mock_prove(block_traces: Vec<BlockTrace>) {
    prover::inner::Prover::<SuperCircuit>::mock_prove_target_circuit_batch(&block_traces).unwrap();
}

pub(crate) fn run_chunk_prove_verify(
    chunk_trace: Vec<BlockTrace>,
    output_dir: &str,
    params_dir: &str,
    assets_dir: &str,
    chunk_vk_filename: &str,
) {
    let witness_block = chunk_trace_to_witness_block(chunk_trace).unwrap();
    log::info!("Got witness block");

    env::set_var("CHUNK_VK_FILENAME", chunk_vk_filename);
    let mut zkevm_prover = prover::zkevm::Prover::from_dirs(params_dir, assets_dir);
    log::info!("Constructed zkevm prover");

    // Load or generate compression wide snark (layer-1).
    let layer1_snark = zkevm_prover
        .inner
        .load_or_gen_last_chunk_snark("layer1", &witness_block, None, Some(&output_dir))
        .unwrap();

    gen_and_verify_chunk_proofs(&mut zkevm_prover, layer1_snark, output_dir, params_dir);
}

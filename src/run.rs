use self::circuit::IntegrationTest;
use bus_mapping::circuit_input_builder::FixedCParams;
use halo2_proofs::halo2curves::bn256::Fr;
use zkevm_circuits::evm_circuit::EvmCircuit;

pub mod circuit;

/// MAX_TXS
const MAX_TXS: usize = 4;
/// MAX_CALLDATA
const MAX_CALLDATA: usize = 512;
/// MAX_RWS
const MAX_RWS: usize = 5888;
/// MAX_BYTECODE
const MAX_BYTECODE: usize = 5000;
/// MAX_COPY_ROWS
const MAX_COPY_ROWS: usize = 5888;
/// MAX_EVM_ROWS
const MAX_EVM_ROWS: usize = 10000;
/// MAX_EXP_STEPS
const MAX_EXP_STEPS: usize = 1000;

const MAX_KECCAK_ROWS: usize = 38000;

const CIRCUITS_PARAMS: FixedCParams = FixedCParams {
    max_rws: MAX_RWS,
    max_txs: MAX_TXS,
    max_calldata: MAX_CALLDATA,
    max_bytecode: MAX_BYTECODE,
    max_copy_rows: MAX_COPY_ROWS,
    max_evm_rows: MAX_EVM_ROWS,
    max_exp_steps: MAX_EXP_STEPS,
    max_keccak_rows: MAX_KECCAK_ROWS,
};

const EVM_CIRCUIT_DEGREE: u32 = 18;
const ROOT_CIRCUIT_SMALL_DEGREE: u32 = 24;

pub async fn run_test() {
    let mut evm_test: IntegrationTest<EvmCircuit<Fr>> =
        IntegrationTest::new("EVM", EVM_CIRCUIT_DEGREE, ROOT_CIRCUIT_SMALL_DEGREE);
    evm_test
        .test_at_block_tag("ERC20 OpenZeppelin transfer successful", false, true)
        .await;
}

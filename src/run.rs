use self::{circuit::IntegrationTest, rpc::TraceCallParams};
use crate::Opts;
use bus_mapping::circuit_input_builder::FixedCParams;
use eth_types::Address;
use halo2_proofs::{halo2curves::bn256::Fr, plonk::Circuit};
use zkevm_circuits::util::SubCircuit;

pub mod builder;
pub mod circuit;
pub mod rpc;

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

#[cfg(not(feature = "super"))]
const EVM_CIRCUIT_DEGREE: u32 = 18;
#[cfg(not(feature = "super"))]
const ROOT_CIRCUIT_SMALL_DEGREE: u32 = 24;

#[cfg(feature = "super")]
const SUPER_CIRCUIT_DEGREE: u32 = 20;
#[cfg(feature = "super")]
const ROOT_CIRCUIT_BIG_DEGREE: u32 = 26;

pub async fn run_test<C>(name: &'static str, opts: &Opts)
where
    C: SubCircuit<Fr> + Circuit<Fr>,
{
    let (degree, root_degree) = {
        #[cfg(not(feature = "super"))]
        {
            (EVM_CIRCUIT_DEGREE, ROOT_CIRCUIT_SMALL_DEGREE)
        }
        #[cfg(feature = "super")]
        {
            (SUPER_CIRCUIT_DEGREE, ROOT_CIRCUIT_BIG_DEGREE)
        }
    };

    let mut it: IntegrationTest<C> = IntegrationTest::new(name, degree, root_degree);
    // TODO: make this configurable
    test_pure_call(
        "0xffDb339065c91c88e8a3cC6857359B6c2FB78cf5"
            .parse()
            .unwrap(),
						"0x79bdc88780158af4bd20b969da5173871713114e".parse().unwrap(),
				100000,
				hex::decode("771602f700000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000003").unwrap(),
        6,
        &mut it,
        opts,
    )
    .await;
}

async fn test_pure_call<C>(
    from: Address,
    to: Address,
    gas: u64,
    data: Vec<u8>,
    block_number: u64,
    it: &mut IntegrationTest<C>,
    opts: &Opts,
) where
    C: SubCircuit<Fr> + Circuit<Fr>,
{
    let params = TraceCallParams {
        from: format!("{:?}", from),
        to: format!("{:?}", to),
        gas: format!("0x{:x}", gas),
        data: hex::encode(data),
    };
    it.test_at_height(
        block_number,
        it.proof_name("PureCall"),
        &params,
        opts.is_root(),
        opts.is_actual(),
    )
    .await;
}

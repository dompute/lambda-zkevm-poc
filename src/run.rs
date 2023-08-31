use self::{circuit::IntegrationTest, rpc::TraceCallParams};
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

pub async fn run_test<C>(name: &'static str, is_root: bool, is_actual: bool, is_gv: bool)
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
    if is_gv {
        test_groth16_verifier(is_root, is_actual, &mut it).await
    } else {
        test_calculation(&mut it).await;
    }
}

async fn test_groth16_verifier<C>(is_root: bool, is_actual: bool, it: &mut IntegrationTest<C>)
where
    C: SubCircuit<Fr> + Circuit<Fr>,
{
    test_pure_call(
        "0xffDb339065c91c88e8a3cC6857359B6c2FB78cf5"
            .parse()
            .unwrap(),
		"0x357224ff702b88ac26a6deda1640face692adb96".parse().unwrap(),
		1_000_000,
		hex::decode(r#"43753b4d28da0fd7778f50c6136d2448f015faf1491ad8f869e5509c1309802feaa0b32f0e1c822477ba388fd90b7172e5088ba8e3c843bb3e7d370b7a57e1050f5daa8f01e68fa3d08c93f1098aea19aa134c90dc676050be6e81d12ebfb8a2eb8b184907eccf490d162477bd89036355856aa70b1dbeb76c4484d2420e06f3928457ba0add63e22690cb781fcf5f106fa441c3558e305fb5ea8ae2a7d1889b65d79f3f298a3d4d12b993972f7cd6cf3383cb43a0c8f7c95288952127e7fb411c4aa79a16c4818e0004b83596391aed769dfeae45e17187e531b9d47744bbdd29e6cf7c04334671d4cf8f42078c195b1e6f223e845622b3fc7904834496ef6390f0e5d90000000000000000000000000000000000000000000000000000000000000021"#).unwrap(),
        7,
        it, 
        is_root,
        is_actual,
    )
    .await;
}

async fn test_calculation<C>( it: &mut IntegrationTest<C>)
where
    C: SubCircuit<Fr> + Circuit<Fr>,
{
    test_pure_call(
        "0xffDb339065c91c88e8a3cC6857359B6c2FB78cf5"
            .parse()
            .unwrap(),
		"0x79bdc88780158af4bd20b969da5173871713114e".parse().unwrap(),
		100_000,
		hex::decode(r#"771602f700000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000003"#).unwrap(),
        6,
        it,
        false,
        true
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
    is_root: bool,
    is_actual: bool,
    
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
        is_root,
        is_actual,
    )
    .await;
}

use crate::gen::types::{get_client, GenDataOutput};
use bus_mapping::{
    circuit_input_builder::{CircuitInputBuilder, FixedCParams},
    mock::BlockData,
};
use eth_types::geth_types::GethData;
use halo2_proofs::{
    self,
    circuit::Value,
    dev::{CellValue, MockProver},
    halo2curves::bn256::{Bn256, Fr, G1Affine},
    plonk::{
        create_proof, keygen_pk, keygen_vk, permutation::Assembly, verify_proof, Circuit,
        ProvingKey,
    },
    poly::{
        commitment::ParamsProver,
        kzg::{
            commitment::{KZGCommitmentScheme, ParamsKZG, ParamsVerifierKZG},
            multiopen::{ProverSHPLONK, VerifierSHPLONK},
            strategy::SingleStrategy,
        },
    },
};
use lazy_static::lazy_static;
use mock::TestContext;
use rand_core::SeedableRng;
use rand_xorshift::XorShiftRng;
use std::{collections::HashMap, marker::PhantomData, sync::Mutex};
use tokio::sync::Mutex as TokioMutex;
use zkevm_circuits::{
    root_circuit::{
        compile, Config, EvmTranscript, NativeLoader, PoseidonTranscript, RootCircuit, Shplonk,
    },
    util::SubCircuit,
    witness::{block_convert, Block},
};

use super::{builder::ZkBuilderClient, rpc::TraceCallParams, CIRCUITS_PARAMS};

lazy_static! {
    /// Data generation.
    static ref GEN_DATA: GenDataOutput = GenDataOutput::load();
    static ref RNG: XorShiftRng = XorShiftRng::from_seed([
        0x59, 0x62, 0xbe, 0x5d, 0x76, 0x3d, 0x31, 0x8d, 0x17, 0xdb, 0x37, 0x32, 0x54, 0x06, 0xbc,
        0xe5,
    ]);
}

lazy_static! {
    static ref GEN_PARAMS: Mutex<HashMap<u32, ParamsKZG<Bn256>>> = Mutex::new(HashMap::new());
}

lazy_static! {
    /// Cache of real proofs from each block to be reused with the Root circuit tests
    static ref PROOF_CACHE: TokioMutex<HashMap<String, Vec<u8>>> = TokioMutex::new(HashMap::new());
}

/// TEST_MOCK_RANDOMNESS
const TEST_MOCK_RANDOMNESS: u64 = 0x100;

/// Generic implementation for integration tests
pub struct IntegrationTest<C: SubCircuit<Fr> + Circuit<Fr>> {
    name: &'static str,
    degree: u32,
    root_degree: u32,
    key: Option<ProvingKey<G1Affine>>,
    root_key: Option<ProvingKey<G1Affine>>,
    fixed: Option<Vec<Vec<CellValue<Fr>>>>,
    permutation: Option<Assembly>,
    // The RootCircuit changes depending on the underlying circuit, so we keep a copy of its fixed
    // columns and permutation here to have a unique version for each SubCircuit.
    root_fixed: Option<Vec<Vec<CellValue<Fr>>>>,
    root_permutation: Option<Assembly>,
    _marker: PhantomData<C>,
}

impl<C: SubCircuit<Fr> + Circuit<Fr>> IntegrationTest<C> {
    pub fn new(name: &'static str, degree: u32, root_degree: u32) -> Self {
        Self {
            name,
            degree,
            root_degree,
            key: None,
            root_key: None,
            fixed: None,
            permutation: None,
            root_fixed: None,
            root_permutation: None,
            _marker: PhantomData,
        }
    }

    pub fn proof_name(&self, block_tag: &str) -> String {
        format!("{}_{}", self.name, block_tag)
    }

    fn get_key(&mut self) -> ProvingKey<G1Affine> {
        match self.key.clone() {
            Some(key) => key,
            None => {
                let block = new_empty_block();
                let circuit = C::new_from_block(&block);
                let general_params = get_general_params(self.degree);

                let verifying_key =
                    keygen_vk(&general_params, &circuit).expect("keygen_vk should not fail");
                let key = keygen_pk(&general_params, verifying_key, &circuit)
                    .expect("keygen_pk should not fail");
                self.key = Some(key.clone());
                key
            }
        }
    }

    fn get_root_key(&mut self) -> ProvingKey<G1Affine> {
        match self.root_key.clone() {
            Some(key) => key,
            None => {
                let params = get_general_params(self.degree);
                let pk = self.get_key();

                let block = new_empty_block();
                let circuit = C::new_from_block(&block);
                let instance = circuit.instance();

                let protocol = compile(
                    &params,
                    pk.get_vk(),
                    Config::kzg().with_num_instance(
                        instance.iter().map(|instance| instance.len()).collect(),
                    ),
                );
                let circuit = RootCircuit::<Bn256, Shplonk<_>>::new(
                    &params,
                    &protocol,
                    Value::unknown(),
                    Value::unknown(),
                )
                .unwrap();

                let general_params = get_general_params(self.root_degree);
                let verifying_key =
                    keygen_vk(&general_params, &circuit).expect("keygen_vk should not fail");
                let key = keygen_pk(&general_params, verifying_key, &circuit)
                    .expect("keygen_pk should not fail");
                self.root_key = Some(key.clone());
                key
            }
        }
    }

    fn test_mock(&mut self, circuit: &C, instance: Vec<Vec<Fr>>) {
        let mock_prover = MockProver::<Fr>::run(self.degree, circuit, instance).unwrap();

        self.test_variadic(&mock_prover);

        mock_prover
            .verify_par()
            .expect("mock prover verification failed");
    }

    fn test_variadic(&mut self, mock_prover: &MockProver<Fr>) {
        let fixed = mock_prover.fixed();

        if let Some(prev_fixed) = self.fixed.clone() {
            assert!(
                fixed.eq(&prev_fixed),
                "circuit fixed columns are not constant for different witnesses"
            );
        } else {
            self.fixed = Some(fixed.clone());
        }

        let permutation = mock_prover.permutation();

        if let Some(prev_permutation) = self.permutation.clone() {
            assert!(
                permutation.eq(&prev_permutation),
                "circuit permutations are not constant for different witnesses"
            );
        } else {
            self.permutation = Some(permutation.clone());
        }
    }

    fn test_root_variadic(&mut self, mock_prover: &MockProver<Fr>) {
        let fixed = mock_prover.fixed();

        match self.root_fixed.clone() {
            Some(prev_fixed) => {
                assert!(
                    fixed.eq(&prev_fixed),
                    "root circuit fixed columns are not constant for different witnesses"
                );
            }
            None => {
                self.root_fixed = Some(fixed.clone());
            }
        };

        let permutation = mock_prover.permutation();

        if let Some(prev_permutation) = self.root_permutation.clone() {
            assert!(
                permutation.eq(&prev_permutation),
                "root circuit permutations are not constant for different witnesses"
            );
        } else {
            self.root_permutation = Some(permutation.clone());
        }
    }

    pub async fn test_at_height(
        &mut self,
        block_num: u64,
        proof_name: String,
        params: &TraceCallParams,
        root: bool,
        actual: bool,
    ) {
        let (builder, _) = gen_inputs(params, block_num).await;

        log::info!(
            "test {} circuit{}, {} prover, block: #{}",
            self.name,
            if root {
                " with aggregation (root circuit)"
            } else {
                ""
            },
            if actual { "real" } else { "mock" },
            block_num,
        );
        let mut block = block_convert(&builder).unwrap();
        block.randomness = Fr::from(TEST_MOCK_RANDOMNESS);
        let circuit = C::new_from_block(&block);
        let instance = circuit.instance();

        #[allow(clippy::collapsible_else_if)]
        if root {
            let params = get_general_params(self.degree);
            let pk = self.get_key();
            let protocol = compile(
                &params,
                pk.get_vk(),
                Config::kzg()
                    .with_num_instance(instance.iter().map(|instance| instance.len()).collect()),
            );

            let proof = {
                let mut proof_cache = PROOF_CACHE.lock().await;
                if let Some(proof) = proof_cache.get(&proof_name) {
                    log::info!("using circuit cached proof");
                    proof.clone()
                } else {
                    let key = self.get_key();
                    log::info!("circuit proof generation (no proof in the cache)");
                    let proof = test_actual_circuit(circuit, self.degree, instance.clone(), key);
                    proof_cache.insert(proof_name, proof.clone());
                    proof
                }
            };

            log::info!("root circuit new");
            let root_circuit = RootCircuit::<Bn256, Shplonk<_>>::new(
                &params,
                &protocol,
                Value::known(&instance),
                Value::known(&proof),
            )
            .unwrap();

            if actual {
                let root_key = self.get_root_key();
                let instance = root_circuit.instance();
                log::info!("root circuit proof generation");
                test_actual_root_circuit(root_circuit, self.root_degree, instance, root_key);
            } else {
                log::info!("root circuit mock prover verification");
                // Mock
                let mock_prover =
                    MockProver::<Fr>::run(self.root_degree, &root_circuit, root_circuit.instance())
                        .unwrap();
                self.test_root_variadic(&mock_prover);
                mock_prover
                    .verify_par()
                    .expect("mock prover verification failed");
            }
        } else {
            if actual {
                let key = self.get_key();
                log::info!("circuit proof generation");
                let proof = test_actual_circuit(circuit, self.degree, instance, key);
                let mut proof_cache = PROOF_CACHE.lock().await;
                proof_cache.insert(proof_name, proof);
            } else {
                log::info!("circuit mock prover verification");
                self.test_mock(&circuit, instance);
            }
        }
    }
}

/// Generate a real proof of the RootCircuit with Keccak transcript and Shplonk accumulation
/// scheme.  Verify the proof and return it.  By using the Keccak transcript (via EvmTranscript)
/// the resulting proof is suitable for verification by the EVM.
///
/// NOTE: MockProver Root Circuit with 64 GiB RAM (2023-06-12):
/// - degree=26 -> OOM
/// - degree=25 -> OK (peak ~35 GiB)
fn test_actual_root_circuit<C: Circuit<Fr>>(
    circuit: C,
    degree: u32,
    instance: Vec<Vec<Fr>>,
    proving_key: ProvingKey<G1Affine>,
) -> Vec<u8> {
    let general_params = get_general_params(degree);
    let verifier_params: ParamsVerifierKZG<Bn256> = general_params.verifier_params().clone();

    let mut transcript = EvmTranscript::<_, NativeLoader, _, _>::new(vec![]);

    // change instace to slice
    let instance: Vec<&[Fr]> = instance.iter().map(|v| v.as_slice()).collect();

    log::info!("gen root circuit proof");
    create_proof::<KZGCommitmentScheme<Bn256>, ProverSHPLONK<'_, Bn256>, _, _, _, _>(
        &general_params,
        &proving_key,
        &[circuit],
        &[&instance],
        RNG.clone(),
        &mut transcript,
    )
    .expect("proof generation should not fail");
    let proof = transcript.finalize();

    log::info!("verify root circuit proof");
    let verifying_key = proving_key.get_vk();
    let mut verifier_transcript = EvmTranscript::<_, NativeLoader, _, _>::new(proof.as_slice());
    let strategy = SingleStrategy::new(&general_params);

    verify_proof::<KZGCommitmentScheme<Bn256>, VerifierSHPLONK<'_, Bn256>, _, _, _>(
        &verifier_params,
        verifying_key,
        strategy,
        &[&instance],
        &mut verifier_transcript,
    )
    .expect("failed to verify circuit");

    proof
}

fn get_general_params(degree: u32) -> ParamsKZG<Bn256> {
    let mut map = GEN_PARAMS.lock().unwrap();
    match map.get(&degree) {
        Some(params) => params.clone(),
        None => {
            let params = ParamsKZG::<Bn256>::setup(degree, RNG.clone());
            map.insert(degree, params.clone());
            params
        }
    }
}

/// Generate a real proof of a Circuit with Poseidon transcript and Shplonk accumulation scheme.
/// Verify the proof and return it.  The proof is suitable to be verified by the Root Circuit.
fn test_actual_circuit<C: Circuit<Fr>>(
    circuit: C,
    degree: u32,
    instance: Vec<Vec<Fr>>,
    proving_key: ProvingKey<G1Affine>,
) -> Vec<u8> {
    let general_params = get_general_params(degree);
    let verifier_params: ParamsVerifierKZG<Bn256> = general_params.verifier_params().clone();

    let mut transcript = PoseidonTranscript::new(Vec::new());

    // change instace to slice
    let instance: Vec<&[Fr]> = instance.iter().map(|v| v.as_slice()).collect();

    log::info!("gen circuit proof");
    create_proof::<KZGCommitmentScheme<Bn256>, ProverSHPLONK<'_, Bn256>, _, _, _, _>(
        &general_params,
        &proving_key,
        &[circuit],
        &[&instance],
        RNG.clone(),
        &mut transcript,
    )
    .expect("proof generation should not fail");
    let proof = transcript.finalize();

    log::info!("verify circuit proof");
    let verifying_key = proving_key.get_vk();
    let mut verifier_transcript = PoseidonTranscript::new(proof.as_slice());
    let strategy = SingleStrategy::new(&general_params);

    verify_proof::<KZGCommitmentScheme<Bn256>, VerifierSHPLONK<'_, Bn256>, _, _, _>(
        &verifier_params,
        verifying_key,
        strategy,
        &[&instance],
        &mut verifier_transcript,
    )
    .expect("failed to verify circuit");

    proof
}

/// returns gen_inputs for a block number
async fn gen_inputs(
    params: &TraceCallParams,
    block_num: u64,
) -> (
    CircuitInputBuilder<FixedCParams>,
    eth_types::Block<eth_types::Transaction>,
) {
    let cli = get_client();
    let cli = ZkBuilderClient::new(cli, CIRCUITS_PARAMS).await.unwrap();

    cli.gen_inputs(params, block_num).await.unwrap()
}

fn new_empty_block() -> Block<Fr> {
    let block: GethData = TestContext::<0, 0>::new(None, |_| {}, |_, _| {}, |b, _| b)
        .unwrap()
        .into();
    let mut builder = BlockData::new_from_geth_data_with_params(block.clone(), CIRCUITS_PARAMS)
        .new_circuit_input_builder();
    builder
        .handle_block(&block.eth_block, &block.geth_traces)
        .unwrap();
    block_convert(&builder).unwrap()
}
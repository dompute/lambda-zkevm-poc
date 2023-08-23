use clap::Parser;
use log::info;

mod gen;
mod node;
mod run;

#[derive(Parser, Debug)]
pub struct Opts {
    #[clap(short, long)]
    root: Option<bool>,

    #[clap(short, long)]
    actual: Option<bool>,

    #[clap(short, long)]
    groth16_verifier: bool,
}

#[tokio::main]
async fn main() {
    gen::types::log_init();
    let opts = Opts::parse();

    let (_api, node_handle) = node::new_anvil_node().await;

    let endpoint = node_handle.http_endpoint();
    info!("Anvil endpoint is: {}", endpoint);
    tokio::spawn(async move {
        if let Err(e) = node_handle.await {
            panic!("Anvil node error: {:?}", e);
        }
        info!("Anvil node exited");
    });

    gen::gen_block_data().await;

    #[cfg(not(feature = "super"))]
    run::run_test::<zkevm_circuits::evm_circuit::EvmCircuit<halo2_proofs::halo2curves::bn256::Fr>>(
        "EVM", &opts,
    )
    .await;

    #[cfg(feature = "super")]
    run::run_test::<
        zkevm_circuits::super_circuit::SuperCircuit<halo2_proofs::halo2curves::bn256::Fr>,
    >("Super", &opts)
    .await;
}

impl Opts {
    pub fn is_root(&self) -> bool {
        self.root.unwrap_or(false)
    }

    pub fn is_actual(&self) -> bool {
        self.actual.unwrap_or(true)
    }
}

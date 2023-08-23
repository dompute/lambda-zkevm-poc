use log::info;

mod gen;
mod node;
mod run;

#[tokio::main]
async fn main() {
    gen::types::log_init();

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
        "EVM",
    )
    .await;

    #[cfg(feature = "super")]
    run::run_test::<
        zkevm_circuits::super_circuit::SuperCircuit<halo2_proofs::halo2curves::bn256::Fr>,
    >("Super")
    .await;
}

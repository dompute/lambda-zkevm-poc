use clap::ArgMatches;
use log::info;

use crate::gen;
use crate::node;
use crate::run;

pub async fn match_operation(subcommand: &str, sub_matchs: &ArgMatches) {
    match subcommand {
        "prove" => {
            let is_root = sub_matchs.get_flag("root");
            let is_actual = sub_matchs.get_flag("actual");
            let is_gv = sub_matchs.get_flag("gv");
            exec_prove(is_root, is_actual, is_gv).await
        }
        "verify" => exec_verify(),
        "dry-run" => exec_dry_run(),
        _ => println!("Unknown subcommand"),
    }
}

pub async fn exec_prove(is_root: bool, is_actual: bool, is_gv: bool) {
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
        "EVM", is_root, is_actual, is_gv,
    )
    .await;

    #[cfg(feature = "super")]
    run::run_test::<
        zkevm_circuits::super_circuit::SuperCircuit<halo2_proofs::halo2curves::bn256::Fr>,
    >("Super", is_root, is_actual, is_gv)
    .await;
}

pub fn exec_verify() {
    println!("Performing 'verify' operation ")
}

pub fn exec_dry_run() {
    println!("Performing 'dry-run' operation ")
}

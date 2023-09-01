use std::{
    fs::File,
    io::{BufRead, BufReader},
};
use clap::ArgMatches;
use log::info;

use crate::dry_run::bytecode_run::bytecode_run;
use crate::gen;
use crate::node;
use crate::run;

pub async fn match_operation(subcommand: &str, sub_matchs: &ArgMatches) {
    match subcommand {
        "prove" => {
            let is_root = sub_matchs.get_flag("root");
            let is_actual = sub_matchs.get_flag("actual");
            let is_gv = sub_matchs.get_flag("gv");
            println!("root {}, actual {}, gv {}", is_root, is_actual, is_gv);
            exec_prove(is_root, is_actual, is_gv).await;
        }
        "verify" => exec_verify(),
        "dry-run" => {
            let calldata = sub_matchs.get_one::<String>("calldata").unwrap();
            let bytecode = sub_matchs.get_one::<String>("bytecode").unwrap();
            let file = sub_matchs.get_one::<String>("file");
            exec_dry_run(&calldata, &bytecode, file);
    }
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

pub fn exec_dry_run(calldata: &str, bytecode: &str, _file: Option<&String>) {
    // let (calldata, bytecode) = if file.is_some() {
    //     read_from_file(file.unwrap()).unwrap();
    // } else {
    let (calldata, bytecode) = parse_from_args(calldata, bytecode).unwrap();
    // };

    match bytecode_run(calldata, bytecode) {
        Ok(r) => {
            println!(
                "Bytecode exec successfully, result (in hex):\n{}",
                hex::encode(r)
            )
        }
        Err(e) => {
            println!("Bytecode exec failed, reason: {}", e.to_string())
        }
    };
}

fn convert(calldata: &str, bytecode: &str) -> anyhow::Result<(Vec<u8>, Vec<u8>)> {
    Ok((
        hex::decode(calldata.trim_start_matches("0x"))?,
        hex::decode(bytecode.trim_start_matches("0x"))?,
    ))
}

fn _read_from_file(file: &str) -> anyhow::Result<(Vec<u8>, Vec<u8>)> {
    let file = File::open(file)?;
    let mut reader = BufReader::new(file);

    let mut calldata = String::new();
    let mut bytecode = String::new();
    reader.read_line(&mut calldata)?;
    reader.read_line(&mut bytecode)?;
    convert(&calldata.trim_end(), &bytecode.trim_end())
}

fn parse_from_args(calldata: &str, bytecode: &str) -> anyhow::Result<(Vec<u8>, Vec<u8>)> {
    convert(calldata, bytecode)
}

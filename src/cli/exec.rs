use clap::ArgMatches;
use std::{
    fs::File,
    io::{BufRead, BufReader},
};

use crate::dry_run::bytecode_run::bytecode_run;

pub async fn match_operation(subcommand: &str, sub_matchs: &ArgMatches) {
    match subcommand {
        "verify" => exec_verify(),
        "dry-run" => {
            let calldata = sub_matchs.get_one::<String>("calldata");
            let bytecode = sub_matchs.get_one::<String>("bytecode");
            let file = sub_matchs.get_one::<String>("file");
            exec_dry_run(calldata, bytecode, file);
        }
        _ => println!("Unknown subcommand"),
    }
}

pub fn exec_verify() {
    println!("Performing 'verify' operation ")
}

pub fn exec_dry_run(calldata: Option<&String>, bytecode: Option<&String>, file: Option<&String>) {
    let (calldata, bytecode) = if file.is_some() {
        read_from_file(file.unwrap()).unwrap()
    } else {
        parse_from_args(
            calldata.expect("should have calldata"),
            bytecode.expect("should have bytecode"),
        )
        .unwrap()
    };

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

fn read_from_file(file: &str) -> anyhow::Result<(Vec<u8>, Vec<u8>)> {
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

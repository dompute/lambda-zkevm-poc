use crate::dry_run::bytecode_run::bytecode_run;
use std::{
    fs::File,
    io::{BufRead, BufReader},
};

pub(crate) fn exec_dry_run(
    calldata: Option<&str>,
    bytecode: Option<&str>,
    hardcode: Option<&str>,
    file: Option<&str>,
) {
    let (calldata, bytecode, hardcode) = if file.is_some() {
        read_from_file(file.unwrap()).unwrap()
    } else {
        parse_from_args(
            calldata.expect("should have calldata"),
            bytecode.expect("should have bytecode"),
            hardcode,
        )
        .unwrap()
    };

    match bytecode_run(calldata, bytecode, hardcode) {
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
fn convert(
    calldata: &str,
    bytecode: &str,
    hardcode: Option<&str>,
) -> anyhow::Result<(Vec<u8>, Vec<u8>, Option<Vec<u8>>)> {
    Ok((
        hex::decode(calldata.trim_start_matches("0x"))?,
        hex::decode(bytecode.trim_start_matches("0x"))?,
        hardcode.map_or(Ok(None), |h| {
            hex::decode(h.trim_start_matches("0x")).map(Some)
        })?,
    ))
}

fn read_from_file(file: &str) -> anyhow::Result<(Vec<u8>, Vec<u8>, Option<Vec<u8>>)> {
    let file = File::open(file)?;
    let mut reader = BufReader::new(file);

    let mut calldata = String::new();
    let mut bytecode = String::new();
    let mut hardcode = String::new();
    reader.read_line(&mut calldata)?;
    reader.read_line(&mut bytecode)?;

    if reader.read_line(&mut hardcode)? == 0 {
        return convert(&calldata.trim_end(), &bytecode.trim_end(), None);
    }

    convert(
        &calldata.trim_end(),
        &bytecode.trim_end(),
        Some(&hardcode.trim_end()),
    )
}

fn parse_from_args(
    calldata: &str,
    bytecode: &str,
    hardcode: Option<&str>,
) -> anyhow::Result<(Vec<u8>, Vec<u8>, Option<Vec<u8>>)> {
    convert(calldata, bytecode, hardcode)
}

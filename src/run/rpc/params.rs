use eth_types::{GethExecStep, GethExecTrace};
use serde::{Deserialize, Deserializer, Serialize};

/// The execution trace type returned by geth RPC debug_trace* methods.
/// Corresponds to `ExecutionResult` in `go-ethereum/internal/ethapi/api.go`.
/// The deserialization truncates the memory of each step in `struct_logs` to
/// the memory size before the expansion, so that it corresponds to the memory
/// before the step is executed.
#[derive(Deserialize, Serialize, Clone, Debug, Eq, PartialEq)]
pub struct ZkGethExecTrace {
    #[serde(deserialize_with = "custom_deserialize")]
    /// Used gas
    pub gas: u64,
    /// True when the transaction has failed.
    pub failed: bool,
    /// Return value of execution which is a hex encoded byte array
    #[serde(rename = "returnValue")]
    pub return_value: String,
    /// Vector of geth execution steps of the trace.
    #[serde(rename = "structLogs")]
    pub struct_logs: Vec<GethExecStep>,
}

impl From<GethExecTrace> for ZkGethExecTrace {
    fn from(trace: GethExecTrace) -> Self {
        Self {
            gas: trace.gas,
            failed: trace.failed,
            return_value: trace.return_value,
            struct_logs: trace.struct_logs,
        }
    }
}

impl From<ZkGethExecTrace> for GethExecTrace {
    fn from(trace: ZkGethExecTrace) -> Self {
        Self {
            gas: trace.gas,
            failed: trace.failed,
            return_value: trace.return_value,
            struct_logs: trace.struct_logs,
        }
    }
}

fn custom_deserialize<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    let hex_string = String::deserialize(deserializer)?;
    let result = u64::from_str_radix(&hex_string[2..], 16);
    // TODO: fix this unwrap
    Ok(result.unwrap())
}

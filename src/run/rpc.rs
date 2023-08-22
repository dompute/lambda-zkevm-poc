use bus_mapping::Error;
use eth_types::{
    Address, Block, Bytes, EIP1186ProofResponse, GethExecTrace, Hash, Transaction, Word, U64,
};
pub use ethers::core::types::BlockNumber;
use ethers::providers::JsonRpcClient;
use serde::Serialize;

use self::params::ZkGethExecTrace;

pub mod params;

/// Serialize a type.
///
/// # Panics
///
/// If the type returns an error during serialization.
pub fn serialize<T: serde::Serialize>(t: &T) -> serde_json::Value {
    serde_json::to_value(t).expect("Types never fail to serialize.")
}

#[derive(Serialize)]
#[doc(hidden)]
pub struct TraceCallParams {
    pub from: String,
    pub to: String,
    pub gas: String,
    pub data: String,
}

#[derive(Serialize)]
#[doc(hidden)]
pub(crate) struct GethLoggerConfig {
    /// enable memory capture
    #[serde(rename = "EnableMemory")]
    enable_memory: bool,
    /// disable stack capture
    #[serde(rename = "DisableStack")]
    disable_stack: bool,
    /// disable storage capture
    #[serde(rename = "DisableStorage")]
    disable_storage: bool,
    /// enable return data capture
    #[serde(rename = "EnableReturnData")]
    enable_return_data: bool,
}

impl Default for GethLoggerConfig {
    fn default() -> Self {
        Self {
            enable_memory: false,
            disable_stack: false,
            disable_storage: false,
            enable_return_data: true,
        }
    }
}

/// Placeholder structure designed to contain the methods that the BusMapping
/// needs in order to enable Geth queries.
pub struct ZkGethClient<P: JsonRpcClient>(pub P);

impl<P: JsonRpcClient> ZkGethClient<P> {
    /// Generates a new `GethClient` instance.
    pub fn new(provider: P) -> Self {
        Self(provider)
    }

    /// Calls `eth_chainId` via JSON-RPC returning the chain id of the network.
    pub async fn get_chain_id(&self) -> Result<u64, Error> {
        let net_id: U64 = self
            .0
            .request("eth_chainId", ())
            .await
            .map_err(|e| Error::JSONRpcError(e.into()))?;
        Ok(net_id.as_u64())
    }

    /// Calls `eth_getBlockByHash` via JSON-RPC returning a [`Block`] returning
    /// all the block information including it's transaction's details.
    pub async fn get_block_by_hash(&self, hash: Hash) -> Result<Block<Transaction>, Error> {
        let hash = serialize(&hash);
        let flag = serialize(&true);
        self.0
            .request("eth_getBlockByHash", [hash, flag])
            .await
            .map_err(|e| Error::JSONRpcError(e.into()))
    }

    /// Calls `eth_getBlockByNumber` via JSON-RPC returning a [`Block`]
    /// returning all the block information including it's transaction's
    /// details.
    pub async fn get_block_by_number(
        &self,
        block_num: BlockNumber,
    ) -> Result<Block<Transaction>, Error> {
        let num = serialize(&block_num);
        let flag = serialize(&true);
        self.0
            .request("eth_getBlockByNumber", [num, flag])
            .await
            .map_err(|e| Error::JSONRpcError(e.into()))
    }

    /// Calls `debug_traceCall` via JSON-RPC returning a
    /// [`Vec<GethExecTrace>`] with each GethTrace corresponding to 1
    /// transaction of the block.
    pub async fn trace_call(
        &self,
        params: &TraceCallParams,
        block_num: BlockNumber,
    ) -> Result<Vec<GethExecTrace>, Error> {
        let num = serialize(&block_num);
        let param = serialize(&params);
        let resp: ZkGethExecTrace = self
            .0
            .request("debug_traceCall", [param, num])
            .await
            .map_err(|e| Error::JSONRpcError(e.into()))?;
        Ok(vec![resp.into()])
    }

    /// Calls `eth_getCode` via JSON-RPC returning a contract code
    pub async fn get_code(
        &self,
        contract_address: Address,
        block_num: BlockNumber,
    ) -> Result<Vec<u8>, Error> {
        let address = serialize(&contract_address);
        let num = serialize(&block_num);
        let resp: Bytes = self
            .0
            .request("eth_getCode", [address, num])
            .await
            .map_err(|e| Error::JSONRpcError(e.into()))?;
        Ok(resp.to_vec())
    }

    /// Calls `eth_getProof` via JSON-RPC returning a
    /// [`EIP1186ProofResponse`] returning the account and
    /// storage-values of the specified account including the Merkle-proof.
    pub async fn get_proof(
        &self,
        account: Address,
        keys: Vec<Word>,
        block_num: BlockNumber,
    ) -> Result<EIP1186ProofResponse, Error> {
        let account = serialize(&account);
        let keys = serialize(&keys);
        let num = serialize(&block_num);
        self.0
            .request("eth_getProof", [account, keys, num])
            .await
            .map_err(|e| Error::JSONRpcError(e.into()))
    }
}

use std::collections::HashMap;

use bus_mapping::{
    circuit_input_builder::{
        build_state_code_db, get_state_accesses, AccessSet, Block, CircuitInputBuilder,
        FixedCParams,
    },
    state_db::{CodeDB, StateDB},
    Error,
};
use eth_types::{Address, ToWord, Word};
use ethers::providers::JsonRpcClient;
use log::debug;

use super::rpc::{TraceCallParams, ZkGethClient};

type EthBlock = eth_types::Block<eth_types::Transaction>;

/// Struct that wraps a GethClient and contains methods to perform all the steps
/// necessary to generate the circuit inputs for a block by querying geth for
/// the necessary information and using the CircuitInputBuilder.
pub struct ZkBuilderClient<P: JsonRpcClient> {
    cli: ZkGethClient<P>,
    chain_id: Word,
    circuits_params: FixedCParams,
}

impl<P: JsonRpcClient> ZkBuilderClient<P> {
    /// Create a new BuilderClient
    pub async fn new(
        client: ZkGethClient<P>,
        circuits_params: FixedCParams,
    ) -> Result<Self, Error> {
        let chain_id = client.get_chain_id().await?;

        Ok(Self {
            cli: client,
            chain_id: chain_id.into(),
            circuits_params,
        })
    }

    /// Step 1. Query geth for Block, Txs, TxExecTraces, history block hashes
    /// and previous state root.
    pub async fn get_block(
        &self,
        params: &TraceCallParams,
        block_num: u64,
    ) -> Result<(EthBlock, Vec<eth_types::GethExecTrace>, Vec<Word>, Word), Error> {
        let eth_block = self.cli.get_block_by_number(block_num.into()).await?;
        let geth_traces = self.cli.trace_call(params, block_num.into()).await?;

        // fetch up to 256 blocks
        let mut n_blocks = std::cmp::min(256, block_num as usize);
        let mut next_hash = eth_block.parent_hash;
        let mut prev_state_root: Option<Word> = None;
        let mut history_hashes = vec![Word::default(); n_blocks];
        while n_blocks > 0 {
            n_blocks -= 1;

            // TODO: consider replacing it with `eth_getHeaderByHash`, it's faster
            let header = self.cli.get_block_by_hash(next_hash).await?;

            // set the previous state root
            if prev_state_root.is_none() {
                prev_state_root = Some(header.state_root.to_word());
            }

            // latest block hash is the last item
            let block_hash = header
                .hash
                .ok_or(Error::EthTypeError(eth_types::Error::IncompleteBlock))?
                .to_word();
            history_hashes[n_blocks] = block_hash;

            // continue
            next_hash = header.parent_hash;
        }

        Ok((
            eth_block,
            geth_traces,
            history_hashes,
            prev_state_root.unwrap_or_default(),
        ))
    }

    /// Step 2. Get State Accesses from TxExecTraces
    pub fn get_state_accesses(
        eth_block: &EthBlock,
        geth_traces: &[eth_types::GethExecTrace],
    ) -> Result<AccessSet, Error> {
        get_state_accesses(eth_block, geth_traces)
    }

    /// Step 3. Query geth for all accounts, storage keys, and codes from
    /// Accesses
    pub async fn get_state(
        &self,
        block_num: u64,
        access_set: AccessSet,
    ) -> Result<
        (
            Vec<eth_types::EIP1186ProofResponse>,
            HashMap<Address, Vec<u8>>,
        ),
        Error,
    > {
        let mut proofs = Vec::new();
        for (address, key_set) in access_set.state {
            let mut keys: Vec<Word> = key_set.iter().cloned().collect();
            keys.sort();
            let proof = self
                .cli
                .get_proof(address, keys, (block_num - 1).into())
                .await
                .unwrap();
            proofs.push(proof);
        }
        let mut codes: HashMap<Address, Vec<u8>> = HashMap::new();
        for address in access_set.code {
            let code = self
                .cli
                .get_code(address, (block_num - 1).into())
                .await
                .unwrap();
            codes.insert(address, code);
        }
        Ok((proofs, codes))
    }

    /// Step 4. Build a partial StateDB from step 3
    pub fn build_state_code_db(
        proofs: Vec<eth_types::EIP1186ProofResponse>,
        codes: HashMap<Address, Vec<u8>>,
    ) -> (StateDB, CodeDB) {
        build_state_code_db(proofs, codes)
    }

    /// Step 5. For each step in TxExecTraces, gen the associated ops and state
    /// circuit inputs
    pub fn gen_inputs_from_state(
        &self,
        sdb: StateDB,
        code_db: CodeDB,
        eth_block: &EthBlock,
        geth_traces: &[eth_types::GethExecTrace],
        history_hashes: Vec<Word>,
        prev_state_root: Word,
    ) -> Result<CircuitInputBuilder<FixedCParams>, Error> {
        let block = Block::new(self.chain_id, history_hashes, prev_state_root, eth_block)?;
        let mut builder = CircuitInputBuilder::new(sdb, code_db, block, self.circuits_params);
        builder.handle_block(eth_block, geth_traces)?;
        Ok(builder)
    }

    /// Perform all the steps to generate the circuit inputs
    pub async fn gen_inputs(
        &self,
        params: &TraceCallParams,
        block_num: u64,
    ) -> Result<
        (
            CircuitInputBuilder<FixedCParams>,
            eth_types::Block<eth_types::Transaction>,
        ),
        Error,
    > {
        let (eth_block, geth_traces, history_hashes, prev_state_root) =
            self.get_block(params, block_num).await?;
        debug!("=== DBG 1 ===");
        debug!("eth_block: {:#?}", eth_block);
        debug!("geth_traces: {:#?}", geth_traces);
        debug!("history_hashes: {:#?}", history_hashes);
        debug!("prev_state_root: {:#?}", prev_state_root);
        let access_set = Self::get_state_accesses(&eth_block, &geth_traces)?;
        debug!("=== DBG 2 ===");
        debug!("access_set: {:#?}", access_set);
        let (proofs, codes) = self.get_state(block_num, access_set).await?;
        debug!("=== DBG 3 ===");
        debug!("proofs: {:#?}", proofs);
        debug!("codes: {:#?}", codes);
        let (state_db, code_db) = Self::build_state_code_db(proofs, codes);
        debug!("=== DBG 4 ===");
        debug!("state_db: {:#?}", state_db);
        debug!("code_db: {:#?}", code_db);
        let builder = self.gen_inputs_from_state(
            state_db,
            code_db,
            &eth_block,
            &geth_traces,
            history_hashes,
            prev_state_root,
        )?;
        debug!("=== DBG 5 ===");
        debug!("builder: {:#?}", builder);
        Ok((builder, eth_block))
    }
}

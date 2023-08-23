use ethers::{
    abi::{self, Tokenize},
    contract::{builders::ContractCall, Contract, ContractFactory},
    core::{
        types::{
            transaction::eip2718::TypedTransaction, Address, TransactionReceipt,
            TransactionRequest, U256, U64,
        },
        utils::WEI_IN_ETHER,
    },
    middleware::SignerMiddleware,
    providers::{Http, Middleware, Provider},
    signers::Signer,
    solc::{CompilerInput, CompilerOutput, EvmVersion, Solc},
};
use log::{error, info};
use serde::de::Deserialize;
use std::{collections::HashMap, fs::File, path::Path, sync::Arc, thread::sleep, time::Duration};
use types::{get_provider, get_wallet, CompiledContract, GenDataOutput, CONTRACTS, CONTRACTS_PATH};

pub mod types;

async fn deploy<T, M>(prov: Arc<M>, compiled: &CompiledContract, args: T) -> Contract<M>
where
    T: Tokenize,
    M: Middleware,
{
    info!("Deploying {}...", compiled.name);
    let factory = ContractFactory::new(compiled.abi.clone(), compiled.bin.clone(), prov);
    factory
        .deploy(args)
        .expect("cannot deploy")
        .confirmations(0usize)
        .send()
        .await
        .expect("cannot confirm deploy")
}

fn erc20_transfer<M>(
    prov: Arc<M>,
    contract_address: Address,
    contract_abi: &abi::Contract,
    to: Address,
    amount: U256,
) -> TypedTransaction
where
    M: Middleware,
{
    let contract = Contract::new(contract_address, contract_abi.clone(), prov);
    let call: ContractCall<M, _> = contract
        .method::<_, bool>("transfer", (to, amount))
        .expect("cannot construct ERC20 transfer call");
    // Set gas to avoid `eth_estimateGas` call
    let call = call.legacy();
    let call = call.gas(100_000);
    call.tx
}

async fn gen_calculation_call<M>(
    deployments: &HashMap<String, (u64, Address)>,
    blocks: &mut HashMap<String, u64>,
    contracts: &HashMap<String, CompiledContract>,
    prov: &Arc<M>,
) where
    M: Middleware,
{
    let contract_address = deployments
        .get("Calculation")
        .expect("contract not found")
        .1;
    let contract_abi = &contracts
        .get("Calculation")
        .expect("contract not found")
        .abi;

    let contract = Contract::new(contract_address, contract_abi.clone(), prov.clone());
    let call: ContractCall<M, _> = contract
        .method::<_, U256>("add", (U256::from(2), U256::from(3)))
        .expect("cannot construct ERC20 transfer call");
    // Set gas to avoid `eth_estimateGas` call
    let call = call.legacy();
    let call = call.gas(100_000);

    let receipt = send_confirm_tx(prov, call.tx).await;
    assert_eq!(receipt.status, Some(U64::from(1u64)));
    blocks.insert(
        "Calculation add successful".to_string(),
        receipt.block_number.unwrap().as_u64(),
    );
    info!("Calculation add successfully");
}

async fn gen_erc20_call<M>(
    deployments: &HashMap<String, (u64, Address)>,
    blocks: &mut HashMap<String, u64>,
    contracts: &HashMap<String, CompiledContract>,
    prov: &Arc<M>,
    to_address: Address,
) where
    M: Middleware,
{
    let contract_address = deployments
        .get("OpenZeppelinERC20TestToken")
        .expect("contract not found")
        .1;
    let contract_abi = &contracts
        .get("OpenZeppelinERC20TestToken")
        .expect("contract not found")
        .abi;

    // OpenZeppelin ERC20 single successful transfer (wallet0 sends 123.45 Tokens to
    // wallet4)
    info!("Doing OpenZeppelin ERC20 single successful transfer...");
    let amount = U256::from_dec_str("123450000000000000000").unwrap();
    let tx = erc20_transfer(
        prov.clone(),
        contract_address,
        contract_abi,
        to_address,
        amount,
    );
    let receipt = send_confirm_tx(prov, tx).await;
    assert_eq!(receipt.status, Some(U64::from(1u64)));
    blocks.insert(
        "ERC20 OpenZeppelin transfer successful".to_string(),
        receipt.block_number.unwrap().as_u64(),
    );
}

async fn send_confirm_tx<M>(prov: &Arc<M>, tx: TypedTransaction) -> TransactionReceipt
where
    M: Middleware,
{
    prov.send_transaction(tx, None)
        .await
        .expect("cannot send ERC20 transfer call")
        .confirmations(0usize)
        .await
        .unwrap()
        .unwrap()
}

fn compile_contracts() -> HashMap<String, CompiledContract> {
    let solc = Solc::default();
    info!("Solc version {}", solc.version().expect("version works"));
    let mut contracts = HashMap::new();
    for (name, contract_path) in CONTRACTS {
        let path_sol = Path::new(CONTRACTS_PATH).join(contract_path);
        let inputs = CompilerInput::new(&path_sol).expect("Compile success");
        // ethers-solc: explicitly indicate the EvmVersion that corresponds to the zkevm circuit's
        // supported Upgrade, e.g. `London/Shanghai/...` specifications.
        let input = inputs
            .clone()
            .first_mut()
            .expect("first exists")
            .clone()
            .evm_version(EvmVersion::London);

        // compilation will either fail with Err variant or return Ok(CompilerOutput)
        // which may contain Errors or Warnings
        let output = solc.compile_output(&input).unwrap();
        let mut deserializer: serde_json::Deserializer<serde_json::de::SliceRead<'_>> =
            serde_json::Deserializer::from_slice(&output);
        // The contracts to test the worst-case usage of certain opcodes, such as SDIV, MLOAD, and
        // EXTCODESIZE, generate big JSON compilation outputs. We disable the recursion limit to
        // avoid parsing failure.
        deserializer.disable_recursion_limit();
        let compiled = match CompilerOutput::deserialize(&mut deserializer) {
            Err(error) => {
                panic!("COMPILATION ERROR {:?}\n{:?}", &path_sol, error);
            }
            // CompilationOutput is succesfully created (might contain Errors or Warnings)
            Ok(output) => {
                info!("COMPILATION OK: {:?}", name);
                output
            }
        };

        if compiled.has_error() {
            panic!(
                "... but CompilerOutput contains errors/warnings: {:?}:\n{:#?}",
                &path_sol, compiled.errors
            );
        }

        let contract = compiled
            .get(path_sol.to_str().expect("path is not str"), name)
            .expect("contract not found");
        let abi = contract.abi.expect("no abi found").clone();
        let bin = contract.bin.expect("no bin found").clone();
        let bin_runtime = contract.bin_runtime.expect("no bin_runtime found").clone();
        let compiled_contract = CompiledContract {
            path: path_sol.to_str().expect("path is not str").to_string(),
            name: name.to_string(),
            abi,
            bin: bin.into_bytes().expect("bin"),
            bin_runtime: bin_runtime.into_bytes().expect("bin_runtime"),
        };

        let mut path_json = path_sol.clone();
        path_json.set_extension("json");
        serde_json::to_writer(
            &File::create(&path_json).expect("cannot create file"),
            &compiled_contract,
        )
        .expect("cannot serialize json into file");

        contracts.insert(name.to_string(), compiled_contract);
    }

    contracts
}

async fn prepare_provider() -> Provider<Http> {
    let prov = get_provider();

    // Wait for geth to be online.
    loop {
        match prov.client_version().await {
            Ok(version) => {
                info!("Geth online: {}", version);
                break;
            }
            Err(err) => {
                error!("Geth not available: {:?}", err);
                sleep(Duration::from_millis(500));
            }
        }
    }

    // Make sure the blockchain is in a clean state: block 0 is the last block.
    let block_number = prov
        .get_block_number()
        .await
        .expect("cannot get block number");
    if block_number.as_u64() != 0 {
        panic!(
            "Blockchain is not in a clean state.  Last block number: {}",
            block_number
        );
    }
    prov
}

async fn transfer(blocks: &mut HashMap<String, u64>, prov: &Provider<Http>, accounts: &[Address]) {
    let wallet0 = get_wallet(0);
    let tx = TransactionRequest::new()
        .to(wallet0.address())
        .value(WEI_IN_ETHER) // send 1 ETH
        .from(accounts[0]);
    prov.send_transaction(tx, None)
        .await
        .expect("cannot send tx")
        .await
        .expect("cannot confirm tx");
    let block_num = prov.get_block_number().await.expect("cannot get block_num");
    blocks.insert("Transfer 0".to_string(), block_num.as_u64());
}

async fn deploy_contracts(
    contracts: &HashMap<String, CompiledContract>,
    blocks: &mut HashMap<String, u64>,
    prov: &Provider<Http>,
) -> HashMap<String, (u64, Address)> {
    let wallet0 = get_wallet(0);
    let mut deployments = HashMap::new();
    let prov_wallet0 = Arc::new(SignerMiddleware::new(get_provider(), wallet0));

    // OpenZeppelinERC20TestToken
    let contract = deploy(
        prov_wallet0.clone(),
        contracts
            .get("OpenZeppelinERC20TestToken")
            .expect("contract not found"),
        prov_wallet0.address(),
    )
    .await;
    let block_num = prov.get_block_number().await.expect("cannot get block_num");
    blocks.insert(
        "Deploy OpenZeppelinERC20TestToken".to_string(),
        block_num.as_u64(),
    );
    deployments.insert(
        "OpenZeppelinERC20TestToken".to_string(),
        (block_num.as_u64(), contract.address()),
    );

    // Calculation
    let contract = deploy(
        prov_wallet0.clone(),
        contracts.get("Calculation").expect("contract not found"),
        (),
    )
    .await;
    let block_num = prov.get_block_number().await.expect("cannot get block_num");
    blocks.insert("Deploy Calculation".to_string(), block_num.as_u64());
    deployments.insert(
        "Calculation".to_string(),
        (block_num.as_u64(), contract.address()),
    );

    // Groth16Verifier
    let contract = deploy(
        prov_wallet0.clone(),
        contracts
            .get("Groth16Verifier")
            .expect("contract not found"),
        (),
    )
    .await;
    let block_num = prov.get_block_number().await.expect("cannot get block_num");
    blocks.insert("Deploy Groth16Verifier".to_string(), block_num.as_u64());
    deployments.insert(
        "Groth16Verifier".to_string(),
        (block_num.as_u64(), contract.address()),
    );

    deployments
}

pub async fn gen_block_data() {
    // Compile contracts
    info!("Compiling contracts...");
    let contracts = compile_contracts();
    let prov = prepare_provider().await;

    let accounts = prov.get_accounts().await.expect("cannot get accounts");
    let wallet0 = get_wallet(0);
    info!("wallet0: {:x}", wallet0.address());

    let mut blocks = HashMap::new();

    info!("Transferring funds from coinbase...");
    transfer(&mut blocks, &prov, &accounts).await;

    // Deploy smart contracts
    let deployments = deploy_contracts(&contracts, &mut blocks, &prov).await;

    // ETH transfers: Generate a block with multiple transfers
    const NUM_TXS: usize = 4; // NUM_TXS must be >= 4 for the rest of the cases to work.
    let wallets = (0..NUM_TXS + 1)
        .map(|i| Arc::new(SignerMiddleware::new(get_provider(), get_wallet(i as u32))))
        .collect::<Vec<_>>();

    // ERC20 calls (OpenZeppelin)
    info!("Generating ERC20 calls...");
    gen_erc20_call(
        &deployments,
        &mut blocks,
        &contracts,
        &wallets[0],
        wallets[4].address(),
    )
    .await;

    // Calculation call
    gen_calculation_call(&deployments, &mut blocks, &contracts, &wallets[0]).await;

    let gen_data = GenDataOutput {
        coinbase: accounts[0],
        wallets: wallets.iter().map(|w| w.address()).collect(),
        blocks,
        deployments,
    };
    gen_data.store();
}

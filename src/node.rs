use anvil::{eth::EthApi, AccountGenerator, NodeConfig, NodeHandle};

pub async fn new_anvil_node() -> (EthApi, NodeHandle) {
    let config = NodeConfig::default()
        .with_chain_id(Some(1337u64))
        .with_port(8546)
        .with_tracing(false)
        .with_steps_tracing(true)
        .with_account_generator(AccountGenerator::new(10).phrase(
            "work man father plunge mystery proud hollow address reunion sauce theory bonus",
        ));
    anvil::spawn(config).await
}

use log::info;

mod gen;
mod node;
mod run;

#[tokio::main]
async fn main() {
    gen::types::log_init();

    let (api, node_handle) = node::new_anvil_node().await;

    let endpoint = node_handle.http_endpoint();
    info!("Anvil endpoint is: {}", endpoint);
    tokio::spawn(async move {
        if let Err(e) = node_handle.await {
            panic!("Anvil node error: {:?}", e);
        }
        info!("Anvil node exited");
    });

    gen::gen_block_data().await;
    run::run_test().await;
}

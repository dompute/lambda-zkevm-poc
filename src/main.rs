mod gen;
mod run;

#[tokio::main]
async fn main() {
    gen::types::log_init();

    gen::gen_block_data().await;
    run::run_test().await;
}

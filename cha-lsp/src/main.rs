#[tokio::main]
async fn main() {
    cha_lsp::run_lsp().await;
}

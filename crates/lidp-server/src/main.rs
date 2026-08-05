#[tokio::main]
async fn main() -> std::io::Result<()> {
    lidp_server::run().await
}

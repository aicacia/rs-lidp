#[tokio::main]
async fn main() -> std::io::Result<()> {
    lidp_management_server::run().await
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    lidp_http_server::run().await
}

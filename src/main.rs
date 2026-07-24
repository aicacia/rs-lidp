#[tokio::main]
async fn main() -> std::io::Result<()> {
    lidp::run().await
}

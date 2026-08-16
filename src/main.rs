#[tokio::main]
async fn main() -> std::io::Result<()> {
    lidp_unified::run().await
}

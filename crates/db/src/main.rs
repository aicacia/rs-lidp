#[tokio::main]
async fn main() -> std::io::Result<()> {
    db::run().await
}

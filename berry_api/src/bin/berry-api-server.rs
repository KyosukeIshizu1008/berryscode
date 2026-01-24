use berry_api::server::BerryApiServer;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "berry_api=info,tower_http=info".into()),
        )
        .init();

    let addr = "[::1]:50051".parse()?;

    tracing::info!("🚀 Starting berry-api-server on {}", addr);

    let server = BerryApiServer::new();
    server.serve(addr).await?;

    Ok(())
}

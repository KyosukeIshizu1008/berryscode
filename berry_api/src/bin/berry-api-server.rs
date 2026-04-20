use berry_api::server::BerryApiServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "berry_api=info,tower_http=info".into()),
        )
        .init();

    let addr_str = std::env::var("BERRY_API_ADDR").unwrap_or_else(|_| "[::1]:50051".to_string());
    let addr: std::net::SocketAddr = addr_str
        .parse()
        .map_err(|e| format!("Invalid BERRY_API_ADDR '{}': {}", addr_str, e))?;

    tracing::info!("🚀 Starting berry-api-server on {}", addr);

    let server = BerryApiServer::new();
    server.serve(addr).await?;

    Ok(())
}

use crate::berry_api::berry_code_service_server::BerryCodeServiceServer;
use crate::services::BerryCodeServiceImpl;
use std::net::SocketAddr;
use tonic::transport::Server;

pub struct BerryApiServer {
    berry_code_service: BerryCodeServiceImpl,
}

impl BerryApiServer {
    pub fn new() -> Self {
        Self {
            berry_code_service: BerryCodeServiceImpl::new(),
        }
    }

    pub async fn serve(self, addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
        Server::builder()
            .add_service(BerryCodeServiceServer::new(self.berry_code_service))
            .serve(addr)
            .await?;

        Ok(())
    }
}

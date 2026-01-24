use crate::berry_api::berry_code_service_server::BerryCodeServiceServer;
use crate::lsp_service::lsp_service_server::LspServiceServer;
use crate::services::{BerryCodeServiceImpl, LspServiceImpl};
use std::net::SocketAddr;
use tonic::transport::Server;

pub struct BerryApiServer {
    berry_code_service: BerryCodeServiceImpl,
    lsp_service: LspServiceImpl,
}

impl BerryApiServer {
    pub fn new() -> Self {
        Self {
            berry_code_service: BerryCodeServiceImpl::new(),
            lsp_service: LspServiceImpl::new(),
        }
    }

    pub async fn serve(self, addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
        Server::builder()
            .add_service(BerryCodeServiceServer::new(self.berry_code_service))
            .add_service(LspServiceServer::new(self.lsp_service))
            .serve(addr)
            .await?;

        Ok(())
    }
}

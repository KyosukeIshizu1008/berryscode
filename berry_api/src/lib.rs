pub mod server;
pub mod services;
pub mod session;
pub mod llm;

// Include generated proto code
pub mod berry_api {
    tonic::include_proto!("berry_api");
}

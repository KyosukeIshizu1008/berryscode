pub mod dap;
pub mod fs;
pub mod git;
pub mod grpc;
pub mod lsp_native;
pub mod rest_client;
pub mod search;
pub mod terminal;
pub mod watcher;

// Re-export get_client from grpc module
pub use grpc::get_client;

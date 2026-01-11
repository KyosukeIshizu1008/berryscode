//! Coder implementations for different edit formats

pub mod base_coder;
pub mod wholefile_coder;

pub use base_coder::Coder;
pub use wholefile_coder::WholefileCoder;

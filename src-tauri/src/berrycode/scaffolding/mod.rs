//! AI-powered project scaffolding module

pub mod generator;
pub mod prompts;

pub use generator::generate_project_structure;
pub use prompts::{FileToCreate, ProjectType, ScaffoldingRequest, ScaffoldingResponse};

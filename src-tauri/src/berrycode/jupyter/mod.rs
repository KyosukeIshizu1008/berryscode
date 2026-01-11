//! Jupyter Notebook support for BerryCode
//!
//! Provides functionality for:
//! - Parsing and rendering .ipynb files
//! - Managing Jupyter kernels
//! - Executing notebook cells
//! - Rendering rich outputs (images, tables, plots, etc.)

pub mod notebook;
pub mod kernel;

pub use notebook::{Notebook, Cell, CellType, Output, OutputType};
pub use kernel::{KernelManager, KernelStatus, ExecutionResult};

//! Jupyter Notebook (.ipynb) parsing and data structures

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use anyhow::{Context, Result};

/// Jupyter Notebook structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notebook {
    pub cells: Vec<Cell>,
    pub metadata: NotebookMetadata,
    pub nbformat: i32,
    pub nbformat_minor: i32,
}

/// Notebook metadata
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NotebookMetadata {
    #[serde(default)]
    pub kernelspec: Option<KernelSpec>,
    #[serde(default)]
    pub language_info: Option<LanguageInfo>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Kernel specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelSpec {
    pub display_name: String,
    pub language: String,
    pub name: String,
}

/// Language information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageInfo {
    pub name: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub mimetype: Option<String>,
    #[serde(default)]
    pub file_extension: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Notebook cell
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cell {
    pub cell_type: CellType,
    #[serde(default)]
    pub metadata: CellMetadata,
    pub source: CellSource,
    #[serde(default)]
    pub outputs: Vec<Output>,
    #[serde(default)]
    pub execution_count: Option<i32>,
}

/// Cell type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CellType {
    Code,
    Markdown,
    Raw,
}

/// Cell metadata
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CellMetadata {
    #[serde(default)]
    pub collapsed: bool,
    #[serde(default)]
    pub scrolled: bool,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Cell source (can be string or array of strings)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CellSource {
    String(String),
    Lines(Vec<String>),
}

impl CellSource {
    /// Convert to a single string
    pub fn to_string(&self) -> String {
        match self {
            CellSource::String(s) => s.clone(),
            CellSource::Lines(lines) => lines.join(""),
        }
    }

    /// Convert from string
    pub fn from_string(s: String) -> Self {
        CellSource::String(s)
    }
}

/// Cell output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Output {
    pub output_type: OutputType,
    #[serde(flatten)]
    pub data: OutputData,
}

/// Output type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OutputType {
    Stream,
    DisplayData,
    ExecuteResult,
    Error,
}

/// Output data (varies by output type)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OutputData {
    Stream {
        name: String,
        text: CellSource,
    },
    DisplayData {
        data: HashMap<String, serde_json::Value>,
        #[serde(default)]
        metadata: HashMap<String, serde_json::Value>,
    },
    ExecuteResult {
        execution_count: i32,
        data: HashMap<String, serde_json::Value>,
        #[serde(default)]
        metadata: HashMap<String, serde_json::Value>,
    },
    Error {
        ename: String,
        evalue: String,
        traceback: Vec<String>,
    },
}

impl Notebook {
    /// Load a notebook from a file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read notebook file: {}", path.as_ref().display()))?;
        Self::from_str(&content)
    }

    /// Parse a notebook from a JSON string
    pub fn from_str(content: &str) -> Result<Self> {
        serde_json::from_str(content)
            .context("Failed to parse notebook JSON")
    }

    /// Save the notebook to a file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = serde_json::to_string_pretty(self)
            .context("Failed to serialize notebook")?;
        fs::write(path.as_ref(), content)
            .with_context(|| format!("Failed to write notebook file: {}", path.as_ref().display()))?;
        Ok(())
    }

    /// Create a new empty notebook
    pub fn new(language: &str) -> Self {
        let kernelspec = match language {
            "python" => KernelSpec {
                display_name: "Python 3".to_string(),
                language: "python".to_string(),
                name: "python3".to_string(),
            },
            "javascript" => KernelSpec {
                display_name: "JavaScript".to_string(),
                language: "javascript".to_string(),
                name: "javascript".to_string(),
            },
            "julia" => KernelSpec {
                display_name: "Julia".to_string(),
                language: "julia".to_string(),
                name: "julia".to_string(),
            },
            _ => KernelSpec {
                display_name: language.to_string(),
                language: language.to_string(),
                name: language.to_string(),
            },
        };

        Notebook {
            cells: Vec::new(),
            metadata: NotebookMetadata {
                kernelspec: Some(kernelspec),
                language_info: None,
                extra: HashMap::new(),
            },
            nbformat: 4,
            nbformat_minor: 5,
        }
    }

    /// Add a code cell
    pub fn add_code_cell(&mut self, source: String) {
        self.cells.push(Cell {
            cell_type: CellType::Code,
            metadata: CellMetadata::default(),
            source: CellSource::from_string(source),
            outputs: Vec::new(),
            execution_count: None,
        });
    }

    /// Add a markdown cell
    pub fn add_markdown_cell(&mut self, source: String) {
        self.cells.push(Cell {
            cell_type: CellType::Markdown,
            metadata: CellMetadata::default(),
            source: CellSource::from_string(source),
            outputs: Vec::new(),
            execution_count: None,
        });
    }

    /// Get cell by index
    pub fn get_cell(&self, index: usize) -> Option<&Cell> {
        self.cells.get(index)
    }

    /// Get mutable cell by index
    pub fn get_cell_mut(&mut self, index: usize) -> Option<&mut Cell> {
        self.cells.get_mut(index)
    }

    /// Update cell source
    pub fn update_cell_source(&mut self, index: usize, source: String) -> Result<()> {
        let cell = self.cells.get_mut(index)
            .context("Cell index out of bounds")?;
        cell.source = CellSource::from_string(source);
        Ok(())
    }

    /// Clear cell outputs
    pub fn clear_cell_outputs(&mut self, index: usize) -> Result<()> {
        let cell = self.cells.get_mut(index)
            .context("Cell index out of bounds")?;
        cell.outputs.clear();
        cell.execution_count = None;
        Ok(())
    }

    /// Clear all outputs
    pub fn clear_all_outputs(&mut self) {
        for cell in &mut self.cells {
            cell.outputs.clear();
            cell.execution_count = None;
        }
    }
}

impl Cell {
    /// Check if this is a code cell
    pub fn is_code(&self) -> bool {
        self.cell_type == CellType::Code
    }

    /// Check if this is a markdown cell
    pub fn is_markdown(&self) -> bool {
        self.cell_type == CellType::Markdown
    }

    /// Get the source as a string
    pub fn get_source(&self) -> String {
        self.source.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_notebook() {
        let mut notebook = Notebook::new("python");
        assert_eq!(notebook.nbformat, 4);
        assert_eq!(notebook.cells.len(), 0);

        notebook.add_code_cell("print('Hello, World!')".to_string());
        notebook.add_markdown_cell("# Test Notebook".to_string());

        assert_eq!(notebook.cells.len(), 2);
        assert!(notebook.cells[0].is_code());
        assert!(notebook.cells[1].is_markdown());
    }

    #[test]
    fn test_cell_source() {
        let source_str = CellSource::String("test".to_string());
        let source_lines = CellSource::Lines(vec!["line1\n".to_string(), "line2\n".to_string()]);

        assert_eq!(source_str.to_string(), "test");
        assert_eq!(source_lines.to_string(), "line1\nline2\n");
    }
}

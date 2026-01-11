//! Jupyter Kernel management and communication
//!
//! This module handles:
//! - Starting and stopping Jupyter kernels
//! - Sending execution requests to kernels
//! - Receiving execution results
//! - Managing kernel state

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

/// Kernel manager
pub struct KernelManager {
    kernels: Arc<RwLock<HashMap<String, Kernel>>>,
}

/// Individual kernel instance
pub struct Kernel {
    pub id: String,
    pub kernel_name: String,
    pub status: KernelStatus,
    pub process: Option<Child>,
    pub connection_info: Option<ConnectionInfo>,
}

/// Kernel status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum KernelStatus {
    Starting,
    Idle,
    Busy,
    Dead,
    Error,
}

/// Kernel connection information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    pub shell_port: u16,
    pub iopub_port: u16,
    pub stdin_port: u16,
    pub control_port: u16,
    pub hb_port: u16,
    pub ip: String,
    pub key: String,
    pub transport: String,
    pub signature_scheme: String,
}

/// Execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub status: ExecutionStatus,
    pub execution_count: Option<i32>,
    pub outputs: Vec<ExecutionOutput>,
    pub error: Option<ExecutionError>,
}

/// Execution status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionStatus {
    Ok,
    Error,
    Abort,
}

/// Execution output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionOutput {
    pub output_type: String,
    pub data: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Execution error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionError {
    pub ename: String,
    pub evalue: String,
    pub traceback: Vec<String>,
}

impl KernelManager {
    /// Create a new kernel manager
    pub fn new() -> Self {
        Self {
            kernels: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start a new kernel
    pub async fn start_kernel(&self, kernel_name: &str) -> Result<String> {
        let kernel_id = Uuid::new_v4().to_string();

        tracing::info!("Starting kernel: {} ({})", kernel_name, kernel_id);

        // For now, we'll use a simple approach: spawn a Python kernel process
        // In a production environment, you'd use the Jupyter kernel protocol
        let kernel = match kernel_name {
            "python3" | "python" => self.start_python_kernel(&kernel_id).await?,
            _ => return Err(anyhow!("Unsupported kernel: {}", kernel_name)),
        };

        self.kernels.write().await.insert(kernel_id.clone(), kernel);

        Ok(kernel_id)
    }

    /// Start a Python kernel
    async fn start_python_kernel(&self, kernel_id: &str) -> Result<Kernel> {
        // Check if Python is available
        let python_check = Command::new("python3")
            .arg("--version")
            .output()
            .or_else(|_| Command::new("python").arg("--version").output())
            .context("Python not found. Please install Python to use Jupyter notebooks.")?;

        if !python_check.status.success() {
            return Err(anyhow!("Python is not working correctly"));
        }

        // For a simple implementation, we'll execute code directly via Python
        // In production, you'd use jupyter kernel protocol via ZMQ
        let kernel = Kernel {
            id: kernel_id.to_string(),
            kernel_name: "python3".to_string(),
            status: KernelStatus::Idle,
            process: None,
            connection_info: None,
        };

        Ok(kernel)
    }

    /// Stop a kernel
    pub async fn stop_kernel(&self, kernel_id: &str) -> Result<()> {
        tracing::info!("Stopping kernel: {}", kernel_id);

        let mut kernels = self.kernels.write().await;

        if let Some(mut kernel) = kernels.remove(kernel_id) {
            kernel.status = KernelStatus::Dead;

            // Kill the process if it exists
            if let Some(mut process) = kernel.process {
                let _ = process.kill();
            }
        }

        Ok(())
    }

    /// Restart a kernel
    pub async fn restart_kernel(&self, kernel_id: &str) -> Result<()> {
        tracing::info!("Restarting kernel: {}", kernel_id);

        let kernel_name = {
            let kernels = self.kernels.read().await;
            kernels.get(kernel_id)
                .map(|k| k.kernel_name.clone())
                .ok_or_else(|| anyhow!("Kernel not found: {}", kernel_id))?
        };

        self.stop_kernel(kernel_id).await?;

        // Use the same ID for consistency
        let mut kernel = self.start_python_kernel(kernel_id).await?;
        kernel.kernel_name = kernel_name;

        self.kernels.write().await.insert(kernel_id.to_string(), kernel);

        Ok(())
    }

    /// Get kernel status
    pub async fn get_status(&self, kernel_id: &str) -> Result<KernelStatus> {
        let kernels = self.kernels.read().await;
        kernels.get(kernel_id)
            .map(|k| k.status.clone())
            .ok_or_else(|| anyhow!("Kernel not found: {}", kernel_id))
    }

    /// Execute code in a kernel
    pub async fn execute(
        &self,
        kernel_id: &str,
        code: &str,
        _silent: bool,
    ) -> Result<ExecutionResult> {
        tracing::debug!("Executing code in kernel {}: {}", kernel_id, code);

        // Update kernel status to busy
        {
            let mut kernels = self.kernels.write().await;
            if let Some(kernel) = kernels.get_mut(kernel_id) {
                kernel.status = KernelStatus::Busy;
            } else {
                return Err(anyhow!("Kernel not found: {}", kernel_id));
            }
        }

        // Execute the code using Python
        let result = self.execute_python_code(code).await;

        // Update kernel status back to idle
        {
            let mut kernels = self.kernels.write().await;
            if let Some(kernel) = kernels.get_mut(kernel_id) {
                kernel.status = KernelStatus::Idle;
            }
        }

        result
    }

    /// Execute Python code (simple implementation)
    async fn execute_python_code(&self, code: &str) -> Result<ExecutionResult> {
        // Create a temporary Python script
        let script = format!(
            r#"
import sys
import json
import traceback
from io import StringIO

# Capture stdout
old_stdout = sys.stdout
sys.stdout = mystdout = StringIO()

# Capture stderr
old_stderr = sys.stderr
sys.stderr = mystderr = StringIO()

result = {{"status": "ok", "outputs": []}}

try:
    # Execute the code
    exec("""{}""")

    # Get stdout
    output = mystdout.getvalue()
    if output:
        result["outputs"].append({{
            "output_type": "stream",
            "data": {{"text/plain": output}}
        }})

except Exception as e:
    result["status"] = "error"
    result["error"] = {{
        "ename": type(e).__name__,
        "evalue": str(e),
        "traceback": traceback.format_exc().split('\n')
    }}

finally:
    # Restore stdout/stderr
    sys.stdout = old_stdout
    sys.stderr = old_stderr

print(json.dumps(result))
"#,
            code.replace("\"", "\\\"").replace("\n", "\\n")
        );

        // Execute Python script
        let output = Command::new("python3")
            .arg("-c")
            .arg(&script)
            .output()
            .or_else(|_| Command::new("python").arg("-c").arg(&script).output())
            .context("Failed to execute Python code")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Ok(ExecutionResult {
                status: ExecutionStatus::Error,
                execution_count: None,
                outputs: Vec::new(),
                error: Some(ExecutionError {
                    ename: "PythonError".to_string(),
                    evalue: "Failed to execute code".to_string(),
                    traceback: stderr.lines().map(|s| s.to_string()).collect(),
                }),
            });
        }

        // Parse the result
        let stdout = String::from_utf8_lossy(&output.stdout);
        let result: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| {
                serde_json::json!({
                    "status": "ok",
                    "outputs": [{
                        "output_type": "stream",
                        "data": {"text/plain": stdout.to_string()}
                    }]
                })
            });

        let status = if result["status"] == "error" {
            ExecutionStatus::Error
        } else {
            ExecutionStatus::Ok
        };

        let outputs: Vec<ExecutionOutput> = result["outputs"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|o| serde_json::from_value(o.clone()).ok())
            .collect();

        let error = result["error"]
            .as_object()
            .and_then(|e| serde_json::from_value(serde_json::Value::Object(e.clone())).ok());

        Ok(ExecutionResult {
            status,
            execution_count: None,
            outputs,
            error,
        })
    }

    /// List all kernels
    pub async fn list_kernels(&self) -> Vec<KernelInfo> {
        let kernels = self.kernels.read().await;
        kernels
            .values()
            .map(|k| KernelInfo {
                id: k.id.clone(),
                name: k.kernel_name.clone(),
                status: k.status.clone(),
            })
            .collect()
    }

    /// Interrupt a kernel
    pub async fn interrupt_kernel(&self, kernel_id: &str) -> Result<()> {
        tracing::info!("Interrupting kernel: {}", kernel_id);

        let mut kernels = self.kernels.write().await;

        if let Some(kernel) = kernels.get_mut(kernel_id) {
            // In a full implementation, this would send an interrupt signal
            kernel.status = KernelStatus::Idle;
            Ok(())
        } else {
            Err(anyhow!("Kernel not found: {}", kernel_id))
        }
    }
}

/// Kernel information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelInfo {
    pub id: String,
    pub name: String,
    pub status: KernelStatus,
}

impl Default for KernelManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_kernel_manager() {
        let manager = KernelManager::new();

        // Start a kernel
        let kernel_id = manager.start_kernel("python3").await.unwrap();

        // Check status
        let status = manager.get_status(&kernel_id).await.unwrap();
        assert_eq!(status, KernelStatus::Idle);

        // Execute code
        let result = manager.execute(&kernel_id, "print('Hello, World!')", false).await.unwrap();
        assert_eq!(result.status, ExecutionStatus::Ok);

        // Stop kernel
        manager.stop_kernel(&kernel_id).await.unwrap();
    }
}

//! Model Context Protocol (MCP) integration
//!
//! This module provides basic MCP server integration, allowing BerryCode to
//! connect to MCP servers and use their tools dynamically.

use serde::{Deserialize, Serialize};
use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader, Write};
use anyhow::{Result, anyhow};

/// MCP tool definition from server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// MCP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: Option<std::collections::HashMap<String, String>>,
}

/// MCP request message
#[derive(Debug, Serialize)]
struct McpRequest {
    jsonrpc: String,
    id: u64,
    method: String,
    params: serde_json::Value,
}

/// MCP response message
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct McpResponse {
    jsonrpc: String,
    id: u64,
    #[serde(default)]
    result: Option<serde_json::Value>,
    #[serde(default)]
    error: Option<serde_json::Value>,
}

/// MCP server client
pub struct McpClient {
    config: McpServerConfig,
    request_id: u64,
}

impl McpClient {
    pub fn new(config: McpServerConfig) -> Self {
        Self {
            config,
            request_id: 0,
        }
    }

    /// List tools available from the MCP server
    pub fn list_tools(&mut self) -> Result<Vec<McpTool>> {
        self.request_id += 1;
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: self.request_id,
            method: "tools/list".to_string(),
            params: serde_json::json!({}),
        };

        let response = self.send_request(&request)?;

        if let Some(error) = response.error {
            return Err(anyhow!("MCP server error: {}", error));
        }

        let tools = response.result
            .and_then(|r| r.get("tools").cloned())
            .and_then(|t| serde_json::from_value(t).ok())
            .unwrap_or_default();

        Ok(tools)
    }

    /// Call a tool on the MCP server
    pub fn call_tool(&mut self, tool_name: &str, arguments: serde_json::Value) -> Result<String> {
        self.request_id += 1;
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: self.request_id,
            method: "tools/call".to_string(),
            params: serde_json::json!({
                "name": tool_name,
                "arguments": arguments
            }),
        };

        let response = self.send_request(&request)?;

        if let Some(error) = response.error {
            return Err(anyhow!("MCP tool execution error: {}", error));
        }

        let result = response.result
            .and_then(|r| r.get("content").cloned())
            .and_then(|c| c.as_array().and_then(|arr| arr.first().cloned()))
            .and_then(|item| item.get("text").and_then(|t| t.as_str().map(|s| s.to_string())))
            .unwrap_or_else(|| "No result from MCP server".to_string());

        Ok(result)
    }

    /// Send a request to the MCP server
    fn send_request(&self, request: &McpRequest) -> Result<McpResponse> {
        // Start MCP server process
        let mut child = Command::new(&self.config.command)
            .args(&self.config.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;

        // Send request
        let request_json = serde_json::to_string(request)?;
        if let Some(mut stdin) = child.stdin.take() {
            writeln!(stdin, "{}", request_json)?;
            stdin.flush()?;
        }

        // Read response
        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                let line = line?;
                if let Ok(response) = serde_json::from_str::<McpResponse>(&line) {
                    return Ok(response);
                }
            }
        }

        Err(anyhow!("No response from MCP server"))
    }
}

/// MCP server manager
pub struct McpManager {
    servers: Vec<McpServerConfig>,
}

impl McpManager {
    pub fn new() -> Self {
        Self {
            servers: Vec::new(),
        }
    }

    /// Load MCP servers from configuration
    pub fn load_from_config(&mut self, config_path: &std::path::Path) -> Result<()> {
        if !config_path.exists() {
            return Ok(());
        }

        let content = std::fs::read_to_string(config_path)?;
        let config: serde_json::Value = serde_json::from_str(&content)?;

        if let Some(servers) = config.get("mcpServers").and_then(|s| s.as_object()) {
            for (name, server_config) in servers {
                if let Some(command) = server_config.get("command").and_then(|c| c.as_str()) {
                    let args = server_config.get("args")
                        .and_then(|a| a.as_array())
                        .map(|arr| arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect())
                        .unwrap_or_default();

                    let env = server_config.get("env")
                        .and_then(|e| e.as_object())
                        .map(|obj| obj.iter()
                            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                            .collect());

                    self.servers.push(McpServerConfig {
                        name: name.clone(),
                        command: command.to_string(),
                        args,
                        env,
                    });
                }
            }
        }

        Ok(())
    }

    /// Get all available tools from all MCP servers
    pub fn get_all_tools(&self) -> Vec<(String, McpTool)> {
        let mut all_tools = Vec::new();

        for server_config in &self.servers {
            let mut client = McpClient::new(server_config.clone());
            if let Ok(tools) = client.list_tools() {
                for tool in tools {
                    all_tools.push((server_config.name.clone(), tool));
                }
            }
        }

        all_tools
    }

    /// Execute an MCP tool
    pub fn execute_tool(&self, server_name: &str, tool_name: &str, arguments: serde_json::Value) -> Result<String> {
        let server_config = self.servers.iter()
            .find(|s| s.name == server_name)
            .ok_or_else(|| anyhow!("MCP server '{}' not found", server_name))?;

        let mut client = McpClient::new(server_config.clone());
        client.call_tool(tool_name, arguments)
    }
}

impl Default for McpManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_manager_creation() {
        let manager = McpManager::new();
        assert_eq!(manager.servers.len(), 0);
    }
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DbType {
    PostgreSQL,
    MySQL,
    SQLite,
    MongoDB,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbConnection {
    pub id: String,   // UUID
    pub name: String, // User-friendly name
    pub db_type: DbType,
    pub host: Option<String>, // None for SQLite
    pub port: Option<u16>,    // None for SQLite
    pub database: String,     // DB name or file path
    pub username: Option<String>,
    pub password: Option<String>, // TODO: Encrypt in storage
    pub ssl: bool,
    pub created_at: i64, // Unix timestamp
    pub last_used: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionTestResult {
    pub success: bool,
    pub message: String,
    pub latency_ms: Option<u64>,
    pub server_version: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DbConnectionList {
    pub connections: Vec<DbConnection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub columns: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rows: Option<Vec<std::collections::HashMap<String, serde_json::Value>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub affected_rows: Option<usize>,
}

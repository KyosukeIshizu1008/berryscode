//! Database client for BerryCode
//! Supports SQLite, PostgreSQL, and MySQL connections

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Database type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DatabaseType {
    SQLite,
    PostgreSQL,
    MySQL,
}

impl DatabaseType {
    pub fn as_str(&self) -> &str {
        match self {
            DatabaseType::SQLite => "SQLite",
            DatabaseType::PostgreSQL => "PostgreSQL",
            DatabaseType::MySQL => "MySQL",
        }
    }
}

/// Database connection info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConnection {
    pub id: String,
    pub name: String,
    pub db_type: DatabaseType,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub database: String,
    pub username: Option<String>,
    pub password: Option<String>,
}

impl DatabaseConnection {
    pub fn new_sqlite(name: String, path: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            db_type: DatabaseType::SQLite,
            host: None,
            port: None,
            database: path,
            username: None,
            password: None,
        }
    }

    pub fn new_postgresql(
        name: String,
        host: String,
        port: u16,
        database: String,
        username: String,
        password: String,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            db_type: DatabaseType::PostgreSQL,
            host: Some(host),
            port: Some(port),
            database,
            username: Some(username),
            password: Some(password),
        }
    }

    pub fn new_mysql(
        name: String,
        host: String,
        port: u16,
        database: String,
        username: String,
        password: String,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            db_type: DatabaseType::MySQL,
            host: Some(host),
            port: Some(port),
            database,
            username: Some(username),
            password: Some(password),
        }
    }
}

/// Query result row
#[derive(Debug, Clone)]
pub struct QueryRow {
    pub values: Vec<String>,
}

/// Query result
#[derive(Debug, Clone)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<QueryRow>,
    pub rows_affected: usize,
}

impl QueryResult {
    pub fn empty() -> Self {
        Self {
            columns: Vec::new(),
            rows: Vec::new(),
            rows_affected: 0,
        }
    }
}

/// Database client
pub struct DatabaseClient {
    connections: Vec<DatabaseConnection>,
}

impl DatabaseClient {
    pub fn new() -> Self {
        Self {
            connections: Vec::new(),
        }
    }

    pub fn add_connection(&mut self, conn: DatabaseConnection) {
        self.connections.push(conn);
    }

    pub fn remove_connection(&mut self, id: &str) {
        self.connections.retain(|c| c.id != id);
    }

    pub fn get_connections(&self) -> &[DatabaseConnection] {
        &self.connections
    }

    pub fn get_connection(&self, id: &str) -> Option<&DatabaseConnection> {
        self.connections.iter().find(|c| c.id == id)
    }

    /// Execute a SQL query (simplified - returns mock data for now)
    pub fn execute_query(&self, _connection_id: &str, query: &str) -> Result<QueryResult> {
        // TODO: Implement actual database connections
        // For now, return mock data based on query type

        let query_lower = query.trim().to_lowercase();

        if query_lower.starts_with("select") {
            // Mock SELECT result
            Ok(QueryResult {
                columns: vec!["id".to_string(), "name".to_string(), "value".to_string()],
                rows: vec![
                    QueryRow {
                        values: vec!["1".to_string(), "Sample".to_string(), "123".to_string()],
                    },
                    QueryRow {
                        values: vec!["2".to_string(), "Test".to_string(), "456".to_string()],
                    },
                ],
                rows_affected: 0,
            })
        } else if query_lower.starts_with("insert")
            || query_lower.starts_with("update")
            || query_lower.starts_with("delete")
        {
            // Mock INSERT/UPDATE/DELETE result
            Ok(QueryResult {
                columns: Vec::new(),
                rows: Vec::new(),
                rows_affected: 1,
            })
        } else {
            Ok(QueryResult::empty())
        }
    }

    /// Get list of tables in a database
    pub fn get_tables(&self, connection_id: &str) -> Result<Vec<String>> {
        let conn = self
            .get_connection(connection_id)
            .context("Connection not found")?;

        // TODO: Implement actual table listing
        // For now, return mock tables
        Ok(vec![
            format!("{}_users", conn.name),
            format!("{}_posts", conn.name),
            format!("{}_comments", conn.name),
        ])
    }

    /// Test database connection
    pub fn test_connection(&self, _connection_id: &str) -> Result<bool> {
        // TODO: Implement actual connection test
        Ok(true)
    }
}

impl Default for DatabaseClient {
    fn default() -> Self {
        Self::new()
    }
}

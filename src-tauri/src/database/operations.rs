use super::types::*;
use anyhow::Result;
use std::fs;
use std::path::PathBuf;
use tauri::Manager;

pub struct DatabaseManager {
    config_path: PathBuf,
}

impl DatabaseManager {
    pub fn new(app_handle: &tauri::AppHandle) -> Result<Self> {
        // Use Tauri's app_config_dir for persistent storage
        let config_dir = app_handle
            .path()
            .app_config_dir()
            .expect("Failed to get app config dir");

        fs::create_dir_all(&config_dir)?;
        let config_path = config_dir.join("db_connections.json");

        Ok(Self { config_path })
    }

    pub fn load_connections(&self) -> Result<Vec<DbConnection>> {
        if !self.config_path.exists() {
            return Ok(Vec::new());
        }

        let data = fs::read_to_string(&self.config_path)?;
        let list: DbConnectionList = serde_json::from_str(&data)?;

        // TODO: Decrypt passwords
        Ok(list.connections)
    }

    pub fn save_connections(&self, connections: &[DbConnection]) -> Result<()> {
        // TODO: Encrypt passwords before saving
        let list = DbConnectionList {
            connections: connections.to_vec(),
        };

        let data = serde_json::to_string_pretty(&list)?;
        fs::write(&self.config_path, data)?;
        Ok(())
    }
}

// Standalone async function - doesn't need self
pub async fn test_connection(conn: &DbConnection) -> Result<ConnectionTestResult> {
    use std::time::Instant;
    let start = Instant::now();

    match conn.db_type {
        DbType::PostgreSQL => test_postgres_connection(conn, start).await,
        DbType::MySQL => test_mysql_connection(conn, start).await,
        DbType::SQLite => test_sqlite_connection(conn, start).await,
        DbType::MongoDB => Ok(ConnectionTestResult {
            success: false,
            message: "MongoDB support coming soon".to_string(),
            latency_ms: Some(start.elapsed().as_millis() as u64),
            server_version: None,
        }),
    }
}

async fn test_postgres_connection(
    conn: &DbConnection,
    start: std::time::Instant,
) -> Result<ConnectionTestResult> {
    use tokio_postgres::NoTls;

    let host = conn.host.as_deref().unwrap_or("localhost");
    let port = conn.port.unwrap_or(5432);
    let database = &conn.database;
    let username = conn.username.as_deref().unwrap_or("postgres");
    let password = conn.password.as_deref().unwrap_or("");

    let connection_string = if conn.ssl {
        format!(
            "host={} port={} dbname={} user={} password={} sslmode=require",
            host, port, database, username, password
        )
    } else {
        format!(
            "host={} port={} dbname={} user={} password={}",
            host, port, database, username, password
        )
    };

    match tokio_postgres::connect(&connection_string, NoTls).await {
        Ok((client, connection)) => {
            // Spawn connection handler
            tokio::spawn(async move {
                if let Err(e) = connection.await {
                    eprintln!("PostgreSQL connection error: {}", e);
                }
            });

            // Test query to get version
            match client.query_one("SELECT version()", &[]).await {
                Ok(row) => {
                    let version: String = row.get(0);
                    let latency = start.elapsed().as_millis() as u64;
                    Ok(ConnectionTestResult {
                        success: true,
                        message: "Successfully connected to PostgreSQL".to_string(),
                        latency_ms: Some(latency),
                        server_version: Some(version),
                    })
                }
                Err(e) => Ok(ConnectionTestResult {
                    success: false,
                    message: format!("PostgreSQL test query failed: {}", e),
                    latency_ms: Some(start.elapsed().as_millis() as u64),
                    server_version: None,
                }),
            }
        }
        Err(e) => Ok(ConnectionTestResult {
            success: false,
            message: format!("PostgreSQL connection failed: {}", e),
            latency_ms: Some(start.elapsed().as_millis() as u64),
            server_version: None,
        }),
    }
}

async fn test_mysql_connection(
    conn: &DbConnection,
    start: std::time::Instant,
) -> Result<ConnectionTestResult> {
    use mysql_async::prelude::*;

    let host = conn.host.as_deref().unwrap_or("localhost");
    let port = conn.port.unwrap_or(3306);
    let database = &conn.database;
    let username = conn.username.as_deref().unwrap_or("root");
    let password = conn.password.as_deref().unwrap_or("");

    let connection_string = format!(
        "mysql://{}:{}@{}:{}/{}",
        username, password, host, port, database
    );

    let opts = match mysql_async::Opts::from_url(&connection_string) {
        Ok(opts) => opts,
        Err(e) => {
            return Ok(ConnectionTestResult {
                success: false,
                message: format!("Invalid MySQL connection string: {}", e),
                latency_ms: Some(start.elapsed().as_millis() as u64),
                server_version: None,
            })
        }
    };

    match mysql_async::Conn::new(opts).await {
        Ok(mut conn_pool) => match conn_pool.query_first::<String, _>("SELECT VERSION()").await {
            Ok(Some(version)) => {
                conn_pool.disconnect().await.ok();
                let latency = start.elapsed().as_millis() as u64;
                Ok(ConnectionTestResult {
                    success: true,
                    message: "Successfully connected to MySQL".to_string(),
                    latency_ms: Some(latency),
                    server_version: Some(version),
                })
            }
            Ok(None) => {
                conn_pool.disconnect().await.ok();
                Ok(ConnectionTestResult {
                    success: false,
                    message: "MySQL connection succeeded but version query returned no result"
                        .to_string(),
                    latency_ms: Some(start.elapsed().as_millis() as u64),
                    server_version: None,
                })
            }
            Err(e) => {
                conn_pool.disconnect().await.ok();
                Ok(ConnectionTestResult {
                    success: false,
                    message: format!("MySQL test query failed: {}", e),
                    latency_ms: Some(start.elapsed().as_millis() as u64),
                    server_version: None,
                })
            }
        },
        Err(e) => Ok(ConnectionTestResult {
            success: false,
            message: format!("MySQL connection failed: {}", e),
            latency_ms: Some(start.elapsed().as_millis() as u64),
            server_version: None,
        }),
    }
}

async fn test_sqlite_connection(
    conn: &DbConnection,
    start: std::time::Instant,
) -> Result<ConnectionTestResult> {
    use rusqlite::Connection;

    // For SQLite, database field contains the file path
    let db_path = &conn.database;

    match Connection::open(db_path) {
        Ok(conn) => {
            match conn.query_row("SELECT sqlite_version()", [], |row| row.get::<_, String>(0)) {
                Ok(version) => {
                    let latency = start.elapsed().as_millis() as u64;
                    Ok(ConnectionTestResult {
                        success: true,
                        message: "Successfully connected to SQLite".to_string(),
                        latency_ms: Some(latency),
                        server_version: Some(format!("SQLite {}", version)),
                    })
                }
                Err(e) => Ok(ConnectionTestResult {
                    success: false,
                    message: format!("SQLite test query failed: {}", e),
                    latency_ms: Some(start.elapsed().as_millis() as u64),
                    server_version: None,
                }),
            }
        }
        Err(e) => Ok(ConnectionTestResult {
            success: false,
            message: format!("SQLite connection failed: {}", e),
            latency_ms: Some(start.elapsed().as_millis() as u64),
            server_version: None,
        }),
    }
}

// ============================================================================
// SQL Execution Functions
// ============================================================================

pub async fn execute_query(conn: &DbConnection, query: String) -> Result<QueryResult> {
    match conn.db_type {
        DbType::PostgreSQL => execute_postgres_query(conn, &query).await,
        DbType::MySQL => execute_mysql_query(conn, &query).await,
        DbType::SQLite => execute_sqlite_query(conn, &query).await,
        DbType::MongoDB => Err(anyhow::anyhow!("MongoDB support coming soon")),
    }
}

async fn execute_postgres_query(conn: &DbConnection, query: &str) -> Result<QueryResult> {
    use tokio_postgres::NoTls;

    let host = conn.host.as_deref().unwrap_or("localhost");
    let port = conn.port.unwrap_or(5432);
    let database = &conn.database;
    let username = conn.username.as_deref().unwrap_or("postgres");
    let password = conn.password.as_deref().unwrap_or("");

    let connection_string = format!(
        "host={} port={} dbname={} user={} password={}",
        host, port, database, username, password
    );

    let (client, connection) = tokio_postgres::connect(&connection_string, NoTls).await?;

    // Spawn connection handler
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("PostgreSQL connection error: {}", e);
        }
    });

    let trimmed_query = query.trim().to_uppercase();
    if trimmed_query.starts_with("SELECT") || trimmed_query.starts_with("EXPLAIN") {
        let rows = client.query(query, &[]).await?;

        let mut columns = Vec::new();
        let mut result_rows = Vec::new();

        if let Some(first_row) = rows.first() {
            columns = first_row
                .columns()
                .iter()
                .map(|col| col.name().to_string())
                .collect();
        }

        for row in rows {
            let mut row_map = std::collections::HashMap::new();
            for (idx, column) in row.columns().iter().enumerate() {
                let value: serde_json::Value = if let Ok(s) = row.try_get::<_, String>(idx) {
                    serde_json::Value::String(s)
                } else if let Ok(i) = row.try_get::<_, i64>(idx) {
                    serde_json::Value::Number(i.into())
                } else if let Ok(f) = row.try_get::<_, f64>(idx) {
                    serde_json::Number::from_f64(f)
                        .map(serde_json::Value::Number)
                        .unwrap_or(serde_json::Value::Null)
                } else if let Ok(b) = row.try_get::<_, bool>(idx) {
                    serde_json::Value::Bool(b)
                } else {
                    serde_json::Value::Null
                };

                row_map.insert(column.name().to_string(), value);
            }
            result_rows.push(row_map);
        }

        Ok(QueryResult {
            columns: Some(columns),
            rows: Some(result_rows),
            affected_rows: None,
        })
    } else {
        let result = client.execute(query, &[]).await?;

        Ok(QueryResult {
            columns: None,
            rows: None,
            affected_rows: Some(result as usize),
        })
    }
}

async fn execute_mysql_query(conn: &DbConnection, query: &str) -> Result<QueryResult> {
    use mysql_async::prelude::*;

    let host = conn.host.as_deref().unwrap_or("localhost");
    let port = conn.port.unwrap_or(3306);
    let database = &conn.database;
    let username = conn.username.as_deref().unwrap_or("root");
    let password = conn.password.as_deref().unwrap_or("");

    let connection_string = format!(
        "mysql://{}:{}@{}:{}/{}",
        username, password, host, port, database
    );

    let opts = mysql_async::Opts::from_url(&connection_string)?;
    let mut conn_pool = mysql_async::Conn::new(opts).await?;

    let trimmed_query = query.trim().to_uppercase();
    if trimmed_query.starts_with("SELECT") || trimmed_query.starts_with("EXPLAIN") {
        let result: Vec<mysql_async::Row> = conn_pool.query(query).await?;

        let mut columns = Vec::new();
        let mut result_rows = Vec::new();

        if let Some(first_row) = result.first() {
            columns = first_row
                .columns_ref()
                .iter()
                .map(|col| col.name_str().to_string())
                .collect();
        }

        for row in result {
            let mut row_map = std::collections::HashMap::new();
            for (idx, column) in row.columns_ref().iter().enumerate() {
                let value: serde_json::Value = if let Some(s) = row.get::<String, _>(idx) {
                    serde_json::Value::String(s)
                } else if let Some(i) = row.get::<i64, _>(idx) {
                    serde_json::Value::Number(serde_json::Number::from(i))
                } else if let Some(f) = row.get::<f64, _>(idx) {
                    serde_json::Number::from_f64(f)
                        .map(serde_json::Value::Number)
                        .unwrap_or(serde_json::Value::Null)
                } else if let Some(b) = row.get::<bool, _>(idx) {
                    serde_json::Value::Bool(b)
                } else {
                    serde_json::Value::Null
                };

                row_map.insert(column.name_str().to_string(), value);
            }
            result_rows.push(row_map);
        }

        conn_pool.disconnect().await?;

        Ok(QueryResult {
            columns: Some(columns),
            rows: Some(result_rows),
            affected_rows: None,
        })
    } else {
        let result = conn_pool.exec_drop(query, ()).await?;
        conn_pool.disconnect().await?;

        Ok(QueryResult {
            columns: None,
            rows: None,
            affected_rows: Some(0), // MySQL doesn't return affected rows easily
        })
    }
}

async fn execute_sqlite_query(conn: &DbConnection, query: &str) -> Result<QueryResult> {
    use rusqlite::Connection;

    let db_path = &conn.database;
    let connection = Connection::open(db_path)?;

    let trimmed_query = query.trim().to_uppercase();
    if trimmed_query.starts_with("SELECT")
        || trimmed_query.starts_with("EXPLAIN")
        || trimmed_query.starts_with("PRAGMA")
    {
        let mut stmt = connection.prepare(query)?;

        let column_names: Vec<String> = stmt
            .column_names()
            .into_iter()
            .map(|s| s.to_string())
            .collect();

        let rows = stmt.query_map([], |row| {
            let mut row_map = std::collections::HashMap::new();
            for (idx, col_name) in column_names.iter().enumerate() {
                let value: serde_json::Value = if let Ok(s) = row.get::<_, String>(idx) {
                    serde_json::Value::String(s)
                } else if let Ok(i) = row.get::<_, i64>(idx) {
                    serde_json::Value::Number(i.into())
                } else if let Ok(f) = row.get::<_, f64>(idx) {
                    serde_json::Number::from_f64(f)
                        .map(serde_json::Value::Number)
                        .unwrap_or(serde_json::Value::Null)
                } else {
                    serde_json::Value::Null
                };

                row_map.insert(col_name.clone(), value);
            }
            Ok(row_map)
        })?;

        let mut result_rows = Vec::new();
        for row in rows {
            result_rows.push(row?);
        }

        Ok(QueryResult {
            columns: Some(column_names),
            rows: Some(result_rows),
            affected_rows: None,
        })
    } else {
        let affected = connection.execute(query, [])?;

        Ok(QueryResult {
            columns: None,
            rows: None,
            affected_rows: Some(affected),
        })
    }
}

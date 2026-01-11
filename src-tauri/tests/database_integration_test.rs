// Database Integration Tests
// Tests migrated from ../tests (parent directory) database_api.rs tests

use tempfile::NamedTempFile;

#[cfg(test)]
mod database_tests {
    use super::*;

    #[tokio::test]
    async fn test_sqlite_connection() {
        // Create temporary SQLite database
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_str().unwrap();

        // Create test database with rusqlite
        let conn = rusqlite::Connection::open(db_path).unwrap();
        conn.execute(
            "CREATE TABLE test_table (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO test_table (id, name) VALUES (1, 'Test')",
            [],
        )
        .unwrap();
        drop(conn);

        // Test connection with our DatabaseManager
        let test_conn = berry_editor_tauri::database::DbConnection {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Test SQLite".to_string(),
            db_type: berry_editor_tauri::database::DbType::SQLite,
            host: None,
            port: None,
            database: db_path.to_string(),
            username: None,
            password: None,
            ssl: false,
            created_at: chrono::Utc::now().timestamp(),
            last_used: None,
        };

        let result = berry_editor_tauri::database::operations::test_connection(&test_conn)
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.message.contains("Successfully connected"));
        assert!(result.server_version.is_some());
    }

    #[tokio::test]
    async fn test_sqlite_query_execution() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_str().unwrap();

        // Create test database
        let conn = rusqlite::Connection::open(db_path).unwrap();
        conn.execute(
            "CREATE TABLE products (id INTEGER PRIMARY KEY, name TEXT, price REAL)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO products (name, price) VALUES ('Product A', 9.99)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO products (name, price) VALUES ('Product B', 19.99)",
            [],
        )
        .unwrap();
        drop(conn);

        let test_conn = berry_editor_tauri::database::DbConnection {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Test SQLite".to_string(),
            db_type: berry_editor_tauri::database::DbType::SQLite,
            host: None,
            port: None,
            database: db_path.to_string(),
            username: None,
            password: None,
            ssl: false,
            created_at: chrono::Utc::now().timestamp(),
            last_used: None,
        };

        // Test SELECT query
        let query_result = berry_editor_tauri::database::operations::execute_query(
            &test_conn,
            "SELECT * FROM products ORDER BY id".to_string(),
        )
        .await
        .unwrap();

        assert!(query_result.columns.is_some());
        assert!(query_result.rows.is_some());
        assert_eq!(query_result.rows.as_ref().unwrap().len(), 2);
        assert_eq!(query_result.affected_rows, None);

        // Verify column names
        let columns = query_result.columns.unwrap();
        assert!(columns.contains(&"id".to_string()));
        assert!(columns.contains(&"name".to_string()));
        assert!(columns.contains(&"price".to_string()));
    }

    #[tokio::test]
    async fn test_sqlite_insert_query() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_str().unwrap();

        // Create test database
        let conn = rusqlite::Connection::open(db_path).unwrap();
        conn.execute(
            "CREATE TABLE test_insert (id INTEGER PRIMARY KEY, value TEXT)",
            [],
        )
        .unwrap();
        drop(conn);

        let test_conn = berry_editor_tauri::database::DbConnection {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Test SQLite".to_string(),
            db_type: berry_editor_tauri::database::DbType::SQLite,
            host: None,
            port: None,
            database: db_path.to_string(),
            username: None,
            password: None,
            ssl: false,
            created_at: chrono::Utc::now().timestamp(),
            last_used: None,
        };

        // Test INSERT query
        let insert_result = berry_editor_tauri::database::operations::execute_query(
            &test_conn,
            "INSERT INTO test_insert (value) VALUES ('hello')".to_string(),
        )
        .await
        .unwrap();

        assert!(insert_result.affected_rows.is_some());
        assert_eq!(insert_result.affected_rows.unwrap(), 1);
        assert!(insert_result.columns.is_none());
        assert!(insert_result.rows.is_none());
    }

    #[tokio::test]
    async fn test_query_error_handling() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_str().unwrap();

        // Create empty database
        let conn = rusqlite::Connection::open(db_path).unwrap();
        drop(conn);

        let test_conn = berry_editor_tauri::database::DbConnection {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Test SQLite".to_string(),
            db_type: berry_editor_tauri::database::DbType::SQLite,
            host: None,
            port: None,
            database: db_path.to_string(),
            username: None,
            password: None,
            ssl: false,
            created_at: chrono::Utc::now().timestamp(),
            last_used: None,
        };

        // Test invalid SQL query
        let result = berry_editor_tauri::database::operations::execute_query(
            &test_conn,
            "SELECT * FROM nonexistent_table".to_string(),
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_connection_latency_measurement() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_str().unwrap();

        let conn = rusqlite::Connection::open(db_path).unwrap();
        drop(conn);

        let test_conn = berry_editor_tauri::database::DbConnection {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Test SQLite".to_string(),
            db_type: berry_editor_tauri::database::DbType::SQLite,
            host: None,
            port: None,
            database: db_path.to_string(),
            username: None,
            password: None,
            ssl: false,
            created_at: chrono::Utc::now().timestamp(),
            last_used: None,
        };

        let result = berry_editor_tauri::database::operations::test_connection(&test_conn)
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.latency_ms.is_some());
        assert!(result.latency_ms.unwrap() >= 0);
    }

    #[test]
    fn test_db_type_serialization() {
        use berry_editor_tauri::database::DbType;

        let db_type = DbType::SQLite;
        let json = serde_json::to_string(&db_type).unwrap();
        assert_eq!(json, "\"SQLite\"");

        let deserialized: DbType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, DbType::SQLite);
    }

    #[test]
    fn test_connection_id_generation() {
        let id1 = uuid::Uuid::new_v4().to_string();
        let id2 = uuid::Uuid::new_v4().to_string();
        assert_ne!(id1, id2);
        assert_eq!(id1.len(), 36); // UUID format
    }

    #[tokio::test]
    async fn test_update_and_delete_operations() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_str().unwrap();

        let conn = rusqlite::Connection::open(db_path).unwrap();
        conn.execute(
            "CREATE TABLE accounts (id INTEGER PRIMARY KEY, balance REAL, name TEXT)",
            [],
        )
        .unwrap();
        conn.execute("INSERT INTO accounts (id, balance, name) VALUES (1, 100.0, 'Account A')", [])
            .unwrap();
        conn.execute("INSERT INTO accounts (id, balance, name) VALUES (2, 200.0, 'Account B')", [])
            .unwrap();
        drop(conn);

        let test_conn = berry_editor_tauri::database::DbConnection {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Test SQLite".to_string(),
            db_type: berry_editor_tauri::database::DbType::SQLite,
            host: None,
            port: None,
            database: db_path.to_string(),
            username: None,
            password: None,
            ssl: false,
            created_at: chrono::Utc::now().timestamp(),
            last_used: None,
        };

        // Test UPDATE
        let update_result = berry_editor_tauri::database::operations::execute_query(
            &test_conn,
            "UPDATE accounts SET balance = 250.0 WHERE id = 1".to_string(),
        )
        .await
        .unwrap();

        assert!(update_result.affected_rows.is_some());
        assert_eq!(update_result.affected_rows.unwrap(), 1);

        // Verify update
        let select_result = berry_editor_tauri::database::operations::execute_query(
            &test_conn,
            "SELECT balance FROM accounts WHERE id = 1".to_string(),
        )
        .await
        .unwrap();

        assert!(select_result.rows.is_some());
        let rows = select_result.rows.unwrap();
        assert_eq!(rows.len(), 1);

        let balance_value = &rows[0].get("balance").unwrap();
        if let serde_json::Value::Number(n) = balance_value {
            let balance = n.as_f64().unwrap();
            assert!((balance - 250.0).abs() < 0.01);
        } else {
            panic!("Expected number value for balance");
        }

        // Test DELETE
        let delete_result = berry_editor_tauri::database::operations::execute_query(
            &test_conn,
            "DELETE FROM accounts WHERE id = 2".to_string(),
        )
        .await
        .unwrap();

        assert!(delete_result.affected_rows.is_some());
        assert_eq!(delete_result.affected_rows.unwrap(), 1);

        // Verify delete
        let count_result = berry_editor_tauri::database::operations::execute_query(
            &test_conn,
            "SELECT COUNT(*) as count FROM accounts".to_string(),
        )
        .await
        .unwrap();

        let count_rows = count_result.rows.unwrap();
        let count_value = &count_rows[0].get("count").unwrap();
        if let serde_json::Value::Number(n) = count_value {
            assert_eq!(n.as_i64().unwrap(), 1);
        }
    }
}

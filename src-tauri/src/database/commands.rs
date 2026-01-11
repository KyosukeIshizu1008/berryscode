use super::{
    operations::{test_connection, DatabaseManager},
    types::*,
};
use std::sync::Mutex;
use tauri::State;

pub struct DbManager {
    manager: Mutex<DatabaseManager>,
}

impl DbManager {
    pub fn new(app_handle: &tauri::AppHandle) -> Self {
        Self {
            manager: Mutex::new(
                DatabaseManager::new(app_handle).expect("Failed to initialize DatabaseManager"),
            ),
        }
    }
}

#[tauri::command]
pub async fn db_list_connections(
    manager: State<'_, DbManager>,
) -> Result<Vec<DbConnection>, String> {
    manager
        .manager
        .lock()
        .unwrap()
        .load_connections()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn db_add_connection(
    connection: DbConnection,
    manager: State<'_, DbManager>,
) -> Result<(), String> {
    let mgr = manager.manager.lock().unwrap();
    let mut connections = mgr.load_connections().map_err(|e| e.to_string())?;

    connections.push(connection);
    mgr.save_connections(&connections)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn db_update_connection(
    connection: DbConnection,
    manager: State<'_, DbManager>,
) -> Result<(), String> {
    let mgr = manager.manager.lock().unwrap();
    let mut connections = mgr.load_connections().map_err(|e| e.to_string())?;

    if let Some(index) = connections.iter().position(|c| c.id == connection.id) {
        connections[index] = connection;
        mgr.save_connections(&connections)
            .map_err(|e| e.to_string())
    } else {
        Err("Connection not found".to_string())
    }
}

#[tauri::command]
pub async fn db_delete_connection(
    connection_id: String,
    manager: State<'_, DbManager>,
) -> Result<(), String> {
    let mgr = manager.manager.lock().unwrap();
    let mut connections = mgr.load_connections().map_err(|e| e.to_string())?;

    connections.retain(|c| c.id != connection_id);
    mgr.save_connections(&connections)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn db_test_connection(
    connection: DbConnection,
    _manager: State<'_, DbManager>,
) -> Result<ConnectionTestResult, String> {
    test_connection(&connection)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn db_execute_query(
    connection: DbConnection,
    query: String,
    _manager: State<'_, DbManager>,
) -> Result<QueryResult, String> {
    use super::operations::execute_query;

    execute_query(&connection, query)
        .await
        .map_err(|e| e.to_string())
}

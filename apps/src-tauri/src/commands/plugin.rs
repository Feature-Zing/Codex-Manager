use crate::commands::shared::rpc_call_in_background;

#[tauri::command]
pub async fn service_plugin_catalog_list(
    addr: Option<String>,
    market_mode: Option<String>,
    source_url: Option<String>,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({
        "marketMode": market_mode,
        "sourceUrl": source_url,
    });
    rpc_call_in_background("plugin/catalog/list", addr, Some(params)).await
}

#[tauri::command]
pub async fn service_plugin_catalog_refresh(
    addr: Option<String>,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background("plugin/catalog/refresh", addr, None).await
}

#[tauri::command]
pub async fn service_plugin_install(
    addr: Option<String>,
    entry: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({ "entry": entry });
    rpc_call_in_background("plugin/install", addr, Some(params)).await
}

#[tauri::command]
pub async fn service_plugin_update(
    addr: Option<String>,
    entry: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({ "entry": entry });
    rpc_call_in_background("plugin/update", addr, Some(params)).await
}

#[tauri::command]
pub async fn service_plugin_uninstall(
    addr: Option<String>,
    plugin_id: String,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({ "pluginId": plugin_id });
    rpc_call_in_background("plugin/uninstall", addr, Some(params)).await
}

#[tauri::command]
pub async fn service_plugin_list(addr: Option<String>) -> Result<serde_json::Value, String> {
    rpc_call_in_background("plugin/list", addr, None).await
}

#[tauri::command]
pub async fn service_plugin_enable(
    addr: Option<String>,
    plugin_id: String,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({ "pluginId": plugin_id });
    rpc_call_in_background("plugin/enable", addr, Some(params)).await
}

#[tauri::command]
pub async fn service_plugin_disable(
    addr: Option<String>,
    plugin_id: String,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({ "pluginId": plugin_id });
    rpc_call_in_background("plugin/disable", addr, Some(params)).await
}

#[tauri::command]
pub async fn service_plugin_tasks_update(
    addr: Option<String>,
    task_id: String,
    interval_seconds: i64,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({
        "taskId": task_id,
        "intervalSeconds": interval_seconds,
    });
    rpc_call_in_background("plugin/tasks/update", addr, Some(params)).await
}

#[tauri::command]
pub async fn service_plugin_tasks_list(
    addr: Option<String>,
    plugin_id: Option<String>,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({
        "pluginId": plugin_id,
    });
    rpc_call_in_background("plugin/tasks/list", addr, Some(params)).await
}

#[tauri::command]
pub async fn service_plugin_tasks_run(
    addr: Option<String>,
    task_id: String,
    input: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({
        "taskId": task_id,
        "input": input,
    });
    rpc_call_in_background("plugin/tasks/run", addr, Some(params)).await
}

#[tauri::command]
pub async fn service_plugin_logs_list(
    addr: Option<String>,
    plugin_id: Option<String>,
    task_id: Option<String>,
    limit: Option<i64>,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({
        "pluginId": plugin_id,
        "taskId": task_id,
        "limit": limit,
    });
    rpc_call_in_background("plugin/logs/list", addr, Some(params)).await
}

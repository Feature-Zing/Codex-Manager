use crate::commands::shared::rpc_call_in_background;

/// 函数 `service_aggregate_api_list`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - addr: 参数 addr
///
/// # 返回
/// 返回函数执行结果
#[tauri::command]
pub async fn service_aggregate_api_list(
    addr: Option<String>,
) -> Result<serde_json::Value, String> {
    rpc_call_in_background("aggregateApi/list", addr, None).await
}

/// 函数 `service_aggregate_api_create`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - addr: 参数 addr
/// - provider_type: 参数 provider_type
/// - supplier_name: 参数 supplier_name
/// - sort: 参数 sort
/// - url: 参数 url
/// - key: 参数 key
///
/// # 返回
/// 返回函数执行结果
#[tauri::command]
pub async fn service_aggregate_api_create(
    addr: Option<String>,
    provider_type: Option<String>,
    supplier_name: Option<String>,
    sort: Option<i64>,
    models: Option<Vec<String>>,
    url: Option<String>,
    key: Option<String>,
    auth_type: Option<String>,
    auth_custom_enabled: Option<bool>,
    auth_params: Option<serde_json::Value>,
    action_custom_enabled: Option<bool>,
    action: Option<String>,
    username: Option<String>,
    password: Option<String>,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({
        "providerType": provider_type,
        "supplierName": supplier_name,
        "sort": sort,
        "models": models,
        "url": url,
        "key": key,
        "authType": auth_type,
        "authCustomEnabled": auth_custom_enabled,
        "authParams": auth_params,
        "actionCustomEnabled": action_custom_enabled,
        "action": action,
        "username": username,
        "password": password,
    });
    rpc_call_in_background("aggregateApi/create", addr, Some(params)).await
}

/// 函数 `service_aggregate_api_update`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - addr: 参数 addr
/// - id: 参数 id
/// - provider_type: 参数 provider_type
/// - supplier_name: 参数 supplier_name
/// - sort: 参数 sort
/// - url: 参数 url
/// - key: 参数 key
///
/// # 返回
/// 返回函数执行结果
#[tauri::command]
pub async fn service_aggregate_api_update(
    addr: Option<String>,
    id: String,
    provider_type: Option<String>,
    supplier_name: Option<String>,
    sort: Option<i64>,
    models: Option<Vec<String>>,
    status: Option<String>,
    url: Option<String>,
    key: Option<String>,
    auth_type: Option<String>,
    auth_custom_enabled: Option<bool>,
    auth_params: Option<serde_json::Value>,
    action_custom_enabled: Option<bool>,
    action: Option<String>,
    username: Option<String>,
    password: Option<String>,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({
        "id": id,
        "providerType": provider_type,
        "supplierName": supplier_name,
        "sort": sort,
        "models": models,
        "status": status,
        "url": url,
        "key": key,
        "authType": auth_type,
        "authCustomEnabled": auth_custom_enabled,
        "authParams": auth_params,
        "actionCustomEnabled": action_custom_enabled,
        "action": action,
        "username": username,
        "password": password,
    });
    rpc_call_in_background("aggregateApi/update", addr, Some(params)).await
}

/// 函数 `service_aggregate_api_read_secret`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - addr: 参数 addr
/// - id: 参数 id
///
/// # 返回
/// 返回函数执行结果
#[tauri::command]
pub async fn service_aggregate_api_read_secret(
    addr: Option<String>,
    id: String,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({ "id": id });
    rpc_call_in_background("aggregateApi/readSecret", addr, Some(params)).await
}

/// 函数 `service_aggregate_api_delete`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - addr: 参数 addr
/// - id: 参数 id
///
/// # 返回
/// 返回函数执行结果
#[tauri::command]
pub async fn service_aggregate_api_delete(
    addr: Option<String>,
    id: String,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({ "id": id });
    rpc_call_in_background("aggregateApi/delete", addr, Some(params)).await
}

/// 函数 `service_aggregate_api_test_connection`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - addr: 参数 addr
/// - id: 参数 id
///
/// # 返回
/// 返回函数执行结果
#[tauri::command]
pub async fn service_aggregate_api_test_connection(
    addr: Option<String>,
    id: String,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({ "id": id });
    rpc_call_in_background("aggregateApi/testConnection", addr, Some(params)).await
}

#[tauri::command]
pub async fn service_aggregate_api_fetch_models(
    addr: Option<String>,
    id: Option<String>,
    provider_type: Option<String>,
    url: Option<String>,
    key: Option<String>,
    auth_type: Option<String>,
    auth_custom_enabled: Option<bool>,
    auth_params: Option<serde_json::Value>,
    action_custom_enabled: Option<bool>,
    action: Option<String>,
    username: Option<String>,
    password: Option<String>,
    preview_only: Option<bool>,
) -> Result<serde_json::Value, String> {
    let params = serde_json::json!({
        "id": id,
        "providerType": provider_type,
        "url": url,
        "key": key,
        "authType": auth_type,
        "authCustomEnabled": auth_custom_enabled,
        "authParams": auth_params,
        "actionCustomEnabled": action_custom_enabled,
        "action": action,
        "username": username,
        "password": password,
        "previewOnly": preview_only,
    });
    rpc_call_in_background("aggregateApi/fetchModels", addr, Some(params)).await
}

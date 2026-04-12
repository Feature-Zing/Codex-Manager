use codexmanager_core::rpc::types::{
    AggregateApiCreateResult, AggregateApiModelListResult, AggregateApiSecretResult,
    AggregateApiSummary, AggregateApiTestResult,
};
use codexmanager_core::storage::{now_ts, AggregateApi};
use reqwest::header::{HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::io::Read;
use std::time::Instant;

use crate::apikey_profile::normalize_upstream_base_url;
use crate::gateway;
use crate::storage_helpers::{generate_aggregate_api_id, open_storage};

pub(crate) const AGGREGATE_API_PROVIDER_CODEX: &str = "codex";
pub(crate) const AGGREGATE_API_PROVIDER_CLAUDE: &str = "claude";
pub(crate) const AGGREGATE_API_AUTH_APIKEY: &str = "apikey";
pub(crate) const AGGREGATE_API_AUTH_USERPASS: &str = "userpass";

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct UserPassSecret {
    username: String,
    password: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiKeyAuthParams {
    location: String,
    name: String,
    #[serde(default)]
    header_value_format: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserPassAuthParams {
    mode: String,
    #[serde(default)]
    username_name: Option<String>,
    #[serde(default)]
    password_name: Option<String>,
}

/// 函数 `normalize_secret`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - value: 参数 value
///
/// # 返回
/// 返回函数执行结果
fn normalize_secret(value: Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

/// 函数 `normalize_supplier_name`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - value: 参数 value
///
/// # 返回
/// 返回函数执行结果
fn normalize_supplier_name(value: Option<String>) -> Result<String, String> {
    let normalized = value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| "supplier name is required".to_string())?;
    Ok(normalized)
}

/// 函数 `normalize_sort`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - value: 参数 value
///
/// # 返回
/// 返回函数执行结果
fn normalize_sort(value: Option<i64>) -> i64 {
    value.unwrap_or(0)
}

fn normalize_status(value: Option<String>) -> Result<String, String> {
    match value {
        Some(raw) => {
            let normalized = raw.trim().to_ascii_lowercase().replace('-', "_");
            match normalized.as_str() {
                "active" | "enabled" | "enable" => Ok("active".to_string()),
                "disabled" | "disable" | "inactive" => Ok("disabled".to_string()),
                other => Err(format!("unsupported aggregate api status: {other}")),
            }
        }
        None => Ok("active".to_string()),
    }
}

fn normalize_auth_type(value: Option<String>) -> Result<String, String> {
    match value {
        Some(raw) => {
            let normalized = raw.trim().to_ascii_lowercase().replace('-', "_");
            match normalized.as_str() {
                "apikey" | "api_key" | "key" => Ok(AGGREGATE_API_AUTH_APIKEY.to_string()),
                "userpass" | "username_password" | "account_password" | "basic" | "http_basic" => {
                    Ok(AGGREGATE_API_AUTH_USERPASS.to_string())
                }
                other => Err(format!("unsupported aggregate api auth type: {other}")),
            }
        }
        None => Ok(AGGREGATE_API_AUTH_APIKEY.to_string()),
    }
}

fn normalize_action(value: Option<String>) -> Result<Option<String>, String> {
    let Some(raw) = value else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let normalized = trimmed.to_string();
    let lower = normalized.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        return Err("aggregate api action must be a path, not a full url".to_string());
    }
    if normalized.contains("://") {
        return Err("aggregate api action is invalid".to_string());
    }
    let with_slash = if normalized.starts_with('/') {
        normalized
    } else {
        format!("/{normalized}")
    };
    Ok(Some(with_slash))
}

fn normalize_auth_params_json(
    auth_type: &str,
    enabled: Option<bool>,
    auth_params: Option<serde_json::Value>,
) -> Result<Option<String>, String> {
    match enabled {
        None => Ok(None),
        Some(false) => Ok(Some(String::new())),
        Some(true) => {
            let value = auth_params.ok_or_else(|| "authParams is required".to_string())?;
            let obj = value
                .as_object()
                .ok_or_else(|| "authParams must be a JSON object".to_string())?;
            if obj.is_empty() {
                return Err("authParams must not be empty".to_string());
            }
            if auth_type == AGGREGATE_API_AUTH_APIKEY {
                let parsed: ApiKeyAuthParams = serde_json::from_value(value.clone())
                    .map_err(|_| "authParams is invalid".to_string())?;
                let location = parsed.location.trim().to_ascii_lowercase();
                if location != "header" && location != "query" {
                    return Err("authParams.location must be header or query".to_string());
                }
                if parsed.name.trim().is_empty() {
                    return Err("authParams.name is required".to_string());
                }
                if location == "header" {
                    let format = parsed
                        .header_value_format
                        .as_deref()
                        .unwrap_or("bearer")
                        .trim()
                        .to_ascii_lowercase();
                    if format != "bearer" && format != "raw" {
                        return Err(
                            "authParams.headerValueFormat must be bearer or raw".to_string()
                        );
                    }
                }
            } else if auth_type == AGGREGATE_API_AUTH_USERPASS {
                let parsed: UserPassAuthParams = serde_json::from_value(value.clone())
                    .map_err(|_| "authParams is invalid".to_string())?;
                let mode = parsed.mode.trim().to_ascii_lowercase();
                match mode.as_str() {
                    "basic" => {}
                    "headerpair" | "querypair" => {
                        if parsed
                            .username_name
                            .as_deref()
                            .map(str::trim)
                            .unwrap_or("")
                            .is_empty()
                        {
                            return Err("authParams.usernameName is required".to_string());
                        }
                        if parsed
                            .password_name
                            .as_deref()
                            .map(str::trim)
                            .unwrap_or("")
                            .is_empty()
                        {
                            return Err("authParams.passwordName is required".to_string());
                        }
                    }
                    _ => {
                        return Err(
                            "authParams.mode must be basic, headerPair, or queryPair".to_string()
                        );
                    }
                }
            }
            serde_json::to_string(&value)
                .map(Some)
                .map_err(|_| "authParams must be a valid JSON object".to_string())
        }
    }
}

fn normalize_action_override(
    enabled: Option<bool>,
    action: Option<String>,
) -> Result<Option<Option<String>>, String> {
    match enabled {
        None => Ok(None),
        Some(false) => Ok(Some(None)),
        Some(true) => normalize_action(action).map(|value| Some(Some(value.unwrap_or_default()))),
    }
}

fn normalize_model_slugs(items: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for item in items {
        let slug = item.trim().to_string();
        if slug.is_empty() {
            continue;
        }
        if seen.insert(slug.clone()) {
            out.push(slug);
        }
    }
    out
}

fn parse_models_json(value: Option<&str>) -> Vec<String> {
    let Some(raw) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Vec::new();
    };
    match serde_json::from_str::<Vec<String>>(raw) {
        Ok(items) => normalize_model_slugs(items),
        Err(_) => Vec::new(),
    }
}

fn serialize_model_slugs(items: Vec<String>) -> Result<Option<String>, String> {
    let normalized = normalize_model_slugs(items);
    if normalized.is_empty() {
        return Ok(None);
    }
    serde_json::to_string(&normalized)
        .map(Some)
        .map_err(|_| "invalid aggregate api models".to_string())
}

fn model_slug_from_value(value: &Value) -> Option<String> {
    match value {
        Value::String(raw) => {
            let slug = raw.trim();
            if slug.is_empty() {
                None
            } else {
                Some(slug.to_string())
            }
        }
        Value::Object(obj) => ["id", "slug", "name", "model"]
            .iter()
            .find_map(|key| obj.get(*key))
            .and_then(model_slug_from_value),
        _ => None,
    }
}

fn parse_model_slugs_from_payload(payload: &Value) -> Vec<String> {
    let items = payload
        .get("data")
        .and_then(Value::as_array)
        .or_else(|| payload.get("items").and_then(Value::as_array))
        .or_else(|| payload.get("models").and_then(Value::as_array))
        .or_else(|| payload.as_array());

    let Some(items) = items else {
        return Vec::new();
    };

    normalize_model_slugs(items.iter().filter_map(model_slug_from_value))
}

#[cfg(test)]
mod tests {
    use codexmanager_core::storage::AggregateApi;

    use super::{
        action_path_or_default, normalize_action_override, normalize_model_slugs,
        parse_model_slugs_from_payload,
    };

    fn aggregate_api_with_action(action: Option<&str>) -> AggregateApi {
        AggregateApi {
            id: "agg-test".to_string(),
            provider_type: "claude".to_string(),
            supplier_name: Some("test".to_string()),
            sort: 0,
            url: "https://open.bigmodel.cn/api/anthropic".to_string(),
            auth_type: "apikey".to_string(),
            auth_params_json: None,
            action: action.map(str::to_string),
            models_json: None,
            status: "active".to_string(),
            created_at: 0,
            updated_at: 0,
            last_test_at: None,
            last_test_status: None,
            last_test_error: None,
        }
    }

    #[test]
    fn action_override_disabled_stays_none() {
        let value =
            normalize_action_override(Some(false), Some("/v1/messages".to_string())).unwrap();
        assert_eq!(value, Some(None));
    }

    #[test]
    fn action_override_enabled_and_empty_preserves_empty_string() {
        let value = normalize_action_override(Some(true), Some("   ".to_string())).unwrap();
        assert_eq!(value, Some(Some(String::new())));
    }

    #[test]
    fn empty_action_uses_default_path() {
        let api = aggregate_api_with_action(Some(""));
        let path = action_path_or_default(&api, "/v1/messages?beta=true");
        assert_eq!(path, "/v1/messages?beta=true");
    }

    #[test]
    fn normalize_model_slugs_deduplicates_and_trims() {
        let items = normalize_model_slugs(vec![
            " gpt-4o ".to_string(),
            "".to_string(),
            "gpt-4o".to_string(),
            "claude-sonnet-4".to_string(),
        ]);
        assert_eq!(items, vec!["gpt-4o".to_string(), "claude-sonnet-4".to_string()]);
    }

    #[test]
    fn parse_model_slugs_from_payload_supports_common_shapes() {
        let payload = serde_json::json!({
            "data": [
                { "id": "gpt-4o" },
                { "slug": "claude-sonnet-4" },
                { "name": "gemini-2.5-pro" }
            ]
        });
        assert_eq!(
            parse_model_slugs_from_payload(&payload),
            vec![
                "gpt-4o".to_string(),
                "claude-sonnet-4".to_string(),
                "gemini-2.5-pro".to_string()
            ]
        );
    }
}

fn serialize_userpass_secret(username: &str, password: &str) -> Result<String, String> {
    let secret = UserPassSecret {
        username: username.trim().to_string(),
        password: password.trim().to_string(),
    };
    serde_json::to_string(&secret).map_err(|_| "invalid username/password".to_string())
}

fn action_path_or_default(api: &AggregateApi, default: &str) -> String {
    match api.action.as_deref().map(str::trim) {
        Some("") => default.to_string(),
        Some(value) => {
            if value.starts_with('/') {
                value.to_string()
            } else {
                format!("/{value}")
            }
        }
        None => default.to_string(),
    }
}

fn with_query_param(url: &str, name: &str, value: &str) -> String {
    let mut parsed = match reqwest::Url::parse(url) {
        Ok(value) => value,
        Err(_) => return url.to_string(),
    };
    let existing = parsed.query_pairs().into_owned().collect::<Vec<_>>();
    parsed.set_query(None);
    {
        let mut query = parsed.query_pairs_mut();
        for (key, val) in existing {
            if key == name {
                continue;
            }
            query.append_pair(key.as_str(), val.as_str());
        }
        query.append_pair(name, value);
    }
    parsed.to_string()
}

fn apply_probe_auth(
    mut builder: reqwest::blocking::RequestBuilder,
    mut url: String,
    api: &AggregateApi,
    secret: &str,
) -> Result<(reqwest::blocking::RequestBuilder, String), String> {
    let auth_type = normalize_auth_type(Some(api.auth_type.clone()))?;
    let auth_params = api
        .auth_params_json
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());

    if auth_type == AGGREGATE_API_AUTH_USERPASS {
        let parsed: UserPassSecret = serde_json::from_str(secret.trim())
            .map_err(|_| "invalid aggregate api secret".to_string())?;
        if let Some(raw) = auth_params {
            let params: UserPassAuthParams =
                serde_json::from_str(raw).map_err(|_| "invalid authParams".to_string())?;
            let mode = params.mode.trim().to_ascii_lowercase();
            if mode == "headerpair" {
                let username_name = params.username_name.as_deref().unwrap_or("username").trim();
                let password_name = params.password_name.as_deref().unwrap_or("password").trim();
                builder = builder
                    .header(username_name, parsed.username.as_str())
                    .header(password_name, parsed.password.as_str());
                return Ok((builder, url));
            }
            if mode == "querypair" {
                let username_name = params.username_name.as_deref().unwrap_or("username").trim();
                let password_name = params.password_name.as_deref().unwrap_or("password").trim();
                url = with_query_param(url.as_str(), username_name, parsed.username.as_str());
                url = with_query_param(url.as_str(), password_name, parsed.password.as_str());
                return Ok((builder, url));
            }
        }
        builder = builder.basic_auth(parsed.username, Some(parsed.password));
        return Ok((builder, url));
    }

    if let Some(raw) = auth_params {
        let params: ApiKeyAuthParams =
            serde_json::from_str(raw).map_err(|_| "invalid authParams".to_string())?;
        let location = params.location.trim().to_ascii_lowercase();
        if location == "query" {
            url = with_query_param(url.as_str(), params.name.trim(), secret.trim());
            return Ok((builder, url));
        }
        let value_format = params
            .header_value_format
            .as_deref()
            .unwrap_or("bearer")
            .trim()
            .to_ascii_lowercase();
        let header_value = if value_format == "raw" {
            secret.trim().to_string()
        } else {
            format!("Bearer {}", secret.trim())
        };
        builder = builder.header(params.name.trim(), header_value);
        return Ok((builder, url));
    }

    let auth_value = format!("Bearer {}", secret.trim());
    builder = builder
        .header(
            HeaderName::from_static("authorization"),
            HeaderValue::from_str(auth_value.as_str())
                .map_err(|_| "invalid aggregate api key".to_string())?,
        )
        .header("x-api-key", secret.trim())
        .header("api-key", secret.trim());
    Ok((builder, url))
}

/// 函数 `normalize_provider_type`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - value: 参数 value
///
/// # 返回
/// 返回函数执行结果
fn normalize_provider_type(value: Option<String>) -> Result<String, String> {
    match value {
        Some(raw) => {
            let normalized = raw.trim().to_ascii_lowercase().replace('-', "_");
            match normalized.as_str() {
                "codex" | "openai" | "openai_compat" | "gpt" | "gemini" | "gemini_native" => {
                    Ok(AGGREGATE_API_PROVIDER_CODEX.to_string())
                }
                "claude" | "anthropic" | "anthropic_native" | "claude_code" => {
                    Ok(AGGREGATE_API_PROVIDER_CLAUDE.to_string())
                }
                other => Err(format!("unsupported aggregate api provider type: {other}")),
            }
        }
        None => Ok(AGGREGATE_API_PROVIDER_CODEX.to_string()),
    }
}

/// 函数 `normalize_provider_type_value`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - value: 参数 value
///
/// # 返回
/// 返回函数执行结果
fn normalize_provider_type_value(value: &str) -> String {
    let normalized = value.trim().to_ascii_lowercase().replace('-', "_");
    match normalized.as_str() {
        "claude" | "anthropic" | "anthropic_native" | "claude_code" => {
            AGGREGATE_API_PROVIDER_CLAUDE.to_string()
        }
        _ => AGGREGATE_API_PROVIDER_CODEX.to_string(),
    }
}

/// 函数 `provider_default_url`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - provider_type: 参数 provider_type
///
/// # 返回
/// 返回函数执行结果
fn provider_default_url(provider_type: &str) -> &'static str {
    match provider_type {
        AGGREGATE_API_PROVIDER_CLAUDE => "https://api.anthropic.com/v1",
        _ => "https://api.openai.com/v1",
    }
}

/// 函数 `normalize_probe_url`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - base_url: 参数 base_url
/// - suffix: 参数 suffix
///
/// # 返回
/// 返回函数执行结果
fn normalize_probe_url(base_url: &str, suffix: &str) -> String {
    let base = base_url.trim().trim_end_matches('/');
    if suffix.trim().is_empty() {
        return base.to_string();
    }
    if base.ends_with("/v1") {
        format!("{base}{suffix}")
    } else {
        format!("{base}/v1{suffix}")
    }
}

fn join_base_url(base_url: &str, suffix: &str) -> String {
    let base = base_url.trim().trim_end_matches('/');
    let path = suffix.trim().trim_start_matches('/');
    if path.is_empty() {
        return base.to_string();
    }
    format!("{base}/{path}")
}

fn push_unique_url(urls: &mut Vec<String>, url: String) {
    if url.trim().is_empty() || urls.iter().any(|item| item == &url) {
        return;
    }
    urls.push(url);
}

fn model_fetch_url_candidates(api: &AggregateApi) -> Vec<String> {
    let mut urls = Vec::new();
    let normalized_models = normalize_probe_url(api.url.as_str(), "/models");
    push_unique_url(&mut urls, normalized_models.clone());
    push_unique_url(&mut urls, join_base_url(api.url.as_str(), "models"));
    push_unique_url(&mut urls, join_base_url(api.url.as_str(), "v1/models"));
    if let Some(action) = api.action.as_deref().map(str::trim).filter(|value| !value.is_empty()) {
        let action_root = action.split('?').next().unwrap_or(action);
        if let Some(prefix) = action_root.rsplit_once('/') {
            let derived = format!("{}/models", prefix.0);
            push_unique_url(
                &mut urls,
                normalize_probe_url(api.url.as_str(), derived.as_str()),
            );
        }
    }
    urls
}

/// 函数 `read_first_chunk`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - response: 参数 response
///
/// # 返回
/// 返回函数执行结果
fn read_first_chunk(mut response: reqwest::blocking::Response) -> Result<(), String> {
    let mut buf = [0u8; 16];
    let read = response.read(&mut buf).map_err(|err| err.to_string())?;
    if read > 0 {
        Ok(())
    } else {
        Err("No response data received".to_string())
    }
}

/// 函数 `build_claude_probe_body`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// 无
///
/// # 返回
/// 返回函数执行结果
fn build_claude_probe_body() -> serde_json::Value {
    json!({
        "model": "claude-haiku-4-5-20251001",
        "max_tokens": 1,
        "messages": [{
            "role": "user",
            "content": "Who are you?"
        }],
        "stream": true
    })
}

/// 函数 `build_codex_probe_body`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// 无
///
/// # 返回
/// 返回函数执行结果
fn build_codex_probe_body() -> serde_json::Value {
    json!({
        "model": "gpt-5.1-codex",
        "input": [{
            "role": "user",
            "content": [{
                "type": "text",
                "text": "Who are you?"
            }]
        }],
        "stream": true
    })
}

/// 函数 `append_client_version_query`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - url: 参数 url
///
/// # 返回
/// 返回函数执行结果
fn append_client_version_query(url: &str) -> String {
    if url.contains("client_version=") {
        return url.to_string();
    }
    let separator = if url.contains('?') { '&' } else { '?' };
    format!(
        "{url}{separator}client_version={}",
        gateway::current_codex_user_agent_version()
    )
}

/// 函数 `probe_codex_only_for_provider`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - provider_type: 参数 provider_type
///
/// # 返回
/// 返回函数执行结果
fn probe_codex_only_for_provider(provider_type: &str) -> bool {
    provider_type != AGGREGATE_API_PROVIDER_CLAUDE
}

/// 函数 `add_codex_probe_headers`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - builder: 参数 builder
/// - secret: 参数 secret
///
/// # 返回
/// 返回函数执行结果
fn add_codex_probe_headers(
    builder: reqwest::blocking::RequestBuilder,
) -> Result<reqwest::blocking::RequestBuilder, String> {
    Ok(builder
        .header("accept", "application/json")
        .header("user-agent", gateway::current_codex_user_agent())
        .header("originator", gateway::current_wire_originator())
        .header("accept-encoding", "identity"))
}

fn send_fetch_models_request(
    client: &reqwest::blocking::Client,
    api: &AggregateApi,
    secret: &str,
    url: &str,
) -> Result<Value, String> {
    let provider_type = normalize_provider_type_value(api.provider_type.as_str());
    let request_url = if provider_type == AGGREGATE_API_PROVIDER_CODEX {
        append_client_version_query(url)
    } else {
        url.to_string()
    };
    let builder = client.get(request_url.as_str());
    let (builder, updated_url) = apply_probe_auth(builder, request_url.clone(), api, secret)?;
    let builder = if updated_url != request_url {
        let rebuilt = client.get(updated_url.as_str());
        let (rebuilt, _) = apply_probe_auth(rebuilt, updated_url, api, secret)?;
        rebuilt
    } else {
        builder
    };
    let response = add_codex_probe_headers(builder)?
        .send()
        .map_err(|err| err.to_string())?;
    let status_code = response.status().as_u16() as i64;
    let body = response.text().map_err(|err| err.to_string())?;
    if !(200..300).contains(&status_code) {
        let detail = body.trim();
        return Err(if detail.is_empty() {
            format!("fetch models http_status={status_code}")
        } else {
            let snippet = detail.chars().take(200).collect::<String>();
            format!("fetch models http_status={status_code}: {snippet}")
        });
    }
    serde_json::from_str::<Value>(&body).map_err(|_| "invalid models response json".to_string())
}

fn fetch_remote_model_slugs(
    client: &reqwest::blocking::Client,
    api: &AggregateApi,
    secret: &str,
) -> Result<Vec<String>, String> {
    let mut errors = Vec::new();
    for url in model_fetch_url_candidates(api) {
        match send_fetch_models_request(client, api, secret, url.as_str()) {
            Ok(payload) => {
                let items = parse_model_slugs_from_payload(&payload);
                if !items.is_empty() {
                    return Ok(items);
                }
                errors.push("models response did not contain any valid model ids".to_string());
            }
            Err(err) => errors.push(err),
        }
    }
    Err(errors.join("; "))
}

struct AggregateApiFetchTarget {
    api_id: Option<String>,
    api: AggregateApi,
    secret: String,
    persist: bool,
}

#[allow(clippy::too_many_arguments)]
fn build_fetch_target(
    api_id: Option<String>,
    provider_type: Option<String>,
    url: Option<String>,
    key: Option<String>,
    auth_type: Option<String>,
    auth_custom_enabled: Option<bool>,
    auth_params: Option<Value>,
    action_custom_enabled: Option<bool>,
    action: Option<String>,
    username: Option<String>,
    password: Option<String>,
    preview_only: bool,
) -> Result<AggregateApiFetchTarget, String> {
    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let existing = if let Some(api_id) = api_id.as_deref().filter(|value| !value.trim().is_empty()) {
        storage
            .find_aggregate_api_by_id(api_id)
            .map_err(|err| err.to_string())?
    } else {
        None
    };
    let existing_secret = if let Some(api_id) = api_id.as_deref().filter(|value| !value.trim().is_empty()) {
        storage
            .find_aggregate_api_secret_by_id(api_id)
            .map_err(|err| err.to_string())?
    } else {
        None
    };

    let resolved_provider_type = normalize_provider_type(match provider_type {
        Some(value) => Some(value),
        None => existing.as_ref().map(|item| item.provider_type.clone()),
    })?;
    let resolved_url = normalize_upstream_base_url(match url {
        Some(value) => Some(value),
        None => existing.as_ref().map(|item| item.url.clone()),
    })?
    .unwrap_or_else(|| provider_default_url(resolved_provider_type.as_str()).to_string());
    let resolved_auth_type = normalize_auth_type(match auth_type {
        Some(value) => Some(value),
        None => existing.as_ref().map(|item| item.auth_type.clone()),
    })?;

    let resolved_auth_params_json = if auth_custom_enabled.is_none() && auth_params.is_none() {
        existing.as_ref().and_then(|item| item.auth_params_json.clone())
    } else {
        normalize_auth_params_json(
            resolved_auth_type.as_str(),
            auth_custom_enabled,
            auth_params,
        )?
        .and_then(|value| if value.is_empty() { None } else { Some(value) })
    };

    let resolved_action = if action_custom_enabled.is_none() && action.is_none() {
        existing.as_ref().and_then(|item| item.action.clone())
    } else {
        normalize_action_override(action_custom_enabled, action)?.unwrap_or(None)
    };

    let resolved_secret = if resolved_auth_type == AGGREGATE_API_AUTH_APIKEY {
        normalize_secret(key)
            .or(existing_secret.clone())
            .ok_or_else(|| "key is required".to_string())?
    } else {
        let next_username = username.as_deref().map(str::trim).unwrap_or("");
        let next_password = password.as_deref().map(str::trim).unwrap_or("");
        if !next_username.is_empty() || !next_password.is_empty() {
            if next_username.is_empty() || next_password.is_empty() {
                return Err("username and password must be provided together".to_string());
            }
            serialize_userpass_secret(next_username, next_password)?
        } else {
            existing_secret.ok_or_else(|| "username and password are required".to_string())?
        }
    };

    let api = AggregateApi {
        id: existing
            .as_ref()
            .map(|item| item.id.clone())
            .unwrap_or_else(|| "aggregate-api-preview".to_string()),
        provider_type: resolved_provider_type,
        supplier_name: existing.as_ref().and_then(|item| item.supplier_name.clone()),
        sort: existing.as_ref().map(|item| item.sort).unwrap_or(0),
        url: resolved_url,
        auth_type: resolved_auth_type,
        auth_params_json: resolved_auth_params_json,
        action: resolved_action,
        models_json: existing.as_ref().and_then(|item| item.models_json.clone()),
        status: existing
            .as_ref()
            .map(|item| item.status.clone())
            .unwrap_or_else(|| "active".to_string()),
        created_at: existing.as_ref().map(|item| item.created_at).unwrap_or(0),
        updated_at: existing.as_ref().map(|item| item.updated_at).unwrap_or(0),
        last_test_at: existing.as_ref().and_then(|item| item.last_test_at),
        last_test_status: existing
            .as_ref()
            .and_then(|item| item.last_test_status.clone()),
        last_test_error: existing
            .as_ref()
            .and_then(|item| item.last_test_error.clone()),
    };

    Ok(AggregateApiFetchTarget {
        api_id: existing.as_ref().map(|item| item.id.clone()),
        api,
        secret: resolved_secret,
        persist: !preview_only,
    })
}

/// 函数 `probe_codex_models_endpoint`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - client: 参数 client
/// - base_url: 参数 base_url
/// - secret: 参数 secret
///
/// # 返回
/// 返回函数执行结果
fn probe_codex_models_endpoint(
    client: &reqwest::blocking::Client,
    api: &AggregateApi,
    secret: &str,
) -> Result<i64, String> {
    let probe_path = action_path_or_default(api, "/models");
    let base_url = normalize_probe_url(api.url.as_str(), probe_path.as_str());
    let url = append_client_version_query(base_url.as_str());
    let builder = client.get(url.as_str());
    let (builder, updated_url) = apply_probe_auth(builder, url.clone(), api, secret)?;
    let builder = if updated_url != url {
        let rebuilt = client.get(updated_url.as_str());
        let (rebuilt, _) = apply_probe_auth(rebuilt, updated_url, api, secret)?;
        rebuilt
    } else {
        builder
    };
    let response = add_codex_probe_headers(builder)?
        .send()
        .map_err(|err| err.to_string())?;

    let status_code = response.status().as_u16() as i64;
    if !response.status().is_success() {
        return Err(format!("codex models probe http_status={status_code}"));
    }
    read_first_chunk(response)?;
    Ok(status_code)
}

/// 函数 `probe_codex_responses_endpoint`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - client: 参数 client
/// - base_url: 参数 base_url
/// - secret: 参数 secret
///
/// # 返回
/// 返回函数执行结果
fn probe_codex_responses_endpoint(
    client: &reqwest::blocking::Client,
    api: &AggregateApi,
    secret: &str,
) -> Result<i64, String> {
    let action_hint = api
        .action
        .as_deref()
        .map(str::trim)
        .unwrap_or("/responses")
        .to_ascii_lowercase();
    let default_path = if action_hint.contains("chat/completions") {
        "/chat/completions"
    } else if action_hint.contains("responses") {
        "/responses"
    } else {
        "/responses"
    };
    let probe_path = action_path_or_default(api, default_path);
    let url = normalize_probe_url(api.url.as_str(), probe_path.as_str());
    let builder = client.post(url.as_str());
    let (builder, updated_url) = apply_probe_auth(builder, url.clone(), api, secret)?;
    let builder = if updated_url != url {
        let rebuilt = client.post(updated_url.as_str());
        let (rebuilt, _) = apply_probe_auth(rebuilt, updated_url, api, secret)?;
        rebuilt
    } else {
        builder
    };
    let request_body = if probe_path.to_ascii_lowercase().contains("chat/completions") {
        json!({
            "model": "gpt-4o-mini",
            "messages": [{"role":"user","content":"hi"}],
            "stream": false
        })
    } else {
        build_codex_probe_body()
    };
    let response = add_codex_probe_headers(builder)?
        .header("content-type", "application/json")
        .header("accept", "text/event-stream")
        .json(&request_body)
        .send()
        .map_err(|err| err.to_string())?;

    let status_code = response.status().as_u16() as i64;
    if !response.status().is_success() {
        return Err(format!("codex probe http_status={status_code}"));
    }
    read_first_chunk(response)?;
    Ok(status_code)
}

/// 函数 `probe_codex_endpoint`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - client: 参数 client
/// - base_url: 参数 base_url
/// - secret: 参数 secret
///
/// # 返回
/// 返回函数执行结果
fn probe_codex_endpoint(
    client: &reqwest::blocking::Client,
    api: &AggregateApi,
    secret: &str,
) -> Result<i64, String> {
    let models_result = probe_codex_models_endpoint(client, api, secret);
    if let Ok(code) = models_result {
        return Ok(code);
    }

    let models_err = models_result
        .err()
        .unwrap_or_else(|| "codex models probe failed".to_string());
    let responses_result = probe_codex_responses_endpoint(client, api, secret);
    if let Ok(code) = responses_result {
        return Ok(code);
    }

    let responses_err = responses_result
        .err()
        .unwrap_or_else(|| "codex responses probe failed".to_string());
    Err(format!("{models_err}; {responses_err}"))
}

/// 函数 `probe_claude_endpoint`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - client: 参数 client
/// - base_url: 参数 base_url
/// - secret: 参数 secret
///
/// # 返回
/// 返回函数执行结果
fn probe_claude_endpoint(
    client: &reqwest::blocking::Client,
    api: &AggregateApi,
    secret: &str,
) -> Result<i64, String> {
    let probe_path = action_path_or_default(api, "/messages?beta=true");
    let url = normalize_probe_url(api.url.as_str(), probe_path.as_str());
    let builder = client.post(url.as_str());
    let (builder, updated_url) = apply_probe_auth(builder, url.clone(), api, secret)?;
    let builder = if updated_url != url {
        let rebuilt = client.post(updated_url.as_str());
        let (rebuilt, _) = apply_probe_auth(rebuilt, updated_url, api, secret)?;
        rebuilt
    } else {
        builder
    };
    let response = builder
        .header("anthropic-version", "2023-06-01")
        .header(
            "anthropic-beta",
            "claude-code-20250219,interleaved-thinking-2025-05-14",
        )
        .header("content-type", "application/json")
        .header("accept", "application/json")
        .header("accept-encoding", "identity")
        .header("user-agent", "claude-cli/2.1.2 (external, cli)")
        .header("x-app", "cli")
        .json(&build_claude_probe_body())
        .send()
        .map_err(|err| err.to_string())?;

    let status_code = response.status().as_u16() as i64;
    if !response.status().is_success() {
        return Err(format!("claude probe http_status={status_code}"));
    }
    read_first_chunk(response)?;
    Ok(status_code)
}

/// 函数 `list_aggregate_apis`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - crate: 参数 crate
///
/// # 返回
/// 返回函数执行结果
pub(crate) fn list_aggregate_apis() -> Result<Vec<AggregateApiSummary>, String> {
    let storage = open_storage().ok_or_else(|| "open storage failed".to_string())?;
    let items = storage
        .list_aggregate_apis()
        .map_err(|err| format!("list aggregate apis failed: {err}"))?;
    Ok(items
        .into_iter()
        .map(|item| AggregateApiSummary {
            id: item.id,
            provider_type: item.provider_type,
            supplier_name: item.supplier_name,
            sort: item.sort,
            url: item.url,
            auth_type: item.auth_type,
            auth_params: item
                .auth_params_json
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .and_then(|value| serde_json::from_str::<serde_json::Value>(value).ok()),
            action: item.action,
            models: parse_models_json(item.models_json.as_deref()),
            status: item.status,
            created_at: item.created_at,
            updated_at: item.updated_at,
            last_test_at: item.last_test_at,
            last_test_status: item.last_test_status,
            last_test_error: item.last_test_error,
        })
        .collect())
}

/// 函数 `create_aggregate_api`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - crate: 参数 crate
///
/// # 返回
/// 返回函数执行结果
pub(crate) fn create_aggregate_api(
    url: Option<String>,
    key: Option<String>,
    provider_type: Option<String>,
    supplier_name: Option<String>,
    sort: Option<i64>,
    models: Option<Vec<String>>,
    auth_type: Option<String>,
    auth_custom_enabled: Option<bool>,
    auth_params: Option<serde_json::Value>,
    action_custom_enabled: Option<bool>,
    action: Option<String>,
    username: Option<String>,
    password: Option<String>,
) -> Result<AggregateApiCreateResult, String> {
    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let normalized_provider_type = normalize_provider_type(provider_type)?;
    let normalized_supplier_name = normalize_supplier_name(supplier_name)?;
    let normalized_sort = normalize_sort(sort);
    let normalized_url = normalize_upstream_base_url(url)?
        .unwrap_or_else(|| provider_default_url(normalized_provider_type.as_str()).to_string());
    let normalized_auth_type = normalize_auth_type(auth_type)?;
    let normalized_auth_params_json = normalize_auth_params_json(
        normalized_auth_type.as_str(),
        auth_custom_enabled,
        auth_params,
    )?;
    let normalized_models_json = serialize_model_slugs(models.unwrap_or_default())?;
    let normalized_action =
        normalize_action_override(action_custom_enabled, action)?.unwrap_or(None);
    let normalized_secret = if normalized_auth_type == AGGREGATE_API_AUTH_APIKEY {
        normalize_secret(key).ok_or_else(|| "key is required".to_string())?
    } else {
        let username = username
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .ok_or_else(|| "username is required".to_string())?;
        let password = password
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .ok_or_else(|| "password is required".to_string())?;
        serialize_userpass_secret(username, password)?
    };
    let id = generate_aggregate_api_id();
    let created_at = now_ts();
    let record = AggregateApi {
        id: id.clone(),
        provider_type: normalized_provider_type,
        supplier_name: Some(normalized_supplier_name),
        sort: normalized_sort,
        url: normalized_url,
        auth_type: normalized_auth_type,
        auth_params_json: normalized_auth_params_json
            .map(|value| if value.is_empty() { None } else { Some(value) })
            .unwrap_or(None),
        action: normalized_action,
        models_json: normalized_models_json,
        status: "active".to_string(),
        created_at,
        updated_at: created_at,
        last_test_at: None,
        last_test_status: None,
        last_test_error: None,
    };
    storage
        .insert_aggregate_api(&record)
        .map_err(|err| err.to_string())?;
    if let Err(err) = storage.upsert_aggregate_api_secret(&id, &normalized_secret) {
        let _ = storage.delete_aggregate_api(&id);
        return Err(format!("persist aggregate api secret failed: {err}"));
    }
    Ok(AggregateApiCreateResult {
        id,
        key: if record.auth_type == AGGREGATE_API_AUTH_APIKEY {
            normalized_secret
        } else {
            String::new()
        },
    })
}

/// 函数 `update_aggregate_api`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - crate: 参数 crate
///
/// # 返回
/// 返回函数执行结果
pub(crate) fn update_aggregate_api(
    api_id: &str,
    url: Option<String>,
    key: Option<String>,
    provider_type: Option<String>,
    supplier_name: Option<String>,
    sort: Option<i64>,
    models: Option<Vec<String>>,
    status: Option<String>,
    auth_type: Option<String>,
    auth_custom_enabled: Option<bool>,
    auth_params: Option<serde_json::Value>,
    action_custom_enabled: Option<bool>,
    action: Option<String>,
    username: Option<String>,
    password: Option<String>,
) -> Result<(), String> {
    if api_id.is_empty() {
        return Err("aggregate api id required".to_string());
    }
    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let existing = storage
        .find_aggregate_api_by_id(api_id)
        .map_err(|err| err.to_string())?
        .ok_or_else(|| "aggregate api not found".to_string())?;
    let existing_auth_type = normalize_auth_type(Some(existing.auth_type.clone()))
        .unwrap_or_else(|_| AGGREGATE_API_AUTH_APIKEY.to_string());
    let normalized_auth_type = match auth_type {
        Some(raw) => Some(normalize_auth_type(Some(raw))?),
        None => None,
    };
    let next_auth_type = normalized_auth_type
        .as_deref()
        .unwrap_or(existing_auth_type.as_str())
        .to_string();
    let auth_type_changed = next_auth_type != existing_auth_type;

    if let Some(next) = normalized_auth_type.as_deref() {
        storage
            .update_aggregate_api_auth_type(api_id, next)
            .map_err(|err| err.to_string())?;
    }
    if let Some(provider_type) = provider_type {
        let normalized_provider_type = normalize_provider_type(Some(provider_type))?;
        storage
            .update_aggregate_api_type(api_id, normalized_provider_type.as_str())
            .map_err(|err| err.to_string())?;
    }
    let normalized_supplier_name = normalize_supplier_name(supplier_name)?;
    storage
        .update_aggregate_api_supplier_name(api_id, Some(normalized_supplier_name.as_str()))
        .map_err(|err| err.to_string())?;
    if sort.is_some() {
        storage
            .update_aggregate_api_sort(api_id, normalize_sort(sort))
            .map_err(|err| err.to_string())?;
    }
    if let Some(status) = status {
        let normalized_status = normalize_status(Some(status))?;
        storage
            .update_aggregate_api_status(api_id, normalized_status.as_str())
            .map_err(|err| err.to_string())?;
    }
    if let Some(url) = url {
        let normalized_url =
            normalize_upstream_base_url(Some(url))?.ok_or_else(|| "url is required".to_string())?;
        storage
            .update_aggregate_api(api_id, normalized_url.as_str())
            .map_err(|err| err.to_string())?;
    }

    if let Some(auth_params_json) =
        normalize_auth_params_json(next_auth_type.as_str(), auth_custom_enabled, auth_params)?
    {
        let normalized = auth_params_json.trim().to_string();
        if normalized.is_empty() {
            storage
                .update_aggregate_api_auth_params_json(api_id, None)
                .map_err(|err| err.to_string())?;
        } else {
            storage
                .update_aggregate_api_auth_params_json(api_id, Some(normalized.as_str()))
                .map_err(|err| err.to_string())?;
        }
    }

    if let Some(action_override) = normalize_action_override(action_custom_enabled, action)? {
        if let Some(action) = action_override {
            let normalized = action.trim().to_string();
            storage
                .update_aggregate_api_action(api_id, Some(normalized.as_str()))
                .map_err(|err| err.to_string())?;
        } else {
            storage
                .update_aggregate_api_action(api_id, None)
                .map_err(|err| err.to_string())?;
        }
    }

    if let Some(models) = models {
        let models_json = serialize_model_slugs(models)?;
        storage
            .update_aggregate_api_models_json(api_id, models_json.as_deref())
            .map_err(|err| err.to_string())?;
    }

    if next_auth_type == AGGREGATE_API_AUTH_APIKEY {
        let normalized_secret = normalize_secret(key);
        if auth_type_changed && normalized_secret.is_none() {
            return Err("key is required when switching authType to apikey".to_string());
        }
        if let Some(secret) = normalized_secret {
            storage
                .upsert_aggregate_api_secret(api_id, &secret)
                .map_err(|err| err.to_string())?;
        }
    } else {
        let username = username.as_deref().map(str::trim).unwrap_or("");
        let password = password.as_deref().map(str::trim).unwrap_or("");
        let has_user = !username.is_empty();
        let has_pass = !password.is_empty();
        if (has_user && !has_pass) || (!has_user && has_pass) {
            return Err("username and password must be provided together".to_string());
        }
        if auth_type_changed && (!has_user || !has_pass) {
            return Err(
                "username and password are required when switching authType to userpass"
                    .to_string(),
            );
        }
        if has_user && has_pass {
            let secret = serialize_userpass_secret(username, password)?;
            storage
                .upsert_aggregate_api_secret(api_id, &secret)
                .map_err(|err| err.to_string())?;
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn fetch_aggregate_api_models(
    api_id: Option<String>,
    provider_type: Option<String>,
    url: Option<String>,
    key: Option<String>,
    auth_type: Option<String>,
    auth_custom_enabled: Option<bool>,
    auth_params: Option<Value>,
    action_custom_enabled: Option<bool>,
    action: Option<String>,
    username: Option<String>,
    password: Option<String>,
    preview_only: bool,
) -> Result<AggregateApiModelListResult, String> {
    let target = build_fetch_target(
        api_id,
        provider_type,
        url,
        key,
        auth_type,
        auth_custom_enabled,
        auth_params,
        action_custom_enabled,
        action,
        username,
        password,
        preview_only,
    )?;
    let client = gateway::fresh_upstream_client();
    let items = fetch_remote_model_slugs(&client, &target.api, &target.secret)?;
    if target.persist {
        if let Some(api_id) = target.api_id.as_deref() {
            let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
            let models_json = serialize_model_slugs(items.clone())?;
            storage
                .update_aggregate_api_models_json(api_id, models_json.as_deref())
                .map_err(|err| err.to_string())?;
        }
    }
    Ok(AggregateApiModelListResult {
        id: target.api_id.unwrap_or_else(|| target.api.id),
        items,
    })
}

/// 函数 `delete_aggregate_api`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - crate: 参数 crate
///
/// # 返回
/// 返回函数执行结果
pub(crate) fn delete_aggregate_api(api_id: &str) -> Result<(), String> {
    if api_id.is_empty() {
        return Err("aggregate api id required".to_string());
    }
    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    storage
        .delete_aggregate_api(api_id)
        .map_err(|err| err.to_string())
}

/// 函数 `read_aggregate_api_secret`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - crate: 参数 crate
///
/// # 返回
/// 返回函数执行结果
pub(crate) fn read_aggregate_api_secret(api_id: &str) -> Result<AggregateApiSecretResult, String> {
    if api_id.is_empty() {
        return Err("aggregate api id required".to_string());
    }
    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let api = storage
        .find_aggregate_api_by_id(api_id)
        .map_err(|err| err.to_string())?
        .ok_or_else(|| "aggregate api not found".to_string())?;
    let key = storage
        .find_aggregate_api_secret_by_id(api_id)
        .map_err(|err| err.to_string())?
        .ok_or_else(|| "aggregate api secret not found".to_string())?;
    let auth_type = normalize_auth_type(Some(api.auth_type))?;
    if auth_type == AGGREGATE_API_AUTH_USERPASS {
        let parsed: UserPassSecret = serde_json::from_str(key.as_str())
            .map_err(|_| "invalid aggregate api secret".to_string())?;
        return Ok(AggregateApiSecretResult {
            id: api_id.to_string(),
            key: String::new(),
            auth_type,
            username: Some(parsed.username),
            password: Some(parsed.password),
        });
    }
    Ok(AggregateApiSecretResult {
        id: api_id.to_string(),
        key,
        auth_type,
        username: None,
        password: None,
    })
}

/// 函数 `test_aggregate_api_connection`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - crate: 参数 crate
///
/// # 返回
/// 返回函数执行结果
pub(crate) fn test_aggregate_api_connection(
    api_id: &str,
) -> Result<AggregateApiTestResult, String> {
    if api_id.is_empty() {
        return Err("aggregate api id required".to_string());
    }
    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let api = storage
        .find_aggregate_api_by_id(api_id)
        .map_err(|err| err.to_string())?
        .ok_or_else(|| "aggregate api not found".to_string())?;
    let secret = storage
        .find_aggregate_api_secret_by_id(api_id)
        .map_err(|err| err.to_string())?;
    let Some(secret) = secret else {
        return Err("aggregate api secret not found".to_string());
    };
    let client = gateway::fresh_upstream_client();
    let started_at = Instant::now();
    let provider_type = normalize_provider_type_value(api.provider_type.as_str());
    let result = if probe_codex_only_for_provider(provider_type.as_str()) {
        probe_codex_endpoint(&client, &api, &secret)
    } else {
        probe_claude_endpoint(&client, &api, &secret)
    };
    let (ok, status_code, last_error) = match result {
        Ok(code) => (true, Some(code), None),
        Err(err) => (false, None, Some(err)),
    };
    let message = last_error.map(|err| format!("provider={provider_type}; {err}"));

    let _ = storage.update_aggregate_api_test_result(api_id, ok, status_code, message.as_deref());
    Ok(AggregateApiTestResult {
        id: api_id.to_string(),
        ok,
        status_code,
        message,
        tested_at: now_ts(),
        latency_ms: started_at.elapsed().as_millis() as i64,
    })
}

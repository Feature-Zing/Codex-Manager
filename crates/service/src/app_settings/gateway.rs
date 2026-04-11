use crate::gateway;
use crate::usage_refresh;
use serde::Deserialize;

use super::{
    normalize_optional_text, save_persisted_app_setting, save_persisted_bool_setting,
    APP_SETTING_GATEWAY_ACCOUNT_MAX_INFLIGHT_KEY, APP_SETTING_GATEWAY_BACKGROUND_TASKS_KEY,
    APP_SETTING_GATEWAY_FREE_ACCOUNT_MAX_MODEL_KEY, APP_SETTING_GATEWAY_MODEL_FORWARD_RULES_KEY,
    APP_SETTING_GATEWAY_ORIGINATOR_KEY, APP_SETTING_GATEWAY_REQUEST_COMPRESSION_ENABLED_KEY,
    APP_SETTING_GATEWAY_RESIDENCY_REQUIREMENT_KEY, APP_SETTING_GATEWAY_ROUTE_STRATEGY_KEY,
    APP_SETTING_GATEWAY_SSE_KEEPALIVE_INTERVAL_MS_KEY, APP_SETTING_GATEWAY_UPSTREAM_PROXY_URL_KEY,
    APP_SETTING_GATEWAY_UPSTREAM_STREAM_TIMEOUT_MS_KEY, APP_SETTING_GATEWAY_USER_AGENT_VERSION_KEY,
};

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundTasksInput {
    pub usage_polling_enabled: Option<bool>,
    pub usage_poll_interval_secs: Option<u64>,
    pub gateway_keepalive_enabled: Option<bool>,
    pub gateway_keepalive_interval_secs: Option<u64>,
    pub aggregate_api_probe_enabled: Option<bool>,
    pub aggregate_api_probe_interval_secs: Option<u64>,
    pub token_refresh_polling_enabled: Option<bool>,
    pub token_refresh_poll_interval_secs: Option<u64>,
    pub usage_refresh_workers: Option<usize>,
    pub http_worker_factor: Option<usize>,
    pub http_worker_min: Option<usize>,
    pub http_stream_worker_factor: Option<usize>,
    pub http_stream_worker_min: Option<usize>,
}

impl BackgroundTasksInput {
    /// 函数 `into_patch`
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
    pub(crate) fn into_patch(self) -> usage_refresh::BackgroundTasksSettingsPatch {
        usage_refresh::BackgroundTasksSettingsPatch {
            usage_polling_enabled: self.usage_polling_enabled,
            usage_poll_interval_secs: self.usage_poll_interval_secs,
            gateway_keepalive_enabled: self.gateway_keepalive_enabled,
            gateway_keepalive_interval_secs: self.gateway_keepalive_interval_secs,
            aggregate_api_probe_enabled: self.aggregate_api_probe_enabled,
            aggregate_api_probe_interval_secs: self.aggregate_api_probe_interval_secs,
            token_refresh_polling_enabled: self.token_refresh_polling_enabled,
            token_refresh_poll_interval_secs: self.token_refresh_poll_interval_secs,
            usage_refresh_workers: self.usage_refresh_workers,
            http_worker_factor: self.http_worker_factor,
            http_worker_min: self.http_worker_min,
            http_stream_worker_factor: self.http_stream_worker_factor,
            http_stream_worker_min: self.http_stream_worker_min,
        }
    }
}

/// 函数 `set_gateway_route_strategy`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - strategy: 参数 strategy
///
/// # 返回
/// 返回函数执行结果
pub fn set_gateway_route_strategy(strategy: &str) -> Result<String, String> {
    let applied = gateway::set_route_strategy(strategy)?.to_string();
    save_persisted_app_setting(APP_SETTING_GATEWAY_ROUTE_STRATEGY_KEY, Some(&applied))?;
    Ok(applied)
}

/// 函数 `set_gateway_free_account_max_model`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - model: 参数 model
///
/// # 返回
/// 返回函数执行结果
pub fn set_gateway_free_account_max_model(model: &str) -> Result<String, String> {
    let applied = gateway::set_free_account_max_model(model)?;
    save_persisted_app_setting(
        APP_SETTING_GATEWAY_FREE_ACCOUNT_MAX_MODEL_KEY,
        Some(&applied),
    )?;
    Ok(applied)
}

/// 函数 `current_gateway_free_account_max_model`
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
pub fn current_gateway_free_account_max_model() -> String {
    gateway::current_free_account_max_model()
}

/// 函数 `set_gateway_model_forward_rules`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-05
///
/// # 参数
/// - raw: 参数 raw
///
/// # 返回
/// 返回函数执行结果
pub fn set_gateway_model_forward_rules(raw: &str) -> Result<String, String> {
    let applied = gateway::set_model_forward_rules(raw)?;
    save_persisted_app_setting(
        APP_SETTING_GATEWAY_MODEL_FORWARD_RULES_KEY,
        if applied.trim().is_empty() {
            None
        } else {
            Some(applied.as_str())
        },
    )?;
    Ok(applied)
}

/// 函数 `current_gateway_model_forward_rules`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-05
///
/// # 参数
/// 无
///
/// # 返回
/// 返回函数执行结果
pub fn current_gateway_model_forward_rules() -> String {
    gateway::current_model_forward_rules()
}

/// 函数 `set_gateway_account_max_inflight`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - limit: 参数 limit
///
/// # 返回
/// 返回函数执行结果
pub fn set_gateway_account_max_inflight(limit: usize) -> Result<usize, String> {
    let applied = gateway::set_account_max_inflight_limit(limit);
    save_persisted_app_setting(
        APP_SETTING_GATEWAY_ACCOUNT_MAX_INFLIGHT_KEY,
        Some(&applied.to_string()),
    )?;
    Ok(applied)
}

/// 函数 `current_gateway_account_max_inflight`
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
pub fn current_gateway_account_max_inflight() -> usize {
    gateway::account_max_inflight_limit()
}

/// 函数 `set_gateway_request_compression_enabled`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - enabled: 参数 enabled
///
/// # 返回
/// 返回函数执行结果
pub fn set_gateway_request_compression_enabled(enabled: bool) -> Result<bool, String> {
    let applied = gateway::set_request_compression_enabled(enabled);
    save_persisted_bool_setting(APP_SETTING_GATEWAY_REQUEST_COMPRESSION_ENABLED_KEY, applied)?;
    Ok(applied)
}

/// 函数 `current_gateway_request_compression_enabled`
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
pub fn current_gateway_request_compression_enabled() -> bool {
    gateway::request_compression_enabled()
}

/// 函数 `set_gateway_originator`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - originator: 参数 originator
///
/// # 返回
/// 返回函数执行结果
pub fn set_gateway_originator(originator: &str) -> Result<String, String> {
    let applied = gateway::set_originator(originator)?;
    save_persisted_app_setting(APP_SETTING_GATEWAY_ORIGINATOR_KEY, Some(&applied))?;
    Ok(applied)
}

/// 函数 `current_gateway_originator`
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
pub fn current_gateway_originator() -> String {
    gateway::current_originator()
}

/// 函数 `set_gateway_user_agent_version`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - version: 参数 version
///
/// # 返回
/// 返回函数执行结果
pub fn set_gateway_user_agent_version(version: &str) -> Result<String, String> {
    let applied = gateway::set_codex_user_agent_version(version)?;
    save_persisted_app_setting(APP_SETTING_GATEWAY_USER_AGENT_VERSION_KEY, Some(&applied))?;
    Ok(applied)
}

/// 函数 `current_gateway_user_agent_version`
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
pub fn current_gateway_user_agent_version() -> String {
    gateway::current_codex_user_agent_version()
}

/// 函数 `set_gateway_residency_requirement`
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
pub fn set_gateway_residency_requirement(value: Option<&str>) -> Result<Option<String>, String> {
    let normalized = normalize_optional_text(value);
    let applied = gateway::set_residency_requirement(normalized.as_deref())?;
    save_persisted_app_setting(
        APP_SETTING_GATEWAY_RESIDENCY_REQUIREMENT_KEY,
        applied.as_deref(),
    )?;
    Ok(applied)
}

/// 函数 `current_gateway_residency_requirement`
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
pub fn current_gateway_residency_requirement() -> Option<String> {
    gateway::current_residency_requirement()
}

/// 函数 `residency_requirement_options`
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
pub fn residency_requirement_options() -> &'static [&'static str] {
    &["", "us"]
}

/// 函数 `set_gateway_upstream_proxy_url`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - proxy_url: 参数 proxy_url
///
/// # 返回
/// 返回函数执行结果
pub fn set_gateway_upstream_proxy_url(proxy_url: Option<&str>) -> Result<Option<String>, String> {
    let normalized = normalize_optional_text(proxy_url);
    let applied = gateway::set_upstream_proxy_url(normalized.as_deref())?;
    save_persisted_app_setting(
        APP_SETTING_GATEWAY_UPSTREAM_PROXY_URL_KEY,
        applied.as_deref(),
    )?;
    Ok(applied)
}

/// 函数 `set_gateway_upstream_stream_timeout_ms`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - timeout_ms: 参数 timeout_ms
///
/// # 返回
/// 返回函数执行结果
pub fn set_gateway_upstream_stream_timeout_ms(timeout_ms: u64) -> Result<u64, String> {
    let applied = gateway::set_upstream_stream_timeout_ms(timeout_ms);
    save_persisted_app_setting(
        APP_SETTING_GATEWAY_UPSTREAM_STREAM_TIMEOUT_MS_KEY,
        Some(&applied.to_string()),
    )?;
    Ok(applied)
}

/// 函数 `current_gateway_upstream_stream_timeout_ms`
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
pub fn current_gateway_upstream_stream_timeout_ms() -> u64 {
    gateway::current_upstream_stream_timeout_ms()
}

/// 函数 `set_gateway_sse_keepalive_interval_ms`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - interval_ms: 参数 interval_ms
///
/// # 返回
/// 返回函数执行结果
pub fn set_gateway_sse_keepalive_interval_ms(interval_ms: u64) -> Result<u64, String> {
    let applied = gateway::set_sse_keepalive_interval_ms(interval_ms)?;
    save_persisted_app_setting(
        APP_SETTING_GATEWAY_SSE_KEEPALIVE_INTERVAL_MS_KEY,
        Some(&applied.to_string()),
    )?;
    Ok(applied)
}

/// 函数 `current_gateway_sse_keepalive_interval_ms`
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
pub fn current_gateway_sse_keepalive_interval_ms() -> u64 {
    gateway::current_sse_keepalive_interval_ms()
}

/// 函数 `set_gateway_background_tasks`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - input: 参数 input
///
/// # 返回
/// 返回函数执行结果
pub fn set_gateway_background_tasks(
    input: BackgroundTasksInput,
) -> Result<serde_json::Value, String> {
    let applied = usage_refresh::set_background_tasks_settings(input.into_patch());
    let raw = serde_json::to_string(&applied)
        .map_err(|err| format!("serialize background tasks failed: {err}"))?;
    save_persisted_app_setting(APP_SETTING_GATEWAY_BACKGROUND_TASKS_KEY, Some(&raw))?;
    serde_json::to_value(applied).map_err(|err| err.to_string())
}

/// 函数 `current_background_tasks_snapshot_value`
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
pub(crate) fn current_background_tasks_snapshot_value() -> Result<serde_json::Value, String> {
    serde_json::to_value(usage_refresh::background_tasks_settings()).map_err(|err| err.to_string())
}

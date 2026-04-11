use codexmanager_core::rpc::types::{RequestLogFilterSummaryResult, RequestLogSourceCountResult};

use crate::storage_helpers::open_storage;

use super::list::{normalize_optional_text, normalize_source_filter, normalize_status_filter};

/// 函数 `read_request_log_filter_summary`
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
pub(crate) fn read_request_log_filter_summary(
    query: Option<String>,
    status_filter: Option<String>,
    source_filter: Option<String>,
) -> Result<RequestLogFilterSummaryResult, String> {
    let storage = open_storage().ok_or_else(|| "open storage failed".to_string())?;
    let query = normalize_optional_text(query);
    let status_filter = normalize_status_filter(status_filter);
    let source_filter = normalize_source_filter(source_filter);
    let total_count = storage
        .count_request_logs(query.as_deref(), None, source_filter.as_deref())
        .map_err(|err| format!("count request logs failed: {err}"))?;
    let filtered = storage
        .summarize_request_logs_filtered(
            query.as_deref(),
            status_filter.as_deref(),
            source_filter.as_deref(),
        )
        .map_err(|err| format!("summarize request logs failed: {err}"))?;
    let source_breakdown = storage
        .summarize_request_logs_by_source(
            query.as_deref(),
            status_filter.as_deref(),
            source_filter.as_deref(),
        )
        .map_err(|err| format!("summarize request logs by source failed: {err}"))?
        .into_iter()
        .map(|(source, count)| RequestLogSourceCountResult { source, count })
        .collect();

    Ok(RequestLogFilterSummaryResult {
        total_count,
        filtered_count: filtered.count,
        success_count: filtered.success_count,
        error_count: filtered.error_count,
        total_tokens: filtered.total_tokens,
        total_cost_usd: filtered.estimated_cost_usd,
        source_breakdown,
    })
}

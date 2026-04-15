use super::*;

/// 函数 `effective_request_timeout_non_stream_uses_total_only`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// 无
///
/// # 返回
/// 无
#[test]
fn effective_request_timeout_non_stream_uses_total_only() {
    assert_eq!(
        effective_request_timeout(
            Some(Duration::from_secs(120)),
            Some(Duration::from_secs(300)),
            false
        ),
        Some(Duration::from_secs(120))
    );
    assert_eq!(
        effective_request_timeout(None, Some(Duration::from_secs(300)), false),
        None
    );
}

/// 函数 `effective_request_timeout_stream_keeps_total_deadline_only`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// 无
///
/// # 返回
/// 无
#[test]
fn effective_request_timeout_stream_keeps_total_deadline_only() {
    assert_eq!(
        effective_request_timeout(
            Some(Duration::from_secs(120)),
            Some(Duration::from_secs(300)),
            true
        ),
        Some(Duration::from_secs(120))
    );
    assert_eq!(
        effective_request_timeout(
            Some(Duration::from_secs(300)),
            Some(Duration::from_secs(120)),
            true
        ),
        Some(Duration::from_secs(300))
    );
}

/// 函数 `effective_request_timeout_stream_ignores_stream_only_timeout`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// 无
///
/// # 返回
/// 无
#[test]
fn effective_request_timeout_stream_ignores_stream_only_timeout() {
    assert_eq!(
        effective_request_timeout(Some(Duration::from_secs(120)), None, true),
        Some(Duration::from_secs(120))
    );
    assert_eq!(
        effective_request_timeout(None, Some(Duration::from_secs(300)), true),
        None
    );
    assert_eq!(effective_request_timeout(None, None, true), None);
}

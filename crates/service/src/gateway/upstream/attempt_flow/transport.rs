use bytes::Bytes;
use codexmanager_core::storage::Account;
use std::time::Instant;
use tiny_http::Request;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RequestCompression {
    None,
    Zstd,
}

#[derive(Debug, Clone, Copy)]
pub(in super::super) struct UpstreamRequestContext<'a> {
    pub(in super::super) request_path: &'a str,
}

impl<'a> UpstreamRequestContext<'a> {
    /// 函数 `from_request`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - in super: 参数 in super
    ///
    /// # 返回
    /// 返回函数执行结果
    pub(in super::super) fn from_request(request: &'a Request) -> Self {
        Self {
            request_path: request.url(),
        }
    }
}

/// 函数 `should_force_connection_close`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - target_url: 参数 target_url
///
/// # 返回
/// 返回函数执行结果
fn should_force_connection_close(target_url: &str) -> bool {
    reqwest::Url::parse(target_url)
        .ok()
        .and_then(|url| url.host_str().map(|host| host.to_ascii_lowercase()))
        .is_some_and(|host| matches!(host.as_str(), "127.0.0.1" | "localhost" | "::1"))
}

/// 函数 `force_connection_close`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - headers: 参数 headers
///
/// # 返回
/// 无
fn force_connection_close(headers: &mut Vec<(String, String)>) {
    if let Some((_, value)) = headers
        .iter_mut()
        .find(|(name, _)| name.eq_ignore_ascii_case("connection"))
    {
        *value = "close".to_string();
    } else {
        headers.push(("Connection".to_string(), "close".to_string()));
    }
}

/// 函数 `extract_prompt_cache_key`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - body: 参数 body
///
/// # 返回
/// 返回函数执行结果
fn extract_prompt_cache_key(body: &[u8]) -> Option<String> {
    if body.is_empty() || body.len() > 64 * 1024 {
        return None;
    }
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(body) else {
        return None;
    };
    value
        .get("prompt_cache_key")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
}

/// 函数 `is_compact_request_path`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - path: 参数 path
///
/// # 返回
/// 返回函数执行结果
fn is_compact_request_path(path: &str) -> bool {
    path == "/v1/responses/compact" || path.starts_with("/v1/responses/compact?")
}

/// 函数 `has_header`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - headers: 参数 headers
/// - name: 参数 name
///
/// # 返回
/// 返回函数执行结果
fn has_header(headers: &[(String, String)], name: &str) -> bool {
    headers
        .iter()
        .any(|(header_name, _)| header_name.eq_ignore_ascii_case(name))
}

/// 函数 `resolve_chatgpt_account_header`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - account: 参数 account
/// - target_url: 参数 target_url
///
/// # 返回
/// 返回函数执行结果
fn resolve_chatgpt_account_header<'a>(account: &'a Account, target_url: &str) -> Option<&'a str> {
    if !super::super::config::should_send_chatgpt_account_header(target_url) {
        return None;
    }
    account
        .chatgpt_account_id
        .as_deref()
        .or(account.workspace_id.as_deref())
}

/// 函数 `resolve_request_compression_with_flag`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - enabled: 参数 enabled
/// - target_url: 参数 target_url
/// - request_path: 参数 request_path
/// - is_stream: 参数 is_stream
///
/// # 返回
/// 返回函数执行结果
fn resolve_request_compression_with_flag(
    enabled: bool,
    target_url: &str,
    request_path: &str,
    is_stream: bool,
) -> RequestCompression {
    if !enabled {
        return RequestCompression::None;
    }
    if !is_stream {
        return RequestCompression::None;
    }
    if is_compact_request_path(request_path) || !request_path.starts_with("/v1/responses") {
        return RequestCompression::None;
    }
    if !super::super::config::is_chatgpt_backend_base(target_url) {
        return RequestCompression::None;
    }
    RequestCompression::Zstd
}

/// 函数 `resolve_request_compression`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - target_url: 参数 target_url
/// - request_path: 参数 request_path
/// - is_stream: 参数 is_stream
///
/// # 返回
/// 返回函数执行结果
fn resolve_request_compression(
    target_url: &str,
    request_path: &str,
    is_stream: bool,
) -> RequestCompression {
    resolve_request_compression_with_flag(
        super::super::super::request_compression_enabled(),
        target_url,
        request_path,
        is_stream,
    )
}

fn should_retry_transport_without_compression(
    target_url: &str,
    request_path: &str,
    is_stream: bool,
    compression: RequestCompression,
) -> bool {
    compression == RequestCompression::Zstd
        && is_stream
        && request_path.starts_with("/v1/responses")
        && !is_compact_request_path(request_path)
        && super::super::config::is_chatgpt_backend_base(target_url)
}

/// 函数 `encode_request_body`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - request_path: 参数 request_path
/// - body: 参数 body
/// - compression: 参数 compression
/// - headers: 参数 headers
///
/// # 返回
/// 返回函数执行结果
fn encode_request_body(
    request_path: &str,
    body: &Bytes,
    compression: RequestCompression,
    headers: &mut Vec<(String, String)>,
) -> Bytes {
    if body.is_empty() || compression == RequestCompression::None {
        return body.clone();
    }
    if has_header(headers, "Content-Encoding") {
        log::warn!(
            "event=gateway_request_compression_skipped reason=content_encoding_exists path={}",
            request_path
        );
        return body.clone();
    }
    match compression {
        RequestCompression::None => body.clone(),
        RequestCompression::Zstd => {
            match zstd::stream::encode_all(std::io::Cursor::new(body.as_ref()), 3) {
                Ok(compressed) => {
                    let post_bytes = compressed.len();
                    headers.push(("Content-Encoding".to_string(), "zstd".to_string()));
                    log::info!(
                    "event=gateway_request_compressed path={} algorithm=zstd pre_bytes={} post_bytes={}",
                    request_path,
                    body.len(),
                    post_bytes
                );
                    Bytes::from(compressed)
                }
                Err(err) => {
                    log::warn!(
                        "event=gateway_request_compression_failed path={} algorithm=zstd err={}",
                        request_path,
                        err
                    );
                    body.clone()
                }
            }
        }
    }
}

/// 函数 `send_upstream_request`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - in super: 参数 in super
///
/// # 返回
/// 返回函数执行结果
pub(in super::super) fn send_upstream_request(
    client: &reqwest::blocking::Client,
    method: &reqwest::Method,
    target_url: &str,
    request_deadline: Option<Instant>,
    request_ctx: UpstreamRequestContext<'_>,
    incoming_headers: &super::super::super::IncomingHeaderSnapshot,
    body: &Bytes,
    is_stream: bool,
    auth_token: &str,
    account: &Account,
    strip_session_affinity: bool,
) -> Result<reqwest::blocking::Response, reqwest::Error> {
    send_upstream_request_with_compression_override(
        client,
        method,
        target_url,
        request_deadline,
        request_ctx,
        incoming_headers,
        body,
        is_stream,
        auth_token,
        account,
        strip_session_affinity,
        None,
    )
}

/// 函数 `send_upstream_request_without_compression`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-04
///
/// # 参数
/// - in super: 参数 in super
///
/// # 返回
/// 返回函数执行结果
pub(in super::super) fn send_upstream_request_without_compression(
    client: &reqwest::blocking::Client,
    method: &reqwest::Method,
    target_url: &str,
    request_deadline: Option<Instant>,
    request_ctx: UpstreamRequestContext<'_>,
    incoming_headers: &super::super::super::IncomingHeaderSnapshot,
    body: &Bytes,
    is_stream: bool,
    auth_token: &str,
    account: &Account,
    strip_session_affinity: bool,
) -> Result<reqwest::blocking::Response, reqwest::Error> {
    send_upstream_request_with_compression_override(
        client,
        method,
        target_url,
        request_deadline,
        request_ctx,
        incoming_headers,
        body,
        is_stream,
        auth_token,
        account,
        strip_session_affinity,
        Some(RequestCompression::None),
    )
}

/// 函数 `send_upstream_request_with_compression_override`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-04
///
/// # 参数
/// - compression_override: 参数 compression_override
///
/// # 返回
/// 返回函数执行结果
fn send_upstream_request_with_compression_override(
    client: &reqwest::blocking::Client,
    method: &reqwest::Method,
    target_url: &str,
    request_deadline: Option<Instant>,
    request_ctx: UpstreamRequestContext<'_>,
    incoming_headers: &super::super::super::IncomingHeaderSnapshot,
    body: &Bytes,
    is_stream: bool,
    auth_token: &str,
    account: &Account,
    strip_session_affinity: bool,
    compression_override: Option<RequestCompression>,
) -> Result<reqwest::blocking::Response, reqwest::Error> {
    let attempt_started_at = Instant::now();
    let prompt_cache_key = extract_prompt_cache_key(body.as_ref());
    let is_compact_request = is_compact_request_path(request_ctx.request_path);
    let request_affinity = super::super::super::session_affinity::derive_outgoing_session_affinity(
        incoming_headers.session_id(),
        incoming_headers.client_request_id(),
        incoming_headers.turn_state(),
        incoming_headers.conversation_id(),
        prompt_cache_key.as_deref(),
    );
    let account_id = account
        .chatgpt_account_id
        .as_deref()
        .or_else(|| account.workspace_id.as_deref());
    super::super::super::session_affinity::log_thread_anchor_conflict(
        request_ctx.request_path,
        account_id,
        incoming_headers.conversation_id(),
        prompt_cache_key.as_deref(),
    );
    super::super::super::session_affinity::log_outgoing_session_affinity(
        request_ctx.request_path,
        account_id,
        incoming_headers.session_id(),
        incoming_headers.client_request_id(),
        incoming_headers.turn_state(),
        incoming_headers.conversation_id(),
        prompt_cache_key.as_deref(),
        request_affinity,
        strip_session_affinity,
    );
    let mut upstream_headers = if is_compact_request {
        let header_input = super::super::header_profile::CodexCompactUpstreamHeaderInput {
            auth_token,
            chatgpt_account_id: resolve_chatgpt_account_header(account, target_url),
            incoming_session_id: request_affinity.incoming_session_id,
            incoming_window_id: incoming_headers.window_id(),
            incoming_subagent: incoming_headers.subagent(),
            incoming_parent_thread_id: incoming_headers.parent_thread_id(),
            passthrough_codex_headers: incoming_headers.passthrough_codex_headers(),
            fallback_session_id: request_affinity.fallback_session_id,
            strip_session_affinity,
            has_body: !body.is_empty(),
        };
        super::super::header_profile::build_codex_compact_upstream_headers(header_input)
    } else {
        let header_input = super::super::header_profile::CodexUpstreamHeaderInput {
            auth_token,
            chatgpt_account_id: resolve_chatgpt_account_header(account, target_url),
            incoming_session_id: request_affinity.incoming_session_id,
            incoming_window_id: incoming_headers.window_id(),
            incoming_client_request_id: request_affinity.incoming_client_request_id,
            incoming_subagent: incoming_headers.subagent(),
            incoming_beta_features: incoming_headers.beta_features(),
            incoming_turn_metadata: incoming_headers.turn_metadata(),
            incoming_parent_thread_id: incoming_headers.parent_thread_id(),
            passthrough_codex_headers: incoming_headers.passthrough_codex_headers(),
            fallback_session_id: request_affinity.fallback_session_id,
            incoming_turn_state: request_affinity.incoming_turn_state,
            include_turn_state: true,
            strip_session_affinity,
            has_body: !body.is_empty(),
        };
        super::super::header_profile::build_codex_upstream_headers(header_input)
    };
    if should_force_connection_close(target_url) {
        // 中文注释：本地 loopback mock/代理更容易复用到脏 keep-alive 连接；
        // 对 localhost/127.0.0.1 强制 close，避免请求落到已失效连接。
        force_connection_close(&mut upstream_headers);
    }
    let upstream_headers_uncompressed = upstream_headers.clone();
    let request_compression = compression_override.unwrap_or_else(|| {
        resolve_request_compression(target_url, request_ctx.request_path, is_stream)
    });
    let body_for_request = encode_request_body(
        request_ctx.request_path,
        body,
        request_compression,
        &mut upstream_headers,
    );
    let build_request = |http: &reqwest::blocking::Client,
                         request_headers: &[(String, String)],
                         request_body: &Bytes| {
        let mut builder = http.request(method.clone(), target_url);
        if let Some(timeout) =
            super::super::support::deadline::send_timeout(request_deadline, is_stream)
        {
            builder = builder.timeout(timeout);
        }
        for (name, value) in request_headers.iter() {
            builder = builder.header(name, value);
        }
        if !request_body.is_empty() {
            builder = builder.body(request_body.clone());
        }
        builder
    };

    let result = match build_request(client, upstream_headers.as_slice(), &body_for_request).send()
    {
        Ok(resp) => Ok(resp),
        Err(first_err) => {
            let fresh = super::super::super::fresh_upstream_client_for_account(account.id.as_str());
            if should_retry_transport_without_compression(
                target_url,
                request_ctx.request_path,
                is_stream,
                request_compression,
            ) {
                log::warn!(
                    "event=gateway_transport_retry_without_compression path={} account_id={} target_url={} first_err={}",
                    request_ctx.request_path,
                    account.id,
                    target_url,
                    first_err
                );
                match build_request(&fresh, upstream_headers_uncompressed.as_slice(), body).send() {
                    Ok(resp) => {
                        log::warn!(
                            "event=gateway_transport_retry_without_compression_succeeded path={} account_id={} target_url={}",
                            request_ctx.request_path,
                            account.id,
                            target_url
                        );
                        Ok(resp)
                    }
                    Err(second_err) => {
                        log::warn!(
                            "event=gateway_transport_retry_without_compression_failed path={} account_id={} target_url={} first_err={} retry_err={}",
                            request_ctx.request_path,
                            account.id,
                            target_url,
                            first_err,
                            second_err
                        );
                        Err(second_err)
                    }
                }
            } else {
                // 中文注释：进程启动后才开启系统代理时，旧单例 client 可能仍走旧网络路径；
                // 这里用 fresh client 立刻重试一次，避免必须手动重连服务。
                match build_request(&fresh, upstream_headers.as_slice(), &body_for_request).send() {
                    Ok(resp) => {
                        log::info!(
                            "event=gateway_transport_retry_with_fresh_client_succeeded path={} account_id={} target_url={}",
                            request_ctx.request_path,
                            account.id,
                            target_url
                        );
                        Ok(resp)
                    }
                    Err(second_err) => {
                        log::warn!(
                            "event=gateway_transport_retry_with_fresh_client_failed path={} account_id={} target_url={} first_err={} retry_err={}",
                            request_ctx.request_path,
                            account.id,
                            target_url,
                            first_err,
                            second_err
                        );
                        Err(second_err)
                    }
                }
            }
        }
    };
    let duration_ms = super::super::super::duration_to_millis(attempt_started_at.elapsed());
    super::super::super::metrics::record_gateway_upstream_attempt(duration_ms, result.is_err());
    result
}

#[cfg(test)]
mod tests {
    use super::{
        encode_request_body, resolve_request_compression_with_flag,
        should_retry_transport_without_compression, RequestCompression,
    };
    use bytes::Bytes;

    /// 函数 `request_compression_only_applies_to_streaming_chatgpt_responses`
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
    fn request_compression_only_applies_to_streaming_chatgpt_responses() {
        assert_eq!(
            resolve_request_compression_with_flag(
                true,
                "https://chatgpt.com/backend-api/codex/responses",
                "/v1/responses",
                true
            ),
            RequestCompression::Zstd
        );
        assert_eq!(
            resolve_request_compression_with_flag(
                true,
                "https://chatgpt.com/backend-api/codex/responses",
                "/v1/responses/compact",
                true
            ),
            RequestCompression::None
        );
        assert_eq!(
            resolve_request_compression_with_flag(
                true,
                "https://api.openai.com/v1/responses",
                "/v1/responses",
                true
            ),
            RequestCompression::None
        );
        assert_eq!(
            resolve_request_compression_with_flag(
                true,
                "https://chatgpt.com/backend-api/codex/responses",
                "/v1/responses",
                false
            ),
            RequestCompression::None
        );
        assert_eq!(
            resolve_request_compression_with_flag(
                false,
                "https://chatgpt.com/backend-api/codex/responses",
                "/v1/responses",
                true
            ),
            RequestCompression::None
        );
    }

    /// 函数 `encode_request_body_adds_zstd_content_encoding`
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
    fn encode_request_body_adds_zstd_content_encoding() {
        let body = Bytes::from_static(br#"{"model":"gpt-5.4","input":"compress me"}"#);
        let mut headers = vec![("Content-Type".to_string(), "application/json".to_string())];

        let actual = encode_request_body(
            "/v1/responses",
            &body,
            RequestCompression::Zstd,
            &mut headers,
        );

        assert!(headers.iter().any(|(name, value)| {
            name.eq_ignore_ascii_case("Content-Encoding") && value == "zstd"
        }));
        let decoded = zstd::stream::decode_all(std::io::Cursor::new(actual.as_ref()))
            .expect("decode zstd body");
        let value: serde_json::Value =
            serde_json::from_slice(&decoded).expect("parse decompressed json");
        assert_eq!(
            value.get("model").and_then(serde_json::Value::as_str),
            Some("gpt-5.4")
        );
    }

    #[test]
    fn transport_retry_without_compression_only_targets_streaming_chatgpt_responses() {
        assert!(should_retry_transport_without_compression(
            "https://chatgpt.com/backend-api/codex/responses",
            "/v1/responses",
            true,
            RequestCompression::Zstd
        ));
        assert!(!should_retry_transport_without_compression(
            "https://chatgpt.com/backend-api/codex/responses",
            "/v1/responses/compact",
            true,
            RequestCompression::Zstd
        ));
        assert!(!should_retry_transport_without_compression(
            "https://api.openai.com/v1/responses",
            "/v1/responses",
            true,
            RequestCompression::Zstd
        ));
        assert!(!should_retry_transport_without_compression(
            "https://chatgpt.com/backend-api/codex/responses",
            "/v1/responses",
            false,
            RequestCompression::Zstd
        ));
        assert!(!should_retry_transport_without_compression(
            "https://chatgpt.com/backend-api/codex/responses",
            "/v1/responses",
            true,
            RequestCompression::None
        ));
    }
}

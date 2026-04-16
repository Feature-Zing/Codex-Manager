use super::{
    apply_openai_stream_meta_defaults, build_completion_fallback_text_chunk,
    classify_upstream_stream_read_error, collector_output_text_trimmed,
    convert_openai_completions_stream_chunk, extract_openai_completed_output_text,
    extract_sse_frame_payload, inspect_sse_frame, is_response_completed_event_name,
    map_chunk_has_completion_text, mark_collector_terminal_success, mark_first_response_ms,
    merge_usage, parse_sse_frame_json, should_emit_keepalive,
    should_skip_completion_live_text_event, stream_idle_timed_out, stream_idle_timeout_message,
    stream_reader_disconnected_message, stream_wait_timeout, update_openai_stream_meta,
    upstream_hint_or_stream_incomplete_message, Arc, Cursor, Mutex, OpenAIStreamMeta,
    PassthroughSseCollector, Read, SseKeepAliveFrame, SseTerminal, UpstreamSseFramePump,
    UpstreamSseFramePumpItem, Value,
};
use std::time::Instant;

pub(crate) struct OpenAICompletionsSseReader {
    upstream: UpstreamSseFramePump,
    out_cursor: Cursor<Vec<u8>>,
    usage_collector: Arc<Mutex<PassthroughSseCollector>>,
    stream_meta: OpenAIStreamMeta,
    emitted_text_delta: bool,
    request_started_at: Instant,
    last_upstream_activity: Instant,
    saw_upstream_frame: bool,
    finished: bool,
}

impl OpenAICompletionsSseReader {
    /// 函数 `new`
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
    pub(crate) fn new(
        upstream: reqwest::blocking::Response,
        usage_collector: Arc<Mutex<PassthroughSseCollector>>,
        request_started_at: Instant,
    ) -> Self {
        Self {
            upstream: UpstreamSseFramePump::new(upstream),
            out_cursor: Cursor::new(Vec::new()),
            usage_collector,
            stream_meta: OpenAIStreamMeta::default(),
            emitted_text_delta: false,
            request_started_at,
            last_upstream_activity: Instant::now(),
            saw_upstream_frame: false,
            finished: false,
        }
    }

    /// 函数 `update_usage_from_frame`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - self: 参数 self
    /// - lines: 参数 lines
    ///
    /// # 返回
    /// 无
    fn update_usage_from_frame(&self, lines: &[String]) {
        let inspection = inspect_sse_frame(lines);
        if inspection.usage.is_none() && inspection.terminal.is_none() {
            return;
        }
        if let Ok(mut collector) = self.usage_collector.lock() {
            if let Some(event_type) = inspection.last_event_type {
                collector.last_event_type = Some(event_type);
            }
            if let Some(parsed) = inspection.usage {
                merge_usage(&mut collector.usage, parsed);
            }
            if let Some(terminal) = inspection.terminal {
                collector.saw_terminal = true;
                if let SseTerminal::Err(message) = terminal {
                    collector.terminal_error = Some(message);
                }
            }
        }
    }

    /// 函数 `try_build_completion_fallback_stream`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - self: 参数 self
    /// - include_done: 参数 include_done
    ///
    /// # 返回
    /// 返回函数执行结果
    fn try_build_completion_fallback_stream(&mut self, include_done: bool) -> Option<Vec<u8>> {
        if self.emitted_text_delta {
            return None;
        }
        let fallback_text = collector_output_text_trimmed(&self.usage_collector)?;
        let mut fallback_chunk =
            build_completion_fallback_text_chunk(&self.stream_meta, fallback_text.as_str());
        apply_openai_stream_meta_defaults(&mut fallback_chunk, &self.stream_meta);
        let payload = serde_json::to_string(&fallback_chunk).unwrap_or_else(|_| "{}".to_string());
        let mut out = format!("data: {payload}\n\n");
        self.emitted_text_delta = true;
        mark_first_response_ms(&self.usage_collector, self.request_started_at);
        if include_done {
            out.push_str("data: [DONE]\n\n");
            self.finished = true;
        }
        mark_collector_terminal_success(&self.usage_collector);
        Some(out.into_bytes())
    }

    /// 函数 `map_frame_to_completions_sse`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - self: 参数 self
    /// - lines: 参数 lines
    ///
    /// # 返回
    /// 返回函数执行结果
    fn map_frame_to_completions_sse(&mut self, lines: &[String]) -> Vec<u8> {
        let Some(data) = extract_sse_frame_payload(lines) else {
            return Vec::new();
        };
        if data.trim() == "[DONE]" {
            if let Some(fallback) = self.try_build_completion_fallback_stream(true) {
                return fallback;
            }
            self.finished = true;
            return b"data: [DONE]\n\n".to_vec();
        }

        let Some(value) = parse_sse_frame_json(lines) else {
            return Vec::new();
        };
        update_openai_stream_meta(&mut self.stream_meta, &value);
        let event_type = value
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if event_type == "response.created" {
            return Vec::new();
        }

        let mut out = String::new();
        if is_response_completed_event_name(event_type) && !self.emitted_text_delta {
            if let Some(fallback_text) = extract_openai_completed_output_text(&value) {
                let mut fallback_chunk =
                    build_completion_fallback_text_chunk(&self.stream_meta, fallback_text.as_str());
                apply_openai_stream_meta_defaults(&mut fallback_chunk, &self.stream_meta);
                let payload =
                    serde_json::to_string(&fallback_chunk).unwrap_or_else(|_| "{}".to_string());
                out.push_str(format!("data: {payload}\n\n").as_str());
                self.emitted_text_delta = true;
                mark_first_response_ms(&self.usage_collector, self.request_started_at);
            }
        }

        if should_skip_completion_live_text_event(event_type, &value) {
            return out.into_bytes();
        }

        if let Some(mut mapped) = convert_openai_completions_stream_chunk(&value) {
            apply_openai_stream_meta_defaults(&mut mapped, &self.stream_meta);
            if map_chunk_has_completion_text(&mapped) {
                self.emitted_text_delta = true;
                mark_first_response_ms(&self.usage_collector, self.request_started_at);
            }
            let payload = serde_json::to_string(&mapped).unwrap_or_else(|_| "{}".to_string());
            out.push_str(format!("data: {payload}\n\n").as_str());
        }

        if is_response_completed_event_name(event_type) {
            out.push_str("data: [DONE]\n\n");
            self.finished = true;
        }

        out.into_bytes()
    }

    /// 函数 `next_chunk`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - self: 参数 self
    ///
    /// # 返回
    /// 返回函数执行结果
    fn next_chunk(&mut self) -> std::io::Result<Vec<u8>> {
        loop {
            match self
                .upstream
                .recv_timeout(stream_wait_timeout(self.last_upstream_activity))
            {
                Ok(UpstreamSseFramePumpItem::Frame(frame)) => {
                    self.last_upstream_activity = Instant::now();
                    self.saw_upstream_frame = true;
                    self.update_usage_from_frame(&frame);
                    let mapped = self.map_frame_to_completions_sse(&frame);
                    if !mapped.is_empty() {
                        mark_first_response_ms(&self.usage_collector, self.request_started_at);
                        return Ok(mapped);
                    }
                    continue;
                }
                Ok(UpstreamSseFramePumpItem::Eof) => {
                    self.last_upstream_activity = Instant::now();
                    if let Some(fallback) = self.try_build_completion_fallback_stream(true) {
                        return Ok(fallback);
                    }
                    if let Ok(mut collector) = self.usage_collector.lock() {
                        if !collector.saw_terminal {
                            // 中文注释：对齐最新 Codex SSE 语义：
                            // 仅凭已收到文本不足以判定成功，必须等到真正 terminal 事件。
                            let hint = collector.upstream_error_hint.clone();
                            collector.terminal_error.get_or_insert_with(|| {
                                upstream_hint_or_stream_incomplete_message(hint.as_deref())
                            });
                        }
                    }
                    self.finished = true;
                    return Ok(Vec::new());
                }
                Ok(UpstreamSseFramePumpItem::Error(err)) => {
                    self.last_upstream_activity = Instant::now();
                    if let Ok(mut collector) = self.usage_collector.lock() {
                        collector
                            .terminal_error
                            .get_or_insert_with(|| classify_upstream_stream_read_error(&err));
                    }
                    self.finished = true;
                    return Ok(Vec::new());
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    if stream_idle_timed_out(self.last_upstream_activity) {
                        if let Ok(mut collector) = self.usage_collector.lock() {
                            collector
                                .terminal_error
                                .get_or_insert_with(stream_idle_timeout_message);
                        }
                        self.finished = true;
                        return Ok(Vec::new());
                    }
                    if should_emit_keepalive(self.saw_upstream_frame) {
                        return Ok(SseKeepAliveFrame::OpenAICompletions.bytes().to_vec());
                    }
                    continue;
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    if let Ok(mut collector) = self.usage_collector.lock() {
                        let hint = collector.upstream_error_hint.clone();
                        collector.terminal_error.get_or_insert_with(|| {
                            hint.unwrap_or_else(stream_reader_disconnected_message)
                        });
                    }
                    self.finished = true;
                    return Ok(Vec::new());
                }
            }
        }
    }
}

impl Read for OpenAICompletionsSseReader {
    /// 函数 `read`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - self: 参数 self
    /// - buf: 参数 buf
    ///
    /// # 返回
    /// 返回函数执行结果
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        loop {
            let read = self.out_cursor.read(buf)?;
            if read > 0 {
                return Ok(read);
            }
            if self.finished {
                return Ok(0);
            }
            self.out_cursor = Cursor::new(self.next_chunk()?);
        }
    }
}

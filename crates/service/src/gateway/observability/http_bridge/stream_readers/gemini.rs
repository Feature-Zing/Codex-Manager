use super::{
    append_output_text, classify_upstream_stream_read_error, collect_output_text_from_event_fields,
    json, mark_collector_terminal_success, mark_first_response_ms, stream_idle_timed_out,
    stream_idle_timeout_message, stream_reader_disconnected_message, stream_wait_timeout,
    upstream_hint_or_stream_incomplete_message, Arc, Cursor, Map, Mutex, PassthroughSseCollector,
    Read, ToolNameRestoreMap, UpstreamSseFramePump, UpstreamSseFramePumpItem, Value,
};
use crate::gateway::{build_gemini_error_body, GeminiStreamOutputMode};
use std::collections::BTreeMap;
use std::time::Instant;

pub(crate) struct GeminiSseReader {
    upstream: UpstreamSseFramePump,
    out_cursor: Cursor<Vec<u8>>,
    state: GeminiSseState,
    usage_collector: Arc<Mutex<PassthroughSseCollector>>,
    tool_name_restore_map: Option<ToolNameRestoreMap>,
    output_mode: GeminiStreamOutputMode,
    wrap_response_envelope: bool,
    request_started_at: Instant,
    last_upstream_activity: Instant,
}

#[derive(Default)]
struct GeminiSseState {
    finished: bool,
    response_id: Option<String>,
    model: Option<String>,
    input_tokens: i64,
    cached_input_tokens: i64,
    output_tokens: i64,
    total_tokens: Option<i64>,
    reasoning_output_tokens: i64,
    output_text: String,
    completed_seen: bool,
    pending_tool_calls: BTreeMap<i64, PendingToolCall>,
    emitted_tool_calls: BTreeMap<i64, String>,
}

#[derive(Default)]
struct PendingToolCall {
    call_id: Option<String>,
    name: Option<String>,
    arguments: String,
}

impl GeminiSseReader {
    pub(crate) fn new(
        upstream: reqwest::blocking::Response,
        usage_collector: Arc<Mutex<PassthroughSseCollector>>,
        tool_name_restore_map: Option<ToolNameRestoreMap>,
        output_mode: GeminiStreamOutputMode,
        wrap_response_envelope: bool,
        request_started_at: Instant,
    ) -> Self {
        Self {
            upstream: UpstreamSseFramePump::new(upstream),
            out_cursor: Cursor::new(Vec::new()),
            state: GeminiSseState::default(),
            usage_collector,
            tool_name_restore_map,
            output_mode,
            wrap_response_envelope,
            request_started_at,
            last_upstream_activity: Instant::now(),
        }
    }

    fn next_chunk(&mut self) -> std::io::Result<Vec<u8>> {
        loop {
            match self
                .upstream
                .recv_timeout(stream_wait_timeout(self.last_upstream_activity))
            {
                Ok(UpstreamSseFramePumpItem::Frame(frame)) => {
                    self.last_upstream_activity = Instant::now();
                    let mapped = self.process_sse_frame(&frame);
                    if !mapped.is_empty() {
                        mark_first_response_ms(&self.usage_collector, self.request_started_at);
                        return Ok(mapped);
                    }
                    continue;
                }
                Ok(UpstreamSseFramePumpItem::Eof) => {
                    self.last_upstream_activity = Instant::now();
                    self.mark_stream_incomplete_if_needed();
                    let finished = self.finish_stream();
                    if !finished.is_empty() {
                        mark_first_response_ms(&self.usage_collector, self.request_started_at);
                    }
                    return Ok(finished);
                }
                Ok(UpstreamSseFramePumpItem::Error(err)) => {
                    self.last_upstream_activity = Instant::now();
                    self.mark_stream_read_error(classify_upstream_stream_read_error(&err));
                    let finished = self.finish_stream();
                    if !finished.is_empty() {
                        mark_first_response_ms(&self.usage_collector, self.request_started_at);
                    }
                    return Ok(finished);
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    if stream_idle_timed_out(self.last_upstream_activity) {
                        self.mark_stream_read_error(stream_idle_timeout_message());
                        let finished = self.finish_stream();
                        if !finished.is_empty() {
                            mark_first_response_ms(&self.usage_collector, self.request_started_at);
                        }
                        return Ok(finished);
                    }
                    continue;
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    self.mark_stream_read_error(stream_reader_disconnected_message());
                    let finished = self.finish_stream();
                    if !finished.is_empty() {
                        mark_first_response_ms(&self.usage_collector, self.request_started_at);
                    }
                    return Ok(finished);
                }
            }
        }
    }

    fn process_sse_frame(&mut self, lines: &[String]) -> Vec<u8> {
        let mut data_lines = Vec::new();
        for line in lines {
            let trimmed = line.trim_end_matches(['\r', '\n']);
            if let Some(rest) = trimmed.strip_prefix("data:") {
                data_lines.push(rest.trim_start().to_string());
            }
        }
        if data_lines.is_empty() {
            return Vec::new();
        }
        let data = data_lines.join("\n");
        if data.trim() == "[DONE]" {
            self.mark_stream_incomplete_if_needed();
            return self.finish_stream();
        }
        let value = match serde_json::from_str::<Value>(&data) {
            Ok(value) => value,
            Err(_) => return Vec::new(),
        };
        self.consume_openai_event(&value)
    }

    fn consume_openai_event(&mut self, value: &Value) -> Vec<u8> {
        self.capture_response_meta(value);
        let Some(event_type) = value.get("type").and_then(Value::as_str) else {
            return Vec::new();
        };
        self.record_last_event_type(event_type);
        let mut out = String::new();
        match event_type {
            "response.output_text.delta" => {
                let fragment = value
                    .get("delta")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if fragment.is_empty() {
                    return Vec::new();
                }
                append_output_text(&mut self.state.output_text, fragment);
                self.append_sse_event(
                    &mut out,
                    &self.build_chunk(vec![json!({ "text": fragment })], None, None),
                );
            }
            "response.output_item.added" | "response.output_item.done" => {
                let Some(item) = value
                    .get("item")
                    .or_else(|| value.get("output_item"))
                    .and_then(Value::as_object)
                else {
                    return Vec::new();
                };
                let item_type = item.get("type").and_then(Value::as_str).unwrap_or_default();
                if !matches!(item_type, "function_call" | "custom_tool_call") {
                    collect_output_text_from_event_fields(value, &mut self.state.output_text);
                    return Vec::new();
                }
                let output_index = value
                    .get("output_index")
                    .and_then(Value::as_i64)
                    .unwrap_or(0);
                let entry = self
                    .state
                    .pending_tool_calls
                    .entry(output_index)
                    .or_default();
                if let Some(call_id) = item
                    .get("call_id")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|current| !current.is_empty())
                {
                    entry.call_id = Some(call_id.to_string());
                }
                if let Some(name) = item
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|current| !current.is_empty())
                {
                    entry.name = Some(name.to_string());
                }
                if let Some(arguments) = extract_function_call_arguments(item) {
                    merge_arguments(&mut entry.arguments, arguments.as_str());
                }
                if event_type == "response.output_item.done"
                    && has_meaningful_tool_arguments(&entry.arguments)
                {
                    self.emit_pending_tool_call(&mut out, output_index);
                }
            }
            "response.function_call_arguments.delta" | "response.function_call_arguments.done" => {
                let output_index = value
                    .get("output_index")
                    .and_then(Value::as_i64)
                    .unwrap_or(0);
                let entry = self
                    .state
                    .pending_tool_calls
                    .entry(output_index)
                    .or_default();
                if let Some(call_id) = value
                    .get("call_id")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|current| !current.is_empty())
                {
                    entry.call_id = Some(call_id.to_string());
                }
                if let Some(delta) = value
                    .get("delta")
                    .or_else(|| value.get("arguments"))
                    .and_then(Value::as_str)
                {
                    merge_arguments(&mut entry.arguments, delta);
                }
                if event_type == "response.function_call_arguments.done"
                    && entry.call_id.is_some()
                    && has_meaningful_tool_arguments(&entry.arguments)
                {
                    self.emit_pending_tool_call(&mut out, output_index);
                }
            }
            _ if event_type.starts_with("response.output_item.")
                || event_type.starts_with("response.content_part.") =>
            {
                collect_output_text_from_event_fields(value, &mut self.state.output_text);
            }
            _ if is_response_completed_event_type(event_type) => {
                self.state.completed_seen = true;
                mark_collector_terminal_success(&self.usage_collector);
                if let Some(response) = value.get("response") {
                    self.emit_completed_response(&mut out, response);
                }
            }
            _ => {}
        }
        out.into_bytes()
    }

    fn emit_pending_tool_call(&mut self, out: &mut String, output_index: i64) {
        let Some(entry) = self.state.pending_tool_calls.get(&output_index) else {
            return;
        };
        let Some(name) = entry.name.as_deref() else {
            return;
        };
        let signature = format!(
            "{}:{}",
            entry.call_id.as_deref().unwrap_or(""),
            entry.arguments
        );
        if self
            .state
            .emitted_tool_calls
            .get(&output_index)
            .is_some_and(|current| current == &signature)
        {
            return;
        }
        self.state
            .emitted_tool_calls
            .insert(output_index, signature);
        let restored_name = restore_tool_name(name, self.tool_name_restore_map.as_ref());
        let args = parse_json_object_or_empty(&entry.arguments);
        let mut function_call = serde_json::Map::new();
        function_call.insert("name".to_string(), Value::String(restored_name));
        function_call.insert("args".to_string(), args);
        if let Some(call_id) = entry.call_id.as_deref() {
            function_call.insert("id".to_string(), Value::String(call_id.to_string()));
        }
        self.append_sse_event(
            out,
            &self.build_chunk(
                vec![json!({ "functionCall": Value::Object(function_call) })],
                None,
                None,
            ),
        );
    }

    fn emit_completed_response(&mut self, out: &mut String, response: &Value) {
        let extracted_message_text = extract_completed_response_message_text(response);
        if self.state.output_text.trim().is_empty()
            && extracted_message_text
                .as_deref()
                .is_some_and(|text| !text.trim().is_empty())
        {
            let extracted_output_text = extracted_message_text.unwrap_or_default();
            append_output_text(&mut self.state.output_text, extracted_output_text.as_str());
            self.append_sse_event(
                out,
                &self.build_chunk(vec![json!({ "text": extracted_output_text })], None, None),
            );
        }
        if let Some(output_items) = response.get("output").and_then(Value::as_array) {
            for (index, item) in output_items.iter().enumerate() {
                let Some(item_obj) = item.as_object() else {
                    continue;
                };
                let item_type = item_obj
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if !matches!(item_type, "function_call" | "custom_tool_call") {
                    continue;
                }
                let output_index = index as i64;
                let entry = self
                    .state
                    .pending_tool_calls
                    .entry(output_index)
                    .or_default();
                if entry.name.is_none() {
                    entry.name = item_obj
                        .get("name")
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .filter(|current| !current.is_empty())
                        .map(str::to_string);
                }
                if entry.call_id.is_none() {
                    entry.call_id = item_obj
                        .get("call_id")
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .filter(|current| !current.is_empty())
                        .map(str::to_string);
                }
                if !has_meaningful_tool_arguments(&entry.arguments) {
                    if let Some(arguments) = extract_function_call_arguments(item_obj) {
                        entry.arguments = arguments;
                    }
                }
                self.emit_pending_tool_call(out, output_index);
            }
        }
        let pending_indices = self
            .state
            .pending_tool_calls
            .keys()
            .copied()
            .collect::<Vec<_>>();
        for output_index in pending_indices {
            self.emit_pending_tool_call(out, output_index);
        }
        let usage_metadata = response
            .get("usage")
            .and_then(Value::as_object)
            .and_then(build_gemini_usage_metadata);
        self.append_sse_event(
            out,
            &self.build_chunk(Vec::new(), Some("STOP"), usage_metadata),
        );
    }

    fn capture_response_meta(&mut self, value: &Value) {
        if let Some(response_id) = value
            .get("response_id")
            .or_else(|| value.get("id"))
            .and_then(Value::as_str)
        {
            self.state.response_id = Some(response_id.to_string());
        }
        if let Some(model) = value.get("model").and_then(Value::as_str) {
            self.state.model = Some(model.to_string());
        }
        if let Some(response) = value.get("response").and_then(Value::as_object) {
            if let Some(response_id) = response.get("id").and_then(Value::as_str) {
                self.state.response_id = Some(response_id.to_string());
            }
            if let Some(model) = response.get("model").and_then(Value::as_str) {
                self.state.model = Some(model.to_string());
            }
            if let Some(usage) = response.get("usage").and_then(Value::as_object) {
                self.state.input_tokens = usage
                    .get("input_tokens")
                    .and_then(Value::as_i64)
                    .or_else(|| usage.get("prompt_tokens").and_then(Value::as_i64))
                    .unwrap_or(self.state.input_tokens);
                self.state.cached_input_tokens = usage
                    .get("input_tokens_details")
                    .and_then(Value::as_object)
                    .and_then(|details| details.get("cached_tokens"))
                    .and_then(Value::as_i64)
                    .or_else(|| usage.get("cached_input_tokens").and_then(Value::as_i64))
                    .unwrap_or(self.state.cached_input_tokens);
                self.state.output_tokens = usage
                    .get("output_tokens")
                    .and_then(Value::as_i64)
                    .or_else(|| usage.get("completion_tokens").and_then(Value::as_i64))
                    .unwrap_or(self.state.output_tokens);
                self.state.total_tokens = usage
                    .get("total_tokens")
                    .and_then(Value::as_i64)
                    .or(self.state.total_tokens);
                self.state.reasoning_output_tokens = usage
                    .get("output_tokens_details")
                    .and_then(Value::as_object)
                    .and_then(|details| details.get("reasoning_tokens"))
                    .and_then(Value::as_i64)
                    .or_else(|| usage.get("reasoning_output_tokens").and_then(Value::as_i64))
                    .unwrap_or(self.state.reasoning_output_tokens);
            }
        }
    }

    fn build_chunk(
        &self,
        parts: Vec<Value>,
        finish_reason: Option<&str>,
        usage_metadata: Option<Value>,
    ) -> Value {
        let mut candidate = serde_json::Map::new();
        candidate.insert("index".to_string(), Value::from(0));
        candidate.insert(
            "content".to_string(),
            json!({ "role": "model", "parts": parts.clone() }),
        );
        if let Some(reason) = finish_reason {
            candidate.insert(
                "finishReason".to_string(),
                Value::String(reason.to_string()),
            );
        }
        let mut payload = serde_json::Map::new();
        payload.insert(
            "responseId".to_string(),
            Value::String(
                self.state
                    .response_id
                    .clone()
                    .unwrap_or_else(|| "resp_codexmanager".to_string()),
            ),
        );
        payload.insert(
            "modelVersion".to_string(),
            Value::String(
                self.state
                    .model
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string()),
            ),
        );
        payload.insert(
            "candidates".to_string(),
            Value::Array(vec![Value::Object(candidate)]),
        );
        if let Some(function_calls) = build_function_calls(&parts) {
            payload.insert("functionCalls".to_string(), function_calls);
        }
        if let Some(usage_metadata) = usage_metadata {
            payload.insert("usageMetadata".to_string(), usage_metadata);
        }
        Value::Object(payload)
    }

    fn append_sse_event(&self, out: &mut String, payload: &Value) {
        append_gemini_event(out, payload, self.output_mode, self.wrap_response_envelope);
    }

    fn finish_stream(&mut self) -> Vec<u8> {
        if self.state.finished {
            return Vec::new();
        }
        self.state.finished = true;
        if let Ok(mut collector) = self.usage_collector.lock() {
            collector.usage.input_tokens = Some(self.state.input_tokens.max(0));
            collector.usage.cached_input_tokens = Some(self.state.cached_input_tokens.max(0));
            collector.usage.output_tokens = Some(self.state.output_tokens.max(0));
            collector.usage.total_tokens = self.state.total_tokens.map(|value| value.max(0));
            collector.usage.reasoning_output_tokens =
                Some(self.state.reasoning_output_tokens.max(0));
            if !self.state.output_text.trim().is_empty() {
                collector.usage.output_text = Some(self.state.output_text.clone());
            }
            if !collector.saw_terminal {
                if let Some(message) = collector.terminal_error.clone() {
                    return build_terminal_error_event(
                        self.output_mode,
                        build_gemini_error_body(message.as_str()),
                    );
                }
            }
        }
        Vec::new()
    }

    fn mark_stream_incomplete_if_needed(&self) {
        if self.state.completed_seen {
            return;
        }
        if let Ok(mut collector) = self.usage_collector.lock() {
            let hint = collector.upstream_error_hint.clone();
            collector
                .terminal_error
                .get_or_insert_with(|| upstream_hint_or_stream_incomplete_message(hint.as_deref()));
        }
    }

    fn mark_stream_read_error(&self, message: String) {
        if self.state.completed_seen {
            return;
        }
        if let Ok(mut collector) = self.usage_collector.lock() {
            collector.terminal_error.get_or_insert(message);
        }
    }

    fn record_last_event_type(&self, event_type: &str) {
        if let Ok(mut collector) = self.usage_collector.lock() {
            collector.last_event_type = Some(event_type.to_string());
        }
    }
}

impl Read for GeminiSseReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        loop {
            let read = self.out_cursor.read(buf)?;
            if read > 0 {
                return Ok(read);
            }
            if self.state.finished {
                return Ok(0);
            }
            let next = self.next_chunk()?;
            self.out_cursor = Cursor::new(next);
        }
    }
}

fn append_gemini_sse_event(buffer: &mut String, payload: &Value) {
    let data = serde_json::to_string(payload).unwrap_or_else(|_| "{}".to_string());
    buffer.push_str("data: ");
    buffer.push_str(&data);
    buffer.push_str("\n\n");
}

fn append_gemini_cli_sse_event(buffer: &mut String, payload: &Value) {
    append_gemini_sse_event(buffer, &json!({ "response": payload }));
}

fn append_gemini_event(
    buffer: &mut String,
    payload: &Value,
    output_mode: GeminiStreamOutputMode,
    wrap_response_envelope: bool,
) {
    match (output_mode, wrap_response_envelope) {
        (GeminiStreamOutputMode::Sse, true) => append_gemini_cli_sse_event(buffer, payload),
        (GeminiStreamOutputMode::Sse, false) => append_gemini_sse_event(buffer, payload),
        (GeminiStreamOutputMode::Raw, true) => {
            let wrapped = json!({ "response": payload });
            buffer.push_str(&serde_json::to_string(&wrapped).unwrap_or_else(|_| "{}".to_string()));
        }
        (GeminiStreamOutputMode::Raw, false) => {
            buffer.push_str(&serde_json::to_string(payload).unwrap_or_else(|_| "{}".to_string()));
        }
    }
}

fn build_terminal_error_event(output_mode: GeminiStreamOutputMode, body: Vec<u8>) -> Vec<u8> {
    match output_mode {
        GeminiStreamOutputMode::Sse => {
            let mut out = Vec::from("event: error\ndata: ".as_bytes());
            out.extend_from_slice(&body);
            out.extend_from_slice(b"\n\n");
            out
        }
        GeminiStreamOutputMode::Raw => body,
    }
}

fn build_function_calls(parts: &[Value]) -> Option<Value> {
    let mut function_calls = Vec::new();
    for part in parts {
        let Some(function_call) = part.get("functionCall").and_then(Value::as_object) else {
            continue;
        };
        let Some(name) = function_call
            .get("name")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|current| !current.is_empty())
        else {
            continue;
        };
        let mut item = serde_json::Map::new();
        item.insert("name".to_string(), Value::String(name.to_string()));
        item.insert(
            "args".to_string(),
            function_call
                .get("args")
                .cloned()
                .unwrap_or_else(|| json!({})),
        );
        if let Some(call_id) = function_call
            .get("id")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|current| !current.is_empty())
        {
            item.insert("id".to_string(), Value::String(call_id.to_string()));
        }
        function_calls.push(Value::Object(item));
    }
    if function_calls.is_empty() {
        None
    } else {
        Some(Value::Array(function_calls))
    }
}

fn build_gemini_usage_metadata(usage: &Map<String, Value>) -> Option<Value> {
    let prompt = extract_usage_i64(usage, &["input_tokens", "prompt_tokens"])?;
    let candidates = extract_usage_i64(usage, &["output_tokens", "completion_tokens"]).unwrap_or(0);
    let total = extract_usage_i64(usage, &["total_tokens"]).unwrap_or(prompt + candidates);
    Some(json!({
        "promptTokenCount": prompt,
        "candidatesTokenCount": candidates,
        "totalTokenCount": total,
    }))
}

fn extract_usage_i64(usage: &Map<String, Value>, keys: &[&str]) -> Option<i64> {
    for key in keys {
        if let Some(value) = usage.get(*key).and_then(Value::as_i64) {
            return Some(value);
        }
    }
    None
}

fn extract_function_call_arguments(item_obj: &Map<String, Value>) -> Option<String> {
    const ARGUMENT_KEYS: [&str; 5] = [
        "arguments",
        "input",
        "arguments_json",
        "parsed_arguments",
        "args",
    ];
    for key in ARGUMENT_KEYS {
        let Some(value) = item_obj.get(key) else {
            continue;
        };
        if value.is_null() {
            continue;
        }
        if let Some(text) = value.as_str() {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
            continue;
        }
        if let Ok(serialized) = serde_json::to_string(value) {
            return Some(serialized);
        }
    }
    None
}

fn parse_json_object_or_empty(raw: &str) -> Value {
    serde_json::from_str::<Value>(raw)
        .ok()
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({}))
}

fn has_meaningful_tool_arguments(raw: &str) -> bool {
    parse_json_object_or_empty(raw)
        .as_object()
        .is_some_and(|obj| !obj.is_empty())
}

fn merge_arguments(existing: &mut String, fragment: &str) {
    let trimmed = fragment.trim();
    if trimmed.is_empty() {
        return;
    }
    if existing.is_empty() {
        existing.push_str(trimmed);
        return;
    }
    if existing == trimmed || existing.ends_with(trimmed) {
        return;
    }
    if trimmed.starts_with(existing.as_str()) {
        *existing = trimmed.to_string();
        return;
    }
    existing.push_str(trimmed);
}

fn restore_tool_name(name: &str, tool_name_restore_map: Option<&ToolNameRestoreMap>) -> String {
    tool_name_restore_map
        .and_then(|map| map.get(name))
        .cloned()
        .unwrap_or_else(|| name.to_string())
}

fn is_response_completed_event_type(event_type: &str) -> bool {
    matches!(event_type, "response.completed" | "response.done")
}

fn extract_completed_response_message_text(response: &Value) -> Option<String> {
    let mut segments = Vec::new();
    if let Some(output_items) = response.get("output").and_then(Value::as_array) {
        for item in output_items {
            let Some(item_obj) = item.as_object() else {
                continue;
            };
            if item_obj.get("type").and_then(Value::as_str) != Some("message") {
                continue;
            }
            let Some(content_items) = item_obj.get("content").and_then(Value::as_array) else {
                continue;
            };
            for content_item in content_items {
                let Some(content_obj) = content_item.as_object() else {
                    continue;
                };
                let content_type = content_obj
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if !matches!(content_type, "output_text" | "text") {
                    continue;
                }
                let Some(text) = content_obj
                    .get("text")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|text| !text.is_empty())
                else {
                    continue;
                };
                segments.push(text.to_string());
            }
        }
    }
    if !segments.is_empty() {
        return Some(segments.join("\n"));
    }
    response
        .get("output_text")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
}

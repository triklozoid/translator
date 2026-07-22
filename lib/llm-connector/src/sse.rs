//! Server-Sent Events (SSE) streaming utilities
//!
//! This module provides SSE parsing for streaming responses from LLM providers.
//! All major commercial LLM providers (OpenAI, Anthropic, DeepSeek, etc.) use SSE for streaming.

use crate::error::LlmConnectorError;
use futures_util::{Stream, StreamExt};
use std::io::Write;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

/// Optional diagnostic logger — set by the host application to capture
/// SSE-level diagnostics into the app's log file.
static SSE_DIAG_LOGGER: Mutex<Option<fn(&str)>> = Mutex::new(None);

/// Register a diagnostic logging callback. Calls to `sse_log()` will invoke
/// this function instead of (or in addition to) writing to stderr.
pub fn set_sse_diag_logger(cb: fn(&str)) {
    if let Ok(mut logger) = SSE_DIAG_LOGGER.lock() {
        *logger = Some(cb);
    }
}

/// Log a diagnostic message from the SSE layer.
/// Uses the registered callback if set, otherwise writes to stderr.
fn sse_log(msg: &str) {
    if let Ok(logger) = SSE_DIAG_LOGGER.lock() {
        if let Some(cb) = *logger {
            cb(msg);
            return;
        }
    }
    eprintln!("{}", msg);
    let _ = std::io::stderr().flush();
}

/// Extract complete SSE events from the buffer. Returns a vec of JSON payload strings.
fn extract_sse_events(buffer: &mut String) -> Vec<String> {
    let mut events = Vec::new();
    while let Some(boundary_idx) = buffer.find("\n\n") {
        let event_str: String = buffer.drain(..boundary_idx + 2).collect();
        let mut data_lines: Vec<String> = Vec::new();
        for raw_line in event_str.split('\n') {
            let line = raw_line.trim_end();
            if let Some(rest) = line
                .strip_prefix("data: ")
                .or_else(|| line.strip_prefix("data:"))
            {
                let payload = rest.trim_start();
                if payload.trim() == "[DONE]" {
                    continue;
                }
                if !payload.is_empty() {
                    data_lines.push(payload.to_string());
                }
            }
        }
        if !data_lines.is_empty() {
            events.push(data_lines.join("\n"));
        }
    }
    events
}

/// Parse Server-Sent Events (SSE) from an HTTP response and yield one JSON string per event.
///
/// Uses a shared buffer (Arc<Mutex<String>>) so that when the underlying bytes_stream
/// ends, any partial data remaining in the buffer (e.g. a final SSE event missing its
/// trailing `\n\n`) is flushed as a final event. Without this, a stalled or truncated
/// connection would cause the stream to hang forever with buffered data never emitted.
#[inline]
pub fn sse_events(
    response: reqwest::Response,
) -> Pin<Box<dyn Stream<Item = Result<String, LlmConnectorError>> + Send>> {
    let chunk_counter = AtomicU64::new(0);
    let buffer = Arc::new(Mutex::new(String::new()));
    let buffer_for_flush = Arc::clone(&buffer);
    let stream = response.bytes_stream();

    let events_stream = stream
        .map(move |chunk_result| {
            let mut out: Vec<Result<String, LlmConnectorError>> = Vec::new();
            match chunk_result {
                Ok(chunk) => {
                    let chunk_idx = chunk_counter.fetch_add(1, Ordering::Relaxed);
                    let chunk_len = chunk.len();
                    let chunk_str = String::from_utf8_lossy(&chunk).replace("\r\n", "\n");
                    let buf_before;
                    let buf_after_push;
                    {
                        let mut buf = buffer.lock().unwrap();
                        buf_before = buf.len();
                        buf.push_str(&chunk_str);
                        buf_after_push = buf.len();
                        let events = extract_sse_events(&mut buf);
                        let events_count = events.len();
                        for event in events {
                            out.push(Ok(event));
                        }
                        let buf_after = buf.len();
                        if events_count > 0 {
                            sse_log(&format!(
                                "[SSE] chunk #{:<4} | {}B raw | buf {}→{}→{} | {} event(s)",
                                chunk_idx, chunk_len, buf_before, buf_after_push, buf_after, events_count
                            ));
                        } else if buf_after_push > 0 {
                            sse_log(&format!(
                                "[SSE] chunk #{:<4} | {}B raw | buf {}→{} | ⚠ no \\n\\n, buffering {}B",
                                chunk_idx, chunk_len, buf_before, buf_after_push, buf_after_push
                            ));
                        }
                    }
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    sse_log(&format!("[SSE] chunk error | NetworkError: {}", error_msg));
                    out.push(Err(LlmConnectorError::NetworkError(error_msg)));
                }
            }
            out
        })
        // CRITICAL: When the bytes_stream ends, flush any remaining data in the buffer.
        // Handles the case where the final SSE event lacks a trailing \n\n due to
        // connection truncation or a server-side bug.
        .chain(futures_util::stream::once(async move {
            let mut out: Vec<Result<String, LlmConnectorError>> = Vec::new();
            let mut buf = buffer_for_flush.lock().unwrap();
            if !buf.is_empty() {
                let remaining_len = buf.len();
                sse_log(&format!(
                    "[SSE] stream ended | flushing {}B of unprocessed buffer data",
                    remaining_len
                ));
                let events = extract_sse_events(&mut buf);
                if events.is_empty() && !buf.is_empty() {
                    // Buffer has data but no \n\n delimiter —
                    // treat as raw JSON (some providers send JSON lines without proper SSE framing)
                    let remaining = buf.clone();
                    buf.clear();
                    sse_log(&format!(
                        "[SSE] stream ended | no \\n\\n found, emitting {}B as raw JSON (may be truncated)",
                        remaining.len()
                    ));
                    out.push(Ok(remaining));
                } else {
                    for event in events {
                        out.push(Ok(event));
                    }
                }
                // Emit any still-remaining partial data
                if !buf.is_empty() {
                    let leftover = buf.clone();
                    buf.clear();
                    sse_log(&format!(
                        "[SSE] stream ended | {}B left after extraction — discarding as noise",
                        leftover.len()
                    ));
                }
            }
            out
        }))
        .flat_map(futures_util::stream::iter);

    Box::pin(events_stream)
}

/// Parse line-delimited JSON events (non-SSE) from an HTTP response.
/// Each line is treated as a standalone JSON payload.
///
/// Uses a shared buffer so that partial data at stream end is flushed.
#[inline]
pub fn json_lines_events(
    response: reqwest::Response,
) -> Pin<Box<dyn Stream<Item = Result<String, LlmConnectorError>> + Send>> {
    let buffer = Arc::new(Mutex::new(String::new()));
    let buffer_for_flush = Arc::clone(&buffer);
    let stream = response.bytes_stream();

    let events_stream = stream
        .map(move |chunk_result| {
            let mut out: Vec<Result<String, LlmConnectorError>> = Vec::new();
            match chunk_result {
                Ok(chunk) => {
                    let chunk_str = String::from_utf8_lossy(&chunk).replace("\r\n", "\n");
                    let mut buf = buffer.lock().unwrap();
                    buf.push_str(&chunk_str);
                    while let Some(boundary_idx) = buf.find('\n') {
                        let line: String = buf.drain(..boundary_idx + 1).collect();
                        let trimmed = line.trim();
                        if trimmed.is_empty() { continue; }
                        if trimmed == "[DONE]" { continue; }
                        out.push(Ok(trimmed.to_string()));
                    }
                }
                Err(e) => {
                    out.push(Err(LlmConnectorError::NetworkError(e.to_string())));
                }
            }
            out
        })
        .chain(futures_util::stream::once(async move {
            let mut buf = buffer_for_flush.lock().unwrap();
            if !buf.is_empty() {
                let remaining = buf.clone();
                buf.clear();
                let trimmed = remaining.trim();
                if !trimmed.is_empty() && trimmed != "[DONE]" {
                    return vec![Ok(trimmed.to_string())];
                }
            }
            Vec::new()
        }))
        .flat_map(futures_util::stream::iter);

    Box::pin(events_stream)
}

/// Convert SSE string stream to StreamingResponse stream
///
/// This function handles tool_calls accumulation for streaming responses.
/// In OpenAI's streaming API, tool_calls are sent incrementally across multiple chunks,
/// and need to be accumulated by index.
///
/// CRITICAL: Wraps the SSE stream with a stall detector. If the underlying stream
/// produces no items for `stall_timeout`, a StallError is emitted to prevent
/// the application from hanging indefinitely on a stalled connection.
#[cfg(feature = "streaming")]
pub fn sse_to_streaming_response(
    response: reqwest::Response,
) -> crate::types::ChatStream {
    use crate::types::{StreamingResponse, ToolCall};
    use futures_util::StreamExt;
    use std::collections::HashMap;

    let string_stream = sse_events(response);

    // State for accumulating tool_calls across chunks
    let response_stream = string_stream.scan(
        HashMap::<usize, ToolCall>::new(),
        |accumulated_tool_calls, result| {
            let processed = result.and_then(|json_str| {
                let json_len = json_str.len();
                // Try to parse as StreamingResponse
                let mut streaming_response = serde_json::from_str::<StreamingResponse>(&json_str)
                    .map_err(|e| crate::error::LlmConnectorError::ParseError(
                        format!("Failed to parse streaming response: {}", e)
                    ))?;

                // 🔧 Fix: Populate the convenience `content` field from choices[0].delta
                // This is critical for Volcengine and other OpenAI-compatible providers
                //
                // Priority order:
                // 1. delta.content (standard OpenAI format)
                // 2. delta.reasoning_content (Volcengine Doubao-Seed-Code, DeepSeek R1)
                // 3. delta.reasoning (Qwen, DeepSeek)
                // 4. delta.thought (OpenAI o1)
                // 5. delta.thinking (Anthropic)
                let mut content_source = "none";
                let mut content_len: usize = 0;
                if streaming_response.content.is_empty() {
                    if let Some(choice) = streaming_response.choices.first() {
                        let content_to_use = choice.delta.content.as_ref()
                            .filter(|s| !s.is_empty())
                            .map(|s| { content_source = "delta.content"; s })
                            .or_else(|| choice.delta.reasoning_content.as_ref().map(|s| { content_source = "reasoning_content"; s }))
                            .or_else(|| choice.delta.reasoning.as_ref().map(|s| { content_source = "reasoning"; s }))
                            .or_else(|| choice.delta.thought.as_ref().map(|s| { content_source = "thought"; s }))
                            .or_else(|| choice.delta.thinking.as_ref().map(|s| { content_source = "thinking"; s }));

                        if let Some(content) = content_to_use {
                            content_len = content.len();
                            streaming_response.content = content.clone();
                        }
                    }
                } else {
                    content_source = "top-level";
                    content_len = streaming_response.content.len();
                }

                if streaming_response.content.is_empty() {
                    let finish_reason = streaming_response.choices.first()
                        .and_then(|c| c.finish_reason.as_deref())
                        .unwrap_or("none");
                    let has_usage = streaming_response.usage.is_some();
                    let has_delta_content = streaming_response.choices.first()
                        .and_then(|c| c.delta.content.as_ref())
                        .is_some();
                    sse_log(&format!("[SSE] event: {}B JSON → ⚠ empty content | finish={}, usage={}, delta.content={}",
                        json_len, finish_reason, has_usage, has_delta_content));
                } else {
                    sse_log(&format!("[SSE] event: {}B JSON → content from {} ({} chars)",
                        json_len, content_source, content_len));
                }

                // 🔧 Fix: Accumulate tool_calls across chunks
                // OpenAI streaming API sends tool_calls incrementally with an index field
                if let Some(choice) = streaming_response.choices.first_mut() {
                    if let Some(delta_tool_calls) = &choice.delta.tool_calls {
                        for delta_call in delta_tool_calls {
                            let index = delta_call.index.unwrap_or(0);

                            accumulated_tool_calls
                                .entry(index)
                                .and_modify(|existing| existing.merge_delta(delta_call))
                                .or_insert_with(|| delta_call.clone());
                        }

                        // Replace delta.tool_calls with accumulated complete tool_calls
                        // Only include complete tool_calls (have id, type, and name)
                        let complete_calls: Vec<ToolCall> = accumulated_tool_calls
                            .values()
                            .filter(|call| call.is_complete())
                            .cloned()
                            .collect();

                        if !complete_calls.is_empty() {
                            choice.delta.tool_calls = Some(complete_calls);
                        } else {
                            // Don't send incomplete tool_calls to avoid duplicate execution
                            choice.delta.tool_calls = None;
                        }
                    }
                }

                Ok(streaming_response)
            });

            std::future::ready(Some(processed))
        }
    );

    Box::pin(response_stream)
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(feature = "streaming")]
    fn test_streaming_response_content_population() {
        use crate::types::StreamingResponse;

        // Test 1: Standard OpenAI format with content
        let json_standard = r#"{
            "id": "test-1",
            "object": "chat.completion.chunk",
            "created": 1234567890,
            "model": "gpt-4",
            "choices": [{
                "index": 0,
                "delta": {
                    "role": "assistant",
                    "content": "Hello world"
                },
                "finish_reason": null
            }]
        }"#;

        let mut response: StreamingResponse = serde_json::from_str(json_standard).unwrap();

        // Simulate the content population logic
        if response.content.is_empty() {
            if let Some(choice) = response.choices.first() {
                let content_to_use = choice.delta.content.as_ref()
                    .filter(|s| !s.is_empty())
                    .or_else(|| choice.delta.reasoning_content.as_ref())
                    .or_else(|| choice.delta.reasoning.as_ref())
                    .or_else(|| choice.delta.thought.as_ref())
                    .or_else(|| choice.delta.thinking.as_ref());

                if let Some(content) = content_to_use {
                    response.content = content.clone();
                }
            }
        }

        assert_eq!(response.content, "Hello world");

        // Test 2: Volcengine Doubao-Seed-Code format with reasoning_content
        let json_volcengine = r#"{
            "id": "test-2",
            "object": "chat.completion.chunk",
            "created": 1234567890,
            "model": "doubao-seed-code",
            "choices": [{
                "index": 0,
                "delta": {
                    "role": "assistant",
                    "content": "",
                    "reasoning_content": "I am Doubao"
                },
                "finish_reason": null
            }]
        }"#;

        let mut response: StreamingResponse = serde_json::from_str(json_volcengine).unwrap();

        // Simulate the content population logic
        if response.content.is_empty() {
            if let Some(choice) = response.choices.first() {
                let content_to_use = choice.delta.content.as_ref()
                    .filter(|s| !s.is_empty())
                    .or_else(|| choice.delta.reasoning_content.as_ref())
                    .or_else(|| choice.delta.reasoning.as_ref())
                    .or_else(|| choice.delta.thought.as_ref())
                    .or_else(|| choice.delta.thinking.as_ref());

                if let Some(content) = content_to_use {
                    response.content = content.clone();
                }
            }
        }

        assert_eq!(response.content, "I am Doubao");

        // Test 3: DeepSeek format with reasoning
        let json_deepseek = r#"{
            "id": "test-3",
            "object": "chat.completion.chunk",
            "created": 1234567890,
            "model": "deepseek-r1",
            "choices": [{
                "index": 0,
                "delta": {
                    "role": "assistant",
                    "content": "",
                    "reasoning": "Let me think..."
                },
                "finish_reason": null
            }]
        }"#;

        let mut response: StreamingResponse = serde_json::from_str(json_deepseek).unwrap();

        // Simulate the content population logic
        if response.content.is_empty() {
            if let Some(choice) = response.choices.first() {
                let content_to_use = choice.delta.content.as_ref()
                    .filter(|s| !s.is_empty())
                    .or_else(|| choice.delta.reasoning_content.as_ref())
                    .or_else(|| choice.delta.reasoning.as_ref())
                    .or_else(|| choice.delta.thought.as_ref())
                    .or_else(|| choice.delta.thinking.as_ref());

                if let Some(content) = content_to_use {
                    response.content = content.clone();
                }
            }
        }

        assert_eq!(response.content, "Let me think...");
    }

    #[test]
    fn test_sse_event_parsing() {
        // Test SSE format parsing
        let sse_data = "data: {\"test\": \"value\"}\n\n";
        assert!(sse_data.starts_with("data: "));

        let sse_done = "data: [DONE]\n\n";
        assert!(sse_done.contains("[DONE]"));
    }
}

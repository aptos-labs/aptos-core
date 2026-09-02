// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::evaluation::{EvaluationConfig, SOURCE_COMMIT_ENV_VAR};
use anyhow::{Context, Result};
use serde_json::{Map, Value};
use std::{
    fs::{File, OpenOptions},
    io::{BufWriter, Write},
    path::Path,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{Instant, SystemTime, UNIX_EPOCH},
};

#[derive(Clone)]
pub(crate) struct Telemetry {
    inner: Option<Arc<TelemetryInner>>,
}

struct TelemetryInner {
    writer: Mutex<BufWriter<File>>,
    process_start: Instant,
    sequence: AtomicU64,
    session_id: String,
    config: EvaluationConfig,
    source_commit: String,
}

impl Telemetry {
    pub(crate) fn new(path: Option<&Path>, config: EvaluationConfig) -> Result<Self> {
        let Some(path) = path else {
            return Ok(Self { inner: None });
        };
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("failed to create telemetry directory {}", parent.display())
            })?;
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .with_context(|| format!("failed to open telemetry file {}", path.display()))?;
        let session_id = format!(
            "{}-{}",
            std::process::id(),
            unix_time_ns().unwrap_or_default()
        );
        Ok(Self {
            inner: Some(Arc::new(TelemetryInner {
                writer: Mutex::new(BufWriter::new(file)),
                process_start: Instant::now(),
                sequence: AtomicU64::new(0),
                session_id,
                config,
                source_commit: std::env::var(SOURCE_COMMIT_ENV_VAR)
                    .unwrap_or_else(|_| "unrecorded".to_string()),
            })),
        })
    }

    #[cfg(test)]
    pub(crate) fn disabled() -> Self {
        Self { inner: None }
    }

    /// Whether anything is recording. Callers that must compute a payload to
    /// report it -- serializing a whole response, canonicalizing a path --
    /// check this first so an unconfigured server pays nothing.
    pub(crate) fn is_enabled(&self) -> bool {
        self.inner.is_some()
    }

    pub(crate) fn emit(&self, event: &str, fields: Value) {
        let fields = bounded(fields);
        let Some(inner) = &self.inner else {
            return;
        };
        let mut record = Map::new();
        record.insert("schema_version".to_string(), Value::from(1));
        record.insert("event".to_string(), Value::from(event));
        record.insert(
            "session_id".to_string(),
            Value::from(inner.session_id.clone()),
        );
        record.insert(
            "sequence".to_string(),
            Value::from(inner.sequence.fetch_add(1, Ordering::Relaxed)),
        );
        record.insert(
            "utc_unix_ms".to_string(),
            Value::from(unix_time_ms().unwrap_or_default()),
        );
        record.insert(
            "monotonic_us".to_string(),
            Value::from(inner.process_start.elapsed().as_micros() as u64),
        );
        record.insert(
            "flow_version".to_string(),
            Value::from(env!("CARGO_PKG_VERSION")),
        );
        record.insert(
            "flow_source_commit".to_string(),
            Value::from(inner.source_commit.clone()),
        );
        record.insert(
            "inference_tactic".to_string(),
            Value::from(inner.config.inference_tactic.as_str()),
        );
        record.insert(
            "evaluation_mode".to_string(),
            Value::from(inner.config.evaluation_mode),
        );
        if let Value::Object(fields) = fields {
            record.extend(fields);
        }

        let line = match serde_json::to_string(&record) {
            Ok(line) => line,
            Err(error) => {
                log::error!("failed to serialize telemetry event `{}`: {}", event, error);
                return;
            },
        };
        let mut writer = match inner.writer.lock() {
            Ok(writer) => writer,
            Err(_) => {
                log::error!("telemetry writer lock poisoned");
                return;
            },
        };
        if let Err(error) = writeln!(writer, "{}", line).and_then(|_| writer.flush()) {
            log::error!("failed to write telemetry event `{}`: {}", event, error);
        }
    }
}

fn unix_time_ms() -> Option<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_millis() as u64)
}

fn unix_time_ns() -> Option<u128> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_nanos())
}

/// Longest string recorded in a telemetry field, in bytes.
///
/// Fields carry caller-supplied values -- a filter, a file path -- and a
/// record is appended per event. A value longer than this tells a reader
/// nothing it needs and is not worth the disk.
const MAX_FIELD_BYTES: usize = 512;

/// Most elements recorded from an array or object field.
const MAX_FIELD_ITEMS: usize = 32;

/// A telemetry value with every string bounded in size.
///
/// Applied here rather than at each call site, so a new field cannot
/// reintroduce an unbounded one.
fn bounded(value: Value) -> Value {
    match value {
        Value::String(text) if text.len() > MAX_FIELD_BYTES => {
            let mut end = MAX_FIELD_BYTES;
            while end > 0 && !text.is_char_boundary(end) {
                end -= 1;
            }
            let dropped = text.len() - end;
            let mut text = text;
            text.truncate(end);
            Value::String(format!("{text}... [{dropped} bytes truncated]"))
        },
        // Length alone is not a bound: a caller can send many small elements.
        Value::Array(items) => {
            let dropped = items.len().saturating_sub(MAX_FIELD_ITEMS);
            let mut items: Vec<Value> = items
                .into_iter()
                .take(MAX_FIELD_ITEMS)
                .map(bounded)
                .collect();
            if dropped > 0 {
                items.push(Value::String(format!("... [{dropped} items truncated]")));
            }
            Value::Array(items)
        },
        Value::Object(fields) => {
            let dropped = fields.len().saturating_sub(MAX_FIELD_ITEMS);
            let mut kept: serde_json::Map<String, Value> = fields
                .into_iter()
                .take(MAX_FIELD_ITEMS)
                // The key is the caller's as much as the value is.
                .map(|(k, v)| match bounded(Value::String(k)) {
                    Value::String(k) => (k, bounded(v)),
                    _ => unreachable!("a string bounds to a string"),
                })
                .collect();
            if dropped > 0 {
                kept.insert(
                    "...".to_string(),
                    Value::String(format!("[{dropped} entries truncated]")),
                );
            }
            Value::Object(kept)
        },
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evaluation::{EvaluationConfig, InferenceTactic};

    #[test]
    fn writes_ordered_jsonl_events() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("events.jsonl");
        let config = EvaluationConfig {
            inference_tactic: InferenceTactic::AgentOnly,
            evaluation_mode: true,
            feedback_level: crate::evaluation::FeedbackLevel::Acceptance,
        };
        let telemetry = Telemetry::new(Some(&path), config).unwrap();
        telemetry.emit("session_start", serde_json::json!({"restart": false}));
        telemetry.emit("session_end", serde_json::json!({"outcome": "success"}));

        let records: Vec<Value> = std::fs::read_to_string(path)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0]["sequence"], 0);
        assert_eq!(records[1]["sequence"], 1);
        assert_eq!(records[0]["inference_tactic"], "agent_only");
        assert_eq!(records[0]["evaluation_mode"], true);
        assert!(
            records[1]["monotonic_us"].as_u64().unwrap()
                >= records[0]["monotonic_us"].as_u64().unwrap()
        );
    }
}

// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! W3C Trace Context propagation for gRPC services.
//!
//! Provides extraction and injection of `traceparent` / `tracestate` headers
//! through tonic gRPC metadata, following the W3C Trace Context specification.

use opentelemetry::trace::{SpanContext, SpanId, TraceContextExt, TraceFlags, TraceId, TraceState};
use tonic::metadata::MetadataMap;
use tracing_opentelemetry::OpenTelemetrySpanExt;
use uuid::Uuid;

pub const TRACEPARENT_HEADER: &str = "traceparent";
pub const TRACESTATE_HEADER: &str = "tracestate";
pub const TRACEPARENT_VERSION: &str = "00";
pub const TRACE_FLAGS_SAMPLED: &str = "01";

const ALL_ZEROS_TRACE_ID: &str = "00000000000000000000000000000000";
const ALL_ZEROS_SPAN_ID: &str = "0000000000000000";

/// Parsed W3C traceparent components.
#[derive(Clone, Debug)]
pub struct TraceContext {
    pub trace_id: String,
    pub parent_span_id: String,
    pub trace_flags: String,
    pub tracestate: Option<String>,
}

impl TraceContext {
    /// Formats the trace context as a W3C traceparent header value.
    pub fn to_traceparent(&self) -> String {
        format!(
            "{}-{}-{}-{}",
            TRACEPARENT_VERSION, self.trace_id, self.parent_span_id, self.trace_flags
        )
    }

    /// Creates a new root trace context with generated IDs.
    pub fn new_root() -> Self {
        Self {
            trace_id: generate_trace_id(),
            parent_span_id: generate_span_id(),
            trace_flags: TRACE_FLAGS_SAMPLED.to_string(),
            tracestate: None,
        }
    }

    /// Creates a child context that inherits the trace_id but gets a new span_id.
    pub fn new_child(&self) -> Self {
        Self {
            trace_id: self.trace_id.clone(),
            parent_span_id: generate_span_id(),
            trace_flags: self.trace_flags.clone(),
            tracestate: self.tracestate.clone(),
        }
    }
}

/// Parses a W3C traceparent header string.
///
/// Format: `{version}-{trace_id}-{parent_id}-{trace_flags}`
/// Example: `00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01`
pub fn parse_traceparent(traceparent: &str) -> Option<TraceContext> {
    let parts: Vec<&str> = traceparent.split('-').collect();
    if parts.len() != 4 {
        return None;
    }
    let (version, trace_id, parent_span_id, trace_flags) = (parts[0], parts[1], parts[2], parts[3]);

    if version.len() != 2
        || trace_id.len() != 32
        || parent_span_id.len() != 16
        || trace_flags.len() != 2
    {
        return None;
    }

    if !trace_id.chars().all(|c| c.is_ascii_hexdigit())
        || !parent_span_id.chars().all(|c| c.is_ascii_hexdigit())
        || !trace_flags.chars().all(|c| c.is_ascii_hexdigit())
    {
        return None;
    }

    // W3C spec: all-zeros trace-id or parent-id are invalid.
    if trace_id == ALL_ZEROS_TRACE_ID || parent_span_id == ALL_ZEROS_SPAN_ID {
        return None;
    }

    Some(TraceContext {
        trace_id: trace_id.to_string(),
        parent_span_id: parent_span_id.to_string(),
        trace_flags: trace_flags.to_string(),
        tracestate: None,
    })
}

/// Extracts a W3C Trace Context from gRPC metadata.
pub fn extract_trace_context_from_metadata(metadata: &MetadataMap) -> Option<TraceContext> {
    let traceparent = metadata
        .get(TRACEPARENT_HEADER)
        .and_then(|v| v.to_str().ok())?;

    parse_traceparent(traceparent).map(|mut ctx| {
        ctx.tracestate = metadata
            .get(TRACESTATE_HEADER)
            .and_then(|v| v.to_str().ok())
            .map(String::from);
        ctx
    })
}

/// Injects a W3C Trace Context into gRPC metadata for outgoing calls.
pub fn inject_trace_context_into_metadata(metadata: &mut MetadataMap, ctx: &TraceContext) {
    if let Ok(val) = ctx.to_traceparent().parse() {
        metadata.insert(TRACEPARENT_HEADER, val);
    }
    if let Some(ref tracestate) = ctx.tracestate {
        if let Ok(val) = tracestate.parse() {
            metadata.insert(TRACESTATE_HEADER, val);
        }
    }
}

/// Injects a trace context into a tonic::Request's metadata.
pub fn inject_trace_context_into_request<T>(request: &mut tonic::Request<T>, ctx: &TraceContext) {
    inject_trace_context_into_metadata(request.metadata_mut(), ctx);
}

/// Extracts trace context from gRPC metadata, creating a new root context if
/// none is present in the incoming headers.
pub fn extract_or_create_trace_context(metadata: &MetadataMap) -> TraceContext {
    extract_trace_context_from_metadata(metadata).unwrap_or_else(TraceContext::new_root)
}

/// Sets the OpenTelemetry parent context on the given `tracing::Span` so that
/// the `tracing-opentelemetry` layer exports it as a child of the incoming
/// trace. Call this right after creating an `info_span!` from an incoming
/// request's `TraceContext`.
pub fn set_otel_parent(span: &tracing::Span, ctx: &TraceContext) {
    let trace_id = TraceId::from_hex(&ctx.trace_id).unwrap_or(TraceId::INVALID);
    let span_id = SpanId::from_hex(&ctx.parent_span_id).unwrap_or(SpanId::INVALID);
    let flags = u8::from_str_radix(&ctx.trace_flags, 16).unwrap_or(1);

    let span_context = SpanContext::new(
        trace_id,
        span_id,
        TraceFlags::new(flags),
        true, // remote = true (came from another service)
        TraceState::default(),
    );

    let otel_context = opentelemetry::Context::new().with_remote_span_context(span_context);
    span.set_parent(otel_context);
}

/// Builds a `TraceContext` from the current `tracing::Span`'s OTel span context.
/// Returns `None` when there is no active OTel span (e.g. OTLP export is
/// disabled). Use this for outgoing cross-process calls so the injected
/// `parent_span_id` matches the real OTel span ID rather than a randomly
/// generated one.
pub fn trace_context_from_current_otel_span() -> Option<TraceContext> {
    let span = tracing::Span::current();
    let otel_ctx = span.context();
    let span_ctx = otel_ctx.span().span_context().clone();
    if span_ctx.is_valid() {
        Some(TraceContext {
            trace_id: format!("{}", span_ctx.trace_id()),
            parent_span_id: format!("{}", span_ctx.span_id()),
            trace_flags: format!("{:02x}", span_ctx.trace_flags().to_u8()),
            tracestate: None,
        })
    } else {
        None
    }
}

/// Generates a 32-hex-character random trace ID.
fn generate_trace_id() -> String {
    Uuid::new_v4().as_simple().to_string()
}

/// Generates a 16-hex-character random span ID.
fn generate_span_id() -> String {
    let bytes = Uuid::new_v4().into_bytes();
    bytes[..8]
        .iter()
        .fold(String::with_capacity(16), |mut s, b| {
            use std::fmt::Write;
            write!(s, "{:02x}", b).unwrap();
            s
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_traceparent() {
        let ctx =
            parse_traceparent("00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01").unwrap();
        assert_eq!(ctx.trace_id, "0af7651916cd43dd8448eb211c80319c");
        assert_eq!(ctx.parent_span_id, "b7ad6b7169203331");
        assert_eq!(ctx.trace_flags, "01");
    }

    #[test]
    fn test_parse_invalid_traceparent() {
        assert!(parse_traceparent("invalid").is_none());
        assert!(parse_traceparent("00-short-id-01").is_none());
        assert!(parse_traceparent("").is_none());
        // W3C spec: all-zeros trace_id is invalid.
        assert!(
            parse_traceparent("00-00000000000000000000000000000000-b7ad6b7169203331-01").is_none()
        );
        // W3C spec: all-zeros parent_id is invalid.
        assert!(
            parse_traceparent("00-0af7651916cd43dd8448eb211c80319c-0000000000000000-01").is_none()
        );
    }

    #[test]
    fn test_roundtrip_traceparent() {
        let original = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";
        let ctx = parse_traceparent(original).unwrap();
        assert_eq!(ctx.to_traceparent(), original);
    }

    #[test]
    fn test_new_root_generates_valid_context() {
        let ctx = TraceContext::new_root();
        assert_eq!(ctx.trace_id.len(), 32);
        assert_eq!(ctx.parent_span_id.len(), 16);
        assert_eq!(ctx.trace_flags, "01");
    }

    #[test]
    fn test_new_child_preserves_trace_id() {
        let parent = TraceContext::new_root();
        let child = parent.new_child();
        assert_eq!(child.trace_id, parent.trace_id);
        assert_ne!(child.parent_span_id, parent.parent_span_id);
    }

    #[test]
    fn test_generated_ids_are_unique() {
        let ids: Vec<_> = (0..100).map(|_| TraceContext::new_root()).collect();
        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                assert_ne!(ids[i].trace_id, ids[j].trace_id);
                assert_ne!(ids[i].parent_span_id, ids[j].parent_span_id);
            }
        }
    }

    #[test]
    fn test_extract_from_metadata() {
        let mut metadata = MetadataMap::new();
        metadata.insert(
            TRACEPARENT_HEADER,
            "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01"
                .parse()
                .unwrap(),
        );
        metadata.insert(TRACESTATE_HEADER, "vendor=opaque".parse().unwrap());

        let ctx = extract_trace_context_from_metadata(&metadata).unwrap();
        assert_eq!(ctx.trace_id, "0af7651916cd43dd8448eb211c80319c");
        assert_eq!(ctx.tracestate.unwrap(), "vendor=opaque");
    }

    #[test]
    fn test_inject_into_metadata() {
        let ctx = TraceContext {
            trace_id: "0af7651916cd43dd8448eb211c80319c".to_string(),
            parent_span_id: "b7ad6b7169203331".to_string(),
            trace_flags: "01".to_string(),
            tracestate: Some("vendor=opaque".to_string()),
        };
        let mut metadata = MetadataMap::new();
        inject_trace_context_into_metadata(&mut metadata, &ctx);

        assert_eq!(
            metadata.get(TRACEPARENT_HEADER).unwrap().to_str().unwrap(),
            "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01"
        );
        assert_eq!(
            metadata.get(TRACESTATE_HEADER).unwrap().to_str().unwrap(),
            "vendor=opaque"
        );
    }
}

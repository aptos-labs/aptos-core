// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Compares the events the two VMs emit.
//!
//! Each VM records its events in a native extension (the legacy VM's
//! `NativeEventContext`, MonoMove's `EventStore`), in emission order. The
//! comparison is positional: it checks the two sequences have the same length
//! and then zips them, comparing each event's BCS payload, type, and kind.
//!
//! The legacy VM yields a `TypeTag` per event; MonoMove yields an interned
//! type. To compare types consistently, both are rendered through MonoMove's
//! `type_to_string` (the legacy tag is resolved into the interner first), so a
//! difference reflects the type, not the formatting.

use crate::resolver::resolve_type_tag;
use mono_move_core::types::type_to_string;
use mono_move_global_context::ExecutionGuard;
use move_core_types::language_storage::TypeTag;

/// The event format, with the data the comparison checks per kind. The legacy
/// VM's V1 (handle) events carry a guid and sequence number; V2 (module) events
/// carry neither.
#[derive(Debug, PartialEq, Eq)]
pub enum EventKindCmp {
    /// Module event (`ContractEvent::V2`).
    V2,
    /// Handle event (`ContractEvent::V1`).
    V1 { guid: Vec<u8>, sequence_number: u64 },
}

/// One event emitted by the legacy VM. The type is kept as a `TypeTag` and
/// resolved during comparison so both VMs' types render the same way.
pub struct V1Event {
    pub type_tag: TypeTag,
    /// BCS-encoded event payload.
    pub data: Vec<u8>,
    pub kind: EventKindCmp,
}

/// One event emitted by MonoMove, with its type already rendered.
pub struct V2Event {
    pub type_str: String,
    /// BCS-encoded event payload.
    pub data: Vec<u8>,
    pub kind: EventKindCmp,
}

/// Aggregated event comparison.
pub struct EventDiff {
    pub v1_count: usize,
    pub v2_count: usize,
    pub matched: usize,
    pub mismatched: usize,
    /// Human-readable reason for each mismatched event, in order.
    pub mismatches: Vec<String>,
}

/// Compares the two VMs' event sequences positionally: lengths must match, and
/// each zipped pair must agree on type, BCS payload, and kind. A length
/// difference counts the surplus events as mismatched.
pub fn compare_events(guard: &ExecutionGuard, v1: &[V1Event], v2: &[V2Event]) -> EventDiff {
    let mut matched = 0;
    let mut mismatched = 0;
    let mut mismatches = Vec::new();

    for (i, (a, b)) in v1.iter().zip(v2.iter()).enumerate() {
        // Render the legacy tag through MonoMove's formatter so a difference is
        // about the type, not the textual form. A tag MonoMove cannot represent
        // (e.g. a function tag) falls back to the canonical string, which will
        // not match and so reads as a mismatch.
        let v1_type = resolve_type_tag(guard, &a.type_tag)
            .map(type_to_string)
            .unwrap_or_else(|_| a.type_tag.to_canonical_string());

        let mut reasons = Vec::new();
        if v1_type != b.type_str {
            reasons.push(format!("type {v1_type} != {}", b.type_str));
        }
        if a.data != b.data {
            reasons.push(format!(
                "data differs (v1 {} bytes, v2 {} bytes)",
                a.data.len(),
                b.data.len(),
            ));
        }
        if a.kind != b.kind {
            reasons.push("kind differs".to_string());
        }
        if reasons.is_empty() {
            matched += 1;
        } else {
            mismatched += 1;
            mismatches.push(format!("event #{i}: {}", reasons.join("; ")));
        }
    }

    // Surplus events on either side have no counterpart to match.
    let surplus = v1.len().abs_diff(v2.len());
    if surplus > 0 {
        mismatched += surplus;
        mismatches.push(format!(
            "event count differs: v1={} v2={}",
            v1.len(),
            v2.len(),
        ));
    }

    EventDiff {
        v1_count: v1.len(),
        v2_count: v2.len(),
        matched,
        mismatched,
        mismatches,
    }
}

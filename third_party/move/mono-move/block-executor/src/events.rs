// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Turns the [`EventStore`] extension's entries into [`ContractEvent`]s, so
//! stub outputs still carry real events and can be compared against the
//! legacy VM. Mirrors the replay-benchmark / testsuite finalization, but
//! produces events instead of rendered strings.

use crate::resource_provider::type_tag_for;
use anyhow::{anyhow, Context, Result};
use aptos_types::{contract_event::ContractEvent, event::EventKey};
use mono_move_core::native::NativeExtensions;
use mono_move_global_context::ExecutionGuard;
use mono_move_natives::{EventKind, EventStore};
use mono_move_runtime::serialize;

/// Serializes every event the execution emitted, in emission order.
///
/// # Safety
///
/// The interpreter heap must still be live: event payloads are flat copies,
/// but their interior pointers (vectors etc.) point into the heap. The guard
/// must be held (layouts and interned types).
pub(crate) unsafe fn finalize_events(
    extensions: &NativeExtensions,
    guard: &ExecutionGuard<'_>,
) -> Result<Vec<ContractEvent>> {
    let store = extensions
        .get_mut::<EventStore>()
        .map_err(|e| anyhow!("event store not installed: {}", e))?;
    store
        .entries()
        .iter()
        .map(|entry| {
            let type_tag = type_tag_for(entry.msg_ty)
                .map_err(|e| anyhow!("failed to reconstruct the event type tag: {}", e))?;
            // SAFETY: forwarded from this function's contract — the heap and
            // the guard are live.
            let blob = unsafe { serialize(guard, entry.msg_data.as_ptr(), entry.msg_ty) }
                .map_err(|e| anyhow!("failed to serialize an event value: {}", e))?;
            match &entry.kind {
                EventKind::V2 => ContractEvent::new_v2(type_tag, blob),
                EventKind::V1 {
                    guid,
                    sequence_number,
                } => {
                    let key: EventKey =
                        bcs::from_bytes(guid).context("guid does not decode to an EventKey")?;
                    ContractEvent::new_v1(key, *sequence_number, type_tag, blob)
                },
            }
        })
        .collect()
}

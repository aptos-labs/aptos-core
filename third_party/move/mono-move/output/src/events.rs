// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! MonoMove event store → Aptos [`ContractEvent`]s.

use crate::error::OutputError;
use aptos_types::{contract_event::ContractEvent, event::EventKey};
use mono_move_core::{
    native::NativeExtensions, type_tag_of, value_layout::LayoutProvider, VMInternalError, VMResult,
};
use mono_move_natives::{EventKind, EventStore};
use mono_move_runtime::serialize;

/// Materializes the emitted events into [`ContractEvent`]s, in emission order.
/// `layouts` BCS-serializes each event's value.
///
/// # Safety
///
/// The heap the event values point into must be live.
pub unsafe fn to_contract_events(
    extensions: &NativeExtensions,
    layouts: &impl LayoutProvider,
) -> VMResult<Vec<ContractEvent>> {
    let store = extensions.get_mut::<EventStore>()?;
    store
        .entries()
        .iter()
        .map(|entry| {
            let type_tag = type_tag_of(entry.msg_ty).ok_or(OutputError::InvalidEventType)?;
            // SAFETY: forwarded from this function's contract — the heap is live.
            let data = unsafe { serialize(layouts, entry.msg_data.as_ptr(), entry.msg_ty) }?;
            let event = match &entry.kind {
                EventKind::V2 => ContractEvent::new_v2(type_tag, data),
                EventKind::V1 {
                    guid,
                    sequence_number,
                } => {
                    let key: EventKey =
                        bcs::from_bytes(guid).map_err(OutputError::InvalidEventGuid)?;
                    ContractEvent::new_v1(key, *sequence_number, type_tag, data)
                },
            };
            event.map_err(|e| VMInternalError::new(OutputError::InvalidEvent(e.to_string())))
        })
        .collect()
}

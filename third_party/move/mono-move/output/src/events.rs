// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! MonoMove event store → Aptos [`ContractEvent`]s.

use crate::error::OutputError;
use aptos_types::{contract_event::ContractEvent, event::EventKey};
use mono_move_core::{type_tag_of, value_layout::LayoutProvider, VMInternalError, VMResult};
use mono_move_natives::{EventKind, EventStore};
use mono_move_runtime::{serialize, SessionEffects};

/// Materializes the emitted events into [`ContractEvent`]s, in emission order.
/// The effects retain every backing allocation reachable from the event values;
/// `layouts` must describe those values' interned types.
pub fn to_contract_events<L: LayoutProvider + ?Sized>(
    effects: &SessionEffects,
    layouts: &L,
) -> VMResult<Vec<ContractEvent>> {
    let store = effects.extension::<EventStore>()?;

    // SAFETY: the effects retain their frozen local heap and `layouts` describes
    // the event values' types; no GC can run after execution.
    unsafe { to_contract_events_from_store(&store, layouts) }
}

/// Materializes an [`EventStore`] into [`ContractEvent`]s, in emission order.
///
/// # Safety
///
/// Every allocation reachable through an entry's `msg_data` must remain live,
/// `layouts` must describe the entry's interned type, and no GC may run during
/// serialization.
pub unsafe fn to_contract_events_from_store<L: LayoutProvider + ?Sized>(
    store: &EventStore,
    layouts: &L,
) -> VMResult<Vec<ContractEvent>> {
    store
        .entries()
        .iter()
        .map(|entry| {
            let type_tag = type_tag_of(entry.msg_ty).ok_or(OutputError::InvalidEventType)?;
            // SAFETY: forwarded from this function's contract.
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

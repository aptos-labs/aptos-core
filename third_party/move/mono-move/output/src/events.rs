// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! MonoMove event store → Aptos [`ContractEvent`]s.

use crate::error::OutputError;
use aptos_types::{
    account_config::{NEW_EPOCH_EVENT_MOVE_TYPE_TAG, NEW_EPOCH_EVENT_V2_MOVE_TYPE_TAG},
    contract_event::ContractEvent,
    event::EventKey,
};
use mono_move_core::{type_tag_of, value_layout::LayoutProvider, VMInternalError, VMResult};
use mono_move_natives::{EventEntry, EventKind, EventStore};
use mono_move_runtime::{serialize, SessionEffects};

/// Whether the effects emitted a reconfiguration (new-epoch) event. Only each
/// event's type is inspected, not its payload, so no value is serialized.
// TODO(perf): record on event emit or when processing gas cost for storage for all events.
pub fn has_new_epoch_event(effects: &SessionEffects) -> VMResult<bool> {
    let store = effects.extension::<EventStore>()?;
    Ok(store.entries().iter().any(|entry| {
        type_tag_of(entry.msg_ty).is_some_and(|tag| {
            tag == *NEW_EPOCH_EVENT_MOVE_TYPE_TAG || tag == *NEW_EPOCH_EVENT_V2_MOVE_TYPE_TAG
        })
    }))
}

/// Materializes the emitted events into [`ContractEvent`]s, in emission order,
/// reporting every event that fails rather than the first, so that one failure
/// cannot hide another. The effects retain every backing allocation reachable
/// from the event values; `layouts` must describe those values' interned types.
//
// TODO(security): prove at compile time that the execution guard backing
// `layouts` is held.
pub fn to_contract_events<L: LayoutProvider + ?Sized>(
    effects: &SessionEffects,
    layouts: &L,
) -> Result<Vec<ContractEvent>, Vec<VMInternalError>> {
    let store = effects.extension::<EventStore>().map_err(|e| vec![e])?;
    let mut events = vec![];
    let mut errors = vec![];
    for entry in store.entries() {
        // SAFETY: the effects retain their frozen local heap and `layouts`
        // describes the event values' types; no GC can run after execution.
        match unsafe { to_contract_event(entry, layouts) } {
            Ok(event) => events.push(event),
            Err(err) => errors.push(err),
        }
    }
    if errors.is_empty() {
        Ok(events)
    } else {
        Err(errors)
    }
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
        // SAFETY: forwarded from this function's contract.
        .map(|entry| unsafe { to_contract_event(entry, layouts) })
        .collect()
}

/// Materializes one emitted event. The event's key and type are checked before
/// its payload is serialized, so a payload MonoMove cannot serialize yet does
/// not hide a malformed key or type.
///
/// # Safety
///
/// As for [`to_contract_events_from_store`].
unsafe fn to_contract_event<L: LayoutProvider + ?Sized>(
    entry: &EventEntry,
    layouts: &L,
) -> VMResult<ContractEvent> {
    let type_tag = type_tag_of(entry.msg_ty).ok_or(OutputError::InvalidEventType)?;
    let key = match &entry.kind {
        EventKind::V2 => None,
        EventKind::V1 {
            guid,
            sequence_number,
        } => Some((
            bcs::from_bytes::<EventKey>(guid).map_err(OutputError::InvalidEventGuid)?,
            *sequence_number,
        )),
    };
    // SAFETY: forwarded from this function's contract.
    let data = unsafe { serialize(layouts, entry.msg_data.as_ptr(), entry.msg_ty) }?;
    let event = match key {
        None => ContractEvent::new_v2(type_tag, data),
        Some((key, sequence_number)) => ContractEvent::new_v1(key, sequence_number, type_tag, data),
    };
    event.map_err(|e| VMInternalError::new(OutputError::InvalidEvent(e.to_string())))
}

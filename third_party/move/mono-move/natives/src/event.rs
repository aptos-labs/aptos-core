// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Natives for the `event` module, plus the backing event store.

use crate::{polymorphic_natives, NativeEntry};
#[cfg(feature = "testing")]
use aptos_types::event::EventKey;
#[cfg(feature = "testing")]
use mono_move_core::native::{Opaque, Ref};
use mono_move_core::{
    native::{
        native_invariant_violation, NativeContext, NativeContextFamily, NativeExtension,
        NativeStatus, Vector,
    },
    types::{view_type, InternedType, Type},
    VMResult,
};
#[cfg(feature = "testing")]
use move_core_types::account_address::AccountAddress;

/// Number of bytes a heap pointer occupies in the flat value representation.
const POINTER_SIZE: usize = 8;

/// Byte offset of `GUID::ID::creation_num` within `EventHandle<T>`, whose first
/// field is a `u64` counter. `addr` follows immediately.
#[cfg(feature = "testing")]
const GUID_CREATION_NUM_OFFSET: usize = 8;

/// Axuiliary info/tag to distinguish the two event formats.
pub enum EventKind {
    /// Module event (`ContractEvent::V2`)
    V2,
    /// Handle event (`ContractEvent::V1`)
    V1 { guid: Vec<u8>, sequence_number: u64 },
}

/// Represents a recorded event. Supports both V1 and V2.
pub struct EventEntry {
    pub msg_ty: InternedType,
    pub msg_data: Vec<u8>,
    pub ptr_offsets: Vec<u32>,
    pub kind: EventKind,
}

/// Per-transaction store of emitted events, in emission order.
///
/// TODO(cleanup): This is currently implemented as a Rust struct, but should eventually be moved to
/// the VM's own heap.
#[derive(Default)]
pub struct EventStore {
    entries: Vec<EventEntry>,
    checkpoints: Vec<usize>,
}

impl EventStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Records an event.
    fn emit(
        &mut self,
        msg_ty: InternedType,
        msg_data: Vec<u8>,
        ptr_offsets: Vec<u32>,
        kind: EventKind,
    ) {
        self.entries.push(EventEntry {
            msg_ty,
            msg_data,
            ptr_offsets,
            kind,
        });
    }

    /// The recorded events, in emission order.
    pub fn entries(&self) -> &[EventEntry] {
        &self.entries
    }
}

impl NativeExtension for EventStore {
    unsafe fn relocate_roots(&mut self, relocate: &mut dyn FnMut(*mut u8) -> Option<*mut u8>) {
        for entry in &mut self.entries {
            for &off in &entry.ptr_offsets {
                let off = off as usize;
                let bytes: [u8; POINTER_SIZE] = entry.msg_data[off..off + POINTER_SIZE]
                    .try_into()
                    .expect("offsets keep each pointer slot in bounds");
                let ptr = usize::from_ne_bytes(bytes) as *mut u8;
                if let Some(new) = relocate(ptr) {
                    entry.msg_data[off..off + POINTER_SIZE]
                        .copy_from_slice(&(new as usize).to_ne_bytes());
                }
            }
        }
    }

    fn on_checkpoint(&mut self) {
        self.checkpoints.push(self.entries.len());
    }

    fn on_rollback(&mut self, n: usize) -> VMResult<()> {
        if n > self.checkpoints.len() {
            return Err(native_invariant_violation(format!(
                "event rollback({n}): only {} checkpoint(s)",
                self.checkpoints.len(),
            )));
        }
        let snapshot = self.checkpoints[self.checkpoints.len() - n];
        self.checkpoints.truncate(self.checkpoints.len() - n);
        self.entries.truncate(snapshot);
        Ok(())
    }
}

/// `0x1::event::write_module_event_to_store<T>(msg: T)`
//
// TODO(metering): charge gas.
pub fn native_write_module_event_to_store<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let msg_ty = ctx.ty_arg(0)?;

    // The event type must be nominal, and the module emitting it must be the
    // one that defines it. Enums are admitted here; an unsupported enum layout
    // is rejected later, when its pointer offsets are computed.
    let Type::Nominal { module_id, .. } = view_type(msg_ty) else {
        return Err(native_invariant_violation(
            "write_module_event_to_store: event type must be a struct or enum".into(),
        ));
    };
    let caller = ctx.caller_module().ok_or_else(|| {
        native_invariant_violation(
            "write_module_event_to_store: scripts cannot emit module events".into(),
        )
    })?;
    if caller != *module_id {
        return Err(native_invariant_violation(
            "write_module_event_to_store: caller module does not define the event type".into(),
        ));
    }

    let msg_data = ctx.arg_raw(0)?;
    let ptr_offsets = ctx.arg_ptr_offsets(0)?;
    ctx.get_extension::<EventStore>()?
        .emit(msg_ty, msg_data, ptr_offsets, EventKind::V2);
    Ok(NativeStatus::Success)
}

/// `0x1::event::write_to_event_store<T>(guid: vector<u8>, count: u64, msg: T)`
//
// TODO(metering): charge gas.
pub fn native_write_to_event_store<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is `guid: vector<u8>`.
    let guid_vec = unsafe { ctx.arg::<Vector<u8>>(0)? };
    let guid = unsafe { guid_vec.as_bytes() }.to_vec();
    // SAFETY: arg 1 is `count: u64`.
    let sequence_number = unsafe { ctx.arg::<u64>(1)? };
    let msg_data = ctx.arg_raw(2)?;
    let ptr_offsets = ctx.arg_ptr_offsets(2)?;
    let msg_ty = ctx.ty_arg(0)?;
    ctx.get_extension::<EventStore>()?
        .emit(msg_ty, msg_data, ptr_offsets, EventKind::V1 {
            guid,
            sequence_number,
        });
    Ok(NativeStatus::Success)
}

/// Builds a `vector<T>` holding a copy of every entry of type `msg_ty` that
/// `selects` accepts, in emission order.
#[cfg(feature = "testing")]
fn collect_events<'a, C: NativeContext>(
    ctx: &'a C,
    msg_ty: InternedType,
    selects: impl Fn(&EventEntry) -> bool,
) -> VMResult<Vector<'a, Opaque>> {
    let elem_size = ctx.value_size(msg_ty)?;
    let descriptor = ctx.required_descriptor(0).ok_or_else(|| {
        native_invariant_violation("emitted events: missing vector<T> descriptor".into())
    })?;

    // Indices only: the allocation below can collect, and a collection
    // re-borrows every extension.
    let indices = {
        let store = ctx.get_extension::<EventStore>()?;
        store
            .entries()
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry.msg_ty == msg_ty && selects(entry))
            .map(|(index, _)| index)
            .collect::<Vec<_>>()
    };

    let vector = ctx.new_vector(descriptor, elem_size, indices.len() as u64)?;

    // Read after the allocation, so the payloads' pointers reflect any
    // relocation it caused, and owned so the borrow is gone before the deep
    // copies below can collect.
    let data = {
        let store = ctx.get_extension::<EventStore>()?;
        let mut data = Vec::with_capacity(indices.len() * elem_size as usize);
        for &index in &indices {
            data.extend_from_slice(&store.entries()[index].msg_data);
        }
        data
    };
    // SAFETY: each payload is the event value's in-frame image, held in a Rust
    // `Vec` outside the heap, and the store keeps every pointer it holds live.
    unsafe { ctx.vector_write_elements(&vector, elem_size, &data) }?;
    Ok(vector)
}

/// `0x1::event::emitted_events<T>(): vector<T>` (test-only)
#[cfg(feature = "testing")]
pub fn native_emitted_events<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let msg_ty = ctx.ty_arg(0)?;
    let vector = collect_events(ctx, msg_ty, |entry| matches!(entry.kind, EventKind::V2))?;
    // SAFETY: return 0 is `vector<T>`.
    unsafe { ctx.set_return(0, vector) }?;
    Ok(NativeStatus::Success)
}

/// `0x1::event::emitted_events_by_handle<T>(handle: &EventHandle<T>): vector<T>`
/// (test-only)
#[cfg(feature = "testing")]
pub fn native_emitted_events_by_handle<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let msg_ty = ctx.ty_arg(0)?;

    // `EventHandle<T> { counter: u64, guid: GUID }` with `GUID { id: ID }` and
    // `ID { creation_num: u64, addr: address }` flattens to `counter` at 0,
    // `creation_num` at 8 and `addr` at 16.
    // SAFETY: arg 0 is `&EventHandle<T>`.
    let handle = unsafe { ctx.arg::<Ref<Opaque>>(0)? };
    let base = handle.ptr();
    // SAFETY: `base` references a live `EventHandle<T>`, so both fields lie
    // within it. Nothing allocates before these reads.
    let (creation_num, addr) = unsafe {
        let mut addr = [0u8; AccountAddress::LENGTH];
        std::ptr::copy_nonoverlapping(
            base.add(GUID_CREATION_NUM_OFFSET + std::mem::size_of::<u64>()),
            addr.as_mut_ptr(),
            AccountAddress::LENGTH,
        );
        (
            std::ptr::read_unaligned(base.add(GUID_CREATION_NUM_OFFSET) as *const u64),
            AccountAddress::new(addr),
        )
    };
    let key = EventKey::new(creation_num, addr);

    let vector = collect_events(ctx, msg_ty, |entry| match &entry.kind {
        EventKind::V2 => false,
        // A guid that does not parse cannot exist on the legacy VM, which
        // validates it at emit time; skipping matches "no such event".
        EventKind::V1 { guid, .. } => bcs::from_bytes::<EventKey>(guid).is_ok_and(|k| k == key),
    })?;
    // SAFETY: return 0 is `vector<T>`.
    unsafe { ctx.set_return(0, vector) }?;
    Ok(NativeStatus::Success)
}

/// Natives for the `event` module.
pub fn make_all_event_natives<F: NativeContextFamily>() -> Vec<NativeEntry<F>> {
    polymorphic_natives![
        (
            "0x1::event::write_module_event_to_store",
            native_write_module_event_to_store
        ),
        (
            "0x1::event::write_to_event_store",
            native_write_to_event_store
        ),
    ]
}

/// Test-only natives for the `event` module.
#[cfg(feature = "testing")]
pub fn make_all_event_test_natives<F: NativeContextFamily>() -> Vec<NativeEntry<F>> {
    polymorphic_natives![
        ("0x1::event::emitted_events", native_emitted_events),
        (
            "0x1::event::emitted_events_by_handle",
            native_emitted_events_by_handle
        ),
    ]
}

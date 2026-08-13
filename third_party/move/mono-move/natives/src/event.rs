// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Natives for the `event` module, plus the backing event store.

use crate::{polymorphic_natives, NativeEntry};
use mono_move_core::{
    native::{
        native_invariant_violation, NativeContext, NativeContextFamily, NativeExtension,
        NativeStatus, Vector,
    },
    types::{view_type, InternedType, Type},
    VMResult,
};

/// Number of bytes a heap pointer occupies in the flat value representation.
const POINTER_SIZE: usize = 8;

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

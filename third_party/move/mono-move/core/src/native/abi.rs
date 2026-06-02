// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use thiserror::Error;

/// Location and size of an argument or return value in the calling frame.
//
// TODO: check whether there's already an equivalent (offset, size) type
// defined elsewhere in the codebase that could be reused instead.
#[derive(Debug, Clone, Copy)]
pub struct FrameSlot {
    /// Byte offset from the start of the native function's frame.
    pub offset: u32,
    /// Byte size of the slot.
    pub size: u32,
}

/// ABI descriptor for a native function: where its arguments and return
/// values sit in the calling frame, plus a few derived offsets the
/// interpreter consults on every dispatch.
//
// TODO: add `heap_ptr_offsets: Vec<u32>` so the GC can scan a native's
// frame correctly. Needed before any native can heap-allocate.
#[derive(Debug, Clone)]
pub struct NativeABI {
    args: Vec<FrameSlot>,
    returns: Vec<FrameSlot>,
    args_end: u32,
    total_frame_size: u32,
}

#[derive(Debug, Clone, Error)]
pub enum NativeABIError {
    #[error("{kind} slots not sorted by offset at index {idx}")]
    Unsorted { kind: &'static str, idx: usize },
    #[error("{kind} slot {idx} overlaps with previous slot")]
    Overlap { kind: &'static str, idx: usize },
}

impl NativeABI {
    /// Safe constructor for a NativeABI that also validates the ABI is well-formed.
    /// `args` and `returns` must be sorted by offset and must not overlap.
    pub fn new(args: Vec<FrameSlot>, returns: Vec<FrameSlot>) -> Result<Self, NativeABIError> {
        check_well_formed(&args, "arg")?;
        check_well_formed(&returns, "return")?;
        let args_end = args.iter().map(|s| s.offset + s.size).max().unwrap_or(0);
        let returns_end = returns.iter().map(|s| s.offset + s.size).max().unwrap_or(0);
        Ok(Self {
            args,
            returns,
            args_end,
            total_frame_size: args_end.max(returns_end),
        })
    }

    pub fn args(&self) -> &[FrameSlot] {
        &self.args
    }

    pub fn returns(&self) -> &[FrameSlot] {
        &self.returns
    }

    pub fn args_end(&self) -> u32 {
        self.args_end
    }

    pub fn total_frame_size(&self) -> u32 {
        self.total_frame_size
    }
}

fn check_well_formed(slots: &[FrameSlot], kind: &'static str) -> Result<(), NativeABIError> {
    for i in 1..slots.len() {
        let prev = &slots[i - 1];
        let curr = &slots[i];
        if curr.offset <= prev.offset {
            return Err(NativeABIError::Unsorted { kind, idx: i });
        }
        if prev.offset + prev.size > curr.offset {
            return Err(NativeABIError::Overlap { kind, idx: i });
        }
    }
    Ok(())
}

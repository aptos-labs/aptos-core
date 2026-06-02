// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

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

/// ABI descriptor for a native function, which contains information about
/// its frame layout -- currently offsets and sizes of arguments and return values, but
/// more may be added later, such as type info.
#[derive(Debug, Clone)]
pub struct NativeABI {
    /// Arg slot layouts, in declaration order.
    pub args: Vec<FrameSlot>,
    /// Return slot layouts, in declaration order.
    pub returns: Vec<FrameSlot>,
}

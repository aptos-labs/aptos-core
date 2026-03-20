// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

mod instruction;

pub use instruction::{
    CodeOffset, DescriptorId, FrameOffset, MicroOp, ENUM_DATA_OFFSET, ENUM_TAG_OFFSET,
    FRAME_METADATA_SIZE, OBJECT_HEADER_SIZE, STRUCT_DATA_OFFSET,
};

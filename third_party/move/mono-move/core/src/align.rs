// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Alignment constants and helpers, re-exported from `mono-move-mem` (the
//! crate that owns [`MAX_ALIGN`]) because most crates depend on this one.

pub use mono_move_mem::{
    align_max, align_up, align_up_u32, checked_align_max, checked_align_up, checked_align_up_u32,
    MAX_ALIGN,
};

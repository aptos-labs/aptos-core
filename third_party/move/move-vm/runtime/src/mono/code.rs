// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::mono::{
    context::{ExecutionContext, FunctionId, StorageId},
    memory::Memory,
};
use move_binary_format::{errors::PartialVMResult, file_format::LocalIndex};

pub enum MonoCode<'code> {
    LoadConst {
        value: &'code [u8],
    },
    CopyLocal {
        size: usize,
        local: LocalIndex,
    },
    StoreLocal {
        size: usize,
        local: LocalIndex,
    },
    BorrowLocal {
        local: LocalIndex,
    },
    ReadRef {
        size: usize,
    },
    WriteRef {
        size: usize,
    },
    BorrowGlobal {
        storage_key: StorageId,
    },
    BorrowField {
        byte_offset: usize,
    },
    CallPrimitive {
        size: usize,
        operation: fn(&dyn ExecutionContext, &mut Memory, usize) -> PartialVMResult<()>,
    },
    CallFunction {
        function_id: FunctionId,
    },
    Return {
        size: usize,
    },
    Branch {
        offset: usize,
    },
    BranchTrue {
        offset: usize,
    },
    BranchFalse {
        offset: usize,
    },
}

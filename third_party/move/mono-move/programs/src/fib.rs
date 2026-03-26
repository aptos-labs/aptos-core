// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Fibonacci — recursive, exponential-time implementation.
//!
//! Useful as a benchmark for function call overhead since the algorithm
//! does almost nothing except recurse.

/// Test cases: (input, expected output).
pub const FIB_CASES: &[(u64, u64)] = &[(0, 0), (1, 1), (2, 1), (10, 55), (20, 6765)];

// ---------------------------------------------------------------------------
// Native Rust
// ---------------------------------------------------------------------------

pub fn native_fib(n: u64) -> u64 {
    if n < 2 {
        return n;
    }
    native_fib(n - 1) + native_fib(n - 2)
}

// ---------------------------------------------------------------------------
// Micro-op
// ---------------------------------------------------------------------------

/// Pseudocode:
///   fn fib(n: u64) -> u64 {
///       if n == 0 { return 0; }
///       if n < 2  { return 1; }
///       let a = fib(n - 1);
///       let b = fib(n - 2);
///       return a + b;
///   }
///
/// Frame layout:
///   [0]  n (arg) / result  [8]  tmp
///   [16] metadata (24 bytes)
///   [40] callee: n / callee_result
#[cfg(feature = "micro-op")]
mod micro_op {
    use mono_move_alloc::GlobalArenaPtr;
    use mono_move_core::{
        CodeOffset as CO, FrameOffset as FO, Function, MicroOp::*, FRAME_METADATA_SIZE,
    };
    use mono_move_runtime::ObjectDescriptor;

    pub fn program() -> (Vec<Function>, Vec<ObjectDescriptor>) {
        let n = 0u32;
        let result = n;
        let tmp = 8u32;
        let args_and_locals_size = 16u32;
        let callee_n = args_and_locals_size + FRAME_METADATA_SIZE as u32;
        let callee_result = callee_n;

        #[rustfmt::skip]
        let code = vec![
            // if n != 0 goto CHECKGE2
            JumpNotZeroU64 { target: CO(3), src: FO(n) },
            StoreImm8 { dst: FO(result), imm: 0 },
            Return,
            // CHECKGE2: if n >= 2 goto RECURSE
            JumpGreaterEqualU64Imm { target: CO(6), src: FO(n), imm: 2 },
            StoreImm8 { dst: FO(result), imm: 1 },
            Return,
            // RECURSE: tmp = fib(n - 1)
            SubU64Imm { dst: FO(callee_n), src: FO(n), imm: 1 },
            CallFunc { func_id: 0 },
            Move8 { dst: FO(tmp), src: FO(callee_result) },
            // fib(n - 2)
            SubU64Imm { dst: FO(callee_n), src: FO(n), imm: 2 },
            CallFunc { func_id: 0 },
            // result = tmp + fib(n - 2)
            AddU64 { dst: FO(result), lhs: FO(tmp), rhs: FO(callee_result) },
            Return,
        ];

        let func = Function {
            name: GlobalArenaPtr::from_static("fib"),
            code,
            args_size: 8,
            args_and_locals_size: args_and_locals_size as usize,
            extended_frame_size: (callee_n + 8) as usize,
            zero_frame: false,
            pointer_offsets: vec![],
        };

        (vec![func], vec![ObjectDescriptor::Trivial])
    }
}

#[cfg(feature = "micro-op")]
pub use micro_op::program as micro_op_fib;

// ---------------------------------------------------------------------------
// Move bytecode
// ---------------------------------------------------------------------------

#[cfg(feature = "move-bytecode")]
mod move_bytecode {
    use move_binary_format::file_format::CompiledModule;

    pub const SOURCE: &str = "
module 0x1::fib {
    public fun fib(n: u64): u64 {
        if (n < 2) { return n };
        fib(n - 1) + fib(n - 2)
    }
}
";

    /// Compile the embedded Move source into a `CompiledModule`.
    pub fn program() -> CompiledModule {
        crate::compile_move_source(SOURCE)
    }
}

#[cfg(feature = "move-bytecode")]
pub use move_bytecode::program as move_bytecode_fib;

# Micro-Ops Instruction Set

The runtime executes **micro-ops** — a low-level, flat instruction set defined in the `mono-move-micro-ops` crate (`MicroOp` enum). Micro-ops are produced by the recompiler from Move bytecode after monomorphization and destackification. They operate on frame-relative byte offsets rather than a virtual operand stack.

See the `MicroOp` enum in `mono-move-micro-ops/src/instruction.rs` for the full instruction listing and per-instruction documentation.

## Design Principles

**Highly specialized.** We are not aiming for a minimal instruction set. Specialized variants (e.g. `HeapMoveFrom8` for the common 8-byte case, fused compare-and-branch) are preferred when they enable faster dispatch or more efficient interpreter code.

**Fixed-size instructions.** All `MicroOp` variants are the same size (currently 24 bytes; we aim to bring this down to 16). This means the program counter is a simple index into the instruction array.

**Variable-size frame slots.** Frame slots are variable-sized. Each micro-op makes the width explicit — either baked into the opcode name (e.g. `Move8` = 8 bytes) or as an explicit `size` field.

**Compiler-driven argument/return placement.** The compiler emits explicit micro-ops to place arguments into the callee's frame before a call, and to write return values at the start of the callee's frame before a return. The `CallFunc`/`Return` instructions themselves only handle the implicit metadata (saving/restoring `pc`, `fp`, `func_ptr`). This keeps the call/return instructions simple and gives the compiler full control over data movement.

**Super-instructions.** Early benchmarks have shown that dispatch overhead accounts for a significant fraction of execution time — not because dispatch is slow, but because the per-instruction work is fast enough that dispatch becomes the bottleneck. Fused instructions (e.g. compare-and-branch) reduce the number of dispatches and are valuable. We should look for more fusion opportunities.

**Inline structs via existing ops.** Most structs are stored inline in the frame, not heap-allocated. The existing data movement ops (`Move`, `Move8`) already handle inline structs. The `Heap*` ops are only for values that must live on the heap (e.g. enums, or structs too large to inline).

## Instruction Categories

- **Data movement**: Immediate stores, register-to-register moves, variable-size copies between frame slots.
- **Arithmetic and bitwise**: Checked integer operations (currently u64-only, overflow/underflow aborts).
- **Control flow**: Call/return, unconditional jumps, and fused compare-and-branch instructions.
- **Vector operations**: Create, length, push/pop, indexed load/store, borrow element.
- **Reference operations**: Borrow a stack local, borrow a vector element or heap object field, read/write through a fat pointer reference.
- **Heap object operations**: Allocate structs/enums, read/write fields, borrow fields through references.
- **Gas metering**: Per-basic-block gas charge (placeholder).
- **Global storage**: `MoveTo`, `MoveFrom`, `BorrowGlobal`, `Exists` — not yet designed or implemented.
- **Debugging / testing**: `StoreRandomU64`, `ForceGC` for stress-testing.

## Addressing Modes

Most operands are frame-pointer-relative offsets (`FrameOffset`, a `u32` wrapper). Additional modes:

- **Immediate** (`*Imm` variants): constant value baked into the instruction.
- **Pointer + static offset** (`HeapMoveFrom`, `HeapMoveTo`): read a heap pointer from a frame slot, then access at a fixed byte offset.
- **Pointer + dynamic offset** (`VecLoadElem`, `VecStoreElem`): read a heap pointer, then compute the element address from a runtime index.

Operand ordering convention: destination (`dst`) or branch target (`target`) comes first, followed by sources and immediates.

## Naming Conventions

Micro-op names follow the pattern `{Op}{Modifier}{Type}{Size}`:

- **Op**: the operation (`Store`, `Move`, `Add`, `HeapMoveFrom`, `Vec`, ...)
- **Modifier**: addressing or source mode (`Imm` = immediate)
- **Type**: data type when relevant (`U64`, ...)
- **Size**: byte width when specialized (`8` = 8 bytes)

Examples: `StoreImm8`, `AddU64Imm`, `HeapMoveFrom8`, `VecLoadElem`.

## Not Yet Implemented

- **Global storage**: `MoveTo`, `MoveFrom`, `BorrowGlobal`, `Exists` — needed for interacting with the block-level storage cache. Not yet designed or implemented.
- **Comparison to register**: standalone `Eq`, `Neq`, `Lt`, `Gt`, `Le`, `Ge` that produce a value (as opposed to the fused compare-and-branch variants).
- **Boolean operations**: logical `Not`, `And`, `Or`.
- **Casting**: truncation and widening between integer types.
- **Additional integer widths**: u8, u16, u32, u128, u256, and signed integers.
- **Closures / function values**.
- **Abort**: exit with error code.

## Open Design Questions

- **Addressing modes**: Do we need other addressing modes beyond fp-relative, immediate, and pointer+offset? Should offsets be more packed than `u32`?
- **Size specialization**: `Move8` and `HeapMoveFrom8`/`HeapMoveTo8` fast-path 8 bytes. May want `Move16` (fat pointers) and others.
- **Immediate representation**: `StoreImm8` carries a `u64`, but is used for other 8-byte types (addresses, bools, pointers). Should immediates be `u64` or `[u8; 8]`?
- **Endianness**: The VM currently assumes native byte order. If instructions or memory layouts need to be portable across architectures, this will need revisiting.
- **Encoding**: Rust enum is convenient for prototyping but may not be optimal for production.

# mono-move-runtime

Runtime for the MonoMove VM: a register-based interpreter with a unified linear stack, bump-allocated heap, and Cheney's copying garbage collector.

## Overview

This crate executes **micro-ops** — a low-level, flat instruction set that a specializer produces from Move bytecode after monomorphization and destackification. It is not a general-purpose VM; it assumes its input has already been verified and lowered by the MonoMove compiler pipeline.

The runtime is at proof-of-concept stage. See `TODO.md` for the backlog of missing features.

## Architecture

### Modules

| Module | Purpose |
|---|---|
| `interpreter.rs` | Core interpreter loop (`step`/`run`), `InterpreterContext` (owns stack + heap + pc state), call/return protocol |
| `heap.rs` | Bump allocator, vector/struct/enum allocation, vector growth, Cheney's copying GC |
| `memory.rs` | `MemoryRegion` (owned 8-byte-aligned allocation), raw pointer helpers (`read_u64`, `write_ptr`, etc.) |
| `types.rs` | `ObjectDescriptor` (GC tracing layouts), `StepResult`, layout constants (header offsets, frame metadata offsets) |
| `verifier.rs` | Static verification of `Function` bodies before execution (frame bounds, jump targets, descriptor validity) |

### Key concepts

**Unified stack.** Call frames live in a single contiguous `MemoryRegion`. See `docs/stack_and_calling_convention.md` for the frame layout diagram and full calling convention.

**Bump-allocated heap with copying GC.** Heap objects (vectors, structs, enums) are bump-allocated. When the bump pointer hits the end, Cheney's copying GC runs: it walks the call stack using per-function `pointer_offsets` to find roots, then does a breadth-first copy of all reachable objects into a fresh to-space. Forwarding pointers handle cycles and double-scans.

**Object layout.** Every heap object starts with an 8-byte header: `[descriptor_id: u32, size_in_bytes: u32]`. The descriptor tells the GC how to trace internal pointers. Vectors additionally store a `length` field after the header (capacity is derived from `size_in_bytes`).

**Fat pointers.** References are 16-byte `(base_ptr, byte_offset)` pairs. This lets borrows point into the interior of heap objects (e.g., a struct field or vector element) without a separate indirection.

**Static verification.** Before execution, `verify_program` checks every function for frame-access bounds, metadata overlap, valid jump targets, and valid descriptor references. This prevents undefined behavior from malformed micro-ops.

### Safety model

The interpreter uses raw pointer arithmetic extensively (`unsafe`). Correctness depends on invariants maintained jointly by the compiler, verifier, and runtime:

1. **Frame metadata integrity** — saved `fp`/`pc`/`func_ptr` are written only by call/return, never by user micro-ops
2. **Pointer-slot accuracy** — `Function::pointer_offsets` exactly matches slots that hold live heap pointers
3. **Object header integrity** — `descriptor_id` and `size` are set by the allocator and never overwritten by user code

The verifier checks what it can statically; the rest is the compiler's responsibility.

## Related Docs

Detailed design documents live in `../docs/`:

| Doc | Covers |
|---|---|
| `stack_and_calling_convention.md` | Frame layout, call/return protocol, unified vs separate stack, GC root discovery via `pointer_offsets`, security considerations |
| `heap_and_gc.md` | Block/transaction memory management, bump allocator + Cheney's copying GC, GC design space analysis (four approaches), memory safety |
| `value_representation.md` | Heap object header, primitive/struct/enum/vector layouts, fat pointer references, vector growth semantics |
| `native_functions.md` | Native function calling convention, error handling, gas metering, generics (WIP — not yet implemented) |
| `vm_security_and_correctness.md` | VM-wide invariants: arithmetic safety, type/memory safety, gas metering, boundedness, determinism, cache consistency |

## Commands

```bash
cargo check -p mono-move-runtime         # Quick compile check
cargo test -p mono-move-runtime           # Run all tests
cargo test -p mono-move-runtime -- <name> # Run a specific test
```

## Tests

Integration tests live in `tests/`:

| Test file | What it covers |
|---|---|
| `vec_sum.rs` | Vector operations, push/pop, iteration, arithmetic |
| `struct_test.rs` | Heap-allocated structs, field access, GC with struct roots |
| `enum_test.rs` | Enum allocation, tag dispatch, GC tracing per-variant |
| `ref_test.rs` | Fat pointer references, borrow/read/write through refs |
| `gc_stress.rs` | GC pressure scenarios, survival of live objects across collections |
| `verifier_test.rs` | Verification error detection for malformed programs |

Additional end-to-end tests and benchmarks live in the sibling `../programs` crate.

## Coding Conventions

- All `unsafe` blocks must have a `// SAFETY:` comment documenting the invariants they rely on
- Bare `unwrap()` is banned in non-test code. `expect()` is only permitted when the property is local and can be proven easily. Otherwise, return an error (`bail!` for now; proper VM errors or invariant violations in the future). Tests may use `unwrap()` freely.
- All arithmetic must be checked (no wrapping/overflowing) unless correctness is proven
- Follow the naming conventions in `mono-move-core/src/instruction/` when adding new micro-ops

## Pre-PR Checklist

- [ ] `cargo +nightly fmt -- --check` passes
- [ ] `cargo test -p mono-move-runtime` passes
- [ ] If your change affects design docs (in `../docs/`) or this file, keep them up to date — check for stale descriptions, renamed files/modules, changed layouts, etc.
- [ ] If your change implements something listed in `TODO.md`, update or remove the corresponding entry
- [ ] Consider running tests and benchmarks in the `../programs` crate for broader coverage

# MonoVM Design Status Summary

*Last updated: March 2026*

This document summarizes the current design state of MonoMove (MonoVM), including implemented components, architectural decisions, and open questions from ongoing discussions.

## Overview

MonoMove is a Move Virtual Machine designed for performance and safety through monomorphization. The project is in active development with core runtime components implemented and being integrated.

## Core Design Principles

1. **Stateless VM**: No long-living context; requires external local (per-transaction) and global contexts
2. **Performance Built-in by Design**: Flat memory representation, monomorphized execution, aggregated gas charges
3. **Long-Living Caches**: Code-derived data cached globally and shared between threads
4. **Safety Built-in by Design**: Runtime reference/type checks and metering as first-class citizens

## Implemented Components

### Runtime (PR #18711 - Merged)
- **Interpreter**: Unified call stack with linear memory buffer
- **Micro-ops instruction set**: 24-byte fixed-size instructions (target: 16 bytes)
- **Heap management**: Bump-allocated heap with Cheney's copying GC
- **GC approach**: **Approach B** - Direct Pointers + Partitioned Frames (chosen over 4 alternatives)

### Global Context (PRs #19053, #19054, #19059)
- **Two-phase state machine**: Execution phase (concurrent) / Maintenance phase (exclusive)
- **RAII guards**: `ExecutionGuard` and `MaintenanceGuard`
- **Arena allocation**: Per-worker arenas, epoch-based reclamation
- **Identifier interning**: Module IDs (32-bit), Type IDs (32-bit with reference tagging)

### Executable Cache (PR #19131 - In Review)
- All allocations per executable in arena (enables raw refs)
- Executable is a leaked box for fine-grained memory management
- Public APIs for reading modules/functions

### Gas Instrumentation (PR #19134 - In Review)
- Prototype implemented for micro-ops
- Designed generically to support alternative IR if needed

## Call Semantics (from George's design)

Three categories of function calls with different handling:

| Call Type | Instruction | Behavior |
|-----------|-------------|----------|
| **Module-local non-generic** | `NonNull<Function>` pointer | Direct call, no lookup |
| **Cross-module same package** | `CallExternal { id, name, ty, ptr }` | Pointer patching on first access; `load_module(id)` for Block-STM validation + gas |
| **Cross-module different package** | `CallExternal { id, name, ty }` | Always use `get_function` lookup (no pointer patching due to version concerns) |

**Rationale**: Cross-package calls are rare; framework calls use option 1 with cache reset on upgrades. For Decibel specifically: (i) all framework calls via pointers, (ii) intra-package calls via pointers, (iii) cross-package calls ~<5 per txn use simpler model.

## Memory Management

### Transaction Memory (Two-Level System)
- **Block Level**: Storage cache + transaction memory allocation
- **Transaction Level**: Dedicated region with own allocator

### GC Decision: Approach B
**Why Approach B was chosen:**
- GC rarely triggers in blockchain transactions
- Per-mutation overhead (Approach D's weakness) is the dominant cost
- Over-retention is negligible for short transactions
- Direct pointer access (no indirection overhead)
- Simplest re-compiler requirements

### Value Representation
- **Primitives**: Raw bytes at specified size
- **Vectors**: Rust-like layout `[ref | len | capacity]` → heap storage
- **Structs/Enums**: Heap-allocated (inline optimization TBD)
- **References**: Direct pointers into managed memory

## Open Questions

### 1. Gas Semantics Source (Victor's question)

**Question**: What should gas semantics be derived from?

| Option | Pros | Cons |
|--------|------|------|
| **Micro-ops** | Precise, simple instrumentation | Backward compatibility concerns - any codegen change needs feature-gating |
| **Bytecode** | Isolation of gas semantics | May diverge if compiler transforms (loop unrolling, optimizations) |
| **Middle IR** | Balance of precision and stability | Additional layer to maintain |

*Status: Under discussion*

### 2. Sharing Global Values Across Transactions

Two approaches for making writes visible to subsequent transactions:

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| **Freeze-on-Finish** | Freeze memory space, expose as read-only | Simple, consistent view | Coarse granularity, late conflict detection |
| **Concurrent Data Structures** | Multi-version structure for concurrent access | Immediate conflict detection | Complex, interacts poorly with GC |

*Status: Need mainnet transaction analysis to inform decision*

### 3. Inline Structs Optimization

**Question**: Should small structs be stored inline on the stack?

- **Benefits**: Direct field access, no pointer chasing, fast `memcpy` for copyable types
- **Concerns**: Two code paths to maintain, requires padding to fixed size
- **Needed**: Struct size distribution analysis from real-world Move code

*Status: Deferred optimization*

### 4. Endianness (Victor's concern)

**Issue**: Developers run local nodes on ARM Macs while validators are x64.

**Requirement**: Runtime (and potentially compiler codegen) must be endianness-agnostic to prevent divergence between architectures.

*Status: Needs implementation guidelines*

### 5. Memory Type Safety (Wolfgang's suggestion)

**Proposal**: Statically fix every memory slot's full monomorphic type. With strongly-typed stack frames (no slot reuse):
- Can traverse memory knowing expected type for each location
- Can verify against stored type markers in paranoid mode
- Self-verifying memory for safety

*Status: Design consideration for paranoid mode*

### 6. Deep Copy Implementation

**Question**: Runtime instruction or compiler-emitted copy loops?
- `CopyLoc` on heap-allocated structs needs recursive deep copy
- Can trigger GC mid-copy

*Status: Open design question*

### 7. Native Function GC Safety

**Issue**: Natives holding raw pointers across GC-triggering calls get dangling pointers.

**Considerations**:
- Approach B doesn't solve this (same as Approach A)
- May need handle-based approach (Approach C) if this becomes blocking
- Need to ensure native allocations are included in root set

*Status: Known limitation, may require future work*

## Remaining Work (from TODO.md)

### High Priority
- [ ] Remaining integer widths (u8, u16, u32, u128, u256)
- [ ] Global storage operations (MoveFrom, MoveTo, BorrowGlobal, Exists)
- [ ] Abort/error handling
- [ ] Gas metering integration

### Medium Priority
- [ ] Comparison and boolean ops
- [ ] Deep copy implementation
- [ ] Native function calls with type arguments

### Future Optimizations
- [ ] Hoist interpreter state into locals (register optimization)
- [ ] Super-instructions
- [ ] Threaded dispatch
- [ ] Copy-and-patch JIT

### Deferred
- [ ] Closures (PackClosure, CallClosure)
- [ ] Signed integers

## Related PRs

| PR | Description | Status |
|----|-------------|--------|
| [#18711](https://github.com/aptos-labs/aptos-core/pull/18711) | Runtime prototype: interpreter, GC, benchmarks | Merged |
| [#19053](https://github.com/aptos-labs/aptos-core/pull/19053) | Global context template | Merged |
| [#19054](https://github.com/aptos-labs/aptos-core/pull/19054) | Global arena implementation | Merged |
| [#19059](https://github.com/aptos-labs/aptos-core/pull/19059) | Identifier and module ID interning | Merged |
| [#19131](https://github.com/aptos-labs/aptos-core/pull/19131) | Executable cache placeholder and APIs | In Review |
| [#19134](https://github.com/aptos-labs/aptos-core/pull/19134) | Gas instrumentation prototype | In Review |

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                     Global Execution Context                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌────────────┐ │
│  │   Module    │  │    Type     │  │  Executable │  │   Arena    │ │
│  │  Interning  │  │  Interning  │  │    Cache    │  │    Pool    │ │
│  └─────────────┘  └─────────────┘  └─────────────┘  └────────────┘ │
│                    Two-Phase: Execution ↔ Maintenance                │
└─────────────────────────────────────────────────────────────────────┘
                                    │
                    ┌───────────────┼───────────────┐
                    ▼               ▼               ▼
          ┌─────────────┐  ┌─────────────┐  ┌─────────────┐
          │    Txn 1    │  │    Txn 2    │  │    Txn N    │
          │   Memory    │  │   Memory    │  │   Memory    │
          │  ┌───────┐  │  │  ┌───────┐  │  │  ┌───────┐  │
          │  │ Stack │  │  │  │ Stack │  │  │  │ Stack │  │
          │  ├───────┤  │  │  ├───────┤  │  │  ├───────┤  │
          │  │ Heap  │  │  │  │ Heap  │  │  │  │ Heap  │  │
          │  │(Bump) │  │  │  │(Bump) │  │  │  │(Bump) │  │
          │  └───────┘  │  │  └───────┘  │  │  └───────┘  │
          └─────────────┘  └─────────────┘  └─────────────┘
                    │               │               │
                    └───────────────┼───────────────┘
                                    ▼
                          ┌─────────────────┐
                          │  Block-STM      │
                          │  Coordination   │
                          └─────────────────┘
```

## References

- [Design Document](design.md) - Full architectural specification
- [GC Design Space](gc_design.md) - Detailed GC approach comparison
- [Runtime TODO](../runtime/TODO.md) - Implementation task list

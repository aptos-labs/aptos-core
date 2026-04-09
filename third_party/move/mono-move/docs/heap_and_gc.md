# Heap and Garbage Collection

This document covers the runtime's heap memory management: the two-level memory architecture (block and transaction), the allocator and garbage collector design, global value handling, and memory safety considerations.

For stack memory, see `docs/stack_and_calling_convention.md`. For value layouts on the heap, see `docs/value_representation.md`.

## Overview

During execution, the VM needs to manage memory for values — vectors (dynamically sized), structs (large ones may need to be on the heap), and global values (may need a separate region). This is distinct from memory for types, code and global context, which is covered in the main design doc.

MonoMove uses a **two-level memory management system** organized around the BlockSTM execution model:

1. **Block Level**: Manages shared state across all transactions — the storage cache and allocation of memory subspaces for individual transactions.
2. **Transaction Level**: Each transaction receives a dedicated memory region with its own allocator.

*Note*: We may also want some data to live across blocks in the future. The block-level cache could potentially be retained (rather than discarded) for use in subsequent blocks. TBD.

This separation enables bounding total memory usage and supports parallel transaction execution with shared access to global values.

## Block Memory Manager

The block memory manager owns two key responsibilities:

1. **Storage Cache**: Caches resources loaded from storage, shared across all transactions within the block. The cached version is the resource state at the beginning of the block. This avoids redundant storage reads and BCS deserialization for frequently accessed resources.
2. **Transaction Memory Allocation**: Hands out memory subspaces to individual transactions. Each transaction receives a dedicated region that is then managed by its own transaction-level memory manager.

### Sharing Global Values Across Transactions

While temporary values created during execution (local variables, intermediate results, newly allocated structs and vectors) are exclusively local to a transaction, writes to global values need to be made visible to subsequent transactions.

Two approaches can enable this sharing:

**Option 1: Freeze-on-Finish**

After a transaction finishes, freeze its memory space and expose it as read-only to subsequent transactions.

- *Pros*: Simple to implement; less error-prone; provides a consistent view to readers.
- *Cons*: Coarse granularity; read-write conflicts are detected late.

**Option 2: Concurrent Data Structures**

Use a multi-version data structure to provide concurrent shared access to global values.

- *Pros*: Read-write conflicts are detected immediately.
- *Cons*: More complex — requires a separate shared mutable subspace at the block level (similar to the storage cache, but mutable); may expose inconsistent views to readers; may require copying data between memory regions; interacts poorly with GC-managed memory (references held by block-level structures must be updated when GC moves memory, unlike freeze-on-finish where frozen regions are not subject to GC).

*TODO*: Analyze mainnet transaction history to better understand real-world read/write patterns and inform the choice between these approaches.

### Per-Block Memory Limits

A per-block memory limit sets the upper bound of memory a node may use for values at any given time. This is important for resource planning and preventing out-of-memory conditions.

**Why This Matters**

Current node configuration uses a few dozen transactions per block to ensure low latency. However, throughput-focused benchmarks may run hundreds or thousands of transactions per block.

Consider a default maximum of 10 MB per transaction (for values only — this excludes code and other global context data):

- 1,000 transactions × 10 MB = 10 GB baseline memory usage.

This is already high, and several factors can further increase memory consumption:

- **Memory freezing + re-execution**: A transaction under re-execution may require two memory spaces — one frozen (finished state) and one active (speculative execution).
- **Garbage collection**: A copying GC requires an additional "to-space", effectively doubling the memory footprint during collection.

In the worst case (all factors combined), peak memory usage could reach 30–40 GB. Typical usage would be significantly lower, but we need to plan for adversarial conditions.

Such limits may be acceptable today, but pose concerns for future scalability. As VM execution speed improves, we may want to include more transactions per block without compromising latency significantly.

**Mitigations**

1. **Conservative initial allocation**: Start with a small allocation per transaction and grow as needed (e.g., 1 MB -> 4 MB -> 10 MB). The idea is that typical high-frequency transactions fit comfortably in the default allocation. Transactions with higher memory demands can request more, but may require pre-declaration or incur significant memory fees.
2. **Hard per-block memory limit**: Enforce a block-level cap and cut off remaining transactions if approaching the limit. We already have a per-block gas limit that functions in a similar way.
3. **Compact-on-freeze**: When freezing a transaction's memory space, retain only the global value writes and discard temporary values. This significantly reduces the footprint of frozen regions for typical transactions, but has limited effect on malicious transactions that maximize global value writes. Note: we may need to do this anyway for write-set generation. This scan may also be required for gas metering purposes.

## Transaction Memory Manager

Each transaction gets its own memory manager for its subspace. The design has two major goals:

1. **Blazingly fast allocation**: Allocation is on the hot path and must be minimal overhead.
2. **Bulk deallocation**: Reclaim memory in batches rather than per-object — both at transaction end (discarding temporaries) and during GC runs (if needed).

**Chosen approach: Bump allocation with Cheney's copying GC using direct pointers.**

The runtime uses a bump allocator for fast allocation. When the heap is full, a copying garbage collector (Cheney's algorithm) runs: it walks the call stack using per-function `frame_layout` (and per-safe-point `safe_point_layouts`) to find roots, then does a breadth-first copy of all reachable objects into a fresh to-space, fixing all pointers in place. This combines the speed of bump allocation with the ability to reclaim memory mid-transaction.

### Handling Global Values

Beyond temporary values, the transaction memory manager also handles operations on global resources (e.g., `move_from`, `move_to`, `borrow_global`).

**Reading global values**: When a transaction reads a global resource, the value may come from:

- The block-level storage cache (base value at block start), or
- Another transaction's writes or modifications (if using concurrent sharing).

The transaction tracks what it has read for later validation (BlockSTM needs to detect read-write conflicts). *Possible optimization*: track not just values but also read constraints (e.g., "checked that resource exists" vs "read the actual contents") for finer-grained conflict detection.

An open question is whether reads require copying the value into local memory, or whether the transaction can reference the source directly. This depends on the memory manager design and the sharing approach.

**Modifying global values**: When a transaction modifies a global resource, it performs a copy-on-write (CoW) into its own memory subspace. This keeps all modifications isolated, which:

- Enables rollback if the transaction aborts or needs re-execution.
- Allows other transactions to reference these modifications (the local memory holds the authoritative version of the transaction's writes).

In BlockSTM, each transaction's modifications integrate with `MVHashMap` — the transaction's local memory effectively becomes a slot in the multi-version structure, replacing the current `Arc<Value>` approach.

Open questions:

- **CoW timing**: Should CoW happen eagerly on `borrow_global_mut`, or lazily on actual write? Lazy CoW avoids unnecessary copies but requires tracking borrowed references to detect when a write occurs.
- **GC interaction**: If GC runs mid-transaction, references to the transaction's modified values (held by block-level structures for sharing) must be updated to reflect moved memory locations. This is likely not a concern if using freeze-on-finish, since frozen memory is not subject to additional GC runs.

## Memory Safety

### Reference Validity

Move's bytecode verifier provides static guarantees about reference safety (no dangling references, proper borrow semantics). However, runtime checks may be valuable as defense-in-depth against verifier bugs or interpreter errors.

Potential runtime checks to consider:

- **Epoch/generation counters**: Each allocated memory chunk tracks a generation number. References include the expected generation; access fails if they don't match. This could catch use-after-free.
- **Bounds checking**: Reference accesses validate indices/offsets are within bounds.

The cost of these checks would need to be weighed against the safety benefit. They could potentially be disabled in production once the implementation is mature.

### Memory Region Isolation

Transactions should only access:

- Their own local memory region.
- The block-level storage cache (read-only base values).
- Other transactions' frozen memory (if using freeze-on-finish sharing).

Violations would indicate serious bugs in the interpreter.

### GC Safety

All pointers must be updated after memory moves during GC. Missing a pointer leads to dangling references. The runtime's safety model depends on three invariants (see also `runtime/AGENTS.md`):

1. **Frame metadata integrity** — saved `fp`/`pc`/`func_ptr` are written only by call/return, never by user micro-ops.
2. **Pointer-slot accuracy** — `Function::frame_layout` (and, at safe points, the matching `safe_point_layouts` entry) together exactly match slots that hold live heap pointers.
3. **Object header integrity** — `descriptor_id` and `size` are set by the allocator and never overwritten by user code.

### Type Safety

With flat memory representation, values are raw bytes interpreted according to type information. The runtime should ensure values are accessed with the correct type.

TBD: Anything the memory manager can do to help mitigate this risk?

---

## GC Design Space

Four approaches were considered for the runtime's memory management. All assume a bump-allocated heap with copying collection (or equivalent). **The implemented design is a hybrid of Approach A and Approach B** — see their Status sections and the Recommendation section at the end for the rationale.

### Approach A: Direct Pointers + Stack Maps at Safe Points

Frame slots hold raw heap pointers. The re-compiler emits a stack map at every GC safe point (allocation sites, call return sites) listing which frame offsets hold live heap pointers. Strictly only safe points need maps, though the liveness analysis the re-compiler runs will typically produce per-PC info anyway. GC scans these maps to find roots, then does a transitive trace (Cheney's copying collector) using object descriptors to find internal heap pointers.

**Pros:**
- No overhead on heap access — direct pointer, single load
- No over-retention — only truly live pointers are roots
- No per-mutation bookkeeping

**Cons:**
- Re-compiler must compute liveness at safe points and emit stack maps (the "maybe alive" problem at control-flow merge points, null-initialization discipline, etc.)
- GC must rewrite every pointer on the stack and inside heap objects when relocating
- Fat pointer bases must be rewritten during GC
- Natives holding raw pointers across GC-triggering calls get dangling pointers

**Status:** Partially implemented as part of the hybrid A+B design (see Approach B's Status).

### Approach B: Direct Pointers + Partitioned Frames

Same as Approach A, but instead of safe-point stack maps the re-compiler marks which frame slots hold pointers. This can be done two ways:
- Contiguous partitioning: pointer region (`fp+0..fp+K`), scalar region (`fp+K..end`)
- Slot list: a per-function `Vec<u32>` of frame offsets that hold pointers

Either way, GC scans the marked pointer slots of every frame — no liveness tracking.

Stale pointers (logically dead but not yet overwritten) in the pointer region cause over-retention: the pointed-to object stays alive until the slot is reused or the frame pops. For short-lived blockchain transactions this is negligible.

**Pros:**
- Same direct access performance as Approach A
- Dramatically simpler re-compiler — just mark pointer slots in the frame layout, no stack maps

**Cons:**
- Over-retention of stale pointers (minor for short transactions)
- Still must rewrite all pointers during GC relocation (stack + internal)
- Fat pointer base rewriting still needed
- Native GC safety still unsolved (same dangling pointer problem)
- Still needs object descriptors for tracing internal heap refs

**Status:** Implemented as a hybrid of Approach A and B. Each `Function` declares two levels of pointer-slot information:

1. `frame_layout: FrameLayoutInfo` — frame offsets that always hold heap pointers, scanned at every PC (Approach B).
2. `safe_point_layouts: SortedSafePointEntries` — additional per-safe-point pointer offsets, scanned only when the frame's PC matches a safe-point entry (Approach A).

At any given safe point, the GC scans the union of both. When `zero_frame` is true, the runtime zeroes the region beyond args (`args_size..extended_frame_size`) at `CallFunc` time so pointer slots start as null. Safe points are allocating instructions (at their own PC) and call return sites (`call_pc + 1`).

This hybrid keeps the common case simple — stable pointer slots use `frame_layout` with no per-PC overhead — while supporting slots that change type across call boundaries (e.g., shared arg/return regions, different callee arg layouts). The specializer is free to use either mechanism: `frame_layout` for slots with a fixed pointer/scalar designation, `safe_point_layouts` for slots whose type varies by PC.

### Approach C: Handle Table + Partitioned Frames

Heap pointers are replaced by handle IDs — indices into an object handle table that stores the actual heap addresses. Frame layout is partitioned (handle region vs scalar region) as in Approach B, so GC knows which slots are handles without stack maps.

GC scans handle regions for root handles, transitively traces via descriptors (same as before), but object relocation only updates handle table entries — not every pointer on the stack and inside objects.

**Pros:**
- No stack maps (partitioned frames)
- GC relocation is cheap — update handle table entries, not every pointer location
- Fat pointers are `(handle_id, offset)` — stable across GC, no base rewriting
- Natives hold handle IDs that remain valid across GC — solves native GC safety
- References (borrows) work naturally — hold a handle ID, dereference through table

**Cons:**
- Every heap access adds an indirection (handle table lookup + data load)
- Handle table itself competes for cache (though likely fits in L1 for short transactions)
- Need free-list or similar for recycling handle table slots
- Over-retention of stale handles in the pointer region (same as Approach B)
- Still needs object descriptors for tracing internal heap refs

**Status:** Not implemented.

### Approach D: Handle Table + Ownership Tree (Parent Pointers)

Builds on Approach C. Each handle table entry stores an additional parent field: the handle ID of the owning container (or a "stack root" sentinel). This forms a tree that mirrors Move's linear ownership model — every value has exactly one owner.

GC does not scan frames or trace the object graph. Instead:
- The re-compiler emits `Drop` when a stack root dies → O(1), nulls the parent
- GC walks the handle table, traces each handle's parent chain upward
- If the chain reaches a live stack root → handle is alive
- If the chain reaches a null/dead parent → handle is unreachable, free it
- Results are cached in a parallel array so each handle's liveness is resolved at most once per collection pass

The re-compiler must notify the runtime of every ownership change:
- `Drop`: stack root dies → null the handle's parent
- Store handle into container (`ObjStore`, `VecPushBack`, `VecStoreElem`): set child's parent to the container's handle
- Move handle out of container (`ObjLoad`, `VecPopBack`, `VecLoadElem`): re-parent the extracted handle to the receiving stack slot
- `WriteRef` of a handle-typed value: ownership transfer through a mutable reference — old child's parent cleared, new child's parent set
- `ReadRef` of a handle-typed value: move/copy out, needs re-parenting
- `Mov`/`Mov8` of a handle between stack slots: re-parent to new slot

References (borrows) do NOT participate in the parent tree. They hold handle IDs but are non-owning. Move's borrow checker guarantees the owner outlives all borrows, so the runtime trusts this invariant.

**GC algorithm (from earlier PoC):**

The collector is a compacting bump allocator. On collection:

1. Walk the handle table and partition into active/inactive sets. For each handle, walk its parent chain upward until reaching `Root` (alive) or `None` (dead). Cache results in a parallel array so each handle's liveness is resolved at most once — amortized O(1) per handle, iterative (no recursion).
2. Recycle inactive handle slots into a free list for reuse by future allocations.
3. Allocate a fresh memory region. Copy each active handle's data contiguously into the new region and update `handle.mem_ptr`. Because all access goes through the handle table, nothing else needs rewriting — no stack fixups, no internal pointer updates.
4. Free the old memory region.

Reallocation (e.g. vector grow) works the same way: bump-allocate a larger chunk, copy contents, update the single handle entry. The old chunk becomes dead space reclaimed at next collection.

**Pros:**
- No stack scanning, no stack maps, no frame partitioning requirements
- No object descriptors needed for GC tracing — parent tree replaces descriptor-driven graph traversal
- No transitive object graph traversal during GC
- O(1) Drop, deferred lazy collection
- Native GC safety (handles are stable)

**Cons:**
- Every handle mutation has overhead (parent pointer update)
- Handle-aware instruction variants needed — re-compiler must distinguish handle-typed operations from scalar operations everywhere
- Re-compiler must emit `Drop` at every value death point — miss one and you leak
- GC walks all handles (live + dead) to partition them, while tracing collectors (A/B/C) only visit reachable objects

**Status:** Not implemented in the current PoC. An earlier standalone PoC demonstrated the core algorithm: handle table with parent pointers, iterative parent-chain liveness check with caching, compacting collector that only updates handle entries.

### Comparison

| | **A: Direct + safe-point maps** | **B: Direct + partitioned** | **C: Handles + partitioned** | **D: Handles + ownership tree** |
|---|---|---|---|---|
| Heap access cost | Direct (1 load) | Direct (1 load) | Indirect (2 loads) | Indirect (2 loads) |
| GC root discovery | Per-PC stack maps | Scan pointer region | Scan pointer region | Parent chain (no scanning) |
| GC graph traversal | Full transitive trace | Same | Same | Parent chain walk (iterative) |
| GC relocation cost | Rewrite all pointers | Same | Update handle table | Update handle table |
| Drop | N/A (GC reclaims) | N/A (GC reclaims) | N/A (GC reclaims) | O(1) null parent |
| Over-retention | None | Stale pointers | Stale handles | None (ownership is precise) |
| Fat pointer GC | Rewrite base | Same | Stable | Stable |
| Native GC safety | Unsafe | Unsafe | Safe | Safe |
| Per-mutation overhead | None | None | None | Parent update every handle op |
| Re-compiler burden | Heavy | Light | Light | Medium |
| Descriptor needs | Yes | Yes | Yes | No |
| GC complexity | Medium (Cheney's + stack maps) | Medium (Cheney's) | Medium (mark-sweep + handle table) | Low (parent chain walk) |
| Instruction set complexity | Low | Low | Low | Medium (handle-aware variants) |

### Recommendation

For most blockchain transactions, the heap fits comfortably in the pre-allocated region and GC never triggers. Collection is a safety net, not a steady-state mechanism. We also do not care about pause-the-world latency. This shifts the trade-offs:

- **GC algorithm cost is nearly irrelevant.** Whether collection does a parent-chain walk or a full graph traversal doesn't matter if it almost never runs. Advantages in GC complexity (D's main strength) carry little weight.
- **Per-mutation overhead is the dominant cost.** Every parent-pointer update in Approach D is paid on the hot path regardless of whether GC fires. For an operation that rarely benefits from the bookkeeping, this is pure overhead.
- **Over-retention is a non-issue.** If the collector rarely runs, stale pointers in B/C just sit until the transaction ends — same as if they were collected.

Under these assumptions, the **A+B hybrid** is the clear winner: zero hot-path overhead, and the re-compiler burden is minimal — only slots whose pointer status changes across call boundaries need safe-point entries, while stable pointer slots use the simpler `frame_layout`. The downsides (over-retention, pointer rewriting during GC) are either limited, or costs you rarely pay.

Pure **Approach A** (stack maps everywhere) burdens the re-compiler with liveness analysis at every safe point for no benefit in the common case. Pure **Approach B** (no per-PC info) cannot correctly describe slots that change type across call boundaries. The hybrid gets the best of both.

**Approach C** is worth considering only if native GC safety becomes a real obstacle. Otherwise it pays the handle indirection cost on every access for GC benefits that rarely materialize.

**Approach D** is the most elegant design and the simplest GC, but the per-mutation parent updates are the wrong trade-off when GC almost never fires. It optimizes the rare case (collection) at the expense of the common case (every handle operation).

### Why GC at all? Bump-only vs. bump + collect

We've gone back and forth on this. If GC rarely fires, why have one at all? Just bump-allocate, run the transaction, throw away the arena. There's a hard limit on how much memory a transaction can allocate. Simple, fast, and the math works out for most transactions.

The thing that keeps bugging us though is that under bump-only, all allocations are memory leaks. The case that won't go away is a loop that allocates transient objects — each iteration's results get consumed, but the heap keeps growing because nothing is ever reclaimed. The program is doing legitimate work, well within the memory limit in terms of live data, but dies because we can't reuse dead memory.

The deeper concern is what we're committing to. If we ship bump-only, contract patterns and memory limits get built around that ceiling. Adding GC later means retrofitting pointer relocation into a runtime that wasn't designed for it and introducing new failure modes. That's a painful retrofit we'd rather not sign up for.

In the end we didn't have to fight about it. We already have a working copying collector, object descriptors, and pointer rewriting — the engineering cost is already paid. With Approach B, the common path *is* bump allocation — zero overhead. The collector just sits there as a safety net for when the heap fills, which is rare. We're not paying for GC. We already have it. It's cheap insurance that buys us the ability to handle allocation-heavy workloads without hitting a hard wall — and the freedom to not worry about what future workloads look like.

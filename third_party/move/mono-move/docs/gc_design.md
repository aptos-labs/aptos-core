# GC Design Space

Four approaches were considered for the runtime's memory management.
All assume a bump-allocated heap with copying collection (or equivalent).
**Approach B was chosen and is implemented** — see its Status section and the
Recommendation section at the end for the rationale.

## Approach A: Direct Pointers + Stack Maps at Safe Points

Frame slots hold raw heap pointers. The re-compiler emits a stack map at every
GC safe point (allocation sites, call return sites) listing which frame offsets
hold live heap pointers. Strictly only safe points need maps, though the
liveness analysis the re-compiler runs will typically produce per-PC info
anyway. GC scans these maps to find roots, then does a
transitive trace (Cheney's copying collector) using object descriptors to find
internal heap pointers.

**Pros:**
- No overhead on heap access — direct pointer, single load
- No over-retention — only truly live pointers are roots
- No per-mutation bookkeeping

**Cons:**
- Re-compiler must compute liveness at safe points and emit stack maps (the
  "maybe alive" problem at control-flow merge points, null-initialization
  discipline, etc.)
- GC must rewrite every pointer on the stack and inside heap objects when relocating
- Fat pointer bases must be rewritten during GC
- Natives holding raw pointers across GC-triggering calls get dangling pointers

**Status:** Superseded by Approach B.

## Approach B: Direct Pointers + Partitioned Frames

Same as Approach A, but instead of safe-point stack maps the re-compiler marks which
frame slots hold pointers. This can be done two ways:
- Contiguous partitioning: pointer region (`fp+0..fp+K`), scalar region (`fp+K..end`)
- Slot list: a per-function `Vec<u32>` of frame offsets that hold pointers

Either way, GC scans the marked pointer slots of every frame — no liveness
tracking.

Stale pointers (logically dead but not yet overwritten) in the pointer region
cause over-retention: the pointed-to object stays alive until the slot is reused
or the frame pops. For short-lived blockchain transactions this is negligible.

**Pros:**
- Same direct access performance as Approach A
- Dramatically simpler re-compiler — just mark pointer slots in the frame layout, no stack maps

**Cons:**
- Over-retention of stale pointers (minor for short transactions)
- Still must rewrite all pointers during GC relocation (stack + internal)
- Fat pointer base rewriting still needed
- Native GC safety still unsolved (same dangling pointer problem)
- Still needs object descriptors for tracing internal heap refs

**Status:** Implemented. Each `Function` declares `pointer_slots: Vec<u32>`
(frame offsets that may hold heap pointers) and `args_size: usize`. The runtime
zeroes the callee's local area (`args_size..data_size`) at `CallFunc` time so
pointer slots start as null. GC scans `pointer_slots` for every live frame —
no per-PC stack maps needed. If higher precision is needed in the future
(e.g. to reduce over-retention), we can switch to Approach A by adding per-PC
stack maps to the re-compiler — the core GC logic (Cheney's copying collector,
object descriptors, pointer rewriting) stays the same.

## Approach C: Handle Table + Partitioned Frames

Heap pointers are replaced by handle IDs — indices into an object handle table
that stores the actual heap addresses. Frame layout is partitioned (handle region
vs scalar region) as in Approach B, so GC knows which slots are handles without
stack maps.

GC scans handle regions for root handles, transitively traces via descriptors
(same as before), but object relocation only updates handle table entries — not
every pointer on the stack and inside objects.

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

## Approach D: Handle Table + Ownership Tree (Parent Pointers)

Builds on Approach C. Each handle table entry stores an additional parent field:
the handle ID of the owning container (or a "stack root" sentinel). This forms a
tree that mirrors Move's linear ownership model — every value has exactly one owner.

GC does not scan frames or trace the object graph. Instead:
- The re-compiler emits `Drop` when a stack root dies → O(1), nulls the parent
- GC walks the handle table, traces each handle's parent chain upward
- If the chain reaches a live stack root → handle is alive
- If the chain reaches a null/dead parent → handle is unreachable, free it
- Results are cached in a parallel array so each handle's liveness is resolved
  at most once per collection pass

The re-compiler must notify the runtime of every ownership change:
- `Drop`: stack root dies → null the handle's parent
- Store handle into container (`ObjStore`, `VecPushBack`, `VecStoreElem`):
  set child's parent to the container's handle
- Move handle out of container (`ObjLoad`, `VecPopBack`, `VecLoadElem`):
  re-parent the extracted handle to the receiving stack slot
- `WriteRef` of a handle-typed value: ownership transfer through a mutable
  reference — old child's parent cleared, new child's parent set
- `ReadRef` of a handle-typed value: move/copy out, needs re-parenting
- `Mov`/`Mov8` of a handle between stack slots: re-parent to new slot

References (borrows) do NOT participate in the parent tree. They hold handle IDs
but are non-owning. Move's borrow checker guarantees the owner outlives all
borrows, so the runtime trusts this invariant.

**GC algorithm (from earlier PoC):**

The collector is a compacting bump allocator. On collection:

1. Walk the handle table and partition into active/inactive sets. For each
   handle, walk its parent chain upward until reaching `Root` (alive) or
   `None` (dead). Cache results in a parallel array so each handle's liveness
   is resolved at most once — amortized O(1) per handle, iterative (no
   recursion).
2. Recycle inactive handle slots into a free list for reuse by future
   allocations.
3. Allocate a fresh memory region. Copy each active handle's data
   contiguously into the new region and update `handle.mem_ptr`. Because all
   access goes through the handle table, nothing else needs rewriting —
   no stack fixups, no internal pointer updates.
4. Free the old memory region.

Reallocation (e.g. vector grow) works the same way: bump-allocate a larger
chunk, copy contents, update the single handle entry. The old chunk becomes
dead space reclaimed at next collection.

**Pros:**
- No stack scanning, no stack maps, no frame partitioning requirements
- No object descriptors needed for GC tracing — parent tree replaces
  descriptor-driven graph traversal
- No transitive object graph traversal during GC
- O(1) Drop, deferred lazy collection
- Native GC safety (handles are stable)

**Cons:**
- Every handle mutation has overhead (parent pointer update)
- Handle-aware instruction variants needed — re-compiler must distinguish
  handle-typed operations from scalar operations everywhere
- Re-compiler must emit `Drop` at every value death point — miss one and you
  leak
- GC walks all handles (live + dead) to partition them, while tracing collectors
  (A/B/C) only visit reachable objects

**Status:** Not implemented in the current PoC. An earlier standalone PoC
demonstrated the core algorithm: handle table with parent pointers,
iterative parent-chain liveness check with caching, compacting
collector that only updates handle entries.

## Comparison

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

## Recommendation

For most blockchain transactions, the heap fits comfortably in the pre-allocated
region and GC never triggers. Collection is a safety net, not a steady-state
mechanism. We also do not care about pause-the-world latency. This shifts the
trade-offs:

- **GC algorithm cost is nearly irrelevant.** Whether collection does a
  parent-chain walk or a full graph traversal doesn't matter if it almost never
  runs. Advantages in GC complexity (D's main strength) carry little weight.
- **Per-mutation overhead is the dominant cost.** Every parent-pointer update in
  Approach D is paid on the hot path regardless of whether GC fires. For an
  operation that rarely benefits from the bookkeeping, this is pure overhead.
- **Over-retention is a non-issue.** If the collector rarely runs, stale pointers
  in B/C just sit until the transaction ends — same as if they were collected.

Under these assumptions, **Approach B** is the clear winner: zero hot-path
overhead, simplest re-compiler, and its downsides (over-retention, pointer
rewriting during GC) are either limited, or costs you rarely pay.

**Approach A** has the same runtime performance as B but burdens the re-compiler
with stack map generation for no benefit. B strictly dominates A.

**Approach C** is worth considering only if native GC safety becomes a real
obstacle. Otherwise it pays the handle indirection cost on every access for
GC benefits that rarely materialize.

**Approach D** is the most elegant design and the simplest GC, but the
per-mutation parent updates are the wrong trade-off when GC almost never fires.
It optimizes the rare case (collection) at the expense of the common case
(every handle operation).

## Why GC at all? Bump-only vs. bump + collect

We've gone back and forth on this. If GC rarely fires, why have one at all?
Just bump-allocate, run the transaction, throw away the arena. There's a hard
limit on how much memory a transaction can allocate. Simple, fast, and the
math works out for most transactions.

The thing that keeps bugging us though is that under bump-only, all
allocations are memory leaks. The case that won't go away is a loop
that allocates transient objects — each iteration's results get consumed,
but the heap keeps growing because nothing is ever reclaimed. The program
is doing legitimate work, well within the memory limit in terms of live
data, but dies because we can't reuse dead memory.

The deeper concern is what we're committing to. If we ship bump-only, contract
patterns and memory limits get built around that ceiling. Adding GC later means
retrofitting pointer relocation into a runtime that wasn't designed for it and
introducing new failure modes. That's a painful retrofit we'd rather not sign
up for.

In the end we didn't have to fight about it. We already have a working copying
collector, object descriptors, and pointer rewriting — the engineering cost is
already paid. With Approach B, the common path *is* bump allocation — zero
overhead. The collector just sits there as a safety net for when the heap
fills, which is rare. We're not paying for GC. We already have it.
It's cheap insurance that buys us the ability to handle allocation-heavy
workloads without hitting a hard wall — and the freedom to not worry about
what future workloads look like.

# Global Storage

This document describes how MonoMove manages per-transaction access to global state under Block-STM: how a transaction records its reads, accumulates its dirty writes, publishes them for other transactions, rolls back to a prior checkpoint, and bounds the memory footprint of a published incarnation.

It does not cover Block-STM scheduling, gas, interpreter state save/restore, native functions reaching into storage, or the mechanism by which superseded incarnation regions are actually freed. See the Out of Scope and Future Work sections.

Related docs:

- `design.md` — the VM's global execution context and long-living caches.
- `heap_and_gc.md` — the bump allocator and Cheney's copying collector used as a primitive here.
- `value_representation.md` — flat-memory layout of values stored in these regions.

## Constraints

The design targets the following requirements:

- Every transaction execution records the reads it performs against global state. The read-set is needed for Block-STM validation and for local caching so repeated reads do not re-enter the shared concurrent structure.
- Every transaction execution accumulates its dirty writes into a working-set. The working-set is publishable to Block-STM so that subsequent transactions can observe it.
- Cross-transaction reads should approach zero-copy. The interface must admit pointer-level sharing into another transaction's region as a future extension, even if the first implementation copies.
- The VM can stop at a point with state accumulated, and the driver can choose to continue or roll back to a prior checkpoint. History is linear: once a rollback discards a later state, that state is gone for good.
- Values keep the flat-memory representation and the bump-allocated heap defined in `heap_and_gc.md` and `value_representation.md`.

## Architecture

A transaction execution is a single **incarnation**. Each incarnation owns:

- A `MemoryRegion` — a bump-allocated arena for all flat-memory allocations the incarnation produces (temporaries, CoW'd resources, newly created resources).
- A **working map** — a per-incarnation map keyed by resource key. Every read and every write the incarnation performs lands in this map.
- A **journal** — a linear log of map-level undo records, used to roll back to a prior checkpoint.

Block-STM's `MVHashMap` indexes resources across transactions. Each entry references the owning incarnation's region by raw pointer. Supersede — the moment a new incarnation of the same transaction replaces a previous one — rewrites the `MVHashMap` entries for that transaction; the previous region's lifetime is then governed by a reclamation mechanism not specified here.

Two distinct callers use the Cheney collector that `heap_and_gc.md` describes: a **heap-full GC** during execution, and a **compaction pass** at quiescent points or at publication. Same mechanics, different root sets.

## A. Per-Incarnation Region and Publication

Each incarnation is self-contained. Re-executing a transaction produces a new incarnation with a fresh `MemoryRegion` and a fresh working map and journal. The previous incarnation's state is not reused.

### MVHashMap entry

An entry in the shared `MVHashMap` chain for a given resource key looks like:

```
(txn_id, incarnation, raw_ptr_into_incarnation_region)
```

The raw pointer is safe because the supersede protocol below ensures that every entry referencing a region is either overwritten or removed before that region is reclaimed. Readers never retain a pointer into another incarnation's region beyond the `MVHashMap` access that materializes the value locally (see section C).

### Supersede protocol

When incarnation `N+1` of transaction `T` publishes its working-set:

1. For every key in `W_{N+1}`, overwrite the `MVHashMap` entry at that key for transaction `T` with the new pointer.
2. For every key in `W_N \ W_{N+1}` — keys the previous incarnation wrote that this one does not — remove the entry at that key for transaction `T`.
3. Retire incarnation `N`'s region. Reclamation is deferred to a mechanism out of scope for this document.

After step 2, no `MVHashMap` entry references incarnation `N`'s region. Because readers do not retain pointers into source regions beyond a single access (section C), correctness relies only on the reclamation mechanism not freeing the region while any worker could still be mid-access into it.

### What publication actually publishes

Publication exposes two things to Block-STM:

- The set of `MVHashMap` entries updated as described above — one entry per key the incarnation wrote.
- A compacted `MemoryRegion` containing only the live data those entries reference. See the Compaction section.

The working map itself is not visible to other transactions. It is a per-incarnation structure used during execution and during validation. The published view is the set of `MVHashMap` entries, and those entries point into the compacted region.

## B. Working Map

The per-incarnation working map is a single unified structure keyed by resource key. Both reads and writes land in this map. One lookup on the hot path serves the purposes that would otherwise require consulting a separate read-set and write-set.

### Entry shape

```
Entry {
    state:            EntryState,
    data_ptr:         *mut u8,   // into block cache, read copy, or bump region
    version_metadata: VersionMetadata,
}
```

`VersionMetadata` carries whatever Block-STM needs to validate this observation against the shared structure. Its concrete form is orthogonal to this design and addressed in the Validation Metadata section.

### Entry state

The state discriminator covers the ways a transaction can interact with a resource:

- `Read` — the transaction only observed the value. `data_ptr` points at the observed data. Under the v1 read interface this is a local copy in the incarnation's region (see section C).
- `CoWed` — the transaction borrowed the resource mutably. `data_ptr` points at a copy in the bump region; the copy is authoritative for subsequent access.
- `Created` — the transaction produced a new resource via `move_to`. `data_ptr` points at freshly allocated data in the bump region.
- `Removed` — the transaction destroyed the resource via `move_from`. `data_ptr` is not used.
- `ExistenceCheckedOnly` — the transaction observed only whether the resource exists. `data_ptr` is not used; validation cares about the existence bit, not the value.

State transitions during execution correspond to map-level events. Mutating a field of an already-`CoWed` resource is a memory-level event and does not transition the entry.

## C. Cross-Transaction Read Interface

The interpreter does not receive a raw pointer back from the storage layer. It receives an opaque **resource view**. The view abstracts over the two possible representations:

- A pointer into local memory that the interpreter may read freely.
- A pointer into another incarnation's region accompanied by a lifetime witness that keeps that region alive for the duration the interpreter holds the view.

### v1 behavior

Always materialize locally. When the storage layer fetches a resource from `MVHashMap`, it copies the value into the incarnation's own region and returns a view over the local copy. The reader never retains a pointer into another region beyond the single access that performed the copy.

This makes the incarnation region ownership story simple: an incarnation's region is referenced only by its own working map and by the `MVHashMap` entries that point into it. Nothing else.

### v2 extension

When we decide to eliminate the copy in common cases, the view gains a second internal variant that carries a lifetime witness (the form of witness is not fixed here) and points directly into the source region. The storage layer chooses which representation to return per access. No other structure in this design — `MVHashMap` entries, the working map, the supersede protocol, the compaction pass — needs to change.

## D. Checkpoints and Rollback

The driver can pause a transaction at a boundary, take a checkpoint, continue, and later decide to roll back to that checkpoint or to commit past it. The mechanism supports nesting: checkpoints form a stack.

### Mechanism

The working map always reflects the current state. A separate **journal** records one undo entry per map-level write. Reads are not journaled. Memory-level writes inside an already-`CoWed` block are not journaled.

```
UndoRecord {
    key:        ResourceKey,
    prev_entry: Option<Entry>,  // None means "not present before this write"
}

Journal = Vec<UndoRecord>

Checkpoint {
    journal_len: usize,  // index into the journal
    bump_mark:   *mut u8, // bump pointer at checkpoint time
}
```

A stack of checkpoints supports nested sub-transactions.

### Invariant for rollback correctness

The first write to a given resource key since the topmost checkpoint must allocate a new bump block. Subsequent writes to the same key within the same checkpoint epoch may mutate in place.

This invariant guarantees that post-checkpoint memory never overlaps with pre-checkpoint memory that a prior state might reference. Resetting the bump pointer to `bump_mark` therefore reclaims exactly the allocations that need to disappear on rollback, and no others.

`borrow_global_mut` enforces the invariant by checking whether the key's current entry was CoW'd in a lower checkpoint epoch. If so, it allocates a fresh bump block and CoWs again; otherwise it hands back the existing block for in-place mutation.

### Operations

- `push_checkpoint()` — push `(journal.len(), bump.mark())` onto the checkpoint stack.
- `commit()` — pop the top checkpoint. The journal is left untouched; the outer scope inherits those entries.
- `rollback()` — pop the top checkpoint. Walk the journal back to the saved length, applying each `prev_entry` into the map. Reset the bump pointer to the saved mark (subject to the GC caveat below).

### Worked trace

Starting at checkpoint `C1` with two existing entries:

```
map:     { R1: (Read,  ptr_cache_R1, v0),  R2: (CoWed, ptr_a, v1) }
journal: []
```

A `borrow_global_mut(R1)` triggers CoW since R1's current entry predates the checkpoint. A new block `ptr_b` is allocated; R1's entry becomes `(CoWed, ptr_b, v0)`. One undo record is appended:

```
map:     { R1: (CoWed, ptr_b, v0),  R2: (CoWed, ptr_a, v1) }
journal: [ (R1, prev=(Read, ptr_cache_R1, v0)) ]
```

Mutating a field of R1 through the borrow writes to `*ptr_b + offset`. The map and journal are unchanged.

A `move_to(R3, v)` inserts R3 into the map and appends an undo record for the transition from not-present:

```
map:     { R1: (CoWed, ptr_b, v0), R2: (CoWed, ptr_a, v1), R3: (Created, ptr_c, None) }
journal: [ (R1, prev=(Read, ptr_cache_R1, v0)),
           (R3, prev=None) ]
```

Rolling back to C1 walks the journal from the tail. Each undo record restores the map entry it captured:

1. Pop `(R3, prev=None)` → remove R3 from the map.
2. Pop `(R1, prev=(Read, ptr_cache_R1, v0))` → restore R1.

Reset the bump pointer to C1's mark. `ptr_b` and `ptr_c` are reclaimed in a single pointer assignment. The map is bit-identical to its C1 snapshot.

### Why reads are not journaled

Across a rollback the read-set must be conservative: every key the transaction ever observed must remain in the final read-set so Block-STM validates it. Dropping a read because its session was rolled back would mean that if the underlying value changed, validation would miss the dependency and the transaction would commit with a stale observation.

This gives the rollback work a specific shape. Rollback restores only the map entries for *keys the transaction wrote*. Reads already in the map stay. The journal size is bounded by the number of writes since the top checkpoint, not by the number of reads.

### Sub-transaction model

The driver — typically AptosVM orchestrating prologue, user, and epilogue phases — calls `push_checkpoint`, `commit`, and `rollback` at explicit boundaries. Move bytecode does not raise a rollback on abort; an abort inside a session either propagates out or is caught by the driver, which then decides whether to invoke `rollback`.

Exception-handling semantics — catching aborts in Move and automatically rolling back to the nearest enclosing checkpoint — are a possible future extension. They require no redesign of this layer; they only add a caller that invokes `rollback` on specific abort conditions.

## Compaction and Heap-Full GC

The Cheney collector described in `heap_and_gc.md` has two distinct callers in this design. They use the same primitive with different root sets and different triggers.

### Heap-full GC

Runs during execution when the bump heap fills. Roots:

- Call-frame pointer slots declared by `Function::frame_layout` and `safe_point_layouts`.
- Working map `data_ptr` fields.
- Journal `prev_entry.data_ptr` fields.

Behaves exactly as in `heap_and_gc.md`, with the addition that the working map and journal are now part of the root set alongside the call stack.

### Compaction

Runs at a quiescent point — the call stack is empty, no in-flight reference into the region exists outside the working map. Roots:

- Working map `data_ptr` fields only.

The journal is cleared before this pass, because compaction is invoked at a point past which no rollback is possible (a commit boundary, or publication itself).

Compaction copies only the live resource data the working map references into a fresh, tightly sized `MemoryRegion`. The old region is discarded.

### Why compaction is needed

Without a compaction pass, an incarnation's published region contains every allocation the transaction ever made: temporaries, intermediate values, stale CoW blocks from older checkpoint epochs, unreachable allocations produced before a mid-session GC that were later rolled back. The working map at publication typically references only a small subset of this memory — the current state of the keys the incarnation wrote.

Because Block-STM keeps published incarnation regions alive until they are superseded or the block ends, this garbage multiplies across incarnations. Compaction bounds a published region's size to the live resource data it exposes.

### When compaction runs

- **Mandatory at publication.** Every incarnation runs compaction before its `MemoryRegion` is exposed to Block-STM. This guarantees the published region is minimal.
- **Optional at intermediate quiescent points.** The driver may invoke compaction at a natural boundary between sessions — for example, after prologue, before the user session begins — to reclaim prologue's temporaries before user-session execution. This is an optimization, not a requirement.

Both invocations use the same primitive. Policy is separable.

## GC Interaction with Checkpoints

Heap-full GC can run between `push_checkpoint` and the matching rollback or commit. This breaks the `bump_mark` stored in the checkpoint in two ways:

1. The address is stale. The mark pointed into the from-space that GC just freed.
2. The partition disappears. Pre-checkpoint and post-checkpoint allocations existed, before GC, as two contiguous regions separated by `bump_mark`. After GC compacts everything into to-space, both groups of live objects are interleaved in one contiguous run. No post-GC address distinguishes the two.

Bump-rewind as a reclamation strategy therefore stops working for any checkpoint whose mark predates a GC.

### What is not broken

Map and journal pointers are updated by GC because both are root sets. Lookups continue to work; journal replay continues to restore the correct values (the restored pointers are post-GC addresses). Pointers into the block cache are stable because the block cache lives outside the txn's GC'd region.

Rollback state restoration — the "walk the journal back and apply each undo record" half — is unaffected.

### Decision

Accept the degradation. At GC time, any checkpoint whose mark predates the collection is marked stale. Rollback with a stale mark performs journal replay only and skips the bump-rewind step.

Consequence: post-rollback, any allocations that rollback made unreachable (by restoring older map entries over them) remain in to-space as unreachable garbage. They are collected by the next GC or by the next compaction pass — which, at publication, always runs.

### Why this is acceptable

- GC is rare; heap is sized for the common case.
- Rollback is rare; it is the abort path.
- The compound case — GC runs mid-session *and* the driver then rolls back — is rarer still.
- When it does happen, next-GC or publication-compaction cleans up. Correctness is preserved throughout.

The alternative — forbidding GC across a live checkpoint — would turn a long allocation-heavy session into an OOM risk, a worse failure mode than transient garbage.

## Validation Metadata

Each map entry carries version metadata sufficient for Block-STM validation. The concrete form — a version coordinate `(txn_id, incarnation)` that Block-STM already produces, a hash of the observed value, or something else — is orthogonal to this design. Pick whatever the MVHashMap already exposes and refine later.

What matters here is only that the metadata is captured at the moment of the read and remains with the entry through to publication and validation.

## Out of Scope

- **Reclamation mechanism for retired incarnation regions.** Supersede retires regions; reclamation is deferred. The likely direction is a dedicated reaper using per-worker monotonic counters as a quiescence signal, sized to the worker pool rather than the transaction count. Not specified here.
- **Interpreter state save/restore across checkpoints.** Checkpoints in this design preserve only the working map, the journal, and the bump pointer. Frame stack, program counter, and gas counter are not snapshotted. The driver is expected to take checkpoints at boundaries where the call stack is empty and no execution context needs preserving.
- **Integration with AptosVM and the Block-STM scheduler.** The mechanics of how the driver invokes `push_checkpoint`, `commit`, and `rollback` around prologue / user / epilogue phases, how the reaper is wired into the scheduler, and how publication is sequenced against MVHashMap updates all live in the integration layer.
- **Gas accounting.** Metering lives elsewhere.

## Future Work

- **Zero-copy reads (C2).** Extend the resource view to optionally carry a lifetime witness and point directly into the source region. No other structure changes.
- **Concrete reclamation scheme.** Flesh out the reaper-plus-counters sketch: thread assignment, queue representation, polling cadence, interaction with the Block-STM scheduler's task dispatch.
- **Checkpoint extension to interpreter state.** If a future workload needs to pause mid-execution of a session rather than only at session boundaries, extend the checkpoint to save frame-pointer, program-counter, and gas state. The storage-layer checkpoint mechanism is orthogonal and does not need to change.
- **Per-key version-stack rollback (D2).** The journal approach here (D1) assumes rollback is the rare path. If speculation-heavy workloads with many nested sub-transactions become common, per-key version stacks with lazy reclamation may amortize better. The working-map entry shape would need to grow.
- **Block-STM supersede tightening.** The supersede protocol here requires removing `MVHashMap` entries for keys that a new incarnation does not rewrite. The existing implementation overwrites entries on re-execution but may not cleanly remove stale keys. Verify and close any gap.

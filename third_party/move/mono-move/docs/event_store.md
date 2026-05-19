# Event Store

Per-transaction store for events emitted by Move code via `event::emit`. Supports nested checkpoint/rollback so execution can discard events alongside other side effects, and serializes lazily at transaction end.

## Design Goals

- **Everything lives on the existing heap** (`runtime/src/heap/`) — same allocator, same GC, same memory accounting as every other VM-managed value. No new arenas, no rogue allocations.
- **Lazy serialization** — store `(type, value)`; BCS-serialize once at transaction end via runtime type traversal. Events that get rolled back never pay the serialization cost.
- **Arbitrary-depth nested checkpoints** — O(1) push, O(n_discarded) rollback.

> **Note:** Technically we could start with a Rust-side struct and migrate later, but (1) heap-managed is the eventual goal anyway, and (2) implementing it directly on the heap isn't much harder — so we just do it that way from the start.

## Memory Layout

The event store is a **custom heap-allocated data structure** — similar in spirit to a Move vector, and ideally reusing the existing vector implementation (`alloc_vec`, `grow_vec_ref`, vector descriptor) for its internal arrays.

All event values are boxed. This is to allow values of different types to be stored in the same data structure. (In other words, the EventStore is heterogeneous.)

Each part (the outer container, the two arrays, every boxed event value) is its own heap object with its own `ObjectDescriptor`. The interpreter context holds **one extra root pointer** to the store; the GC reaches everything else through the standard descriptor-driven scan.

The outer container plus its two arrays:

```
Interp ctx        Heap (EventStore)                Heap (entries vec)
┌────────┐        ┌──────────────────────┐         ┌──────────────────────┐
│        │        │ desc_id(4) | size(4) │         │ desc_id(4) | size(4) │
│ root ●─┼───────►│ entries     ●────────┼────────►│ length (u64)         │
│        │        │ checkpoints ●        │         ├──────────────────────┤
└────────┘        └─────────────┼────────┘         │ EventEntry[0]        │  16 B
 (or null                       │                  │ EventEntry[1]        │  each
  before first                  ▼                  │ ...                  │
  emit)                Heap (checkpoints vec)      └──────────────────────┘
                     ┌──────────────────────┐
                     │ desc_id(4) | size(4) │
                     │ length (u64)         │
                     ├──────────────────────┤
                     │ u32 (snapshot)       │
                     │ u32 (snapshot)       │
                     │ ...                  │
                     └──────────────────────┘
```

A single `EventEntry` and its boxed value:

```
EventEntry                        Heap (boxed value)
┌──────────────────────┐          ┌──────────────────────┐
│ ty:    InternedType  │          │ desc_id(4) | size(4) │
│ value: HeapPtr   ●───┼─────────►│ boxed value          │
└──────────────────────┘          │ ...                  │
  16 bytes                        └──────────────────────┘
```

Descriptor summary:

- EventStore: `Struct { size = 16, pointer_offsets = [0, 8] }`.
- entries vector: `Vector { elem_size = 16, elem_pointer_offsets = [8] }` — `ty` is an arena pointer into the long-lived global type DAG (not on the heap), so only `value` is listed as a heap pointer.
- checkpoints vector: `Vector { elem_size = 4, elem_pointer_offsets = [] }`.
- Boxed value: a fresh heap chunk allocated at emit time. Its descriptor depends on the source value's runtime layout.
    - **Heap struct** — the box holds just a pointer to the source's existing heap chunk. Descriptor: `Struct { size = 8, pointer_offsets = [0] }`.
    - **Inline struct** — descriptor marks the inline struct's pointer fields — effectively promoting the inline struct to the heap.

`module_id` is derivable from `ty` (`Type::Nominal { module_id, .. }`), so the entry doesn't carry it separately.

## GC Safety

`emit` and the growth path can each perform multiple allocations in sequence. Any of those allocations can trigger GC, and an in-flight pointer that isn't yet wired into a heap-reachable slot would otherwise be lost.

This is exactly where `PinnedRoots` can be helpful. Alternatively we could reserve enough space upfront for all allocations the operation will make. Tradeoff: pinning costs per-allocation bookkeeping but handles variable-size or branching allocation paths; pre-reserving is simpler at the call site but needs a worst-case footprint estimate.

## Key Operations

The Move entrypoint is `event::emit<T: store + drop>(msg: T)` (defined in `aptos_framework::event`), which immediately forwards to the actual native `write_module_event_to_store<T>(msg)`. Both take the event payload by owned value, not by reference. The old event-handle-based system (`EventHandle<T>` plus `write_to_event_store`) is omitted — MonoMove does not carry it forward.

The Event Store provides the following APIs to MonoMove's native functions:

- **emit(ty, value)** — boxes the given event value, and then append it to `entries` along with its type.
- **create_checkpoint() → Handle** — push the current entries length onto the checkpoints array; return the new marker's index.
- **rollback(h)** — truncate the entries array back to the snapshot at `h`, then truncate the checkpoints array down to and including `h`. Orphaned boxed values and orphaned old-buffer copies become unreachable and are reclaimed at the next GC.
- **iter** — read-only walk in emission order, used at transaction end for serialization.

No explicit commit: a marker that's never rolled back to just sits in the checkpoints array (4 bytes) until transaction end.

## Growing

The two internal arrays grow via the existing vector machinery (`grow_vec_ref` / `alloc_vec`). No special growth logic for the event store.

## End-of-Transaction Serialization

After Move execution finishes, the host walks the entries array and BCS-serializes each boxed value using the runtime type traversal. Output shape is whatever the rest of the pipeline expects (e.g. `Vec<ContractEvent>`).

This step requires additional gas metering, covering the type traversal and BCS encoding work.

## Integration with Natives

`write_module_event_to_store` is the sole entry point for adding entries. The native shim reads the owned `msg: T` from the calling frame via the native context's argument accessors and forwards to the store's emit operation. Checkpoint and rollback are driven by the execution engine — not surfaced to native code.

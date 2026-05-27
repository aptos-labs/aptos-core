# Global Storage

This document describes how MonoMove manages per-transaction access to global state.
Specifically, it covers:

- how transactions record their reads,
- how transactions record their pending writes,
- how transactions checkpoint execution and roll back to previously saved state (e.g., save prologue state, roll back on epilogue failure),
- how transactions make their writes visible to other transactions and bound their memory footprint (in Block-STM).

## Requirements

- Execution of every transaction records the reads it performs against the global state.
  The read-set is needed for:
    1) Block-STM validation,
    2) local caching so repeated reads do not re-enter the shared concurrent data structure,
    3) charging gas for IO on first load only (TBD if it has to be BCS-based).

- Obtaining a global reference does not involve copying the entire value representation behind the reference.
  Hence, VM may allow pointers into another transaction's memory region.
  Because VM can read something other transaction writes, it is important that representation of the read value is consistent with the representation of the written value.
  For example, if writes are represented as an overlay over immutable flat-memory value, reads should understand this representation.

- Execution of every transaction records its pending writes as a write-set.
  The write-set can be made visible to Block-STM so that other transactions can observe it.
  Writes should be charged gas for IO (TBD if it has to be BCS-based).

- Values are trees of allocations.
  For small structs, trees can have 1 node: all fields inline.
  For larger structs, or enums, trees can have multiple nodes (each node is a heap allocation).
  Global storage design should work for both cases.

- Within a single transaction, the VM has an ability to checkpoint the write-set.
  At later points, the VM can choose to continue or roll back any number of checkpoints from the top of the checkpoint stack.
  **History is linear:** discarded states during roll back cannot be recovered.
  For example, if checkpoints are *[S1, S2, S3]* with *S3* on top, rolling back two steps restores the state saved by *S1* and discards *S2* and *S3*.
  This approach is used to handle prologue, user and epilogue sessions efficiently in Aptos VM.
  Supporting general try-catch is not a goal.

## Design Summary

A transaction caches all global value reads in a **working map**.
The working map holds pointers to heap allocations (local or in other transaction space) and inline data.
Before any mutation, copy-on-write (CoW) is performed.
The CoW mechanism is orthogonal to its granularity - the implementation may choose **eager** (deep copy the whole nested resource at `borrow_global_mut` time) or **lazy** (copy one allocation at a time on demand, as the bytecode actually navigates into them).
Values modified after CoW form the write-set; later sections describe how modifications are detected.

Alongside the working map, the transaction stores:
- a linear journal of working-map entry transitions used for rollback,
- a monotonic per-transaction epoch counter that advances at every checkpoint.

The journal is append-only.
On checkpoint, the runtime saves the current journal length and the epoch counter.
On rollback, the journal is walked from the end back to the saved length and each entry's undo is applied to the working map.
Entries are appended only on the *first* state transition of the value in the current checkpoint epoch.
Subsequent in-place mutations within the same epoch do not append, so the journal's size is bounded by the number of unique keys touched.
Checkpoint cost is O(1); rollback cost is O(unique keys touched since the checkpoint).
Pre-checkpoint allocations are never reached on the mutation path, so they remain physically untouched after rollback; allocations made post-checkpoint that become unreachable after rollback are reclaimed by GC.
Alternatives to the linear-journal are described in later sections.

## Per-Transaction State

### Working Map

The working map is `HashMap<Key, Entry>` and, for the initial implementation, does not live on the same heap as values.
The `Key` is `(AccountAddress, InternedType)`.
The type is interned, so equality and hashing are cheap (note that global storage access requires canonicalization of generic instantiations).
Note that eventually we may want to move the map to the heap (for more accurate accounting & security when it comes to memory limits).

> IMPORTANT: because this is a hashmap, its traversal order is non-deterministic.
  We make sure that write-set generation enforces determinism in later sections.

Entries track reads of global values and possible modifications.
Interpreter copies entry bytes or pointers into destination frame slots directly.

```rust
enum Read {
  // Value did not exist.
  DoesNotExist,
  // Points into another arena (other transaction write or block cache).
  // Version can be a Block-STM version, i.e., transaction index + incarnation (8 bytes total).
  ExternalHeap { ptr: *const u8, version: Version },
  // Optimization for inline structs/enums: small values copied directly.
  Inline { bytes: [u8; INLINE_MAX] },
}

enum MaybeWrite {
  // There was no write.
  NotModified,
  // Points into local transaction arena. The read was CoWed, but not necessarily modified.
  LocalHeap { ptr: *mut u8 },
  // Optimization for inline structs/enums: small values copied directly. Also, not necessarily modified.
  Inline { bytes: [u8; INLINE_MAX] },
  // Value was deleted.
  Deleted,
}

struct Entry {
  read: Read,
  write: MaybeWrite,
  // Epoch of this write.
  epoch: u64,
  // Other fields, e.g. Block-STM validation metadata.
}
```

Inline values exist because a heap allocation per small resource is wasteful.
Small structs like `Coin<APT>` with a `u64` balance field already live in the frame's slot.
For such small resources, they are always copied between slots and the working map.
Resources that have nested heap allocations always take the `LocalHeap`/`ExternalHeap` path.
Here, `ExternalHeap` is the read-only zero-copy variant.
It refers either into another transaction's arena (carrying a Block-STM version, TBD) or into the block cache.
A single variant covers both because they are structurally the same - a raw pointer into externally-managed memory plus enough metadata to validate.
`LocalHeap` means the transaction CoWed the `ExternalHeap` read or executed `move_to`.

## Bytecode Interface

The instruction set gains new micro-ops for global storage: `Exists`, `BorrowGlobal`, `BorrowGlobalMut`, `MoveFrom`, `MoveTo`.
All instructions have these parameters in common:
  - `addr`: the resource address as a runtime frame offset,
  - `ty`: the resource type (`InternedType`), embedded at compile time,
  - `descriptor_id`: info about resource size (used to decide whether the resource qualifies for inline optimization). TBD if this can be merged with the type.

Every instruction starts by reading `addr`, combining with `ty` to form the resource key, and performing a lookup in the working map.
On a miss, the value is fetched from storage (Block-STM) and is recorded as a read in the working map.

### Existence

```rust
enum MicroOp {
  Exists {
    addr: FrameOffset,
    ty: InternedType,
    dst: FrameOffset,
    // Note: descriptor is inferrable from type.
    descriptor_id: DescriptorId,
  }
}
```

`Exists` performs an existence check.
Existence of the resource is determined from the entry's `(read, write)` pair:
- `(DoesNotExist, NotModified)`: does not exist.
- `(_, NotModified)`: exists.
- `(_, Deleted)`: does not exist.
- `(_, _)`: exists otherwise.
Writes the boolean outcome to the `dst` slot.
The journal is not touched because `Exists` is a read-only observation.

### Borrowing

```rust
enum MicroOp {
  BorrowGlobal {
    addr: FrameOffset,
    ty: InternedType,
    dst: FrameOffset,
    // Note: descriptor is inferrable from type.
    descriptor_id: DescriptorId,
  }
}
```

`BorrowGlobal` is the read-only borrow.
It aborts if the resource does not exist.
The value borrowed comes from the entry's `write` if set, or from `read` otherwise.
For `Inline` values the bytes are copied into `dst`.
For heap-allocated variants (`LocalHeap` or `ExternalHeap`) the pointer is copied into `dst`.
The journal is not touched because `BorrowGlobal` is a read-only observation.

```rust
enum MicroOp {
  BorrowGlobalMut {
    addr: FrameOffset,
    ty: InternedType,
    dst: FrameOffset,
    // Note: descriptor is inferrable from type.
    descriptor_id: DescriptorId,
  }
}
```

`BorrowGlobalMut` is the mutable borrow.
Like the immutable borrow, the instruction aborts if the resource does not exist.
Because a mutable borrow may result in a storage write later, the following invariant is enforced:

> When any write to an allocation happens (e.g., via `WriteRef`, `VecPushBack`), the allocation belongs to the local transaction heap, as do all allocations on the path from the resource root to this allocation.
  Furthermore, for a write that happens at epoch E, all allocations along the path (including the allocation where the write happens) have been produced in epoch E.

The motivation here is that nested allocations are not safely mutable while their parent is shared.
The invariant keeps mutations isolated and local by copying at least the reachable path before any write happens.

On mutable borrow, the following actions are performed:
1. If the entry has a previous write which is `LocalHeap` or `Inline` with `entry.epoch == current_epoch`, there is no need to copy any allocations.
   The inline data or the pointer is copied into `dst`.
2. Otherwise, the data was read-only (need CoW for mutation) or has been saved (epoch changed, need to CoW).
   A new write needs to be issued.

   For `Inline`, we copy bytes from the source (`write` if set, otherwise `read`) into the new `write` and into `dst`, and set the entry's epoch to `current_epoch`.
   The old write is added to the journal's undo log.

   For `LocalHeap`/`ExternalHeap`, we bump-allocate a fresh root and deep-copy the value (see later sections on the effect of the copies and mitigations).
   We set `write` to `LocalHeap { ptr: new_root }`, copy the new pointer into `dst`, and set the entry's epoch to `current_epoch`.
   The old write is logged in the journal.

### Moves

```rust
enum MicroOp {
  MoveFrom {
    addr: FrameOffset,
    ty: InternedType,
    dst: FrameOffset,
    // Note: descriptor is inferrable from type.
    descriptor_id: DescriptorId,
  }
}
```

`MoveFrom` removes the resource from global storage and copies its bytes or pointer into `dst`.
The instruction aborts the transaction if the resource doesn't exist.
The implementation is equivalent to `borrow_global_mut`: a CoW is needed for external pointers or for saved writes from older epochs.
The main difference is that the working map records the resource as `Deleted`, and the heap/slot become the sole owners of the resource.

```rust
enum MicroOp {
  MoveTo {
    signer: FrameOffset,
    ty: InternedType,
    src: FrameOffset,
    // Note: descriptor is inferrable from type.
    descriptor_id: DescriptorId,
  }
}
```

`MoveTo` publishes a resource at the given address.
The instruction aborts the transaction if the resource already exists.
The pointer or inline data is moved to the working map from the `src` slot, and the previous write is appended to the journal.
The previous write is `NotModified` for a fresh resource, or `Deleted` if a `move_from` removed K earlier in the transaction (possibly in a prior epoch).
Both must be recorded faithfully: a rollback of `MoveTo` after a prior `move_from` has to restore the entry's `write` to `Deleted`, not `NotModified`, otherwise the existence check would report the resource as present when it was actually deleted.
There is no need to CoW because the local heap owns the moved resource.

## Checkpoints and Rollback

The working set and the journal support `checkpoint()` and `rollback(n)` (undoes the last `n` checkpoints) during a single transaction.
These operations are invoked exclusively by the driver (Aptos VM), when there is no live data other than global values / events.
Also, the checkpoint stack's depth is bounded by the Aptos VM session model (~3 levels: prologue, user, epilogue).

A checkpoint stack runs alongside the working map.
`checkpoint()` pushes `(journal.len(), current_epoch)` onto the stack and then increments `current_epoch`.

`rollback(n)` undoes the last `n` checkpoints.
It reads the `n`-th-from-top checkpoint, walks the journal back to that checkpoint's saved `journal.len()` applying each entry's undo, restores `current_epoch` to the saved value, and pops the `n` checkpoints from the stack.
The bump arena is not touched: allocations made between the rolled-back checkpoints and the current position become unreachable once the working map is restored, and GC reclaims them on its next collection (or at end-of-transaction arena reset).
Working-map recovery is one journal pass bounded by the number of unique keys touched.
The journal is part of the root set for GC.

## Publication of writes

In the end, the transaction needs to:
1. Charge gas for its writes.
2. Make writes visible to other transactions.

The issue in the current design is that `borrow_global_mut`/`move_to` may CoW the value, but because there is no dirty bit, it is not possible to check whether the resource has actually been modified.
While patterns like `let x = borrow_global_mut<T>(...); if (cond) { mutate(x); }` are not very common, they exist.

The proposal is to support this gradually.

### Step 1

Any `borrow_global_mut`/`move_to` is considered side-effecting and produces a write.

### Step 2

When the transaction makes its writes visible, it also compacts its heap to remove any temporary data that is now dead (compaction).
In the end, only event and global storage allocations remain live.
During this traversal, all allocations are visited.
Hence, while traversing, it is possible to:
1. Simulate `bcs::serialized_size` of a resource written.
2. Detect whether the allocation is a modification by running `memcmp` against the read value.
   This also allows filtering out modifications such as assigning the same value every time.

### Storage Metadata, Refunds, and Aptos VM integration

Aptos VM needs to charge gas for writes and compute refunds when the user session finishes.
Specifically, for each entry whose bytes differ from the source:
1. Compute BCS size of the new value (write).
2. Compute BCS size of the old value (read, or 0 if newly created).
3. Calculate the charge based on old and new sizes, and compute the refund and state value metadata.

To make MonoMove work with this approach, the following must be done.

**Pass 1 at end of user session:**
Iterate over the heap to compact, compute individual gas charges, and charge as a sum (enforces determinism).
Also compute refunds.

Then there are two options to handle the epilogue:
- Option 1: run another compaction. Enforce that epilogue writes are BCS-serializable.
- Option 2: do nothing. Via tests, ensure epilogue writes are BCS-serializable and that they do not create excessive memory allocations that need compaction.

## Extensions

This design has three explicit decisions:
1. Deep copy on new-epoch `borrow_global_mut` or `move_from`.
2. Eventual detection of a write via comparison with read (or over-approximation).
3. Journal for rollback support.

These can all be improved if needed and built on top.

### Per-Allocation Lazy CoW

Eager deep copy at `BorrowGlobalMut` and `MoveTo` pays the full cost of a resource's reachable graph upfront, even when the eventual writes only touch a small fraction.
For example, if we mutate a resource, then checkpoint, then mutate again, two full copies are made.
This is wasteful (however, the final write still requires the full resource, so the copy is eventually needed).
An extension is to do CoW lazily:
- each allocation carries its own epoch tag in the header,
- the allocations actually mutated on a given borrow path get copied.

With per-allocation epoch tags, any instruction that mutably borrows may need to CoW (for example, `HeapBorrow`, `VecBorrow`, etc.).
For each, we read the target allocation's epoch from its header and compare against `current_epoch`.
If the target is current, the borrow returns a reference based on the target as-is.
If the target is older, the runtime copies it:
- bump-allocates a fresh copy,
- `memcpy`s the target's bytes and stamps the new header with `current_epoch`,
- writes the new pointer back into the parent's pointer field (the *patch*),
- returns the reference to the new copy.

The patch is safe due to the invariant that any mutable path is always owned.
For a child to be mutated, its parent must be owned, which in turn requires the grandparent to be owned, all the way up to the working-map entry.

### Per-Allocation Dirty Bit Tracking

The current design detects writes by byte-comparing the write graph against its read source (if it does at all).
Instead, a per-allocation dirty bit in the object header eliminates the comparison for clean allocations: write instructions (such as `WriteRef`) mark the target allocation's dirty bit, and the compaction walk detects resources that contain dirty allocations.

The mechanism is simple.
Each heap allocation gains a dirty bit in its header — one of the spare flag bits if the header has already been repacked for per-allocation epoch tags (see above).
Instructions that write (`WriteRef`, `VecStoreElem`, `VecPushBack`) compute `(base - OBJECT_HEADER_SIZE).flags |= DIRTY` to set the bit.
At publication, the compaction walk reads the dirty bit per allocation.
Clean allocations are byte-equal to their source by construction and skip the publication path; dirty allocations are emitted into the write-set.

#### Alternatives to Dirty Bit Tracking and CoWs

**CoW instruction** can be used to let the specializer fold copying into conditionals.

**Per-reference dirty-flag pointer** for write detection, where each reference carries an extra 8-byte pointer to the entry's dirty bit and write ops update the bit through it.
This avoids the publication-time byte comparison entirely — every mutation marks the entry directly.
This has its costs: references grow in size, and sub-borrows must inherit the pointer.

**CoW at WriteRef time** can also be used instead of borrow time (related to the alternative above).
The reference carries the pointer to the resource root (parent).
CoW is performed only at `WriteRef` (or other writing instructions) time.
This again increases reference size.

Alternatives to the journal have also been considered, and are listed below.

**Map-level snapshot at checkpoint**, where the entire working map is cloned at checkpoint creation and rollback discards the new map for the saved one.
Checkpoint cost is `O(working map size)`; rollback cost is `O(1)`.
The trade goes the wrong way: checkpoints are on the hot path (session boundary), rollbacks are on the cold path (aborts).

**Per-key version stacks** in the working map, in place of the linear journal.
Each entry would hold a stack of `(epoch, value)` versions, with rollback popping entries above the target epoch.
The shape suggests laziness, since only keys actually modified pay any per-key cost.
Rollback is therefore much faster, because it is applied lazily and only to writes that span multiple epochs.
This is very likely to be added.

**Persistent data structures with structural sharing at the value layer** (HAMT-style) are not applicable.
Values do not need to be structurally shared, and the sharing would add extra indirection on every pointer hop.

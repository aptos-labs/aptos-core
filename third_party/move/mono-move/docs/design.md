# MonoMove: Design Document

MonoMove is a Move Virtual Machine (VM) designed for performance and safety.
At its core, it relies on monomorphization in order to handle Move generics and
maximize efficiency.
MonoMove design follows the following principles:

1. **Stateless VM**:
   VM does not store any long-living context.
   For execution, VM requires an external local (per-transaction) and global
   contexts.

2. **Performance Built-in by Design**:
   Value system uses a flat memory representation.
   Hot execution paths have minimal pointer chasing: hot data is tightly
   packed, cold data separated.
   Execution operates on monomorphized basic blocks or
   fully-monomorphized functions.
   Gas charges are aggregated when possible, for better efficiency.

3. **Long-Living Caches**:
   Code-derived data (types, instantiated types, monomorphized functions, etc.)
   are cached globally and are shared between threads.
   Allocations of the values used by the VM are managed by per-transaction
   (e.g., if VM created a value) or global context (e.g., if VM borrowed a
   resource from storage).
   Values use flat-memory representation.
   Flat value representation is cached globally to avoid BCS-to-flat conversions.

4. **Safety Built-in by Design**:
   Additional runtime reference or type checks are built-in.
   Metering is a first-class citizen.


## Execution Model

Throughout the document, the following execution model is used.

1. There is a single executor instance that runs blocks (validator nodes) or
   chunks (state syncing nodes) of transactions.
   There is only parallelism within each block, but any two distinct blocks
   are sequential.
2. For each transaction block, a Block-STM instance is created to run
   transactions in the block in parallel.
   Block-STM is using a fixed number of workers that execute different tasks
   (executing transactions speculatively, validating execution, running
   post-processing hooks for committed transactions).
3. Every transaction execution is an invocation of Aptos VM (wrapper around
   Move VM).
   Aptos VM dispatches calls to Move VM to execute specified functions or
   scripts.


## MonoMove Design

### 1. Global Execution Context

Throughout transaction block execution, a global context is owned by executor
that stores long-living caches for code-related data and configs.
Memory allocations for these caches are managed in epoch-based reclamation
style, so that garbage collection (GC) only runs between transaction blocks.
With this design, only concurrent allocations to the cache need to be
supported, but not deallocations.
**This imposes an invariant that our various limits need to imply caches cannot get full within a block.***

#### Rationale

1. Concurrent deallocation is a hard problem.
   It is difficult to ensure data is not deallocated if it is in use
   (efficiently).
2. For smart contract code, deallocations are rarely needed.
   Either code-derived data can be indefinitely persisted (e.g., interning
   strings as unique IDs), or becomes dead seldomly (on module upgrade).

#### Data Structures

The global context is a 2-phase state machine that can either be in
**Execution** or **Maintenance** states:

1. **Execution**: multiple workers can hold allocation and read data
   concurrently.
2. **Maintenance**: no workers allowed; context is checked if GC is needed,
   data may be deallocated.

To enforce these state at compile time, a guard pattern is used.
There are two distinct RAII guards:

- **`ExecutionGuard`**: obtained by workers during block execution.
  Holding this guard means it is safe to hand out references tied to the
  guard's lifetime, and it is legal to allocate new cache entries through
  the guard.
- **`MaintenanceGuard`**: obtained by the single-threaded executor *between
  blocks*.
  While held, no new `ExecutionGuard` can be created. This provides a clear
  "border" when deallocation finishes.

##### Implementation Sketch

```rust
// Note: in practice, can use a single atomic here.
struct GlobalExecutionContext {
   // Set to true only between blocks.
   maintenance: AtomicBool,
   // Number of live worker guards. Must be 0 during maintenance.
   active_executions: AtomicUsize,

   // ... caches, arenas, config, etc ...
}

struct ExecutionGuard<'a> {
   ctx: &'a GlobalExecutionContext,
}

impl Drop for ExecutionGuard<'_> {
   fn drop(&mut self) {
      self.ctx.active_executions.fetch_sub(1, Ordering::Release);
   }
}

struct MaintenanceGuard<'a> {
   ctx: &'a GlobalExecutionContext,
}

impl Drop for MaintenanceGuard<'_> {
   fn drop(&mut self) {
      self.ctx.maintenance.store(false, Ordering::Release);
   }
}

impl GlobalExecutionContext {
   fn execution_guard(&self) -> Option<ExecutionGuard<'_>> {
      // If in maintenance mode, return None. Otherwise, increase the execution
      // count and return the guard.
      ...
   }

   fn maintenance_guard(&self) -> Option<MaintenanceGuard<'_>> {
      // If already in maintence mode or there are still executions, return None.
      // Otherwise move to  maintence mode and return the guard.
      ...
   }
}
```


### 2. Global Identifiers

Module address-name pairs, function/struct identifiers, and fully-instantiated
types (and type lists) are interned as compact integer IDs.
Interning is managed by the global context which owns the interner tables and
the corresponding caches (e.g., for reverse lookups).
These IDs (and the data interned behind them: addresses, strings, and type
representations) are executor-only implementation details:

- **Not persisted**:
  IDs are never written to storage and are never included in transaction
  outputs.
- **Non-deterministic**:
  IDs may differ across nodes and process runs.
- **Safe across module upgrades**:
  Module upgrades do not invalidate IDs.
- **Memory management**:
  Global context may perform a reset of interner tables at transaction block
  boundaries to reclaim memory or prevent ID overflow.
  A reset invalidates **all** dependent interned tables and caches (IDs,
  reverse lookups, and type-derived caches such as abilities).
- **Per-table counters**:
  Each interner table maintains its own counter (e.g., types, type lists,
  module IDs, name strings).
  Counters are checked at transaction block boundaries so overflow (and hence
  the deallocation) never happens at runtime.
- **No leakage across reset**:
  IDs must not leak into unguarded long-lived state that may outlive
  a reset.
  This is a reasonable assumption in practice because resets only occur near
  ID-counter exhaustion or when memory usage becomes unmanageable, which should
  be rare (e.g., after long node uptime, with traffic skewed toward a stable set
  of types).
  Using an ID after a reset is a code-invariant violation and may lead
  to non-determinism (and chain halt), but must never cause UB.
  It could also lead to type confusion, but because of ID non-determinsim, type
  confusion is likely non-deterministic across nodes (leading to chain halt).

#### 2.1 Module Identifiers

Address-name pairs or `language_storage::ModuleId`s are interned as a 32-bit
identifier.
The most significant bit of the integer identifier is reserved to indicate if
the address is special (0x0, 0x1, ..., 0xF) or not.

```rust
/// Compact module identifier (32-bit) encoding address-name pair.
/// MSB is reserved to indicate if address is special (1) or not (0).
pub struct ModuleId(u32);

impl MaintenanceGuard<'_> {
   pub fn module_id(id: &language_storage::ModuleId) -> ModuleId {
      ...
   }

   pub fn module_id_for_addr_name(
      addr: &AccountAddress,
      name: &IdentStr,
   ) -> ModuleId {
      ...
   }

   // Possibility: APIs to return ID with the size.
}
```

`ModuleId`s are obtained in 2 ways:
1. When a module is loaded for the first time (or a script is loaded), all
   addresses, and module names are interned.
2. When transaction payload is executed in Aptos VM, entry function module
   identifier is interned.
As a result, even if modules are cached, at least 1 lookup is done per user
transaction.

A reverse lookup also needs to be supported for the following scenarios:
1. Setting the location of an error or debug information.
   This is needed only for aborts which are infrequent (< 90% of transactions
   traffic).
2. To calculate the size of the storage key corresponding to this ID (when
   publishing modules, storage keys are charged for).
   If bottleneck, size can be cached in ID itself (with `Hash`, `Eq`, etc.
   ignoring this field).

#### 2.2 Fully-Instantiated Types

Fully-instantiated types and type lists are interned as well.
Every type is a 32-bit identifier.
Two most significant bits are reserved to indicate if the type is a reference
(immutable or mutable).
This design is chosen because:

1. 30 bits is large enough to encode many possible types.
2. References can only be top-level in Move.
   References inside structs are unlikely to be implemented any time soon.
3. Reference and dereference operations are common, tagging IDs allows to avoid
   context accesses.

All primitive types such as `u64`, `address` are pre-defined ID values and are
not stored in the interning table.
This allows to avoid any lookups to the context for primitive types and ensures
that the invariant is that tables only store compound types.

Every list of types (e.g., type argument list) is also interned as a 32-bit
identifier.
A most significant bit is reserved to indicate if this is a small list with a
single type as a payload.
As a result, there is no need to "decode" `TypeListId` for single-type list
via context.
Empty `TypeSliceId` is always 0.


```rust
/// Type representation.
/// Two MSBs are reserved for references:
///   00 - not a reference
///   01 - immutable reference
///   11 - mutable reference
///   10 - reserved.
pub struct TypeId(u32);

/// Represents a list of types.
/// For empty list, 0 is reserved.
/// Two MSBs are reserved for small-vec optimization:
///   00 - not a small-vec
///   01 - this list has a single element directly encoded
///   10 / 11 -- reserved.
pub struct TypeListId(u32);
```

For type interning, fully-instantiated `SignatureToken`s are converted to
`TypeId`s.
This is done via hash-consing approach and auxiliary data structure storing
compound types:

```rust
enum TypeRepr {
   Vector {
      elem_id: TypeId,
   },
   Struct {
      // Uniquely identifies struct instantiation, see below.
      struct_id: StructId,
   },
   Function {
      args: TypeSliceId,
      results: TypeSliceId,
      // Ability set is represented as u8.
      abilities: AbilitySet,
   },
}
```

For every compound type, an additional metadata is tracked:

1. Ability sets - used for paranoid type checks.
2. Element type for vector types - used for paranoid type checks.
3. Argument and result types for function types - used for paranoid type checks.
4. Number of argument and result types for function types - used for paranoid
   stack balance checks.

The reverse lookups for this data are predominantly part of runtime type
checks.
This checks can be moved out of the hot path via tracing (see later
sections).

#### 2.3 Struct and Function Identifiers

Structs and functions are uniquely identified by IDs obtained via interning
table.
These IDs include:

- `ModuleId` indicating the module owning this struct or function.
- 32-bit integer encoding struct or function name.
- `TypeListId` for type arguments (set to 0 if non-generic).

```rust
/// Represents struct / function ID without type argument list. Can
/// only be derived from struct or function IDs.
pub struct TemplateId(ModuleId, u32);

/// Represents a struct identifier: module ID, struct name ID and
/// type argument list ID (0 for non-generic structs).
/// TODO: consider using a bit to indicate this is a resource group tag.
pub struct StructId(TemplateId, TypeListId);

/// Represents a function identifier: module ID, function name ID
/// and type argument list ID (0 if non-generic).
pub struct FunctionId(TemplateId, TypeListId);
```

Like `ModuleId`s, function and struct IDs are obtained:
1. When a module is loaded for the first time (or a script is loaded).
   All data in the module is interned and the loaded module is cached.
2. When transaction payload is executed in Aptos VM, entry function identifiers
   and type argument tags are interned.


### 3. Generic Types

In the file format, generics are represented as `TypeParam(u16)` variant in
`SignatureToken`.
MonoMove still needs to manage generics.
It uses a flattened format to avoid pointer chasing and ensure memory is
managed more efficiently (a compressed tree where instantiated leaves are
`TypeId`s and only truly generic leaves are kept as type parameters).
This efficient representation is needed because some instantiations may happen
at runtime and are not cached.

1. Primitives: 1 byte header.
2. Vectors or references: 1 byte header, followed by element encoding.
3. Structs: 1 byte header, followed by struct ID (8 bytes), number of type
   arguments, encoded type arguments.
4. Functions: 1 byte header, followed by number of arguments (1 byte), number
   of returns (1 byte), abilities (1 byte) then encoded arguments and returns.
5. Resolved type (`TypeId`): 1 byte header, 4-byte payload.
6. Type parameter: 1 byte header, followed by u16 index.

```rust
pub struct GenericType {
   // TODO: consider this being allocated in executable's arena.
   bytes: Box<[u8]>,
}

impl GlobalExecutionContextGuard<'_> {
   fn instantiate_generic_type(&self, template: &GenericType, ty_args: &[TypeId]) -> TypeId {
      // Walks the type tokens, interning what is needed.
      // We can also use a cache, because each ty_args list has a unique ID
      // so we can have a cache for each generic type.
      ...
   }
}
```

### 4. Transaction Memory Management & Value Representation

During execution, the VM needs to manage memory for values — vectors (dynamically sized),
structs (large ones may need to be on the heap), and global values (may need a separate region).

This is distinct from memory for types, code and global context, which is covered in ealier sections.

MonoMove uses a **two-level memory management system** organized around the BlockSTM execution model:

1. **Block Level**: Manages shared state across all transactions -- the storage cache and allocation
   of memory subspaces for individual transactions.
2. **Transaction Level**: Each transaction receives a dedicated memory region with its own
   allocator.

*Note*: We may also want some data to live across blocks in the future. The block-level cache could
potentially be retained (rather than discarded) for use in subsequent blocks. TBD.

This separation enables bounding total memory usage and supports parallel transaction execution
with shared access to global values.

#### 4.1 Block Memory Manager

The block memory manager owns two key responsibilities:

1. **Storage Cache**: Caches resources loaded from storage, shared across all transactions
   within the block. The cached version is the resource state at the beginning of the block.
   This avoids redundant storage reads and BCS deserialization for frequently accessed resources.

2. **Transaction Memory Allocation**: Hands out memory subspaces to individual transactions.
   Each transaction receives a dedicated region that is then managed by its own
   transaction-level memory manager.

##### Sharing Global Values Across Transactions

While temporary values created during execution (local variables, intermediate results,
newly allocated structs and vectors) are exclusively local to a transaction, writes to
global values need to be made visible to subsequent transactions.

Two approaches can enable this sharing:

**Option 1: Freeze-on-Finish**

After a transaction finishes, freeze its memory space and expose it as read-only to
subsequent transactions.

- *Pros*: Simple to implement; less error-prone; provides a consistent view to readers.
- *Cons*: Coarse granularity; read-write conflicts are detected late.

**Option 2: Concurrent Data Structures**

Use a multi-version data structure to provide concurrent shared access to global values.

- *Pros*: Read-write conflicts are detected immediately.
- *Cons*: More complex -- requires a separate shared mutable subspace at the block level
  (similar to the storage cache, but mutable); may expose inconsistent views to readers;
  may require copying data between memory regions; interacts poorly with GC-managed memory
  (references held by block-level structures must be updated when GC moves memory, unlike
  freeze-on-finish where frozen regions are not subject to GC).

*TODO*: Analyze mainnet transaction history to better understand real-world read/write
patterns and inform the choice between these approaches.

##### Per-Block Memory Limits

A per-block memory limit sets the upper bound of memory a node may use for values at any
given time. This is important for resource planning and preventing out-of-memory conditions.

**Why This Matters**

Current node configuration uses a few dozen transactions per block to ensure low latency.
However, throughput-focused benchmarks may run hundreds or thousands of transactions per block.

Consider a default maximum of 10 MB per transaction (for values only — this excludes code
and other global context data):
- 1,000 transactions × 10 MB = 10 GB baseline memory usage.

This is already high, and several factors can further increase memory consumption:
- **Memory freezing + re-execution**: A transaction under re-execution may require two
  memory spaces -- one frozen (finished state) and one active (speculative execution).
- **Garbage collection**: A copying GC requires an additional "to-space", effectively
  doubling the memory footprint during collection.

In the worst case (all factors combined), peak memory usage could reach 30–40 GB. Typical
usage would be significantly lower, but we need to plan for adversarial conditions.

Such limits may be acceptable today, but pose concerns for future scalability. As VM
execution speed improves, we may want to include more transactions per block without
compromising latency significantly.

**Mitigations**

1. **Conservative initial allocation**: Start with a small allocation per transaction and
   grow as needed (e.g., 1 MB -> 4 MB -> 10 MB). The idea is that typical high-frequency
   transactions fit comfortably in the default allocation. Transactions with higher memory
   demands can request more, but may require pre-declaration or incur significant memory fees.
2. **Hard per-block memory limit**: Enforce a block-level cap and cut off remaining
   transactions if approaching the limit. We already have a per-block gas limit that
   functions in a similar way.
3. **Compact-on-freeze**: When freezing a transaction's memory space, retain only the
   global value writes and discard temporary values. This significantly reduces the
   footprint of frozen regions for typical transactions, but has limited effect on
   malicious transactions that maximize global value writes. Note: we may need to do
   this anyway for write-set generation This scan may also be required for gas metering
   purposes.

#### 4.2 Transaction Memory Manager

Each transaction gets its own memory manager for its subspace. The design has two
major goals:

1. **Blazingly fast allocation**: Allocation is on the hot path and must be minimal overhead.
2. **Bulk deallocation**: Reclaim memory in batches rather than per-object — both at
   transaction end (discarding temporaries) and during GC runs (if needed).

Two designs are under consideration:

**Option 1: Simple Bump Allocator**

Allocate by advancing a pointer; never deallocate individual objects. At transaction end,
destroy everything at once.

- *Pros*: Simplicity; speed — allocation is just a pointer bump.
- *Cons*: Limited scalability — if the allocator needs to grow, why not also run GC?
  Cannot handle pathological cases (e.g., a loop allocating large amounts of temporary data
  that could otherwise be reclaimed). Though some nuance here: (1) if these cases are truly
  pathological, is handling them important? (2) real-world Move code should be examined for
  compelling examples; (3) these considerations may be moot if GC is justified for other
  reasons.

**Option 2: Compacting GC**

A compacting garbage collector is well-suited for MonoMove because we only care about latency
at the block level. Pausing the world within a single transaction is a non-issue, provided that
the cost of running is GC is accounted for (e.g. via gas).

- *Pros*: Similar allocation speed (bump allocation) and bulk deallocation
  capabilities as Option 1, but with much better scalability — can reclaim memory mid-transaction
  or grow the heap, with defragmentation as a free byproduct of each GC run.
- *Cons*: Complexity — requires tracking of live set, and either pointer fixup or indirection.

Two implementation approaches are under consideration:

**Design A: Direct Pointers with Pointer Fixup**

References are raw pointers. During garbage collection, the collector traverses values
recursively to move them into the to-space while fixing all internal pointers.

- The memory manager must understand value layouts to locate and update pointers.
- Live set discovery starts from externally-managed roots (e.g., the operand stack, locals).
- *Pros*: No indirection overhead on value access.
- *Cons*: Tight coupling between memory manager and runtime; pointer fixup adds complexity
  and cost to each GC cycle; memory may need to be moved in fragmented pieces rather than
  large contiguous chunks.

**Design B: Handle-Based Indirection**

References are handle IDs. Each handle stores: (1) the actual memory address and size, and
(2) a parent field indicating ownership status.

```rust
enum Parent {
    None,            // Dead — memory can be reclaimed
    Root,            // Owned externally (e.g., on the operand stack)
    Handle(HandleId) // Owned by another managed value (e.g., element in a vector)
}

struct Handle {
    mem_ptr: *mut u8,
    size: usize,
    parent: Parent,
}
```

Key properties:

- **Index stability**: Once allocated, a handle's ID never changes while alive. This allows
  safe references via handle IDs. *TODO*: How do cross-transaction reads work here? If stable
  data must be copied on read, this could be expensive for read-heavy workloads. A CoW-like
  approach may be worth considering.
- **Liveness via parent chains**: A handle is alive if its parent is `Root`, or if its parent
  is another handle that is itself alive. This can be computed efficiently via memoization.
- **Handle recycling**: Dead handles are recycled to prevent the handle table from growing
  unboundedly.

During GC, the collector scans the handle table to partition handles into alive and dead sets.
Live memory is moved to the to-space in bulk; only the `mem_ptr` fields in handles need
updating. Dead handles are recycled, and their memory is implicitly reclaimed by not copying it.

- *Pros*: Decouples memory manager from value layout; simpler and faster GC; enables
  moving whole chunks without fragmentation.
- *Cons*: Adds an indirection layer on every access; requires the runtime to report
  ownership changes (e.g., when a value moves into or out of a container).

##### Design Considerations

The choice between Design A and Design B involves several trade-offs:

| Aspect | Design A (Direct Pointers) | Design B (Handles) |
|--------|---------------------------|-------------------|
| Access speed | No indirection | One extra pointer chase per access |
| GC complexity | Must traverse values, fix pointers | Scan handle table, follow parent chains |
| Memory movement | Fragmented pieces | Large contiguous chunks |
| Coupling | Memory manager must know value layouts | Memory manager is layout-agnostic |
| Runtime burden | None | Must report ownership changes |

Design A favors raw access speed at the cost of tighter coupling and more complex GC.
Design B favors architectural simplicity and bulk operations at the cost of indirection.

The indirection cost in Design B is predictable and may actually be cache-friendly
(the handle table is likely to stay hot). Whether this overhead is acceptable depends
on workload characteristics and should be validated with benchmarks.
The ownership reporting burden in Design B aligns naturally with Move's explicit ownership semantics,
but adds plumbing throughout the runtime.

A decision will need to be made based on performance measurements and implementation
complexity assessment.

##### Handling Global Values

Beyond temporary values, the transaction memory manager also handles operations on global
resources (e.g., `move_from`, `move_to`, `borrow_global`).

**Reading global values**: When a transaction reads a global resource, the value may come
from:
- The block-level storage cache (base value at block start), or
- Another transaction's writes or modifications (if using concurrent sharing).

The transaction tracks what it has read for later validation (BlockSTM needs to detect
read-write conflicts). *Possible optimization*: track not just values but also read constraints
(e.g., "checked that resource exists" vs "read the actual contents") for finer-grained
conflict detection.

An open question is whether reads require copying the value into local memory, or whether
the transaction can reference the source directly. This depends on the memory manager design
and the sharing approach.

**Modifying global values**: When a transaction modifies a global resource, it performs a
copy-on-write (CoW) into its own memory subspace. This keeps all modifications isolated, which:
- Enables rollback if the transaction aborts or needs re-execution.
- Allows other transactions to reference these modifications (the local memory holds the
  authoritative version of the transaction's writes).

In BlockSTM, each transaction's modifications integrate with `MVHashMap` — the transaction's
local memory effectively becomes a slot in the multi-version structure, replacing the current
`Arc<Value>` approach.

Open questions:

- **CoW timing**: Should CoW happen eagerly on `borrow_global_mut`, or lazily on actual
  write? Lazy CoW avoids unnecessary copies but requires tracking borrowed references to
  detect when a write occurs.
- **GC interaction**: If GC runs mid-transaction, references to the transaction's modified
  values (held by block-level structures for sharing) must be updated to reflect moved
  memory locations. This is likely not a concern if using freeze-on-finish (Section 4.1),
  since frozen memory is not subject to additional GC runs.

#### 4.3 Value Representation

Values are represented flat in memory. All values created by the VM and any modifications
are allocated in the transaction's memory region. Primitives (`u8`, `u64`, `bool`, `address`,
etc.) are trivially represented as raw bytes of the specified size.

**Structs**

The naive approach is to store structs on the heap, with a reference (pointer or handle) on
the stack pointing to the heap-allocated data.

However, inline structs have merits worth considering:
- **Field access**: With heap-allocated structs, accessing a field at a certain offset
  requires dynamic pointer arithmetic at load-time. Inline structs allow direct access to
  inner memory without pointer chasing.
- **Performance**: Pointer chasing everywhere is not ideal for performance and also
  complicates GC (more pointers to track/update).
- **Copyable structs**: For these, `memcpy` is fast, making inline storage attractive.
- **Trade-off**: The main concern is added complexity — supporting both inline and heap
  representations means two code paths to maintain. Inline structs may also require padding to
  a fixed size (though this does give predictable stack frame sizes).

Understanding struct size distributions in real-world Move code would help inform this
decision (what fraction are "small"? what threshold makes sense?).

*Possible optimization*: Small structs could be stored inline on the stack to avoid heap
allocation and indirection. This keeps the stack frame predictable in size (small structs
would be padded to a fixed size). For copyable structs this is particularly attractive since
`memcpy` is fast. The threshold size and whether this optimization is worthwhile depends on
complexity analysis.

**Enums**

Similar considerations apply as with structs. The naive approach stores enums on the heap,
with a reference on the stack pointing to the discriminant and variant payload.

Inline enums would need zero-padding so all variants occupy the same size (important for
monomorphization). The same trade-offs apply: better field access performance vs. added
complexity from dual representations.

*Note*: For simple enums with explicit representation (e.g., `#[repr(u64)]`), heap allocation
should be avoided entirely — this could be enforced at the language level.

**Vectors**

Vectors use a layout similar to Rust's `Vec`: a header containing a reference to heap-allocated
element storage, length, and capacity. The reference is a pointer (Design A) or handle
(Design B).

```
Stack: [ref | len | capacity]
        |
        v
Heap:  [elem 0][elem 1][elem 2]...
```

The underlying memory address can be used to distinguish whether the vector's storage
resides in transaction-local memory or in the global storage cache.

**Function Values**

Function values are represented as a pointer to a `Function` object. The object may initially
be shallow (unresolved), with full resolution deferred until the function is actually invoked.

*Note*: Embedding function pointers directly works within a block, but if we implement a
cross-block value cache, cached values containing function pointers would need updating when
function metadata is invalidated or relocated.

**References**

The representation of references depends on the memory manager design:
- **Design A (Direct Pointers)**: References are raw pointers into managed memory. Interior
  references (e.g., to a struct field or vector element) are simply pointers to that location.
- **Design B (Handles)**: References are handle IDs, requiring a table lookup to obtain
  the actual memory address. Interior references require an additional offset field to
  locate data within the handle's managed memory.

#### 4.4 Memory Safety

The memory management design should consider safety at runtime — preventing memory corruption,
invalid access, and undefined behavior. Below are some areas worth considering -- this list
is not exhaustive.

##### Reference Validity

Move's bytecode verifier provides static guarantees about reference safety (no dangling
references, proper borrow semantics). However, runtime checks may be valuable as
defense-in-depth against verifier bugs or interpreter errors.

Potential runtime checks to consider:
- **Epoch/generation counters**: Each handle or allocated memory chunk tracks a generation number.
  References include the expected generation; access fails if they don't match. This could
  catch use-after-free when handles are recycled.
- **Bounds checking**: Reference accesses validate indices/offsets are within bounds.

The cost of these checks would need to be weighed against the safety benefit. They could
potentially be disabled in production once the implementation is mature.

##### Memory Region Isolation

Transactions should only access:
- Their own local memory region.
- The block-level storage cache (read-only base values).
- Other transactions' frozen memory (if using freeze-on-finish sharing).

Violations would indicate serious bugs in the interpreter.

##### GC Safety

If using a compacting GC:
- **Design A**: All pointers must be updated after memory moves. Missing a pointer could
  lead to dangling references.
- **Design B**: Only handle table entries need updating. References via handle IDs remain
  valid as long as the handle is alive.

Design B may provide stronger GC safety guarantees since the indirection layer isolates
application code from memory movement.

##### Type Safety

With flat memory representation, values are raw bytes interpreted according to type
information. The runtime should ensure values are accessed with the correct type.

TBD: Anything the memory manager can do to help mitigate this risk?

### 5. Runtime Instruction Set
TBA

### 6. The Interpreter Loop
TBA

### 7. Extension Points
TBA: Native Function Interfaces, Gas Metering, Runtime Instrumentation Interfaces
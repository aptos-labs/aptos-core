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

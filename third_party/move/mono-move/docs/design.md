# MonoMove: Design Document

MonoMove is a Move Virtual Machine (VM) designed for performance and safety. At its core, it relies on monomorphization in order to handle Move generics and maximize efficiency. MonoMove design follows the following principles:

1. **Stateless VM**: VM does not store any long-living context. For execution, VM requires an external local (per-transaction) and global contexts.
2. **Performance Built-in by Design**: Value system uses a flat memory representation. Hot execution paths have minimal pointer chasing: hot data is tightly packed, cold data is separated. Execution operates on monomorphized basic blocks or fully-monomorphized functions. Gas charges are aggregated per basic block, for better efficiency.
3. **Long-Living Caches**: Code-derived data (types, instantiated types, monomorphized functions, etc.) is cached globally and is shared between threads. Allocations of the values used by the VM are managed by per-transaction (e.g., if VM created a value) or global context (e.g., if VM borrowed a resource from storage). Values use flat-memory representation that is cached globally to avoid BCS-to-flat conversions.
4. **Safety Built-in by Design**: Additional runtime reference or type checks are built-in. Metering is a first-class citizen.

## **Execution Model**

Throughout the document, the following execution model is used.

1. There is a single executor instance that runs blocks (validator nodes) or chunks (state syncing nodes) of transactions. There is only parallelism within each block, but any two distinct blocks are sequential.
2. For each transaction block, a Block-STM instance is created to run transactions in the block in parallel. Block-STM is using a fixed number of workers that execute different tasks (executing transactions speculatively, validating execution, running post-processing hooks for committed transactions).
3. Every transaction execution is an invocation of Aptos VM (wrapper around Move VM). Aptos VM dispatches calls to Move VM to execute specified functions or scripts.

# **1. Global Execution Context**

Throughout transaction block execution, a global context is owned by executor that stores long-living caches for code-related data and configs. Memory allocations for these caches are managed in epoch-based reclamation style, so that garbage collection (GC) only runs between transaction blocks. With this design, only concurrent allocations to the cache need to be supported, but not deallocations.

<aside>
💡

**Limits need to enforce caches cannot get full within a block.**

</aside>

### **Rationale**

1. Concurrent deallocation is a hard problem. It is difficult to ensure data is not deallocated if it is in use (efficiently).
2. For smart contract code, deallocations are rarely needed. Either code-derived data can be indefinitely persisted (e.g., interning strings), or becomes dead seldomly (on module upgrade).

### **Data Structures**

The global context is a 2-phase state machine that can either be in **Execution** or **Maintenance** states:

1. **Execution**: multiple workers can hold allocation and read data concurrently.
2. **Maintenance**: no workers allowed; context is checked if GC is needed, data may be deallocated.

To enforce these states at compile time, a guard pattern is used. There are two distinct RAII guards:

- **`ExecutionGuard`**: obtained by workers during block execution. Holding this guard means it is safe to hand out references tied to the guard's lifetime, and it is legal to allocate new cache entries through the guard.
- **`MaintenanceGuard`**: obtained by the single-threaded executor *between blocks*. While held, no new `ExecutionGuard` can be created. This provides a clear "border" when deallocation finishes.

```jsx
    End of block i                                                Start of block i+1
  -----------------------+                                      +-----------------------
    Execution:           |      Maintenance:                    | Execution:
                         |      (between blocks)                |
    - workers hold       |                                      | - workers acquire
      &'a ExecutionGuard |      - executor holds                |   &'a ExecutionGuard
    - workers may read   |        &'a mut MaintenanceGuard      |   again
      ExecutionGuard     |      - GC runs to check limits       |
      and allocate data  |        and deallocate data           |
    - lifetime of read   |      - caches may be reset           |
      data is also 'a    |      - promotion of cold / hot       |
                         |        data between tiers            |
  -----------------------+                                      +-----------------------
```

**Implementation Sketch**

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
      // If already in maintenance mode or there are still executions, return None.
      // Otherwise move to maintenance mode and return the guard.
      ...
   }
}
```

# **2. Global Identifiers**

Module address-name pairs, function/struct identifiers, and fully-instantiated types (and type lists) are interned using stable pointers to allocated data. Interning is managed by the global context, which owns the interner tables. Interning is an executor-only implementation detail with the following properties:

1. Interned data is never written to storage and is never included in transaction outputs.
2. Pointer addresses for the same data can vary across nodes and process runs.
3. Interned data is not invalidated by module upgrades.
4. `ExecutionGuard` can read from interner tables and can allocate more data. `MaintenanceGuard` checks allocation limits at transaction block boundaries so that allocation always succeeds during block execution time. `ExecutionGuard` returns `&'a T` references (not owned IDs) with lifetimes tied to the guard, so Rust enforces at compile-time that these references cannot be stored in long-lived state that outlives block execution (the safety requirement). The pointer-backed representation also allows storing additional cached data behind `T`.
5. Only `MaintenanceGuard` can deallocate interned data. For example, this guard can reset interner tables at transaction block boundaries to reclaim memory. A reset may invalidate **all** dependent interner tables and loaded code.

### **2.1 String Identifiers**

All string data (module names, function names, struct names) is interned. Data structures owned by global context store raw pointers (`NonNull<str>`). External APIs do not see these pointers directly.

### **2.2 Module Identifiers**

Identifiers for modules are interned as `ExecutableId`s. When interning, the module name is interned using the string interner first. Then, `ExecutableId` is created and allocated in the module interning table and the reference to the allocated data is returned.

```rust
// Intentionally non-`Copy` and non-`Clone`.
// Uses references tied to `ExecutionGuard`.
pub struct ExecutableId {
   address: AccountAddress,
   name: NonNull<str>,
   // ... cached data can be stored here ...
};

impl ExecutableId {
   pub fn address(&self) -> &AccountAddress { .. }
   pub fn name(&self) -> &str { .. }
}

impl ExecutionGuard<'_> {
   // Note: also have APIs to intern address-name pair directly.
   pub fn intern_module_id<'a>(
      &'a self,
      id: &language_storage::ModuleId,
   ) -> &'a ExecutableId {
      // On cache miss, creates `ExecutableId`, allocates it, and
      // returns a reference to it.
      ...
   }
}
```

Interning can happen in 2 scenarios:

1. When a module is loaded for the first time (or a script is loaded), all addresses, and module names are interned.
2. When transaction payload is executed in Aptos VM, entry function module identifier is interned. As a result, even if modules are cached, at least 1 lookup is done per user transaction.

This design allows to obtain module address or name directly from the `ExecutableId` reference, e.g., when setting the location of an error or getting debug information context-free. `ExecutableIdData` can also store cached information about the module. For example, when publishing modules, gas is charged proportionally to storage key sizes. To speed up the computation, the key size can be cached in `ExecutableIdData`.

### **2.3 Fully-Instantiated Types**

Fully-instantiated types and type lists are interned as well.

```rust
/// Represents any type. Non-`Clone` and non-`Copy`.
pub struct Type(TypeImpl)

enum TypeImpl {
   U8,
   ..
   Signer,

   Vector { elem: NonNull<Type> },
   Struct { id: NonNull<StructId> },
   Function {
      args: TypeListId,
      results: TypeListId,
      abilities: AbilitySet,
   },

   Ref { to: NonNull<Type> },
   RefMut { to: NonNull<Type> },
}

pub struct TypeList(*const [NonNull<Type>]);

impl TypeList {
   // Safe because guard guarantees the lifetime of list.
   pub fn get_unchecked<'a>(&'a self, i: usize) -> &'a Type {
      ...
   }
}

impl ExecutionGuard<'_> {
   pub fn intern_type<'a>(&'a self, tok: &SignatureToken) -> &'a Type {
      // On cache miss, allocate canonical node, then unsafe cast.
      ...
   }

   pub fn intern_type_list<'a>(&'a self, toks: &[SignatureToken]) -> &'a TypeList {
      // On cache miss, allocate canonical node, then unsafe cast.
      ...
   }
}
```

All fully-instantiated signature tokens are converted to a cononical `Type` representation via `ExecutionGuard`. All primitive types such as `u64`, `address` are pre-defined static values and are not stored in the interning table. This makes checks like "is this type a vector of `u8`" cheaper (no extra pointer dereference to get vector element type).

```rust
static U8_TYPE: Type = Type(TypeImpl::U8);
```

Like with execution IDs, `ExecutionGuard` owns all types and provides the only way to obtain type reference. This design makes it impoossible for types to leak across execution boundaries.

Every list of types (e.g., type argument list) is also interned as `TypeList` in a seprate interner table. It stores a pointer to list allocation which stores pointers to types in type interner table. Only `ExecutionGuard` can create and give out references to type lists. As a result,it is also safe for `TypeList` to provides a safe Rust API to obtain a reference to a type at specified index. Empty type list cam be set statically to null to avoid empty list allocations, prevent lookups, etc.

```rust
static EMPTY_TYPE_LIST: TypeList = TypeList(core::ptr::null());
```

Pointer-based design has the following benefits:

1. Types are uniquely identified by pointer addresses and are still compact (pointer-sized).
2. Structural information is preserved. For example, for runtime checks one can obtain element type of a vector via pointer dereference (no extra lookups or synchronization).
3. Data behind the pointer can cache more information. For example, type abilities can be computed during interning.

Note that pointer-based approach still does not eliminate possibility of "type confusion" entirely. If there is a bug in interner, pointer equality may be broken and lead to structurally same types being different, etc.

### **2.4 Struct and Function Identifiers**

Structs and functions are uniquely identified by IDs obtained via interning table.

Like `ExecutionId`s, function and struct IDs are obtained:

1. When a module is loaded for the first time (or a script is loaded). All data in the module is interned and the loaded module is cached.
2. When transaction payload is executed in Aptos VM, entry function identifiers and type argument tags are interned:
    - Entry function payload arguments are collapsed to do 1 lookup in cache for `NonNull<FunctionId>`.
    - On a miss, intern type arguments and generic function Id.
    - Worst-case number of lookups is still 5+ (because of type tags). As an optimization, composite hashes can be pre-computed when decoding the payload (when in validaotr's mempool).

```rust
/// Represents generic function ID without type argument list. Can
/// only be derived from struct or function IDs.
pub struct GenericFunctionId {
   // Pointer to interned module ID data.
   executable_id: NonNull<ExecutableId>,
   // Pointer to interned string data.
   name: NonNull<str>
}

/// Represents a struct identifier: module ID, struct name ID and
/// type argument list ID.
pub struct FunctionId {
   id: NonNull<GenericFunctionId>,
   ty_args: TypeList,
};

impl ExecutionGuard<'_> {
   pub fn intern_function_id<'a>(&'a self, ...) -> &'a FunctionId {
      // On cache miss, allocate canonical node, then unsafe cast.
      ...
   }
}

// Same as above.
pub struct GenericStructId { .. }
pub struct StructId { .. };
```

`ExecutionGuard` gives out references to all these identifiers. As a result, keys are still compact (pointer-size) integers. At the same time, the structural data can be obtained through pointer dereferences (e.g., for debugging).

# **3. Executables**

### **Executables**

When a module or a script are loaded, verified `CompiledModule` and `CompiledScript` are converted into `Executable`. Executable stores:

1. Monomorphized function and struct definitions. These include non-generics as well as fully-monomorphized generics.
2. Generic function and struct definitions (to be monomorphized lazily at runtime).
3. A constant pool for large non-inlined constants (e.g., vectors).
4. A bump-allocated arena where all executable data is stored.

During load-time, all data structures (e.g., Move bytecode, constants, types) are allocated in the arena, and raw pointers are used. Executable can give out references to this data with the lifetime bounded by the lifetime of the executable (via unsafe Rust). This is safe as it is guaranteed that no data is re-allocated when executing transactions - only new allocations can be added due to monomorphization.

When executable is dropped, memory used by maps and other data structures storing pointers to arena is freed. Then, memory from arena is deallocated in constant-time. *Invariant: executable is never dropped during the execution of a transaction block.* This design ensures raw pointers are stable, and simplifies memory management and concurrency.

```rust
struct Executable {
   // Arena for all allocations. Lock is only taken when allocating a NEW
   // instantiation during runtime monomorphization. Note that this can be
   // optimized if needed.
   arena: Mutex<Bump>,

   // Non-generic (or monomorphized) caches for function definitions. Read-only
   // during block execution, but instantiations can be added here if needed
   // (via some synchronization or at block-boundaries). Stores both regular
   // and native functions.
   functions: HashMap<FunctionId, NonNull<Function>>,
  
   // TODO: generic functions
   // TODO: constants
   // TODO: structs
}
```

A non-generic or monomorphized function is represented via `Function` struct. The runtime's current `Function` representation (in `mono-move-core`) is a post-monomorphization, type-erased form focused on execution:

```rust
pub struct Function {
   pub name: GlobalArenaPtr<str>,
   pub code: Vec<MicroOp>,
   /// Size of the argument region at the start of the frame.
   pub args_size: usize,
   /// Size of the arguments + locals region.
   pub args_and_locals_size: usize,
   /// Total frame footprint including metadata and callee slots.
   pub extended_frame_size: usize,
   /// Whether the runtime must zero-init the region beyond args on frame creation.
   pub zero_frame: bool,
   /// Frame byte-offsets of slots that may hold heap pointers (GC roots).
   pub pointer_offsets: Vec<FrameOffset>,
}
```

Key points:

- **Type-erased**: After monomorphization, type signatures (`param_types`, `return_types`) are no longer needed at runtime. All type information is baked into concrete micro-op operand sizes and offsets.
- **`pointer_offsets` for GC**: Each function declares which frame offsets may hold heap pointers. The GC uses these as root sets when scanning the call stack — no per-PC stack maps are needed (see `docs/heap_and_gc.md`). When `zero_frame` is true, the runtime zeroes non-argument slots on frame creation so pointer slots start as null.
- **Frame sizing**: `args_size`, `args_and_locals_size`, and `extended_frame_size` define the frame layout precisely, including a callee arg/return region at the end for non-leaf functions.

The full `Function` struct will eventually also include:

1. Visibility of this function (enum): private, package, public. This information is *only needed for runtime checks*.
2. Attributes for this function, stored as a bitset:
    - If this function is an entry function.
    - If this function is persistent.
    - If this function has re-entrancy module lock.
    - If this function is trusted (resides at special address).
    
    Any future attributes are part of this set. Note that non-private struct APIs are translated to execution instructions and are not calls.

Instructions that call non-generic functions are set at load- or execution- time in the following way:

1. Every instruction that calls to function *local to this module* embeds the pointer to this function at load-time:
    
    ```rust
    Instruction::CallLocal {
       ptr: NonNull<Function>,
    },
    ```
    
    This is safe in case of module upgrades: the caller is upgraded together with the callee. The overhead of function dispatch is 1 memory load.
    
2. Every instruction that calls to function *external to this Move module* embeds the "link" to the function (indirection layer):
    
    ```rust
    Instruction::CallExternal {
       link: *const FunctionLink,
       // Index into side table that stores additional metadata for this call
       // and which is not used on the hot path.
       metadata_idx: u16,
    },
    ```
    
    where the link stores the actual pointer to the function and the version of the executable where this function is defined.
    
    ```rust
    struct FunctionLink {
       // Function definition, can be null if not yet set.
       ptr: AtomicPtr<Function>,
    }
    ```
    
    This link allows to update the function pointer to the newest version after module upgrade, without any changes to the caller's code. The algorithm works as follows:
    
    - When executable is loaded for the first time, a `FunctionLink` is created for a specified function ID with null `ptr` value. The link is stored in a map stored in global context. If link already exists in context, it is embedded in the call.
    - During execution, the following is checked. If `ptr` is null, it means the function was not yet cached. Based on the `idx` to side-table, executable is loaded, function pointer extracted and link sets to point to it. If `ptr` is not null, only the executable ID is recorded in the read-set (note: for some additional bookkeeping, might be a full executable load). and the call dispatches to the `ptr`. Upgrade implementation guarantees that the executable points to the newest function.
    - During upgrade, for every executable, for every function the corresponding links are set to null. Because upgrade happens at Block-STM commit-time, linking to older versions is not needed. Alternative to this approach is storing version in the link so that the caller checks it to invalidate. Because upgrades are rare, doing more work on upgrade is preferred.
        
        NOTE: For replay during post-commit hook, links **cannot** be used because they store most recent code versions. Hence, replay needs to always run on top of its executable read-set and resolve calls via `idx` to fetch function from that version. This can be mitigated by having a flag in the link to specify if it was ever upgraded (if not, link is safe to use during replay).
        
    
    The overheads of this approach at execution are 2 loads and additional executable look-up (to record in read-set: unavoidable).
    
    An alternative is to store `AtomicPtr<(u32, Function)>` in the caller (initially set to null). During resolution, the pointer of the callee is saved along with the executable version in the instruction (via atomic swap). However, then every function call needs to check the version against the most recent read, and upgraded code is made visible only for callers that executed (on-demand re-linking). This approach has slightly lower overhead of only 1 load, but it is not clear saving 1 extra load is worth it.
    

In order to avoid 2 loads on every function call, the followng optimization is possible (which keeps upgradability correct).

(1) When a package of modules is published, during Block-STM commit time, all executables are directly linked to each other before being added to cache. This is safe to do because modules cannot be removed from packages.

(2) When modules are pre-fetched into executable cache (e.g., framework to avoid cold starts), this is done in package granularity. All calls within the same package are linked to each other.

With (1) and (2), framework code never uses links within the same package. For other modules loads, in order to do this optimization, information needs to be obtained if callee belongs to the same package. This can be done by storing package information in `ModuleHandle` in file format or via "shallow" loading of immediate dependencies. Then, instruction like this can be used:

```rust
Instruction::CallLocal {
   // Note: can be null, because dependencies are resolved lazily.
   ptr: AtomicPtr<Function>,
},
```

It is also worth mentioning the impact of this design on any Block-STM's post-commit replays (for asynchronous runtime checks). Because linking eagerly changes to newer versions of the code, replay, if done naively, can also link to version not from the original execution state but to the newer version. Hence, for cross-module (or cross-package, if optimization is used) replay, calls need to be resolved via: 1) fetching the module from read-set (correct version), 2) lookup of the `Function` via map.

### **Generic Types**

In the file format, generics are represented as `TypeParam(u16)` variant in `SignatureToken`. MonoMove still needs to manage generics. It uses a flattened format to avoid pointer chasing and ensure memory is managed more efficiently (a compressed tree where instantiated leaves are `TypeId`s and only truly generic leaves are kept as type parameters). This efficient representation is needed because some instantiations may happen at runtime and are not cached.

1. Primitives: 1 byte header.
2. Vectors or references: 1 byte header, followed by element encoding.
3. Structs: 1 byte header, followed by struct ID (8 bytes), number of type arguments, encoded type arguments.
4. Functions: 1 byte header, followed by number of arguments (1 byte), number of returns (1 byte), abilities (1 byte) then encoded arguments and returns.
5. Resolved type (`TypeId`): 1 byte header, 4-byte payload.
6. Type parameter: 1 byte header, followed by u16 index.

```rust
pub struct GenericType {
   // This data is being allocated in executable's arena (see below).
   bytes: NonNull<[u8]>,
}

impl GlobalExecutionContextGuard<'_> {
   fn instantiate_generic_type<'a>(
      &'a self,
      ty: &GenericType,
      ty_args: TypeList<'a>,
   ) -> Type<'a> {
      // Walks the type tokens, interning what is needed.
      // We can also use a cache, because each ty_args list has a unique ID
      // so we can have a cache for each generic type.
      ...
   }
}
```

# **4. Execution Engine**

## 4.1 Runtime Instruction Set

The runtime executes **micro-ops** — a low-level, flat instruction set produced by the specializer from Move bytecode after monomorphization and destackification. Micro-ops operate on frame-relative byte offsets rather than a virtual operand stack. Categories include arithmetic, data movement, control flow (fused compare-and-branch), call/return, vector operations, heap object operations, reference operations, and gas metering.

The instruction set design (principles, addressing modes, naming conventions, open questions) is documented alongside the code in `mono-move-core/src/instruction/mod.rs`.

## 4.2 Stack Memory Model & Calling Convention

The VM uses a unified linear stack where frame data and frame metadata coexist in one contiguous buffer. Each frame contains: arguments (written by caller), locals, 24-byte metadata (`saved_pc`, `saved_fp`, `saved_func_ptr`), and callee arg/return slots. The frame pointer (`fp`) points past the metadata, so operand access is a single `fp + offset`.

Call dispatch writes the 24-byte metadata at the end of the caller’s frame, places arguments for the callee, and advances `fp`. Return reads the metadata at `fp - 24` to restore the caller’s state. Local access uses compile-time byte offsets — no index lookups.

See `docs/stack_and_calling_convention.md` for the full design: frame layout diagram, call/return protocol, unified vs. separate stack trade-offs, per-function calling convention customization, and security considerations (stack overflow, control flow hijacking, uninitialized memory, etc.).

## 4.3 Heap and Garbage Collection

The runtime uses a two-level memory architecture: a block-level manager that owns the storage cache and hands out per-transaction memory subspaces, and a transaction-level bump allocator paired with Cheney's copying GC. The common path is pure bump allocation with zero overhead; the collector is a safety net for when the heap fills.

See `docs/heap_and_gc.md` for the full design: block/transaction memory management, global value sharing, per-block memory limits, GC design space analysis (four approaches evaluated), and memory safety considerations.

## 4.4 Value Representation

Values are represented flat in memory. Primitives are N bytes flat. Structs and enums support both inline and heap representations. Vectors are heap-allocated with an 8-byte header, length, and element data. References are 16-byte fat pointers `(base_ptr, byte_offset)`.

See `docs/value_representation.md` for the full design: memory layouts with diagrams, inline vs. heap trade-offs for structs and enums, vector layout and growth semantics, fat pointer mechanics, and reference alternatives.

# 5. Native Functions

Native functions are first-class citizens in MonoMove — they have direct access to VM internals and follow the same calling convention as Move functions.

See `docs/native_functions.md` for the full design: calling convention, error handling, gas metering strategies, generics/monomorphization, security considerations, and distributed ownership concerns.

# 6. VM Security & Correctness

See `docs/vm_security_and_correctness.md` for the full set of security principles, vulnerability categories, and key invariants (arithmetic safety, type/memory safety, gas metering, boundedness, determinism, cache consistency, reference aliasing, panic safety, etc.).

# 7. Extension Points

TBA: Gas Metering, Gas Profiling, Runtime Instrumentation Interfaces

# 8. Translations within the VM
TBA

# 9. Gas metering
TBA
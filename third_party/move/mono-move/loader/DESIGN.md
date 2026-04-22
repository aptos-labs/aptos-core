# MonoMove Loader Design

## 1. Context

The MonoMove runtime uses a long-living module cache that satisfies the following requirements.

- Stores executable IR and lowered code to avoid repeated deserialization, verification and translation.
  Note that executable IR already uses type pointers to the long-living interned type cache.

- Supports upgrades: 1) within same block, 2) between speculative block trees (with Zaptos, blocks with published code may not be committed immediately).
  Because of speculation and concurrent upgrades, newer or speculative code can co-exist with older versions at the same time.

- Does not evict modules during execution of transactions (only at safe points when no execution happens).

When a single transaction executes, it may "load" modules into its local read-set.
The read-set is used to cache a consistent local view of the cache for transaction.
This is needed to avoid cases where some transaction upgrades module `A`, and other transaction ends up seeing both old and new versions at the same time.

Loading a module can either be a hit in long-living module cache or a miss.
On a miss, loader fetches the module from storage, deserializes it, verifies, translates to execution IR, and adds it to the long-living cache.
To cover for the worst case, every module load has to be charged gas.
Importantly, the charging must be **deterministic** irrespective of the cache state that may be different on different validators due to speculation.

Transaction may need to load modules for multiple reasons.

1. When executing a cross-module call.
2. When lowering non-generic function IR to micro-ops (struct layout information such as size, field offsets is needed).
3. When monomorphizing generic function IR to micro-ops (same as for non-generics, layout information is needed).
4. When resolving the function behind the closure (e.g., closure was deserialized from storage and have not been loaded yet).
5. When walking enum values (layouts of variants need to be known, which may result in module loading).

Note that (2) and (3) also cover cases like 1) deserializing data on global storage access, 2) walking VM values based on layouts (serialization, GC).

## 2. Challenges

1. **Loading costs are deterministic.**
   Two executions of the same transaction must charge the same cost regardless of what the long-living cache stores.
   As a result, transactions need to enforce cache hit has the same gas charging semantics as the cache miss.

2. **Loading costs are charged on first access.**
   Multiple accesses to same module `A` within same transaction should be charged once to cover its loading costs.
   The charge should happen on the first access to such a module `A`.
   While repeated accessed can be overcharged by a small factor, it can easily become too restrictive for contracts.
   If there is a loop that calls into module `A`, gas must not be charged for `A` on every iteration.
   Similarly, if `A` is a hot library called from many places within a transaction, charging for all call sites would be prohibitively expensive for call-heavy Move programs.

3. **Layout costs depend on multiple modules.**
   In order to lower a function, specializer needs to know layouts of struct types.
   Layout information is aggregated over multiple modules, so its cost depends on all of them.
   This problem is tightly coupled with *charge on first access*.
   For example, if transaction has loaded and charged for modules `A`, `B`, and `C`, it cannot charge aggregated cost for layout that uses `A`, `B`, and `D`.
   Instead, only extra `D` has to be charged.
   As a result, semantics of gas charging may impact how efficient loading is.

4. **Modules are upgradable.**
   On upgrade, modules may change their code and size, more structs may be added, enums may gain new variants.
   Because the gas charge is proportional to module size in bytes, transactions that loaded upgraded module must be invalidated and re-executed by Block-STM.
   Again, semantics of gas charging may impact validation and upgrade logic.

## 3. Proposed Design

### Pull Validation for Modules

To handle module upgrades, Block-STM may use push- or pull-based validation.
For pull, transactions eagerly check freshness of loaded modules to detect upgrades.
For push, on code upgrade, all cache entries that transitively depend on upgraded module need to be invalidated.
Additionally, readers of that module or any derived cache entries must be invalidated.
Derived cache entries can include struct layouts (constructed from multiple modules), sums of module sizes, etc.

MonoMove runtime uses pull-based invalidation.
It makes implementation simpler and more efficient.
There are two main reasons why pull approach wins.

Firstly, with Zaptos, push invalidation may result in adding many more entries to the cache.
Consider an example below, where there is an already executed fork `B1-B2`, and currently executing fork `B2`.
If `B2` republishes module `N`, with push-based approach any code that depend on `N` needs to be duplicated on both `B2` and `B1-B3` branch.

```
Speculative execution tree:

    B0 (committed)
    ├── B1 (speculative)
    │   └── B3 (speculative, builds on B1)
    └── B2 (speculative, publishes new version of module N)
```

Secondly, the existing gas metering model for structures like layouts requires filtering out modules that are already charged.
For example, consider a transaction that has `A`, `B`, and `C` charged.
Suppose that is accesses layout information from cache that depends on `A`, `B`, and `D`.
Because of metering, it needs to only charge for non-visited subset of the modules.
But, computing this subset is effectively a pull-based validation.

### Loading policies

When loading a module, it might not be possible to lower its non-generic functions to micro-ops because layouts from other modules are needed.
For example, consider the following collections of Move modules.
If module `a` is loaded, `f1` cannot be lowered until layout of `A1` is known.
That in turn requires modules `b` and `c`.

```move
// Example 1

module 0x23::a {
   struct A1 { x: 0x23::b::B1, y: u64 }
   struct A2 { x: u64, y: u64 }
   public fun f1(x: &A1): u64 { x.y }
   public fun f2(x: &A2): u64 { x.y }
}

module 0x23::b {
   struct B1 { x: 0x23::c::C, y: u64 }
   struct B2 { x: 0x23::d::D, y: u64 }
   public fun g(x: &B2): u64 { x.y }

}

module 0x23::c {
   struct C { x: u64 }
}

module 0x23::d {
   struct D { x: bool }
}
```

In this design, we propose three loading policies.
Policies try to balance between the two:

1. Load and charge more modules upfront (more work), no loading / charging when resolving calls (less work).
2. Load and charge a single module upfront (less work), loading / charging when resolving calls (more work).

#### Lazy Loading with Lazy Lowering (LL)

A single module is loaded at a time and any cross-module work is deferred to execution.
In Example 1, when loading `a`, only `f2` is lowered because it does not depend on any external modules.
Lowering of `f1` is deferred until execution.
If `f1` is called, modules `b` and `c` are loaded at the same time to compute layout information of `A1` (`A2` is local, no modules to load).

#### Lazy Loading with Eager Lowering (EL)

The module is loaded together with all the modules needed to compute layouts for any struct the target module uses.
In Example 1, when loading `a`, both `f1` and `f2` are lowered.
Additionally, modules `b` and `c` are loaded to compute layout information of `A1`.

An important observation is that the set of loaded modules is not a superset of modules loaded for dependencies.
For example, when loading `b`, modules `c` and `d` are loaded.

**Implementation detail.**
Because it is not possible to distinguish imported enums from structs, EL also include modules where used enums are defined.
In Example 2, when loading `a`, it also needs modules `b` and `c` to lower `f`.
While `c` is not strictly needed for lowering (field `y` has a known size), it is not possible to know if `y` is a struct or an enum until `c` is loaded.

```move
// Example 2

module 0x23::a {
    struct A1 { x: 0x23::b::B, y: 0x23::c::C, z: u64 }
    public fun f(x: &A1): u64 { x.z }
}

module 0x23::b {
    struct B { x: u64 }
}

module 0x23::c {
    enum C {
        V1 { x: u64 },
        V2 { x: 0x23::d::D },
    }
}

module 0x34::d {
    struct D { x: u8 }
}
```

#### Package Loading (PL)

The module is loaded together with all other modules that belong to the same package.
Essentially, package becomes a single atomic unit for loading.

In Example 1, assume that `a` and `b` are in the same package.
Then, only `f2` is lowered because layouts for other functions are not available.
However, if `a`, `b` and `c` are in the same package, both `f1` and `f2` can be lowered.

### Loading framework

Framework code is treated separately.
While framework can upgrade, these upgrades are controlled and rare (around every 2 weeks).
At the same time, framework is widely used by ecosystem modules, with the majority of cross-module calls resolving to the framework.
Hence, it is a strong requirement to ensure such calls dispatch fast, ideally via direct pointers.

In this design, a single version of framework is always cached, its functions lowered and layouts available.
In addition, gas is never charge for loading of any framework module.
As a result, any module using framework dependencies can use them freely for its own lowering.
In Example 3, when module `a` below is loaded, `f` is lowered eagerly because string layout is always available.

```move
// Example 3

module 0x23::a {
    struct A { x: 0x1::std::String, y: u64 }
    public fun f(x: &A): u64 { x.y }
}
```

In order to ensure single framework version under Zaptos, every framework upgrade needs to trigger a reconfiguration.
Then, the existing behavior in Zaptos is enough to keep the cache safe because blocks cannot be added on top of branches with reconfigurations.

```
B0 (committed, old framework)
├── B1 (publishes framework; has_reconfiguration = true)
│   └── B2 → B3 → B4        ← parent chain has reconfig, empty blocks
└── B1' (no reconfig)
    └── B2'                  ← parent chain has no reconfig, executes normally
```

### Mandatory Sets

All three loading policies can be unified under the same concept: **mandatory sets (MS)**.
Mandatory set is a set of modules of fixed size that has to be loaded on the first module access or during the first function call.
In Example 1, under LL policy `MS(a) = {a}`, `MS(f1) = {b, c}`, and `MS(f2) = {}`.
Under EL policy, `MS(a) = {a, b, c}`, `MS(f1) = {}`, and `MS(f2) = {}`.
Under PL policy, mandatory set is all modules in the package.

In order to make metering deterministic:

1. When module `m` is loaded, gas is charged for all modules in `MS(m)` not in current read-set.
2. When function `f` is called (after its module has been loaded), gas is charged for all modules in `MS(f)` not in current read-set.
3. All charged modules are recorded in a per-transaction read-set.

Here, (2) also handles metering for generic calls because type argument layouts are in mandatory set of the caller.

The goal of mandatory sets is to enforce determinism under a particular loading policy.
Depending on the policy, module loading may be more or less expensive, making function call resolution less or more expensive.
The best policy is workload dependent.
Interpreter can also use inline caches to avoid repeated set intersection checks, if they are expensive.

The important property of mandatory sets is that they cannot change size because of upgrade.
That is, if `MS(a) = {a, b, c}`, it is not possible that it becomes `{a, b}` or `{a, b, c, d}` after some downstream upgrade.
This makes them particularly attractive because they can be cached per each module instance.
For example, if module `a` is cached in long-living cache, there is no need to re-traverse its dependencies in some complex way to simulate EL loading and metering.
It is sufficient to check modules in `MS(a)`, which can be implemented lock-free.

#### Implementation of Mandatory Sets

For every module, long-living cache stores a pointer to the slot.
Slot serves as a versioning primitive (for future Zaptos support).

```rust
pub struct Slot<T> {
    /// Committed baseline value (storage).
    base: AtomicPtr<T>,        
    /// If true, need to resolve via overlay + pending.
    stale: AtomicBool,
    // Writes from all executed blocks in the speculative tree, compressed to at most 1 version per block.
    overlay: Mutex<SmallVec<[(BlockId, *const T); 2]>>,
    // Current block writes in-flight.
    pending: Mutex<SmallVec<[(TxnIdx, *const T); 1]>>, 
}
```

Mandatory sets are implemented as a slice of pointers to `Slot` instances in the cache.
Slots are stable and never claimed by garbage collection unless the cache is flushed.

### Handling Cache Misses for Loading Policies

When loader encounters a cache miss, it needs to load the mandatory set of the target module.
These loads can also be cache misses and have to be inserted into long-living cache.
In order to enforce correctness, loader splits "loading", "linking" and "insertion" into their own phases.

1. A set of cache misses is obtained (corresponding to the mandatory set).
   The modules can still be in compiled, file-format representation.
2. Modules are translated to execution IR.
   Non-generic functions are lowered when possible.
3. Modules are ordered in reversed topological order based on their dependencies.
   Then, they are inserted one by one into cache.
   This is critical for correctness under concurrent loads: it is possible that other thread may insert module into cache.
   The cache resolves the race returning the *canonical* pointer, which can be now linked against.
4. Other modules linked again the just inserted module. Once done, the insertion repeats.

Steps (3) and (4) allow to safely store direct pointers between modules.
Cache only needs to enforce GC does not evict modules before modules that point to them.

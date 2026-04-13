# Module Loading and Linking

This document covers how MonoMove loads modules from storage, charges gas for loading, handles cache invalidation on upgrades, and links loaded modules.
The core design decouples loading (I/O, gas, deterministic) from linking (in-memory, unmetered, best-effort optimization), and uses cell-based indirection for cross-module calls following the guard/inline-cache pattern from JIT compilers.

## 1. The Problem

Module loading and gas metering is one of the hardest problems in blockchain VM design.
There is no single solution that addresses all scenarios.
This section describes the core challenges.

### Non-deterministic cache across validators

The module cache differs across validator nodes because Block-STM executes transaction in non-deterministic way.
When accessing module `A`, it may be cached on one node but not another.
Because a cache miss is expensive (storage I/O, deserialization, verification), gas must be charged for module loading to ensure deterministic execution costs.

### Upgrades invalidate cached data

Move supports module upgrades.
When a module is upgraded, its size changes, its code changes, and any cached data derived from it (linked function pointers, struct layouts) becomes stale.
All loaded data (caches included) that depends on upgraded module must be invalidated.
Readers of the upgraded module or caches depending on the upgraded module must be invalidated.

### "Charge once" semantics

Modules can be overcharged but to a small factor.
If there is a loop that calls into module `B`, gas must not be charged for `B` on every iteration.
Similarly, if `B` is a hot library (e.g., `0x23::math`) called from many places within a transaction, charging for all call sites would be prohibitively expensive for call-heavy Move programs.

### Caching data defined over many modules

Caching data aggregated over multiple modules is a particularly problematic case.
For example, struct layouts.
Structs are defined across modules, and while struct definitions themselves cannot be upgraded (and thus invalidated), a layout cache miss can trigger loading of *all* modules used to define the struct's fields transitively.
If module `A` defines `struct S { field: B::T }` and `B::T` contains `C::U`, computing the layout of `S` requires loading modules `A`, `B`, and `C`.

### Speculative execution

Block-STM executes transactions speculatively and in parallel.
Zaptos extends this across blocks: speculative blocks form a tree (e.g., blocks B1 and B2 both build on B0).
If there is a publish in block B2, it cannot cause cache invalidation yet because some B3 may be executing on B0.
The global cache cannot be mutated during speculative execution.

```
Speculative execution tree:

    B0 (committed)
    ├── B1 (speculative)
    │   └── B3 (speculative, builds on B1)
    └── B2 (speculative, publishes module N)
        └── B4 (speculative, builds on B2)

B2's publish of N cannot invalidate the global cache
because B3 is executing on B0 -> B1 (doesn't see B2).
```

## 2. Invalidation: Push vs. Pull

When a module is upgraded, its upstream users must be invalidated.
There are two approaches.

### Push Invalidation

On upgrade, the writer eagerly finds and invalidates all users.
Walk the dependency graph, null out stale entries, mark readers for re-execution.

*Pros:*
- Cache is always clean.
  Readers never see stale data.
- Invalidation work is proportional to the upgrade event, not spread across reads.

*Cons:*
- Requires maintaining a reverse dependency graph (who depends on B?).
  For transitive dependencies this graph is large and expensive to maintain.
- Layout dependencies make push nearly useless (see below).
- Push invalidation does not work well under speculation with Zaptos (unless we de-optimize code during specultive upgrade for safety and then re-optimize back).
- On framework upgrade, the blast radius is enormous (can be mitiagted by special-casing framework, but similar risks exist for populat third-party modules).

### Pull Invalidation

Readers check freshness at access time.
Each cache entry stores a version or identity (e.g., module hash); the reader compares against the current value and reloads on mismatch.

*Pros:*
- No reverse dependency graph needed.
- Invalidation is lazy and proportional to actual use: if nobody reads module `A` after `B` is upgraded, no invalidation work happens.
- Works naturally with speculative execution: a re-executed transaction simply re-checks versions on access.
  No rollback needed.
- Layout dependencies are handled implicitly: pulling a layout pulls the modules it depends on.

*Cons:*
- Every cache access pays a freshness check.
- Stale entries linger until accessed.

### Why layouts kill Push invalidation if done naively

Layouts are the most problematic dependency type for *general* push invalidation.
Here is why.

One cannot simply cache the sum of module sizes used for a layout, because this leads to overcharging.
Consider layouts that use modules `A`, `B`, `C`, then a layout using `B`, `C`, and then a layout using just `C`.
Charging each layout's full module set would make the cost quadratic.
To charge correctly, you need a per-transaction set of already-charged modules.
But if you must read all constituent modules anyway to charge gas correctly, you have already implemented pull-based invalidation.

In other words: for layouts, the work needed to charge gas correctly (reading constituent modules, deduplicating charges) *is* the pull invalidation check.
General push gains nothing here.

However, push invalidation *does* work for layout **cost validation** specifically, because struct definitions are immutable and the reverse index is stable.
See Section 4 (Layout Costs) for the push-based layout cost validation strategy.

### Comparison

|                        | Push                            | Pull                                  |
| ---------------------- | ------------------------------- | ------------------------------------- |
| Dependency tracking    | Reverse graph (expensive)       | None (version check on access)        |
| Upgrade cost           | O(dependents), eager            | O(0) at upgrade, O(readers) lazy      |
| Read cost              | No freshness check              | Version check per access              |
| Layout handling        | Degenerates to pull (general)   | Natural fit (push viable for costs)   |
| Speculative execution  | Requires rollback on abort      | Re-execution re-checks, no rollback   |
| Framework upgrade      | Enormous blast radius           | Demand-driven, proportional to use    |

**Conclusion:** Pull invalidation seems the right choice for general module invalidation.
The layout dependency problem makes general push equivalent to pull in the worst case, while pull is simpler, more natural for speculative execution, and proportional to actual work.
The exception is layout cost validation, where push works well because the reverse index (module to layouts using it) is immutable (see Section 4).
This gives a hybrid: pull for general invalidation, push for layout / monomorphization costs.

## 3. Decoupling Loading from Linking

The core insight of MonoMove's module system is that **loading** and **linking** can be cleanly decoupled into two independent phases.

### Loading (Phase 1)

Loading is the I/O phase.
It fetches module bytes from storage (on cache miss) or validates cache entries (on cache hit).
Loading is:

- **Consensus-visible**: tracked by Block-STM in the transaction's read-set.
- **Gas-charged**: every module access incurs a gas cost to ensure deterministic execution costs regardless of cache state.
- **Deterministic**: all validators must agree on how much gas was charged.

The loader is responsible for getting modules into memory and ensuring gas is paid.

### Linking (Phase 2)

Linking is the optimization phase.
It takes loaded modules and resolves cross-module references: function pointers, struct layouts, type information.s
Linking is:

- **Consensus-invisible**: not tracked by Block-STM.
  Different validators may produce differently-optimized linked forms after loading depending on what is available in the cache (recall that the cache state may be different between nodes).
- **Unmetered**: no gas charged.
  Linking quality is an implementation detail.
- **A total function**: given a set of loaded modules, linking *always* produces a correct result.
  There is no failure mode.
  More modules available means better optimization (more references pre-resolved).
  Fewer modules means conservative fallback (lazy dispatch at call time).
  Correctness is unconditional.

### Pipeline

```
1. LOAD:   load(id) -> Vec<Executable>     // Fetch from storage or cache, charge gas
2. LINK:   link(loaded, cache) -> Linked   // Resolve references using loaded + cached modules
3. INSERT: cache.insert(linked)            // Cache optimized format
```

**Step 1** handles I/O and gas.
On cache hit, gas is charged and the cached executable is returned.
On cache miss, the module is fetched from storage, deserialized, and verified, then gas is charged.

**Step 2** receives cache misses from the loader and has read access to the existing global cache.
It resolves function pointers, computes layouts, and optimizes the executable (if possible).
For dependencies that happen to be in cache, the linker may resolve them eagerly (see Section 7: Optimistic Linking).

**Step 3** inserts the linked executables into the cache for future use.

```
Storage --> [LOADING] --> loaded modules --> [LINKING] --> linked form --> [CACHE]
              │                                 │
              │  gas-charged                    │  gas: not charged
              │  deterministic                  │  deterministic: not required
              │  Block-STM tracked              │  total function 
                                                │
                                       Global cache (opportunistic)
```

This separation is what makes the system work: gas is fully determined by the loading phase, while linking quality is a best-effort optimization that cannot affect correctness or consensus.
It also allows to feature gate loading only while evolvign linking unconditionally.

## 4. Gas Metering

### The Core Principle

Never charge gas for objects derived from modules (layouts, linked forms, monomorphized code) directly.
Instead, use a **proxy** which is cheap to validate and invalidate.
The module itself is the proxy: charge for the module, and derive everything else from it without additional gas.

### Charge-Once Semantics

Within a single transaction, each module is charged at most once.
The transaction maintains a per-transaction visited set (the read-set).
On module access:

- Not in read-set: charge gas, add to read-set.
- Already in read-set: skip charging.

This ensures that a loop calling `B::foo` 1000 times pays the loading cost for `B` once.

### Layout Costs

Struct layout computation is where gas metering gets subtle.
A layout miss can trigger loading of multiple modules transitively.
Charging for each constituent module via the read-set handles deduplication, but the cost formula for the layout itself needs care.

The proposed approach: charge layout cost via a compressed formula that depends on the modules used to construct it, minimizing cache hit overhead.

```
cost(layout L) = base_cost
               + c0 * len(modules_used(L))
               + c1 * max(m for m in modules_used(L))
```

This formula has a key property: the set of modules used for a layout is stable across upgrades because struct definitions are immutable in Move.
Only module *sizes* can change (a module can grow by adding functions).

### Validating Layout Cost: Push for Layouts

A naive approach -- "check that the max-size module hasn't changed on cache hit" -- has a determinism problem.
Consider: layout L uses modules A (size 30) and B (size 25), so `max = 30` (A).
If B is published with new size 35, node 1 (old B cached) computes `max = 30`, node 2 (new B) computes `max = 35`.
Different gas charges for the same layout -- consensus violation.

To validate correctly, we could load all constituent modules on every cache hit, but that defeats the purpose of caching.
Instead, we use **push invalidation specifically for layout costs**.
This works because the property that killed push in the general case (reverse deps change on upgrade) does not apply here: struct definitions are immutable, so the reverse index "which layouts include module M?" is fixed forever once computed.

When module M is published with a new size:

1. Look up the immutable reverse index: which cached layouts include M?
2. For each such layout, update M's cached size in place and recompute the max and cost.

```
M published with new size S:
    if S == M.old_size:
        skip                                // same size, formula unchanged
    else:
        for each layout L containing M:     // immutable reverse index, O(1) lookup
            L.sizes[M] = S                  // update one entry
            L.cached_max = max(L.sizes)     // O(fields)
            L.cost = base + c0 * len + c1 * L.cached_max
```

This is O(layouts_using_M) per publish, and each update is O(fields) which is small.
The reverse index never needs maintenance because struct fields cannot change.
If the module size did not change, the entire push is skipped (the formula produces the same result).

Layout cache entries are updated **in place** -- the entry is a stable, long-lived object.
Other cached data that depends on layout costs (e.g., monomorphized functions) can hold a pointer to the layout entry and always see the fresh cost without any additional push propagation.
See the monomorphization subsection below.

For speculation: the updated costs live in the block overlay.
On discard, dropped.
On commit, promoted to base.
Same pattern as function cells (Section 6).

This gives us a **hybrid invalidation strategy**: pull for general module invalidation, push for layout cost validation.
Each uses the approach that fits its dependency structure.

### Monomorphization Costs

A monomorphized function `foo<A::S>` (defined in module F, where `A::S` has fields from modules A, B, C, D) depends on three things:

1. **The function body** — from module F. Changes when F is upgraded.
2. **The layouts of concrete types** — spans modules A, B, C, D. Struct definitions are immutable, so the layout *shape* never changes, but module *sizes* change on upgrade, affecting the cost formula.
3. **Type substitution** — cost of creating new types and computing sizes.

The key insight: monomorphized functions **link directly to layout cache entries** they depend on.
Layout entries are push-maintained in place (see above).
The mono function reads the cost through its pointer -- always fresh, no additional push propagation needed.

```
mono foo<A::S> ---ptr---> LayoutCacheEntry { sizes, cached_max, cost }
                                    ^
                                    |
                          push-updated in place when module A published
```

The cost of a monomorphized function composes from already-maintained pieces:

```
cost(mono F<T>) = function_cost(F)                         // pull: F in read-set
                + layout_cost(T)                            // read from push-maintained entry
```

There is no reverse index from layouts to mono functions, and no push fanout beyond the layout level.
Push stops at layout entries.
Mono functions are just readers.

**Pull for bodies.**
Module F is in the transaction's read-set.
If F is upgraded, pull detects the version mismatch on next access.
All monomorphized specializations of functions in F are invalidated and re-generated on demand.

### Pre-Loading Transitive Dependencies

One could also pre-load transitive dependencies of a module (struct layout dependencies, fixed-size enum dependencies) as an optimization to ensure they are in cache for linking.
However, this has the same gas problem as layouts: how to charge without overcharging?
One approach is `max(module_size, transitive_dep_sizes)`, but this adds complexity for uncertain benefit.
This remains an open design choice.

### Framework Modules

Framework modules (`0x1::*`) receive special treatment:

- Always pre-loaded and pre-linked at load-time.
- Never included in a transaction's read-set for gas purposes.
- Always linked directly (no indirection, no lazy dispatch).
- On framework upgrade (governance proposal): full cache flush and reconfiguration.
  This is acceptable because framework upgrades are rare.

TBD: consider interaction with Zaptos.

## 5. Edge Tracking and Metering

An alternative to per-module tracking is tracking and metering *edges* (dependency relationships) rather than individual modules.

### The Idea

Instead of asking "which modules did this transaction load?", ask "which dependency edges did this transaction traverse?".
An edge is a pair `(caller_module, callee_module)` representing a cross-module call or type reference.

### Motivation

If each caller-callee pair has its own inline cache (e.g., each `CallExternal` site remembers its target), then the question "has `B` been charged?" becomes local per-edge.
There is no need for a shared per-transaction set check on every call.
The information is embedded in the edge itself.

### Pros

- Easier to track: per-edge state eliminates shared set lookups.
- Push invalidation may be easier: upgrade to `B` means "find all readers of edges pointing to `B`".
  Each read-set entry is an edge, so the scan is direct.
- Natural fit for inline caches: the edge *is* the cache entry.

### Cons

- Read-set size increases significantly (edges >> modules).
- Overcharging of popular libraries: every caller of `0x23::math` pays an edge cost, even though from an I/O perspective the module is loaded once.
  This overcharging does reflect the actual work per cross-module call (each call *does* perform extra work), but it penalizes call-heavy programs.
- The reverse-map problem: keeping track of upstream immediate dependencies of modules achieves the same as edge tracking for push invalidation, without requiring edges in the read-set.
  This undercuts the main advantage.

### Comparison

|                        | Per-module tracking              | Edge tracking                         |
| ---------------------- | -------------------------------- | ------------------------------------- |
| Granularity            | Coarse (whole module)            | Fine (per dependency)                 |
| Read-set size          | Small (unique modules)           | Large (unique edges)                  |
| Charge-once check      | Shared set lookup                | Local per-edge (inline cache)         |
| Push invalidation      | Reverse dependency graph needed  | Scan edges pointing to upgraded module|
| Overcharging risk      | Lower                            | Higher for hot libraries              |
| Block-STM fit          | Natural (modules are the unit)   | Needs mapping edges back to modules   |

**Status:** Node tracking is preferred for now due to simplicity and natural fit with Block-STM's read-set mechanism.

## 6. Cell-Based Linking

Cell-based linking is the core mechanism for cross-module function calls.
It follows the **guard/inline-cache optimization pattern** from JIT compilers: embed a pointer to the target, guard it with a version check, and fall back to slow resolution on mismatch.

### Cell Structure

For every cross-module call target `B::foo`, there is a cell that belongs to module `B`:

```rust
struct FunctionCell {
    /// Fast path: pointer to resolved function + version tag or hash.
    /// Null means "not yet resolved" or "invalidated".
    base: AtomicPtr<(Function, Version)>,

    /// Speculative overlay: maps speculative block + txn index version to function.
    /// Empty during committed execution. Used by Zaptos for speculative blocks that publish modules.
    overlay: SmallVec<(Version, *const Function)>,
}
```

### Fast Path (2 Hops)

On a cross-module call `A -> B::foo`:

1. **Hop 1**: Read `cell.base` pointer.
2. **Hop 2**: If non-null and overlay is empty, dereference to get `(Function, u32)`.
   Dispatch to `Function`.
   Extend the transaction's read-set with `(B, version)` for Block-STM validation.

This is comparable to virtual dispatch in OOP languages: one indirection to find the vtable, one to find the method.
Two hops is fast enough and only paid for cross-package non-framework calls.

### Slow Path

If the cell is null (not yet linked, or invalidated after upgrade) or the overlay is non-empty (speculative execution with a publish of `B` in a parent block):

1. Resolve `B::foo` from overlay (on miss, from cache or storage).
2. Charge gas for `B` (if not already in read-set).
3. Update `cell.base` with the resolved function pointer.
4. Dispatch.

### Upgrade Handling

When module `B` is upgraded (published):

- **Push-update the overlay** or **invalidate the base pointer**.
  This ensures that any in-flight call to `B::foo` atomically resolves to the correct version.
  The cell acts as the synchronization point.
- Block-STM detects the conflict: any transaction that read `(B, old_version)` via the cell's fast path is invalidated and re-executed.
- On re-execution, the transaction hits the slow path (cell was invalidated), resolves the new version, and proceeds.

### Speculative Execution

Under Zaptos, speculative blocks may publish modules without committing:

- The **base pointer** is never modified by speculative execution.
  It always reflects the last committed version.
- The **overlay** captures speculative versions.
  A call during speculative execution checks the overlay first, falls through to base if empty.
- On **speculative block commit**: overlay entries are promoted to base, overlay is cleared.
- On **speculative block discard**: overlay is dropped.
  Base is untouched.
  No rollback needed.

```
Committed state:     cell.base -> B::foo v3
                     cell.overlay = []

Speculative B2:      cell.base -> B::foo v3  (untouched)
                     cell.overlay = [(B2, B::foo v4)]

B2 commits:          cell.base -> B::foo v4  (promoted)
                     cell.overlay = []

B2 discarded:        cell.base -> B::foo v3  (untouched)
                     cell.overlay = []        (dropped)
```

### Why This Works

The cell pattern gives us the best of both worlds:

- **Fast common case**: 2 hops, no set lookup, no version comparison (Block-STM handles validation asynchronously).
- **Correct on upgrade**: push-invalidation of the cell pointer, Block-STM catches stale reads.
- **Speculative-safe**: overlay mechanism avoids mutating committed state.
- **Simple invalidation**: null the pointer.
  No dependency graph traversal.

Note that the read-set still needs to be extended with `(B, version)` on the fast path so that Block-STM can validate.
But this is a single write to the read-set, not a lookup-then-conditionally-charge operation.

## 7. Optimistic Linking

While gas must still be charged for modules, the linker can eagerly link against any dependencies that happen to be in the global cache at link time.

### The Pipeline

```
1. Load:      cache hit -> return; cache miss -> fetch, deserialize, verify
2. Link:      for each cache miss, resolve against cache; compute layouts
3. Insert:    add linked form to cache
```

During step 2, if module `B` is a dependency of a newly-loaded module `A` and `B` happens to be in the global cache, the linker populates `A`'s function cells with pointers to `B`'s functions.
If `B` is not cached, the cells are left null (lazy resolution at call time).

Similarly for layouts: if all modules needed to define a struct's fields are in cache, the layout is computed and cached eagerly along with its cost.
If not, layout computation is deferred to first access.

### Why This Is Safe

The linker is a total function.
Its output is correct regardless of which dependencies were available at link time:

- **With `B` in cache**: `A`'s cell for `B::foo` is pre-resolved.
  Fast path on first call.
- **Without `B` in cache**: `A`'s cell for `B::foo` is null.
  Slow path on first call resolves from storage.

Both produce identical execution results.
The difference is only performance.
Gas is determined entirely by the loading phase, so optimistic linking cannot affect consensus.

### Framework Linking

Framework modules are always linked directly:

- Pre-loaded and pre-linked at epoch start.
- Cells for framework calls are always populated (never null).
- No gas charged, no read-set entry, no version checks.
- On framework upgrade: full cache flush.
  All cells invalidated.
  Acceptable because framework upgrades are governance events (rare).

## 8. Speculative Execution and Block-STM

This section summarizes how the loading and linking design interacts with speculative execution.

### Global Cache Is Read-Only During Speculation

The global (epoch-level) cache is never mutated by speculative transactions.
Only committed transactions may update it.
Each speculative block has its own overlay that captures publishes and invalidations:

```
Epoch cache (committed, read-only during speculation)
    ├── B1 overlay (speculative, builds on B0)
    │   ├── B3 overlay (speculative, builds on B1)
    │   └── B4 overlay (speculative, builds on B1)
    └── B2 overlay (speculative, publishes module N)
```

### Speculative Publishes

When a speculative transaction publishes module `N`:

- The new executable goes into the block's overlay, not the global cache.
- Function cells for `N` get an overlay entry (see Section 6).
- Other transactions in the same speculative block see the overlay version.
- Transactions in sibling blocks (different fork) see the committed version.

### Block Lifecycle

| Event                    | Action                              | Cost           |
| ------------------------ | ----------------------------------- | -------------- |
| Speculative publish of N | N goes into block overlay           | O(1)           |
| Module read during spec  | Check overlay, fall through to cache| O(overlay)     |
| Block discarded          | Drop overlay, cache untouched       | O(1)           |
| Block committed          | Merge overlay into global cache     | O(overlay size)|

### Why Pull Invalidation Fits

When a speculative transaction is aborted and re-executed:

- Pull invalidation means the re-execution simply re-reads from storage/cache.
  Fresh data is obtained naturally.
  There is no stale state to roll back.
- Push invalidation would require undoing eagerly-pushed invalidations on abort, which interacts poorly with concurrent speculative workers accessing the same cache entries.

Block-STM's standard conflict detection handles the rest: if transaction T10 read module N and transaction T5 wrote N, Block-STM detects the conflict and re-executes T10.

## 9. Open Questions

- **Gas constant calibration.**
  The layout cost formula parameters (`base_cost`, `c0`, `c1`) need calibration against real workloads and mainnet transaction history.

- **Framework upgrade under Zaptos.**
  How does Zaptos handle a framework upgrade in a speculative block?
  A full cache flush during speculation would invalidate sibling speculative blocks.

- **Pre-loading transitive dependencies.**
  Pre-loading a module's transitive struct layout dependencies as an optimization (so they are in cache for linking) has the same gas problem as layouts: charging `max(module_size, transitive_dep_sizes)` avoids overcharging but adds complexity.
  Is the linking quality improvement worth the metering complexity?

- **Interaction with closures.**
  Closures capture references to functions, which may be in other modules.
  How does closure capture interact with the cell-based linking model and the read-set?

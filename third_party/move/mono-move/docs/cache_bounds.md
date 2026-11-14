# Cache Bounds

How MonoMove bounds the memory held by code-derived data: interned identifiers and types, loaded modules, lowered and monomorphized functions, layouts and descriptors.

Value memory (stacks, heaps, frozen heaps of finished transactions) is a separate budget, see `heap_and_gc.md`.

## 1. Problem

`GlobalContext` holds one shared `Context` and a pool of per-worker bump arenas. Everything in it grows monotonically until `MaintenanceGuard::reset_arena_pool` drops all of it. Nothing measures the growth and nothing triggers the reset, so today every one of these is unbounded:

| Structure | Grows when |
|---|---|
| `identifiers`, `module_ids`, `types`, `type_lists`, `function_refs` (maps + arena bytes) | anything is interned for the first time, including user-supplied type arguments |
| `module_cache` (`ModuleSlot` + `LoadedModule`) | a module is loaded, or a slot is created for a name that is looked up |
| `LoadedModule::functions`, `LoadedModule::instantiated_functions` | a function is lowered, or a generic function meets new type arguments |
| `script_cache` | a script with new bytes runs |
| `descriptors`, `layouts` (append-only tables + by-type maps) | a new type needs a layout or a GC descriptor |

The goal is a hard bound of the form

```
code memory  <=  W + N * E
```

where `W` is the maximum size of the shared cache, `N` the number of executors (workers), and `E` the most code memory one executor may hold on its own.

## 2. Goals and non-goals

- **Hard bound**, independent of what transactions do.
- **Determinism.** Cache state differs between validators (speculation, restarts, different flush times). Nothing a transaction observes (status, gas, outputs) may depend on it.
- **Simple, and free in the common case.** The common case is a cache well below its bound. That path gets one extra add per first-use of a module and nothing else.
- **Graceful when full.** A full cache makes transactions slower, never wrong and never stuck. Running with almost no cache is acceptable.
- Non-goal: a clever eviction policy. There is no LRU and no per-entry eviction.
- Non-goal: flushing in the middle of a block. Workers hold raw pointers through their guards and through unmaterialized `SessionEffects`; reclaiming under them needs a stop-the-world protocol that this design avoids.

## 3. The design on one page

```
            shared cache (one Context, N arena shards)
  0 ───────────── W_flush ───────── W_freeze ─────────── W
  │   normal        │  flush at next   │  no new entries;  │
  │                 │  block boundary  │  misses run       │
  │                 │                  │  privately        │
                                       └── W - N*E ──┘

  executor 1..N:  [ private context, <= E, reset after each private transaction ]
```

Five rules:

1. **One weight function.** Every cacheable object has a deterministic *weight* in bytes, an upper bound on what it occupies. The same number is used for both limits below.
2. **Per-transaction limit `T` (deterministic).** A transaction is charged the weight of every module, instantiation and type it uses for the first time, whether it was a cache hit or a miss, exactly like load gas. Above `T` it aborts with a deterministic status.
3. **Shared-cache size `S` (node-local).** One atomic counter, incremented by the weight of every object actually inserted into the shared cache. It only goes up during execution.
4. **Admission.** Any operation that may grow the shared cache first checks `S < W_freeze`. If the check fails the transaction is re-run in **private mode**: against a context the worker owns alone, which starts empty and is reset when the transaction is done.
5. **Flush at block boundaries.** In the maintenance phase, if `S >= W_flush`, reset the shared context (the existing `reset_arena_pool`). Otherwise leave it alone.

`W_freeze = W - N*E` and `E = T + F`, where `F` is the weight of the framework (§4.2).

There is no overlay, no second lookup level and no eviction. Private mode runs the same code against a different `GlobalContext` instance.

## 4. Details

### 4.1 Weights

A weight is a deterministic function of on-chain inputs only (bytecode, type arguments), never of cache state, and is at least the number of bytes the object and everything built with it retain.

| Object | Weight | Covers |
|---|---|---|
| Module (or script) | `BASE_MODULE + K_MODULE * bytecode_len` | slot, interned ID and identifiers, `CompiledModule`, IR, interned signature types, lowered code of all its non-generic functions, layouts and descriptors of its closed types |
| Generic instantiation `f<τ>` | `BASE_INST + K_FUNCTION * ir_len(f) + K_LAYOUT * layout_closure(τ)` | lowered code, substituted types, layouts and descriptors in the closure of its types |
| Type from a `TypeTag` (transaction type arguments) | `K_TYPE * nodes + identifier_bytes` | type nodes, type lists, identifiers, module IDs |
| Resource type published outside lowered code | `K_LAYOUT * layout_closure(ty)` | layouts and descriptors |
| Failed module lookup | `BASE_MODULE` | the slot and interned name left behind |

`layout_closure` counts every nominal instantiation in the closure, not only those missing from the cache.

A built object stores its weight next to its gas cost (`LoadedModule::cost` today), so a hit charges without recomputing.

The constants are calibrated: tests build modules and instantiations under a counting allocator and assert `weight >= measured bytes`, over mainnet modules and adversarial ones (deep signatures, many small functions). An underestimate is not a consensus problem, since weights are deterministic either way; it only means the real bound is looser than the configured one by the gap.

### 4.2 The per-transaction limit `T`

A counter next to the `GasMeter`, charged at the same first-use events as load gas and recorded in the same per-transaction read-set, so it rides on machinery the loader already has:

- first access to a module in this transaction: wherever `gas_meter.charge(module.cost())` is today;
- first use of a generic instantiation: wherever deterministic lowering gas will be charged (`TODO(metering)` in `Loader::lower_function_with_ty_args`);
- interning of user-supplied type arguments.

Exceeding `T` is a deterministic abort, kept and charged like out-of-gas. `T` is consensus-relevant and lives in the on-chain gas schedule. It generalizes the legacy VM's `max_num_dependencies` and `max_total_dependency_size`.

Two requirements on the loader:

- **Charge before build.** On a miss, charge the weight as soon as the bytes are fetched and before deserializing, one module at a time. Today a mandatory set is fully built and inserted, then charged as a sum. The abort decision is the same either way; charging first is what keeps the memory actually built within `T`.
- **Framework modules are exempt**, as they are from load gas and as special addresses are from the legacy dependency limits. Their total weight `F` is a per-epoch constant (a framework upgrade is a reconfiguration), and enters the bound through `E = T + F`. Instantiations of framework generics with arbitrary type arguments are not exempt.

So one transaction can cause at most `E = T + F` of code memory to be built, in any mode.

### 4.3 Shared-cache accounting and admission

`Context` gains `used: AtomicUsize` (`S`). Every insert path of `ExecutionGuard` adds to it: exact bytes for arena allocations (including the duplicates an interning race leaks), stored weights for boxed objects, a fixed per-entry constant for map and table entries. A guard batches its increments and publishes them when the operation ends, so the shared counter is not written on every allocation.

`ExecutionGuard::admit()` is `S < W_freeze`. Because `S` never decreases during execution, a failed admission stays failed until maintenance; no separate "frozen" flag is needed.

Operations that may grow the cache call `admit()` once, on entry, before doing any work. Once admitted, an operation runs to completion and inserts whatever it built, even if `S` crosses `W_freeze` meanwhile. The call sites are few:

- loader miss paths: building a module or script, lowering a function, monomorphizing an instantiation, computing a mandatory set, publishing a resource type's layout, creating a module slot, interning the name in `resolve_function`;
- interning user-supplied type arguments in the transaction executor (the only interning outside the loader today).

Hits never call `admit()`. A transaction served entirely by the cache runs at full speed whether the cache is full or not.

A refused admission returns an internal `CacheFull` error. It is not a transaction result: it must bypass abort handling and the epilogue, and the executor must never turn it into a status. The worker discards the session and re-runs the transaction in private mode. The interner API stays infallible; no caller outside the listed sites changes.

Keeping the check at operation entry rather than at each insert is deliberate. A per-insert check would make every interner method fallible, or need a second place to put the entry that was refused. The price is that each worker may finish one admitted operation past `W_freeze`, which is at most `E`, and `W_freeze` already leaves room for it.

### 4.4 Private mode

Each worker owns a second `GlobalContext` with one arena, created lazily. Private mode is:

1. run the transaction from the start with a guard on the private context;
2. materialize its outcome into storage formats (bytes, `StateKey`s);
3. reset the private context (`maintenance_context().reset_arena_pool()`, as the test harness already does between sessions).

The loader, specializer, interpreter and natives are unchanged: they see an `ExecutionGuard` and cannot tell which context is behind it. The private context has no freeze threshold; rule 2 already limits what a transaction can put in it to `E`.

The transaction pays the same gas and the same memory charges as in shared mode, since neither depends on cache state, so the result is identical. What changes is wall-clock time: everything the transaction touches is loaded from storage, framework included. That is the "no cache" mode, and it is only reached by transactions that miss while the cache is full.

**Boundary rule.** Nothing pointer-shaped crosses between a private context and the rest of the block. A private transaction reads state as bytes and decodes with its own layouts; its writes leave as bytes. This is required, not just tidy: within a block, shared structures use interned pointers and table indices as identity (`InMemoryStorageKey` embeds an `InternedType`, heap object headers carry a `DescriptorId`, `SessionEffects` point into the context that produced them), and those identities mean nothing outside the context that issued them. Step 2 must therefore happen before step 3.

### 4.5 Flush at block boundaries

`MaintenanceGuard` gains one call for the block coordinator to make between blocks:

```rust
/// Resets the shared context if it has reached the flush watermark.
pub fn enforce_cache_bounds(&mut self) -> bool
```

It resets when `S >= W_flush` and sets `S` back to what the reset leaves behind (framework symbols, and the framework itself once it is preloaded). The safe point is the existing one: no execution guard alive, and no unmaterialized `SessionEffects` from earlier blocks.

`W_flush` sits below `W_freeze` by a headroom larger than what a normal block adds, so that a block which starts below `W_flush` does not freeze. Freezing mid-block is then the abnormal case (attack, or a headroom set too small), and it lasts until the next boundary.

A flush costs a cold start: the hot modules are reloaded by the next transactions that use them, once. If that proves too slow, re-warm by name: keep use counts keyed by owned module IDs (not pointers), and reload the top few in the same maintenance phase after the reset. That needs no pointer bookkeeping at all.

### 4.6 Why the bound holds

- Shared cache: an operation is admitted only while `S < W_freeze = W - N*E`. After `S` crosses the threshold, each worker completes at most the one operation it had in flight, adding at most `E`. So `S <= W` always.
- Each worker additionally holds either objects under construction for a shared-mode operation (at most `E`, and they move into `S` or are dropped), or a private context (at most `E`).
- Total `<= W + N*E`.

Not covered, and bounded separately: transient working memory of deserializing and verifying one module (bounded by module size limits), and value memory.

### 4.7 Cost of an attack

Filling the cache means loading `W_freeze` worth of distinct on-chain code and paying load gas for it. The effect is that, until the next block boundary, transactions that miss run uncached. Then the flush restores normal operation. Memory never exceeds the bound and no transaction's result changes.

One existing hole must be closed for this to hold: user type arguments are interned before the prologue runs, so a transaction that is then discarded grows the cache for free. Intern them after the prologue, where the weight is paid for.

## 5. Types and interning

Interned types are the hard part of any bounded design, because pointer equality is used as type equality everywhere, and because types form a DAG across modules.

### 5.1 Why this design needs no change to interning

Pointer equality only has to hold inside one identity space. Here a transaction runs against exactly one context from start to finish, shared or private, and each context has one interner. Private mode does not mix its types with the shared ones, so there is never a second pointer for the same type in view, and copies and comparisons stay O(1). The cost is that a private transaction re-interns what the shared cache already has; that is part of running uncached.

Whole-context flush handles reclamation: types, the code that embeds them, and the tables indexed by IDs all die together, so nothing can dangle.

### 5.2 Types owned by their defining module

The idea: store each instantiation of `0x23::aggregator::Aggregator` on the `aggregator` module slot, so evicting a module frees its types, and interning stays O(1).

It is sound only if a type never outlives anything it points to, and ownership by defining module does not give that:

- `Option<Aggregator<u64>>` would live on `option` and point into `aggregator`. Evicting `aggregator` leaves it dangling. The type arguments of a generic have no relation to the module dependency graph, so no eviction order fixes this.
- `Table<a::K, b::V>` points into two unrelated modules.
- `vector<T>`, references, function types and type lists have no defining module.

Evicting one module therefore needs the reverse closure of everything mentioning its types: other modules' types, layouts and descriptors (append-only tables indexed by ID), monomorphized functions keyed by type lists, call sites patched to `CallByPtr`, mandatory-set slot pointers. That is a tracing collector for code, which is what this design is trying not to build.

The useful core of the idea is that **ownership should follow lifetime**. A type must live in the shortest-lived region among its components, and regions must have nested lifetimes. Per-module regions are not nested. A small fixed set of tiers is.

### 5.3 Follow-up: a pinned framework tier

Stage 1 (§3) has two lifetimes that never mix. The natural next step adds one tier under both:

- **Pinned tier**: the framework modules and everything derived from them alone (IR, non-generic lowered code, the closed types that appear in them, their layouts and descriptors). Built only in the maintenance phase, **immutable during execution**, dropped only on a framework upgrade.
- Shared and private contexts look up pinned first, then themselves, and may point into it.

This is safe for the reasons a mutable read-through overlay is not:

- Pinned never changes while anyone executes, so every context agrees on whether a type is pinned. There is exactly one pointer per type in any context's view, with no race to handle.
- Pinned is built only from the framework, so it never points to a shorter-lived tier, and its content does not depend on what users send.
- ID tables split by range: pinned layouts and descriptors take `[0, P)`, each context numbers from `P`.

It makes a flush cheap (the framework survives) and private mode fast (framework hits are served from pinned), and it matches the loader design's "a single version of the framework is always cached". The one structural change is that instantiations of framework generics with non-pinned types must be stored in the context, not inside the pinned `LoadedModule`.

It is a follow-up because Stage 1 is correct and bounded without it; do it when measurements say private mode or flush recovery is too slow.

## 6. Configuration

| Parameter | Where | Meaning |
|---|---|---|
| `T` | on-chain gas schedule | per-transaction code-memory limit (deterministic) |
| `K_*`, `BASE_*` | on-chain gas schedule | weight constants |
| `max_code_memory` | node config (`MaintenanceConfig`) | the hard bound `W + N*E` |
| `flush_headroom` | node config | `W_freeze - W_flush` |

Derived at startup: `E = T + F`, `W = max_code_memory - N*E`, `W_freeze = W - N*E`, `W_flush = W_freeze - flush_headroom`. A node whose budget leaves `W_freeze` at zero still works: every transaction that misses runs privately.

The node-local values can differ between validators without affecting results.

## 7. Implementation order

Each step is useful on its own.

1. **Accounting.** Weights on `LoadedModule` and `FunctionSlot`; byte counts in `GlobalArenaShard::alloc*`; `Context::used`; metrics. No behavior change.
2. **Flush.** Real `MaintenanceConfig`; `MaintenanceGuard::enforce_cache_bounds`. Bounds growth across blocks, which is the realistic case.
3. **Per-transaction limit.** Memory meter beside `GasMeter`; charges at first-use sites in `loader.rs`; charge-before-build in `get_verified_module_from_storage`; new deterministic error kind; type arguments interned after the prologue.
4. **Admission and private mode.** `ExecutionGuard::admit`; `CacheFull`; per-worker private `GlobalContext` and the retry in the worker loop. Completes the hard bound.
5. Optional: re-warm by name (§4.5), pinned tier (§5.3).

Debug builds assert, per transaction, that the weight inserted through its guard does not exceed the weight charged to it.

## 8. Alternatives considered

| Alternative | Why not |
|---|---|
| Read-through overlay: when full, read the shared cache and write new entries to a per-transaction layer | Two lookup levels in every cache, a split ID space for layouts and descriptors, a race protocol so a type cannot become shared after a transaction made a local copy, and a standing invariant that no shared object ever stores a local pointer (inline caches, patched call sites). One slip is a use-after-free across transactions. The pinned tier gets most of the benefit with an immutable lower level. |
| Flush mid-block when full | Needs every worker to drop its guard and every unmaterialized output to be gone. |
| Per-module or LRU eviction | Needs the reverse closure in §5.2. |
| Always intern types in the shared cache, bound only modules and code | Types (and the layouts and descriptors that follow them) then grow with the number of transactions in a block, not with `N`. |
| Check the limit on every insert instead of on operation entry | Makes the whole interner API fallible for a tighter threshold that the `N*E` slack already covers. |
| Bound by gas alone, no `T` | Works in principle, but couples a memory bound to gas pricing. An explicit limit is what the legacy VM does too. |

## 9. Open questions

- **Block coordinator.** The boundary rule (§4.4) needs the coordinator to hand a private transaction the writes of earlier transactions as bytes, and to accept materialized outputs alongside unmaterialized ones. It is not built yet, so this is a requirement on it.
- **Module upgrades.** A private context is reset after every transaction, so it cannot hold a stale module. Keeping it warm across transactions would need the same validation as the shared cache, whose upgrade story is still open (`TODO(completeness)` in `module_cache.rs`).
- **Charging the framework.** Exempting it (§4.2) follows precedent and keeps framework growth from eating into `T`. Charging it would remove `F` from the formulas at the cost of transactions near the limit failing after an upgrade.
- **Zaptos.** A flush needs a point where no block is executing and no earlier output is unmaterialized. With pipelined blocks that point has to be made, not assumed.

# Cache Bounds

How MonoMove bounds the memory held by code-derived data: interned identifiers and types, loaded modules, lowered and monomorphized functions, layouts and descriptors.

Value memory (stacks, heaps, frozen heaps, the per-block resource arena) is a separate budget, see `heap_and_gc.md`.

## 1. Problem

`GlobalContext` holds one shared `Context` and one bump arena per worker. Everything in it grows monotonically until `MaintenanceGuard::reset_arena_pool` drops all of it:

| Structure | Grows when |
|---|---|
| `identifiers`, `module_ids`, `types`, `type_lists`, `function_refs` (maps + arena bytes) | anything is interned for the first time, including user-supplied type arguments |
| `module_cache` (`ModuleSlot` + `LoadedModule`) | a module is loaded, or a slot is created for a name that is looked up |
| `LoadedModule::functions`, `LoadedModule::instantiated_functions` | a function is lowered, or a generic function meets new type arguments |
| `script_cache` | a script with new bytes runs |
| `descriptors`, `layouts` (append-only tables + by-type maps) | a new type needs a layout or a GC descriptor |

In production the context is owned by `AptosModuleCacheManager` (`aptos-move/block-executor/src/code_cache_global_manager.rs`) as an `Arc<GlobalContext>` with `concurrency_level` arenas. It is handed to every block and never reset; nothing measures it. (`reset_arena_pool` is only called by the test harness.)

The goal is a hard bound of the form

```
code memory  <=  W + N * E
```

where `N` is the number of executors, `E` the most code memory one transaction can cause to be built, and `W` a watermark for the cache.

## 2. Goals and non-goals

- **Hard bound**, independent of what transactions do.
- **Determinism.** Cache state differs between validators. Nothing a transaction observes (status, gas, outputs) may depend on it.
- **Simple, and free in the common case.** A cache well below its bound costs one extra add per first use of a module, and one comparison per transaction.
- **Graceful when full.** A full cache makes execution slower, never wrong and never stuck.
- Non-goal: an eviction policy. There is no LRU and no per-entry eviction; the only way to reclaim is to drop the whole context.

## 3. The design on one page

Three numbers:

1. **Weight.** Every cacheable object has a deterministic weight in bytes, an upper bound on what it occupies.
2. **`T`, the per-transaction limit (deterministic).** A transaction is charged the weight of every module, instantiation and type it uses for the first time, hit or miss, exactly like load gas. Above `T` it aborts with a deterministic status. With the framework's weight `F` (exempt from `T`, §4.2), no transaction can cause more than `E = T + F` to be built.
3. **`S`, the size of the cache (node-local).** One atomic counter, incremented by the weight of everything actually inserted. It only goes up between flushes.

One check, made by the block executor **before it starts any transaction**:

```
if S + N * E > max_code_memory:
    flush            # sequential execution: drop the context here, between two transactions
```

and one more at block start, to keep the first from firing in normal operation: flush if `S >= W_flush`.

That is the whole mechanism for sequential execution, which is what MonoMove runs today (`MonoTransactionExecutor::execute` rejects `ViewMode::Parallel`). With `W = max_code_memory - N * E`, a transaction starts only while `S <= W`, each running transaction adds at most `E`, so `S <= W + N * E`.

Nothing inside the VM changes behavior: no fallible interner, no second lookup level, no new error path. The VM gains the accounting and the `T` meter; the policy lives in the block executor.

Parallel execution cannot flush between transactions. §4.6 covers what it does instead.

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

The constants are calibrated: tests build modules and instantiations under a counting allocator and assert `weight >= measured bytes`, over mainnet modules and adversarial ones. An underestimate is not a consensus problem, since weights are deterministic either way; it only means the real bound is looser than the configured one by the gap.

### 4.2 The per-transaction limit `T`

A counter next to the `GasMeter`, charged at the same first-use events as load gas and recorded in the same per-transaction read-set:

- first access to a module in this transaction: wherever `gas_meter.charge(module.cost())` is today;
- first use of a generic instantiation: wherever deterministic lowering gas will be charged (`TODO(metering)` in `Loader::lower_function_with_ty_args`);
- interning of user-supplied type arguments.

Exceeding `T` is a deterministic abort, kept and charged like out-of-gas. `T` is consensus-relevant and lives in the on-chain gas schedule. It generalizes the legacy VM's `max_num_dependencies` and `max_total_dependency_size`.

Requirements:

- **Charge before build.** On a miss, charge the weight as soon as the bytes are fetched and before deserializing, one module at a time. Today a mandatory set is fully built and inserted, then charged as a sum. The abort decision is the same either way; charging first is what keeps the memory actually built within `T`.
- **Framework modules are exempt**, as they are from load gas and as special addresses are from the legacy dependency limits. Their total weight `F` is a per-epoch constant, and enters the bound through `E = T + F`. Instantiations of framework generics with arbitrary type arguments are not exempt.
- **Always on.** The block executor currently runs MonoMove with `without_metering()`. The `T` meter must be enforced regardless of whether gas is.
- **Pay before growing.** User type arguments are interned before the prologue (`user_txn/execute.rs`), so a transaction that is then discarded grows the cache for free. Intern them after the prologue.

### 4.3 Accounting

`Context` gains `used: AtomicUsize` (`S`). Every insert path of `ExecutionGuard` adds to it: exact bytes for arena allocations (including the duplicates an interning race leaks), stored weights for boxed objects, a fixed per-entry constant for map and table entries. A guard batches its increments and publishes them when it is dropped, at the end of the transaction, so the shared counter is not written on every allocation.

Debug builds assert, per transaction, that the weight inserted through its guard does not exceed the weight charged to it.

### 4.4 Flushing between transactions (sequential execution)

`execute_transactions_sequential` runs one transaction, applies its writes to the block's `UnsyncMap`, and materializes its output before the next one starts. So between two transactions no session is alive, and a flush is possible, provided everything in the block that still points into the context is dealt with. Reading the wiring, that is:

| Holder | What it holds | On flush |
|---|---|---|
| `UnsyncMap::resource_map` | `MonoValue` heap pointers under `InMemoryStorageKey` keys (which embed an `InternedType`) | cleared |
| `UnsyncMap::arena` (`SharedArena`) | base values decoded with the context's layouts; object headers carry `DescriptorId`s | replaced with a fresh arena |
| `UnsyncMap::groups` | group members as bytes under `StateKey` | kept |
| committed outputs (`ret`) | materialized `TransactionOutput`s | kept |
| `BlockGasLimitProcessor` | in-memory keys kept until the epilogue, where `materialize_storage_key` converts them | converted to `StateKey` before the flush |
| `MonoTransactionExecutor` | an `Arc<GlobalContext>` clone; guards are taken per call | dropped and re-created, so the flush has exclusive access |

Clearing `resource_map` also drops the writes of earlier transactions in this block, which later transactions must still see. They survive as bytes: on a flush, the writes of the outputs committed since the previous flush are folded into a `StateKey -> Option<StateValue>` delta, and `BlockSTMSequentialProvider` reads `resource_map`, then the delta, then `base_view`. Entries in `resource_map` are always newer than the delta, so the order is correct without any invalidation. In a block that never flushes, the delta is never built.

After a flush `S` is back to what a reset leaves behind, and the next transaction reloads what it needs.

### 4.5 Flushing at block start

`AptosModuleCacheManager::try_lock` is where the legacy caches are size-checked and flushed per block, and where a `TODO(correctness)` already asks for the MonoMove context to be reset or version-checked. The MonoMove check goes there: flush if `S >= W_flush`.

`W_flush` sits below `W` by a headroom larger than what a normal block adds, so a block that starts below `W_flush` does not flush in the middle.

Both flushes need `&mut GlobalContext`, and the manager holds an `Arc` that it clones to executors. Either take it with `Arc::get_mut`, which succeeds exactly when no executor holds a clone, or keep the context under the manager's existing mutex.

When the manager's lock cannot be taken, `try_lock_inner` builds a fresh context for that block. The per-transaction check applies to it unchanged, and it is dropped with the block.

### 4.6 Parallel execution

MonoMove does not run in parallel yet, so this is a requirement on that work, not on today's code. The check is the same, made by each worker before it starts an incarnation; what differs is what a failed check can do, because other workers hold guards and the `MVHashMap` holds pointers.

A flush is impossible while any worker is inside a transaction, or while the `MVHashMap`, the outputs and the read sets of uncommitted transactions exist. So under concurrency a flush always means stopping every worker and giving up the speculative work past the committed prefix. There are three ways to proceed, in order of preference:

- **Cut at the committed prefix and resume in parallel.** The worker whose check fails halts the scheduler, as the block gas limit already does (`scheduler.halt()`). Workers finish the incarnation they are in (the `N * E` term). The committed prefix finishes materializing, which still needs the context. Then the `MVHashMap`, and the outputs and read sets of everything uncommitted, are dropped; the context is flushed; and a fresh Block-STM run executes the remaining transactions on top of `base_view` plus the prefix's writes as bytes, the same delta as §4.4. The first transaction of each run reads only committed state, so it always commits, and every cut makes progress. Cost: the speculative work past the prefix, and a cold cache. What is new in the block executor is resuming on a suffix while carrying the block-level state across (block limit processor, delayed-field counter, output vector, epilogue index).
- **Fall back.** Stop the parallel attempt and re-run the whole block on the sequential path, which can flush. This is the existing parallel-to-sequential fallback in `execute_block`, so it costs nothing to build, but it discards the committed prefix too and loses the block's parallelism.
- **Private mode**, to never stop at all. The worker runs the transaction against a context it owns alone, materializes the output, and resets that context. This keeps the block parallel and is the "stop caching globally, cache per executor, free at the end" scheme. It needs a path for values to cross as bytes, because the `MVHashMap` would be keyed by `InMemoryStorageKey` and hold `MonoValue` pointers, and neither means anything to a second context (§5.2).

Either way the bound is unchanged: an incarnation starts only while `S <= W`, and each of the `N` running ones adds at most `E`.

### 4.7 Why the bound holds

- A transaction starts only if `S + N * E <= max_code_memory`, i.e. `S <= W`.
- At most `N` transactions run at once, and each causes at most `E` to be built, whether those objects are already inserted (counted in `S`) or still under construction.
- So code memory `<= W + N * E = max_code_memory`.

Not covered, and bounded separately: transient working memory of deserializing and verifying one module, and value memory.

### 4.8 Cost of an attack

Filling the cache means touching `W - S` worth of distinct on-chain code. The effect is a flush, after which hot modules are reloaded once. Memory never exceeds the bound and no result changes.

How cheap that is depends on load gas. With legacy-style pricing, where the per-transaction dependency limit rather than gas is what binds, forcing a flush every block is affordable, so recovery from a flush should be made cheap rather than assumed rare: preload the framework after a reset (already a `TODO(completeness)` in `try_lock`), and see §5.3.

## 5. Types and interning

Interned types are the hard part of any bounded design, because pointer equality is used as type equality everywhere, and because types form a DAG across modules.

### 5.1 In this design

There is one interner. A flush drops types together with the code that embeds them, the tables indexed by IDs, and (§4.4) the block state that uses them as keys. Nothing can dangle and interning does not change.

### 5.2 If a second interner is ever added (private mode)

Pointer equality only holds inside one identity space, so a transaction must use exactly one interner from start to finish: all of its types are private, or all are shared. The same type may then exist in both, as two different pointers, and no transaction sees both.

The consequence is at the block level. Today block state is kept in the shared identity space: `UnsyncMap` keys embed an `InternedType`, and its values are heap objects whose headers carry `DescriptorId`s and which can contain function pointers. A private transaction can neither look up nor store there.

**Keys are the easy half.** `StateKey` is itself interned, in a process-wide registry that has nothing to do with `GlobalContext`: equality is `Arc::ptr_eq`, the hash is precomputed, and entries are weak, so the registry shrinks as keys are dropped and is not another cache to bound. A `StateKey` therefore means the same thing in every context and survives every flush. If block state is keyed by `StateKey` (as it is for the legacy VM, and already for MonoMove's resource groups), a private transaction looks up the same map as everyone else, and a flush has no keys to convert. The price is building the `StateKey` from `(address, InternedType)`: a `StructTag` plus a registry lookup under a read lock, once per key a transaction touches for the first time, since later accesses hit its own working map. Caching the `StructTag` on the type (the `TODO(perf)` on `nominal_tag`) removes most of that.

**Values are the hard half.** A heap pointer is only valid in the context that built it. So a map entry needs two forms, a heap value or bytes, and a reader that meets the form it cannot use converts: a private reader serializes a heap value with the shared layouts (the entry must carry its type or layout for that) and decodes it with its own; a shared reader decodes bytes into the block arena. A private transaction's writes go in as bytes.

That is one map with a two-form value, converted lazily on read, and it is what private mode costs. It is the reason private mode is not in Stage 1.

A read-through variant (read the shared cache, write new entries to a local layer) has the same block-level problem for every key or value that involves a local type, plus two lookup levels in every cache and a standing invariant that no shared object ever stores a local pointer.

### 5.3 Types owned by their defining module

The idea: store each instantiation of `0x23::aggregator::Aggregator` on the `aggregator` module slot, so evicting a module frees its types, and interning stays O(1).

It is sound only if a type never outlives anything it points to, and ownership by defining module does not give that:

- `Option<Aggregator<u64>>` would live on `option` and point into `aggregator`. Evicting `aggregator` leaves it dangling. The type arguments of a generic have no relation to the module dependency graph, so no eviction order fixes this.
- `Table<a::K, b::V>` points into two unrelated modules.
- `vector<T>`, references, function types and type lists have no defining module.

Evicting one module therefore needs the reverse closure of everything mentioning its types: other modules' types, layouts and descriptors (append-only tables indexed by ID), monomorphized functions keyed by type lists, call sites patched to `CallByPtr`, mandatory-set slot pointers. That is a tracing collector for code.

The useful core of the idea is that **ownership should follow lifetime**: a type must live in the shortest-lived region among its components, and regions must have nested lifetimes. Per-module regions are not nested. A small fixed set of tiers is, and one extra tier is enough to be worth having:

- **Pinned tier**: the framework modules and everything derived from them alone (IR, non-generic lowered code, the closed types that appear in them, their layouts and descriptors). Built only in the maintenance phase, immutable during execution, dropped only on a framework upgrade.
- The context looks up pinned first, then itself, and may point into it.

Because pinned never changes while anyone executes, there is exactly one pointer per type in view and no race to handle; because it is built only from the framework, it never points to anything shorter-lived and its content does not depend on what users send. ID tables split by range: pinned takes `[0, P)`, the context numbers from `P`. Instantiations of framework generics with non-pinned types must then be stored in the context, not inside the pinned `LoadedModule`.

With it a flush keeps the framework, which is most of what a flush costs. It is a follow-up: Stage 1 is correct and bounded without it.

## 6. Configuration

| Parameter | Where | Meaning |
|---|---|---|
| `T` | on-chain gas schedule | per-transaction code-memory limit (deterministic) |
| `K_*`, `BASE_*` | on-chain gas schedule | weight constants |
| `max_code_memory` | `BlockExecutorModuleCacheLocalConfig` → `MaintenanceConfig` | the hard bound `W + N * E` |
| `flush_headroom` | same | `W - W_flush` |

Derived at startup: `E = T + F`, `N = concurrency_level`, `W = max_code_memory - N * E`. The budget must leave `W > F`, or every transaction flushes.

The node-local values can differ between validators without affecting results.

## 7. Implementation order

Each step is useful on its own.

1. **Accounting.** Weights on `LoadedModule` and `FunctionSlot`; byte counts in `GlobalArenaShard::alloc*`; `Context::used`; metrics. No behavior change.
2. **Flush at block start** in `AptosModuleCacheManager::try_lock`, with a real `MaintenanceConfig`. Bounds growth across blocks.
3. **Per-transaction limit.** Memory meter beside `GasMeter`; charges at first-use sites in `loader.rs`; charge-before-build in `get_verified_module_from_storage`; new deterministic error kind; type arguments interned after the prologue.
4. **Flush between transactions** in `execute_transactions_sequential`: the check, the `UnsyncMap` reset, the bytes delta in `BlockSTMSequentialProvider`. Completes the hard bound.
5. With parallel MonoMove execution: the prefix cut of §4.6, or the fallback until it exists.
6. Optional: pinned tier (§5.3), private mode (§4.6).

## 8. Alternatives considered

| Alternative | Why not |
|---|---|
| Private per-executor context once the cache is full | Not needed while execution is sequential, where a flush between transactions does the job with one interner. For parallel execution it is an optimization over falling back, with the block-level cost in §5.2. |
| Read-through overlay | §5.2. |
| Check on every insert inside the VM instead of before each transaction | Makes interning fallible and adds an error path through the VM, for a tighter threshold that the `N * E` term already covers. |
| Per-module or LRU eviction | Needs the reverse closure in §5.3. |
| Flush only at block start | One block can add far more than `W`. |
| Bound by gas alone, no `T` | Couples a memory bound to gas pricing. An explicit limit is what the legacy VM does too. |

## 9. Open questions

- **Module upgrades.** The context is never invalidated today, so an upgrade leaves a stale translation cached. That needs its own answer; the flush hook in `try_lock` is where a coarse one (reset on publish) would go.
- **Charging the framework.** Exempting it (§4.2) follows precedent and keeps framework growth from eating into `T`. Charging it would remove `F` from the formulas at the cost of transactions near the limit failing after an upgrade.
- **Zaptos.** A flush needs a point where no block is executing against the context. With pipelined blocks sharing one context, that point has to be made, not assumed.

# Cache Bounds

How MonoMove bounds the memory held by code-derived data, and the interning redesign that makes the bound simple.

Value memory (stacks, heaps, frozen heaps, the per-block resource arena) is a separate budget, see `heap_and_gc.md`.

## 1. Problem

`GlobalContext` holds one shared `Context` and one bump arena per worker. Identifiers, types, modules, lowered functions, layouts and descriptors all grow monotonically until `MaintenanceGuard::reset_arena_pool` drops everything. In production the context is owned by `AptosModuleCacheManager` (`aptos-move/block-executor`), handed to every block, and never reset.

Two things make it hard to bound as it stands:

- **Everything is one lifetime.** Types live in arenas shared by all modules, so nothing can be freed without freeing everything.
- **Everything is identity.** Pointer equality on interned types is used as type equality, and those pointers are baked into block state: `UnsyncMap` keys embed an `InternedType`, heap object headers carry a `DescriptorId`. So nothing can be made local to a transaction without breaking what other transactions see.

Most interned types need neither property. Separating the ones that do from the ones that do not is the redesign.

## 2. What types are used for

From the specializer, loader and runtime:

| Where a type appears | Example | Needs identity across modules and transactions? |
|---|---|---|
| A module's pools: signatures, field types, instantiation signatures, IR operands (`PreparedModule::build`, destack) | `vector<T>`, `&Coin<T>`, `T` | No. Read as a template; compared only inside the module (e.g. type-keyed slot recycling). |
| Substitution intermediates during lowering (`try_build_context`) | substituted home-slot list, callee parameter list | No. Used once to compute slot sizes. The current code interns whole lists just to pass them to `subst_type_list` (there is a `TODO(perf)` on it). |
| Type arguments of a call, native call or closure (`CallByName`, `CallNative`, `FunctionRef`) | `[Coin<APT>]` | Yes. Key of the monomorphization cache. |
| Resource, table-value and event types (`Exists`, `BorrowGlobal`, `MoveTo`, `InMemoryStorageKey`) | `CoinStore<APT>` | Yes. Key into block state. |
| Types that have a layout or a GC descriptor (`Layouts::by_ty`, `Descriptors::*`) | `vector<Coin<APT>>` | Yes. The IDs end up in heap objects other transactions read. |

Everything in the "yes" rows is a closed type: no type parameters. Everything with a type parameter is in a "no" row.

Today all of these go through the same interner. `PreparedModule::build` interns a module's entire signature pool, type parameters and references included, into the shared maps at load time.

## 3. Three spaces

```
   code:   LoadedModule ── owns ──> module space (terms)
           lowered Function
              │
              │ points to
              ▼
   key space:  names, ground types, type lists, function refs,
               layouts, descriptors, module slots
              ▲
              │ points to
   values: heap objects (descriptor ids, closure function refs),
           block state keys
```

**Module space.** The types written in a module's bytecode, open or closed, as *terms*. Interned in an arena the `LoadedModule` owns, with a plain single-threaded map: deduplicated within the module, meaningless outside it. Freed with the module.

**Key space.** Canonical *ground* (closed) types, plus the other things that are identity: identifiers and module IDs, type-argument lists, function refs, the layout and descriptor tables, module slots. Shared by all workers, as the interner is today. Small per entry.

**Code.** Modules and lowered functions. The bulk of the memory.

One rule about pointers: **code and values point into key space; key space points only to itself; nothing points into code except the transaction running it.** That rule is what the bounds below rest on.

### 3.1 Interning

`PreparedModule::build` and `destack` are already generic over `Interner`, so the split is mostly a second implementation:

- A **module interner** (bump arena + `UnorderedMap`) is passed to `build`. Terms, type parameters, references and the partially substituted instantiation signatures all stay in the module. Names it delegates to key space. Loading a module no longer touches the shared type maps.
- The **key-space interner** is the existing `DashMap`-based one, holding ground types only.
- One bridge: `instantiate(term, ty_args: &[Ty]) -> Ty`. It reads a term from any module and interns the ground result in key space. This replaces `subst_type` at the points that produce keys.

What lowering `f<τ>` puts into key space is then exactly what something is keyed by:

- the type-argument list of each call, native call and closure it contains;
- the resource types of its global-storage ops;
- the vector and nominal types among its slots and fields, because their layouts are looked up by type.

It does not intern references, parameter and return lists, or anything open. Those are computed into a scratch buffer for the duration of the lowering. `Function::param_tys` and `return_tys`, the one place substituted signatures are kept, become owned data on the function: a ground type plus a by-value / `&` / `&mut` marker.

Terms and ground types get distinct Rust types (`Term`, `Ty`), with no `==` between them; `instantiate` and structural matching (as `SignatureTokenKey` does today) are the only ways across. A pointer comparison between spaces does not compile.

### 3.2 What the split buys

- **A module is self-contained.** Nothing outside a module points into its arena: after lowering, a function holds ground types, IDs and names only. So a `LoadedModule` can be owned by a single transaction and dropped at its end, or evicted from the cache on its own, without tracing anything. Other modules reach it only through its slot, which already handles being empty.
- **Identity is small.** Key space holds no IR, no bytecode and no templates, so it can afford to be always shared.
- **Local code does not fork identity.** A transaction running a module nobody else has cached still produces the same ground types, layout IDs and descriptor IDs as everyone else. Block state needs no second representation.

## 4. Bounds

Two budgets, because the two spaces fail differently: code is big but can be local; keys are small but cannot.

```
code memory  <=  K_max  +  W_code  +  N * E
```

### 4.1 Weights and per-transaction limits

Every first use of a module or of an instantiation `f<τ>` in a transaction is an event with two deterministic weights, computed from on-chain inputs only and stored on the built object next to its gas cost:

| | Code weight | Key weight |
|---|---|---|
| Module | bytecode, IR, term arena, lowered non-generic functions | identifier pool, module IDs, slot |
| Instantiation `f<τ>` | lowered code | ground types and lists it instantiates, layouts and descriptors in their closure |
| User type arguments, failed lookups | none | type nodes and names |

A transaction is charged both, hit or miss, at the sites where load gas is charged, and aborts deterministically above `T_code` or `T_key`. Both live in the gas schedule. They generalize the legacy `max_num_dependencies` and `max_total_dependency_size`.

Weights must upper-bound real bytes; tests check them against a counting allocator. On a miss the charge comes before the build, one module at a time (today a mandatory set is built, inserted, then charged as a sum).

### 4.2 Code: shared up to `W_code`, then local

A built module or function is published to the shared cache only if its weight fits: reserve against an atomic counter, publish on success. Otherwise the transaction keeps it and frees it when it ends:

- the read-set entry for a module becomes shared-reference or owned;
- a function lowered for a shared module but not publishable goes in a per-transaction table keyed by its function ref.

No transaction can build more than `E = T_code`, so code memory is at most `W_code + N * E`. The framework is preloaded and lowered at reset, counted in `W_code`, never evicted, and exempt from `T_code` as it is from load gas.

A module can be local in one transaction and shared in the next; nothing depends on which. The read-set already pins one copy per transaction.

At block start, if the cache is near `W_code`, drop code and keep key space. Since modules are self-contained this is safe per module, so the framework and any modules worth keeping stay; no tier or tracing is needed.

### 4.3 Keys: a deterministic limit per block

Key space cannot be local and cannot be reclaimed while a block runs. So bound what a block can add, deterministically:

- **`X_key`, a block limit like the gas limit.** At commit, in order, each transaction's first-use events are added to the block's set; an event counts once per block. When the summed key weight exceeds `X_key`, the block is cut and the rest is retried later, through the existing `BlockGasLimitProcessor`. Counting once per block is what makes the limit usable: transactions in a block share almost all of their instantiations, so honest blocks sit far below it, while a per-transaction sum would count every hit.
- A stamp on the shared object (module slot, function ref) makes "first in this block" one compare-and-set at commit. Events with no such object, like user type arguments, are counted every time.

Sequentially, every executed transaction is committed, so a block grows key space by at most `X_key + T_key`, whatever the cache held.

In parallel, aborted incarnations and transactions past the cut also intern. That part is not deterministic, so it gets a node-local guard: an incarnation starts only while this block's growth is below `G`; past it, the attempt halts and the block falls back to the existing sequential path, which is bounded as above. A block therefore adds at most

```
X_block = G + X_key + (N + 1) * T_key
```

At block start, in `AptosModuleCacheManager::try_lock`: if `K + X_block > K_max`, reset everything. That is the only time key space is reclaimed, and it is a point where nothing is running. There is no flush in the middle of a block, no private context and no bytes path for block state.

### 4.4 Cost of an attack

- Filling the code cache makes the attacker's own transactions run uncached. Everyone else's cached code keeps working.
- Reaching `X_key` cuts the block, as reaching the gas limit does, so key weight has to be priced like block capacity. Each instantiation's key weight should carry gas accordingly.
- Reaching `K_max` over many blocks forces a reset at a block boundary, after which the framework is preloaded and hot modules reload once.

## 5. Safety rules

The pointer rule of §3 has to be true of the code. Where it is not today:

1. **Closures memoize a function pointer into the heap object.** `CallClosure` flips an `Unresolved` closure to `Resolved` after the first call. That value can be written to storage and read by a later transaction, after a local function is freed. Memoize only pointers to shared functions; resolve the rest through a per-transaction cache.
2. **Call-site patching.** `CallByPtr` is not emitted yet. When it is, shared code may be patched only to shared targets, and cross-module patching makes per-module eviction unsafe. Keep cross-module calls by name with a per-transaction inline cache.
3. **Publishing is the only way into the shared cache**, and what is published may reference only key space and its own arena. A module or function satisfies this by construction once terms are module-local.
4. **Pay before growing key space.** User type arguments are interned before the prologue (`user_txn/execute.rs`), so a discarded transaction grows key space for free. Intern them after.
5. **The limits are always on.** The block executor runs MonoMove with `without_metering()` today. `T_code` and `T_key` must hold regardless.

## 6. Configuration

| Parameter | Where | Meaning |
|---|---|---|
| `T_code`, `T_key` | gas schedule | per-transaction limits |
| `X_key` | on-chain block limits | distinct key weight per block |
| weight constants | gas schedule | §4.1 |
| `W_code`, `K_max`, `G` | `BlockExecutorModuleCacheLocalConfig` | node-local sizes |

`N = concurrency_level`. Node-local values can differ between validators without affecting results.

## 7. Implementation order

1. **Module interner.** `ModuleInterner`, `Term` / `Ty`, `instantiate`; `PreparedModule::build` on the module interner; lowering instantiates keys and uses scratch for the rest; owned `param_tys`. No behavior change, and module loads stop contending on the shared type maps.
2. **Accounting.** Weights, counters for code and keys, metrics.
3. **Reset at block start** in `try_lock`. Bounds growth across blocks.
4. **Per-transaction limits**, charge-before-build, rule 4.
5. **Local code** (§4.2), rules 1–3.
6. **`X_key`** in `BlockGasLimitProcessor`; the guard `G` with parallel execution.

## 8. Alternatives considered

| Alternative | Why not |
|---|---|
| Keys local to a transaction when the key space is full | Types, layout IDs and descriptor IDs are identity inside a block. Local ones need a freeze protocol so a type cannot be both local and shared, a split ID space, block-state keys by `StateKey`, and values exchanged as bytes whenever they contain a local descriptor. |
| Private context per executor | Forks all identity, not only code; same block-state cost. |
| Flush key space mid-block (between transactions, or at the committed prefix in parallel) | Works without a consensus rule, but needs block state rebuilt as bytes and, in parallel, resuming a block after a cut. It is the fallback if `X_key` is not wanted. |
| Types owned by their defining module | `Option<Aggregator<u64>>` on `option` points into `aggregator`; `Table<a::K, b::V>` into two modules; `vector<T>` has none. Evicting a module would need the reverse closure. The split above gets the benefit differently: a module owns its terms, and ground types belong to no module. |
| Deterministic block limit for code as well | Honest blocks touch a lot of distinct code, aborted incarnations can each load megabytes, and a cheap way to cut blocks is a worse failure than running uncached. |
| Bound keys by gas alone | A per-transaction sum counts hits, so either the bound is useless or every transaction pays for instantiations that are already cached. |

## 9. Open questions

- **Names.** Identifiers are interned into key space when a module is built, so a module's identifier pool is part of its key weight even when its code stays local. If that dominates `T_key`, intern names lazily, when they first appear in a ground type or a by-name call, and key a module's own function table by string.
- **Module upgrades.** Layouts in key space outlive the module version they were computed from. The coarse answer is to reset on publish at the next block start; a real one is part of the versioning design.
- **Parallel guard.** `G` trades wasted parallel attempts against reserved headroom, and needs measuring once MonoMove runs in parallel.
- **Zaptos.** A reset needs a point where no block is executing against the context.

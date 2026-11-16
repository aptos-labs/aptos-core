# Cache Bounds

How MonoMove bounds the memory of its type and code caches, and the interning redesign that makes it possible.

Scope: types and code. Value memory (stacks, heaps, frozen heaps, the per-block resource arena) is a separate budget, see `heap_and_gc.md`. Block state (`MVHashMap`, `UnsyncMap`) is not redesigned here; §6 states what this design needs from it.

## 1. Problem and constraints

`GlobalContext` holds one shared `Context` and one bump arena per worker. Identifiers, types, modules, lowered functions, layouts and descriptors grow until `MaintenanceGuard::reset_arena_pool` drops everything. In production the context is owned by `AptosModuleCacheManager` (`aptos-move/block-executor`), handed to every block, and never reset.

Target:

```
type and code memory  <=  W + N * E
```

`W` is the size of the shared cache, `N` the number of executors, `E` the most one transaction can build.

Constraints:

- **Nothing is flushed while a block executes.** Block state holds pointers into the cache; it is never rebuilt or converted mid-block.
- **No fallback to sequential execution.** The mechanism is the same for sequential and parallel execution and never stops workers.
- **Determinism.** Cache state differs between validators. Nothing a transaction observes may depend on it.

So when the cache is full, a transaction that needs something new has to keep it to itself and free it when it ends. The hard part is types: pointer equality on interned types is type equality everywhere, so a private type must never meet a shared copy of itself.

## 2. What types are used for

From the specializer, loader and runtime:

| Where a type appears | Example | Needs identity across modules and transactions? |
|---|---|---|
| A module's pools: signatures, field types, instantiation signatures, IR operands (`PreparedModule::build`, destack) | `vector<T>`, `&Coin<T>`, `T` | No. Read as a template; compared only inside the module. |
| Substitution intermediates during lowering (`try_build_context`) | substituted home-slot list, callee parameter list | No. Used once to compute slot sizes. |
| Type arguments of a call, native call or closure | `[Coin<APT>]` | Yes. Key of the monomorphization cache. |
| Resource, table-value and event types | `CoinStore<APT>` | Yes. Key into block state. |
| Types that have a layout or a GC descriptor | `vector<Coin<APT>>` | Yes. The IDs end up in heap objects. |

Everything in the "yes" rows is a closed type. Today all rows go through one interner, and `PreparedModule::build` interns a module's whole signature pool into the shared maps at load time.

## 3. Types anchored on their defining module

### 3.1 Terms stay in the module

The types written in a module's bytecode, open or closed, are *terms*. They are interned in an arena the `LoadedModule` owns, with a plain single-threaded map: deduplicated within the module, meaningless outside it, freed with it. `PreparedModule::build` and `destack` are already generic over `Interner`, so this is a second implementation passed to `build`.

### 3.2 Identity is the definition

A closed type that needs identity gets it from the module that defines it:

```
LoadedModule
 ├─ terms (arena)
 ├─ TypeDef  per struct/enum
 │    └─ instances:  type arguments  ->  TypeInst
 └─ FuncDef  per function
      └─ instances:  type arguments  ->  Function        (today's instantiated_functions)

Ty = primitive | &TypeInst
```

- **A non-generic type is its definition.** `0x1::coin::Info` is the single `TypeInst` created with the module. Resolving a struct handle yields that pointer; there is no lookup and nothing to intern.
- **A generic instance is an entry in its definition's table**, keyed by the pointers of its type arguments: `Coin<APT>` lives under `Coin`. This is the shape `instantiated_functions` already has.
- **`vector<T>` and function types** are instances of built-in definitions on a pseudo-module that is always present.
- **Argument lists are compared by content**, as slices of `Ty` pointers. They need no interning of their own.
- **Names.** A module's identifiers stay in its own pool. A definition is found from its module's slot by name; cross-module lookups hash the string once, at lowering.

`instantiate(term, ty_args) -> Ty` is the one bridge from terms to identity: it walks a term and looks up, or creates, the instances it names. Lowering `f<τ>` calls it for exactly the things that are keys: type arguments of calls, natives and closures; resource types; vector and nominal types among its slots and fields. References, parameter lists and anything open are computed into a scratch buffer and dropped. `Function::param_tys` and `return_tys` become owned data on the function.

Terms and `Ty` are distinct Rust types with no `==` between them.

### 3.3 Derived data hangs off identity

| Derived from | Data | Recorded dependencies |
|---|---|---|
| `TypeInst` | layout, GC descriptors | the modules whose definitions its layout reads |
| `FuncDef` + arguments | lowered `Function` | `mandatory_dependencies`, as today |
| module ID | `LoadedModule` (bytecode, IR, terms) | — |

A layout then belongs to a specific version of its definition and lists the other modules it read, the way a lowered function does. That closes the existing `TODO(correctness)` on `publish_enum_descriptor` about layout caches surviving an upgrade.

### 3.4 What anchoring buys

- **A module that is local to a transaction makes its types local with no extra machinery.** Its definitions are in the transaction's copy; nobody else can reach them.
- **Nothing outside a module points into its terms.** After lowering, a function holds `Ty` pointers, IDs and names only.
- **No global structural hashing.** The shared `types` map goes away; contention is per definition.
- **Memory is attributable.** Every instance, layout and function belongs to one definition.

One thing anchoring does not give: an instance is not always *stored* where it is indexed. `Option<Aggregator<u64>>` is indexed under `Option` but points into `aggregator`. If `aggregator` is local to a transaction, the shared `Option` table cannot hold that entry. The rule: **an instance is stored with the shortest-lived thing it mentions.** With two lifetimes that is always well defined, and §4 is arranged so the choice is never a race.

## 4. Shared and local

Every table in §3.2 and §3.3 exists twice: in the shared cache, and in a per-transaction local set that is empty in the common case. Lookup is shared first, then local.

### 4.1 The gate

Publishing into the shared cache goes through one gate, a reader-writer lock around a flag:

```
publish(entry, weight):
    hold gate (shared)
    if gate is closed            -> keep entry local
    if used + weight > W         -> release, close the gate (exclusive), keep entry local
    insert into shared table, used += weight
```

- The gate starts open at block start and, once closed, stays closed for the rest of the block.
- Closing takes the lock exclusively, so it waits for publishes in flight. After it returns, the shared cache does not change until the block ends.
- A transaction creates a **local** entry only after it has seen the gate closed and then looked the entry up in shared once more.

While the gate is open, execution is as today: everything built is shared, and nothing local exists. After it closes, the shared cache is read-only and everything new is local.

### 4.2 Why a type has exactly one identity in a transaction

- A local entry for a key exists only if the key was absent from shared after the gate closed. Shared no longer changes, so the key stays absent: no transaction can see a shared copy and a local copy of the same type.
- Nothing is published after any local entry exists, so the shared cache never points to local data.
- Local entries point to shared entries or to the same transaction's local entries. Both outlive them.
- A module is pinned once per transaction by the read-set, as today, so a transaction sees one copy of each module whether shared or local.

The storage rule of §3.4 is then automatic: before the gate closes nothing is local, so every instance is shared; after, every new instance is local.

### 4.3 The per-transaction limit

Every first use of a module or an instantiation in a transaction has a deterministic weight, computed from on-chain inputs only and stored on the built object next to its gas cost. A transaction is charged it, hit or miss, where load gas is charged, and aborts deterministically above `T`. `T` lives in the gas schedule and generalizes the legacy `max_num_dependencies` and `max_total_dependency_size`.

- Weights must upper-bound real bytes; tests check them against a counting allocator.
- On a miss the charge comes before the build, one module at a time. Today a mandatory set is built, inserted, then charged as a sum.
- The framework is preloaded and lowered at reset, counted in `W`, and exempt, as it is from load gas.

So a transaction builds at most `E = T`, local or not.

### 4.4 The bound

- A publish reserves before it inserts, so the shared cache never exceeds `W`.
- Each executor holds at most `E` of local entries and entries under construction.
- Total `<= W + N * E`, in sequential and parallel execution alike, with no flush and no pause.

### 4.5 Between blocks

`AptosModuleCacheManager::try_lock` is where the legacy caches are size-checked per block. The check for MonoMove goes there, with nothing executing: if the gate closed in the previous block, or `used` is above a watermark, reclaim and reopen the gate.

Stage 1 reclaims by resetting the shared cache and preloading the framework. Because a module's code and terms are self-contained, a later stage can evict cold modules individually instead and keep the rest; that needs the instances of an evicted module's types either kept as bare identity or dropped together with everything that depends on the module, via the recorded dependencies.

## 5. Safety rules

1. **Publishing is the only way into the shared cache**, and only through the gate. `OnceLock` function slots and the instantiation maps on a shared module are written by `publish`, not directly.
2. **Closures memoize a function pointer into the heap object.** `CallClosure` flips an `Unresolved` closure to `Resolved` after its first call, and that value can outlive a local function. Memoize only pointers to shared functions; resolve the rest through a per-transaction cache.
3. **Call-site patching.** `CallByPtr` is not emitted yet. When it is, shared code may be patched only to shared targets, under the gate.
4. **Pay before growing.** User type arguments are interned before the prologue (`user_txn/execute.rs`), so a discarded transaction grows the cache for free. Intern them after.
5. **The limit is always on.** The block executor runs MonoMove with `without_metering()` today. `T` must hold regardless.

## 6. What this needs from block state

After the gate closes, a transaction can hold types nobody else has. Two things leave a transaction and carry type identity:

- **Keys.** `InMemoryStorageKey` embeds a `Ty`. Two transactions with their own local copy of a type would not agree on the key.
- **Values.** Heap object headers carry a `DescriptorId`, and closures carry a function ref. Another transaction reading the value by pointer could not interpret a local one.

Block state is out of scope here, so the design is staged around it.

**Stage 1: identity is exempt from the gate.** Definitions, instances, the layout and descriptor IDs assigned to them, and function refs are always published, even after the gate closes. They are small: a pointer, an argument slice and two IDs per instance. Everything bulky stays behind the gate: module code and terms, lowered functions, and the *data* behind a layout or descriptor ID, which a transaction looks up by ID in the shared table and then in its own. Keys and values are then exactly what they are today, and block state does not change.

The price is that identity is not under `W + N * E`. It is limited per transaction by `T` and reclaimed between blocks, so within a block it can grow by the number of executions times what one transaction may create. That is a soft bound: the reset watermark has to leave that much headroom.

**Stage 2: identity behind the gate too.** The bound becomes hard. It needs block state to key by `StateKey` when a type is local (`StateKey` is interned process-wide and means the same to every transaction), and to pass a value with a local layout as bytes rather than by pointer. Whether a type is local is the same for every transaction once the gate is closed, so both choices are consistent across the block.

## 7. Configuration

| Parameter | Where | Meaning |
|---|---|---|
| `T` and the weight constants | gas schedule | per-transaction limit |
| `W`, reclaim watermark | `BlockExecutorModuleCacheLocalConfig` | node-local |

`N = concurrency_level`. Node-local values can differ between validators without affecting results.

## 8. Implementation order

1. **Module-local terms.** `ModuleInterner`, `Term` / `Ty`, `instantiate`; lowering instantiates keys and uses scratch for the rest. No behavior change.
2. **Definitions and instances.** `TypeDef` / `TypeInst` on the module; layouts and descriptors hung off instances with their dependencies; the global `types` map removed.
3. **Accounting and the per-transaction limit.** Weights, `used`, `T`, charge-before-build, rule 4.
4. **Reclaim at block start** in `try_lock`.
5. **The gate and local tables**, rules 1–3. This completes Stage 1.
6. Later: per-module eviction (§4.5), Stage 2 (§6).

## 9. Alternatives considered

| Alternative | Why not |
|---|---|
| Read-only shared cache during a block, with local results promoted between blocks | No lock on the miss path and no gate, but anything new is rebuilt by every transaction that needs it until the next block, and promotion needs conflict handling between transactions that built the same thing. |
| Flush mid-block, or cut the block and resume | Rebuilds block state. Excluded by §1. |
| Fall back to sequential when speculation overshoots | Excluded by §1. |
| A deterministic per-block limit on new identity | Bounds only what commits; aborted incarnations also intern, and bounding those needs one of the two rows above. |
| A private context per executor | Forks all identity, not only what is new. |
| One global key space instead of per-definition tables | Works, but keeps global structural hashing, gives layouts no version, and local modules need their types tracked separately. |

## 10. Open questions

- **Stage 1 headroom.** How much identity a block can create in the worst case depends on how many incarnations Block-STM runs; it needs measuring.
- **Reclaim policy.** A reset every time the gate closes lets an attacker force a cold start per block by filling `W`. Per-module eviction (§4.5) removes most of that cost.
- **Module upgrades.** Recorded dependencies make a stale layout or function detectable; the versioning itself is the loader's open item.
- **Zaptos.** Reclaim needs a point where no block is executing against the context.

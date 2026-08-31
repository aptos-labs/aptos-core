# move-fuzz: Design

This document describes what the Move fuzzer is, how the pieces fit together, and
why the design looks the way it does. It is meant to be read once, front to
back, in about half an hour; after that you should be able to open any file in
`src/` and know what it is responsible for.

- User-facing instructions live in [`../README.md`](../README.md).
- Agent-facing working rules live in [`../AGENT.md`](../AGENT.md).
- This file is the *architecture* reference. When you change phase behavior,
  DUG semantics, chain construction, or the persistence format, update it.

All paths below are relative to `third_party/move/tools/move-fuzz/`.

---

## 1. Motivation

### 1.1 What makes Move contracts hard to fuzz

A classical coverage-guided fuzzer takes a byte buffer, feeds it to one
entrypoint, and mutates the buffer. That model breaks down for Move smart
contracts in three separate ways.

**There is no single entrypoint.** A Move package exposes many `public` and
`public entry` functions. Interesting behavior lives in the *combination* of
them, not in any one of them.

**Arguments are not bytes.** A transaction argument is a typed Move value. Some
types -- integers, addresses, strings, vectors of those -- can be synthesized
from nothing. Others cannot: you cannot conjure a `Coin<T>`, a `&mut Vault`, or a
capability struct out of random bytes, because Move's type system only lets such
values be *produced by other Move code*. A fuzzer that only randomizes
scalar arguments can never call the functions that matter.

**The interesting state is global and cross-transaction.** Most real bugs need a
setup sequence: `create_pool` then `add_liquidity` then `swap`. Running `swap`
against genesis state just aborts with `MISSING_DATA` forever. The fuzzer needs
to discover, on its own, *which* function produces the state that some other
function reads.

### 1.2 The two ideas

move-fuzz addresses these with two ideas that map onto the two halves of the
crate.

**Idea 1 -- synthesize driver scripts (static, `prep/`).** Instead of calling a
target function directly, generate a Move *script* that builds the target's
non-trivial arguments by calling other functions, and exposes only the trivially
fuzzable leaves as script parameters. The search for "how do I obtain a value of
type `T`" is a backward search over a flow graph whose nodes are function
instantiations and datatypes. Every feasible flow graph is lowered into one
concrete `.move` script. Those scripts, compiled to bytecode, become the fuzzer's
entrypoints.

**Idea 2 -- learn a Def-Use Graph over global state (dynamic, `executor/`).**
Run each script and record which global-state slots it *reads* and which it
*writes*, keyed by `(account address, struct tag)`. That yields a bipartite
graph: scripts define resource types, scripts use resource types. If script `B`
reads a type that only script `A` writes, then `A -> B` is a candidate
transaction chain. This graph is the **DUG** (Def-Use Graph), and it is what
turns single-transaction fuzzing into multi-transaction fuzzing.

The DUG is not computed up front from static analysis. It is *observed*, one
execution at a time, from real VM reads and writes -- which means it stays
correct in the presence of dynamic dispatch, tables, resource groups, and
objects, none of which a purely static analysis handles well.

---

## 2. The `auto` pipeline end to end

`aptos move fuzz <PATH> auto` is the whole product. Everything else
(`list`, `build`, `test`, `exec`) is periphery.

```text
  aptos move fuzz <PATH> [top-level opts] auto [opts]
        |
        |  cli.rs :: run_on
        v
  +---------------------------------------------------------------+
  | 0. Workspace prep                                              |
  |    copy project -> TempDir (unless --in-place)                 |
  |    parse --alias / --resource, canonicalize --subdir           |
  +---------------------------------------------------------------+
        |  deps.rs :: resolve
        v
  +---------------------------------------------------------------+
  | 1. Project resolution                                          |
  |    walk for Move.toml, recurse into deps (git/local)           |
  |    consolidate named addresses, apply alias groups             |
  |    materialize accounts (fixed / generated / resource)         |
  |    classify Primary | Dependency | Framework                   |
  |    toposort -> Project { pkgs, named_accounts, language }      |
  +---------------------------------------------------------------+
        |  cli.rs :: cmd_auto  +  package.rs
        v
  +---------------------------------------------------------------+
  | 2. Build every package, in topological order                   |
  |    per package: fingerprint -> load .move-fuzz/package-cache/  |
  |                 or compile and populate the cache              |
  |    emits Vec<PkgDefinition> (topologically ordered)            |
  +---------------------------------------------------------------+
        |  prep/model.rs :: Model::new    (ALWAYS runs)
        v
  +---------------------------------------------------------------+
  | 3. Static analysis                                             |
  |    DatatypeRegistry: every struct/enum, its generics+abilities |
  |    FunctionRegistry:  every script-callable public function    |
  +---------------------------------------------------------------+
        |  prep/model.rs :: Model::populate  (skipped on cache hit)
        v
  +---------------------------------------------------------------+
  | 4. Driver-script generation                                    |
  |    for each Primary function x ability-set combo x lambda combo|
  |      GraphBuilder::process -> candidate FlowGraphs             |
  |      is_feasible + compact_generics                            |
  |      DriverCanvas::try_build -> statements + params            |
  |      dedup, cap at MAX_SCRIPTS_PER_FUNCTION, write .move       |
  |    into <workdir>/autogen/sources/                             |
  |    (--dry-run STOPS HERE: sources are written, never compiled) |
  +---------------------------------------------------------------+
        |  package.rs :: build (the "Autogen" package)
        v
  +---------------------------------------------------------------+
  | 5. Compile entrypoints                                         |
  |    Autogen depends on every analyzed package                   |
  |    each CompiledScript is paired with its ScriptSignature      |
  |    persisted to .move-fuzz/entrypoints_cache.json              |
  +---------------------------------------------------------------+
        |  fuzzer.rs :: entrypoint
        v
  +---------------------------------------------------------------+
  | 6. Campaign                                                    |
  |    build TypePool, drop scripts with unsatisfiable generics    |
  |    TracingExecutor::new (genesis) + publish packages + users   |
  |    scan initial state -> seed DUG and object dictionaries      |
  |    restore .move-fuzz/auto_state.json if it still applies      |
  |    -> PHASE 1 -> (saturation) -> PHASE 2 -> (saturation) stop  |
  +---------------------------------------------------------------+
```

Stage 2 and stages 4/5 are individually cached. On a re-run against an unchanged
project, stage 2 is served entirely from the package cache and stages 4 and 5 are
skipped in favor of the entrypoint cache; stages 0, 1, 3 and 6 always run.

**Two things about `--dry-run` that the diagram makes explicit, because they are
easy to get wrong.** It returns immediately after `Model::populate` (see
`fuzzer.rs`, the `if dry_run { ... return Ok(()) }` block): the generated `.move`
sources are written to `<workdir>/autogen/sources/`, but the autogen package is
never built, no bytecode is produced, and *the entrypoint cache is never
written*. A dry run therefore never speeds up a subsequent real run.

---

## 3. Module map

| File | Owns |
|---|---|
| `src/lib.rs` | module root, nothing else |
| `src/cli.rs` | `clap` subcommands; `run_on()`; `cmd_auto()` -- state dir, package build+cache loop, `autogen` package scaffolding, string dictionary, hand-off to `fuzzer::entrypoint`; also `write_frontend_stats` for the pre-loop stages |
| `src/bin/move-fuzz-dev.rs` | standalone binary with the same CLI shape as `aptos move fuzz`, for iteration without building the whole Aptos CLI |
| `src/common.rs` | `Account` (`Ref` / `Owned` / `Resource`); `TxnArgType` / `TxnArg` used by the runbook path |
| `src/language.rs` | `LanguageSetting` -> `CompilerConfig` and CLI flags |
| `src/utils.rs` | `with_logging_disabled` (silences the compiler during builds) |
| `src/subexec.rs` | subprocess supervision, used by the localnet simulator |
| `src/account.rs` | `AddressRegistry`: named-address <-> account bookkeeping, `NamedAddressKind` (Primary/Dependency/Framework), stable user addresses via `stable_user_address` (`0xFD` in byte 0, the user ordinal big-endian in the last 8 bytes) |
| `src/deps.rs` | package discovery and dependency resolution; `PkgKind`; named-address consolidation, alias groups, resource-account derivation; toposort into `Project` |
| `src/package.rs` | `FuzzPackage`; `build()` / `unit_test()`; build-cache fingerprint, slot layout, load/save; package metadata extraction |
| `src/simulator.rs` | drives a **real** `aptos node run-local-testnet` subprocess (used only by `exec`) |
| `src/testnet.rs` | provisioning + JSON runbook execution against that simulator |
| `src/prep/mod.rs`, `src/mutate/mod.rs` | submodule declarations only |
| `src/prep/ident.rs` | `ModuleIdent` / `DatatypeIdent` / `FunctionIdent` |
| `src/prep/typing.rs` | the type model: `TypeExpr` (syntactic), `TypeRef` (with ref-ness), `TypeBase` / `TypeItem` (with abilities); the simple-vs-complex split (`TypeMode`); `TypeSubstitution` (tag -> base) and `TypeUnification` (base <-> base); `IntrinsicType` for `BitVector` / `String` / `Object` |
| `src/prep/datatype.rs` | `DatatypeRegistry`: declarations + field/variant contents, `convert_signature_token`, `instantiate_type_ref` |
| `src/prep/function.rs` | `FunctionRegistry`: script-callable functions; the source-level `public` / `public entry` filter (`parse_script_public_functions`) |
| `src/prep/graph.rs` | `GraphBuilder` / `FlowGraph`: backward search for argument providers (internal / external / copyable), value conversions, feasibility checking, generic compaction, exploration budgets |
| `src/prep/canvas.rs` | `DriverCanvas`: lowers a feasible `FlowGraph` into `DriverStatement`s plus `BasicInput` parameters, renders the `.move` script, emits `ScriptSignature` |
| `src/prep/model.rs` | `Model` (the two registries) and `populate()`: the generation loop over primary functions, ability-set combinations, lambda binding search, per-function caps, progress reporting |
| `src/mutate/mutator.rs` | `Mutator` and `TypePool`: value generation and mutation per `BasicInput`, address/object dictionaries, type-argument generation and mutation, string dictionary |
| `src/executor/mod.rs` | coverage-map helpers: `merge_coverage`, `count_coverage_entries`, `collect_coverage_keys`, `coverage_delta`, `clone_exec_coverage_map` |
| `src/executor/tracing.rs` | `TracingExecutor` over `FakeExecutor`: genesis, gas tweaks, package provisioning and publishing, the read-recording state view, write extraction, full-state scan, and forking via `Clone` |
| `src/executor/oneshot.rs` | `ExecStatus` taxonomy; `OneshotFuzzer` -- Phase 1 |
| `src/executor/sequence.rs` | `ResourceTag`, `SeedInput`, `ExecResourceProfile`, `DefUseGraph`, `Chain` / `SeedChain`, `SequenceDb`, chain construction and sequence mutation, `ChainFuzzer` -- Phase 2 |
| `src/fuzzer.rs` | campaign orchestration: entrypoint generation/caching, type pool, campaign fingerprint, the main loop, scheduling, phase transition, checkpointing, stats |
| `src/state.rs` | every `Persisted*` type plus versioned, atomic load/save helpers |
| `tests/demo/`, `tests/prep/` | tiny Move fixture packages |

Dependency direction is essentially one-way:

```text
   cli.rs
     |
     +--> deps.rs ---> package.rs ---> language.rs
     |                     |
     |                     v
     +--> fuzzer.rs -----> prep/{model,graph,canvas,typing,datatype,function,ident}
              |
              +---------> executor/{tracing,oneshot,sequence} ---> mutate/mutator.rs
              |
              +---------> state.rs   (serialization for everything above)

   simulator.rs + testnet.rs + common.rs + subexec.rs   (exec subcommand only)
```

Note the last line: the `exec` subcommand runs against a real local testnet
subprocess and shares almost nothing with `auto`, which runs entirely in-process
against `FakeExecutor`. Do not confuse the two paths.

---

## 4. Core data model

### 4.1 `Model` -- the static view

```rust
Model {
    datatype_registry: DatatypeRegistry,   // ident -> (generics, abilities, kind) + contents
    function_registry:  FunctionRegistry,  // ident -> (generics, params, returns, kind, is_entry)
}
```

Both registries are `BTreeMap`s keyed by ident, so iteration order is
deterministic. Both are populated by walking every `CompiledModule` of every
included package. `PkgKind` is carried through so later stages can apply the
**external provider policy**: only `Primary` and `Dependency` functions may be
introduced as argument providers, and `Primary` outranks `Dependency`
(`PkgKind::is_external_provider_candidate` / `external_provider_rank`).
Framework functions are visible for *type* resolution but are never pulled in as
providers -- otherwise every graph would be dominated by stdlib constructors.

`FunctionRegistry` registers only `Visibility::Public` functions, and when module
source text can be located it further intersects with functions the source
declares as `public` / `public entry` (`parse_script_public_functions`), which
drops `public(package)` and `public(friend)`. When no source text is available
the registry falls back to bytecode visibility alone.

`Model::new` is cheap relative to generation and runs on every `auto` invocation,
including entrypoint-cache hits -- the `TypePool` used to filter and instantiate
scripts is derived from it (`build_type_pool`), so it cannot be skipped.

### 4.2 The type ladder (`prep/typing.rs`)

```text
SignatureToken           (move-binary-format, raw bytecode)
      |  DatatypeRegistry::convert_signature_token
      v
TypeRef  = Refty<TypeExpr>  = Base | ImmRef | MutRef             (declaration-level)
      |  DatatypeRegistry::instantiate_type_ref(ty, type_args)
      v
TypeItem = Refty<TypeBase>  = Base | ImmRef | MutRef             (instantiated, carries abilities)
      |  TypeMode::convert
      v
TypeMode = Simple(SimpleType) | Complex(ComplexType)
```

The **simple/complex split is the pivot of the whole generation design**:

- **Simple** = can be materialized directly as a transaction argument: all
  integers, `bool`, `address`, `signer`, `String`, `BitVector`, `Object<...>`,
  and vectors of simple types. These become script *parameters*
  (`BasicInput`) and are what the mutator actually fuzzes.
- **Complex** = a user datatype, a generic parameter, or a vector of those.
  These cannot be passed in and must be *produced inside the script* by calling
  something. These become nodes in the flow graph.

`Object<T>` is deliberately classified as simple (`SimpleType::ObjectKnown` /
`SimpleType::ObjectParam`): on-chain it is just an address, so the fuzzer
supplies an address and lets the VM's `Object` conversion abort if it is wrong.

### 4.3 `FlowGraph` (`prep/graph.rs`)

A DAG built with `petgraph::StableGraph`:

- Nodes: `Function(FunctionInst { ident, type_args })` or `Datatype(DatatypeItem)`
  where `DatatypeItem` is `Base` / `ImmRef` / `MutRef` of a `ComplexType`.
- Edges:
  - `Use(i)`: datatype -> function; the datatype is argument `i` of the call.
  - `Def(i)`: function -> datatype; the datatype is return value `i` of the call.
  - `Copy`, `Deref`, `Freeze`, `ImmBorrow`, `MutBorrow`,
    `VectorToElement`, `ElementToVector`: datatype -> datatype value conversions.

Exactly one function node has no outgoing edges: that is the **root**, the
target function being fuzzed. Data flows *toward* the root.

Example, for target `pool::swap(&signer, Coin, u64)` where `Coin` is a complex
non-droppable datatype produced by `pool::mint(&signer, u64): Coin`:

```text
   [ Fn pool::mint ] --Def(0)--> [ Base Coin ] --Use(1)--> [ Fn pool::swap ]   (root)
```

`GraphBuilder::process()` does the search:

```text
process(decl, type_args)
  reset budgets
  add_call(base_graph, decl, type_args, tail=None)

add_call(graph, decl, type_args, tail)
  bail if trace depth >= max_trace_depth
  bail if this exact instantiation already appears max_call_repetition times
  push node; link Def edge to tail if any
  for each complex parameter:
      add_arg -> plan_for_datatype   (cartesian-product over all candidate graphs)

plan_for_datatype(graph, dt_node)
  probe_internal   : reuse an unused return value of a function already in the graph
  probe_external   : bring in a new Primary/Dependency function whose return unifies
                     (recurses into add_call)
  probe_copyable   : copy an identical datatype node already in the graph (if `copy`)
  plus structural conversions:
      Base<T>  <- Deref(&T) / Deref(&mut T)          if T has `copy`
      Base<T>  <- VectorToElement(vector<T>)
      Base<vector<T>> <- ElementToVector(T)
      &T       <- Freeze(&mut T) | ImmBorrow(T)
      &mut T   <- MutBorrow(T)
```

Every probe can multiply the candidate set, so exploration is bounded twice:
`MAX_DERIVED_GRAPHS_PER_PROCESS = 4096` graphs **per primary function**
(reset by `process()`), and an optional wall-clock deadline from
`--max-script-gen-secs-per-function` (default 600, `0` disables). Whichever
fires is reported through `process_limit_hit()`, and truncation means graphs may
be *incomplete*.

`GraphBuilder::is_feasible()` is the gate that keeps incomplete graphs out. It
rejects a graph when:

- two `Use` edges target the same parameter index of one call (a malformed graph);
- a call is missing a provider for any of its complex parameters -- concretely,
  the set of provided `Use` indices differs from the set of complex parameter
  indices. This is the truncation guard: an acyclic graph is not necessarily a
  complete one;
- more than one `signer` parameter is needed across the whole graph (a script
  gets one sender);
- a return value is neither consumed by a `Def` edge nor droppable;
- a non-droppable base-typed datatype node has no *consuming* outgoing edge
  (`Use`, `VectorToElement`, `ElementToVector`) -- borrowing does not consume,
  so a produced-but-only-borrowed linear value is rejected.

After that, `compact_generics()` renumbers the surviving type parameters into a
contiguous `T0..Tn`.

### 4.4 `DriverCanvas` and `ScriptSignature` (`prep/canvas.rs`)

`DriverCanvas::try_build(model, graph, lambda_bindings)` topologically sorts the
graph and walks it, emitting a straight-line statement list. It returns `Option`
and degrades to `None` rather than panicking when a variable is missing -- the
correct entrypoint for codegen; do not reintroduce `expect()` here.

For the example graph above, generation produces roughly:

```move
script {
    fun fuzz_script_7(p0: signer, p1: u64, p2: u64) {
        let v0 = &p0;
        let v1 = <addr>::pool::mint(v0, p1);
        let v2 = &p0;
        <addr>::pool::swap(v2, v1, p2);
    }
}
```

(Hand-traced through `try_build`, not captured from a run; the address is written
as `<addr>` rather than a literal. To see a real one, use `--in-place --dry-run`
and read `<project>/autogen/sources/`.)

with

```rust
ScriptSignature {
    name: "fuzz_script_7",
    ident: <addr>::pool::swap,          // the primary function this script targets
    generics: vec![],                   // ability constraints of T0..Tn
    parameters: vec![Signer, U64, U64], // BasicInput, signer forced to index 0
}
```

Details worth knowing:

- **One signer.** All `signer` / `&signer` / `&mut signer` uses share a single
  script parameter, forced to position 0 (`script_params.insert(0, ...)`). At
  runtime the VM injects it from the transaction sender, so the fuzzers strip
  `BasicInput::Signer` from the argument list they generate and instead choose a
  *sender address*.
- **`BitVector` bridge.** `std::bit_vector::BitVector` has no transaction-argument
  representation, so the canvas takes a `vector<bool>` parameter and emits inline
  Move code that builds the `BitVector` (`format_bitvec_bridge`, recursive for
  nested vectors).
- **Lambdas.** A `Function`-typed parameter is satisfied by pre-selecting a
  concrete function whose signature unifies (`find_matching_functions` in
  `model.rs`) and emitting `let vN = |a0, a1| target(a0, a1);`.
  `vector<Function>` is unsupported and hits a `todo!()` in `prep/typing.rs`.
- **Dedup.** `dedup_key()` renders generics + parameters + statements into a
  string; `Model::populate` keeps a global set so structurally identical wrappers
  are emitted once.

### 4.5 `SeedInput` -- a concrete invocation

```rust
SeedInput { sender: AccountAddress, ty_args: Vec<TypeTag>, args: Vec<MoveValue> }  // core TypeTag
```

This is the fuzzer's unit of work and the unit of persistence. A Phase 1 corpus
entry is one `SeedInput`; a Phase 2 corpus entry is a `Vec<SeedInput>`, one per
chain step.

### 4.6 `ResourceTag` -- a global-state slot

```rust
ResourceTag { account: AccountAddress, struct_tag: StructTag }
```

This is the vocabulary of the DUG. It is produced from real VM accesses in
`executor/tracing.rs`:

- resource and resource-group access paths map directly;
- table items, raw keys, and trading-native keys have no struct tag, so they are
  folded into a **synthetic** tag `0x1::global_state::{table_,raw_,trading_native_}<hash>`
  where the hash is a `DefaultHasher` over the state key's `Debug` rendering
  (`synthetic_struct_tag`). Table-backed protocols often expose no other durable
  cross-transaction state, so keeping them is deliberate.
- `0x1::transaction_context::*` is filtered out (`should_track_resource_tag`) --
  it is high-volume per-transaction noise, not durable state.

### 4.7 The DUG (`executor/sequence.rs`)

Three kinds of node and four kinds of edge:

```text
            script nodes                type nodes (ResourceTag)         script nodes
            (producers)                                                  (consumers)

     S3 ------- def ----------->  T0 = (0xa11ce, 0x..::pool::Pool)  ---- use ------> S9
     S3 ------- def ----------->  T1 = (0xobj42, 0x..::vault::Vault) --- use ------> S11
  (initial state) -------------->  T2 = (0x1,     0x1::account::Account)

            seed nodes (one per observed execution)

     K = SeedNode { id, script_index: 3, seed: SeedInput{..}, succeeded: true }
            |  seed_def -> {T0, T1}
            |  seed_use -> {T2}
```

Fields (`DefUseGraph`):

| Field | Meaning |
|---|---|
| `type_nodes` / `type_index` | interned `ResourceTag`s |
| `defs[script]` / `uses[script]` | abstract, per-script write/read type sets |
| `producers[type]` | inverse of `defs` |
| `initial_types` | types already present after genesis + package publishing + account funding |
| `ever_succeeded` | scripts that succeeded at least once |
| `object_addresses` | addresses observed to carry an `0x1::object::ObjectGroup` write |
| `seed_nodes`, `seed_defs`, `seed_uses`, `seed_producers` | the concrete overlay: one node per *execution*, carrying the exact `SeedInput` |
| `modification_count`, `seed_modification_count` | monotonic change markers used to gate chain reconstruction |

The abstract layer answers "which script can produce type T". The seed layer
answers the sharper question "which *concrete inputs* were observed to produce
type T", which is what makes constructed chains actually work rather than merely
type-check.

**Writes are only recorded from successful executions**; reads are always
recorded (`ExecResourceProfile::from_execution`). A failed transaction still
tells you what it *wanted*, which is exactly the signal Phase 2 needs.

#### Object address abstraction

Aptos objects live at addresses derived at creation time, so the same logical
resource appears under a different `ResourceTag` on every run and even between
two executions of the same script. Without abstraction the DUG would explode into
singleton type nodes and no chain would ever link up.

The rule:

```text
is_object_abstractable_tag(tag) := tag.account is a known object address
                                   AND tag.struct_tag is not ObjectGroup itself

tags_are_compatible(a, b)       := a == b
                                   OR (both abstractable AND a.struct_tag == b.struct_tag)
```

Everything that asks "is this type available" or "who produces this type" goes
through the equivalence class (`equivalent_type_nodes`, `type_is_available`,
`approx_producers_of`, `approx_seed_producers_of`) rather than exact index
equality. Object addresses are learned from `ObjectGroup` writes via
`note_object_address`, which is called from `add_initial_resource_tag` and `add_def` --
note that it is deliberately *not* called from `add_use`.

### 4.8 `Chain`, `SeedChain`, `SequenceDb`

- `Chain { steps: Vec<usize> }` -- script indices in execution order;
  `steps[0]` runs first, `steps.last()` is the target.
- `SeedChain { chain, seed_inputs, target_seed_id }` -- a chain plus the concrete
  per-step inputs harvested from DUG seed nodes.
- `SequenceEntry` -- a chain that actually produced something, with its per-step
  inputs and its per-step read/write tag sets.
- `SequenceDb` -- the cross-fuzzer pool of those entries (capped at
  `MAX_SEQUENCE_DB_ENTRIES = 4096`, pruned by `entry_quality`). It serves two
  purposes: seed sharing by prefix matching, and proposing new chains by
  extension and mutation.

---

## 5. Phase 1 -- single-transaction fuzzing

One `OneshotFuzzer` per generated script. Each owns:

- a **private fork** of the executor (`TracingExecutor::clone`),
- its own `Mutator` seeded with `derive_seed(base_seed, script_index)`,
- a corpus of scored `SeedInput`s (`MAX_ONESHOT_CORPUS = 256`),
- its own accumulated `ExecCoverageMap`,
- a `replay_log` of every executed seed.

`OneshotFuzzer::run_one()`:

```text
1. decide: generate fresh, or mutate a corpus seed (Mutator::should_mutate,
   GEN_PROB 50 vs MUT_PROB 50)
   - if mutating, the *same* corpus entry supplies both the mutation base and
     (70% of the time) the sender
2. build ty_args + args for non-signer parameters
3. clear the VM trace buffer
4. run the transaction with read tracking; the write set is COMMITTED to this
   fuzzer's fork
5. flush the trace buffer; parse it into a CoverageMap; merge -> found_new?
6. build an ExecResourceProfile from (writes, reads, succeeded)
7. return (status, corpus_size, found_new, shared_writes, profile, seed)
```

Two consequences that surprise people:

- **Phase 1 is not stateless.** Each fuzzer's fork accumulates every successful
  write it has ever made. Successive iterations therefore run against a
  progressively richer state, which is intentional -- it is how a single-script
  fuzzer ever gets past its own initialization function. Reproducing an
  interesting execution requires the whole replay log, not just the last seed.
- **Forks do not share state.** Fuzzer `A`'s writes are invisible to fuzzer `B`'s
  storage. The only cross-fuzzer channels are (a) the object dictionary
  broadcast (`absorb_shared_object_writes`, so every mutator learns addresses of
  newly created objects) and (b) the DUG / `SequenceDb`.

The orchestrator (`fuzzer.rs`) consumes each result:

```rust
score = 32*found_new + 20*dug_changed + 10*produced_writes + 4*success + 6*missing_data
```

(`SEED_SCORE_COVERAGE`, `SEED_SCORE_DUG`, `SEED_SCORE_STATE_PROGRESS`,
`SEED_SCORE_SUCCESS`, `SEED_SCORE_MISSING_DATA`.) Any non-zero score means the
seed is remembered with that score; corpus selection is score-weighted
(`pick_seed_index`) and pruning evicts the lowest `(score, last_used_at)`.

Every profile is folded into the **bootstrap DUG** via `add_seed_observation`,
so by the time Phase 1 ends the graph is already populated with both abstract
edges and concrete seed nodes. A single-step `SequenceDb` entry is recorded
whenever the execution found coverage, changed the DUG, or wrote state.

When a transaction fails with `StatusCode::MISSING_DATA`, the orchestrator
records a **missing-data signal** for that script: the reads that are compatible
with one of the script's known-unmet dependency tags
(`observed_unresolved_dependency_tags`), plus a hit counter. This is the single
most valuable signal for Phase 2 -- it says "this script wants state that nobody
has given it yet, and here is exactly which state".

**Scheduling.** In Phase 1 every fuzzer runs every round. Ordering (which
matters once Phase 2 starts throttling) is by
`(most missing-data hits, most recently productive, best seed score, largest corpus)`.

---

## 6. The saturation rule and the phase transition

Both phase decisions are evaluated inside the ~5 s reporting block of the main
loop, never per-iteration.

```text
              start
                |
                v
         +-------------+
         |  PHASE 1    |  every oneshot fuzzer runs each round
         |  bootstrap  |  bootstrap DUG grows from every execution
         +------+------+
                |
                |  ALL scripts have gone >= saturation_secs without new coverage
                |     (last_script_coverage_time[i] reset on each found_new)
                v
         build chains from the bootstrap DUG, create ChainFuzzers,
         install it as the live DUG, reset novelty timers
                |
                v
         +-------------+
         |  PHASE 2    |  oneshots throttled to 70% (full round every 20 iters)
         |  chains     |  chains at 60% (full round every 10 iters)
         +------+------+
                |
                |  report showed coverage_delta == 0
                |  AND no coverage/DUG novelty for saturation_secs
                v
              save checkpoint, return
```

The Phase 1 rule is per-script and conjunctive: `saturation_secs` (default 120)
must have elapsed since the last new coverage for **every** script. One script
still making progress keeps the whole campaign in Phase 1. Timers start at
campaign start, so a campaign with zero coverage anywhere still transitions after
`saturation_secs`.

The Phase 2 rule is global and additionally requires the last report to show zero
coverage growth. `last_phase2_novelty` is bumped by *either* new coverage *or* a
DUG change, from either a oneshot or a chain fuzzer, so a campaign that is still
learning new state relationships will not stop even if raw coverage has plateaued.

There is no transition back from Phase 2 to Phase 1, and no other stopping
condition -- no wall-clock or iteration cap.

---

## 7. Phase 2 -- multi-transaction fuzzing

### 7.1 Chain construction

At the transition, and again on every DUG-driven reconstruction,
`construct_seed_chains` builds chains **backward from a concrete seed node**:

```text
targets = all seed nodes, shuffled, then sorted by Reverse(seed_target_priority)
   priority = (#unresolved deps that some seed can produce,
               #types this seed produces,
               statefulness = 2*produced - consumed,
               this seed failed,
               #unresolved deps)

build_one_seed_chain(target K):
   nodes = [K]
   loop while len < max_chain_length:
       (pos, T) = first_unmet_dependency_in_seed_chain(nodes)   # simulate the chain
                                                                # from initial_types
       if none: break
       ranked = seed nodes producing T (object-equivalence aware),
                filtered by per-script repetition cap,
                sorted by (#currently-unresolved deps it resolves,
                           #types it produces, it failed, #types it reads)
       if empty: give up on this target
       insert a uniform pick from the top 3 at position `pos`
   reject if len <= 1 or dependencies still unsatisfied
   emit SeedChain { steps = script indices, seed_inputs = the concrete seeds }
```

Worked example -- target seed `K` runs script 11, which reads `T1`, which is not
in `initial_types`:

```text
  step 0:  [ K ]              first unmet = (pos 0, T1)
  step 1:  [ P , K ]          P is the best-ranked seed producing T1
  step 2:  P itself reads T0, also unmet
           [ Q , P , K ]      Q produces T0
  step 3:  no unmet dependencies remain -> emit
```

The randomized top-3 pick is the only stochastic element, so chain shape is
largely determined by the DUG.

`construct_seed_chains_for_targets` is the same routine restricted to seeds of a
given script -- used to chase missing-data signals.

### 7.2 Diversification

Raw candidate chains cluster on whatever module happens to be well connected.
`diversify_candidates` greedily reorders them, minimizing `chain_sort_key`:

```text
( Reverse(missing-data tags this chain's prefix would resolve),
  Reverse(missing-data hits on the target),
  Reverse(#unmet deps of the target that anything can produce),
  Reverse(#types the target defines),
  Reverse(#unmet deps of the target),
  count of already-selected chains with the same target module,
  count of already-selected chains with the same module signature,
  chain length )
```

and incrementing the two counts after each pick, so repeatedly targeting the same
module gets progressively penalized. "Module signature" is the run-length-encoded
sequence of module names along the chain, so `a->a->b` and `a->b` collapse.

### 7.3 `ChainFuzzer::run_one`

```text
seq_db_prob = 80 if local corpus empty
              50 if no new coverage for CORPUS_STALE_SECS (60 s)
              20 otherwise

with probability seq_db_prob, and only if a concrete-state-compatible prefix
exists, draw the prefix inputs from SequenceDb::pick_concrete_prefix_seed

for each step i:
    inputs[i] = mutate(prefix[i])                    if the prefix covers step i
                else fresh or mutated from the local corpus (Mutator::should_mutate)
    sender kept from the seed 70% of the time

clear trace buffer ONCE for the whole chain
for each step: execute; on failure, abort the chain early
flush trace buffer once; parse coverage for the whole chain
return (last status, corpus size, found_new, all writes, per-step profiles, executed prefix)
```

Prefix compatibility is not just "steps match". `find_concrete_prefix_seeds`
additionally requires the entry to be *state consistent*
(`entry_prefix_is_state_consistent`: replaying its reads and writes from
`initial_resource_tags()` never reads something unavailable) and *relevant* (its
produced types overlap the next step's exact unmet dependencies).

A `SequenceDb` entry is recorded for the longest all-succeeded prefix whenever
the run found coverage, changed the DUG, or progressed state.

### 7.4 Growing the chain population

Four mechanisms add chains, all capped at `MAX_CHAIN_FUZZERS = 50` and
deduplicated by `chain_instance_identity = sha3("move-fuzz-chain-instance-v1" ||
steps || serialized bootstrap seed)` -- the same step sequence with a different
bootstrap seed is a legitimately different fuzzer.

| Mechanism | Trigger | Source |
|---|---|---|
| initial construction | phase transition | `construct_seed_chains` over the bootstrap DUG |
| DUG reconstruction | `CHAIN_REBUILD_INTERVAL_SECS = 60` **and** the DUG or seed catalog changed since the last marker | `construct_seed_chains` |
| sequence extension | `SEQUENCE_MUTATION_INTERVAL_SECS = 30`, unconditional | `SequenceDb::propose_extensions`: append a DUG-linked consumer to an all-succeeded entry |
| sequence mutation | same timer | `SequenceDb::propose_mutations`: step deletion, step duplication, subsequence extraction, prefix/suffix splicing -- round-robined across strategies, each validated by `candidate_chain_is_valid` |
| targeted missing-data | any round where a `MISSING_DATA` status was observed | `construct_seed_chains_for_targets` restricted to the failing script |

Every new fuzzer is bootstrapped with the best seed available
(`bootstrap_chain_seed`): the chain's own concrete seed inputs if it has them,
else a `SequenceDb` concrete prefix, else per-step samples from the corresponding
oneshot corpora.

### 7.5 Scheduling in Phase 2

Oneshot fuzzers keep running (they are still the cheapest source of new seed
nodes) but at 70% of the population per round, with a full round every 20
iterations. Chain fuzzers run at 60%, full round every 10 iterations. Chain
ordering is
`(most missing-data hits on target, rarest target module, rarest module signature, most recently productive, best score, largest corpus)`.

---

## 8. Persistence and caching

```text
<project>/.move-fuzz/                    (or --state-dir; --reset-state wipes it)
 |
 +- auto_state.json          v6   the fuzz-loop checkpoint (written ~every 5 s
 |                                when dirty, and once more on stop)
 +- entrypoints_cache.json   v2   generated scripts + compiled bytecode
 +- fuzz_stats.json               progress/telemetry, rewritten every ~5 s
 +- package-cache/
     +- <sha3(pkg name || stable manifest path)>/
         +- build_cache_info.json   v2
         +- build/<PackageName>/...  copied `build/` output

<workdir>/autogen/          generated Move package: Move.toml + sources/*.move
<workdir>/cov.trace         the VM instruction trace, truncated before each execution
```

`<workdir>` is a `TempDir` copy of the project unless `--in-place` is given.

**Atomicity, precisely.** `auto_state.json`, `entrypoints_cache.json`, and
`build_cache_info.json` are always written to a `.tmp` sibling and `rename`d
(`state.rs`). `fuzz_stats.json` has *two* writers and only one of them is atomic:
the fuzz-loop writer in `fuzzer.rs` does tmp+rename, but the pre-loop stage
writer `cli.rs::write_frontend_stats` uses a bare `fs::write`. A tool polling
`fuzz_stats.json` during `building_packages`, `preparing_autogen`, or
`script_generation` can therefore observe a truncated file and must tolerate a
JSON parse failure by retrying.

### What invalidates what

| Artifact | Key | Invalidated by |
|---|---|---|
| package build | slot dir = `sha3(package name, project-relative manifest path)`; content key = `build_cache_fingerprint` = sha3 of package name, source digest, dev flag, assigned named addresses, compiler/language/bytecode config, **and every dependency's fingerprint** | any source edit, address reassignment, toolchain/config change, or any change in a transitive dependency (fingerprints chain through the topological order) |
| entrypoint cache | `ENTRYPOINT_CACHE_VERSION` + `entrypoint_cache_fingerprint` = sha3 of `max_trace_depth`, `max_call_repetition`, `max_script_gen_secs_per_function`, and per-package `(kind, name, source digest, bytecode/compiler/language version)` | changing any generation budget, rebuilding any package, bumping the schema |
| auto state | `AUTO_STATE_VERSION` + `campaign_fingerprint` = sha3 of `max_chain_length`, `max_chain_repetition`, `num_user_accounts`, the **sorted** entrypoint identities, and the **sorted** rendered initial resource writes | changing chain/account knobs, changing the script set, a different genesis/provisioning state, bumping the schema |

Because the entrypoint cache stores compiled bytecode, a cache hit skips stage 4
(graph exploration and script generation) and stage 5 (compilation of the autogen
package). It does **not** skip stage 3: `Model::new` runs unconditionally before
the cache is consulted, and the `Model` is needed afterward anyway to build the
`TypePool` that filters scripts with unsatisfiable generic constraints. Script
generation itself is not resumable: an interrupt mid-generation means starting
over until a complete entrypoint cache exists, and `--dry-run` never writes that
cache.

### Restoring `auto_state.json`

`restore_auto_state` is defensive and bails out (returning "start fresh", never
an error) on any mismatch:

1. schema version differs;
2. script count changed, or saved oneshot state is incomplete;
3. current entrypoint identities are not unique;
4. a saved entrypoint identity no longer exists -- otherwise a saved index is
   remapped through `old_to_new` (identity is
   `sha3("move-fuzz-entrypoint-v2", script name, target ident, generic ability bits, parameter renderings, bytecode)`);
5. a checkpointed sender address is no longer known to the executor;
6. the campaign fingerprint differs;
7. a script index is out of range.

Only then are the pieces restored: per-script corpora and coverage, the DUG
(abstract + seed layers, with script indices remapped), the `SequenceDb`, the
chain fuzzers (remapped by step *identity* where available, falling back to index
remapping), object dictionaries, the chain seed nonce, and the missing-data
signals. Note that the last identity check inside the oneshot restore loop fires
*after* `missing_data_signals` has already been overwritten and after some
fuzzers have been replayed, so that particular bail leaves partially restored
state behind; it is unreachable in practice because check 4 above already
validated every saved identity.

**Executor state is restored by replay, not by snapshot.** Each fuzzer's
`replay_log` is re-executed transaction by transaction against a fresh fork
(`replay_checkpoint_log`). That is what makes the restored state faithful, and it
is also the main cost of resuming a long campaign -- see limitations.

---

## 9. Execution and simulation layer

`TracingExecutor` (`executor/tracing.rs`) wraps
`aptos_language_e2e_tests::executor::FakeExecutor`. There is no network, no
consensus, no mempool: transactions are executed directly by `AptosVM` against an
in-memory state store.

**Setup.** `FakeExecutor::from_head_genesis().set_not_parallel()`, then the gas
schedule is read from genesis, `max_transaction_size_in_bytes` is raised to
`MAX_TRANSACTION_SIZE_IN_BYTES = 1 MiB`, and the modified schedule is written
back. Each package is then provisioned: framework packages only register their
named addresses (they are already in genesis); primary and dependency packages
create accounts for their named addresses and publish their modules, grouped and
signed per publishing address. `--num-user-accounts` extra user accounts are
created at the stable addresses produced by `stable_user_address` (`0xFD` in byte
0, the ordinal big-endian in the last 8 bytes), each funded with
`INITIAL_APT_BALANCE` = 1e15 octas (10M APT).

**Gas.** `InstructionGasParameters::zeros()`, `MiscGasParameters::zeros()`, and
`StorageGasParameters::unlimited()`; only transaction-level limits (size, max gas
units, price) are kept, and the per-value-node charge is left off. The fuzzer is
measuring reachability, not cost, and metering would just throttle throughput and
produce uninteresting `OUT_OF_GAS` outcomes.

**Forking.** `impl Clone for TracingExecutor` calls
`FakeExecutor::duplicate_with_assumption()`, which materializes the current state
delta as the base of a brand-new store. Every `OneshotFuzzer` and every
`ChainFuzzer` is constructed from a clone taken *after* provisioning, so all of
them start from the same fully provisioned chain and then diverge independently.

**Read tracking.** `RecordingStateView` wraps the state view and records every
`get_state_value` key, which is then classified into `ResourceRead`s. Reads are
recorded even for transactions that abort or are discarded -- that is where the
missing-data signal comes from. Writes come from the transaction output's write
set (`extract_resource_writes`), skipping deletions and code publishing.

**Initial state scan.** `scan_all_resource_writes()` walks the entire state delta
after provisioning and reports it as `ResourceWrite`s. This seeds
`DefUseGraph::initial_types` (so scripts reading genesis state are not treated as
having unmet dependencies) and every mutator's object dictionary, and it feeds
the campaign fingerprint.

**Coverage.** The fuzzer turns on the Move VM's instruction tracer
(`set_debugging_enabled(true)` + `enable_tracing(Some("<workdir>/cov.trace"))`).
Each iteration truncates the trace file, executes, flushes, and parses the file
with `move_coverage::coverage_map::CoverageMap::from_trace_file`. Coverage is a
set of `(module address, module name, function name, pc)` entries; per-fuzzer maps
are merged by `executor/mod.rs::merge_coverage`, which ignores zero-count entries
and reports whether any *new* entry appeared. Note that `move-coverage`
deliberately does not count script code ("Don't count scripts (for now)"), so the
driver scripts themselves contribute no coverage -- only the module code they
reach.

The trace file path, the tracer's enabled flag, and its buffered writer are
**process-global singletons** in `move_vm_runtime::tracing`. That is the hard
constraint that makes the fuzzing loop single-threaded.

---

## 10. Determinism and reproducibility

**What is seeded.** `--seed S` (default 0) feeds a splitmix64 mixer,
`derive_seed(base, salt)`:

- oneshot fuzzer `i` -> `derive_seed(S, i)`
- the chain-construction RNG -> `derive_seed(S, 0x9E3779B97F4A7C15)`
- chain fuzzer `n` -> `derive_seed(S, chain_seed_nonce++)`, and the nonce is
  checkpointed so resumed campaigns do not reuse streams
- within a `ChainFuzzer`, step `i`'s mutator -> `seed.wrapping_add(i)`

**What is deterministic given identical inputs.** Static analysis and script
generation: the registries are `BTreeMap`s, primary declarations are sorted
(entry functions first, then by ident), provider candidates are sorted by
`(package rank, arity, generics, ident)`, and ability-set enumeration is a
deterministic powerset. Absent a budget cutoff, the same project produces the
same scripts in the same order.

**What is not, and why.**

1. **Wall clock is load-bearing.** The Phase 1 -> Phase 2 transition, the stop
   rule, chain rebuild and sequence-mutation intervals, corpus staleness, energy
   scheduling by "time since last new coverage", and the per-function script
   generation budget are all time-driven. Two runs with the same seed on
   different machines will diverge.
2. **Named addresses can be random.** In `deps.rs::resolve`, a named address that
   is `PkgNamedAddr::Devel` or `PkgNamedAddr::Unset` gets
   `Ed25519PrivateKey::generate(&mut OsRng)` -- a fresh address every process.
   Those addresses appear in provisioning writes, which feed the campaign
   fingerprint, so for such projects a cross-process resume is expected to be
   rejected and the campaign restarts (gracefully, but it restarts). Projects
   with fixed addresses in `Move.toml` do not have this problem.
3. **Temp workdir.** Without `--in-place` the project is copied to a fresh
   `TempDir`. Package cache keys are computed against the *project-relative* path
   (`stable_project_path`) so the build cache survives, but absolute paths baked
   into build artifacts do not.
4. **Synthetic tags.** Table/raw state keys are hashed with `DefaultHasher` over
   a `Debug` rendering. Stable within a build; not guaranteed stable across Rust
   releases -- yet these tags are persisted in `auto_state.json`.
5. **Iteration order of the state scan.** `get_state_delta` returns a `HashMap`;
   downstream consumers either use ordered collections or sort before hashing
   (the campaign fingerprint sorts rendered writes), so this does not leak into
   results, but do not rely on scan order.

**Reproducing an execution.** There is currently no crash-reproducer file. The
only faithful reproduction path is the per-fuzzer `replay_log` inside
`auto_state.json`: it is an ordered transcript of every executed `SeedInput`, and
re-running it against a fresh provisioned executor reconstructs the exact state.

---

## 11. Budgets and tuning constants

| Constant | Value | Where | Controls |
|---|---|---|---|
| `max_trace_depth` | 3 | CLI | call-trace depth during graph exploration |
| `max_call_repetition` | 1 | CLI | repeats of one instantiation within a trace |
| `max_script_gen_secs_per_function` | 600 (`0` = off) | CLI | wall-clock cap per primary function |
| `MAX_DERIVED_GRAPHS_PER_PROCESS` | 4096 | `prep/graph.rs` | graphs per primary function (reset by `process()`) |
| `MAX_EXTERNAL_PROVIDER_MATCHES_PER_DATATYPE` | 64 | `prep/graph.rs` | external provider matches per datatype |
| `MAX_SCRIPTS_PER_FUNCTION` | 24 | `prep/model.rs` | emitted scripts per primary function |
| `MAX_INSTANTIATIONS_PER_STRUCT` / `MAX_GENERIC_TYPE_POOL_ROUNDS` | 8 / 3 | `fuzzer.rs` | generic type-pool expansion |
| `num_user_accounts` | 3 | CLI | fuzzable user accounts |
| `saturation_secs` | 120 | CLI | both phase-boundary timers |
| `max_chain_length` / `max_chain_repetition` | 5 / 2 | CLI | chain shape |
| `MAX_CHAIN_FUZZERS` | 50 | `executor/sequence.rs` | live chain fuzzers |
| `MAX_ONESHOT_CORPUS` / `MAX_CHAIN_CORPUS` | 256 / 160 | `oneshot.rs` / `sequence.rs` | per-fuzzer corpus |
| `MAX_SEQUENCE_DB_ENTRIES` | 4096 | `executor/sequence.rs` | shared sequence pool |
| `CHAIN_REBUILD_INTERVAL_SECS` / `SEQUENCE_MUTATION_INTERVAL_SECS` | 60 / 30 | `fuzzer.rs` | chain-population growth cadence |
| `SEQ_DB_PROB_BASE` / `_STALE` / `_EMPTY_CORPUS` | 20 / 50 / 80 | `executor/sequence.rs` | odds of drawing a shared prefix seed |
| `CORPUS_STALE_SECS` | 60 | `executor/sequence.rs` | when a local corpus counts as stale |
| `GEN_PROB` / `MUT_PROB` | 50 / 50 | `mutate/mutator.rs` | fresh-generation vs. mutation split |
| `MAX_TRANSACTION_SIZE_IN_BYTES` | 1 MiB | `executor/tracing.rs` | raised txn size limit |
| `INITIAL_APT_BALANCE` | 1e15 octas (10M APT) | `executor/tracing.rs` | per-account funding |

---

## 12. Known limitations and future work

### Architectural

1. **Single-threaded by construction.** The VM tracer writes to one
   process-global file, and each fuzzer owns a private state fork. Parallelism
   needs either per-worker trace sinks or an in-VM coverage counter that does not
   go through a file at all.
2. **Resume is replay-based and unbounded.** Every executed seed is appended to a
   `replay_log` that is never pruned, serialized in full into `auto_state.json`,
   and re-executed on resume. Both the state file and the restart cost grow
   linearly with total executions. A state-store snapshot (or a bounded,
   coverage-preserving log) would fix this.
3. **No cross-fuzzer state sharing.** Because forks are private, discovered state
   propagates only as object-address hints and DUG/`SequenceDb` knowledge. Two
   scripts can each set up half of a precondition and never combine them except
   through an explicitly constructed chain.

### Oracle and reporting

4. **There is no bug oracle.** `ExecStatus` distinguishes success, declared
   aborts, intrinsic failures, discards, and out-of-gas, and new statuses are
   logged once and counted -- but nothing is triaged, minimized, or written out
   as a reproducer. A campaign that finds a genuine invariant violation reports
   it as one more abort code in `fuzz_stats.json`.
5. **No property or invariant hooks**, no differential oracle, no coverage export
   on exit, no corpus import/export.
6. **Gas is disabled**, so gas-metering bugs and DoS surfaces are out of scope by
   construction.
7. **`auto` has no time or iteration cap**; the only self-termination is Phase 2
   saturation.

### Generation

8. **The reachable surface is `public`-only.** `public(package)` and
   `public(friend)` functions are excluded, and the filter is a hand-rolled
   source tokenizer (`parse_script_public_functions`) applied on top of bytecode
   visibility -- so if a module's source cannot be located, the fuzzer silently
   falls back to bytecode visibility alone.
9. **Generation is budget-sensitive.** Hitting `MAX_DERIVED_GRAPHS_PER_PROCESS`
   or the wall-clock cap truncates exploration. The *configured* budget is part of
   the entrypoint-cache fingerprint, but machine-speed jitter is not, so the same
   configuration can yield different script sets on different machines.
10. **`vector<Function>` is `todo!()`** (`prep/typing.rs`), and each `Function`
    parameter is bound to one pre-selected callee per combination rather than
    being fuzzed over.
11. **Script identity embeds the ordinal.** `ScriptSignature::name` is
    `fuzz_script_<N>`, and that name is hashed into the entrypoint identity, so
    inserting one script early shifts the names -- and thus the identities -- of
    everything after it. In practice the entrypoint cache regenerates the whole
    set at once, so this rarely bites, but it does weaken the identity-based
    remapping in `restore_auto_state`.

### Dynamic model

12. **Object modeling is heuristic.** An address counts as an object only once an
    `0x1::object::ObjectGroup` write has been seen at it; `note_object_address` is
    called from `add_initial_resource_tag` and `add_def` but not `add_use`. Object
    arguments are matched by `DatatypeIdent` with type arguments ignored, so a
    wrong instantiation shows up as an abort rather than being avoided.
13. **Synthetic table/raw tags are `DefaultHasher`-derived** yet persisted (see
    section 10).
14. **Legacy code paths are still present but unwired.** `discover_profiles` has
    no callers at all -- not even tests. `DefUseGraph::from_profiles`,
    `construct_chains` / `build_one_chain`, and `SequenceDb::pick_prefix_seed` /
    `find_prefix_seeds` are reachable only from `#[cfg(test)]` code. The live
    pipeline builds the DUG online via `add_seed_observation`, constructs chains
    via `construct_seed_chains`, and draws prefixes via
    `pick_concrete_prefix_seed`. These should be either wired back in as a
    fallback or deleted -- keeping two chain-construction algorithms alive
    invites drift.

### Candidate next steps

- Parallel workers with per-worker coverage sinks.
- Snapshot-based checkpointing to replace replay logs.
- A real oracle layer: reproducer emission, crash de-duplication, minimization,
  and optional Move-level invariant hooks.
- Test-mode compilation to reach `public(package)` / `friend` surface.
- Retire or reintegrate the offline profiling path.

---

## 13. Where to start reading

In this order:

1. `src/cli.rs::cmd_auto` -- every flag and everything that happens before the
   fuzzer is entered, including package caching and the `autogen` package.
2. `src/fuzzer.rs::entrypoint` -- the entire campaign in one function. Read the
   main `loop`: oneshot round, chain round, missing-data chain spawning, report +
   checkpoint, phase transition, Phase 2 growth, stop check.
3. `src/executor/sequence.rs`, top half -- `ResourceTag`, `ExecResourceProfile`,
   `DefUseGraph`. This is the data model everything else is expressed in.
4. `src/prep/model.rs::Model::populate` -- the generation loop; then
   `prep/graph.rs::GraphBuilder::process` / `is_feasible` and
   `prep/canvas.rs::DriverCanvas::try_build`.
5. `src/executor/oneshot.rs::run_one` and
   `src/executor/sequence.rs::ChainFuzzer::run_one` -- what one iteration costs.
6. `src/state.rs` -- if you are changing anything that gets checkpointed.

## 14. Debugging a run

- **`fuzz_stats.json` first.** It names the current stage
  (`building_packages`, `preparing_autogen`, `script_generation`, then the
  fuzz-loop stats) and is rewritten every few seconds. A "stuck" run is usually
  a slow package build or a function eating its whole generation budget. During
  the pre-loop stages the file is not written atomically, so retry on a parse
  error rather than concluding the run died.
- `-v` / `-vv` / `-vvv` for info / debug / trace. `-vv` prints the per-script and
  per-chain tables, coverage events, and new status codes.
- `--dry-run` stops after script *generation*: it writes
  `<workdir>/autogen/sources/*.move` and returns without compiling them and
  without populating the entrypoint cache. It is the right flag for debugging
  resolution, model building, and codegen -- but if you need bytecode or want to
  warm the cache, run without it.
- `--in-place` keeps `autogen/` and `cov.trace` in the project directory so you
  can read the generated scripts. Combine with `--dry-run` to inspect codegen.
- `--reset-state` for a clean slate; `--state-dir` to keep experiments apart.
- The crate has ~157 unit tests, most of them in `executor/sequence.rs` covering
  DUG semantics, chain construction, `SequenceDb` matching/extension/mutation,
  and snapshot round-trips. `cargo test -p move-fuzz --lib` is the fast loop.

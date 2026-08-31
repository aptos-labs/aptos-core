# Type Operations in MonoMove

Inventory of every type-related operation in the MonoMove VM, where it runs in
the pipeline, and the properties that matter for staging, performance, and
security. Implementation lives under `third_party/move/mono-move/`.

This is a map of the current system, not a redesign. Existing `TODO(...)`
labels at call sites remain the source of truth for planned work.

Related: [loader/DESIGN.md](../loader/DESIGN.md),
[specializer/DESIGN.md](../specializer/DESIGN.md),
[vm_security_and_correctness.md](vm_security_and_correctness.md),
[native_functions_interface.md](native_functions_interface.md).

---

## 1. Type model

A single **canonical type DAG** lives in the global arena (`core/src/types.rs`).
Each node is an interned [`Type`]; composites point at children via
`InternedType` / `InternedTypeList`. **Pointer equality implies structural
equality**, except for [`Type::TypeParam`]: parameters are interned by index
alone, so `T0` in two different generic scopes is the same pointer.

| Variant | Identity | Slot size |
|---|---|---|
| Primitives (`Bool` … `Signer`) | statics (`BOOL_TY`, …); no arena alloc | intrinsic |
| `Vector { elem }` | interned wrapper | 8-byte heap pointer |
| `ImmutRef` / `MutRef` | interned wrapper | 16-byte fat pointer |
| `Nominal { module_id, name, ty_args }` | interned identity (struct or enum) | published `ValueLayout`, or none if open |
| `Function { args, results, abilities }` | interned; abilities are part of identity | 8-byte heap pointer |
| `TypeParam { idx }` | interned by index | none until substituted |

Lists of types (function params/returns, generic args) are themselves interned
(`InternedTypeList`). The empty list is a static.

Viewing a node is `view_type` / `view_type_list` / `view_name`: they widen the
arena lifetime to `&'static`. Callers must not stash those references past
maintenance (arena reset). See the safety contract on `types.rs`.

Derived identities interned alongside types:

- `InternedIdentifier` — identifier strings
- `InternedModuleId` — `(address, name)`
- `InternedFunctionRef` — `(module_id, func_name, ty_args)` (closure / lazy
  function identity)

---

## 2. The `Interner` API

Trait in `core/src/interner.rs`. The production implementation is
`ExecutionGuard` (`global-context/src/context.rs`): allocate in the worker
arena, then insert into a `DashMap` keyed by structural hash. Duplicate
allocations are discarded; the canonical pointer is returned.

| Constructor | Builds |
|---|---|
| `type_param_of(idx)` | `Type::TypeParam` |
| `vector_of(elem)` | `Type::Vector` |
| `immut_ref_of` / `mut_ref_of` | references |
| `function_of(args, results, abilities)` | function types |
| `type_list_of(&[InternedType])` | interned list (empty → static) |
| `nominal_of(module_id, name, ty_args)` | struct/enum type |
| `function_ref_of(...)` | interned function identity |
| `module_id_of` / `identifier_of` | module ids / names |
| `subst_type` / `subst_type_list` | substitute params, re-intern |

Interning always **allocates then probes**. Composite constructors do not look
up by `SignatureToken` first. `try_intern_for_module` / `SignatureTokenKey`
exist for cross-format hash/eq (`global-context/src/context/types.rs`) but
`intern_sig_token` does not use the probe-before-allocate path
(`TODO(perf)` on `prepared_module.rs`).

Substitution (`ExecutionGuard::subst_type`):

1. Empty `ty_args` → identity (no walk).
2. Primitives unchanged.
3. Recurse into children; if every child pointer is unchanged, reuse the
   parent pointer.
4. Otherwise re-intern the rewritten node (`vector_of`, `nominal_of`, …).
5. `TypeParam { idx }` indexes `ty_args`; out of range is
   `TypeSubstitutionError::IndexOutOfBounds` (invariant violation).

The walk is recursive. Inner-type hashes are not cached, so insertion during
re-interning is `O(N²)` in type DAG size (`TODO(perf, metering)` on
`subst_type`). There is **no type-size cap** at intern time
(`TODO(metering)` on the `Interner` trait).

---

## 3. Operations other than intern / subst

These consume interned types. None of them mutate the type DAG.

### 3.1 Structural queries

| Op | File | What |
|---|---|---|
| `view_type` / `view_type_list` / `view_name` | `types.rs` | arena deref |
| `strip_ref` | `types.rs` | `&T` / `&mut T` → `T` |
| `convert_mut_to_immut_ref` | `types.rs` | intern `&T` from `&mut T` |
| `is_closed_type` | `types.rs` | no `TypeParam` anywhere; **uncached recursive**, exponential on shared DAGs |
| `is_assignable(expected, actual)` | `types.rs` | pointer eq, plus function-ability variance and covariant `&T` |
| `Type::short_name` / `is_u64` | `types.rs` | kind / fast-path gate |
| `display_type` / `type_to_string` | `types.rs` | unbounded recursive print |
| `intrinsic_slot_size_and_align` | `types.rs` | size/align without layout table |

`is_assignable` is **not** used in destack or lowering today. It is tested in
`testsuite/tests/assignability_tests.rs`. Runtime values are untyped bytes;
assignability is assumed from the verifier + destack typing.

### 3.2 Abilities

`AbilityCalculator` (`core/src/abilities.rs`) walks interned types under a
fixed type-parameter constraint vector and a nominal→`StructHandle` lookup.

- Primitives / signer / refs / function abilities are immediate.
- Vectors: `VECTOR` abilities predicated on the element (never phantom).
- Nominals: **declaration in the asking module**, not the definition. The
  handle may be a strictly narrower ability/phantom set than the defining
  module (`TODO(correctness)` on that file).
- Memo keyed on `InternedType` only — sound only because lookup and
  `ty_param_ctx` are fixed for the calculator's lifetime.

Used today when destack builds a `PackClosure` function type (intersect
callee abilities with captured-value abilities). Not used by lowering or the
interpreter.

### 3.3 Bitwise-copy classification

`PreparedModule::is_bitwise_copy_type` (`prepared_module.rs`): whether a
byte-copy of a frame slot is a complete copy (no owned heap). Conservatively
`false` for type parameters, imported nominals, and nested nominal type
arguments that are not a bare parameter. Uncached recursion at module load /
optimize (`TODO(metering, perf)`). Destack `optimize.rs` uses it to decide
whether `Copy` can stay a bitwise move.

### 3.4 Tags (TypeTag / StructTag)

| Op | Direction | Notes |
|---|---|---|
| `intern_type_tag` / `intern_struct_tag` | `TypeTag` → interned | recursive; used for txn type args and resource keys |
| `type_tag_of` / `struct_tag_of` | interned → `TypeTag` | `None` for refs, functions, open params; **re-walks every call** (`TODO(perf)` cache) |
| `nominal_tag` | interned → `StructTag` | storage keys; fails if not nominal |
| `module_id_of` (interner helper) | interned module → `language_storage::ModuleId` | |

`type_tag_of` has no depth/size bound (`TODO(correctness)`: either bound here
or prove the txn already charged for the tag).

### 3.5 Layouts and descriptors

Published in the global context, keyed by **closed** `InternedType`:

- `ValueLayout` / `LayoutId` — in-memory size, align, field offsets, BCS
  size flags (`LayoutProvider::size_and_align`, `layout_by_ty`).
- `ObjectDescriptor` / `DescriptorId` — GC tracing (vector by element type,
  struct by type, captured-data, reserved trivial/closure).
- Enum variant layouts — separate map `enum_variants_by_type`.

Discovery is a post-order DFS (`discover_type_metadata` in
`specializer/src/lower/context.rs`):

1. `subst_type(ty, function_ty_args)` first.
2. Skip if already visited (interned pointer).
3. Open nominals (`!is_closed_type`) are **not** published — publishing would
   poison the `OnceLock`-style layout for later concrete substitutions.
4. Nominal field types are substituted under the **nominal's** `ty_args`,
   then discovered with an empty substitution table.
5. Missing field info (module not loaded) defers: no layout published.

`LayoutProvider::size_and_align` uses intrinsic sizes for primitives/refs/
vectors/functions; nominals need a published layout.

### 3.6 Function-signature instantiation

`PreparedModule` stores:

- `function_signatures[handle]` — interned params/returns, possibly open.
- `function_instantiation_signatures[inst]` — **already**
  `subst_type_list`'d under the call site's own type arguments. May still
  contain the **caller's** free `TypeParam`s.

Lowering additionally `subst_type_list`s those under the function being
monomorphized (`instantiate_callee_signature`, home-slot types, returns).

---

## 4. Pipeline stages

```text
  TypeTag / bytecode SignatureToken
           │
           ▼
  ┌────────────────────┐
  │  Intern (arena)    │  identifiers, module ids, types, lists
  └────────┬───────────┘
           │
           ▼
  ┌────────────────────┐
  │  PreparedModule    │  per-index interned pools + call-site subst
  └────────┬───────────┘
           │
           ▼
  ┌────────────────────┐
  │  Destack (module)  │  polymorphic stackless IR; more intern/subst
  └────────┬───────────┘
           │
           ▼
  ┌────────────────────┐
  │  Load / cache      │  ModuleIR in global module cache
  └────────┬───────────┘
           │  first call / eager policy
           ▼
  ┌────────────────────┐
  │  Discover layouts  │  subst + DFS; may load more modules
  └────────┬───────────┘
           │
           ▼
  ┌────────────────────┐
  │  Lower (JIT)       │  subst home/callee types → sized slots / micro-ops
  └────────┬───────────┘
           │
           ▼
  ┌────────────────────┐
  │  Execute           │  mostly type-erased; types on natives, storage, closures
  └────────────────────┘
```

Stages 1–3 are **once per module version** (cache miss). Stages 4–5 are
**once per (function, ty_args)** (instantiation cache). Stage 6 is per
transaction. Maintenance (`MaintenanceGuard`) can reset the arena and every
pointer-keyed cache between execution windows.

### Stage A — Transaction / storage ingress

**When:** user txn payload, resource-group blobs, system calls.

| Work | Where |
|---|---|
| Intern txn type arguments from `TypeTag` | `aptos-transaction-executor/src/user_txn/execute.rs` (`intern_type_tag`, then `type_list_of`) |
| Intern `StructTag`s from resource groups | `providers.rs` (`intern_struct_tag`) |
| Intern entry function module/name | `intern_address_name`, `intern_identifier` |
| Substitute callee params for argument placement | `calls.rs` `param_types` → `subst_type_list` |

Untrusted `TypeTag`s from the payload are interned into the **global** type
cache (`TODO(security)` on `execute.rs`: cache pollution / size).

### Stage B — Module prepare (cache miss, after deserialize + verify)

`PreparedModule::build` (`prepared_module.rs`):

1. Intern every identifier and module handle.
2. Intern every `SignatureToken` in the signature pool (`intern_sig_token`,
   recursive).
3. Intern each struct handle as a nominal whose `ty_args` are
   `type_param_of(0..n)` (the generic form used later as a subst template).
4. Intern field types (declared structs/enums) and constant types.
5. Intern function-handle param/return **lists**.
6. For each function instantiation: intern the call-site `ty_args` list,
   then `subst_type_list` into the handle's params and returns.

After this, bytecode indices map to interned pointers with no further intern
of those table entries.

### Stage C — Destack (bytecode → polymorphic stackless IR)

`specializer/src/destack/`. Still **open** types; `TypeParam` nodes remain.

**SSA conversion** (`ssa_conversion.rs`) maintains `local_types` and
`value_id_types`. It interns/substitutes when bytecode encodes an
instantiation or a derived type:

| Bytecode family | Type work |
|---|---|
| Locals / params | already interned in `PreparedModule` |
| `Pack` / `Unpack` (non-generic) | `interned_nominal_def_type_at` |
| `PackGeneric` / field/variant instantiations | `type_list_of` of signature + `subst_type` of generic nominal / field |
| `ImmBorrowField` / mut / generic | intern `immut_ref_of` / `mut_ref_of` of (possibly substituted) field type |
| `FreezeRef` | `convert_mut_to_immut_ref` (interns `&T`) |
| Global storage (`MoveTo`, `BorrowGlobal`, …) | intern refs of resource type; generic forms subst first |
| Vector borrow | intern `&` / `&mut` of element type |
| `CallGeneric` | `fun_inst_parts` → intern `ty_args` list (params/returns already subst'd in `PreparedModule`) |
| `PackClosure` | `function_of` of remaining params + returns; `AbilityCalculator` on captures |
| Constants | `interned_constant_type_at` |

**Slot allocation** (`slot_alloc.rs`) is type-keyed reuse: a dead Home slot
is recycled only for a later SSA value of the **same interned type pointer**.
Output: `home_slot_types: Vec<InternedType>` (still possibly open).

**Optimize** (`optimize.rs`) consults `is_bitwise_copy_type` for copy
elision. Compaction remaps `home_slot_types`.

IR instructions embed interned types for owners (struct/enum), resources,
closure types, and call `ty_args` lists. Those embeddings are **not** fully
concrete until lowering substitutes the enclosing function's `ty_args`.

### Stage D — Load

`loader/src/loader.rs`. On miss: prepare + destack, insert `LoadedModule`.
Does not lower until a function is needed (lazy) or all local layouts are
known (eager). Instantiated functions are cached on
`(func_name, InternedTypeList)` including **phantom** args
(`TODO(perf)`: phantoms distinguish identical code).

Loading more modules for layouts is a type-driven walk: discovering a
nominal whose defining module is not loaded pulls that module (see
`LoweringPolicy` and loader DESIGN).

### Stage E — Layout / descriptor discovery

`try_discover_types_for_lowering_in_function` (or `_in_module` with empty
`ty_args`):

Walks home slots, return types, call signatures (after
`instantiate_callee_signature`), native resource/layout type args, field
owners in instructions, `PackClosure` captures, global-storage resource
types, constant types.

Each type: `subst_type` under the function instantiation, then
`discover_type_metadata`. Side effects: `publish_layout`, vector/struct
descriptors, enum variant layouts, captured-data descriptors.

May load modules (`SpecializerContext::get_fields`) when a nominal is not
local. Gas for those modules is charged by the loader around this call.

### Stage F — Lowering (polymorphic IR → monomorphic micro-ops)

`try_build_context` then `try_lower_function` (`lower/context.rs`,
`lower/translate.rs`).

1. Arity check: `ty_args` length vs declared type parameters.
2. Intern `home_slot_types` as a list (round-trip; `TODO(perf)`: subst
   should take a slice), `subst_type_list`, then `layout_slots` /
   `gc_layout_supports`. Open or unpublished layouts → skip lowering.
3. Same for own returns.
4. Per instruction: `subst_type` of embedded owner/resource types;
   require closed (`is_closed_type`); look up field offsets from
   `layout_by_ty`; resolve enum layouts; intern `function_ref_of` for
   closures.
5. Emit sized-slot micro-ops. Types are **compiled into offsets, sizes,
   descriptor ids** — not carried on most ops.

Native call sites keep an `InternedTypeList` of type arguments on the
micro-op (`CallNative`), because natives are not monomorphized.

### Stage G — Execution

The interpreter (`runtime/src/interpreter.rs`) is register-based over raw
bytes. Remaining type operations:

| Path | Type work |
|---|---|
| `CallNative` | pass `view_type_list(ty_args)` into `NativeContext::ty_arg` |
| Constant load | `interned_constant_type_at` + BCS bytes |
| Global storage | `InMemoryStorageKey::Resource { ty }` keyed by interned type; `nominal_tag` / `struct_tag_of` for `StateKey` |
| Resource groups | `resource_group_of(ty)` from defining-module metadata |
| Closures | `InternedFunctionRef` → loader `load_function(module, name, ty_args)` |
| BCS / compare / size | `layout_by_ty` + `value_utils` walks |
| `type_info` natives | `view_type` + `type_to_string` (unbounded; `TODO(metering)`) |
| Events | `type_tag_of` on the event payload type |

Micro-op verifier (`runtime/src/verifier.rs`) uses interned types only for
a few size queries (`type_size`), not a full typecheck.

### Stage H — Maintenance

`MaintenanceGuard::reset_all_caches` clears identifier, module-id, type,
type-list, function-ref maps, module cache, descriptors, and layouts
**before** arena reset. Any interned pointer held across this boundary is
invalid.

---

## 5. Who trusts what

Destack documents verifier invariants (`destack/translate.rs`): stack
balance, type consistency of bytecode, index bounds, type-parameter
indices in range, borrow-checker so type-keyed slot recycling is sound.

Substitution out-of-range is treated as an invariant violation (the
enclosing context should already have matching arity). Lowering's arity
mismatch currently skips (`TODO(correctness)`: reachable from snapshot
printing, should not be reachable in execution).

Ability lookup uses the **asking module's** handle. Compatibility with the
definition is a bytecode-verifier / linking assumption
(`TODO(correctness)` on `abilities.rs`).

Values are untyped bytes. Type confusion is prevented by:

- verifier + destack producing consistent `home_slot_types`
- lowering baking sizes/offsets from published layouts
- GC using descriptors published for those same closed types
- natives being TCB (`native_functions_interface.md`)

---

## 6. Staging and optimization notes

Observations from the current call graph; several already have inline TODOs.

**Repeated substitution of the same open type.** Destack substitutes at
generic Pack/field/call sites into IR embeddings. Lowering substitutes
those embeddings again under the function `ty_args`. Discovery substitutes
a third time. A closed interned type after the first subst could be cached
on `(open_ty, ty_args)` — the interned result is already the natural key.

**Allocate-then-dedup.** `intern_sig_token` and every `Interner`
constructor allocate a `Type` node even on a hit. Probe-before-allocate
(and using `SignatureTokenKey` / `try_intern_for_module`) is the existing
perf TODO. Shared signatures (`vector<T>`, `&T`) make this hot at prepare.

**Hashing is recursive and uncached.** `TypeInternerKey::hash` walks the
DAG (`TODO(metering)`: non-recursive). Combined with subst re-intern, this
is the `O(N²)` note on `subst_type`.

**Slice vs interned list.** Lowering intern's `home_slot_types` only to
call `subst_type_list`. A subst API over `&[InternedType]` would drop that
canonicalization.

**Phantom type arguments** distinguish lowered-function cache keys even
when they do not affect code (`loader.rs`).

**Layout keyed only by closed types.** Correct (avoids poisoning). Means
generic IR cannot precompute offsets; lowering must remain JIT. Eager
policy only helps non-generic functions.

**`is_closed_type` / `is_bitwise_copy_type` / `type_tag_of` / `type_to_string`**
are recursive, uncached (except ability memo). Shared interned subtrees
make `is_closed_type` exponential. Memo by interned pointer is the natural
fix and is already noted.

**Abilities at destack only.** Closure function types need abilities in
the interned `Type::Function` identity. If more of destack needed
abilities, hoist one `AbilityCalculator` per function (`TODO(perf)` in
`closure_type`).

**Tag conversion on the hot path.** Storage keys and events rebuild
`StructTag`/`TypeTag` from the interned graph. Cache per interned type in
the global context (`TODO(perf)` on `type_tag_of`).

**Natives vs monomorphization.** `type_name` / `type_of` re-traverse at
runtime; comments note the specializer could bake the string/TypeInfo
(`natives/src/type_info.rs`). Layout-using natives (`bcs`, events) still
need published layouts.

**Type-keyed slot recycling** is already at destack, using interned
pointer equality. It must stay on types from the same parameter scope.

---

## 7. Security and metering properties

From `docs/vm_security_and_correctness.md` applied to this map:

| Property | How types are involved |
|---|---|
| Boundedness | Interning, subst, `is_closed_type`, tag conversion, `type_to_string`, layout DFS, and `TypeInternerKey` hash/eq are all recursive over attacker-shaped type DAGs. There is no intern-time size/depth cap. |
| Cache pollution | User `TypeTag`s and module signatures intern into process-global `DashMap`s until maintenance. Distinct types never evict. |
| Derived-cache coherence | Layouts and descriptors are derived from interned types + loaded field tables. Module upgrade must not serve a stale layout for the same interned identity (enum variant add is the documented case). Pull-based loader charging is the current approach. |
| Arena lifetime | Interned pointers in IR, keys, and `InMemoryStorageKey` are invalid after maintenance reset. |
| TypeParam aliasing | Pointer equality of `TypeParam { idx }` is **not** a scope-safe equality. `is_assignable` documents this. Bitwise-copy and ability memos are only sound within one scope. |
| Charge-before-work | Loader charges modules used for layouts. Intern/subst/hash/tag walks during prepare, destack, discovery, lowering, and natives are not all metered (`TODO(metering)` at those sites). |
| Determinism | Interning is structural; DashMap is not iterated for semantics. Tag strings and displays must stay canonical if they ever affect execution (type_info, crypto algebra matching). |
| No panic on untrusted data | Subst OOB is an error. `intern_type_tag` uses `anyhow`. View helpers are `unsafe` under the guard contract. |

Highest-leverage staging for **both** speed and bounds:

1. Cap and meter type DAG size at intern (ingress + `intern_sig_token` +
   subst re-intern).
2. Memoize closedness, tags, and subst results by interned pointer.
3. Keep layouts/descriptors published only for closed types; invalidate
   with module versions, not interned identity alone.
4. Bound or specialize native type reflection instead of unbounded
   `type_to_string` at call time.

---

## 8. File index

| Area | Path |
|---|---|
| Type DAG, views, assignability, closedness | `core/src/types.rs` |
| Interner trait, tags | `core/src/interner.rs` |
| Prepare + `intern_sig_token` / `intern_type_tag` | `core/src/prepared_module.rs` |
| Abilities | `core/src/abilities.rs` |
| Layout table API | `core/src/value_layout.rs` |
| Storage keys / `nominal_tag` | `core/src/storage/resource_provider.rs` |
| DashMap intern + subst + layout publish | `global-context/src/context.rs`, `context/types.rs` |
| Load / instantiate cache | `loader/src/loader.rs` |
| Destack intern/subst/abilities/slots | `specializer/src/destack/` |
| Discovery + lowering subst/layouts | `specializer/src/lower/context.rs`, `translate.rs` |
| Interpreter / natives / BCS | `runtime/src/interpreter.rs`, `native_context.rs`, `value_utils.rs` |
| Txn type-arg intern | `aptos-transaction-executor/src/user_txn/execute.rs` |
| Tests | `global-context/tests/subst_tests.rs`, `testsuite/tests/{assignability,abilities,types}_tests.rs` |

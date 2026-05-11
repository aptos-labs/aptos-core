# Native Functions: Current Inventory & VM Requirements

> Companion to `docs/native_functions.md` (which describes the *future* MonoMove direction). This document is a survey of the **current** Move VM native interface and *every* native function in `aptos-move/framework/`, grouped by the VM capability each one needs. The goal is to enumerate, before we design the MonoMove native interface, exactly what the new interface must support — and to surface the "tricky" natives that will not slot into a minimal interface.

This is a status snapshot of what exists today, not a design proposal. Numbers and exact file paths are correct as of 2026-05-11; lists of natives may drift as new ones land. Cardinalities are approximate counts of distinct registered names (not Rust function definitions, since several names share an implementation). The contents here were compiled from static reading of the source rather than runtime tracing, so some categorizations, capability mappings, and call-graph claims are likely wrong in places — treat this document as a starting point for review, not a settled reference, and verify anything load-bearing against the code before relying on it.

## Sources surveyed

**Old VM interface** (`third_party/move/move-vm/`):
- `runtime/src/native_functions.rs` — `NativeFunction` type, `NativeContext`, `LoaderContext`, registry (`NativeFunctions::resolve`)
- `types/src/natives/function.rs` — `NativeResult` enum and `pop_arg!` macro
- `runtime/src/native_extensions.rs` — `NativeContextExtensions`, `SessionListener`
- `types/src/gas.rs` — `DependencyGasMeter`, `NativeGasMeter` traits

**Safe wrapper** (`aptos-move/aptos-native-interface/`):
- `lib.rs`, `builder.rs`, `context.rs`, `errors.rs`, `native.rs`, `helpers.rs`, `rayon_pool.rs`

**Native implementations** (`aptos-move/framework/`):
- `move-stdlib/src/natives/` — 11 files
- `natives/src/` — top-level Aptos framework natives (account, event, code, …)
- `natives/src/aggregator_natives/` — aggregator v1/v2 + delayed field
- `natives/src/cryptography/` (incl. `algebra/`) — all crypto natives
- `table-natives/src/lib.rs`

# Part I — The current native function interface

The unsafe interface and the safe wrapper layer.

## 1.1 Unsafe interface (Move VM)

Every native, at the lowest level, has the signature

```rust
pub type UnboxedNativeFunction =
    dyn for<'a> Fn(&mut NativeContext, &'a [Type], VecDeque<Value>)
        -> PartialVMResult<NativeResult>
        + Send + Sync + 'static;
```

Implications:
- **Arguments** arrive as a `VecDeque<Value>`. Natives pop from the *back* (last argument first). There is no in-place stack access; the interpreter materializes the arguments into a heap-allocated deque before calling.
- **Type arguments** arrive as `&[Type]`, fully resolved at call time. Generic natives are *not* monomorphized — there is one Rust function per native, dispatched at runtime on the type arguments.
- **Result** is a `NativeResult` enum with five variants (`Success`, `Abort`, `OutOfGas`, `CallFunction`, `LoadModule`) — see §1.4.
- A `PartialVMError` returned from the closure is an *invariant violation*: it terminates the transaction without charging the abort cost.

### `NativeContext`

`NativeContext<'a, 'b, 'c>` is the bag of capabilities a native gets at call time. The fields make explicit what the VM exposes to natives:

| Field | Purpose |
| --- | --- |
| `interpreter: &dyn InterpreterDebugInterface` | Stack-frame introspection (`get_stack_frames`, `debug_print_stack_trace`). |
| `data_cache: &mut dyn NativeContextMoveVmDataCache` | Global storage access (`native_check_resource_exists`, `native_borrow_resource[_mut]`). |
| `module_storage: &dyn ModuleStorage` | Code loading, runtime environment, layout caches. |
| `extensions: &mut NativeContextExtensions<'b>` | Heterogeneous per-session extension state (see §1.3). |
| `gas_meter: &mut dyn NativeGasMeter` | Charging + heap-memory tracking + dependency metering. |
| `traversal_context: &mut TraversalContext<'c>` | "Has module X been loaded/metered in this txn?" |

Methods on `NativeContext` (and the closely-tied `LoaderContext` it can hand out):

- `exists_at`, `borrow_resource`, `borrow_resource_mut` — global state.
- `type_to_type_tag`, `type_to_type_layout`, `type_to_type_layout_with_delayed_fields`, `type_to_type_layout_check_no_delayed_fields`, `type_to_fully_annotated_layout` — type reflection.
- `extensions`, `extensions_mut`, `extensions_with_loader_context`.
- `stack_frames(n)` — caller-frame inspection.
- `gas_meter()`, `legacy_gas_budget()`.
- `loader_context()` — yields a `LoaderContext` whose methods include `resolve_function` (function values), `charge_gas_for_dependencies` (lazy dispatch), and `get_captured_layouts_for_string_utils` (closure pretty-printing).
- `function_value_extension()` — shim that lets serde (de)serialize function values.

### `NativeResult` (the five outcomes)

```rust
pub enum NativeResult {
    Success     { cost, ret_vals },
    Abort       { cost, abort_code, abort_message },
    OutOfGas    { partial_cost },
    CallFunction{ cost, module_name, func_name, ty_args, args },  // tail-call into Move
    LoadModule  { module_name },                                  // legacy module-load request
}
```

The interpreter's behavior after a return:
- `Success` → push `ret_vals`, charge `cost`, continue.
- `Abort` → unwind transaction with abort code.
- `OutOfGas` → emulate incremental metering; sink remaining gas.
- `CallFunction` → push `args`, transfer control to the resolved Move function (used for dispatchable FAs and account abstraction; see §2.F).
- `LoadModule` → charge dependency gas for the named module, then re-invoke the same native (legacy path, supplanted by direct `charge_gas_for_dependencies`).

### Argument popping

`pop_arg!($args, $t)` is a macro that pops the last argument and `value_as::<$t>` casts it; failure becomes `UNKNOWN_INVARIANT_VIOLATION_ERROR`. The safe wrapper provides `safely_pop_arg!`, `safely_pop_vec_arg!`, `safely_get_struct_field_as!`, `safely_assert_eq!` with the same shape, but routed through `SafeNativeError::InvariantViolation`.

## 1.2 Safe wrapper (`aptos-native-interface`)

The safe wrapper layer adds three things:

1. **`SafeNativeContext` = `NativeContext` + Aptos-specific shared state.** It `Deref`s to `NativeContext` and additionally carries `&TimedFeatures`, `&Features`, `gas_feature_version`, `&NativeGasParameters`, `&MiscGasParameters`, and an optional gas-calibration hook.
2. **`SafeNativeError`** — a saner error enum than naked `PartialVMError`:
   - `Abort { abort_code, abort_message }`
   - `LimitExceeded(LimitExceededError)` — out-of-gas or memory/dependency limit
   - `InvariantViolation(PartialVMError)`
   - `FunctionDispatch { module, func, ty_args, args }` — maps to `NativeResult::CallFunction`
   - `LoadModule { module_name }` — legacy
3. **`SafeNativeBuilder::make_native`** — adapter that converts a safe native (`fn(&mut SafeNativeContext, &[Type], VecDeque<Value>) -> SafeNativeResult<SmallVec<[Value; 1]>>`) into an `Arc<dyn UnboxedNativeFunction>` for registration. It threads shared data, normalizes errors, and (in legacy paths) accumulates `legacy_gas_used` / `legacy_heap_memory_usage` to mimic incremental charging when the gas meter doesn't yet have direct access.

`SafeNativeContext::charge(expr)` is the canonical metering entry point. It evaluates a gas expression against the current schedule, fires the calibration hook (if any), and either charges the live gas meter (new path, gas feature ≥ `RELEASE_V1_32`) or accumulates into the legacy counter. The contract: **charge before doing the work**.

Other notable APIs on `SafeNativeContext`:
- `eval_gas`, `abs_val_size`, `abs_val_size_dereferenced`, `abs_val_gas_params` — value-size based metering.
- `use_heap_memory(amount)` — heap accounting.
- `max_value_nest_depth()`, `gas_feature_version()`.
- `get_feature_flags()`, `timed_feature_enabled(flag)`.
- `charge_gas_for_dependencies(module_id)` — for dispatching natives.
- `load_function(module_id, fn_name)` — used by `function_info` natives.
- `extensions_with_loader_context_and_gas_params` — escape hatch when a native wants the extension *and* a mutable loader context (for table natives).

## 1.3 Native extensions framework

`NativeContextExtensions<'a>` is a `HashMap<TypeId, Box<dyn NativeSessionListener<'a>>>` (heterogeneous, keyed by Rust `TypeId` via `better_any::Tid`). Two important traits:

- **`SessionListener`** — `start(&session_hash, &script_hash, session_counter)`, `finish()`, `abort()`. The host calls these around each VM session so the extension can checkpoint its mutable state.
- **`NativeRuntimeRefCheckModelsCompleted`** — marker, asserted at extension-registration time to confirm every native in this extension that returns a reference has had its runtime ref-check model declared. The runtime ref-checks live in `NativeRuntimeRefChecksModel` on the extensions container.

The extension lifetime `'a` lets an extension hold a *non-`'static`* borrowed reference (typically a resolver into the host's state view). This is why `Tid` is used instead of `Any`.

## 1.4 Gas-meter interface

Two traits the native interface depends on:

- **`DependencyGasMeter::charge_dependency(kind, addr, name, size)`** — module-load metering, usable from the dispatch-driven natives.
- **`NativeGasMeter`** (extends `DependencyGasMeter`) — `charge_native_execution(amount)`, `use_heap_memory_in_native_context(amount)`, `legacy_gas_budget_in_native_context()`.

A native can ask for the gas budget (to emulate "would this work fit?"). It cannot iterate the call stack via the gas meter; that goes through `interpreter.get_stack_frames`.

---

# Part II — Capability requirements, with the natives that need them

These are the buckets, ordered roughly by how broadly used they are. For each bucket, the natives listed are the ones whose behavior *materially* depends on that capability — not every native that touches it incidentally. The very common categories (popping argument values, charging gas, constructing primitive return values) are not enumerated because **essentially every native uses them**; they form the baseline of the interface.

### TL;DR — the 20 buckets, in 6 themes

The 20 buckets cluster into 6 themes. Buckets marked ★ are the "tricky" ones — where any new VM interface design will live or die. The tricky buckets concentrate in **Code** (4 of 5), with the rest split between **State** (E, M), **Crypto** (O, P), and **Data** (K).

```
Native function capabilities
│
├── 1. Data plumbing — value arguments and returns
│   ├── A.  Baseline primitives          — pop args, charge gas, return primitives (~25–30 natives)
│   ├── K.  Reference-typed I/O       ★  — &T / &mut T arguments and returns
│   └── L.  Variant / enum handling      — Option / Result / Ordering, tagged variants
│
├── 2. Type reflection — runtime metadata for the native's type arguments
│   ├── B.  TypeTag introspection        — canonical type tag for a generic argument
│   └── C.  Runtime layouts              — MoveTypeLayout for (de)serialization
│
├── 3. Code & function model — VM's notion of functions, modules, frames
│   ├── F.  CallFunction dispatch     ★  — tail-call into a Move function (FA, AA)
│   ├── G.  Function resolution       ★  — resolve by name, type-match, charge dep gas
│   ├── H.  Closure introspection     ★  — unpack closures, fetch captured layouts
│   ├── I.  Caller-frame introspection ★ — peek the calling Move frame
│   └── J.  Module-traversal checks      — "has this module been visited in this txn?"
│
├── 4. External state — state living outside the call stack
│   ├── D.  Global storage               — exists_at, borrow_resource[_mut]
│   ├── E.  Extension contexts        ★  — per-session, TypeId-keyed, SessionListener
│   └── M.  Delayed fields            ★  — aggregator v2 placeholders, delayed-field serde
│
├── 5. Crypto — bounded calls into Rust crypto libraries
│   ├── N.  Sync deterministic crypto    — hashes, ed25519, BLS, etc.
│   ├── O.  Rayon-using crypto        ★  — pairings/MSM in an isolated thread pool
│   └── P.  Shadow memory             ★  — opaque handles into native-managed arenas (algebra, ristretto)
│
└── 6. Environment & build — config and build variants
    ├── Q.  Feature gating               — Features, TimedFeatures, gas version
    ├── R.  VM-config gating             — max_value_nest_depth, etc.
    ├── S.  Test-only natives            — only registered with feature = "testing"
    └── T.  Host I/O                     — println!-style debug natives
```

## A. Plain primitive natives — the baseline (no list)

Pure functions of primitive inputs that return primitive outputs. They use only argument popping, basic value construction, and `context.charge`. Examples to characterize the shape: `signer::borrow_address`, `account::create_address`, `account::create_signer`, `move_stdlib::hash::sha2_256`/`sha3_256`, `aptos_hash::sip_hash`/`keccak256`/`sha2_512_internal`/`sha3_512_internal`/`ripemd160_internal`/`blake2b_256_internal`, `mem::swap`, `vector::move_range`, `string::internal_check_utf8`/`internal_is_char_boundary`/`internal_sub_string`/`internal_index_of`, `consensus_config::validator_txn_enabled_internal`, `cmp::compare`.

There are roughly 25–30 natives of this shape. The new VM interface must support them efficiently — these are the hot path — but they need none of the special capabilities below.

## B. Type-argument introspection: `TypeTag`

Natives that need a runtime `TypeTag` for a generic type parameter (typically to identify which structure/format was asked for, or to feed it into `to_canonical_string` for naming):

- `type_info::type_of`, `type_info::type_name` — read the type tag of a generic argument and project it into a Move `TypeInfo` struct or string.
- `event::write_to_event_store`, `event::write_module_event_to_store`, `event::emitted_events`, `event::emitted_events_by_handle` — embed the type tag in the contract event.
- `table::new_table_handle` — store key/value type tags in `TableInfo`.
- `crypto_algebra::*` (every native uses `structure_from_ty_arg!`/`format_from_ty_arg!` to convert `TypeTag` → `Structure`/`SerializationFormat`/`HashToStructureSuite` enums via `try_from(type_tag.to_canonical_string())`).
- `string_utils::native_format_list` — uses type tags to verify list-cons types against `0x1::string_utils::Cons`/`NIL`.

**Trickiness**: Algebra natives match the canonical string of the type tag against hard-coded strings (e.g., `"0x1::bls12381_algebra::Fr"`). This is a tight coupling between native code and on-chain module names that the new VM must reproduce or replace.

## C. Type-argument-driven runtime layout (for (de)serialization)

Natives that need `MoveTypeLayout` for a generic type, so they can call `ValueSerDeContext::serialize`/`deserialize`/`serialized_size`. The layout converter walks types lazily, may charge dependency gas, may flag delayed fields, and may fail on depth limits.

- `bcs::to_bytes`, `bcs::serialized_size`, `bcs::constant_serialized_size` (also charges per type-node visited; uses an optional local struct cache).
- `from_bcs::from_bytes` (aliased to `util::from_bytes`).
- `event::write_to_event_store`, `event::write_module_event_to_store` — serialize the message with delayed-field support enabled.
- `event::emitted_events[_by_handle]` (test-only) — deserialize blobs back to values.
- `table::add_box`, `borrow_box`, `borrow_box_mut`, `contains_box`, `remove_box`, `destroy_empty_box`, `drop_unchecked_box` — key/value layouts; layouts cached on the `Table` struct and threaded through the `LoaderContext`.
- `string_utils::native_format`, `string_utils::native_format_list` — use `type_to_fully_annotated_layout` (the *annotated* variant, which carries struct/variant names).
- `debug::print` (legacy) — uses `type_to_fully_annotated_layout` via `native_format_debug`.

**Trickiness**:
1. Layouts can contain delayed fields; some natives must enable `with_delayed_fields_serde()`, others must explicitly check `into_layout_when_has_no_delayed_fields()` (event v1 tests, for instance).
2. Layouts may need `with_legacy_signer()` (bcs::to_bytes, from_bytes) for backward compatibility.
3. Layout construction itself charges gas and can fail (depth limits, dependency limits) — the natives must decide whether to remap that error or surface it directly. `bcs::to_bytes` has explicit lazy-loading vs. legacy branches over this.
4. `ValueSerDeContext::with_func_args_deserialization(&function_value_extension)` is required whenever the value may contain a function value — function values can't be (de)serialized without access to the module storage.

## D. Global storage access (load/borrow resource)

Natives that read or borrow resources from world state:

- `object::exists_at` — `context.exists_at(addr, ty)`, charges per-item + per-byte loaded.
- `storage_slot::borrow_storage_slot_resource`, `borrow_storage_slot_resource_mut` — `context.borrow_resource[_mut](addr, ty)`, returning a `&` or `&mut` to a global resource and the bytes-loaded count.

The `table::*` natives access storage too, but via their *own* `TableResolver` held inside `NativeTableContext`, not via `context.exists_at`. From the VM's perspective, the table natives don't need the global-storage native API.

**Trickiness**: `storage_slot::*` returns a borrowed *reference* into global storage. References-out-of-natives are explicitly flagged as a risk in the existing design doc; the runtime ref-checks framework exists to model these.

## E. Native extension contexts (per-session shadow state)

Each Aptos native extension is a struct that:
- Implements `SessionListener` (start/finish/abort hooks),
- Implements `NativeRuntimeRefCheckModelsCompleted`,
- Is keyed by Rust `TypeId` in `NativeContextExtensions`,
- Often holds a `RefCell` for interior mutability, and
- May hold a non-`'static` borrowed reference (a resolver into host state).

Full inventory of the extensions and their natives:

| Extension | Holds | Backs natives in |
| --- | --- | --- |
| `NativeTransactionContext` | session hash, AUID counter, local counter, script hash, chain ID, user txn context, session counter | `transaction_context::*` (14 natives) and `type_info::chain_id_internal` |
| `NativeEventContext` | `Vec<(ContractEvent, Option<MoveTypeLayout>)>` | `event::write_to_event_store`, `event::write_module_event_to_store`, `event::emitted_events[_by_handle]` |
| `NativeCodeContext` | enabled flag, `Option<PublishRequest>` | `code::request_publish`, `code::request_publish_with_allowed_deps` |
| `NativeObjectContext` | `RefCell<HashMap<(addr, addr), addr>>` (memoization cache) | `object::create_user_derived_object_address_impl` |
| `RandomnessContext` | 8-byte txn-local counter, `unbiasable` flag | `randomness::fetch_and_increment_txn_counter`, `randomness::is_unbiasable` |
| `NativeStateStorageContext<'a>` | `&'a dyn StateStorageView<Key = StateKey>` | `state_storage::get_state_storage_usage_only_at_epoch_beginning` |
| `NativeAggregatorContext<'a>` | session hash, `&dyn AggregatorV1Resolver`, `RefCell<AggregatorData>`, delayed-field flag, `&dyn DelayedFieldResolver`, `RefCell<DelayedFieldData>` | `aggregator::*` (4), `aggregator_v2::*` (14), `aggregator_factory::*` |
| `AlgebraContext` | `bytes_used: usize`, `objs: Vec<Rc<dyn Any>>` (handle-indexed elements) | All 22 `crypto_algebra::*_internal` natives |
| `NativeRistrettoPointContext` | `RefCell<PointStore>` with `Vec<RistrettoPoint>` (handle-indexed) | All 20+ `ristretto255::point_*` natives + bulletproofs (which read point handles) |
| `NativeTableContext<'a>` | `&'a dyn TableResolver`, session hash, `RefCell<TableData>` with new/removed/live tables | All 8 `table::*` natives |

**Trickiness**:
1. **Lifetime parametricity**: `NativeAggregatorContext<'a>`, `NativeStateStorageContext<'a>`, and `NativeTableContext<'a>` carry borrowed resolver references — they cannot be `'static`. The MonoMove extension model must accept non-`'static` extensions (currently achieved via `better_any::Tid`).
2. **Session callbacks**: Each extension can checkpoint state on `start` and roll back on `abort`. Several of today's extensions have `// TODO(sessions): implement` placeholders — the new VM should make this contract explicit.
3. **Interior mutability is pervasive**: nearly every extension hides its mutable state behind a `RefCell` because `NativeContextExtensions::get` returns `&T`, not `&mut T`, and many natives need to read multiple extensions concurrently.

## F. Dynamic dispatch / control transfer (`NativeResult::CallFunction`)

Natives that, instead of returning a value, instruct the VM to call into another Move function with the remaining arguments. The dispatch target is encoded in the last argument (a `FunctionInfo` struct that the native unpacks):

- `dispatchable_fungible_asset::dispatchable_withdraw`
- `dispatchable_fungible_asset::dispatchable_deposit`
- `dispatchable_fungible_asset::dispatchable_derived_balance`
- `dispatchable_fungible_asset::dispatchable_derived_supply`
- `account_abstraction::dispatchable_authenticate`

All five share a single Rust implementation (`native_dispatch`) that:
1. Pops the `FunctionInfo` argument to extract `(ModuleId, Identifier)`.
2. Checks (via `traversal_context().check_is_special_or_visited` or `legacy_check_visited`) that the target module has already been loaded *in this transaction*. This is a precondition to dispatch — gas for the dispatch target must have been charged earlier via `function_info::load_function_impl`.
3. Returns `SafeNativeError::FunctionDispatch { module, func, ty_args, args }`, which `make_native` translates to `NativeResult::CallFunction`.

**Trickiness**: Paranoid-mode stack typing requires the *argument order* of the native to exactly match the target Move function (except for the trailing `FunctionInfo` consumed by the native). This is an unwritten ABI between native and Move signature that the new VM must preserve.

## G. Function resolution & loading

Natives that load a function definition or charge gas for its transitive dependencies:

- `function_info::load_function_impl` — calls `charge_gas_for_dependencies(module_id)` so a subsequent dispatch can run for a flat fee. Falls back to `NativeResult::LoadModule` on the legacy gas path.
- `function_info::check_dispatch_type_compatibility_impl` — loads *two* functions via `context.load_function`, compares their parameter and return types and ability constraints.
- `reflect::native_resolve` — `LoaderContext::resolve_function(module_id, fn_id, expected_ty)` returns either an `AbstractFunction` (wrapped into a `Value::closure` with zero captured args) or one of four resolution errors (`FunctionNotFound`, `FunctionNotAccessible`, `FunctionIncompatibleType`, `FunctionNotInstantiated`). The error is returned to Move as a `Result::Err(u16)`.

**Trickiness**: `resolve_function` performs *type matching with inference* (`subst.match_tys`) on the expected type vs. the function's declared type, infers type arguments, and intern them in the runtime environment's type pool. This is a non-trivial subsystem to expose to natives.

## H. Function-value / closure introspection

Natives that read closures (function values with captured args):

- `reflect::native_resolve` — produces a closure via `Value::closure(fun, iter::empty())`.
- `string_utils::native_format` (the `MoveTypeLayout::Function` arm) — unpacks a closure via `Closure::unpack()`, retrieves captured-argument layouts via `LoaderContext::get_captured_layouts_for_string_utils` (which may need to do *delayed* layout construction), and formats both the function id and the captured args.
- `Closure` serialization is implicit in every layout-driven (de)serializer that passes `with_func_args_deserialization(&function_value_extension)`.

**Trickiness**: A closure's captured-argument layouts may not have been computed yet at the point of inspection. The loader context lazily constructs them, charging gas. Pretty-printing a closure can therefore fail with out-of-gas — which makes the formatting natives a layered failure surface (format error vs. gas error vs. layout-not-yet-resolved).

## I. Caller-frame introspection (interpreter peek)

Natives that examine their caller's stack frame:

- `event::write_module_event_to_store` — `context.stack_frames(1)` and asserts that the caller's module ID matches the *struct's* module ID, preventing cross-module event emission. Aborts if the caller is a script.
- `debug::native_stack_trace`, `debug::print_stack_trace` (and the `_old_` legacy variants) — `context.print_stack_trace(&mut s)` for human-readable traces.

**Trickiness**: `event::write_module_event_to_store` makes a *security-relevant* decision based on caller introspection. The new VM must be able to give natives a reliable view of the caller without leaking implementation details of the new calling convention.

## J. Module-traversal / load-state checks

Natives that need to know whether a module has been "visited" (i.e., loaded with gas charged) in the current transaction:

- `dispatchable_fungible_asset::dispatchable_*` and `account_abstraction::dispatchable_authenticate` — `check_is_special_or_visited` or `legacy_check_visited` before dispatching.
- `function_info::check_dispatch_type_compatibility_impl` — same check before loading the RHS function.

**Trickiness**: The "is special" predicate depends on whether account-abstraction / derivable-account-abstraction features are enabled, which entangles this with feature gating (§Q).

## K. Reference-typed argument & return handling

Natives whose I/O is references rather than owned values. The VM must support these without compromising borrow safety:

- `mem::swap(&mut T, &mut T)` — `Reference::swap_values`.
- `vector::move_range(&mut vector<T>, u64, u64, &mut vector<T>, u64)` — `VectorRef::move_range`.
- `signer::borrow_address(&signer): &address` — `SignerRef::borrow_signer` returns a `Value` of reference type.
- `permissioned_signer::borrow_address`, `is_permissioned_signer_impl`, `permission_address`, `signer_from_permissioned_handle_impl` — all read from `SignerRef`.
- `storage_slot::borrow_storage_slot_resource[_mut]` — return a reference *into global storage*.
- Aggregator natives — read `StructRef`, project field references via `borrow_field`, read addresses/u64/u128.
- Table natives — `StructRef` for the table handle, `Reference` reads on values.
- Many `string`, `string_utils`, `function_info`, `reflect` natives — read `&String` via `StructRef`→`borrow_field(0)`→`VectorRef::as_bytes_ref`.
- `bcs::to_bytes(&T)`, `bcs::serialized_size(&T)` — pop a `Reference`, `read_ref()` to clone the value (currently performs a deep copy; flagged TODO #14175 for in-place).

**Trickiness**:
1. `bcs::to_bytes` reading a reference does a *deep copy* of the entire value; this is the largest known performance defect in the current native interface (see comment at `bcs.rs:88`).
2. Natives that return references must register a `NativeRuntimeRefChecksModel` so the runtime borrow-check engine knows the input-to-output borrow relationship.

## L. Variant / enum struct handling

Natives that construct or read tagged-variant structs (Move enums):

- `cmp::compare` returns an `Ordering` variant (`Struct::pack(vec![Value::u16(variant)])`).
- `bcs::constant_serialized_size` returns `Option<u64>` — `Option_SOME_TAG`/`OPTION_NONE_TAG`, gated by `is_enum_option_enabled`.
- `transaction_context::entry_function_payload_internal`, `multisig_payload_internal` return `Option<_>` — same enum/legacy gating.
- `reflect::native_resolve` returns `Result<Closure, ErrorTag>` via `Struct::pack_variant`.
- `aggregator_v2::*` uses both `Value::u128`/`u64` and `Struct::pack` depending on whether delayed-field optimization is enabled.
- `string_utils::native_format` (the WithVariants / RuntimeVariants struct-layout arms) unpacks via `Struct::unpack_with_tag` and indexes by tag.

**Trickiness**: Many natives need to support *both* the enum-variant representation (`is_enum_option_enabled = true`) and the legacy single-field-vector representation. This forks the value-construction logic at runtime.

## M. Delayed fields / aggregator-V2 state

Natives that read or write the delayed-field extension:

- `aggregator_v2::create_aggregator`, `create_unbounded_aggregator`, `try_add`, `try_sub`, `is_at_least_impl`, `read`, `snapshot`, `create_snapshot`, `read_snapshot`, `create_derived_string`, `read_derived_string`, `derive_string_concat` (12 active + 2 deprecated).
- `aggregator::add`, `read`, `sub`, `destroy` (v1).
- `aggregator_factory::*`.
- Indirectly: any native that serializes a value containing a delayed-field placeholder must use `with_delayed_fields_serde()`. This includes `bcs::to_bytes` (transparently), `event::*`, and `table::*` value (de)serialization paths.

**Trickiness**:
1. Aggregator natives switch behavior based on whether the resolver supports delayed-field optimization. When yes, values are stored as `Value::delayed_value(id)` (a placeholder); when no, they're stored as plain `u64`/`u128`.
2. `DelayedFieldData` is held inside the aggregator context as a `RefCell`. Concurrent read of the resolver and mutable borrow of the data is achieved via the `get_context_data` helper that returns `Option<(&dyn DelayedFieldResolver, RefMut<DelayedFieldData>)>`.
3. `BoundedMath` / `SignedU128` are aggregator-specific arithmetic helpers; not strictly part of the VM, but the VM must expose enough to let them work.

## N. Cryptographic primitives (deterministic, sync, no external state)

Natives that call into Rust crypto libraries with bounded work. They charge gas before invoking. Counted in dozens:

- `move_stdlib::hash::sha2_256`, `sha3_256`.
- `aptos_hash::sip_hash`, `keccak256`, `sha2_512_internal`, `sha3_512_internal`, `ripemd160_internal`, `blake2b_256_internal`.
- `ed25519::public_key_validate`, `signature_verify_strict`.
- `multi_ed25519::public_key_validate_v2`, `public_key_validate_with_gas_fix`, `signature_verify_strict`.
- `secp256k1::ecdsa_recover`.
- `bls12381::aggregate_pubkeys_internal`, `aggregate_signatures_internal`, `signature_subgroup_check_internal`, `validate_pubkey_internal`, `verify_aggregate_signature_internal`, `verify_multisignature_internal`, `verify_normal_signature_internal`, `verify_proof_of_possession_internal`, `verify_signature_share_internal`.
- `ristretto255::*` (point and scalar arithmetic when results are returned by value).

**Trickiness**: Mostly straightforward, but `bls12381` natives have non-trivial gas formulae that depend on the number of inputs and message lengths. They use `safely_pop_vec_arg!` to pop typed vectors of structs.

## O. Cryptographic primitives that internally spawn rayon work

Natives that invoke crypto libraries which themselves use `rayon::par_iter` (transitively), and must be sandboxed to avoid deadlocking on the block executor's rayon pool. They wrap the work in `with_native_rayon(|| ...)`:

- `crypto_algebra::pairing_internal`, `crypto_algebra::multi_pairing_internal` (in `algebra/pairing.rs`).
- `crypto_algebra::scalar_mul_internal`, `multi_scalar_mul_internal` (in `algebra/arithmetics/scalar_mul.rs`).

`with_native_rayon` builds a per-caller-thread isolated `rayon::ThreadPool` (lazily, threads-per-pool set once at startup via `init_native_rayon_pool`). The per-thread pool prevents the deadlock pattern described in `rayon_pool.rs` (par_exec workers stealing each other's block-executor jobs through a writer-preferring `RwLock`).

**Trickiness**:
1. Forgetting to wrap a rayon-using native in `with_native_rayon` is a latent deadlock.
2. The pool is *per caller thread*; concurrent native calls from different block-executor workers do not contend.
3. The new VM design must keep this isolation, either by continuing to expose `with_native_rayon` or by mandating it at the framework level for any native that links in rayon-using crates.

## P. Shadow / handle-based memory natives

Natives that maintain their *own* opaque heap inside a native extension and hand out integer handles to Move. The Move struct just wraps a `u64` handle; the actual data lives in the extension's `Vec` (or similar). The existing native_functions.md explicitly calls this out as "particularly hazardous":

- **`crypto_algebra::*`** — `AlgebraContext.objs: Vec<Rc<dyn Any>>`. The `store_element!` macro performs *its own memory accounting* (`bytes_used`) against a hard `MEMORY_LIMIT_IN_BYTES = 1 << 20` (1 MB) and aborts with `E_TOO_MUCH_MEMORY_USED` if exceeded. `safe_borrow_element!` does the dynamic downcast on `dyn Any`. State is cleared on session `start`.
- **`ristretto255::point_*`** (in `ristretto255_point.rs`) — `NativeRistrettoPointContext.point_data.points: Vec<RistrettoPoint>`. `NUM_POINTS_LIMIT = 10000`. Handle stored in the `RistrettoPoint` Move struct's `handle` field at index 0. State is cleared on session `start`.
- `bulletproofs::*` reads existing point handles from `RistrettoPoint` Move structs (via `get_point_handle`) — i.e., it reads the ristretto255 extension's shadow memory.

**Trickiness**:
1. **Bypasses normal memory accounting**: the VM's heap-memory tracking does not see these allocations. Each extension implements its own limit, which is therefore an independent attack surface.
2. **Type erasure via `dyn Any`**: algebra elements are `Rc<dyn Any>` and downcast at the call site. A wrong type tag would be an invariant violation.
3. **Session-bound lifetime**: the data is cleared on `SessionListener::start`; the new VM must keep this contract or the natives leak across transactions.

## Q. Feature & timed-feature gating

Natives gated on on-chain `Features` (binary flags) and `TimedFeatures` (block-time-gated flags). The list of flags actually observed in natives:

- **`Features::is_*_enabled()`**: `is_lazy_loading_enabled`, `is_enum_option_enabled`, `is_account_abstraction_enabled`, `is_derivable_account_abstraction_enabled`.
- **`FeatureFlag::is_enabled(flag)`**: `BLS12_381_STRUCTURES`, `BN254_STRUCTURES`, `PERMISSIONED_SIGNER`, `SIGNER_NATIVE_FORMAT_FIX`.
- **`TimedFeatureFlag`**: `FixMemoryUsageTracking`, `FixTableNativesMemoryDoubleCounting`, `ChargeBytesForPrints`, `ConstantSerializedSizeLocalCache`.
- **`gas_feature_version()`**: gates entire code paths (e.g., `randomness::fetch_and_increment_txn_counter` only charges if `>= RELEASE_V1_23`; `table-natives` rounds bytes to pages only if `>= 12`; `bcs::to_bytes` has lazy-loading vs. eager-loading branches).

Natives where feature gating is *non-trivial* (multiple code paths, or behavior diverges):

- `bcs::to_bytes`, `bcs::constant_serialized_size` — lazy loading branch, enum option branch.
- `aggregator_v2::*` — delayed field optimization branch.
- `crypto_algebra::*` — every native gated on `BLS12_381_STRUCTURES` / `BN254_STRUCTURES`.
- `permissioned_signer::*` — every native gated on `PERMISSIONED_SIGNER`.
- `randomness::fetch_and_increment_txn_counter` — gas-feature-version-gated charging.
- `string_utils::native_format` (signer arm) — `SIGNER_NATIVE_FORMAT_FIX`.
- `transaction_context::entry_function_payload_internal`, `multisig_payload_internal` — `is_enum_option_enabled`.
- `dispatchable_fungible_asset::*`, `function_info::*` — special-address visited check gated on AA / DAA.
- `table-natives::*` — gas-feature-version branches + `FixTableNativesMemoryDoubleCounting`.

**Trickiness**: The new VM must continue to make features and the gas feature version cheaply queryable from inside a native, because some natives query them in tight loops (e.g., `string_utils::native_format_impl` per-node).

**Implications for the new VM.** The new VM does not need to reimplement the historical per-flag branching listed above — natives should just use the latest implementation of each operation, since the alternative branches exist only to preserve replay-compatibility for code that ran on older feature/gas-version configurations. The `Features` / `TimedFeatures` / `gas_feature_version` *query infrastructure* should still be exposed to natives so future natives can opt back in for backward-compatibility if needed, but designing that surface is a defer-until-needed concern.

## R. VM-config-dependent behavior

Natives that read `RuntimeEnvironment.vm_config()`:

- `cmp::compare` — `vm_config().include_closure_mask_in_cmp`, `DEFAULT_MAX_VM_VALUE_NESTED_DEPTH`.
- `bcs::*`, `from_bytes`, `event::*` — `max_value_nest_depth()` (via `enable_depth_checks`).

## S. Test-only natives

Natives compiled only with `feature = "testing"` or otherwise marked for tests:

- `move_stdlib::unit_test::create_signers_for_testing`.
- `event::emitted_events`, `event::emitted_events_by_handle`.
- `bls12381::generate_keys_internal`, `sign_internal`, `generate_proof_of_possession_internal`.
- `multi_ed25519::generate_keys_internal`, `sign_internal`.
- `ed25519::generate_keys_internal`, `sign_internal`, etc.
- `bulletproofs::prove_range_internal`, `prove_batch_range_internal`.
- `crypto_algebra::rand_insecure_internal`.
- `ristretto255_scalar::random_scalar_internal`.
- `debug::native_print`, `native_stack_trace`, `print`, `print_stack_trace` (debug natives no-op outside `testing` feature).
- `transaction_context::monotonically_increasing_counter_internal_for_test_only`.

**Trickiness**: Test-only natives can use non-deterministic sources (`OsRng`, host time). They are registered into the same dispatch table as production natives; the production build cfg's them out at the function-pointer level, not at registration time. The new VM must keep these isolated from production execution.

## T. Direct host-side side effects (println!)

Two natives perform stdout I/O:

- `debug::native_print`, `debug::native_old_debug_print` — `println!("[debug] ...")` only if `cfg!(feature = "testing")`.
- `debug::print_stack_trace`, `debug::native_old_print_stacktrace` — same.

These are the only natives that observably interact with the outside world (other than rayon thread spawning). They are not deterministic and are only enabled for local tooling.

---

# Part III — Cross-cutting concerns

A few capabilities don't map cleanly to a single bucket but cut across many natives.

## 3.1 Argument ABI: position-of-FunctionInfo and similar conventions

Several natives have *positional* expectations about the argument list that are not derivable from the type signature:

- Dispatch natives (`dispatchable_fungible_asset::*`, `account_abstraction::dispatchable_authenticate`) require the *last* argument to be a `&FunctionInfo` whose fields are `(address, ModuleName, FunctionName)`. The first two are read as `&String` references through `StructRef`/`Reference`/`VectorRef`.
- `function_info::check_dispatch_type_compatibility_impl` reads *two* `FunctionInfo`s from the back of the arg deque.
- The arg-pop order for nested-reference reads (e.g., `event::write_to_event_store`'s third-arg `Vec<u8>` guid) is fragile and entangled with whether the args came from a `&String` or `&Vec<u8>`.

The new VM should consider whether to formalize these ABIs or hide them behind a typed argument-extraction layer.

## 3.2 Charge-before-work and incremental charging

The existing safe-wrapper contract is "always charge before doing work." But there are several documented exceptions where the cost is computed *after* the work (`bcs::to_bytes` charges per-output-byte after serialization, for instance) or where charging post-hoc is the natural path (`table::*` charges `key_cost` and `mem_usage` at the end). The MonoMove `native_functions.md` already discusses this. From the inventory: the post-hoc charges live in `bcs`, `table-natives`, `aggregator_v2::create_derived_string`, and parts of `string_utils`.

## 3.3 Runtime ref-check models

Natives that return references must have a `NativeRuntimeRefChecksModel` registered for them (via `add_native_runtime_ref_checks_model`). Today this is asserted at extension-add time via the `NativeRuntimeRefCheckModelsCompleted` marker. The set of natives that return references is:

- `signer::borrow_address`, `permissioned_signer::borrow_address`.
- `storage_slot::borrow_storage_slot_resource`, `borrow_storage_slot_resource_mut`.
- `mem::swap` returns nothing but takes two mutable references.
- `vector::move_range` similarly.
- Possibly more inside the algebra/ristretto natives via opaque handles, though those return *owned* `Value::struct_(...)` wrappers around `u64` handles.

The new VM must continue to model these reference flows — or change the rules and re-audit each native.

## 3.4 The `LoadModule` legacy result

The `NativeResult::LoadModule` variant and `SafeNativeError::LoadModule` exist solely to support natives that, on the legacy gas path, want the VM to load a module and re-invoke them. `function_info::load_function_impl` is the canonical user. Once `RELEASE_V1_32` is everywhere, this path can disappear: natives will use `context.charge_gas_for_dependencies(module_id)` directly. The new VM should not need to support `LoadModule`.

## 3.5 Native registration

Registration today is a flat table `Vec<(AccountAddress, Identifier, Identifier, NativeFunction)>` consumed by `NativeFunctions::new`, with duplicate-name detection. Each Aptos top-level module (`event`, `code`, `transaction_context`, …) has its own `make_all` builder; `all_natives` in `framework/natives/src/lib.rs` and `move-stdlib/src/natives/mod.rs` aggregate them. The new VM's registration must support:
- Per-module table loading,
- Configurable gas hooks (for calibration),
- The `with_incremental_gas_charging` scope override (currently used by `move-stdlib` and `table-natives` to disable incremental charging for specific groups).

---

# Part IV — Open questions for the MonoMove native interface

Surfaced by this inventory, in rough priority order:

1. **How are arguments passed?** Today's `VecDeque<Value>` is allocation-heavy. The companion design doc (`docs/native_functions.md`) proposes reading args directly from the stack frame via `fp + offset`. The new interface must still let macros like `safely_pop_arg!` keep their ergonomics — or replace them.

2. **How are type arguments exposed?** Most natives need both a `Type` (for layout) and a `TypeTag` (for canonical string matching, e.g., algebra). The new interface should decide whether type args are passed as `Type`, `TypeTag`, monomorphization-tag, or a combination.

3. **Lifetime model for extensions.** `NativeContextExtensions<'a>` uses `better_any::Tid` to allow non-`'static` extensions. With MonoMove's stricter execution/maintenance phasing, can extensions piggy-back on the `ExecutionGuard` lifetime instead?

4. **Reference semantics out of natives.** `signer::borrow_address`, `storage_slot::*`, and the various ref-typed args make natives a first-class participant in borrow checking. The runtime ref-check model is currently bolted on; MonoMove may want to make it part of the native ABI.

5. **Shadow memory natives.** Algebra elements and Ristretto points bypass MonoMove's heap-and-GC story (`docs/heap_and_gc.md`). Either MonoMove provides a sanctioned way for natives to allocate (with global accounting), or these natives keep their own arenas — but then must be denied the ability to scale up without re-audit.

6. **Function values and lazy layouts.** `string_utils::native_format` formatting a closure may need to construct captured-arg layouts on demand and may run out of gas mid-format. The new interface should make this failure mode first-class instead of accidental.

7. **`CallFunction` / dispatch.** Five FA + AA dispatchers depend on tail-call semantics. The MonoMove calling convention (`docs/stack_and_calling_convention.md`) should explicitly carve out a path for this, or these natives need a different mechanism.

8. **Rayon isolation.** The current `with_native_rayon` pattern is fragile (easy to forget). Can MonoMove enforce isolation at a higher level — e.g., a "may use rayon" marker on the native that triggers the pool wrap automatically?

9. **Charge-before-work contract.** Today this is by convention, not enforcement. With monomorphized basic blocks, can we statically place gas charges at the right point?

10. **Test-only / debug natives.** They share the dispatch table with production natives. MonoMove may want to keep them in a separate table that's only registered in test builds.

# Working around the upstream `rustc_public` gaps (`rust-lang/rust#161892`)

Addresses the upstream `rustc_public` blockers tracked in
[rust-lang/rust#161892](https://github.com/rust-lang/rust/issues/161892),
[`rust-mir-design.md`](../designs/rust-mir-design.md)
("`verex-rust`: Rustc Public driver and raw-VIR boundary"),
[`rust-verification.md`](../designs/rust-verification.md), and
[`argon-traits-review.md`](argon-traits-review.md).

The query layer is implemented in `driver/polyfill.rs`. Its predicate results
retain rustc binders, spans, and arbitrary parent `DefId`s; caller-dependent
resolution retains the trait method and normalized arguments for M2. On this
pin, `Instance::try_resolve` returns `None` for parameter dispatch rather than
an `InstanceKind::Param` variant. The bridge borrows rustc's existing scoped TLS
context; it introduces no unsafe code or extra context storage.

The mapper uses all five query families. It exports typed const binders,
promoted values, source call sites, caller-resolved concrete calls, declared
Copy abilities, and lifetime outlives predicates. The VerexSL lifetime binder
spells these as `{a : lifetime outlives b}`. General trait predicates,
associated-item equalities, and checked dictionary execution remain the M2
consumer of the complete predicate/resolution APIs; parameter-dependent calls
continue to carry the explicit trait-evidence refusal until then.

## 1. Decision

We will adopt **Approach B**: a small, strictly isolated
`verex-rust/rust-exporter/src/driver/polyfill.rs` module in the exporter crate
that uses `rustc_public::run_with_tcx!` and
`rustc_public::rustc_internal::{internal, stable}` to implement the five
queries missing from the pinned `rustc_public` (`nightly-2026-07-23`) API,
exposing **only `rustc_public` types** (`GenericPredicates`, `Ty`, `FnDef`,
`GenericArgs`, `Allocation`, `Span`) to [`mapper.rs`](../../verex-rust/rust-exporter/src/driver/mapper.rs).

[`mapper.rs`](../../verex-rust/rust-exporter/src/driver/mapper.rs) continues to
make zero `rustc_middle` calls and operates exclusively on `rustc_public` data
structures. Each function in `polyfill.rs` mirrors an upstream `rustc_public`
query tracked in [rust-lang/rust#161892](https://github.com/rust-lang/rust/issues/161892)
and will be deleted with a one-line call-site switch when the pinned toolchain
includes the upstream method.

The callable evidence slices additionally read the compiler identities of the
`Fn`, `FnMut`, and `FnOnce` traits, `FnOnce::Output`, `Copy`, and `Drop` through
confined bridge helpers. The pinned
public API exposes their predicates but no language-item lookup. These helpers
return public IDs only; replace them when a public identity query is available.
`std::process::abort` is identified by its compiler diagnostic item; the abort
intrinsic uses the public intrinsic query. Evidence signatures are instantiated
through a confined type-substitution helper. These queries return public values
only; they do not authorize unverified receiver conversions.

---

## 2. Inventory of upstream `rustc_public` gaps in `nightly-2026-07-23`

Inspection of `compiler/rustc_public` and `compiler/rustc_public_bridge` in the
pinned sysroot (`rustc 1.99.0-nightly (6f72b5dd5 2026-07-22)`) shows why each
of the five capabilities is currently blocked:

1. **Function, `impl`, and ADT `where` predicates (`explicit_predicates_of`):**
   - `CompilerInterface::explicit_predicates_of(def_id: DefId)` and
     `predicates_of(def_id: DefId)` already exist in
     `rustc_public/src/compiler_interface.rs`, and `Stable<'tcx>` is already
     implemented for `ty::PredicateKind<'tcx>`, `ty::ClauseKind<'tcx>`,
     `ty::TraitPredicate<'tcx>`, `ty::OutlivesPredicate<'tcx, T>`, and
     `ty::ProjectionPredicate<'tcx>`.
   - However, `rustc_public/src/ty.rs` only exposes `.explicit_predicates_of()`
     on `TraitDecl` (not on `FnDef`, `ImplDef`, or `AdtDef`), and
     `CompilerInterface::explicit_predicates_of` wraps `parent` with
     `tables.trait_def(did)`, which assumes the parent `DefId` is a `TraitDef`.
   - **Impact:** Blocks exporting `T: Trait` bounds (M2), `T: Copy` on generic
     parameters, `'a: 'b` region outlives on functions/ADTs
     ([`mapper.rs` lines 386–395, 481–495](../../verex-rust/rust-exporter/src/driver/mapper.rs#L386-L395)),
     and associated-type equality predicates (`<T as Trait>::Assoc == U`).

2. **`Instance::resolve` uses an empty `ParamEnv` (`TypingEnv::fully_monomorphized()`):**
   - In `rustc_public_bridge/src/context/impls.rs`, `resolve_instance` calls
     `Instance::try_resolve(self.tcx, self.fully_monomorphized(), def_id, args_ref)`.
   - Inside a generic body `fn f<K>(slice: &[K])`, a call such as
     `slice.iter()` or `for x in slice` invokes `<Iter<'_, K> as Iterator>::next`.
     The impl `impl<'a, T> Iterator for slice::Iter<'a, T>` is concrete, but has
     the default `T: Sized` obligation. Because `fully_monomorphized()` has an
     empty `ParamEnv`, trait selection does not know `K: Sized` and returns
     `None`.
   - Furthermore, `Stable<'tcx> for ty::Instance<'tcx>` in `rustc_public`
     handles only `Item | Intrinsic | LlvmIntrinsic | Virtual | Shim` (not
     `ty::InstanceKind::Param`), and `Instance::ty()` asserts
     `!instance.has_non_region_param()`.
   - **Impact:** Blocks `.iter()` and concrete container/adapter trait impls in
     generic bodies ([`rust-verification.md`](../designs/rust-verification.md#constants)),
     and conflates genuinely parameter-dependent calls (`<T as Step>::step`)
     with concrete impls that merely need `K: Sized` or a caller `where` bound.

3. **Declared type of `const` generic parameters (`const N: usize`):**
   - `GenericDef` (stored in `GenericParamDef.def_id`) is defined with
     `crate_def!` instead of `crate_def_with_ty!`, so `GenericParamDef` for
     `GenericParamDefKind::Const` does not expose `.ty()`.
   - **Impact:** [`mapper.rs` lines 373–376](../../verex-rust/rust-exporter/src/driver/mapper.rs#L373-L376)
     must reject any function with a `const` generic binder.

4. **Promoted constant evaluation (`UnevaluatedConst { promoted: Some(p), .. }`):**
   - `rustc_public` exposes `Instance::try_const_eval` for top-level `ConstDef`
     items, but has no query to read `tcx.promoted_mir(def_id)[promoted]` or
     evaluate an `UnevaluatedConst` with `promoted: Some(p)`.
   - **Impact:** Blocks `&5`, `x == None`, `a == Unit::A`, and
     `assert_eq!(a, 1)` ([`rust-verification.md`](../designs/rust-verification.md#constants)).

5. **Macro call-site spans (`Span::source_callsite`):**
   - `rustc_public::ty::Span` exposes `get_filename()`, `get_lines()`, and
     `diagnostic()`, but not `rustc_span::Span::source_callsite()`.
   - **Impact:** Diagnostics and panic sites expanded from external macros
     (`assert!`, `assert_eq!`, `panic!`) lack a primary source span at the
     user's macro invocation site and must fall back to dominator heuristics
     ([`rust-verification.md`](../designs/rust-verification.md#constants)).

---

## 3. Alternatives considered

| Criterion | Approach A: Pure `rustc_public` heuristics + VerexSL splicing | Approach B (Chosen): Confined `driver/polyfill.rs` via `rustc_internal` |
|---|---|---|
| **Architectural boundary** | Zero `rustc_internal` calls anywhere in `verex-rust-export`. | `rustc_internal` confined to one ~80-line `driver/polyfill.rs` module; `mapper.rs` stays 100% on `rustc_public` types. |
| **Function / `impl` / ADT `where` bounds** | Synthesize trait bounds bottom-up from unresolved `<T as Trait>::m` calls in MIR bodies + call-graph fixed point; infer `T: Copy` from multi-use `Operand::Copy`; splice remaining bounds from `.lean` specs. | Exact: queries `tcx.explicit_predicates_of(def_id)` and converts clauses to `rustc_public::ty::PredicateKind` via `rustc_internal::stable`. |
| **Unused / marker / lifetime bounds (`'a: 'b`, `T: Clone` unused in body)** | Not recoverable from MIR calls unless repeated in VerexSL `spec`. | Exact: returned directly by `polyfill::explicit_predicates_of`. |
| **`.iter()` in generic bodies (`K: Sized`)** | Substitute dummy `Sized` probe types for `TyKind::Param` before calling `Instance::resolve`, then restore `TyKind::Param`. | Exact: resolves in the caller's real `TypingEnv::non_body_analysis(tcx, caller_def_id)`. |
| **`const N: usize` binder types** | Still blocked (or cast `ConstDef(param.def_id.0).ty()`). | Exact: `polyfill::const_param_ty(param.def_id)`. |
| **Promoted constants (`assert_eq!(a, 1)`, `x == None`)** | Still blocked until upstream lands. | Unblocked: evaluates `mir::UnevaluatedConst` / `promoted_mir` in `polyfill.rs`. |
| **Macro `Span::source_callsite`** | Still blocked; keeps dominator heuristic. | Unblocked: `polyfill::source_callsite(span)` wraps `span.source_callsite()`. |
| **Upstream retirement cost** | Requires ripping out call-graph synthesis, probe-type substitution, and `Copy` inference when `#161892` lands. | Delete `polyfill.rs` and replace `polyfill::foo(x)` with `x.foo()`. |

---

## 4. Detailed design of Approach B (`driver/polyfill.rs`)

### 4.1 Driver callback plumbing (`main.rs`)

[`verex-rust/rust-exporter/src/main.rs`](../../verex-rust/rust-exporter/src/main.rs#L10-L20)
already enables `#![feature(rustc_private)]` and links `rustc_middle` and
`rustc_public`.

1. Switch [`main.rs` line 190](../../verex-rust/rust-exporter/src/main.rs#L190)
   from `rustc_public::run!` to `rustc_public::run_with_tcx!`, passing
   a callback that runs `extract_m0(emit_artifact)` in the initialized context.
2. Inside `driver/polyfill.rs`, borrow the already scoped context with
   `rustc_middle::ty::tls::with`. Helpers take only public arguments
   (`DefId`, `FnDef`, `GenericDef`, `GenericArgs`, `Span`); `mapper.rs` never
   imports or threads `TyCtxt`. No additional TLS or unsafe lifetime erasure is needed.

### 4.2 Polyfill API surface

`verex-rust/rust-exporter/src/driver/polyfill.rs` will expose five narrow
functions returning standard `rustc_public` types:

#### 1. `explicit_predicates_of`

```rust
pub(super) struct ItemPredicates {
    pub parent: Option<DefId>,
    pub predicates: Vec<(Binder<PredicateKind>, Span)>,
}

pub(super) fn explicit_predicates_of(def_id: DefId) -> ItemPredicates
```

- **Implementation:**
  1. Convert `def_id` via `let did = rustc_internal::internal(tcx, def_id);`.
  2. Query `let preds = tcx.explicit_predicates_of(did);`.
  3. Map `preds.parent` via `rustc_internal::stable` (or `CrateItem` / `DefId`
     lookup, avoiding `compiler_interface.rs`'s buggy `tables.trait_def(parent)`
     cast when `parent` is an `ImplDef`, `AdtDef`, or `FnDef`).
  4. For each `(clause, span)` in `preds.predicates`, convert
     `clause.as_predicate().kind()` (retaining its binder) and `span` via
     `rustc_internal::stable(...)`.
     *(Note: Filter out `ClauseKind::HostEffect` and
     `ClauseKind::UnstableFeature` before calling `stable`, since
     `rustc_public`'s `Stable` impl marks those two arms `unimplemented!()`).*
- **Mapper integration:**
  - In [`generic_binder`](../../verex-rust/rust-exporter/src/driver/mapper.rs#L380-L396)
    and signature mapping:
    - `ClauseKind::Trait(pred)` with `Polarity::Positive` on `Copy` attaches
      `Ability::Copy` to the type parameter's `GenericBinder` (removing the
      fallback move-rewrite on `T: Copy`).
    - Other `ClauseKind::Trait(pred)` clauses map to
      `GenericPredicate::Implements { type_, trait_ }`.
    - `ClauseKind::Projection(proj)` maps to
      `GenericPredicate::AssociatedTypeEq` / `AssociatedConstEq`.
    - `ClauseKind::RegionOutlives` and `ClauseKind::TypeOutlives` map to
      `GenericPredicate::LifetimeOutlives` (allowing [`hides_lifetime_relations`](../../verex-rust/rust-exporter/src/driver/mapper.rs#L485-L495)
      to read declared `'a: 'b` bounds instead of conservatively relating
      unknown ADT lifetimes to all).

#### 2. `resolve_in_caller`

```rust
pub(super) enum ResolvedCallee {
    /// Resolved to a concrete function item (possibly still parameterized by
    /// the caller's generic parameters, e.g. `slice::Iter<'_, K>::next`).
    Item(FnDef, GenericArgs),
    /// Dispatches on a trait bound of the caller's generic parameters
    /// (`ty::InstanceKind::Param`), requiring M2 trait evidence.
    TraitParam { trait_method: FnDef, args: GenericArgs },
    /// Closure body call.
    Closure(ClosureDef, GenericArgs),
    /// Unsupported instance kind (`Virtual`, `Intrinsic`, `Shim`, etc.).
    Unsupported(&'static str),
}

pub(super) fn resolve_in_caller(
    caller: DefId,
    callee: FnDef,
    args: &GenericArgs,
) -> Result<ResolvedCallee, String>
```

- **Implementation:**
  1. Convert `caller`, `callee.def_id()`, and `args` via
     `rustc_internal::internal(tcx, ...)`.
  2. Construct the caller's typing environment:
     `let typing_env = rustc_middle::ty::TypingEnv::non_body_analysis(tcx, caller_did);`
  3. Normalize `internal_args` in `typing_env` (`tcx.try_normalize_erasing_regions(typing_env, internal_args)`)
     and call `rustc_middle::ty::Instance::try_resolve(tcx, typing_env, callee_did, norm_args)`.
  4. Match on `instance.def`:
     - `ty::InstanceKind::Item(resolved_did)`:
       Convert `resolved_did` and `instance.args` **separately** to `FnDef` and
       `GenericArgs` via `rustc_internal::stable` (do **not** call
       `rustc_internal::stable(instance)`, because `rustc_public::Instance`
       stores `InstanceDef` whose `instance_ty` asserts
       `!instance.has_non_region_param()`).
     - `ty::InstanceKind::Param(..)`:
       Return `Ok(ResolvedCallee::TraitParam { trait_method: callee, args: rustc_internal::stable(norm_args) })`.
     - `ty::InstanceKind::Virtual(..)` -> `Unsupported("is dispatched dynamically")`.
     - `ty::InstanceKind::Intrinsic(..) | ty::InstanceKind::LlvmIntrinsic(..)` -> `Unsupported("is a compiler intrinsic")`.
     - `ty::InstanceKind::Shim(..)` -> `Unsupported("reaches compiler-generated glue")`.
- **Mapper integration:**
  - Replaces `Instance::resolve` in [`resolve_callee`](../../verex-rust/rust-exporter/src/driver/mapper.rs#L750-L777)
    by passing the enclosing body's `DefId`.
  - Concrete impl calls inside generic bodies (such as `slice.iter()` on
    `&[K]`, which only needs `K: Sized` from `caller`'s `TypingEnv`) now
    resolve to `ResolvedCallee::Item` and export cleanly.
  - Genuinely parameter-dependent trait calls (`<T as Step>::step`) return
    `ResolvedCallee::TraitParam`, which M1 rejects as `PARAMETER_DEPENDENT_CALL`
    and M2 lowers directly to an `EvidenceId` dictionary call.

#### 3. `const_param_ty`

```rust
pub(super) fn const_param_ty(param_def: GenericDef) -> RustTy
```

- **Implementation:**
  - `ConstDef(param_def.0).ty()` (or via `tcx.type_of(rustc_internal::internal(tcx, param_def.0)).instantiate_identity()`
    converted with `rustc_internal::stable`).
- **Mapper integration:**
  - In [`binder_kind` / `generic_binder`](../../verex-rust/rust-exporter/src/driver/mapper.rs#L361-L396),
    `GenericParamDefKind::Const` maps to `BinderKind::Const` with
    `type_: Some(self.types.intern(polyfill::const_param_ty(parameter.def_id), loc)?)`.

#### 4. `promoted_body` / `eval_promoted_const`

```rust
pub(super) fn promoted_body(owner: DefId, promoted: Promoted) -> Result<Body, String>
```

- **Implementation:**
  - Converts `owner` via `rustc_internal::internal(tcx, owner)`, fetches
    `&tcx.promoted_mir(owner_did)[rustc_middle::mir::Promoted::from_u32(promoted)]`,
    and converts the promoted `mir::Body<'tcx>` to `rustc_public::mir::Body` via
    `rustc_internal::stable(body.clone())` (or evaluates it via
    `tcx.const_eval_resolve` when monomorphic).
- **Mapper integration:**
  - Unblocks promoted constants (`&5`, `x == None`, `a == Unit::A`,
    `assert_eq!(a, 1)`) in `EvaluatedConstants`.

#### 5. `source_callsite`

```rust
pub(super) fn source_callsite(span: Span) -> Span
```

- **Implementation:**
  - `rustc_internal::stable(rustc_internal::internal(tcx, span).source_callsite())`
  - Plus a helper `walk_to_local_callsite(span: Span) -> Span` that repeatedly
    steps `source_callsite()` while the span lies in an external crate until it
    reaches the local crate's macro invocation site.
- **Mapper integration:**
  - Used in `SourceTables` when mapping spans of statements/terminators expanded
    from external macros (`assert!`, `assert_eq!`, `panic!`), attaching the
    primary location directly to the macro call in the `.rs` file.

---

## 5. Retirement

Each query family is in place and its consumers are listed above. What is open
is retirement: when the pinned nightly carries the upstream fix for
[rust-lang/rust#161892](https://github.com/rust-lang/rust/issues/161892) and
its sibling queries, each function of `polyfill.rs` is removed and replaced by
the direct `rustc_public` call, one call site at a time. General trait
predicates, associated-item equalities and checked dictionary execution are the
remaining consumers of the complete predicate and resolution APIs
([`rust-verification.md`](rust-verification.md), item 2 of the Order).

## 6. Guardrails and retirement policy

- **Strict module boundary:** Only `verex-rust/rust-exporter/src/driver/polyfill.rs`
  (and `main.rs` for `run_with_tcx!`) may import `rustc_middle` or
  `rustc_public::rustc_internal`. [`mapper.rs`](../../verex-rust/rust-exporter/src/driver/mapper.rs)
  and [`probe.rs`](../../verex-rust/rust-exporter/src/driver/probe.rs) must not
  import `rustc_middle` or `rustc_internal`.
- **Import evidence:** While any `polyfill.rs` query is active, `RawUnit.evidence`
  records `verex-rust-export via rustc Public (+ rustc_internal polyfill for rust-lang/rust#161892)`.
- **Retirement:** When the pinned nightly toolchain is bumped to a revision
  containing upstream fixes for [rust-lang/rust#161892](https://github.com/rust-lang/rust/issues/161892)
  and its sibling queries, each corresponding function in `polyfill.rs` is
  removed and replaced by the direct `rustc_public` method call.

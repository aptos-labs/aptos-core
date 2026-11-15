# Review: Rust generics with static trait bounds and `dyn Trait`

Reviewed 2026-11-01. Covers the representation and soundness of trait-bounded
generics and `dyn Trait` across
[`rust-mir-design.md`](../designs/rust-mir-design.md),
[`vir-design.md`](../designs/vir-design.md),
[`native-verification.md`](../designs/native-verification.md),
[`interior-mutability.md`](../designs/interior-mutability.md),
[`native-function-values.md`](native-function-values.md),
[`rust-verification.md`](../designs/rust-verification.md), and
[`assumptions.md`](../status/assumptions.md).

## 1. Current representation

### 1.1 Generics with static trait bounds

Rather than importing monomorphized copies per concrete type, the design
([`rust-mir-design.md`](../designs/rust-mir-design.md),
[`vir-design.md`](../designs/vir-design.md)) keeps **one parametric VIR body**
per generic function and represents trait bounds via explicit predicates and
dictionary-passing **evidence**:

- **Core VIR schema (`verex-ir/VerexIR/Syntax.lean`):**
  - **Declarations:** Traits, associated items, and `impl` blocks are
    language-neutral, first-class core declarations: `TraitDecl`,
    `AssociatedItemDecl`, and `ImplDecl`.
  - **Binders and arguments:** `BinderKind` and `GenericArgument` have four
    kinds: `.typeArg`, `.const`, `.lifetime`, and
    `.evidence (value : EvidenceId)` (spelled `{e : evidence}` in VerexSL via
    `verexEvidenceBinder`).
  - **Predicates:** `GenericBinder` and `Signature` carry `Ability` (`Copy`
    maps to `.copy`) and `GenericPredicate`:
    - `.implements (type : TypeId) (trait : TraitRef)`
    - `.associatedTypeEq (trait : TraitRef) (item : AssociatedItemId) (value : TypeId)`
    - `.associatedConstEq (trait : TraitRef) (item : AssociatedItemId) (value : ConstValue)`
    - `.lifetimeOutlives`, `.constEq`, `.ability`
- **Dictionary-passing and modular verification
  ([`rust-mir-design.md`](../designs/rust-mir-design.md)):**
  - Unresolved static trait bounds on a generic function become explicit
    **evidence parameters** (for example,
    `step_twice<T>(stepEvidence : StepEvidence<T>, value : &mut T)`).
  - Behavioral laws attach as separate contracts over the evidence
    (`StepContract<T>(evidence)`). A generic function is proved once
    parametrically over `T` and `evidence`, and concrete `impl` blocks prove
    `StepContract` once; call sites instantiate the generic theorem with
    concrete impl witnesses without reverifying the generic body.
- **Special case — `Fn*` trait bounds
  ([`rust-verification.md`](../designs/rust-verification.md#closures)):**
  - For a closure passed to a generic (`F: Fn*(...)`), the proposed design
    represents `Fn*` evidence as first-class **function values** (Move Prover
    style, using behavioral predicates such as `ensures_of<F>(f, x, result)`
    and `result_of<f>(...)`), deriving the evidence binder from
    `<F as Fn<...>>::call` in the body and instantiating it at call sites with
    the closure body.

#### Current implementation and workarounds ([`rust-verification.md`](../designs/rust-verification.md#the-standard-library-specification-library))

Because Rustc Public does not yet expose `where` predicates on functions and
impls ([rust-lang/rust#161892](https://github.com/rust-lang/rust/issues/161892))
and the checked `EvidenceId` dictionary table is deferred to Milestone M2:

- **Concrete trait calls** (`Instance::resolve` succeeds): Resolved at export
  time directly to the concrete `impl` method in the `impl`'s scope.
- **Parameter-dependent trait calls** (`T: Step` where `Instance::resolve`
  depends on `T`): Rejected/refused by the exporter with
  `PARAMETER_DEPENDENT_CALL` until M2 trait evidence lands.
- **Synthetic `impl Trait` parameters:** Rejected at export.
- **Associated types in dependency signatures:** Because MIR types are
  normalized while Rustc Public lacks a signature-normalization query, each
  distinct associated type alias (`<I as SliceIndex>::Output`) is promoted to
  an explicit **generic type parameter** after the function's own generics, and
  instantiated at call sites by unifying with the normalized MIR
  operand/destination types.
- **Contracts at a shape (`ShapeContract`):** Standard-library methods that
  depend on trait implementations (such as `Index::index` or iterator adapters
  `Rev`/`Enumerate`) are specified per concrete instantiation shape
  (`spec index at {T} {usize} {A} {T} where ...`).
- **`T: Copy`:** Since the bound on `T` is not exposed by Rustc Public, copying
  a value of type `T` in a generic body is conservatively mapped to a move.

---

### 1.2 `dyn Trait` (trait objects)

There is **no general `Ty.dyn` node in VIR yet**; `dyn Trait` is split between
a concrete representation for `dyn Fn*`, type-keyed modeling for `Any`-like
maps, and a staged post-M2 feature for general vtable trait objects:

1. **General `dyn Trait` (post-M2 milestone):**
   - **Planned representation
     ([`rust-mir-design.md`](../designs/rust-mir-design.md)):** Designed as a
     separate feature requiring **existential values paired with runtime
     vtable/dictionary dispatch** (as opposed to static evidence parameters).
   - **Current exporter behavior:** `RigidTy::Dynamic` is not in the core `Ty`
     union and is rejected when interning types (or causes per-function
     refusal, which [`rust-verification.md`](../designs/rust-verification.md) notes is the
     most common blocker across external registry crates). For ADT reachability
     analysis, `verex-rust/rust-exporter/src/driver/mapper.rs` conservatively
     marks `RigidTy::Dynamic` as non-`Copy` and destructor-bearing
     (`reach.own = true; reach.destructor = true`).
2. **`dyn Fn` / `dyn FnMut` / `dyn FnOnce`
   ([`native-function-values.md`](native-function-values.md),
   [`rust-verification.md`](../designs/rust-verification.md#closures)):**
   - Unlike known closure types (which compile to nominal structs of their
     upvars plus direct calls), `dyn Fn*` trait objects share the **native
     function-value carrier `FV_τ`** (`Ty.function`).
   - Per link set, `FV_τ` is a generated inductive datatype with one
     constructor per known closure struct or function-pointer site in the link
     set (holding native carriers for captured upvars) plus a
     `foreign (key : ForeignClosure τ)` variant for unknown/external targets,
     dispatched via `invoke`.
3. **Type-keyed `Box<dyn Trait>` maps
   ([`interior-mutability.md` §6](../designs/interior-mutability.md#6-library-models)):**
   - Heterogeneous maps of the shape `HashMap<TypeId, RefCell<Box<dyn Trait>>>`
     (such as MonoVM's `NativeExtensions`) bypass vtable dispatch by
     decomposing the map by `TypeId`—modeling it as one `Option AllocId` slot
     per registered concrete extension type `T` (where `AllocId` names a
     `RefCell<T>` in memory).

---

## 2. Soundness review

### Verdict

**Architecturally sound in the current slice, with 4 specific soundness hazards
to guard against as M2 (`Evidence`) and `dyn` land.**

The core architecture—keeping one generic body in VIR, refusing what is not yet
modeled rather than approximating it, and separating structural equality from
trait methods—is fundamentally sound. In particular, the current design already
catches two classic Rust verification traps that break naive parametricity:

1. **Prophecies under type parameters
   ([`rust-verification.md`](../designs/rust-verification.md#references-in-aggregates)):**
   Instantiating `T := &mut U` breaks a generic proof that treats
   `drop(x : T)` as a no-op, because dropping `&mut U` resolves its prophecy
   (`current = prophecy`). Treating `&mut` as destructor-bearing, keeping
   `refFree` on `InvokeMeaning.Coherent`, and verifying reference-instantiated
   generics via substituted compilation (`Term.callInstance`) is **sound**.
2. **`TypeId`-keyed `Box<dyn Trait>` maps
   ([`interior-mutability.md` §6](../designs/interior-mutability.md#6-library-models)):**
   Modeling `HashMap<TypeId, RefCell<Box<dyn NativeExtension>>>` as one
   `Option AllocId` slot per concrete `T` with `a_T₁ ≠ a_T₂` is **sound**,
   because `TypeId::of::<T>()` is injective on `'static` types and the wrapper
   API ties the map key to the downcast type `T`.

---

### 4 soundness hazards in the planned design

#### Hazard 1. `dyn FnMut` and `FnMut` evidence cannot use stateless `FV_τ` dispatch

- **Where:** [`native-function-values.md` ("Rust")](native-function-values.md)
  and [`rust-verification.md`](../designs/rust-verification.md#closures).
- **The issue:**
  - In [`native-function-values.md`](native-function-values.md), `dyn Fn*`
    values are mapped to `FV_τ` over the link set's closure structs plus
    `foreign`. For `&dyn Fn`, the environment is shared (`&self`), so `FV_τ` is
    sound.
  - However, calling `FnMut::call_mut(&mut self, args)` on `&mut dyn FnMut` or
    a generic `F: FnMut` **mutates the closure's captured fields in `self`
    across calls**. Move closures do not have mutable self-captures, so
    `FV_τ`'s `invoke v args` does not return an updated `v' : FV_τ`.
  - Similarly, Step 3d proposes instantiating `apply<F>`'s `Fn*` evidence at a
    call site *"with the closure body as a nullary function value."* A nullary
    function value drops the `self` parameter (the closure struct holding the
    upvars). That is only sound for captureless (zero-sized) closures.
- **Fix required:**
  - For `F: Fn*` evidence, the closure struct `f : F` (or `&f` / `&mut f`) must
    remain the explicit first value argument (`self`) passed to the evidence
    call, not erased into a nullary function value; for `FnMut`, `ensures_of`
    must relate `old(f)` and post-state `f`.
  - Furthermore, if a closure captures `&mut U`, its struct type `F` is not
    `refFree`, so passing `F` to a generic function must follow Step 3b's rule
    for generics instantiated at reference-holding types (`NTy.lendable`
    currently treats `.function` as `refFree`).

#### Hazard 2. `&mut dyn Trait` unsizing and prophecy write-back (dynamic type invariance)

- **Where:** [`rust-mir-design.md`](../designs/rust-mir-design.md) and
  [`rust-verification.md`](../designs/rust-verification.md#shared-views-of-a-holder).
- **The issue:**
  - When a concrete mutable reference `&mut Concrete` is unsized to
    `&mut dyn Trait`, the lender `x : Concrete` holds a loan hole of type
    `Concrete`, while the borrower holds `&mut dyn Trait`.
  - This has the exact same write-back hazard that Step 3e identified for
    `&mut [T; N]` unsized to `&mut [T]`: if `dyn Trait` is modeled as an
    existential `∃ T, (T × TraitEvidence<T>)`, then a naive prophetic borrow
    carrier `(current : dyn Trait, prophecy : dyn Trait)` would allow
    `prophecy` to have a *different* underlying concrete type `T' ≠ Concrete`.
    When the loan dies and writes back into `x : Concrete`, filling the hole
    would either fail or make the path vacuous.
- **Fix required:**
  - A mutable borrow of `dyn Trait` (and `dyn FnMut`) must pack the existential
    **outside** the `(current, prophecy)` pair—i.e.,
    `∃ T (ev : TraitEvidence<T>), (current : T, prophecy : T)`—because
    `&mut dyn Trait` can mutate the `T` payload in place, but can never change
    its dynamic type tag or vtable.

#### Hazard 3. Implicit `Drop` on unbounded type parameters (`fn f<T>(x: T)`)

- **Where:** [M4](rust-verification.md#m4--drop-and-panic),
  [`rust-verification.md` ("Drops")](../designs/rust-verification.md#drops),
  and [`assumptions.md`](../status/assumptions.md).
- **The issue:**
  - Unlike ordinary static trait bounds (`T: Step`), **every** Rust type
    parameter `T` has implicit `Drop` glue in MIR (`Drop(place)`), with *no*
    `T: Drop` bound in the signature.
  - Today, `dropsDestructor` soundly rejects in-unit calls that instantiate a
    dropping generic with a destructor-bearing type, and records an unstated
    assumption in [`assumptions.md`](../status/assumptions.md) for external
    callers.
  - When M4 introduces effectful destructors, `Drop(x)` on `x : T` inside a
    generic body is an implicit trait call (`Drop::drop(&mut x)`).
- **Fix required:**
  - A generic theorem `∀ T, Satisfies (f<T>) ...` cannot remain universally
    quantified over *all* `T` without either:
    1. Adding a `NoDestructor T` (trivial-drop) hypothesis to the generic
       theorem's binder (and verifying destructor-bearing instantiations via
       `Term.callInstance`), or
    2. Treating every dropped type parameter `T` as carrying implicit `Drop`
       evidence.

#### Hazard 4. Associated-type promotion in dependency signatures without equality constraints

- **Where:**
  [`rust-verification.md` ("Associated types are type parameters")](../designs/rust-verification.md#associated-types-are-type-parameters).
- **The issue:**
  - Promoting each distinct associated type alias in a dependency signature to
    an unconstrained trailing type parameter
    (`native fun index {T} {I} {A} {Output}(_1 : &Vec<T, A>, _2 : I) -> &Output`)
    drops the functional dependency `<I as SliceIndex<[T]>>::Output = Output`
    from the signature itself.
  - Why it is currently safe: Call sites unify `{Output}` with rustc's
    already-normalized MIR types, and library specs use `ShapeContract`
    (`spec index at {T} {usize} {A} {T}`) to pin `{Output}` for concrete index
    types.
  - Where it could become unsound:
    1. If a user or library author writes an *unshaped* contract on a
       dependency function with a promoted `{Output}` parameter, `{Output}` is
       universally quantified independently of `{T}` and `{I}`.
    2. In `verifyShapeSiblings`
       ([`native-verification.md`](../designs/native-verification.md#contracts-at-a-shape)),
       `{usize}` and `{u64}` map to the same 64-bit native carrier
       `NTy.int 64`, which is why the checker enforces that sibling shapes at
       `{usize}` and `{u64}` have identical contracts. If two shapes ever had
       the *same* native input carriers `(T, I, A)` but mapped to *different*
       `{Output}` types or postconditions, the universal quantification over
       `typeInstantiation` would be inconsistent.
- **Fix required:**
  - Validation should forbid authoring an *unshaped* `spec` on any dependency
    function that has promoted associated-type parameters (require
    `spec ... at ...` until M2 attaches `GenericPredicate.associatedTypeEq`).

---

## 3. Summary table

| Mechanism | Soundness status | Key condition / caveat |
|---|---|---|
| Unmonomorphized generic bodies + `EvidenceId` | **Sound** | Evidence contracts (`StepContract`) must never silently equate `PartialEq`/`Ord`/`Clone` on `T` with VIR primitive `equal`/`compare`/`copyValue` unless the `impl` proves a lawfulness obligation. |
| Generics instantiated with `&mut U` | **Sound** | Properly blocked from generic transfer by `refFree` + `dropsDestructor`, with monomorphic specialization (`Term.callInstance`) planned. |
| Promoted associated-type parameters + `ShapeContract` | **Conditionally sound** | Safe when pinned by `ShapeContract` and checked by `verifyShapeSiblings`; unshaped specs on functions with promoted associated-type binders should be rejected by validation. |
| `Fn*` trait bounds as function-value evidence | **Needs refinement** | Sound only if the closure struct `self` (`F` / `&F` / `&mut F`) is passed as an explicit argument (not erased to a nullary function value) and `refFree`/lending rules apply to `F`. |
| `dyn Fn` / `dyn FnMut` / `dyn FnOnce` as `FV_τ` | **Sound for `Fn` / `FnOnce`; unsound for `FnMut` as-is** | `&mut dyn FnMut` mutates its captured environment across calls; `invoke` must thread the mutated closure carrier state while keeping its constructor tag invariant. |
| General `&mut dyn Trait` (existential + vtable) | **Needs invariant tag** | The existential type `T` and vtable must sit *outside* the `(current, prophecy)` pair so write-back to the concrete lender preserves the dynamic type. |
| `TypeId`-keyed `Box<dyn Trait>` maps | **Sound** | Relies on `TypeId::of::<T>()` injectivity for `'static` types and the typed wrapper API. |

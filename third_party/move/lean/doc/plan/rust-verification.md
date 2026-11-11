# Closing the Rust verification gaps

What is open in Rust source verification, and the order it is attacked in.
Refines section 7 of [`roadmap.md`](roadmap.md). The model of what the path
supports, how each feature is modelled and where it refuses is
[`rust-verification.md`](../designs/rust-verification.md); the frontend is
[`rust-mir-design.md`](../designs/rust-mir-design.md); the unsafe profile is
[`unsafe-pointers.md`](../designs/unsafe-pointers.md) and
[`interior-mutability.md`](../designs/interior-mutability.md).

Each item adds fixtures to the Rust `SourceVerify` gate. A baseline
expectation is never updated to conceal a failure, this work's or an inherited
one: what the Move Prover's suite leaves unproved, and why, is in
[`prover-test-problems.md`](../status/prover-test-problems.md). The order is
checked by measurement rather than by its items being closed: the first stop
over the 180 dependency-free registry crates and the Rust benchmark problems
are re-run after items 1, 2 and 3 below.

## Order

1. Library combinators, with `iter_mut`. They reuse the iterator models and the
   `Fn*` evidence, both in place, and are what the benchmark's problems reach
   first. `Option::map` verifies through the `FnOnce` evidence its bound
   supplies (`library_option_map`), a concrete closure at the call site
   supplying that evidence. What is left is the `Map<I, F>` model with
   `spec next at {B} {slice::iter::Iter<lifetime _, Item>} {F} {&Item}` and
   `spec map at {Self} {B} {F} {Item}`, so that a `for` over `iter().map(..)`
   with an invariant verifies (`library_iter_map`, `option_map_shared`). Such a
   declaration is `shapeOnly`, so those contracts state shapes, as the
   `TryFromIntError` conversion does, naming `F` as a variable of the shape, as
   `Result`'s `branch` names `T` and `E`. The sharing mark the call needs is in
   place ([`rust-verification.md`](../designs/rust-verification.md), "Sharing of
   a type argument at a dictionary slot"); open beside it is whether a nominal's
   `arguments` row keeps the mark, which would also fix a function-typed field
   at a shared-reference argument.

   The `for` form of `iter_mut` waits on a decision rather than on work: either
   `VIR-SHAPE-OVERLAP` relaxed so a native at a reference instance carries a
   contract at its shape beside its generic one, which also means routing such a
   call through the contract path rather than the inline path, or skolems
   generalized to references from the backburner of
   [`roadmap.md`](roadmap.md). `iter_mut` verifies in the `while let` form
   meanwhile. Narrowing `VEREX-SHAPE-REQUIRED` is not in this item: a generic
   contract for `<Map<I, F> as Iterator>::next` would have to state the inner
   step, which needs an `Iterator` dictionary for `I` — multi-method trait
   evidence, item 2. Until then the shape for `I` is what pins `Item`, the
   `FnMut` evidence pinning only `B`.

2. Trait evidence (M2), the slice after the single-method dictionary:
   associated types and their equalities, which `Iterator` itself needs, then
   conditional method preconditions. What is left of the associated-type gap is
   the unit's own generic items. A dependency declaration binds a projection its
   signature or one of its dictionaries names (`collect_associated_types`,
   `DependencyFunction`'s `associated_types`), and every dictionary is
   normalized in the environment of the item declaring it, which answers under
   an equality bound: `fn apply<I: Iterator<Item = u32>, F: FnMut(I::Item) ->
   u32>` exports. An unequated projection in a bound of the unit's own generic
   item has no answer and no binder, and stops the export:
   `fn apply<I: Iterator, F: FnMut(I::Item) -> u32>` refuses with `unsupported
   Rust scalar type: Alias(Projection, … Iterator::Item …)`. So the work is to
   give the unit's own generic items the binders dependencies already have, the
   design being settled: associated types as generic parameters, since Rustc
   Public cannot normalize them. The preconditions come before supertraits
   because `MethodContract` requires an everywhere-defined non-panicking method,
   which excludes `Index::index` and panicking arithmetic from being evidence at
   all, and with them the benchmark's `simple_map` problems written as a Rust
   programmer would.

   Also open here: supertraits, higher-ranked bounds, `dyn`, and general
   parameter-dependent dispatch, which stays an export refusal until a supported
   method dictionary covers it.

3. Recursive types. The carrier is in place: `NTy.nominal` with its exhaustive
   arms, `nominalBody?`, `productiveWitness?` and `admits`, the nominal fields
   of `Carriers` bundled as `TypedNominals`, and the runtime instance. What is
   open:

   | Step | Content |
   |---|---|
   | 1 | `ntyOf` produces `.nominal` with its flag; `Proj.unfold`, `Compile`, `Term`, `Solve.Normal`, `List` fixtures; `.nominal` stops being `notCarried`; `admits` exact on every type `ntyOf` gives, by the witness premise of `TypedAs`'s `.nominal` constructor |
   | 2 | Clause translator, `size` measure, `len` and `seq` |
   | 3 | `&mut` into recursive values, cursors over `Option<Box<Node>>`: `push_back` with `ensures seq(l) == seq(old(l)) ++ [x]`, a tree insert |
   | 4 | Generated carriers per recursive group: the same fixtures at heartbeats below the runtime instance; a per-unit carrier class |

   `Proj.unfold (body : NTy) (rest : Proj body σ)` needs no unit: it reads and
   writes through the codecs, `body.codec.decode? (nominal.encode v)` and back,
   both `Option` already, so agreement follows from tightness, the compiler
   checks that `body` is the declaration's rows at the arguments, and the
   family's `fold`/`unfold` laws serve only the normal form. The normal form
   adds `unfold (fold v) = v` and injectivity. A recursive spec function
   defaults to the family's `size` as its measure. `fold`, `unfold` and `size`
   are still to add to `TypedNominals`.

   Soundness conditions these steps owe: that `TypeArgs` being inhabitable keeps
   the flag rule under substitution, which needs inhabitable to imply a witness;
   that `typeFuel` suffices for the witness search, or the flag is tied to the
   same search, since a productive type whose search runs out gets the `.unit`
   point; `carriesInvariant` and `owns` read only the source and arguments, so
   they can miss an invariant or a destructor in the rows; `heldFunctionTypes`
   returns nothing for a nominal; `refFree` reads only the arguments, relying on
   rejecting a recursive type holding `&mut`; `NTy.equal` with handles compares
   by `eraseGhosts`, so a map in the rows compares by entries; a closure
   capturing a nominal is rejected (`capturedUnsupported?`). `.enum`
   inhabitability reads only the first variant (`NRows.firstInhabitable`), which
   wrongly refuses `Result<Void, E>`.

   Until these steps land, a function whose local or result has a recursive type
   has no denotation, and other functions of the unit are unaffected. The work
   is estimated, from counts of the `.param` and `.function` arms and `Proj`
   sites and not from a build, at about 3000 new lines, mostly proofs in the
   lowest modules. Each edit of `Types.lean` rebuilds nearly all of `verex-ir`,
   so the item wants the package otherwise idle.

4. M4, drops and panics. `panic=abort` is pinned, so unwinding is not in it and
   it reduces to effectful user `Drop`, which discharges the trivial-destructor
   assumption. It needs a design pass before it is a work item. The destructor
   variant of the drop message, which a `for` over `iter().map(..)` meets, is
   part of it.

5. Breadth: strings' UTF-8 error offsets and a `chars` iterator model,
   arbitrary mutable subranges with their write-back splicing (which must also
   preserve the borrowed extent), floats, function pointers as evidence and then
   `dyn Fn`, M3's remaining region precision including iterator result regions,
   M5 with interior mutability, M7's alignment proofs, and M1.5's prebuilt
   exporter and sysroot. `HashMap` and `BTreeMap` over the intrinsic map value
   are not designed; a design precedes them. `Box<dyn Trait>`, termination of
   recursive functions, and `Rc` with `RefCell`
   ([`interior-mutability.md`](../designs/interior-mutability.md)) come with
   their own designs. Closures beyond the implemented evidence follow
   [`native-function-values.md`](native-function-values.md) ("Rust"). Proof
   files per Rust module are open; a module's specifications already come from
   its `.spec.lean`.

6. Retire the confined [`rustc_public` polyfill](rustc-public-polyfill.md) once
   the upstream fix is in the pinned toolchain.

## Open verification gaps

Rust `SourceVerify` fixtures verify except for these:

- Evidence composition: a generic closure carrying its caller's method evidence,
  passed to a verified generic callee, does not discharge the callee's
  method-contract precondition (`evidence_composition`, function `outer`;
  `multiple` in its `_errors` twin).
- A specialized `FnOnce` contract of an opaque callee does not verify
  (`fnonce_adapters`).
- One slice-iterator loop invariant relating the last element read to the
  position is not preserved automatically (`library_iter_errors`).
- A nested range loop does not prove a nonlinear overflow bound automatically
  (`library_for_errors`, `nested`).
- The cheap witness pass exhausts its budget on behavior compositions before the
  later pass succeeds. This is cost, not correctness.
- A leaf statement carrying a function value spells the unit's carrier family
  at every encoding and behavior term, which makes it about three times the
  size it needs and every normal-form step over it dearer; the wider type
  language adds about a third to the unfoldings of type reduction. Targets with
  contract calls and function values pay 20–40% for this, and those recorded
  over budget in `VerexSL/Tests` are its measure. Bind the carrier family once
  per leaf statement; an induced frame built at a call with no type arguments
  is the larger part and the larger change.
- Relating a *read* through a yielded mutable reference to the element before
  the write is not derived automatically (`library_slice_get_mut_errors`).
- An element-wise postcondition over the lender needs a loop invariant reading
  the held mutable reference's current contents; a bare field read of a value
  holding references denotes the prophecy
  ([`denotation.md`](../designs/denotation.md)), so the spelling for the current
  one inside an invariant is the open question there.
- The theorem of an opaque adapter's instance (`verifyOpaqueInstance`) binds
  no `DataInvariantsSatisfiable` hypothesis and its simp set names no
  `Carriers.own` equation, where the public theorems take both
  (`satisfiabilityOf`). Whether a unit with data invariants needs them there is
  not checked by any fixture.
- The differential check is parked. Its tree evaluator
  (`VerexRust/Tests/DenoteEval.lean`) does not build on the carriers base:
  values of declared types carry certificates and their encodings are
  noncomputable, and the terms added since have no tree cases. `lake test` in
  `verex-rust` builds the test library and runs no differential check, so
  nothing tests the denotation against Rust execution. Rebuild the tree over
  the plain layer, where values compute, and state `checked_tree_means`
  against the denotation with certificates forgotten.

## Milestones

The milestones of [`rust-mir-design.md`](../designs/rust-mir-design.md) in the
design's order. M0, M1 and the import path of M1.5 are in place except for the
rows of the deferred-work register below; M2's contracts and `verify` are in
place for the slice without general trait evidence.

### M1.5 — Lean build integration and managed toolchain

- A prebuilt exporter and sysroot distribution; the managed path builds the
  exporter from the checkout today.

### M2 — Basic semantics and contracts

- Verify generic bodies parametrically over trait evidence and method contracts
  beyond the single-method, `Fn`, `FnMut` and `FnOnce` dictionaries: associated
  types and their equalities, conditional method preconditions, supertraits,
  higher-ranked bounds, `dyn`.
- Constrained and const-generic function metadata; semantic normalization of the
  remaining validation-only bodies. General range indexing stays behind the
  standard-library dependency model.

### M3 — Ownership and ordinary references

- Regions over the prophetic model: dependency and unknown-aggregate alias
  propagation, nominal variance, the remaining use-site constraints,
  returned-reference region precision. Argument-changing recursive and external
  result-source selection stays conservative.

### M4 — Drop and panic

- Explicit destruction: a drop runs its type's drop glue and the destructor's
  effects, and guard and destructor effects are visible to proofs.
- Cleanup edges, `Resume` and effectful drop glue are rejection boundaries
  today; only trivial drops have a denotation. Unwinding and `catch_unwind`
  remain out of scope under the pinned `panic=abort`.

### M5 — Unsafe import and checked raw pointers

- Complete U0: preserve all unsafe-related MIR constructs and reject each
  unsupported semantic form with its Rust origin.
- Define the strict allocation/provenance memory profile and the explicit UB
  outcome; add no-UB safety obligations to the contract interface.
- Implement U1 typed raw-pointer creation, checked loads/stores, simple offsets,
  and modular unsafe-function contracts.
- Compare defined fixture behavior with rustc execution and use Miri only as a
  differential bug-finding test.

### M6 — Reference-bearing values and interior mutability

- An abstract `RefCell` store, dynamic borrow rules, and guard drop.
- `Rc` and the broader library models, staged with U2 where they require
  allocation semantics.

### M7 — Alignment

- State and prove the generic core-VIR semantic results.
- Validate or prove refinement from the MIR exchange through `RawUnit` to the
  supported `ValidatedUnit` fragment.
- Establish semantic equivalence for the
  `ValidatedUnit → Rust source → MIR → RawUnit → ValidatedUnit` round trip, and
  separately for supported Verex source backends. A check that the re-lowered
  unit equals the imported one up to the printer's temporary substitutions
  belongs here, with the exporter's results; until then both are trusted steps
  of a Rust theorem.
- Prove that generic type/const/implementation-evidence instantiation preserves
  the function meaning, so generic theorems transfer to concrete calls. Any
  later native specialization must be a separate refinement of this result.
- Compose import alignment with `f.verified` into an explicit theorem about the
  imported Rust MIR body. Rust-to-dictionary alignment, including selection of
  the declared trait's implementation, is part of that boundary.

## Deferred-work register

Rust-specific entries mirrored from the core VIR design; an item stays here
until it lands, is rejected, or is superseded by a recorded decision.

| Deferred item | Reactivate by |
|---|---|
| Approval and publication of RawUnit JSON v1 as an external compatibility contract | Before fixtures beyond the supported mirror become compatibility artifacts |
| Retire the confined Rustc Public polyfill | Replace `driver/polyfill.rs` wrappers with public queries when the pinned toolchain incorporates [rust-lang/rust#161892](https://github.com/rust-lang/rust/issues/161892); the mapper continues to use only public types |
| Remaining MIR-to-RawUnit mapper forms: symbolic const-dependent types, destructive/provenance administration exposed by Rustc Public, constrained and const generic functions | Once their metadata is public; M4 adds cleanup and effectful drop glue |
| Differential tests of the mapper against Charon's MIR export | With the remaining mapper forms (M1) |
| Checked implementation-selection evidence and dictionary interpretation beyond the static slices (`EvidenceId` is opaque) | M2, before general trait method calls execute or transfer contracts and proofs |
| Destructive/provenance nodes, cleanup edges, pointer-alignment assertions, and reachable `unreachable` blocks, rejected by structurization | M4 for destruction, cleanup, and richer panic behavior; M5 for retag/provenance and alignment assertions |
| Cleanup scopes and unwinding | M4 drop effects; full unwinding is a separately approved milestone |
| A proof-grade semantic-body/alignment receipt (the cache digest binds inputs, not alignment) | The shared proof-facing receipt gate; M7 before source-level theorem claims |
| Raw pointers, unsafe operations, allocation/provenance, and UB (rejected at the first unsupported node, no partial JSON) | M5, U0–U2 |
| Trait objects, specialization, higher-ranked bounds outside the initial fragment, async/coroutines, FFI, inline assembly | Separately scoped post-M2/M5 milestones with core/profile classification first |
| Library models (`Rc`, `RefCell`, and peers) | M6 or a separately approved library-model milestone |

## What stops real crates

First stop over 180 dependency-free registry crates: a local of `dyn` type 49,
raw pointer 22, `impl Trait` parameter 19, associated type alias 12, float 9,
const parameter in a signature 8; 27 export, with 104 of their 383 functions
refused. The limit is that a function whose signature the unit cannot state has
no contract, not even an authored one, so a generic iterator adapter and
everything calling it stay unverified. Per-function binders for associated
types, as a dependency's signature has them, would not help yet, since these
bodies are mostly refused for lack of trait evidence. Revisit both as trait
evidence grows.

The Rust benchmark problems `rust_math64`, `rust_bit_vector`, `rust_simple_map`
and `rust_lower_bound` are in `bench/problems.toml`, so coverage and cost are
tracked on the same algorithms in both languages. Closure-based iterator
combinators, general trait evidence and generic associated constants are what
they still reach. `add`, `remove` and `borrow_mut` of `simple_map` need proof
steps: a vector update under a uniqueness invariant is not automatic.

## Open design questions

1. What stability expectations and promotion thresholds should the rolling
   corpus enforce when optimized generic MIR changes between rustc revisions?
2. How are stable loop identities maintained across harmless MIR changes?
3. Which type-observing operations force specialized proof representatives
   instead of the generic theorem?
4. Which library types receive primitive models beyond those the specification
   library covers?
5. A rule for reading a subvector (`(v[a .. b]).length == b - a`,
   `(v[a .. b])[k] == v[a + k]` for `k < b - a`) belongs to the normal form,
   added when a fixture first reaches it.

## Dependencies on the verification work

Native verification is done, and so are the proof language's obligation keys,
path decisions and selectors (L0) and the loop points of L2. The library models
and contracts build on native values and their roles. The structurizer, the
printer, the exporter, loop identities and the crate driver do not depend on
that work.

## Open decisions

1. How a stale loop reference is detected. Move detects none: its invariants are
   written in the loop, and a proof file's `loop k` follows source order, so
   adding a loop before another shifts its number. Since invariants and facts at
   points are proved, a shifted reference fails rather than proves something
   wrong. A Rust specification file holds the invariants themselves, outside the
   source. Proposed: the reference may spell the loop's condition,
   `loop 1 while i < values.length where …`; lowering compares it with the
   condition of the `k`-th loop, up to locations and compiler temporaries, and
   reports a mismatch as stale with the loop's current condition. Without it,
   Rust behaves as Move's proof files do, plus the check that the invariant's
   locals are the loop's. A stored fingerprint would be a second artifact to
   keep in sync. Parked, with anchoring by loop labels, on the backburner of
   [`roadmap.md`](roadmap.md).

# Closing the Rust verification gaps

Refines section 7 of [`roadmap.md`](roadmap.md) and
milestones M2, M1.5 and M6 of
[`rust-mir-design.md`](../designs/rust-mir-design.md). The unsafe profile
stays in [`unsafe-pointers.md`](../designs/unsafe-pointers.md).

## Implementation boundary and next gates

The sections below include the rationale for work already implemented; they
are not a list of outstanding features. The current frontend supports checked
arithmetic and panic locations, reducible loops and invariants, native library
contracts at shapes, declared lifetime bounds, const-generic metadata, promoted
constants, direct closure calls, two-phase borrows, and the supported `Box` slice.
The exporter, renderer and denotation each have separate rejection boundaries;
successful export alone is not successful verification.

The confined [`rustc_public` polyfill](rustc-public-polyfill.md) is an accepted
temporary exception until the upstream Rust bug is fixed in the pinned toolchain.
Missing upstream queries no longer block the local use of predicates, const
parameter types, promoted constants, caller-aware resolution or macro locations.
Compiler-private access stays in the bridge. Replace each helper with its public
query and retire it when that query becomes available; this exception does not
authorize moving compiler internals into the mapper.

The first three implementation gates are:

1. Establish the restacked baseline across all four Lake packages, exporter
   checks, and the full Move Prover Lean suite. Record actual differential
   outcomes separately from inconclusive cases; an allowlisted generic call is
   not evidence of interpreter/denotation agreement.
2. Close the generic interface gaps: require shape contracts where dependency
   associated types have been promoted to independent binders, expose the
   trivial-destructor requirement of dropped abstract parameters, and retain
   closure receivers and their state in higher-order contracts. Include negative
   tests for each boundary.
3. Carry static trait evidence through one narrow end-to-end slice:
   `Step::step(&mut self)` and `step_twice<T: Step>`. Prove the generic body once,
   check method signatures and implementation contracts, instantiate two
   implementations, reject malformed evidence and incorrect contracts, and
   compare interpreter execution. Associated types, supertraits, `dyn`, and
   effectful destructors follow after this gate; concrete dispatch alone does
   not satisfy it.

The restack comparison uses parent `059a708b0c2` (`wrwg/verex-fx`). A clean
parent verifier reproduces the six full Move Prover baseline failures in
`ghost_field_intrinsic_map_ops`, `loops`, `references`, `math8`,
`lambda_variant_field_access`, and `closures/inline/folds_of_wrapper_mut`.
It also reproduces the source-verification failures in
`behavior_nominal_arguments`. These are inherited failures, not passing gates;
their expectations must not be updated merely to conceal the failures. The
parent's stale denotation-performance baseline was regenerated independently
and used unchanged to check this branch. The final full Move Prover run is
462 passed / 6 failed, with exactly those same failing fixtures. The IR and Move
package suites pass. The exporter passes 24 tests, `cargo check`, formatting,
and Clippy. The new positive/negative source tests and generic-proof reuse audit
pass; the differential suite checks 3,653,449 inputs with 955 explicitly
inconclusive cases (not counted as agreement).

The `bench_bit_vector_errors` timeout is also present in clean pre-change
HEAD `1661d5efd0d`: replaying the same rendering gives identical diagnostics
and the same 173 decided leaves. Its budget and expected output are unchanged.
The shape-only guard required migrating the `u128`-to-`u64` conversion contract
to its concrete `TryFromIntError` shape; positive and negative conversion cases
are now included in the Rust smoke gate.

The complete E2E driver was replayed in an isolated copy with generated
expectations compared against the working tree, so a baseline mismatch could
not stop coverage early. All 124 Move and 181 Rust source fixtures, package
verification, Check fixtures, VM checks and differential checks completed.
After reviewing and refreshing diagnostic-only changes, the only remaining
mismatches are `behavior_nominal_arguments` and `bench_bit_vector_errors` above.
The expanded Rust smoke run passed 40 of 41 cases; the remaining group passed
on read-only replay after refreshing its diagnostic-only expectation. No
verification budget or inherited failure expectation was weakened.

General trait evidence is still unimplemented. The bridge supplies the compiler
facts needed to implement it; it does not turn them into checked VIR evidence.

### First static dictionary slice

The local single-method `fn step(&mut self)` slice lowers to a typed function
value parameter, marked by `@[method_evidence = n]`, where `n` is its parameter
index. The dictionary takes the receiver explicitly and captures nothing. The
exporter obtains it from a positive declared trait predicate, forwards it through
generic calls, and resolves a concrete implementation in the caller's typing
environment. Generic bodies are not cloned.

The shared validator checks the dictionary parameter's signature and rejects
out-of-range indices; ordinary closure typing checks a concrete implementation's
receiver and result. Generated contracts require `MethodContract`: the method
is defined, does not panic, and preserves global memory at every receiver value.
This is a proof obligation for the concrete dictionary, discharged through the
implementation's verified contract or body, not a trusted native assumption.
Receiver changes remain in `EnsuresOf`'s before/after arguments. The generic proof
can compose those transitions without knowing the implementation.

This deliberately small interface does not yet support conditional method
preconditions, arbitrary method results, multiple associated items, associated
equalities, supertrait evidence or `dyn`. The `Fn` and `FnMut` slices below
uses the same checked dictionary interface. Ordinary Rust methods
may panic; only implementations establishing this slice's stronger contract can
be used to verify its generic callers. Rust-to-dictionary alignment, including
selection of the declared trait's implementation, remains in the exporter's M7
trust boundary. It is not a general implementation of VIR's trait declarations.

The `static_step` source fixtures cover a counter, a toggle, and generic
forwarding; negative fixtures reject false implementation/caller contracts and
a method that can panic. `StaticMethodEvidence.lean` audits reuse of the generic
theorem. Exporter native execution and Rust profile interpreter tests use the
same normal and overflow-boundary cases, and the profile tests corrupt a
dictionary's receiver and parameter index. These evidence fixtures have a
separate exporter directory, included in the default differential suite with
zero inconclusive inputs required. The denotation-tree evaluator executes
`callGeneric` through a specialized compiled body and `invoke` for dictionaries
including checked captures and runtime type instantiations. The
`method_evidence` differential fixture also checks a panicking implementation through generic forwarding; this
is execution coverage, not evidence that it meets `MethodContract`.

### Stateful `FnMut` dictionaries

A declared `F: FnMut(A, ...) -> R` bound supplies a dictionary taking `&mut F`
and the spread arguments, retaining generic argument and result types. The exporter requires the
declared `FnOnce::Output` equality and identifies the compiler's trait and
associated item by identity, not spelling. It forwards the dictionary through
generic calls without inferring a bound from observed calls or cloning the body.
Concrete dictionaries target mutable closure bodies or checked adapters that
reborrow an `Fn` closure's receiver as shared. Multiple bounds get distinct,
collision-free evidence names. Closures inside bounded generic functions capture
the inherited dictionaries as partial applications, preserving their contracts.

Move-captured state remains in the closure struct. `ensures_of` relates the old
receiver, result and new receiver, in that order. `MethodContract` still requires
an everywhere-defined, non-panicking method preserving global memory; the
closure's verified contract must discharge it. The `static_fnmut` fixtures prove
counter and toggle updates through opaque generic helpers and forwarding, and
`FnMutEvidence.lean` checks reuse of those generic theorems. Native Rust execution
and the strict evidence differential suite compare the same mutation and
wrapping-overflow cases. Negative fixtures cover false caller and closure
contracts, panics, malformed dictionaries, and unsupported bounds.
`evidence_composition` adds multiple dictionaries, name collisions, inherited
evidence, receiver adapters, argument tuples, and generic/struct results. The
exporter completes types along generic call edges, including transitive calls,
rather than interning an unrelated receiver/result cross product.

Closures capturing mutable references retain the existing refusal when an opaque
generic contract would have to transfer to a type containing a loan. No loan or
receiver state is erased to make that call verify. Consuming `FnOnce` dictionaries
are described below; general associated-type equalities and conditional method
contracts remain open.
Tuple-returning callbacks use the target's packed result throughout the native
function type, invocation, and generic contract transport. The `tuple_callbacks`
fixtures cover opaque forwarding, generic results instantiated by tuples,
single-element and nested tuples, and shared-receiver adapters, with positive
and false contracts and differential execution. Source `ensures_of` keeps its
separate result-element operands; translation packs them into the result value.
The representation is described in
[`higher-order-functions.md`](../designs/higher-order-functions.md#function-values-of-tuple-returning-functions).
Tuples containing borrowed references still need aggregate sharing and loan
transfer support. Move regression coverage for this shared change is deferred.

### Shared `Fn` dictionaries

A declared `F: Fn(A, ...) -> R` bound supplies a dictionary taking `&F`, using
compiler identities and the declared `FnOnce::Output` equality. It calls the
shared closure body directly and forwards the evidence through generic helpers.
The existing `MethodContract` proof supports this shared receiver without a new
axiom or an extra receiver-state output in `ensures_of`.

`static_fn` covers scalar and generic results (including tuples), arguments,
multiple independent bounds, and repeated unit-returning calls. Native Rust,
strict differential execution, dictionary validation, source proofs, and false
contract/panic regressions exercise the implementation. The formerly refused
`closure_generic_call` fixture is now a positive export regression. Consuming
`FnOnce` dictionaries are a separate slice below; higher-ranked bounds and
conditional method contracts remain open.

### Consuming `FnOnce` dictionaries

A declared `F: FnOnce(A) -> R` bound supplies an explicit dictionary taking `F`
by value. Generic helpers forward that dictionary without adding a `Copy` bound
or specializing the body. Concrete evidence targets a closure's `call_once`
body, retaining its non-`Copy` captured values and the receiver's consumption.
The existing method contract still requires a non-panicking, memory-preserving
invocation; owned receivers use the same checked invocation semantics.

`static_fnonce` covers opaque generic helpers and forwarding, generic arguments
and results, scalar and tuple results, and nullary unit calls. Native execution,
strict differential checks, validation of malformed dictionaries, and source
proofs exercise the slice. `FnOnceEvidence` checks reuse of the generic proof.
False caller contracts and a potentially panicking consuming callback are
rejected. Concrete `Fn` and `FnMut` closure bodies use checked consuming
adapters: borrow the owned receiver, call the original body, then explicitly
drop the receiver. Generated contracts preserve the callback's behavior; for a
mutable target they existentially retain its final receiver state. The
`fnonce_adapters` fixtures cover scalar, tuple, and unit-returning callbacks,
with false-contract regressions and strict differential execution. The source
proof fixture also instantiates the generic helpers' result `R` with `()`, as
described below.

Generic `Fn`/`FnMut` dictionary forwarding to `FnOnce` uses a checked
receiver-noescape requirement. The validator certifies that the receiver loan
cannot escape through the result or mutable writebacks; supplying a dictionary
requires that receipt and recursively checked captured dictionaries. The
interpreter and denotation enforce the same requirement at function entry.
The exporter propagates it only along dictionary paths used by consuming
adapters. `fnonce_forward` exercises scalar and tuple results, mutable receiver
updates, and forwarding through shared and mutable adapters. Unmarked or
escaping dictionaries remain rejected. Destructor effects,
mutable-reference capture transfer, and conditional method contracts remain
outside this slice.

### Generic callable Unit results

A generic callable result `R` instantiated with `()` now has the same empty
native result row as an explicitly Unit-returning callable. A shared `&Unit`
result retains its returned slot and sharing mark. Since substituting Unit in
the generic row does not erase that slot, the compiler checks whether the
native reading agrees with substitution before reusing a generic theorem.
Unit placeholders for lifetime and const binders do not trigger this check;
only Unit in a type-parameter slot can erase a callable result slot.

For supported opaque helpers, it compiles the existing body at the requested
frame and independently proves the same authored contract there. The caller
uses that checked contract, preserving opacity, preconditions, and mutable
receiver effects. This keeps one generic source body; it does not claim reuse
of the generic theorem across incompatible rows. Transferable instantiations
continue to use the existing generic proof path.

The bounded opaque path requires reference-free type arguments and acyclic
bodies whose calls are callable invocations. Borrowed receiver parameters are
supported. Direct function-call chains, loops, body specifications or
assumptions, and model state remain unsupported in this path; carrying their
conditions into an independently checked specialization is still follow-up
work. A native declaration whose contract does not transfer to the Unit reading
also remains unsupported: this path requires a body to check.

`SourceVerify/fnonce_adapters` now calls the opaque generic `mutable_call` and
`once` helpers at `R = ()`; the dedicated unit-only helper workaround is gone.
`VerexSL.Tests.GenericUnitResults` checks Unit and non-Unit proofs, preservation
of a mutable receiver update, rejection of a missing helper precondition, and
the shared-Unit result row. It also checks a scoped dictionary at the Unit
specialization and rejects missing scoped evidence. Generic borrowed-to-consuming
forwarding at `R = ()` still encounters the direct-call-chain limitation of this
specialization path.

### Generic `Fn` to `FnMut` forwarding

An `Fn`-bounded helper can now pass its callback to an `FnMut`-bounded helper.
The exporter creates an adapter only at a call edge that needs one. The adapter
captures the declared shared dictionary, takes `&mut F`, and invokes that
dictionary through a shared reborrow. It neither specializes the generic body
nor adds an independent `FnMut` assumption to the caller.

Rust source verification synthesizes ordinary specifications for the marked
adapter bodies: the receiver is unchanged and the result obeys the original
shared dictionary's `ensures_of`. These specifications are proved, not assumed.
Concrete closure receiver adapters use the same specification generation,
referring to the source closure's `call` method. Neither adapter kind requires
user specifications naming generated functions. The solver follows nested
concrete callback facts through verified target contracts and avoids expanding
the same observation twice. Derived callback facts use the existing
instantiated-frame transport theorem before comparison with the caller's facts. Generated adapters do not participate
in Cargo module-file discovery; their specifications are supplied separately.

The `fn_forward` fixtures cover generic and tuple results, argument forwarding,
repeated calls, multiple independent bounds, and nullary unit calls. Source
proofs are split into `fn_forward`, `_multi`, `_unit`, and `_tuple` to keep each
unit small. The repeated unit-returning case verifies with both helpers opaque
at the default solver budget. Frame lookup compares hypotheses at reducible
transparency, avoiding evaluation of unrelated invocation semantics or frame
instantiations. `_errors` rejects false results, a false implementation contract,
and a potentially panicking callback; `_unit_errors` rejects false receiver
changes, denial of the forwarded callback's behavior, and a behavioral claim
about a callback that was never called. The combined evidence fixture remains
subject to the strict differential-execution gate. The denotation evaluator now
constructs generic closures only after checking their frames, carries those
frames through calls, decodes captures separately, and retains the supplied
arguments' mutable-reference prophecies. Its evaluator-to-denotation theorem
covers these cases without new axioms. Optimized, unassigned zero-sized locals
borrowed mutably are materialized before borrowing, so capture-free callbacks
also execute in the interpreter. Existing loan restrictions
and the requirement for unconditional non-panicking method contracts remain.

## Problem

A Move file and a Rust file reach the same verifier: each becomes a
validated unit, is rendered as VerexSL, lowered back, compiled to a
denotation and verified
([`source-verification.md`](../designs/source-verification.md)). For Move
this path verifies the benchmark's standard-library and framework modules up
to `ordered_map`, runs the Move Prover's test corpus, and verifies the
`SourceVerify` fixtures. Rust now has a broad `SourceVerify` corpus covering
arithmetic, loops, declared regions, library contracts, generics, closures,
promoted constants and references. Its positive tests are paired with false
contracts and refusal tests; the original two fixtures no longer describe its
coverage.

Where a Rust body reaches the verifier, it behaves as for Move: a missing
precondition on `a + b` fails as a possible panic, and a caller verifies
against its callee's contract. Small safe functions outside that shape stop
earlier:

| Stage | Behavior | Cause |
|---|---|---|
| Export | General parameter-dependent trait calls, including derived generic `Clone`, are rejected. | General checked dictionaries and associated-type equalities remain M2 work. Dependency projections such as indexing use promoted binders and shape contracts. |
| Validation | A loop whose exits continue to code they reach only through a cycle, or not at all in common, is rejected (`VIR-CFG-MULTI-EXIT`). | The structurizer needs one block after the loop that every continuing exit reaches without a cycle; early returns, `continue`/`break` of enclosing loops, and valued `break`s meeting after the loop are supported (`RustToVerexSL/loop_exits.rs`). |
| Validation | A mutable borrow of an `unsize` or `subslice` place is rejected (`VIR-SEMANTIC-BORROW-UNSIZED-HOLE`). | The runtime's hole cannot sit in a slice of a vector. |
| Denotation | A `&mut` held in an enum is rejected (`a loan held inside an aggregate`), as is a reference type argument of a function value or a native, opaque or recursive callee. | A loan's death resolves a local or a tuple's or struct's component at its path. |

The `RustToVerexSL` baselines check that a rendering lowers back to a
validated unit and prints as a fixed point, and list each function that
compiles to no denotation with the construct that stops it.

## Goal

The first useful slice of `rust-mir-design.md` ("Control flow and loops":
functions, structs, enums, ordinary references, selections and reducible
loops), together with `Option`, `Result`, `Vec` and slices, verifies
through `verex-rust verify` with Move's contract language, native
denotation and solver ([`native-verification.md`](../designs/native-verification.md)),
and proof files written in the proof language
([`proof-language.md`](proof-language.md)), and every failed obligation
names its key and its Rust source location.

The goal beyond that slice is the semantics of all safe Rust. What verifies must
be true of real Rust, and what is not modelled yet is rejected, never given another
meaning. What remains: general trait-bounded generics and `dyn` (the local
predicate-query prerequisite is supplied by the temporary polyfill), destructor
effects and unwinding (M4), interior mutability (`interior-mutability.md`, whose cell
identities keep `&T` a snapshot), `Box` and recursive types (step 3e), floats, and
concurrency. Closures are step 3d, following
[`native-function-values.md`](native-function-values.md) ("Rust"). Unsafe code is
restricted so that verification guarantees what safe Rust assumes: a misuse is a located
failed obligation, or refused, and the checker may be stricter than Rust
(`unsafe-pointers.md`, `interior-mutability.md`, §7 of each). So a safe interface over
unsafe code is not an axiom in principle; the library's contracts are trusted only until
their bodies verify under that profile.

## Step 1. Imported bodies verify


**Printing in the alignment chain.** A theorem is about the rendered unit,
and the printer writes a compiler temporary as its value only where the
footprint analysis of `VerexSL/Print/Temporaries.lean` shows that keeps the
body's behavior. A check that the re-lowered unit equals the imported one up
to those substitutions belongs to M7's alignment results, with the
exporter's; until then both are trusted steps of a Rust theorem.

**Drop flags.** Under `panic=abort`, rustc guards the drop of a value moved
on some paths only by a boolean local, its drop flag. Definite
initialization records, for a boolean local assigned a literal, what is
initialized while it holds `true`, and a branch on it assumes that; a
borrowed local records nothing from its borrow on
(`RustToVerexSL/drop_flags.rs`). The structurizer treats an `unreachable`
loop exit, a switch's impossible default, as one that never continues.

**Checks folded.** Each check rustc inserts folds into the operation it
guards, so that a site follows the Rust source. Folding belongs to the
structurizer, which still has the check's kind (`RawAssertKind`), and is the
semantics-preserving normalization `rust-mir-design.md` names for these
assertions; the exporter stays a mechanical mapping. The imported and the
re-lowered unit then agree node for node, which message alignment reads, and
`checkedArithmeticShape?` is retired:

- An overflow check folds its `overflowing*` operation into `checked* .panic`
  wherever the tuple local is written once and read only by the check and at
  its value, in any position, loops included; those reads become a new local
  of the value's type.
- A division's or remainder's checks, the signed division's second one
  included, fold into `checkedDivide .panic` and `checkedModulo .panic`, a
  negation's into `checkedNegate .panic`, a shift's range check into the
  checked shifts.
- A bounds check folds into `checkVectorIndex .panic` before the element
  access, and Rust's lowering of `v[i]` emits that check as Move's does, so
  the printer elides it and the rendering stays `v[i]`. An array's check
  compares against a literal, which folds where it equals the array type's
  length.
- An operation rustc leaves unchecked because it cannot fail, as with a
  literal divisor or a literal distance in range, is the same checked
  operation, so every division and shift is an `op` site.

A check folds only where its condition is the exact failure condition of the
operation it guards, over the same operands, with nothing observable between
them. Any other check stays the conditional it is today, which is sound and
counts as an `if`. A panic's payload is not observable, so a fold that changes
it preserves behavior.

**Rust primitives in the denotation.** Every primitive and type the Rust
profile admits gets a native denotation: its values are carriers, `usize`
and `isize` the integers `NTy.int w` at the unit's `target_pointer_width`
`w`, and an overflowing operation's result the row of its value and flag.
What the baselines stop at after folding: integer `as` casts (truncating and
sign-extending); bitwise not; bitwise operations and checked shifts on signed
integers (`Term.bitwise` and `Term.shift` are unsigned today), and bitwise
operations on `bool`; comparisons of `bool` and `char`; discriminant reads;
`char`, arrays (a vector with a fact of its length) and `!`; tuple element
places; lifetime arguments of a nominal, which no carrier reads, so that a
shared reference in a field is its referent's value. An unchecked operation no fold reaches is carried as the partial
operation it is: `Spec.stuck` outside its domain, where the runtime has no
outcome, so the theorem owes the domain. `.length` stays carried at width 64,
and a target of another width is reported as unsupported until a fixture
needs it. Each case follows the operation's meaning
in `VerexIR/Semantics/Operations.lean` through the type's codec
(`NTy.codec`), and the agreement with `BigStep` extends case by case
([`denotation.md`](../designs/denotation.md)). The native-only check holds
of every case, on contracts (`reportRuntimeValues`), leaves
(`verex_check_native`) and Rust proof files (`reportAuthoredRuntimeNames`):
a case that needs a runtime value is unsupported, never encoded. A test enumerates
what the Rust profile admits and fails on any primitive or type without a
native case.

**Normal form.** The solver states an overflowing operation's flag as the
range condition of the exact result, as it already states the panic of
`core.prim.checkedAddPanic` (`¬(0 ≤ a.val + b.val ∧ …)`), and a truncating
cast as its value modulo the width, so a panic branch is decided as for
Move's checked operations. These rules change the normal form, so they are a
step of their own, gated as the normal-form steps of `proof-language.md`
are: every automatic target unchanged in outcome, at or below its
heartbeats. Folding leaves the flags of calls to `overflowing_add` and its kin, which the
exporter maps to these primitives, and of checks it does not fold.

**Drops.** A drop has no effect on the values a verified body reads, as in
the runtime, and the denotation says so, at a type parameter and at a row of
them. M4's destructor effects remain out of scope, and nothing may erase them
silently, so two limits hold until M4:

- An ADT has a destructor when it or an ADT reached through its fields has a
  `Drop` impl; the exporter states it per ADT from the crates' impls, as it
  states `Copy`. A drop at a type with a destructor is rejected at its location.
  `S<T>(T)` with no impl of its own reaches only `T`. Dependency types such as
  `Vec` declare their destruction by a role in step 3 and are rejected until
  then; an opaque dependency type also counts as having one where its fields
  hold a type argument, which the unit cannot see. A function value's captures
  are not visible in its type, so a drop of one is rejected. A shared reference
  denotes its referent, so a field or nested argument of one counts its
  referent's destructor, which rejects more than rustc would run.
- A theorem about a body dropping a type parameter assumes that the argument's
  destructor has no observable effect and does not panic, recorded in
  [`assumptions.md`](../status/assumptions.md). A call that instantiates such a
  parameter at a type with a destructor is rejected where the specialization
  instantiates it.

**Generic callees.** A call of a generic function without trait bounds
reads its callee's specialized contract
([`native-verification.md`](../designs/native-verification.md), SC1–SC5),
as a Move call does; nothing in it is Rust-specific.

**Failure contract.** A panic is the failure kind `panic` of
`Failure := ThrowKind × Array Int`; its message is not carried, so no
clause reads it. Lowering appends an implicit `aborts_if false` to every Rust
contract ([`verex-sl.md`](../designs/verex-sl.md)), so a panic is permitted
only where a user's `aborts_if` condition holds, and a function without one
owes that no path panics. That is what `aborts_if_is_strict` gives a Move
function, by another path: `isStrict` is false for Rust, and a Move function's
`False` belongs to no clause (`VerexSL/Contract.lean`). An `aborts_if` without
a code permits a panic where its condition holds; a code never matches a panic
(`abortCodeMatches_panic`). Whether Rust spells these clauses `panics_if`
instead is open decision 2.

**Panic obligations.** They follow the proof language's addressing
([`proof-language.md`](proof-language.md), "Obligation addressing and
provenance"). A failing path of a function with `aborts_if` clauses owes
them, under their keys. The failing paths of a strict function without
them are one obligation, which has no clause and is keyed `leaf` today, for
Rust and for Move alike; a selector takes paths out of it by their abort
site (`when aborts at op 3`, `when aborts at abort 2`,
`when aborts at call 1 of g`). This step adds:

- One key for that obligation, shared by both profiles and located at the
  function's `spec` item: printed `no panic` for Rust and `no abort` for
  Move (`on no panic when aborts at op 3 { … }`), added to the key table
  of `proof-language.md`. For Rust it names the implicit clause, which a
  condition property marks and `clauseNames` names `no panic` without counting
  it among the `aborts_if` clauses; for Move it names the strict function's
  `False`, marked at the `spec` item.
- The implicit clause never supplies the contract's location. Today a Rust
  function with no `spec` item takes its `contract.loc` from that clause
  (`lowerContract`), so on a call cycle it is used through a contract that
  nothing verifies; with `contract.loc = none` it is unspecified, as in Move.
- Sites that follow the Rust source. A folded check is one `op` site, marked
  as Move's checked operations are (`opAt`). A user `assert!` is an abort
  site, as Move's is, never an `if`; so are `panic!` and `unreachable!`; a
  call to `unwrap`, once step 3 models it, is a call site.
- Sites are numbered per kind by their position in the rendered body. With
  the checks folded, `if k` and `match k` count what the Rust source
  writes, and `op j` counts operations, including those whose check the
  source shows only as an operator (`a / b`, `v[i]`). A `v[i]` is one `op`, in
  both profiles: today its check and its element read are numbered apart.
- The message names the operation and its `.rs` position, and the proof
  request prints the path's selector. Alignment of a rendering with its source
  (`alignExpr`) is shallow, because the rendering folds temporaries, so a site
  takes its position by the order the two bodies run the sites of each shape
  (`renderedSiteOrigins`), and other messages inside a body still map coarsely.

**Struct specifications.** A struct's data invariants and ghost fields are
clauses of its `spec S where` item in the specification file
([`ghost-fields.md`](../designs/ghost-fields.md)). Stored invariants and
static memory have no Rust counterpart: a Rust unit has no resources.

Gate: Rust `SourceVerify` fixtures for arithmetic at every integer
width including `usize` and `isize`, `overflowing_add`, casts, division, structs and field updates
through `&mut`, a struct with a shared reference field, enums and `match`, arms computing values, functions
returning `&mut`, `()` functions with branches, a generic function dropping a
parameter, and a struct invariant. A drop at a type with a destructor is
rejected at its location (a local `Drop` impl's method has a scope, so these
Rust fixtures stop at the printer until the `mod` item of step 3), and an
unspecified function on a call cycle is reported, as in Move. The `RustToVerexSL` baselines show folded checks,
in loop bodies too, and no `-- no denotation` line for a construct this step
names. Each has a `_false` counterpart that fails under the key of its clause, or under
`no panic` with the panic site's decision. A proof file routes one of two
panic sites of a function by its selector, and a division's site is an `op`,
not an `if`.

## Step 2. Loops

**Loop invariants.** `rust-mir-design.md` attaches loop invariants to
imported structured-loop identities. This step implements that attachment:

- A loop is addressed as the proof language addresses it
  ([`proof-language.md`](proof-language.md)): `loop k` is the function's
  `k`-th loop in the rendered body, counted from 1 (`bodySites` in
  `VerexSL/Verify/Contracts.lean`), and an invariant's obligations are
  `invariant 2 of loop 1 entry` and `invariant 2 of loop 1 step`, so a Rust
  function's specification and proof files name a loop alike. A `loop k` for
  which the function has no `k`-th loop is an error at the item. A number that
  shifted is caught as in Move's proof files, by the invariant failing or
  naming a local the loop does not have (open decision 1).
- The specification file addresses a loop inside its function's `spec`
  block, and lowering places the conditions where an in-body loop invariant
  goes, when it lowers the spliced item: the rendering is composed before the
  specification file is elaborated, so the `.rs.lean` keeps the imported body,
  and messages map the conditions to the specification file through its
  splice. `loop k where` in a `spec` block is new VerexSL syntax (parser,
  elaboration, printing); today a loop's specification is a member of the
  loop in the body (`verexLoopSpecificationMember`, `VerexSL/Syntax.lean`):

  ```lean
  spec sum where
    requires values.length <= 1000
    ensures result <= values.length * 4294967295
    loop 1 where
      invariant i <= values.length
      invariant s <= i * 4294967295
  ```

- The specification file holds the conditions only. Facts and proof steps
  at a loop are proof-file steps at the proof language's points
  (`at loop 1 entry`, `at loop 1 step`), as for Move.
- Invariants name the loop's source locals. A local rustc removed or
  renamed is an error at the clause.
- A unit printed after lowering shows attached invariants in place, so that
  it remains a fixed point of printing.

Gate: counting, summing a slice, a search returning an index, and a nested
loop verify; a wrong invariant fails at its clause; a `loop 3` in a function
with two loops is an error at the item; a body whose overflow checks fold shows
them as `op` sites inside the loop.

## Step 3. Dependencies and the standard library

**Scopes in VerexSL.** Every dependency function and every method lives in a
scope, and a scope's keyword carries its kind: `mod m where`, `impl Counter
where`, `trait Step where`, `trait impl «<Token as Drop>» where`. A dependency's
function renders as `native fun`, its unobserved type as `opaque struct`. Open:
a trait impl's name is rustc's text, all the exporter states; spelling it `impl
Drop for Token` needs the exporter to state the trait and the type as data.

**Dependency namespaces.** Each crate the unit reaches (`core`, `alloc`, a
Cargo dependency) is a namespace of the unit, as a Move package's dependency
modules are: it holds the declarations the crate reaches, in their modules'
scopes, and states `verify = false`. A dependency's enum carries its shape, its variants being public, as `Option`
and `Result` do, unless a payload's type has no mapping; a struct carries it when
the crate observes it (constructs or projects). Any other is opaque (the Rust
property `struct.opaque`), and validation rejects constructing or destructuring it. Safe Rust
observes only visible fields, so private fields never enter the unit. The
interfaces of `RawUnit.dependencies` remain the boundary of separate
compilation ([`relocatable-namespaces.md`](../designs/relocatable-namespaces.md)),
where a precompiled crate links over its namespace.

An associated function belongs to the scope of its type, trait, or trait
impl (`Vec::push` is `push` in scope `vec::Vec` of `alloc`); a primitive
type's in `primitive::<type>` (`u32::checked_add`). Every call is resolved
as codegen resolves it, so a trait method called on a concrete type reaches
its impl's method (the unit's evidence states it). A called dependency
function is declared by its generic signature, generics of its impl first,
without its body (the Rust property `function.dependency`); verification
rejects calling it until a contract or model gives it meaning. A call on a
type parameter stays rejected until trait evidence (M2).

**Associated types are type parameters.** Rustc Public has no normalizing
signature query: the signature of a resolved instance still names the alias, and
an impl's binding may be another projection (`Vec`'s `Output` is `I::Output`), so
VIR could normalize a projection type only by reimplementing trait selection.
The types in MIR are normalized. So each distinct alias in a dependency signature
becomes a generic parameter after the function's generics (`native fun index {T}
{I} {A} {Output}(_1 : &Vec<T, A>, _2 : I) -> &Output`, binders in rustc's order), and a call instantiates it
by unifying the declared signature with the call's operand and destination types,
wherever the alias is nested (`Option<&Output>` of `get`). This is
`rust-mir-design.md`'s "explicit type parameters plus equality constraints after
rustc normalization"; the equality is a fact of the exporter's output, trusted
until M7.

A trait-method call whose impl selection depends on a type parameter is rejected
at export until trait evidence (M2): `Instance::resolve` fails on `&v[0]` with
`v : &Vec<T>`.

**Contracts at a shape.** A contract may name a shape of its function's generics,
and applies to a call whose instantiation unifies with it: `spec index at {T} {usize}
{A} {T} where …`. A shape is a generic contract over its own variables: the
callee's contract is looked up by the callee and the shape unifying with the
call's type arguments, and specialization (SC1–SC5) goes on unchanged. Two shapes
matching one call are an error of the library. A call matching none has no contract
and is rejected at its location.

The temporary polyfill supplies declared predicates, macro call sites, promoted
constant evaluation and declared `Copy` bounds. General trait predicates and
associated equalities still need checked evidence; availability of the query
alone does not implement that evidence.

**A standard-library specification library.** VerexSL files maintained in
`verex-rust/lib/{core,alloc}` give contracts to the `core` and `alloc` functions
the fixtures and benchmark problems use: integer `checked_*`, `wrapping_*` and
`saturating_*`, `Vec::new`, `push`, `pop`, `len`, `get` and indexing, `Option` and
`Result` accessors. A unit declares only the dependency functions it reaches, so
the driver keeps the library items whose target the unit declares before it
splices them.

`Vec<T, A>` is the VIR vector type and `String` the VIR string. The library
declares it (`@[model(vector T)] opaque struct Vec {T} {A} where`), and the Rust
import rewrites the raw unit before validation: every `Vec<T, A>` is `Vector<T>`,
the declaration is dropped, its scope keeps its name, and `A` is an unused
parameter of each native. Runtime typing, `readsUnique`, `typesAgreeCheck` and the
denotation are unchanged, since the value is a vector; `deref` is the identity; a
drop of `Vec<T>` is a drop of `Vector<T>`, so `Vec<u32>` passes and `Vec<Guard>` is
rejected by the existing rule. A nominal carried as a vector would not do: the
native reading is injective (`readsUnique`), and a type a value reads at must be
the one the runtime types it at (`typesAgreeCheck`).

The functions are natives with library contracts, at a shape where needed (`spec
index at {T} {usize} {A} {T} where aborts_if i >= self.length; ensures result ==
self[i]`; `spec push where ensures self == old(self) ++ [x]`). `push` has no
failure, capacity and allocation being outside the model (`assumptions.md`). `get`
returns `Option<&T>`, whose shared reference is its referent. `v[i]` on a `Vec` is a
call site, `aborts at call j of index`; an import normalization to the `op` site is
a later step. The Rust backend prints an owned vector as `Vec<T>` and a borrowed one
as `&[T]`.

The library declares what the import rewrites, so that generic code
never matches a Rust path. The exporter maps `wrapping_shl` and
`wrapping_shr` to the bare shifts, which have no outcome where Rust masks the
distance; this step gives them library contracts (`distance mod width`) and
removes that mapping, so they stay dependency calls. The other `wrapping_*`
mappings stand: `wrapping_add` is the total modular `add`. A call to a dependency function with
neither a contract nor a model is rejected at its location, as a Move native
without a specification or model is. Library contracts are trusted and each
theorem lists those it uses; verifying them against the generic MIR of their
bodies needs the unsafe profile for most of `alloc` and comes after its
library models ([`unsafe-pointers.md`](../designs/unsafe-pointers.md), §6).

**`for` loops.** A `for` loop is the loop rustc writes: `Range` is an observed
struct, `next` and `into_iter` have contracts at integer shapes, `iter` is the
name rustc gives the hidden iterator, and the invariant names its fields (`loop 1
where invariant iter.start <= iter.end && …`); a nested loop's is `iter'`.

Gate: fixtures with methods, with `Option` and `Result` in signatures and
matches, `checked_add` and `wrapping_add`, `Vec` push, pop, length and indexing under
loop invariants, and updates through `&mut [T]`; a call to an unmodeled
standard function is rejected with its location.

## Step 3b. References in aggregates

A measurement of 135 rustc-accepted functions holding references in tuples,
structs and enums let no false contract through. It stopped 71 at rendering, 26
at export, 17 at the borrow analysis and 8 at the denotation. The runtime already
nests borrows: only the denotation is flat.

**Carrier.** A nested `&mut` keeps the carrier `(current, prophecy)` and its path
is a `Proj`. `refFree` stays required for equality, order and search.

**Death.** The certificate states each loan's holder as a path, and a loan dies at
its path (`Term.resolveAt`), a no-op on an inactive variant. Two loans of one
struct are two deaths at one anchor. A death assumes `current = prophecy`, so
resolving a copy already moved out would assume a false equation and make the path
vacuous. Hence validation reads a reference moved out of an aggregate as a reborrow
through it, as `normalizeReferenceCopies` reads a copy of a local, so the component
stays true on every path, a conditional move included: its other path resolves the
component at the value it kept. A death states what the place
holds at that death, not what it held at some other time, and an anchor runs on
every path reaching it, so a loan dies only once every holder in the merged state
there is unread from then on, along the continuation including a loop's next
iteration. `VIR-SEMANTIC-LOAN-DEATH` checks this for each recorded death, from the
body's structure and on its own enumeration of each path's first access, and refuses
the unit otherwise. A Move `a;` on a `&mut` moves it into a discarded temporary that
no place holds, so nothing resolves it: the lender's value is unconstrained, which
leaves such a function unverified rather than unsound.

**The cursor.** `while let Some(r) = cur { cur = next(r) }` verifies unbounded
(`SourceVerify/cursor_errors.rs`, `enum_cursor_errors.rs`). A move of a component with an
overwrite following on every path, before any join, exit or use of an overlapping place,
prefixes included, is a transfer (`transferredMoves`): the moved reference becomes the
loan's only holder. Any other such move stays the reborrow it reads as. Four things carry
it. The runtime clears the component (`clearBorrowValue`), since a `.borrow` is found by
loan id, and the term's `takeAt` leaves `(π, π)` there, whose value nothing reads and whose
resolution assumes nothing. A write of a reference is shallow: it does not conflict with a
reborrow through it, and such a loan no longer goes through the place. A loan going on
through another holder gets a death for the dead place alone (`LoanDeath.alone`): the term
resolves the place and the run reconciles the reference it holds with its lender there
(`settleDeaths`), so a rebound local loses nothing; the loan itself ends at its own death.
And `final(x)` is usable in a loop invariant of any reference a place holds, so a cursor's
invariant states what the lender will hold, as `final(h.r) == final(old(h).r)`. A shared
view holds no prophecy, so `final` through one is rejected by name. A written place is a
new place, so writing one a live loan moved out of is no conflict:
`loanDeathLivenessDiagnostics` refuses a death naming a place still read. Linked lists need
`Box` too.

**Shared views of a holder.** Step 3e, stage 1: `&S` of a struct holding `&mut` is
what it observes, no loan among it.

**Drop.** A drop denotes nothing, and the analysis anchors the deaths of the loans
a dropped value holds at the drop.
`core::mem::drop` is that drop: `@[model(drop)] fun drop {T}` in `lib/core/mem.spec.lean`
makes the import rewrite a call to `tmp := arg; d := (); drop tmp`, the callee's parameter
dying at its scope end, so a destructor it reaches is rejected as at any scope end. A generic
callee dropping its parameter stays rejected at a type holding `&mut`.

**Signatures.** A parameter or result of a type holding `&mut` is lent, and the
loan list of an enum is consumed by the active variant. No new spelling: the entry
is `old(x)` and the exit bare `x`, compositionally, so `h.r` is the final referent,
`old(h).r` the entry, and `Option<&mut T>` presents as `Option<T>`.

**Regions.** Parameter and result types come from the declared signature with its
lifetimes, because a result must be tied to its source (`get_mut`, `first_mut`); a
local's stay erased, but a parameter's local takes the signature's type. A bound
region becomes a lifetime binder after the function's others, which a call states as
inferred. An elided lifetime that relates nothing stays erased, where the result has
no region or the signature no other one. The analysis keys incoming loans by declared
lifetime, and a by-value aggregate's reference component gets an incoming loan, which
removes the escape rejection of `take(Holder) -> &mut`. `FnDef::fn_sig` keeps a bound
region and erases an early-bound one, the only kind a `where` clause can mention, so
`'a: 'b` is not read (rust-lang/rust#161892) and such a lifetime relates to all. A
relation only an ADT's bound implies is rejected in a body
(`VIR-SEMANTIC-BORROW-RESULT-REGION`), and a dependency holding an ADT over several
lifetimes, whose body nothing checks, is exported inferred whole.

Declared outlives predicates now propagate through ADT arguments. In particular,
`Ordered<'a, 'b: 'a>` is covered by the positive
`SourceVerify/declared_lifetimes_bound.rs` fixture. Iterator result-region precision
still needs its own tests; the query gap is no longer the reason to defer it.

**`Copy` of a type parameter.** The mapper uses the declared bounds through
`polyfill::copy_parameters`, under the item's typing environment. Optimized MIR's
use of `copy` alone is not evidence of a `Copy` bound. Instantiations remain
subject to the shared ability checker.

**Generics at references.** A generic theorem does not transfer to `T := &mut U`:
the generic proof drops `T` as nothing, and dropping a `&mut` resolves its prophecy.
So an instantiation at a type holding one is verified monomorphically, and
`InvokeMeaning.Coherent`'s `refFree` stays the transfer's hypothesis.

`compileFunction_agrees` is one family over a substitution:
`compileFunctionAt unit handle σ = .ok f`, without one exactly `compileFunction`.
`CompileNamespace` carries the substitution, which `nty` applies; a callee's signature
is read unsubstituted (`declaredNty`). `Term.callInstance` carries the handle and `θ`
and means `Meanings.atInstance`, closed `propheticMeaningAt (some θ)`: the run at the
frame over the substituted rows, so lending follows the `.ref` type, with `Coherent` at
the family `θ` induces, since at the caller's a type parameter is not its argument. The
callee is inlined at the rows, with no contract of its generic declaration.

Rejected at the call: a callee dropping the parameter (`dropsDestructor` counts any
non-`refFree` instantiation, so `mem::drop` at a holder as well; a concrete `drop` of a
holder is not an instantiation and dies through its paths), and a native, opaque or
recursive callee, whose generic contract does not transfer. A shared reference over the
argument erases after the substitution (`ntyAt`), in the fields of an instantiated
declaration too. A reference
moved out of an aggregate is a reborrow through it, as validation reads it where the
type shows it; a holder moved out of one is rejected. A loop in such a callee meets no
invariant of its generic declaration, so it is not verified automatically.

The check runs substituted instances (`Tests/Differential/generic_references.rs`): the
interpreter runs the generic body at the concrete instantiation, the term inlines the
substituted compile, and a perturbed prophecy leaves no execution. It also evaluates
`coherentCheck` at the frame of each call at type arguments, a false one a disagreement. Closures (step 3d,
stage 4) also need lending through a function value's captures, since `NTy.lendable`
treats `.function` as `refFree`.

**`match`.** By value it takes the scrutinee, binds the payload to the arm's local
and transfers the holder in the certificate. By reference it is a reborrow at the
path `deref` then `variant`.

**Unsize.** A shared `&[T; N]` as `&[T]` is a borrow of the place `unsize p`. A
mutable borrow of an `unsize` or `subslice` place is rejected at validation
(`VIR-SEMANTIC-BORROW-UNSIZED-HOLE`): the runtime's hole cannot sit in a slice of a
vector, so the run has no derivation and a contract would hold vacuously; the design
for it is a loan over the whole lender with a length guard at write-back.

**A differential check.** `compileFunction_agrees` is an axiom, so a mistake here
has no proof to catch it. `verex-rust/VerexRust/Tests/DifferentialCheck.lean`, run
natively by `lake test` (the smoke suite thins its inputs with
`VEREX_DIFFERENTIAL=smoke`), exports `Tests/Differential/*.rs` and the exporter's
fixtures, runs the interpreter on the exported unit, and compares its outcome with
the term compiled from the rendering lowered back: boundary values per integer
type, every row of 8-bit integers for the operations and casts. A `&mut`
parameter's prophecy is its written-back value, and any other leaves no execution
unless the reference is returned. The term runs as a tree of its outcomes
(`Term.tree`, `VerexRust/Tests/DenoteEval.lean`), proved to mean the denotation
(`checked_tree_means`), with choices of prophecies and compiled calls inlined.
Generic calls use a specialized compile and transport arguments and results through the skolem
codecs. Invocation decodes captures and retains runtime type frames; generic
closure construction checks frame coherence before evaluating the target.
The evidence fixtures require every tested input to be conclusive. The theorem
relates the tree to this inlined compiled-body meaning, not to runtime dispatch
or compiler correctness. Globals, missing dependency bodies, and functions over
strings still have coverage gaps: their trees can be `unknown`, or they have no
inputs. Rust's `as` wraps and rustc spells no checked cast, so `Term.cast` is reached by an authored unit
(`VerexRust/Tests/Programs/DifferentialCasts.lean`). `run_sound` and `run_complete` relate the
interpreter to `BigStep`.

**Stages,** each gated on its own:

| Stage | Gate |
|---|---|
| Export: reference ADT arguments, `PtrMetadata` without a borrow, declared signature types | `RustToVerexSL` baselines |
| Tuples returned from a call | `Check/References/NestedHolders` |
| A struct holding one `&mut` by value | constructed, written through, read after it dies, returned |
| `Option<&mut T>` and user enums | `match` by value and by reference |
| Generics at references | `id::<&mut u32>` verifies with the lender seeing the write, a wrapper and `Option<&mut u32>` through generics, a dropping, native, opaque or recursive callee rejected (`generic_references_errors`) |
| Declared lifetimes in the analysis | `sel<'a, 'b>` writes the unrelated lender |
| `first_mut`, `get_mut`, a user iterator | a loop over a slice through them |

## Step 4. Crates

`verex-rust verify` accepts a Cargo package with the arguments
`import-crate` takes (manifest, package, features, target).

**A crate is one namespace; its modules are scopes.** The crate is what
rustc compiles and what a Cargo dependency names; crates form a DAG, while
the modules of one crate refer to each other in cycles (the root calls
`a::f`, `a` uses `super::T`). A VIR namespace is the unit of validation and
relocation ([`relocatable-namespaces.md`](../designs/relocatable-namespaces.md)),
so a crate stays one namespace, and a Rust module is a naming scope inside
it, as a Lean `namespace` is:

- A VIR name carries its scope, the module path relative to its namespace
  (`crate::a::b::f` is `f` in scope `a::b` of namespace `crate`); Move names
  have the empty scope. A namespace declares its scopes with their
  locations; the exporter writes them from the definition paths.
- VerexSL spells a scope as a reopenable item `mod a where …` inside
  `verex namespace`. A path resolves from the innermost scope outward to the
  namespace root; the rendering prints the shortest unambiguous path.
- The module file `src/a.rs` takes its specifications from
  `src/a.spec.lean`, spliced as a reopened `mod a where` block, so its items
  name the module's declarations bare. An inline `mod m { … }` has no file
  of its own: its specifications are a nested `mod m where` block in the
  enclosing file's specification file. Proof files name targets by path in
  the namespace (`verify a::f`), and the namespace counts verified once the
  proof file of each scope that has one ran.
- Callees across modules, mutual recursion included, are callees within one
  namespace, which verification already orders and handles.

A workspace dependency is a dependency namespace (step 3) whose functions
are verified with their own specification files or trusted with library
contracts.

Names carry their scopes, the exporter writes a crate's modules, VerexSL spells
scopes, and `verex-rust verify <Cargo.toml>` finds each module file's
specifications. Open: proof files per module.

Gate: the Cargo fixture verifies with specifications in two modules that
use each other and a call into its dependency crate.

## Step 3c. What real code needs first

Writing the benchmark's problems as a Rust programmer would (step 5) found these
ahead of everything else.

**Panics.** The exporter exports an unmappable constant or a callee with an
unsupported signature as an opaque node rather than failing. An import pass before
check folding collapses every diverging region, the blocks from which all paths
reach a diverging call or `unreachable`, into `abort`, which is `throw_ panic` and
an `abort j` site. That needs no list of panic functions and no purity condition: a
region's contents are unobservable under `panic=abort`, and a stateless failure
carries no message. A diverging callee that doesn't panic (`exit`, a loop) only adds
a spurious panic path, which is sound and incomplete, recorded in `assumptions.md`.
The import prunes an opaque declaration nothing then calls, as it drops unreferenced
opaque types, and validation rejects an opaque node or declaration that survives.

Macro diagnostics use the polyfilled source call site, preserving expansion
information and locating a panic at the macro invocation in the user's crate.

**Constants.** Export evaluates a constant (`try_const_eval`): integers, `bool` and
`char` to literals, structs and arrays through their bytes and layout; a constant at
`&T` exports as a shared borrow of the referent's value, since a struct referent has
no `ConstValue`, which covers `&str` and `&STATIC`. A constant states the shape only
of an ADT whose variants and fields are all visible.
A constant generic over a parameter (`T::MAX`) remains a precise export error.
Promoted constants are evaluated through the bridge, including constant references
and assertions that create promoteds.
`static mut` stays rejected.

**`?`.** A zero-sized constant of an ADT type is the constructor of its single layout
variant (`Option::<Infallible>::None`), and `Try::branch` and
`FromResidual::from_residual` are library contracts at `Option` and `Result` shapes.

**`cmp`.** A variant's discriminant may be negative in VerexSL, and `Ord::cmp` is a
library contract per integer type.

**Iterators.** A library declares the fields of an opaque iterator (`@[model] struct
Iter {a : lifetime} {T} where`, its fields deeper than it: `slice : &[a] Vector<T>`, `pos
: usize`, `end : usize`), which the import puts in place of the opaque declaration,
dropping the opaque and destructor properties, so `next` has a contract and an
invariant names progress: `loop 1 where invariant iter.pos <= iter.end && iter.end ==
iter.slice.length`. `RangeInclusive`, `Rev`, `Enumerate` and `StepBy` hold `core`'s own
fields, `slice::Iter` two positions in place of its pointers: the elements left are
`slice[pos .. end]` and `slice` never changes, as a `Range` keeps both its bounds. So
`iter_mut`'s shape can hold the whole lender and indices, where a shrinking subslice
would be the mutable borrow `VIR-SEMANTIC-BORROW-UNSIZED-HOLE` rejects. `Enumerate::next_back`
reads `len` as `end - pos`.

A generic instantiation is built over all binder positions, with a placeholder at a
lifetime's or constant's: `θ` holds `Unit` there, since no carrier reads it. A shape's
`arguments` hold it at a lifetime's, which the surface does not spell (`at {T} {A}
{Iter<lifetime _, T>}` of `{a : lifetime} {T} {A} {IntoIter}`). A call's frame reads its
real generic arguments, lifetimes included, as the runtime does, so `Term.callGeneric`
and `FrameOf` hold those, `FrameOf` stating the callee's binder kinds. A call passing
its caller's own parameters runs in the caller's frame only when all of them are
types. A function with a constant binder stops at export, as the pinned Rustc Public
does not expose its type.

Without trait evidence an adapter's `next` has a contract per inner iterator, a shape
each: `Rev` and `Enumerate` over `Range` and `RangeInclusive` per integer type and
over `slice::Iter`, with `Rev<Enumerate<Iter>>`, `Enumerate<Rev<Iter>>`, and `StepBy` over
an unsigned `Range` no wider than `usize`. A chain the library does not list has no
contract and is rejected at its call. A contract at a shape comes into a rendering
only for a call naming what the shape names, since each type it adds to a namespace's
table makes a frame certificate dearer; a function with calls no shape matches keeps
its shapes for the message listing them. `iter_mut` waits for step 3b.

Caller-aware resolution now runs in the caller's typing environment through
the bridge, so `K: Sized` is available when resolving a generic iterator call.
Parameter-dependent dispatch still requires a supported method dictionary.

A rule for reading a subvector (`(v[a .. b]).length == b - a`, `(v[a .. b])[k] == v[a +
k]` for `k < b - a`) belongs to the normal form, added when a fixture first reaches it.

## Step 3d. Closures

A closure type is a struct of its upvars with a `self`-taking body, and a call
through a known closure type is a direct call. Everything needed is public:
`RigidTy::Closure`, `ClosureDef::body`, `AggregateKind::Closure`, and upvar types in
the closure's generic arguments. MIR inlining is not a route: at `-O` it also inlines
`sort_by`'s `RawVec` transmutes.

**Identity.** Rustc's positional name, its path below the module: `struct
«f::{closure#0}»` in the enclosing module scope, parent generics first, then one
lifetime binder per region of the upvar types (a reference upvar has one); the
body is `call`, `call_mut` or `call_once` by the closure's kind, in
`impl «f::{closure#0}»`. A field is named by the body's debug info. A specification
file addresses it as any scope: `impl «f::{closure#0}» where spec call where`.
The backend prints the construction as the closure expression and neither the struct
nor the impl as items, so the rendering stays a fixed point:

```lean
let g :=
  closure «f::{closure#0}» {u0 : lifetime} has Copy (c : &[u0] u32 := &c)
    fun call(self : &«f::{closure#0}»<lifetime _>, a : u32) -> u32 := …
```

Lowering makes the struct (the property `struct.closure`) and the body items of the
module and leaves the construction, joining a contract authored in the scope to the
body. An interface, which holds no bodies, prints both as items.

**Construction.** The first construction in the body rustc nests the closure in (its
parent, `f` for `f::{closure#0}`, stated in the struct's property) spells the closure,
and a further one is `new`. A sized closure is constructed by an aggregate in its
parent, and one held but constructed by no reached body is rejected at export. A
zero-sized closure, one without upvars or over such closures only, has no construction
in MIR, since rustc removes the assignments of zero-sized values: it is the constant
`new «…» {}` at every use, wherever that is (`let c = make(); let d = c; d(1)` reads an
unassigned `d`), and its parent spells it at its first use there, or at its entry if it
has none. A closure no reached body names is dead code and is not exported.

**Calls.** Untupled at import: `Fn*::call*(s, t)` on a concrete closure type is the
direct call `call(s, t.0, …, t.n)` of the body's spread signature. The tuple is built
in the call's block and used only by the call, so the call reads its elements. At a
concrete type a call is always the closure's own kind, so no shim occurs there.

**Contracts.** The body is a function: inlined at a direct call without a
specification, a target of its own with one. Its `no panic` key and its `loop k` are
its own, and the enclosing function's loop numbers exclude it.

**Captures.** Upvars are fields, so `FnMut` is `&mut C` with `C { n : &mut u32 }`,
step 3b's holder. An `Fn` closure with a `&mut` in an upvar's type is a shared view
of a holder (step 3e). `copy` iff every upvar type is `Copy`, wherever its type arguments are;
`drop` always; a destructor iff an upvar's type has one, which the drop reads from the
struct's field types as for any struct with visible fields. A by-value call spells its
closure `copy g` where it is the last use, so a copy the unit cannot state to be one, of
a closure that is not `Copy` or that mentions a type parameter, is a move.

**A closure passed to a generic.** `Fn*` evidence must come from the declared
predicates now available through the bridge, including when a function merely
forwards its closure to another generic. Do not infer bounds from observed calls.
The evidence's method takes the receiver explicitly: `&F` for `Fn`, `&mut F` for
`FnMut`, and `F` for `FnOnce`. Its behavioral contract relates the old and new
receiver as well as arguments and result. A nullary function value would erase
captured state and is not a valid encoding. A closure carrying mutable references
must retain the existing substitution and loan checks. Library combinators get
contracts with the bound declared as data; `Map<I, F>` is a modeled struct like
step 3c's `Iter`. The `Fn` and `FnMut` evidence slices are implemented as described
above, along with consuming `FnOnce` bodies. Adapting borrowed receivers to
consuming evidence is supported for concrete closure bodies and generic
borrowed dictionaries carrying a checked receiver-noescape requirement. The
requirement covers both returned values and mutable writebacks, and follows
captured dictionaries. A generic result type alone cannot
justify releasing the receiver loan before dropping the owned closure;
`Check/Closures/ReceiverEscape.lean` pins this rejection boundary.

| Stage | Gate |
|---|---|
| 1 Closure items, untupled direct calls, printed closure expressions | fixed point and `verify` of a direct call, a once-called closure, a loop in a closure; the differential check covers them |
| 2 `&mut` and move upvars | `FnMut` over a counter and shared-reading `Fn` closures over borrowed or moved `&mut` holders verify; opaque generic transfer of loan-containing captures remains restricted |
| 3 Specifications on closures | `spec call` with a `loop 1` invariant verifies; `no panic` reported on the closure |
| 4 `Fn*` evidence for own generics | `FnMut` arguments, generic/struct/tuple results, forwarding and receiver adapters; declared `Fn` shared receivers and consuming `FnOnce` bodies; concrete consuming adapters; bounded opaque `R = ()` contracts; generic borrowed-to-consuming forwarding with checked receiver scope |
| 5 Library combinators | `Option::map` verifies; a `for` over `iter().map(..)` with an invariant verifies |
| 6 Function pointers as evidence, then `dyn Fn` | a function pointer verifies; `dyn Fn` rejected at its location until then |

## Step 3e. What ordinary Rust needs next

**Two-phase borrows.** `v.push(v.len())`. The exporter maps
`MutBorrowKind::TwoPhaseBorrow` mechanically to the profile borrow kind `rustTwoPhase`, and
a copy of a `&mut` into a temporary to `rustReferenceCopy`, since optimized MIR spells
`add(h.r, *h.r)` as `_7 = copy h.r` used once at the call. The runtime gives neither any
meaning, so a survivor is rejected. Before structurization (`Import/TwoPhase.lean`) each is
minted at its temporary's one use, when that is an operand and nothing between changes what
its place names or lends it: a written, moved or mutably borrowed place overlapping it (a
prefix of it or an extension, distinct fields being disjoint), or an index local of it.
Otherwise it is rejected at its location, `VIR-RUST-TWO-PHASE`, since a write of a deref's
base local between would change which place the borrow names. A reference copy that is not
an operand's one use is the plain reborrow it reads as, at the copy. Reads between see the
lender's old value, because the prophecy enters the slot only at activation, and the folded
unit is re-checked by the borrow analysis. It renders as `let _5 := len(&c); add(&mut c, _5)`.

**Mutable unsize of a whole array.** Deferred; `VIR-SEMANTIC-BORROW-UNSIZED-HOLE` stays,
which is sound. A hole at the array place is no obstacle, since `unsize` is the projection
`subslice 0 0 true` a borrow can strip. What blocks it is the length guard, "the write-back
has no derivation when the length differs", which the current runtime cannot state:

- Settling is total. `settleLoans`, `applyWriteBack` and `applyPendingWriteBack` always
  produce a state; a fill that fails falls through to `pending`, and
  `resolveReturnedBorrows` uses `.getD`.
- `fillHole?` cannot check a length: `.loanHole loan` holds only the loan, and a hole may
  be filled from another frame through `pending`. `HasType.hole` needs the referent's type
  and `HasType.vector` has `length_matches`, so a fill at an array slot needs the size.
- `&mut [T]` and `&mut Vec<T>` share the VIR type `&mut Vector<T>`, so a length change
  through the slice cannot be ruled out statically. Rustc rules it out, but authored VIR
  can write `push(&mut unsize a, 1)` or call a callee that assigns `*v`.

The options, each changing the semantics and its proofs (`Semantics` holds 65 `fillHole?`
and 191 `loanHole` mentions); the estimate for A is a few hundred lines of proof:

- A, the likely choice: `RuntimeState` gets `sizedLoans` and a `refused` flag. The loan's type
  is the array, so `HasType.hole` stays; a new rule types the borrow's current at the slice.
  Every fill site checks the size, and a mismatch leaves the hole and sets the flag. The
  final `run` rule and the interpreter require `refused = false`, beside `hole_free`, so a
  mismatched run has no complete derivation. It touches `Operations` (5 fill sites),
  `LoanTyping`, `LoanRenaming*`, `Preservation`, `BigStep` and `Interpreter`.
- B: `.loanHole` carries an optional length, or a new array-hole constructor. The fill
  checks locally, but every `loanHole` match changes and a failed fill still needs the flag.
- C: settling becomes partial (`settleLoans?`, with `BigStep` premises): no derivation at
  the step itself, the most faithful and the most expensive.
- D, rejected: dropping `length_matches` with no runtime guard. With the prophecy at the
  array type (`current = arrayVector π`) a length-changing path is vacuous while the run
  returns, so `compileFunction_agrees` would be false for authored VIR.

The denotation is the same under A, B and C. A mutable `subslice` stays rejected in all of
them, as does the `t @ ..` of `mutable_subslice_errors`. A loan of `&mut v[a..b]` is a later stage: the whole vector with the subrange as
current, spliced back at death under the same guard.

**Shared views of a holder.** A shared observation erases loans: `&T` denotes
`erase T`, with `erase (ref σ) = erase σ` and structural elsewhere, replacing the
`refFree` test in `ntyOfFuel`. The runtime's shared borrow yields `value.observe`, so a
callee holds no `borrow` and its frame exports nothing, and `HasType.shared` types the
observation at the erased referent; `erase` is the identity on reference-free types, so
Move is untouched. Every dereference under a shared dereference is shared. The
certificate does not change: a shared borrow mints no loan, and an observation is not a
holder. In a clause `it.s` is the current value; `final` of a nested reference through
a shared view is rejected by name.

The typing rule `HasType.observed` admits such a value at `&mut σ` when it is neither a
live borrow nor a hole. A retired borrow leaves `RuntimeValue.retired`, typed only at
`&mut σ` by `HasType.dead`, so a unit under a view is a real `()`, for `&mut ()` as for a
generic `T` at `()`. A shared dereference is stuck on a live value, which `retired` counts
as; a shared borrow is stuck on a value holding a hole. Validation excludes each: a view
of a holder is a use of it, so the analysis keeps the holder's loan live until then and a
write of the lender before it conflicts; a move out of the holder is a reborrow through
it, which a view conflicts with while the moved reference lives; and a mutable borrow or a
write under a view targets an immutable place. `retired` never leaves a function: a
result holding a loan keeps it live, so a write of the lender before the return conflicts.
A nominal holding a view at its own type arguments (`struct S<'a, T> { v: &'a T }` at
`T := &mut u8`) reads its fields at them, since `.subst` of a declared row cannot erase below
`.param`: `ntyOfFuel` carries the type arguments of its scope, a nominal's fields are read at
its own, and its native type is `S [&mut u8] [u8]` (`SourceVerify/generic_views.rs`). The
arguments are not what its values hold, so such a type is `refFree`, and a frame's reading of a
required type is at its parameters' types (`frameNty` in `Coherent`), not a substitution. A generic
contract transfers to `θ` exactly when `θ` is `refFree` and reading at `θ` agrees with substituting
it (`CompileNamespace.transfers`), over the callee's signature, locals, contract and body and,
transitively, what it calls; such a call is `callGeneric`, any other `callInstance`.
A shared reference holds no loan, so validation lets it move out of an aggregate. A clause
reads a view a field or a local holds whole, what it holds as current (`NTy.unerase`), as a
parameter's, and `*copy(p)` lowers to the place `*p`, which the erased deref reads.
Import folds a reference copy through a shared view, only dereferenced, into the place it
reads (`Import/TwoPhase.lean`), since a view holds nothing a reborrow could lend. A write
through the copy is not folded: the copy stays a reborrow, and validation rejects it.

| Stage | Gate |
|---|---|
| 1 Erased observations | `idx_len_field`; `fn len(&self)` on a `&mut Vec` holder; a `&mut self` method calling it; `let v = &r` with `r : &mut`; the differential check covers them |
| 2 Observations in aggregates and closures | `Option<&S>`, a struct holding `&S`, an `Fn` closure over a `&mut` upvar, `iter()` on a `&mut`-holding wrapper driving a loop |

**`Box` and recursive types.** Box is its content, as in Aeneas and Creusot:
`@[model(T)] opaque struct Box {T} {A}`, a model case aliasing the table entry to the
argument's type. A `Place::Deref` whose base is an identity-modeled nominal is its base,
at import, read from the place's type (`Library.applyModels`). `Box::new`, `as_ref` and
`as_mut` are library natives.

Optimized MIR has no such deref, since `ElaborateBoxDerefs` has run: `*b` reads
`p = copy (b.0.0) as *const T (Transmute)`, one pointer per use, behind a reference first
`t = copy *r`, and a box whose content moved out is freed by `f = &mut b; <Box<T> as
Drop>::drop(move f)`. The exporter undoes exactly that shape (`BoxFolds`), by the box's
language item: a single-assignment pointer used only by derefs, with the box's local
unchanged from its assignment to each use. Anything else stays an unsupported raw
pointer. It erases the free, which only releases the allocation, only for a box whose
content a folded pointer reads; a drop of the content stays a drop, so a destructor it
reaches is rejected whole or behind the flag of a move out (`box_destructor_errors`). The
pointer checks of a dereference are pinned off (`doc/status/assumptions.md`).

The exporter states a destructor where a `Drop` impl is direct and where an opaque ADT's
hidden fields reach one or hold a parameter, which import cannot see. What a visible
field reaches is read from its type after models apply, so `Box<Guard>` reaches `Guard`
and `Box<u32>` nothing. An opaque type hiding a box counts as having a destructor,
which is conservative.

A recursive nominal is carried by the family, since `NTy` has no binder and no recursive
type has a carrier in its universe: `NTy.nominal source arguments productive` for a declaration
whose rows reach itself, carried by `Carriers.nominals`, with a codec to
`RuntimeValue.nominal`, `fold`/`unfold` with their inverses, and `size` decreasing to
children. A runtime instance needs no generation; a generated inductive per recursive
group is the cost step, Aeneas's translation. `Proj` gets a step `unfold`, then today's
`variant`/`field`, and the normal form adds `unfold (fold v) = v` and injectivity. A
recursive spec function defaults to the family's `size` as its measure. A recursive type
holding `&mut` is rejected, since `referencePaths` would be infinite; `&mut` to a
recursive value is fine. Polymorphic recursion is a precise export error, by memoizing
`intern_instantiated_fields` on `(adt, arguments)`, which today hangs.

| Stage | Gate |
|---|---|
| 1 Box as its content | `Box::new`, `*b` read and move, write through `&mut *b`, drop; fixed point; the differential check covers them |
| 2 Recursive declarations | `List` constructed and matched by value and by reference; `len` verifies against a recursive `seq`; `destructors::list` and `::plain` leave the known list |
| 3 `&mut` into recursive values, cursors over `Option<Box<Node>>` | `push_back` with `ensures seq(l) == seq(old(l)) ++ [x]`; a tree insert |
| 4 Generated carriers per recursive group | same fixtures; heartbeats below the runtime instance |

Stages 2 and up are open. Until then a function whose local or result has a recursive
type has no denotation, and other functions of the unit are unaffected. Stage 2 is estimated, from counts of the
`.param` and `.function` arms and `Proj` sites and not from a build, at about 3000 new lines,
mostly proofs in the lowest modules: 45 arms over `NTy` in `Denote/Types.lean` plus about 15
new declarations; about 70 mentions across `NativeMaps`, `GlobalTyping`, `FunctionValues`,
`Resources`, `Invocation`, `Closures`, `Behavior`; new cases in `Compile`, `Term` and
`Solve/Normal`; 18 `Proj` sites and agreement; the clause translator and a default measure.
Each edit of `Types.lean` rebuilds nearly all of `verex-ir`.

`Proj.unfold` needs no unit: `Proj.unfold (body : NTy) (rest : Proj body σ)` reads and
writes through the codecs, `body.codec.decode? (nominal.encode v)` and back, both `Option`
already, so agreement follows from tightness, the compiler checks that `body` is the
declaration's rows at the arguments, and the family's `fold`/`unfold` laws serve only the
normal form.

Inhabitants. `productive : Bool` is computed by `ntyOf` from the unit, as a least fixpoint
over the recursive group with parameters assumed inhabited, and
`NTy.inhabitable (.nominal _ args p) := p && args.inhabitable`. `inhabitable` stays
family-free, so `TypeArgs`, `Carriers.instantiate` and their theorems stay as they are. The
runtime carrier is `{ v // NTy.admits unit τ v = true }`: `struct Inf(Box<Inf>)` has an empty
carrier, as in Rust, and `Option<Inf>` carries only `None`. `.enum` inhabitability reads only
the first variant (`NRows.firstInhabitable`), so `Option<List>` is inhabitable at any `List`,
and the same rule wrongly refuses `Result<Void, E>`.

`admits` is exact on every typed type, not on all types. Since `inhabitable` reads neither
family nor unit, `.nominal s a true` must have a value even where the flag is not the unit's
productivity, and `admits_encode` is universal. So a flag of `true` adds one point,
`witnessOrUnit unit s a`, beside the values of the rows: the witness when the rows have one,
else `.unit`. The invariant `productiveWitness? unit source args = some v → NTy.admits unit
(.nominal source args true) v = true` makes that point redundant where the flag is right, and
the `.nominal` constructor of `TypedAs` has the rule "flag = the unit's productivity at the
type's arguments" as its own premise, not only `ntyOf`. A substitution keeps the flag, and
keeps the rule because `TypeArgs` are inhabitable, which `struct W<T>(T)` at an empty `T` is not. This keeps a wrong-flag point away from every hypothesis over
typed types, `OwnersRepresentable` above all, which is assumed over every admitted value, so
that a point it could not represent would make every theorem of the unit vacuous.

| Step of stage 2 | Content |
|---|---|
| 1 | `NTy.nominal`, every exhaustive `NTy` arm and its proof; `nominalBody?`, `productiveWitness?` and `admits` in `Types.lean`; nominal fields of `Carriers` bundled as `TypedNominals`; the runtime instance `RuntimeNominal`. `ntyOf` produces no `.nominal`, so no baseline moves |
| 2 | `Denote/Nominal.lean`: the witness invariant; the rule above as a premise of `TypedAs`'s `.nominal` constructor |
| 3 | `ntyOf` produces `.nominal` with its flag; `Proj.unfold`, `Compile`, `Term`, `Solve.Normal`, `List` fixtures; `.nominal` stops being `notCarried`; `admits` exact on every type `ntyOf` gives, by that premise |
| 4 | Clause translator, `size` measure, `len` and `seq` |

A family's nominal fields are bundled in `Carriers`; a per-unit class waits for stage 4's
generated carriers.

Step 1 leaves these for step 2 or 3 to make sound before `ntyOf` produces `.nominal`. Step 2:
that `TypeArgs` being inhabitable keeps the rule under substitution, which needs inhabitable
to imply a witness; that `typeFuel` suffices for the witness search, or the flag is tied to the
same search, since a productive type whose search runs out gets the `.unit` point. Step 3:
`carriesInvariant` and `owns` read only the source and arguments, so they can miss an invariant
or a destructor in the rows; `heldFunctionTypes` returns nothing for a nominal; `refFree` reads
only the arguments, relying on rejecting a recursive type holding `&mut`; `NTy.equal` with
handles compares by `eraseGhosts`, so a map in the rows compares by entries; a closure capturing
a nominal is rejected (`capturedUnsupported?`). `fold`, `unfold` and `size` are still to add to
`TypedNominals`.

Not yet: `Box<dyn Trait>`; termination of recursive functions; `Rc` and `RefCell`
(`interior-mutability.md`). `HashMap` and `BTreeMap` as models over the intrinsic map
value are not designed.

**Library specification functions.** A `spec fun` is a fourth item kind of a library
file, kept when a kept contract or a user clause names it, spliced in its scope like a
contract, so a clause writes it by path (`u32::pow(x, n)`). It is a definition, so nothing
trusts it; only contracts stating with it are trusted.

**Strings.** `String` stays `@[model(string)]`: a `vector u8` model would make every
clause a byte computation. Stage 1 adds `stringConcat`, `stringBytes` and `charEncode`,
with `push_str`, `push` and `as_bytes` contracts in those terms.

Borrowed `core::str::from_utf8` now has a bounded logical UTF-8 validity
relation: a string witness has both the input's byte length and its exact
`stringBytes`. Its native contract characterizes success with that relation and
preserves successful bytes and length. The length equality excludes oversized
logical strings, whose `stringBytes` definition falls back to an empty vector.
Source checks prove decoding `str::as_bytes` succeeds and verify successful
bytes/length preservation; negative checks reject false success, failure, and
empty-output claims. Source verification checks library-spec retention; the rendering regression
checks the exported dependency signatures and source roundtrip.
This uses the existing trusted native-library-contract boundary, not a proof of
the standard library decoder. It adds no decoder primitive or axioms. Concrete
malformed-byte proofs, error offsets, owned `String::from_utf8`, and `chars` as a
step 3c iterator model remain stage 2. Strings still have no differential samples,
and the denotation evaluator does not yet execute string length.

**Which MIR is verified.** The exporter pins `-Copt-level=0` and `-Zub-checks=no`, so a
source has one MIR whatever the user's profile. The bridge reads promoted MIR;
there is no fallback that disables promotion and thereby changes the program
selected for verification.

**What stops real crates after per-function refusal.** First stop over 180
dependency-free registry crates: a local of `dyn` type 49, raw pointer 22, `impl Trait`
parameter 19, associated type alias 12, float 9, const parameter in a signature 8; 27
export, with 104 of their 383 functions refused. A local's type is read by no other
function, so a body stating a type the unit cannot is refused. A signature the unit cannot
state, by a type, a struct field it reaches, an associated type of a parameter or a const
parameter, makes the function opaque: it is reported, and every caller is refused whatever
its contract. An `impl Trait` parameter is a generic parameter.

The limit: such a function has no contract, not even an authored one, so a generic iterator
adapter and everything calling it stay unverified. Per-function binders for associated types,
as a dependency's signature has them, would not help yet, since these bodies are mostly
refused for lack of trait evidence. Revisit both when trait evidence is available
(rust-lang/rust#161892).

## Step 5. A Rust benchmark

Rust counterparts of Move benchmark problems (`math64`, `bit_vector`,
`simple_map` over `Vec`, `ordered_map`'s binary search) join `bounds` as
`rust` problems in `bench/problems.toml`, so coverage and cost are tracked
on the same algorithms in both languages.

`rust_math64`, `rust_bit_vector`, `rust_simple_map` and `rust_lower_bound` are in
`bench/problems.toml`. The following table records the initial obstacles found
when writing those benchmarks; it is historical, not the current support matrix.
Panic macros, ordinary constants, ordering, `?`, and direct closures now have
positive and negative source-verification fixtures. Closure-based iterator
combinators and general trait evidence remain open; generic associated constants
are also distinct from the ordinary constants now supported.

| Initially stopped | Construct | Original boundary |
|---|---|---|
| Most functions | `assert!`, `panic!`, `expect`, `debug_assert!`, `assert_eq!`, `static` | export: constant types |
| Most crates | `const` items, `u64::MAX` | export: `Unevaluated` |
| Most loops | `iter_mut`; `.iter()` in a generic body | step 3b; see "Iterators" |
| Comparators | `a.cmp(&b)`, negative discriminants | rendering does not re-import |
| Iterator chains | closures | export |
| Error handling | `?` | export: `Option<Infallible>` |
| Generic containers | `K: PartialEq` | general trait evidence |

`add`, `remove` and `borrow_mut` of `simple_map` need proof steps: a vector update
under a uniqueness invariant is not automatic.


## Order

Steps 1 and 2 complete M2's contracts-and-verify item for the slice without
traits. Step 3 moves M6's library models before M3–M5, because safe Rust
code needs `Option`, `Result` and `Vec` before it needs references inside
ADTs or drop effects. Step 4 completes M1.5 for verification. M3 (regions
over the prophetic model), M4 (drops and panics), trait evidence (M2, using the temporary query bridge) and the unsafe profile follow in the order of
`rust-mir-design.md`, each adding fixtures to the same Rust `SourceVerify`
gate.

## Dependencies on the verification work

Native verification is done, and so are the proof language's obligation
keys, path decisions and selectors (L0) and the loop points of L2, so
steps 1 and 2 wait on neither. They add to that work: the key of a strict
function's failing paths (`no panic`, `no abort`) in the proof language's
key table, and the normal-form rules of step 1, as a normal-form step. The library models and contracts of step 3 build on native
values and their roles. The structurizer, the printer, the exporter, loop
identities and the crate driver do not depend on that work.

## Open decisions

1. How a stale loop reference is detected. Move detects none: its
   invariants are written in the loop, and a proof file's `loop k` follows
   source order, so adding a loop before another shifts its number. Since
   invariants and facts at points are proved, a shifted reference fails
   rather than proves something wrong. A Rust specification file holds the
   invariants themselves, outside the source. Proposed: the reference may
   spell the loop's condition, `loop 1 while i < values.length where …`;
   lowering compares it with the condition of the `k`-th loop, up to
   locations and compiler temporaries, and reports a mismatch as stale with
   the loop's current condition. Without it, Rust behaves as Move's proof
   files do, plus the check that the invariant's locals are the loop's. A
   stored fingerprint would be a second artifact to keep in sync. Parked,
   with anchoring by loop labels, on the backburner of
   [`roadmap.md`](roadmap.md).
2. How Rust spells its failure clauses. Proposed: Move's `aborts_if`,
   which already permits a panic where its condition holds, so the contract
   language stays one. `rust-mir-design.md` (Specifications) and
   `vir-design.md` name a `panics_if`, a clause of the exceptional outcome
   `panic` as `aborts_if` is of `abort`. In a Rust function every failure is
   a panic, so the two permit the same paths; `panics_if` would be
   `aborts_if` restricted to the failure kind `panic`, as a code restricts
   it to an abort code. Whichever is chosen, the two designs and this plan
   say it alike.

## Decided

- Mutable references in aggregates are step 3b.
- A cast in a Rust clause stays rejected. Specification arithmetic never wraps
  (`verex-sl.md`), and a Move cast to `num` prints as the operand, so a wrapping
  cast in a clause would be the one exception; a clause writes `x % 256`.

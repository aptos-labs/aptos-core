# Closing the Rust verification gaps

Proposed 2026-10-28. Refines section 7 of [`roadmap.md`](roadmap.md) and
milestones M2, M1.5 and M6 of
[`rust-mir-design.md`](../designs/rust-mir-design.md). The unsafe profile
stays in [`unsafe-pointers.md`](../designs/unsafe-pointers.md).

## Problem

A Move file and a Rust file reach the same verifier: each becomes a
validated unit, is rendered as VerexSL, lowered back, compiled to a
denotation and verified
([`source-verification.md`](../designs/source-verification.md)). For Move
this path verifies the benchmark's standard-library and framework modules up
to `ordered_map`, runs the Move Prover's test corpus, and verifies the
`SourceVerify` fixtures. For Rust it verifies two fixtures:
`SourceVerify/bounds.rs`, also the benchmark's only Rust problem, with
branches, a modular call and a `&mut` parameter, and
`SourceVerify/references.rs`, with field updates through `&mut` and calls
passing a mutable reference on.

Where a Rust body reaches the verifier, it behaves as for Move: a missing
precondition on `a + b` fails as a possible panic, and a caller verifies
against its callee's contract. Small safe functions outside that shape stop
earlier:

| Stage | Behavior | Cause |
|---|---|---|
| Export | A call whose dependency signature names an associated type (`v[i]` on `Vec`) is rejected, as is a generic struct deriving `Clone`. | VIR has no projection type; a derived `Clone` of a generic struct calls `Clone::clone` on a type parameter (M2). |
| Validation | A loop whose exits continue to code they reach only through a cycle, or not at all in common, is rejected (`VIR-CFG-MULTI-EXIT`). | The structurizer needs one block after the loop that every continuing exit reaches without a cycle; early returns, `continue`/`break` of enclosing loops, and valued `break`s meeting after the loop are supported (`RustToVerexSL/loop_exits.rs`). |
| Denotation | `usize` and `isize` values, `as` casts, and the `overflowing_add` family have no denotation (`not carried by the denotation`). Slice indexing and enum discriminant reads use pointer-width integers and fail with them. | `Denote/Compile.lean` carries Move's checked primitives and integer widths. |
| Denotation | `char`, arrays, `!`, tuple element places (`values.0` as a place), comparisons of `bool` and `char`, and bitwise operations on `bool` and on signed integers have no denotation. | The same: `Denote/Compile.lean` and `ntyOfFuel` carry what Move has. |
| Denotation | A function that drops a value (a generic value, or one whose type has drop glue) has no denotation (`drop is not carried`). | What a destructor does is M4; the runtime removes the dropped value without running it. |
| Denotation | A struct with a lifetime parameter (`Borrowed<'a, T>`) has no denotation, and a `&mut` held in a tuple, struct or enum is rejected (`a loan held inside an aggregate`), as is a reference as a type argument. | `ntyOfFuel` reads only the type arguments of a nominal, and a loan's death resolves only a local that is the reference itself. |
| Specification | A loop cannot receive an invariant. | Loop invariants are in-body specification blocks; a Rust file has none, and its specification file holds items only. |
| Messages | A possible panic of a function without `aborts_if` is reported as a residual Lean goal keyed `leaf`. | The panicking path owes the implicit `aborts_if false` lowering appends to every Rust contract ([`verex-sl.md`](../designs/verex-sl.md)). That clause has no location, so `clauseNames` gives it no name. |
| Rendering | A unit with scopes (modules, methods, dependency crates) stops at the printer (`scopes are outside the current VerexSL printer`). | VerexSL has no `mod` item yet (step 3). |

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

Not in scope: trait-bounded generics (blocked upstream,
[rust-lang/rust#161892](https://github.com/rust-lang/rust/issues/161892)),
destructor effects and unwinding (M4), raw pointers (`unsafe-pointers.md`),
interior mutability (`interior-mutability.md`), and Rust closures, which
follow [`native-function-values.md`](native-function-values.md) ("Rust").

## Step 1. Imported bodies verify

In order: the remaining types and primitives, drops, the failing paths' key,
the normal form. Struct specifications are independent of the rest. Checks are
folded already, so most divisions, shifts and negations reach the denotation
as Move's checked operations, and the key's selectors name the sites folding
produces.

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
  then.
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
- The message names the operation and its `.rs` position, through the
  site's origin. Today `reportObligation` names a clause or a callee, never
  the failing path's site, so this is new: the failing path's decisions are
  at hand (`leafDecisions`), and the proof request prints the path's selector.

**Struct specifications.** A struct's data invariants and ghost fields are
clauses of its `spec S where` item in the specification file
([`ghost-fields.md`](../designs/ghost-fields.md)). Stored invariants and
static memory have no Rust counterpart: a Rust unit has no resources.

Gate: Rust `SourceVerify` fixtures, free functions without dependency types
since those stop at the printer until step 3, for arithmetic at every integer
width including `usize` and `isize`, `overflowing_add`, casts, division, structs and field updates
through `&mut`, a struct with a shared reference field, enums and `match`, arms computing values, functions
returning `&mut`, `()` functions with branches, a generic function dropping a
parameter, and a struct invariant. A drop at a type with a destructor is
rejected at its location, and an unspecified function on a call cycle is
reported, as in Move. The `RustToVerexSL` baselines show folded checks,
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
  `k`-th loop in source order, counted from 1 (`bodySites` in
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

**The `mod` item.** Every dependency function and every method lives in a
scope, and the printer rejects a namespace with scopes, so this comes first:
the `mod` item of step 4 in VerexSL (syntax, resolution, printing). It is
independent of steps 1 and 2 and may land before them.

**Dependency namespaces.** Each crate the unit reaches (`core`, `alloc`, a
Cargo dependency) is a namespace of the unit, as a Move package's dependency
modules are: it holds the declarations the crate reaches, in their modules'
scopes, and states `verify = false`. A dependency's type whose shape the
crate observes (constructs, matches, or projects) carries it, as `Option`
and `Result` do; any other is opaque (the Rust property `struct.opaque`), as
`Vec` is, and validation rejects constructing or destructuring it. Safe Rust
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

**A projection type.** A signature that names an associated type (`v[i]` on
`Vec` returns `<Vec<T, A> as Index<I>>::Output`) is rejected at the call. VIR
gets a projection type, normalized at concrete instantiations through the
impls' bindings; its match sites include the VerexSL lowering, printer, and
denotation.

Open, upstream:

- Trait bounds of functions and impls are not exposed by Rustc Public
  ([rust-lang/rust#161892](https://github.com/rust-lang/rust/issues/161892)),
  so no binder states its predicates, local or dependency.
- `Copy` of a type parameter: a body copying a `T: Copy` value needs the
  bound, which Rustc Public does not expose; the exporter maps such a copy to
  a move. The binder needs an explicit marker that its bounds are not
  exported, which VerexSL must spell. A struct's `Copy` is exported (its
  `Copy` impl applying it to its own parameters); a `Copy` impl for one
  instantiation (`impl Copy for S<u32>`) is not stated, and copying that
  instantiation fails semantic preparation.

**A standard-library specification library.** VerexSL files maintained in
`verex-rust` give contracts to the `core` and `alloc` functions the
fixtures and benchmark problems use: integer `checked_*`, `wrapping_*` and
`saturating_*`, `Vec::new`, `push`, `pop`, `len`, `get` and indexing,
`Option` and `Result` accessors. They declare model roles for opaque types:
`Vec<T>` is the VIR vector, as slices already are, its carrier
`SpecVector`; `String` is the VIR `string`, as `str` already is, its carrier
Lean's `String`. Roles are declared on the dependency
declaration in the library, and the verifier dispatches on the role, as for
Move's intrinsic maps ([`intrinsic-maps.md`](../designs/intrinsic-maps.md));
generic code never matches a Rust path. The exporter maps `wrapping_shl` and
`wrapping_shr` to the bare shifts, which have no outcome where Rust masks the
distance; this step gives them library contracts (`distance mod width`) and
removes that mapping, so they stay dependency calls. The other `wrapping_*`
mappings stand: `wrapping_add` is the total modular `add`. A call to a dependency function with
neither a contract nor a model is rejected at its location, as a Move native
without a specification or model is. Library contracts are trusted and each
theorem lists those it uses; verifying them against the generic MIR of their
bodies needs the unsafe profile for most of `alloc` and comes after its
library models ([`unsafe-pointers.md`](../designs/unsafe-pointers.md), §6).

Gate: fixtures with methods, with `Option` and `Result` in signatures and
matches, `checked_add` and `wrapping_add`, `Vec` push, pop, length and indexing under
loop invariants, and updates through `&mut [T]`; a call to an unmodeled
standard function is rejected with its location.

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

Names carry their scopes, the exporter writes a crate's modules, and
`verex-rust verify <Cargo.toml>` finds each module file's specifications. Open:
proof files per module; the `mod` item is step 3's first part. Until it lands
a unit with scopes (modules, methods, dependency crates) stops at the printer,
as `RustToVerexSL/method.rs` and `drop.rs` show.

Gate: the Cargo fixture verifies with specifications in two modules that
use each other and a call into its dependency crate.

## Step 5. A Rust benchmark

Rust counterparts of Move benchmark problems (`math64`, `bit_vector`,
`simple_map` over `Vec`, `ordered_map`'s binary search) join `bounds` as
`rust` problems in `bench/problems.toml`, so coverage and cost are tracked
on the same algorithms in both languages.

## Order

Steps 1 and 2 complete M2's contracts-and-verify item for the slice without
traits, on free functions. Step 3 opens with the `mod` item, which methods and
dependencies need, and moves M6's library models before M3–M5, because safe Rust
code needs `Option`, `Result` and `Vec` before it needs references inside
ADTs or drop effects. Step 4 completes M1.5 for verification. M3 (regions
over the prophetic model), M4 (drops and panics), trait evidence (M2, after
the upstream query) and the unsafe profile follow in the order of
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
3. Mutable references in aggregates. `rust-mir-design.md` says reference-bearing values need
   nothing special, because borrows nest as values nest, and schedules them in
   M6. That holds of the runtime; the
   denotation rejects them, and `Option<&mut T>`, `get_mut` and `iter_mut` need
   them. The borrow analysis is the other half: Move structs have no reference
   fields, so only Rust fixtures exercise its nominal paths, it does not model
   nominal variance, and rustc's check is admission evidence only, so the
   analysis can reject what rustc accepts. Proposed: a step after step 3, opening
   with a measurement of which fixtures rustc accepts and the analysis rejects;
   the denotation of a loan held in an aggregate follows from what that shows.

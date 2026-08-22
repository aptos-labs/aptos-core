# Move: Lean-authored Move contracts

This is a working prototype for authoring Move modules in Lean. `Move` is the
source programming model and compiler namespace, and the Lake package rooted
at this directory; the [`MoveModel` package](../../move-model/MoveModel/README.md) it depends
on holds the target IR, operational semantics, analyses, and prover. The
compiler integration is still called Leaner.

The same source has two distinct uses:

1. **Source verification.** A `spec` is associated declaratively with a Move
   function and `verify f` produces a theorem checked by the Lean kernel,
   directly over the authored function's generated source semantics.
2. **Executable compilation.** Selected declarations are lowered through
   typed base LCNF, `Move.Compiler.LIR`, and `MoveModel.IR` into versioned
   XIR, then compiled by the complete compiler-v2 pipeline and checked by the
   production Move bytecode verifier.

A compiler-correctness theorem connecting a source-level `f.verified` theorem
to the emitted bytecode remains future work; the prototype does not conflate
those claims.

This README is an example-based tour. The language itself is defined in
[`leaner-move.md`](leaner-move.md); the verification design is in
[`verification-design.md`](verification-design.md); compiler and proof
roadmaps are in [`design-plan.md`](design-plan.md) with a short overview in
[`overview.md`](overview.md), loop lowering in
[`loop-design.md`](loop-design.md), data invariants in
[`invariant-design.md`](invariant-design.md), and the cost analysis of the
verification encoding in [`performance-analysis.md`](performance-analysis.md).

## A complete module

```lean
import Move

open scoped Move

module Account where

  struct BalanceValue has Copy, Drop, Store where
    value : U64

  struct Balance has Key where
    balance : BalanceValue

  entry fun deposit (addr : Address) (amount : U64) : Action Unit := do
    let value ← &mut Balance[addr].balance.value
    value := *value + amount

  entry fun withdraw (addr : Address) (amount : U64) : Action Unit := do
    let value ← &mut Balance[addr].balance.value
    let old ← *value
    if old < amount then
      abort 1
    value := *value - amount
```

`module` creates the namespace, opens the Move API, and registers the
Move module; it compiles at end of input and performs no filesystem writes
during an ordinary `lake build`. A resource is a `struct` declaring `has Key`;
`&mut Balance[addr].balance.value` chains a global borrow with two checked
field borrows; `value := *value + amount` reads and writes through the
reference in order.

The sugar always has a public core API underneath, usable directly:

```lean
let balance ← borrowGlobalMut Balance addr
let value ← borrowFieldMut balance (fieldOfProjection Balance.value)
let old ← read value
write value (old + amount)
```

## Pure functions and contracts

Declarative contracts use the `Move.Spec` scope. Within `ensures`, `result`
is implicitly bound; `verify` creates the Lean-only `choose.contract : Prop`
and `choose.verified : choose.contract`:

```lean
open scoped Move Move.Spec

module Choices where
  enum Choice where
    | Fallback
    | Chosen (value : U64)

  fun choose (fallback : U64) (choice : Choice) : U64 :=
    match choice with
    | .Fallback => fallback
    | .Chosen value => value

  spec choose (fallback : U64) (choice : Choice) where
    ensures
      result = match choice with
        | .Fallback => fallback
        | .Chosen value => value

  verify choose
```

## Effectful contracts

For an `Action` function, the same declaration style generates a relational
semantics from the retained `fun` body; `initial`, `final`, `result`, and
`abortCode` are implicit clause binders, `old(...)` observes the pre-state,
and repeatable `aborts_if P with C` clauses bound every admitted abort. The
postcondition needs to hold only where the declared aborts are ruled out;
this is built into the semantics of contract satisfaction, and the proof
tactics surface each abort condition's negation as a hypothesis of
`ensures`. Omitting the abort clauses entirely leaves abort behavior
uninterpreted — any code may then be raised, and the postcondition still has
to hold for every successful execution:

```lean
spec deposit (addr : Address) (amount : U64) where
  requires existsAt<Balance>(addr);
  modifies Balance[addr];
  ensures
    Balance[addr].balance.value =
      old(Balance[addr].balance.value) + amount;
  aborts_if
    ¬old(Balance[addr].balance.value).toNat + amount.toNat < U64.size
    with Semantics.Checked.arithmeticAbortCode

verify deposit
```

For a mutable-reference parameter, the contract talks about values rather
than reference identities: the parameter name denotes its final referent in
`ensures` and `old(parameter)` its initial referent. The prophecy-backed loan
implementing this stays internal to the proof:

```lean
spec add (map : &mut Map K V) (key : K) (value : V) where
  requires Model.WellFormed map;
  ensures map.entries.toList = Model.add (old(map)) key value ∧
    Model.Sorted map;
  aborts_if Model.Contains map key with 1;
  aborts_if U64.size ≤ map.entries.toList.length + 1
    with Move.Semantics.Vector.indexOutOfBounds
```

A function changes only the global memory its `modifies` clause lists, so
contracts never state a frame condition: `deposit` above leaves every other
address of `Balance` — and every other resource — untouched, and a spec with
no `modifies` clause changes no global memory at all.

Global state stays abstract and compositional: each generated theorem
quantifies over one typed `ResourceStore` per resource family the function
uses, so adding a resource in another module never forces a shared `World`
record.

## Data invariants

A `spec` on a type constrains every value of it, and the value carries the
proof. Operations therefore need no precondition, and the invariant is owed
only where a value is created:

```lean
struct Percent where
  value : U64

spec Percent where
  invariant .value.toNat ≤ 100
```

Clauses read like the other spec blocks and may be repeated, conjoined:

```lean
spec Range where
  invariant .low.toNat ≤ .high.toNat;
  invariant .high.toNat ≤ 1000
```

`this` denotes the value and `.field` abbreviates `this.field`. An enum
invariant matches on `this`, and each constructor then carries the proof of
its own variant, so patterns bind it with a trailing `_`:

```lean
enum Payment where
  | None
  | Direct (amount : U64)

spec Payment where
  invariant match this with
    | .None => True
    | .Direct amount => 0 < amount.toNat

fun first_part (payment : Payment) : U64 :=
  match payment with
  | .None _ => 0
  | .Direct amount _ => amount
```

A literal like `{ value := 50 }` or `.Direct 5` discharges its obligation
during elaboration, so source carries no proof text and a violation is
reported at the literal; creating a value under a condition uses the
dependent `if h : 0 < amount then .Direct amount else .None`, whose branch
hypothesis discharges it. Mutation is unconstrained while a borrow is live:
the obligation lands where the value is rebuilt, when the loan dies — for a
local value and for a resource behind `&mut R[addr].field` alike. The proof is
erased before Move sees the type.

## Integers

All six Move widths are one generic certified type (`{ x : Int // 0 ≤ x ∧
x < 2^bits }` indexed by a width name), so `U8` ... `U256` share every
operation and lemma. Arithmetic aborts on overflow, shifts take a `U8`
amount and abort at the bit width, `&&&`/`|||`/`^^^` never abort, and
`(x.cast : T)` is Move's `as` with its checked range:

```lean
fun narrow (value : U64) : U8 :=
  (value.cast : U8)

spec narrow (value : U64) where
  ensures result.toNat = value.toNat;
  aborts_if ¬value.toNat < U8.size
    with Semantics.Checked.arithmeticAbortCode

fun shifted (value : U64) (amount : U8) : U64 :=
  value <<< amount
```

## Vectors

```lean
fun replace : Action U64 := do
  let values : Vector U64 := vector![10, 20, 30]
  let middle ← &mut values[1]
  middle := 42
  (*middle)
```

Literals, `push`, `length`, element borrows, and mutation through
`&mut values` (`r.insert i e`, `r.remove i`) all lower to native vector
operations; indexed access aborts on out-of-bounds, and growing past the
`u64` length domain aborts like the runtime. The logical vector certifies
that domain by construction, so `length` is exact in specifications and
cursor arithmetic over indices provably cannot overflow.

## Enums

```lean
enum Op where
  | Idle
  | Transfer (amount : U64)
  | Split (left right : U64)

fun total (op : Op) : U64 :=
  match op with
  | .Idle => 0
  | .Transfer amount => amount
  | .Split left right => left + right
```

## Loops

`while`, `loop`, `break`, and `continue` compile to in-function CFG loops,
with labels for targeting an outer loop:

```lean
fun count_down (n : U64) : U64 := do
  let mut n := n
  while 0 < n do
    n := n - 1
  n

fun labeled_exit (n : U64) : U64 := do
  let mut n := n
  loop@outer
    loop
      if n < 1 then break@outer
      n := n - 1
      break
  n
```

## Recursion

Recursive functions are `partial`; general direct and mutual recursion
compile as calls. `continue f args...` marks one direct tail self-call for
checked, stack-safe lowering to a loop — it is an error anywhere else, and
unmarked recursive calls keep call semantics:

```lean
partial fun countdown (remaining accumulator : U64) : U64 :=
  if remaining < 1 then accumulator
  else continue countdown (remaining - 1) (accumulator + 1)
```

Recursive source semantics is the least finite-unfolding relation, so
`verify` proofs are partial correctness with `Move.Verify.satisfies_fix` as
the induction rule. `Tests/Verification/Quicksort.lean` verifies a generic in-place
quicksort against the semantics derived from its authored body.

## Generics

Generic functions, structs, enums, and resources compile as true generics —
no monomorphization. Instantiation is inferred or written with named type
arguments:

```lean
struct Vault (T) has Key where
  value : T

fun publish_generic {T} (signer : &Signer) (value : T) : Action Unit :=
  moveTo signer ({ value } : Vault T)

fun has_generic {T} (address : Address) : Action Bool :=
  existsAt (Vault T) address

fun has_vault (address : Address) : Action Bool :=
  has_generic (T := U64) address
```

## Visibility

A plain `fun` is private. `public fun` and `friend fun` declare `public` and
`public(friend)` visibility, and `entry fun` a public entry function:

```lean
public fun contains {K V} (map : &Map K V) (key : &K) : Action Bool := ...

friend fun add_to (addr : Address) (amount : U64) : Action Unit := ...
```

## Attributes

An attribute list may precede the `struct`, `enum`, or `fun` keyword. An
attribute is a name applied to positional arguments — name paths, constants,
or parenthesized instantiated types — and is recorded as metadata on the
compiled module:

```lean
@[resource_group (scope global)]
struct Registry has Key where
  value : U64

@[randomness 7, lint.skip]
entry fun act (addr : Address) : Action Unit := ...
```

## Cross-module calls

Importing another Lean-authored module is an ordinary Lean `import`; its
public functions, specs, and theorems are all available:

```lean
import Move.Tests.Compiler.Fixtures.Modules.Math

module Client where
  fun imported_identity (value : U64) : U64 :=
    Math.identity (Math.identity value)
```

The minimal example is
[`Tests/Compiler/MultipleModules.lean`](Tests/Compiler/MultipleModules.lean) with
its dependency
[`Tests/Compiler/Fixtures/Modules/Math.lean`](Tests/Compiler/Fixtures/Modules/Math.lean).

## Compiling `.lean` sources

Move compiler v2 launches Lean with a private output path, reads the
versioned XIR, and runs its normal checking, optimization, file-format, and
bytecode-verification pipeline. Direct source lists and Move package source
discovery both recognize `.lean` beside `.move`. Lean is launched from the
Lake package root (`third_party/move/lean`); set `LEANER_ROOT` to point
elsewhere for a different checkout layout.

For tests and transformations the compiled module is available as a Lean
value:

```lean
def compiled : MoveModel.IR.Module := lowerToIR ``Tests.MovePrograms.Account

#test run "deposit" (memory 7 10) [.address 7, .u64 5]
  = Tests.okRet (memory 7 15) []
```

`lowerToIR` directly embeds semantic IR and derives its address and canonical
module name from the registered Move namespace. `module% "M" structs [...]
functions [...]` remains the low-level escape hatch for an explicit selection;
the old implicit form is available as `module_from_context% "M"`. XIR is
materialized only by an explicit export command.

**Trusted build inputs.** Compiling a `.lean` source runs Lean elaboration,
including its macros and metaprograms. Treat direct sources and package
dependencies containing `.lean` files as trusted build inputs, just as you
would a build script; compiler-v2 does not sandbox Lean elaboration.

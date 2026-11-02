# VerexSL, the source language

VerexSL is the source language of Verex. It is one language with explicitly
selected *semantic profiles* — today Move and Rust — hosted in Lean: a source
file is an ordinary Lean file, Verex declarations are Lean command syntax, and
elaboration produces a validated unit of the shared Verex IR. Everything
downstream — execution, verification, backends — works on that unit, not on
the Lean text. There is no fallback that evaluates an unrecognized Lean term
as Verex: what does not lower faithfully is a located error.

Ordinary Lean stays available around Verex declarations, for metaprogramming,
proofs, and compile-time computation. Sources are therefore treated like
trusted build scripts.

The language and its contracts are specified in
[`../designs/verex-sl.md`](../designs/verex-sl.md); this chapter is a working
description of the Move profile. Examples are taken from the verifier's own
corpus: the framework's map modules and the end-to-end fixtures under
`verex-e2e-tests/VerexE2ETests/SourceVerify`.

## A module

A Move-profile module is a namespace whose header names an address and a
module:

```lean
verex module 0x42::counter where
  struct Counter has Copy, Drop, Store where
    value : u64

  fun increment(c : &mut Counter) -> Unit := c.value := c.value + 1

  spec increment where
    ensures c.value == old(c).value + 1
    aborts_if c.value + 1 > MAX_U64
```

Most users never write this file by hand. Verifying a Move package
(`move prove --lean --package-dir <package>`, or `aptos move prove --lean`)
renders each Move source to exactly this form and verifies the rendering:
a package's modules go to `build/verex-verify.lean` under the package, a
single Move file of a package to `build/verex/<path>.lean`, and a Move file
of no package to `.verex/<file>.move.lean` beside it. A rendering records
where it came from (`verex sources at "<root>" "<file>" <hash>`) and, when
a proof file exists, `pragma proof_file = "<path>"`. Authored Lean sources,
rendered Move sources, and imported bytecode meet in the same validated IR,
so the round trip is by construction, not by convention.

What follows is the rendered dialect: Move's spellings carry over where
they fit Lean's syntax (`u64`, `&mut`, `x.field`, `v[i]`, `assert!`), and
where they do not — quantifiers, loop invariants, blocks — VerexSL has its
own spelling, given below beside the Move one.

## Declarations

A module holds constants, structs and enums, functions, and the
specification items of the sections below. Declarations are predeclared
before bodies elaborate, so mutual recursion needs no `mutual` block.

```lean
const E_OVER_CAP : u64 := 1
const ADDRESSES : Vector<Address> := #[@0x1, @0x2]

struct Vault has Key where
  balance : u64
  cap : u64

enum Tagged {T has Copy, Drop} has Copy, Drop where
  | Empty
  | Filled (value : T, tag : u64)

public fun make {T has Copy, Drop}(value : T, tag : u64) -> Tagged<T> :=
  new Tagged<T>::Filled { value, tag }
```

Abilities are `has Copy, Drop, Store, Key`; type parameters are written
`{T has …}` before the parameter list and `<T>` at a use. Visibility is
`private` (default), `public`, `package`, or `friend`; `entry`, `native`,
and `view` are modifiers, and `opaque fun` supplies a body as a summary
only. A function always states its result type, `-> Unit` included, and
its body follows `:=` — an expression, or a `do` block. Module-level
`pragma` items and `use`/`friend` items stand among the declarations.

## Expressions and control

The expression language is Move 2's: `&T` and `&mut T` parameters, `x.f`
and `v[i]` projections that insert the needed borrow, `*r` for an explicit
read, assignment `p := e` to any place (`c.value := …`, `*x := …`,
`Counter[addr].value := …`). Calls are `f(x)` or `x.f()`; `new T { f := v,
g }` packs a struct or variant; `e is V` tests a variant; `x as u64` casts;
`#[a, b]` is a vector literal and `v.length` its length; `abort(code)` and
`assert!(cond, code)` abort; a function value is `function[Fn(u64) ->
u64](g)` and is called with `invoke(f, x)`.

A `do` block holds statements, one per line; its last entry is its value.
That convention decides how `return` reads: a trailing `return e` supplies
the block's value, `return e;` with the semicolon leaves the function from
anywhere.

```lean
fun drain(m : &mut OrderedMap<u64, u64>) -> u64 := do
  let count := 0
  while !m.is_empty() do
    let (_k, _v) := m.pop_front()
    count := count + 1
  where
    invariant count + spec_len(m) == spec_len(old(m))
  count
```

Control is structured and every form is an expression:

- `if c then e₁ else e₂`; an `if` without `else` has type `Unit`.
- `match e with | pat => e | pat if guard => e`, with variant patterns
  `Tagged<T>::Filled { value := _, tag := tag }`.
- `while c do … where <loop specification>`: the `where` region is the
  loop's own and holds `invariant` clauses and the logical `let`s they use
  (below).
- `for i in a..b do …`, the half-open range loop, each bound evaluated once.
- `loop[@label] e`: an unconditional loop; it leaves by `break[@label]
  [value]`, so a loop may produce a value, and `continue[@label]` restarts
  it. Labels belong to `loop`, `break`, and `continue` only.
- `return e;`, `abort(code)`, `panic`; abrupt control is polymorphic, so a
  branch that aborts needs no value.

```lean
fun early_return(n : u8) -> u8 := do
  let i := 0u8
  while i < n do
    if i == 1u8 then return i;
    i := i + 1u8
  i

fun count_loop(n : u64) -> u64 := do
  let i := 0
  loop do
    spec invariant i <= n
    if i >= n then break
    i := i + 1
  i
```

Checked Move arithmetic aborts on overflow with the arithmetic error, as do
a cast out of range, an index out of bounds, and a missing resource;
specifications see these runtime failures as `EXECUTION_FAILURE` (`-1`).

## Types

The core types are `Bool`, `Address`, `Signer`, the fixed-width integers
(`u8` … `u256`, `i8` … `i256`; generically `UInt<w>`, `SInt<w>`), tuples,
`Unit`, vectors `Vector<T>`, nominal types `path::Name<T>`, references `&T`
and `&mut T`, and function types `Fn(T) -> U`; `Char`, `string`, `Bytes`,
and the pointer-width `UPtr`/`IPtr` serve the Rust profile. Specifications
additionally use the unbounded `Int` and `Nat`, and quantifier domains (a
type, a range `a .. b`, a vector).

## Specifications

Specifications share the executable expression language; logical-only
constructs extend it. Everything in a specification evaluates over
mathematical integers: bounded values widen to `Int` automatically when they
enter a spec expression, and spec arithmetic never wraps or aborts. Bounds
`MAX_U64`, `MIN_I64`, and so on are named; a local or module constant of the
same name takes precedence.

### Contracts

A function's contract is a `spec f where` block of clauses (`spec f { … }`
is the same block braced). From the `vault` fixture:

```lean
public fun deposit(addr : Address, amount : u64) -> Unit := do
  let vault := &mut Vault[addr]
  assert!(vault.balance + amount <= vault.cap, E_OVER_CAP)
  vault.balance := vault.balance + amount

spec deposit where
  aborts_if !exists<Vault>(addr)
  aborts_if global<Vault>(addr).balance + amount > MAX_U64
  aborts_if global<Vault>(addr).balance + amount
      > global<Vault>(addr).cap with E_OVER_CAP
  ensures global<Vault>(addr).balance
      == old(global<Vault>(addr).balance) + amount
  ensures global<Vault>(addr).cap == old(global<Vault>(addr).cap)
  modifies global<Vault>(addr), *
```

The clauses:

- `requires e` — owed by callers, assumed in the body.
- `ensures e` — of the post-state, on every returning path.
- `aborts_if e [with code]` — the function aborts exactly when some
  `aborts_if` holds (the disjunction is complete), with the stated code
  where one is given; `aborts_with c₁, c₂` lists the codes an abort may
  carry. `pragma aborts_if_is_partial` makes the conditions sufficient
  only. A spec with no abort clause leaves aborting unconstrained — the
  `ensures` are owed on the returning paths only — and `aborts_if false`
  states that the function never aborts.
- `let_pre x := e` and `let_post x := e` — names for a pre-state and a
  post-state value, usable in the clauses after them.
- `modifies global<T>(a)` — the frame: what the function may change. A
  plain list is closed; `modifies global<T>(a), *` closes the listed
  families at their keys and leaves the rest open; `modifies *` opens
  every family. `reads T` / `reads *` states what is read. A function-typed
  parameter keeps memory unless `modifies_of<f>(a : Address)
  global<Counter>(a)` or `modifies_of<f> *` gives it a footprint.
- `pragma opaque` — callers see the contract only, never the body;
  `pragma unroll = N` bounds a loop's iterations instead of an invariant;
  `pragma verify`, `verex_verify`, `heartbeats` are under
  [Verification](#verification).

In a clause, `old(e)` reads `e` at entry, `result` is the returned value,
and a `&mut` parameter read plainly is its final value, so `increased(old(c),
c)` compares the value at entry with the value at exit. For a returned
mutable reference, `final(result)` names the value its lender holds once the
caller's writes through it end. A specification function takes values, so
a `&mut` argument passes its value, before or after as spelled:

```lean
spec fun increased(old_c : Counter, c : Counter) : Bool := old_c.value < c.value

spec increment where
  ensures increased(old(c), c)
```

A function-typed parameter is specified by its behavioral predicates,
`requires_of<f>(x)`, `aborts_of<f>(x)`, and `ensures_of<f>(x, result)`,
which stand for the contract of whatever value is passed:

```lean
fun apply(f : Fn(u64) -> u64, x : u64) -> u64 := invoke(f, x)

spec apply where
  pragma opaque
  aborts_if aborts_of<f>(x)
  ensures ensures_of<f>(x, result)
```

### Specification functions

An executable `fun` that does not write global storage is also a
specification function: called from a spec position it denotes its
successful mathematical value, over `Int`, reading storage in the caller's
state (or the pre-state under `old`). Its aborts impose nothing: an aborting
path has an unspecified value, and the caller's contract states the
executable success conditions. A `spec fun` states a model body explicitly;
`opaque spec fun` declares one without a body, constrained by axioms; a
recursive `spec fun` names its `decreases` measure, and its unfolding is a
lemma a proof may apply.

```lean
spec fun sum(n : Int) : Int decreases n := if n <= 0 then 0 else n + sum(n - 1)

opaque spec fun spec_swap(x : u16) : Int

spec module where
  axiom ∀ (a : u16; b : u16), a != b ==> spec_swap(a) != spec_swap(b)
```

### Quantifiers

VerexSL's quantifiers are `∀`/`forall` and `∃`/`exists` over
parenthesized binders separated by `;`, each `x : T` over a type or `x in
d` over a range or a vector, then `,` and the body. Move's `forall i in
0..len(v): v[i] > 0` renders as:

```lean
∀ (i in 0 .. v.length), v[i] > 0
∀ (i : u64), i <= x ==> i < result
∃ (y : T; k : u64), y == _x && 2 * k == n
∀ (row in v), ∀ (j in 0 .. row.length), row[j] == row[j]
```

Implication is `==>`, and `<==>` is equivalence. The proof language of
chapter 2 accepts this dialect only.

### In-body specifications

Inside a body, `spec assert e` owes `e` where it stands and assumes it
afterwards, `spec assume e` assumes it unchecked (and enters the assumption
ledger), and `spec apply L(a)` instantiates a lemma. `spec do` opens a block
of such statements, and a whole body may be one (`fun correct() -> Unit :=
spec do …`). Move's `spec { assert …; }` block renders to these.

Loop invariants belong to the loop. A `while` loop states them in its
`where` region, after the body; a `loop` states them as `spec invariant`
statements at the head of its body. Move's header block `while ({ spec {
invariant … }; c }) { … }` renders to the `where` form. Each invariant is
owed at entry and at the start of every iteration, and assumed in the body
and after the loop:

```lean
fun fees(periods : u64) -> u64 := do
  let i := 0
  let total := 0
  while i < periods do
    total := total + 5
    i := i + 1
  where
    invariant i <= periods
    invariant total == 5 * i
  total
```

The `where` region may bind logical names with `let` for the invariants
after it. Without an invariant, `pragma unroll = N` in the function's spec
block verifies the loop for at most `N` iterations.

### Lemmas

A `spec lemma` states that its `requires` clauses imply its `ensures`
clauses, with an optional `proof` of steps in the proof language
([chapter 2](proof-language.md)); a recursive lemma names its `decreases`
measure and is proved by induction through its own `apply`:

```lean
spec lemma sum_bound(n : Int) decreases n where
  requires 0 <= n
  ensures sum(n) >= n
  proof
    split n > 0
    apply n > 0 ==> sum_bound(n - 1)
```

Lemmas are applied by `spec apply` in a body or `apply` in a proof; the
premise is owed at the application.

### Module and data invariants

Module-level properties live in one `spec module where` block: an
`invariant` is assumed at entry and owed at exit of every function of the
module, an `axiom` is assumed outright (and lands in the assumption ledger).
A generic `invariant {T} …` or `axiom {T} …` holds at every instantiation
the verified function reaches:

```lean
spec module where
  invariant {T} ∀ (a : Address), exists<Box<T>>(a) ==> exists<Marker>(a)
```

A data invariant attaches to a struct or enum as `spec S where invariant
…`, with `this` the value; it is owed wherever a value is built and assumed
wherever one is read:

```lean
spec Tagged where
  invariant match this with
    | Tagged<T>::Empty {} => true
    | Tagged<T>::Filled { value := _, tag := tag } => tag > 0
```

A `ghost f : T := e` clause in such a block adds a model-only field to every
value, set at each pack. A Move `spec` variable `x` arrives as the ghost
resource `Ghost$x`, with `update x = e` a write of it.

## Intrinsics and their models

Storage-backed containers (Aptos `Table`, `OrderedMap`, `BigOrderedMap`, …)
are *intrinsic maps*: the owner's spec block marks it `pragma intrinsic =
map` and binds the map theory's roles to functions of the module. The
executable roles name the operations (`map_new = new`, `map_len = length`,
`map_borrow = borrow`, `map_add_no_override = add`, …); the specification
roles name the model's functions — `map_spec_get = spec_get`, likewise
`map_spec_set`, `map_spec_del`, `map_spec_len`, `map_spec_has_key`,
`map_spec_key_at`, `map_spec_rank`, `map_spec_new`. Verification never
opens the container's representation: every operation is its role's
contract over the bound specification functions, and those functions are
what contracts and proofs reason over. The map lemmas of the proof language
([chapter 2](proof-language.md)) are named from them.

## Global storage

The Move profile uses index notation for resources: `&Vault[addr]`, `&mut
Vault[addr]`, `Vault[addr].balance := e`, with the builtins
`exists<T>(addr)`, `move_to<T>(signer, value)`, and `move_from<T>(addr)`.
In specifications a resource is `global<T>(addr)`, and `exists<T>(addr)`
is its presence. Stored invariants of untouched resource types hold across
calls, and a contract's `modifies` frame states what a function may change.

## Verification

`move prove --lean` verifies the functions of the package's own modules;
dependency modules are rendered with `verify = false`, so their contracts
apply and their bodies inline while only the package's functions are
obligations. `pragma verify = false` on a spec block or a module excludes
it as in the Move Prover, `pragma verex_verify = false` for Verex alone, and
`pragma verify = manual` states that a function is established by its
proof file. Each verified function gets a bounded automatic attempt under
a default budget (`--heartbeats`); `pragma heartbeats = N` on its spec
block raises its own. A function the automation does not close is
proved by hand in the proof file beside its Move source — the subject of
the next chapter. An authored VerexSL file may also hold `theorem` and
`verify f by …` items among the module's declarations, which the renderer
leaves to the proof file.

The full workflow, flags, and pragmas:
[`../designs/source-verification.md`](../designs/source-verification.md).

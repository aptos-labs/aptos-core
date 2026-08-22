# Leaner source verification — project plan

`spec f` / `verify f` translate a Move function's retained `fun` body directly
into the relational `Move.Semantics.Spec` semantics and discharge the generated
contract automatically (`Move/Verify/Syntax.lean`).  This document tracks what
the surface and translator do **not** yet handle, as a roadmap.  Each item
quotes the diagnostic it raises (or notes it is a silent modeling gap), so the
site is greppable.

## Implemented (context)

Struct/enum/resource declarations; integers `u8..u256` and `i8..i256` as one
model — a `NumType` (width + signedness) whose `lo`/`hi` bounds are the only
difference, so there is one operation family, one checked semantics, one
typing rule and one specification per operation (see
[`unified-int-design.md`](unified-int-design.md)); vectors; checked
arithmetic; `if`/`while`/`loop` with invariants; recursion via `Spec.fix`;
data invariants (certified values); **global invariants** — regular and
`update`, cross-resource, registered per referenced family, discharged by
generated `@[grind]` reestablishment lemmas; the global-storage primitives
`moveTo` / `moveFrom` / `existsAt`; `modifies` framing; abort clauses; the
`requires` / `ensures` / `aborts` / `modifies` contract surface, with resource
observations (`R[a]`, `existsAt<R>`, `old`) in both **requires and ensures**;
ability declarations `has Copy, Drop, Store, Key` on a `struct`/`enum`, and
ability *bounds* on its parameters, `struct Vault (T : Store, Copy)` — Move's
`<T: store + copy>`; a parameter without a bound is still inferred from the
container's own abilities.

**Function bodies.** Direct global borrows `&mut R[a]` / `&R[a]` (whole-resource
replacement and reads) and field borrows chained through them; element borrows
through the active `&mut Vector` parameter and element-field borrows of local
vectors; `if`/`else` statements followed by a continuation, dependent and
pattern `if` (`if h : c`, `if let p := e`), `match` statements, `return` inside
`loop`/`while`; checked arithmetic, casts, checked vector access and Move calls
in any value position — each hoisted in front of the term in evaluation order,
so its abort stays observable; the explicitly spelled core primitives
(`borrowLocal`…`borrowElemMut`, `read`/`readImm`/`freeze`/`write`,
`Move.abort`, `Move.Vector.get`/`set`), desugared to the surface forms they
lower to and given the same semantics.

**Calls.** Effectful callees with a mutable-reference parameter (the caller's
live `&mut` is passed and its final value written back), including recursive
ones; pure callees without a `spec` — their relational semantics is generated
on demand from retained source and persisted across modules; a callee's
`sourceSpec` no longer has to be declared before the caller's `verify`; named
type arguments at a call (`has_generic (T := U64) a`) instantiate the callee's
semantics.

**Generic global storage.** `existsAt (Vault T) a`, generic `moveTo`/`moveFrom`,
`&mut (Vault U64)[a].f`; in contracts `existsAt<Vault T>(a)`, `(Vault T)[a].f`,
`modifies (Vault T)[a]`.  A head's store is in scope for every instantiation
(`∀ T, ResourceStore S (Vault T)`), so a caller reaches a callee's generic
family without spelling the instantiation, and a type parameter no argument
determines is passed to the semantics by name; distinct heads are independent,
two instantiations of one head are not assumed to be.

## Move language coverage — not in Leaner yet

The sections below this one are about the *verification translator*: which
accepted Leaner programs `spec`/`verify` can handle.  This section is about
the *language*: which features of Move on Aptos (per
[the Move book](../../../documentation/book/src/SUMMARY.md)) Leaner source
cannot express at all, so neither the compiler nor the verifier sees them.
Everything listed is rejected at the compilation boundary (Lean elaboration
or "unsupported call … while compiling Move function"); nothing is silently
mis-compiled.

### Functions

- **Function values** (Move 2.2): function types `|u64|bool has copy+drop`,
  lambdas and closures (`|x| x + 1`, captured variables), function-typed
  struct fields and storable function values, dynamic dispatch, and the
  reentrancy check.  Lean closures are Lean-only values
  ("Move-representability": functions are rejected); there is no Leaner
  function type, no lambda syntax, and no `Spec` semantics for evaluation.
  This is the largest missing feature, and the one the framework uses most.
- **Inline functions** (`inline fun`) and lambda parameters of inline
  functions.
- **Native functions** (`native fun`).
- **`package` visibility** and **`friend` declarations** (`friend 0x1::m;`):
  `friend fun` exists but, with no friend-module declarations, is
  module-internal.
- **Tuple-valued functions** and tuple destructuring (`let (a, b) = f()`).
- Explicit **`acquires`** annotations (Leaner infers them — by design).
- Move **`#[test]` unit tests** / `#[expected_failure]`: Leaner tests are the
  `#test run` interpreter checks and Lean `#guard`s.

### Types and values

- **Address and signer literals** (`@0x1`, named addresses): both arrive
  only as arguments; module addresses are the fixed `0x0`.
- **Typed and grouped integer literals** (`112u8`, `1_000`): width comes
  from the expected type (`(112 : U8)`); Lean accepts `0xFF`.
- **Byte-string literals** `b"hello"` / `x"DEAD"` and `vector<u8>`
  constants; **abort messages** (`abort b"…"`, Move 2.4).
- **Constants of complex expressions**: a Leaner constant is a `def` of a
  literal.
- **Positional structs** `struct S(u64)` and positional-field patterns;
  **struct destructuring in `let`** (`let S { f, g } = s`, `let { value } :=
  vault`); **partial patterns** `..`.
- **`match` extensions**: guards (`pattern if cond =>`), literal and range
  patterns (`0..100`, `100..=999`), matching primitive discriminators and
  mixed-tuple discriminators, matching through `&`/`&mut` references,
  literal/range patterns in `let`; the `is` variant test (`e is V1`).
- **Vector operations** beyond `empty`/`push`/`length`/`get`/`set`/
  `insert`/`remove`/element borrows: `pop_back`, `swap`, `swap_remove`,
  `append`, `reverse*`, `contains`, `index_of`, `trim*`, `rotate*`,
  `is_empty`, `singleton`, `destroy_empty`.
- **Comparison and equality** are covered (`<`, `<=`, `>`, `>=`, `==`,
  `!=`, `!`); `<` on non-integer types is Move's structural order.  Signed
  integers `I8`…`I256` are a Leaner extension beyond Move.

### Statements

- **`for (i in lo..hi)`** loops (`while`/`loop`/labels exist).
- **`assert!` / `assert_eq!` / `assert_ne!`** (`Move.assert` has no
  compiler support; write `if ¬c then abort code`).
- **Compound assignments** (`x += 1`, Move 2.1) — write `x := x + 1`.

### Specification language

Leaner contracts are their own surface (`spec f … where requires / modifies /
ensures / aborts_if`, data and global invariants), not the Move Prover's
MSL.  MSL constructs without a Leaner counterpart: `spec fun` / `spec
schema` / `include` / `apply`, `pragma` (`opaque`, `verify`, `aborts_if_is_
partial`, `intrinsic`), `aborts_with`, `emits`, `let post`, `global` spec
variables, and the Prover's quantifier/choice helpers (`forall`/`exists`
are Lean's binders).

### Suggested priority (language)

1. **Function values** — closures as first-class Leaner values with a
   relational semantics (and the reentrancy discipline), since they are
   pervasive in the framework.
2. Tuples / multiple returns; struct destructuring and partial patterns.
3. `for` loops, `assert!`, compound assignments (surface sugar over existing
   lowering).
4. Byte-string literals, address literals, module addresses, `friend`
   declarations, the remaining vector operations.

## Roadmap — verification, not yet implemented

### Function bodies the translator rejects

- **Sibling nested mutable borrows** — a second `&mut` borrow from a live
  mutable reference that is not a field or element of it.
  - *"nested mutable borrows are not yet supported by source specification
    generation"*
- **Effects in conditional positions** — arithmetic, a cast, a checked vector
  access or a call as the right operand of `&&`/`||`, inside a branch of a
  value `if`, a `match` arm, or a `fun` body.  Evaluation there is
  conditional, so the effect cannot be sequenced in front of the term.
  - *"automatic source specifications cannot sequence this operation here,
    where its evaluation is conditional; bind it to a local first"*
- **`if let pat ← e`** and **dependent `while` conditions.**
  - *"`if let pat ← e` is not supported by source specification generation;
    bind `e` with `let` first"* / *"a dependent `while` condition is not
    supported …"*
- **`match` with a motive or `generalizing` clause, on several discriminants,
  or with a named discriminant.**
  - *"`match` with a motive or generalizing clause is not supported …"* and
    siblings.
- **Receiver-style checked vector operations** (`values.get i`, `r.insert i e`)
  — raw source does not retain what they resolve to.
  - *"automatic source specifications require fully qualified
    `Move.Vector.get`, `Move.Vector.set`, `Move.Vector.insert`, or
    `Move.Vector.remove`"*
- **A core primitive in a form the surface cannot express** (a `borrowField`
  with a computed descriptor, `assert`) — the safety net behind the
  desugaring.
  - *"automatic source specifications do not yet model `{operation}`; provide
    an explicit `sourceSpec` or omit `verify`"*

### Calls

- **Mutually recursive callees.**
  - *"mutually recursive Move functions are not yet supported by automatic
    source specifications (`{f}`)"*
- **Callers of a recursive callee** are not discharged automatically: the
  callee's semantics is a fixed point, which the automatic proof does not
  unfold.  Such a caller is proved by hand from the callee's verified contract
  (`wp_of_satisfies` / `wp_call`).  No diagnostic; the automatic proof fails.
- **More than one `&mut` parameter.**
  - *"source contracts currently support at most one mutable-reference
    parameter"*
- A callee must be a `fun` (retained source); a Lean `def` has no semantics to
  generate.
  - *"Move callee `{f}` has no retained source; declare it with `fun` …"*

### Types

- **Product-/tuple-valued** Move functions.
- Recursive and indexed structs/enums are not Move types; they are rejected at
  the compilation boundary by design, not a verification gap.

### Recursion

- **Mutually-recursive contract families:** `contract_intro` cannot open them;
  `satisfies_fixFamily` must be applied by hand.
  - *"`contract_intro` does not yet open mutually recursive contract families …"*

### Known modeling gaps (no diagnostic; behavioral)

- **`Signer.address` is uninterpreted** (`signer::address_of`). Sound for
  verification, but `moveTo`'s frame location must be written
  `Counter[account.address]`, and `moveTo`'s resource type is read from the
  published value's ascription (`({ … } : Counter)`).
- **Generic families are framed per named instantiation.** The frame of a
  contract covers the instantiations the body and the clauses name
  (`Vault T`, `Vault U64`); an instantiation nobody names is left
  unconstrained, and two instantiations of one head are never assumed
  independent (conservative: `Vault T` and `Vault U` may coincide).
- **Generic global invariants.** `spec module where invariant` names families
  by bare identifier; a generic family cannot yet carry a global invariant.
- **Update global invariants** must be reflexive at unchanged addresses (a
  `≤`-style relation verifies; `<` cannot). This is faithful to the Move
  Prover's "checked at every update over all addresses" reading, not a gap.

### The list is not exhaustive

Statement forms the translator does not recognize at all fall through to two
catch-alls rather than to a named item above, so an unlisted construct may
still be rejected:

- *"unsupported effectful statement in automatic source specification: …"*
- *"unsupported `do` statement in automatic source specification: …"*

## Suggested priority (verification)

1. Tuple-valued functions — they dominate the remaining framework surface.
2. Mutually recursive callees and contract families.
3. Generic global invariants.
4. Sibling nested mutable borrows; effects in conditional positions
   (short-circuit operands), via a conditional `Spec`.

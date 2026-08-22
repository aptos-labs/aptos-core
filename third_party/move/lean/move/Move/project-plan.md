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

**Calls.** Effectful callees with one or two mutable-reference parameters (the
caller's live `&mut`s are passed and their final values written back in source
order), including recursive ones; callers reuse an already verified recursive
callee's contract instead of unfolding its fixed point. Pure callees without a
`spec` have relational semantics generated on demand from retained source and
persisted across modules; a callee's `sourceSpec` no longer has to be declared
before the caller's `verify`; named type arguments at a call
(`has_generic (T := U64) a`) instantiate the callee's semantics.

**Recursion.** A mutually recursive strongly connected component is translated
to one heterogeneous `Spec.fixFamily`: each member retains its own argument and
result type. `contract_intro` opens the whole contract family and supplies the
family-wide recursive hypothesis, while each member still receives its normal
`f.sourceSpec`, `f.contractSpec`, and `f.verified` declarations.

**Generic global storage.** `existsAt (Vault T) a`, generic `moveTo`/`moveFrom`,
`&mut (Vault U64)[a].f`; in contracts `existsAt<Vault T>(a)`, `(Vault T)[a].f`,
`modifies (Vault T)[a]`.  A head's store is in scope for every instantiation
(`∀ T, ResourceStore S (Vault T)`), so a caller reaches a callee's generic
family without spelling the instantiation, and a type parameter no argument
determines is passed to the semantics by name; distinct heads are independent,
two instantiations of one head are not assumed to be.

Global invariants may constrain a concrete instantiation of a generic family,
for example `(Vault U64)[a].value`; registration is keyed by the generic head
so mutations of that instantiation discover and re-establish the invariant.

## Move language coverage

The sections below this one are about the *verification translator*: which
accepted Leaner programs `spec`/`verify` can handle.  This section is about
the *language*: which features of Move on Aptos (per
[the Move book](../../../documentation/book/src/SUMMARY.md)) Leaner source
cannot express at all, so neither the compiler nor the verifier sees them.
Everything listed is rejected at the compilation boundary (Lean elaboration
or "unsupported call … while compiling Move function"); nothing is silently
mis-compiled.

The non-deferred surface work is implemented: inline/native declarations;
`package` and friend visibility; Move test attributes; literal and named
addresses (`address_alias`, `module M at alias|0x…`, `@alias`); typed/grouped
integers and complex constants; byte strings and abort messages; tuples and
multiple returns; positional and named struct destructuring; partial patterns;
primitive/byte-string literal, range, guarded, mixed-discriminator, and
reference matches; `is`; the complete planned vector operation set; range
loops; assertions; and compound assignments. Explicit `acquires` remains
inferred by design. Literal/range patterns outside `match` are intentionally
rejected, matching Move 2.4's irrefutable-pattern rule.

The one deliberately deferred language family is **function values** (Move
2.2): function types, lambdas/closures and captures, storable function values,
dynamic dispatch, and the reentrancy discipline.

### Specification language (deferred)

Leaner contracts are their own surface (`spec f … where requires / modifies /
ensures / aborts_if`, data and global invariants), not the Move Prover's
MSL.  MSL constructs without a Leaner counterpart: `spec fun` / `spec
schema` / `include` / `apply`, `pragma` (`opaque`, `verify`, `aborts_if_is_
partial`, `intrinsic`), `aborts_with`, `emits`, `let post`, `global` spec
variables, and the Prover's quantifier/choice helpers (`forall`/`exists`
are Lean's binders).

MSL is outside the current scope. Function values are the only deferred Move
language feature tracked here.

## Test and diagnostic coverage

Tests are grouped by responsibility under `Move/Tests`: `Language` checks the
accepted source surface, `Verification` checks contracts and proofs,
`Compiler` checks XIR/export and compiler integration, and `Negative` pins
expected failures. Negative tests use exact `#guard_msgs` assertions for
surface, specification, lowering, and automatic-verification diagnostics.

Accepted programs now receive a retained-source, poison-aware borrow program,
certificate, and `wellBorrowed` theorem, implemented in
`Verify/SourceProgram.lean`, `Verify/BorrowChecker.lean`, and
`Verify/Syntax.lean`. The checker deliberately does not duplicate compiler-v2:
overlapping mutable handles may exist until a destructive use activates a
prophecy, with later conflicting uses rejected by static poisoning. Calls,
returned references, loops, and recursive SCCs use replayed fixpoint summaries.

Compiler-side XIR tests additionally reject malformed control-flow targets,
local and type indices, return arity, operation arity, resource references,
and address constants. Transactional `reject_*` tests check that these errors
reach command-line users. Positive Leaner transactional baselines are scanned
for compiler, bytecode-verifier, or linker failures so a broken fixture cannot
be accepted as a successful regression baseline.

## Roadmap — verification, not yet implemented

### Function bodies the translator rejects

- Disjoint sibling field borrows are supported, including reads through both
  simultaneously-live loans. Overlapping paths are rejected. Ordinary
  multi-discriminant matches are translated directly (currently up to two,
  which covers Move's mixed-tuple surface); Lean-only motive/generalizing
  clauses are not Move syntax.
- **A core primitive in a form the surface cannot express** (a `borrowField`
  with a computed descriptor, `assert`) — the safety net behind the
  desugaring.
  - *"automatic source specifications do not yet model `{operation}`; provide
    an explicit `sourceSpec` or omit `verify`"*

### Calls

- A callee must be a `fun` (retained source); a Lean `def` has no semantics to
  generate.
  - *"Move callee `{f}` has no retained source; declare it with `fun` …"*

### Types

- Recursive and indexed structs/enums are not Move types; they are rejected at
  the compilation boundary by design, not a verification gap.

### Known modeling gaps (no diagnostic; behavioral)

- **Generic families are framed per named instantiation.** The frame of a
  contract covers the instantiations the body and the clauses name
  (`Vault T`, `Vault U64`); an instantiation nobody names is left
  unconstrained, and two instantiations of one head are never assumed
  independent (conservative: `Vault T` and `Vault U` may coincide).
- **Update global invariants** must be reflexive at unchanged addresses (a
  `≤`-style relation verifies; `<` cannot). This is faithful to the Move
  Prover's "checked at every update over all addresses" reading, not a gap.

### The list is not exhaustive

Statement forms the translator does not recognize at all fall through to two
catch-alls rather than to a named item above, so an unlisted construct may
still be rejected:

- *"unsupported effectful statement in automatic source specification: …"*
- *"unsupported `do` statement in automatic source specification: …"*

The planned verification gaps from the previous roadmap are implemented. The
remaining entries above are explicit design boundaries or conservative
modeling choices, rather than silently missing source semantics.

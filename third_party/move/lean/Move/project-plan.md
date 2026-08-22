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
`moveTo` / `moveFrom` / `exists_`; `modifies` framing; abort clauses; the
`requires` / `ensures` / `aborts` / `modifies` contract surface, with resource
observations (`R[a]`, `exists<R>`, `old`) in both **requires and ensures**.

## Roadmap — not yet implemented

### Function bodies the translator rejects (highest practical value)

- **Direct global borrows.** `&T[a]` / `&mut T[a]` that are not immediately a
  field write-back or read. Only `&mut R[a].field := …` and `&R[a].field` are
  modeled.
  - *"direct mutable global borrows are not yet supported by source
    specification generation"* / *"direct immutable global borrows …"*
- **Nested mutable borrows** and **mutable vector-element field borrows.**
  - *"nested mutable borrows are not yet supported …"* /
    *"mutable vector-element field borrows are not yet supported …"*
- **The explicitly spelled core primitives** — the `unsupportedSourceOperation`
  set: `borrowLocal`/`borrowGlobal`/`borrowField`/`borrowElem` and their `Mut`
  variants, `freeze`, `read`, `readImm`, `write`, `assert`, `abort`,
  `Vector.get`, `Vector.set`.  The surface sugar (`&x`, `&mut R[a].f`, `*r`,
  `r := v`) is modeled; writing the primitive itself is not.
  - *"automatic source specifications do not yet model `{operation}`; provide
    an explicit `sourceSpec` or omit `verify`"*
- **Arithmetic in index/embedded position** — must be bound to a local first.
  - *"automatic source specifications do not yet support arithmetic in this
    context; bind it to a local first"*
- **Dependent / pattern `if` conditions.**
  - *"dependent and pattern `if` conditions are not yet supported …"*
- **`return` inside `loop` / `while`.**
  - *"`return` inside `loop` / `while` is not yet supported for `verify`"*

### Calls

- **Effectful callee with a mutable-reference parameter.**
  - *"automatic source specifications do not yet model calls to effectful Move
    callee `{f}` with a mutable-reference parameter"*
- **Pure callee with no relational summary** — must be inlined.
  - *"automatic source specifications do not yet model pure Move callee `{f}`;
    inline it or omit `verify`"*
- A callee's `sourceSpec` must be declared before the caller's `verify`.

### Types

- **Recursive** structs/enums and **indexed** structs/enums.
  - *"recursive Move type `{name}` is not supported by the prototype"* /
    *"indexed Move structure `{name}` is not supported"* /
    *"indexed Move enum `{name}` is not supported by the prototype"*
- **Product-/tuple-valued** Move functions.

### Recursion

- **Mutually-recursive contract families:** `contract_intro` cannot open them;
  `satisfies_fixFamily` must be applied by hand.
  - *"`contract_intro` does not yet open mutually recursive contract families …"*
- **Recursive contracts with a mutable-reference parameter.**
  - *"recursive source contracts with mutable-reference parameters are not yet
    supported"*

### Known modeling gaps (no diagnostic; behavioral)

- **`Signer.address` is uninterpreted** (`signer::address_of`). Sound for
  verification, but `moveTo`'s frame location must be written
  `Counter[account.address]`, and `moveTo`'s resource type is read from the
  published value's ascription (`({ … } : Counter)`).
- **Generic global-storage primitives** (`exists_ (Vault T) addr`, generic
  `moveTo`/`moveFrom`) are not yet on the source-verify path — only concrete
  resource families.
- **Update global invariants** must be reflexive at unchanged addresses (a
  `≤`-style relation verifies; `<` cannot). This is faithful to the Move
  Prover's "checked at every update over all addresses" reading, not a gap.

### The list is not exhaustive

Statement forms the translator does not recognize at all fall through to two
catch-alls rather than to a named item above, so an unlisted construct may
still be rejected:

- *"unsupported effectful statement in automatic source specification: …"*
- *"unsupported `do` statement in automatic source specification: …"*

## Suggested priority

1. **Direct global borrows** and **calls with mutable-ref parameters** — these
   dominate real framework code.
2. Generic global-storage primitives (needed for generic resource wrappers).
3. Recursive/indexed types.
4. `return` in loops; dependent `if`.

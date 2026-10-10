# Invariants

This document covers two kinds of invariant, which differ in what they
constrain and where they are checked:

- a **data invariant** (below) constrains one value of a struct or enum type
  and is certified in the value — checked once, where the value is created;
- a **global invariant** ([last section](#global-invariants)) constrains the
  whole global resource state and is certified in the state — checked at each
  point the state changes, assumed everywhere else.

Both reuse the same relational-semantics mechanism (the `undefined`
well-definedness component and the `Spec.certified` shape); they are two
granularities of the same idea, "carry the proof so reading is free."

# Data invariants

Status: implemented for structs, enums, and resources

A data invariant is a property every value of a struct or enum type satisfies.
A value of such a type is *certified*: it carries the proof, exactly as
`Move.UInt` carries its range bound and `Move.Vector` carries its length
bound. Nothing is re-proved when a value is read, passed, stored, or
returned. The only proof obligation is where a value is created.

## Surface

```lean
struct Map (K V) where
  entries : Vector (Entry K V)

spec Map {K} {V} where
  invariant Model.SortedEntries .entries.toList
```

- Clauses read like the other spec blocks and may be repeated, separated by
  `;` and conjoined — the same shape as repeated `aborts_if`. The `invariant`
  keyword is what distinguishes a data invariant from a function contract.
- `this` denotes the value being constrained, and `.field` abbreviates
  `this.field`. For an enum the condition matches on it:

  ```lean
  enum Payment where
    | None
    | Direct (amount : U64)
    | Split (left right : U64)

  spec Payment where
    invariant match this with
    | .None => True
    | .Direct amount => 0 < amount.toNat
    | .Split left right => 0 < left.toNat ∧ 0 < right.toNat
  ```

- The declaration may appear before or after the type; `module` has all
  of its items in hand before elaborating them, so a pre-pass attaches the
  invariant to the type it names.

## Representation

Stating the condition requires a value of the type, but the certified type
cannot exist until its condition does. The way out is a **raw twin**: a
proof-free copy of the declaration, used only to state the condition.

```lean
-- generated from `struct Map`
structure Map.Raw (K V) where
  entries : Vector (Entry K V)

-- generated from `spec Map where …`; `this` is the raw value
def Map.Invariant {K V} (this : Map.Raw K V) : Prop :=
  Model.SortedEntries .entries.toList

-- the type users write and the compiler sees
structure Map (K V) where
  entries : Vector (Entry K V)
  invariant : Map.Invariant { entries := entries } := by move_invariant
```

The certified type keeps the original fields, so `map.entries` needs no
indirection, and `map.invariant` is the proof, available wherever a `Map` is.
The raw twin is a Lean-level artifact: never compiled, never mentioned in
source or contracts.

For an **enum** the same twin states the condition, and the proof rides along
in each constructor, so `match` still discriminates on the real value:

```lean
inductive Payment.Raw where
  | None
  | Direct (amount : U64)
  | Split (left right : U64)

def Payment.Invariant (this : Payment.Raw) : Prop := match this with …

inductive Payment where
  | None (invariant : Payment.Invariant .None := by move_invariant)
  | Direct (amount : U64)
      (invariant : Payment.Invariant (.Direct amount) := by move_invariant)
  | Split (left right : U64)
      (invariant : Payment.Invariant (.Split left right) := by move_invariant)
```

Each constructor's obligation reduces by iota to that variant's arm of the
condition. Construction is unchanged in source — the default discharges the
argument — but *patterns* bind the proof with a trailing `_`: `.Direct
amount` becomes `.Direct amount _`. Lean runs a constructor's default tactic
inside a pattern too, so the binder cannot be left implicit; the rule is
explicit rather than heuristically padded, and forgetting it reports
"cannot establish the data invariant of this value here (if this is a
pattern of a certified enum, bind the proof with a trailing `_`)". It applies
only to enums that declare an invariant, and the proof is in scope in each
arm — `firstPart` in `Tests/Verification/Invariants.lean` uses it.

Two consequences of stating the condition over the twin:

- Predicates used in an invariant take the raw components, not the certified
  type — `Model.SortedEntries .entries.toList` rather than
  `Model.Sorted this`. A definition like `Model.Sorted : Map K V → Prop`
  would be circular, and in `OrderedMap` it disappears.
- The condition may only mention declarations that precede the type, so
  `SortedEntries` (which speaks about `Entry`, not `Map`) moves above
  `struct Map`.

`Move.Compare.Less` is defined for every type through a global `LT` instance,
so sortedness needs no instance binder on `Map`; `Compare.Total` stays a
proof-side assumption of the functions, as today.

## Where the obligation lands

Every value originates from a creation site, and there are two kinds.

**Statically dischargeable — a literal.** `Map.empty`'s body is
`{ entries := Move.Vector.empty }`, whose obligation reduces to
`SortedEntries [] = True`. The field default discharges it during
elaboration, so the source carries no proof text and a violation is reported
at the literal. `move_invariant` is the usual layered discharger (`trivial`,
`decide`, `simp [move_norm, …]`, `assumption`), extensible per module through
a simp-set attribute.

**Runtime-dependent — re-establishment at the end of a mutation.** `add`
borrows `&mut map.entries`, inserts at a searched index, and the `Map` is
rebuilt when that loan dies. Sortedness there depends on a value computed at
run time, so the obligation belongs in the function's verification condition,
at the point the loan dies — "not enforced during the mutation, enforced on
the prophecy result".

**Conditional creation in pure code.** `if h : 0 < amount then .Direct
amount else .None` — the dependent `if` puts the branch condition in scope,
and `move_invariant` discharges the creation from it. A plain `if` would not:
there is nothing to prove the invariant from. The source `<` on integers is
the compiler's `UInt.less` primitive; its numeric meaning is the trust-base
axiom `UInt.less_eq_true_iff` (the integer-specific counterpart of
`logicalLT_uint`), exposed as the simp lemma `UInt.lt_iff_toNat_lt`.

**Resources.** A certified value stored in global memory is certified when
read back, so `R[addr]` reads are free and `R[addr].invariant` is available
in proofs. A mutable borrow `&mut R[addr].field` of a certified resource is
translated with the write-back as a creation site: `withBorrowMutSpec`
around `withMutation` of the focus, then `Spec.certified` re-creating the
owner from its fields before it is stored — the same rule as the local case,
at the point the loan dies. Publication of a value needs nothing: the value
was certified when created.

Nothing else generates an obligation: reads, parameter passing, returns, and
storage are free.

## Well-definedness in the relational semantics

A runtime-dependent obligation has to be *positive*. Today `wp` is derived
from `ok` and `aborts` alone, so a rule whose `ok` relation is empty when the
invariant fails would make `wp` hold vacuously — verification would succeed
for a function that breaks the invariant. `Spec` therefore gains a third
component:

```lean
structure Spec (State Result : Type) where
  ok : State → Result → State → Prop
  aborts : State → Nat → Prop
  undefined : State → Prop := fun _ => False
```

- Every existing primitive keeps the default: `pure`, `abort`, `get`, `set`,
  `modify`, the checked arithmetic, the borrow and vector rules — all are
  total, so the change is mechanical.
- `bind` propagates it: an execution is undefined if the first action is, or
  if it succeeds into a continuation that is:
  ```lean
  undefined := fun initial =>
    action.undefined initial ∨
      ∃ value middle, action.ok initial value middle ∧
        (next value).undefined middle
  ```
- `wp` gains the conjunct `¬ action.undefined initial`, and `Satisfies`
  requires it of every state meeting `requires`. `wp_bind` splits it along
  the same lines as the abort half, so the existing proof rules keep their
  shape.
- `fix` takes the union over finite approximations, as for `ok`/`aborts`.

The loan-death rule for a certified type is then the only source of
`undefined`: it is `¬ T.Invariant rebuilt`. This is the standard
partial-correctness-with-well-definedness setup, and it gives a home to any
future source-level `assert`.

## Compiler impact

A `Prop` field must not reach Move. LCNF erases proofs already, so the work
is in the frontend:

- `structDeclFor` currently errors with "field has an erased type" for a
  field it cannot translate; proof fields must be skipped instead, and left
  out of the emitted `StructDecl.fieldNames`.
- The pack path checks `vars.size == numFields`; `numFields` must count
  runtime fields only, since LCNF drops the proof argument. The existing skip
  of instance arguments (`instWidth`) is the precedent.
- Enum variants need the same treatment for their trailing proof argument.
- Field projection and XIR need no change — executable code never reads a
  proof field.

## What it buys, on the motivating example

`Tests/Verification/OrderedMap.lean` threads well-formedness by hand through five
contracts:

```lean
spec add {K} {V} [Move.Compare.Total K]
    (map : &mut Map K V) (key : K) (value : V) where
  requires Model.WellFormed map;
  ensures map.entries.toList = Model.add (old(map)) key value ∧
    Model.Sorted map;
  aborts_if …
```

With the invariant on `Map`:

```lean
spec add {K} {V} [Move.Compare.Total K]
    (map : &mut Map K V) (key : K) (value : V) where
  ensures map.entries.toList = Model.add (old(map)) key value;
  aborts_if …
```

Five `requires Model.WellFormed map` clauses, two `Model.Sorted` conjuncts,
and both `WellFormed` and `Sorted` as definitions disappear. Proof content is
unchanged but relocated: `Model.Insertion.add_sorted` moves from discharging
an `ensures` conjunct to discharging the loan-death obligation, and
`permitted.sorted` becomes `map.invariant`.

## Scope

Implemented:

- `spec T where invariant P; …` for structs, with type binders, the raw
  twin, the generated `T.Invariant`, and the certified field with its
  `move_invariant` default;
- the `undefined` component and the loan-death rule;
- compiler erasure of proof fields;
- `OrderedMap` converted, `Model.WellFormed` and `Model.Sorted` removed.

- **Enums**, as sketched above, with the trailing-`_` pattern rule; the
  compiler erases the proof argument from variants, constructor arity, and
  destructuring.
- **Resources**, with the write-back of a mutable field borrow as the
  creation site.

Out of scope: invariants relating a value to global state — only the value is
in scope, never `initial`/`final` — and a nested certified structure inside a
certified owner's mutated field path (the inner rebuild is an ordinary
update, which Lean rejects for a certified inner type).

## What shipped

- `spec T where <term>` inside a `module`, consumed by a pre-pass in the
  module expander, so the declaration may precede or follow its type.
- `T.Raw`, `T.Invariant` (tagged `@[move_invariant_norm]`, parameters
  implicit so it applies to a value directly), the certified structure with
  `invariant : T.Invariant … := by move_invariant`, an `Inhabited` instance
  whose default value carries the proof, and a `#register_move_invariant`
  registration the source translator reads.
- `Spec.certified` with its `wp_certified` rule, emitted by the translator
  when a mutable field borrow into a certified owner dies.
- Compiler erasure: proof fields are dropped from `StructDecl` and from the
  `pack` arity, so Move sees only data.
- `move_invariant`, layered cheapest-first (`trivial`, `rfl`, `decide`,
  `assumption`) before any simp, and unfolding only `move_invariant_norm`.

`Tests/Verification/Invariants.lean` covers structs, a two-clause invariant, a
certified enum (construction under a dependent `if`, a `match` with the proof
in scope), and a certified resource (a mutable field borrow whose write-back
re-establishes the invariant), plus compiler erasure and execution;
`Tests/Verification/OrderedMap.lean` is the real conversion.

## Resolved questions

1. Pattern padding for certified enums: users write the trailing `_`; Lean
   runs the default tactic in patterns, so padding cannot be avoided by
   making the argument implicit, and heuristic rewriting was rejected.
2. `move_invariant` is user-extensible through the `move_invariant_norm` simp
   set; whether modules need more than that is unknown until one does.
3. A mutable borrow into a certified owner may select any field path; only
   the outermost owner is re-created through `Spec.certified`, inner
   non-certified structures by ordinary update.

# Global invariants

Status: implemented for structs, enums, and resources, including the global
storage primitives `moveTo`/`moveFrom`/`existsAt`

A global invariant constrains the whole global resource state — a condition
over the resources stored in memory, quantified over addresses and possibly
relating *several* resource families.  It is the Move Prover's `invariant`
module member.

```lean
struct Counter has Key where
  value : U64

spec module where
  -- Regular: a state predicate, assumed at reads, asserted at each write.
  invariant ∀ a, 0 < Counter[a].value.toNat;
  -- Update: a pre/post relation, asserted at each write (never assumed).
  invariant update ∀ a, old(Counter[a]).value ≤ Counter[a].value
```

Two forms (matching the Move Prover, `documentation/book/src/spec-lang.md`):

- **Regular** `invariant ∀ a, P` is a state predicate `State → Prop`,
  assumed on entry and re-established at each write.  *No `this`, no `old`* —
  it ranges over addresses with the resource surface `Counter[a]`,
  `existsAt<R>(a)`.
- **Update** `invariant update ∀ a, R` is a relation `State → State →
  Prop` between the pre- and post-state of a change, asserted at each write
  only.  `old(R[a])` is the pre-state; bare `R[a]` the post-state.

Both are general predicates over `get`/`contains` of **any** families named,
so cross-resource invariants like `all a: existsAt<Debit>(a) ↔ existsAt<Credit>(a)`
are expressible — not restricted to one family.  A value-accessed family
`R[a].field` carries an implicit `existsAt<R>(a)` guard (absent addresses are
unconstrained); families named only through `existsAt<R>` add no guard.  Several
`invariant` clauses conjoin.

Unlike a data invariant, a global invariant has no single value to attach a
proof to — the "value" is the abstract global state.  So the proof cannot ride
in a field; the obligation is emitted at the state transitions instead, and an
invariant is registered under **every** family it names, so a write to any of
them re-checks it (and only those) — exactly the Move Prover's rule.

## Semantics: certified state, checked at the change

Following the Move Prover (`documentation/book/src/spec-lang.md`): a global
invariant is **assumed** wherever global state is read, and **asserted at the
moment the state is updated** — immediately after the write, not at the end of
the function.  Equivalently, and matching the framing "before a change the
affected invariants are assumed, after the change they are asserted": the
pre-state satisfies the invariant (assumed), and the post-state must satisfy
it again (asserted).

The global state is thereby *certified* the same way a `Move.UInt` or a
certified struct is: every state a verified program can observe satisfies the
invariant, so a read is free and only a write owes a proof.  Concretely a
function's proof gets:

| position | clause | direction |
|---|---|---|
| function entry | `Inv initial` | assumed |
| immediately after each global write to a mentioned family | `Inv (post-write state)` | asserted |
| (the continuation of that write) | `Inv (post-write state)` | assumed |

Because entry assumes the invariant and every write re-establishes it, the
current state satisfies `Inv` at every program point between writes — which is
exactly what makes the "assumed at reads" reading sound, and gives the
pre-state assumption a write needs (`Counter[addr].value > 0` is available to
justify the `- 1`, and the write then fails to re-establish it when the value
was 1).

```lean
entry fun decrement (addr : Address) : Action Unit := do
  let value ← &mut Counter[addr].value
  value := *value - 1        -- write-back asserts Inv(post): FAILS (1 → 0)
```

## Mechanism: `certifyState`, reusing the well-definedness component

The state assertion is a positive obligation checked at the write, exactly
the role the `undefined` component plays for data invariants.  Introduce one
primitive:

```lean
def Spec.certifyState (Inv : State → Prop) : Spec State Unit where
  ok        := fun initial result final => Inv initial ∧ result = () ∧ final = initial
  aborts    := fun _ _ => False
  undefined := fun initial => ¬ Inv initial
```

with the weakest precondition

```lean
wp (certifyState Inv) ens abt s  ↔  Inv s ∧ (Inv s → ens () s)
```

— `Inv s` is the assertion (the update's obligation), and `Inv s` is handed to
the continuation (the assumption downstream).  The translator inserts
`certifyState Inv` immediately after each global write that touches a family
the invariant mentions.  So the write of `&mut R[addr].field` becomes

```
withBorrowMutFocusSpec … (write-back) >>= fun _ => certifyState Inv >>= …
```

and `moveTo`/`moveFrom` are wrapped the same way — each is followed by a
`certifyState (forallStored body)` over the family it publishes to or removes
from.  This lands the obligation at the update point (not the function end)
and re-supplies the invariant to the rest of the body.

The write-back obligation `forallStored body (insert s a v)` — and its `erase`
counterpart — is discharged automatically.  Two `iff` rewrites,
`forallStored_insert_iff` and `forallStored_erase_iff`, split it into the
changed value's `body v` (a real obligation: the published/updated value must
satisfy the invariant) and the untouched frame (`∀ other ≠ a, …`), the latter
closed from the entry certificate.  The generated per-family predicate
`GlobalInvariant_<R>` is tagged `@[grind]` so the finisher can unfold it after
`forallStored_get` supplies a framed value, needing no bespoke tactic.

## Global storage primitives in the source semantics

`existsAt R addr`, `moveFrom R addr`, and `moveTo signer value` translate to the
relational `Resource.containsSpec` / `moveFromSpec` / `moveToSpec` over the
same `ResourceStore.descriptor` a `&mut R[addr]` borrow uses.  `moveTo`
publishes at the signer's address: `Ref.address : Ref Signer → Address`
(uninterpreted, Move's `signer::address_of`) is the bridge from the opaque
signer to the store key, so `moveTo account v` becomes `moveToSpec descriptor
(Signer.address account) v`.  A `moveTo`/`moveFrom` on a family carrying a
global invariant re-certifies the state immediately afterward; `existsAt` is a
read and re-certifies nothing.  The families a function touches — for the
descriptor bindings, the entry certificate, and the frame — are inferred from
these primitives as well as from borrows (`existsAt`/`moveFrom` name the family
directly; `moveTo` through the published value's ascription).

Function entry conjoins `Inv initial` into the generated `requires`.  It is an
*implicit* precondition, never written by the user: at a call site the caller
discharges it from its own copy of `Inv` (which it holds, having assumed it on
entry and re-established it at its writes), so the invariant composes
modularly — every function assumes it on entry and guarantees it on exit.  A
read-only function assumes it and asserts nothing; a writing function assumes
it on entry and asserts it after each write.

## Which invariants a write must re-establish

The Move Prover re-checks an invariant at an instruction only if that
instruction touches a resource the invariant mentions.  A global invariant
declares its families by the `existsAt<R>`/`R[addr]` occurrences in its
predicate — the same inference the `modifies` frame already performs — and a
write to family `S` carries the `certifyState`/`certifyUpdate` obligation only
for invariants that mention `S`.

### Discharge: folded predicate + reestablishment lemmas

The obligation is `Inv (insert s w v)` — a `∀ address` predicate.  Handing that
to the shared `simp_all`/`grind` finisher makes it explode (it churns on the
unbounded quantifier), so each invariant is generated as three declarations:

- `Inv_at (state) (a) : Prop` — the per-address body `guard → P`, `@[grind]`;
- `Inv (state) : Prop := ∀ a, Inv_at state a` — `@[irreducible]`, so the
  finisher never expands the quantifier;
- one `@[grind ←]` **reestablishment lemma** per named family and write shape
  (`insert`/`erase`): `entry → (Inv_at s w → Inv_at (post) w) → Inv (post)`,
  proved once by `intro a; by_cases a = w; [changed; frame]`.  The `a ≠ w`
  frame comes from the entry invariant via `lookup_insert_other` and the
  cross-store `IndependentResourceStores` laws; the changed address `w` gets
  the pre-state invariant threaded in (`Inv_at s w → Inv_at (post) w`).

`grind` closes `Inv (insert …)` by applying the reestablishment lemma (the
invariant stays opaque — no quantifier blow-up), leaving only the changed
value's small obligation.  This is "evaluated exactly after a change at `R[w]`,
everything else is unchanged and holds from the pre-state" made mechanical: the
single new obligation is at `w`.  Update invariants are identical but framed by
reflexivity (`Inv_at pre pre a` for `a ≠ w`) instead of an entry assumption.

The verifier's resource set is closed under invariant mentions, so a function
that writes only `S` but whose invariant relates `S` and `T` still brings
`T`'s store into scope; and `IndependentResourceStores` is supplied in both
directions since a write may frame the other family either way.

## Relation to data invariants, `modifies`, and the trust base

- **Data vs global.**  A data invariant certifies a *value* at *construction*;
  its proof rides in the value.  A global invariant certifies the *state* at
  each *update*; with no value to hold the proof, the obligation is emitted at
  the write and the assumption threaded from entry and prior writes.  A
  resource may carry both — a per-value data invariant on its type and a
  cross-address or cross-resource global invariant on the store.
- **`modifies`.**  The frame limits which addresses and families a function
  changes; the global-invariant assertion fires only at those writes, and only
  for the invariants naming those families.  The two are complementary: one
  bounds *what* changes, the other certifies that *what changed still
  satisfies* the invariant.
- **Trust base.**  The entry assumption `Inv initial` is the same kind of
  assumption as a data invariant's at a transaction boundary: the global state
  respects every declared invariant when the transaction begins, because every
  prior transaction re-established each invariant at its writes.  This is the
  induction the certified-state reading rests on, and belongs in the
  documented trust base.

## Scope

In scope, matching the request:

- **regular** global invariants only — assumed at reads, asserted at each
  update, checked at the point of change;
- address-quantified `∀ addr, existsAt<R>(addr) → P` and cross-resource forms;
- type-parametric invariants;
- multiple `invariant` clauses (conjoined).

Deliberately out of scope for the first step (the Move Prover features):

- **update invariants** (`invariant update …`, a relation between the pre- and
  post-state of an update) — the user asked for regular invariants only;
- **disabling** (`disable_invariants_in_body`, `delegate_invariants_to_caller`,
  `[suspendable]`) — the feature that moves the check from the update point to
  the function end or the caller, for code that transiently breaks the
  invariant while publishing several resources;
- **isolated** invariants.

## Resolved questions

1. Surface keyword: shipped as `spec module where invariant R: P`, one clause
   per family with `this` bound to a stored value — parallel to the
   data-invariant surface `spec T where invariant P` rather than a
   distinguished `module` subject with an explicit `∀ addr` quantifier. The
   per-family form keeps each clause's `this` a single value and makes the
   implicit `forallStored` quantifier uniform with data invariants.
2. Entry conjoins `Inv initial` into the generated `requires`, so a caller
   discharges it from its own copy — modular composition through the existing
   contract structure.
3. `moveTo`/`moveFrom`/`existsAt` are part of the generated source semantics
   (see above); a global invariant over a published or removed family is
   re-certified at those transitions.

## Open questions

1. State-parametric key naming for `moveTo`: the modified address is
   `Signer.address account`, which the `modifies` clause must name as
   `Counter[account.address]`.  This is explicit but slightly indirect; a
   sugar that infers the signer's address for a `moveTo`-only function could
   remove it.

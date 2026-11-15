# Three encodings of a data invariant, side by side

## Scope

A data invariant is a property every value of a struct or enum has whenever
no one is in the middle of changing it. The verifier has encoded it in three
ways. This document shows the three on one example, what each makes easy and
what each makes hard, and why the second, the certified carriers, was
withdrawn for the third. It is about the encodings, not about the history;
that is in [`transitions.md`](transitions.md).

The Lean excerpts are sketches: abridged, implicit arguments dropped, the
generated name of `Counter` written `Counter`. Encoding C is the design of
[`plain-values-semantics.md`](plain-values-semantics.md), which phase 2 of
[`plain-values.md`](plain-values.md) builds; its figures are not measured
yet.

## The example

```
struct Counter has key where
  n : u32
spec Counter where
  invariant n <= 100

fun bump(c : &mut Counter) -> Unit := do      -- writes through a reference
  (*c).n := (*c).n + 1u32
spec bump where
  requires c.n < 100

fun peek(a : address) -> u32 :=               -- reads from storage
  Counter[a].n
spec peek where
  ensures result <= 100
```

`bump` owes the invariant when its mutation ends. `peek` needs the invariant
of a value it did not make.

## The three encodings at a glance

| | A. A predicate over memory | B. In the type: certified carriers | C. Plain values, validity as a condition |
|---|---|---|---|
| In the tree | until 2026-11-05 | since 2026-11-05; withdrawn by decision of 2026-11-13, removed in phase 2 | built in phase 2 |
| A `Counter` value is | its fields | its fields with a proof of the invariant | its fields |
| The invariant is | a predicate `DataInvariants` over all of memory | part of the value's type | a predicate `Holds` on a value |
| It is owed | at a function's exit, after every write, at every join | where a value is made and where a mutable loan ends | where a value is made and where a mutable loan ends |
| It may be used | after assuming `DataInvariants` at entry, by a search | anywhere: the value carries it | where a hypothesis states it |
| It is part of the proved theorem | for stored values only | yes | yes |

## A. A predicate over memory

Values are plain. One predicate says that every stored value satisfies its
invariant, and each function preserves it.

```
def Counter.carrier := SpecInt 32 × Unit              -- the fields
def DataInvariants (s : Memory) : Prop :=
  ∀ type key value, s type key = some value → invariant type value

theorem peek.verified :
  DataInvariants s → wp (peek a) (fun r s' => r.val ≤ 100 ∧ DataInvariants s') s
```

In `peek` the fact `n ≤ 100` is not at hand. A tactic finds
`DataInvariants s` in the context and instantiates it at the cell that was
read. In `bump` the write is followed by an obligation to show the predicate
again.

**What is good.** Terms are small: a value is its fields, a field read is a
projection, and there is one spelling of everything. A program without
invariants pays nothing.

**What is bad.**

- The predicate quantifies over all of memory, so it is proved again after
  every write and at every arrival of a join, each time by search. A read
  that follows a write of the same resource type needs a case split on the
  key. On `primary_fungible_store::deposit`, three lines of Move, 1 660M of
  2 374M heartbeats went into two such leaves after 34 join arrivals had
  failed to prove the predicate.
- Only stored values are covered by the theorem. The invariant of a local
  value was a check the solver made at a marker (`constructedAt`,
  `mutationEndAt`), and a proof of the function's theorem did not have to
  pass through it.

## B. In the type: certified carriers

The invariant moves into the type. A value with an invariant is a subtype of
its plain value, and every type gets two Lean types: the plain one and the
certified one.

```
def Counter.plain   := SpecInt 32 × Unit
def Counter.carrier := { v : Counter.plain // v.fst.val ≤ 100 }

def NTy.plain   : NTy → Type          -- the shape alone
def NTy.carrier : NTy → Type          -- the shape with invariants certified
def NTy.forget      : τ.carrier → τ.plain
def NTy.ofDeepHolds : (v : τ.plain) → deepHolds τ v → τ.carrier

--  a mutable reference: the current value may be mid-change, so it is plain;
--  the value at the loan's end is certified
--  .ref τ  ↦  τ.plain × τ.carrier
```

`peek` now needs no search. Memory holds certified values, so the value read
is a `Counter.carrier` and `n ≤ 100` is its second component:

```
theorem peek.verified : wp (peek a) (fun r _ => r.val ≤ 100) s
-- leaf:  c : Counter.carrier  ⊢  c.val.fst.val ≤ 100     by c.property
```

`bump` certifies the current value where the loan ends. The step is
`certifyingAt`, whose rule is an equivalence: the invariant is owed by the
meaning of the program, not by a solver's side check.

**What is good.**

- A read owes nothing and searches for nothing. The fact travels with the
  value through calls, joins, loops and containers without anyone passing it.
- The invariant of every value, local or stored, is part of the theorem.
- A fact cannot be forgotten: a value of the certified type cannot be built
  without it.

**What is bad.** Each point is a consequence of the type, not of how it was
implemented.

1. **Every value has two spellings.** Specifications and arithmetic read the
   plain value; the program carries the certified one. `forget` and
   `ofDeepHolds` relate them, and every rule of the solver's normal form has
   to work in both. Two examples from the tree:

   ```
   -- B: the hypothesis of a slice borrow; the left side is not a variable,
   --    so it cannot be substituted, and the leaf stays open
   h : NTy.forget (.sliceRef Percent) value = SpecSliceRef.ofArrays current final
   -- C: the same fact
   h : value = SpecSliceRef.ofArrays current final          -- subst

   -- B: a projection that does not reduce, because the pair was built at
   --    `NTy.plainOf F τ` and projected at `NTy.plain τ`: equal types,
   --    different terms
   (entry, ()).fst
   ```

2. **A certified type can be empty.** Nothing inhabits
   `{ v // v.n ≥ 1 ∧ v.n = 0 }`, and nothing structural inhabits a type
   whose invariant excludes zero. But a total read needs a value when it is
   off its domain:

   ```
   ensures vector<Counter>[][17].n == 0     -- index 17 of an empty vector
   -- B: the default is a certified Counter, so every theorem carries
   --    [CarriersNonempty …] and a hypothesis that invariants are satisfiable
   -- C: the default is an unspecified plain value; nothing is claimed of it
   ```

3. **A type parameter needs both layers.** For `fun first<T>(v : &vector<T>)`
   the family that interprets `T` supplies a certified type, a plain type and
   the map between them, and instantiating `T := Counter` has to respect all
   three. That needed transports of certificates and a second route for
   generic bodies.

   ```
   -- B                               -- C
   lent  : Nat → Type                 carrier : Nat → Type
   plain : Nat → Type                 holds   : (i : Nat) → carrier i → Prop
   forget i : lent i → plain i
   ```

4. **The fact is inside the value.** A leaf that needs `n ≤ 100` has to open
   the subtype and unfold `deepHolds` to reach it. That was 12 to 24% of the
   cost of the `OrderedMap` scenarios. As a hypothesis it stays folded until
   a step asks for it.

5. **The type hides the order of checks.** At the end of a loan the current
   value is equated with the certified value promised when the loan began.
   Because that value is certified by its type, stating the equation looks
   harmless, and the carriers stated it before checking the current value.
   An equation with a certified side can remove the very run in which the
   check fails. With validity as a step, the step comes first by
   construction.

6. **Not every invariant fits in a type.** One that reads memory or the
   executing unit cannot be a subtype, so encoding A stayed in the tree
   beside B.

Measured on the benchmark's `OrderedMap`: 757M heartbeats before, 2 872M
with the carriers as merged, 15 of 17 targets. The nominal invariant sorts, a
variant of B with a generated sort per declaration in place of the subtype,
reached 1 143M at the price of about 100M of setup per unit, and kept the
two layers of a value.

## C. Plain values, validity as a condition

Values are plain, as in A. The invariant is a predicate on a value, as local
as in B. The places that owe it are B's, and so is the rule that makes it
part of the theorem.

```
def Counter.carrier := SpecInt 32 × Unit
def NTy.Holds : (τ : NTy) → τ.carrier → Prop            -- Counter: v.fst.val ≤ 100

--  a producer checks, and the check is the step's meaning
wp (checkingAt site τ v k) post s  ↔  NTy.Holds τ v ∧ wp (k v) post s

--  a receiver gets a hypothesis
--    arguments and results:   Contract.validArgs, Contract.validResult
--    a prophecy:              ∀ final, NTy.Holds τ final → …
--    a mutable reference:     τ.carrier × τ.carrier
--    storage:                 a cell holds { v : τ.carrier // NTy.Holds τ v },
--                             and a read returns v with the theorem Holds τ v
```

`bump` and `peek` reach the same leaves as under B:

```
-- bump, at the end of the loan:   ⊢ (c.fst.fst.val + 1) % 2^32 ≤ 100
-- peek:   h : NTy.Holds Counter c   ⊢ c.fst.val ≤ 100
```

**What is good.** One spelling of every value; no empty types, so no
inhabitant hypotheses; a type parameter is a type and a predicate; facts are
hypotheses that stay folded; the order of check and equation is visible; and
the theorem is as strong as B's.

**What is bad.** What B's typing did silently now has to be done.

- A join or a loop that abstracts over a value loses its fact unless the
  fact is passed: the shared continuation takes a bundle of `Holds` premises,
  and each arrival proves its own.
- A collection operation needs a lemma that it keeps `Holds` (`push`,
  `insert`, `swap`, `slice`).
- A receipt that is forgotten is a fact that is missing, and a target that
  verified stops verifying. It cannot make a wrong program verify: every
  producer still checks.
- Storage keeps one subtype, at the cell and nowhere inside a value, so that
  a read's fact is a theorem as in B and not a search as in A.

## Side by side

| | A. Memory predicate | B. Certified carriers | C. Plain with `Holds` |
|---|---|---|---|
| Reading a stored value's invariant | search of the context, case split on the key | free | a theorem of the read |
| A local value's invariant | checked by the solver, not in the theorem | in the theorem | in the theorem |
| After a write | the whole predicate again | the written value, where its loan ends | the written value, where its loan ends |
| At a join | proved again per arrival | nothing: the type carries it | a premise per value that varies |
| Spellings of one value | one | two, with maps between them | one |
| Empty types | none | possible; hypotheses on every theorem | none |
| Type parameters | a type | two types and a map | a type and a predicate |
| Using a fact at a leaf | instantiate the predicate | open the subtype | open a hypothesis |
| Order of check and equation | visible | hidden by the type | visible |
| Invariants that read memory | yes | no; A kept beside it | as conditions, to be designed |
| What can go wrong silently | a local value's check not in the theorem | a rule that erases a layer | a fact not passed (incomplete, not unsound) |
| `OrderedMap`, heartbeats | 757M | 2 872M | target: 757M |

## The reasons, in three sentences

Encoding A was withdrawn because a predicate over all of memory is proved
again wherever memory may have changed. Encoding B removed that and was
withdrawn because a certificate inside a value gives every value two
spellings, makes types possibly empty and doubles what a type parameter
means, and a solver that works by normal forms pays for each of these at
every step. Encoding C keeps what B got right, where the invariant is owed
and that it is part of the theorem, and states the fact beside the value
instead of inside it.

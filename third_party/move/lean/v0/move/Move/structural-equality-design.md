# Structural equality — current status and proposed proof law

## Runtime status

Move bytecode `Eq` compares reference-free values structurally. Leaner lowers
the compiler marker `Move.Compare.equal` to that operation, and the generic
low-priority `BEq` instance makes `left == right` available for Move values.
Types with a host `DecidableEq` may otherwise cause Lean to synthesize a
different `BEq` implementation, so `Address` has an exact instance selecting
the compiler marker.

This is enough to compile and execute address equality correctly. The
transactional address test checks both the true and false cases on the
production VM.

## Remaining verification-model gap

The source proof model does not yet provide a general theorem connecting the
opaque marker to Lean equality. `Move.Compare.LawfulEq T` exposes such a law,
but callers must currently supply the instance:

```lean
class LawfulEq (T : Type) : Prop where
  eq_iff : ∀ left right : T,
    Move.Compare.equal left right = true ↔ left = right
```

Unsigned integers have a dedicated trusted law in `Move.Verify.Syntax`.
`Address`, signed integers, `Bool`, vectors, structs, and enums do not yet gain
an unconditional proof law merely because their runtime equality compiles.
Contracts that restate their result with `==` are still provable by
reflexivity; contracts that replace `== true` with Lean equality need a law.

References are the important boundary. VM equality dereferences references
before comparing their targets, while the host representation of a mutable
reference also contains proof-model state. A blanket law over every Lean type
would therefore be unsound.

## Proposed proof interface

A faithful follow-up is a data-free marker for reference-free Move value
types, with one trust-base theorem:

```lean
namespace Move.Compare

class Structural (T : Type) : Prop

axiom equal_eq_true_iff_of_structural
    {T : Type} [Structural T] (left right : T) :
    equal left right = true ↔ left = right

instance [Structural T] : LawfulEq T :=
  ⟨equal_eq_true_iff_of_structural⟩

end Move.Compare
```

Instances should cover `MoveInt S W`, `Bool`, `Address`, `Unit`, and
`Vector T` when `T` is structural. The module expander can emit instances for
declared structs and enums. It must not emit them for `Ref`, `MutRef`, or
`Signer` without first defining a source equality model that matches VM
dereferencing.

Generic functions may continue to spell `[Compare.LawfulEq T]` or
`[Compare.Total T]`. Automatically inserting structural constraints changes
the signatures that generic `spec` declarations must match, so that should be
a separate compatibility decision.

## Scope

The `Structural` proof law described above is a proposal, not an implemented
claim. Runtime structural equality and address equality lowering are
implemented. This distinction keeps the trusted verification surface explicit
and avoids documenting axioms or generated instances that are not present in
the repository.

# Structural equality — a modeling gap and its fix

## The finding

In the Move VM, `==` is the `Eq` bytecode: it compares two **ref-free values
structurally**.  The IR models exactly that:

```lean
-- MoveModel/IR/Semantics.lean:130
| .eq, [v₁, v₂], m => do
    let a ← v₁.derefWith deref
    let b ← v₂.derefWith deref
    if a.refFree && b.refFree && a.sameTypeShape b then
      pure (.ok [.bool (a == b)] m) else none
```

Runtime equality *is* equality of the modeled value.  The **source** model does
not say so.  `Compare.equal` is an opaque marker (`Basic.lean:178`) and its law
lives in a class the caller must supply:

```lean
-- Verify/Compare.lean:23
class LawfulEq (T : Type) : Prop where
  eq_iff : ∀ left right : T, equal left right = true ↔ left = right
```

**`LawfulEq` has no instance anywhere in the repository.**  Consequently:

| `==` on | law available today |
|---|---|
| `U64` and the other `UInt W` | yes — axiom `logicalBEq_uint` (`Verify/Syntax.lean:108`) |
| `I8` … `I256` | **none** |
| `Bool`, `Address`, a `struct`, an `enum`, a `Vector` | **none** |
| a type parameter `{T}` | only if the spec author writes `[Compare.Total T]` |

So a contract as ordinary as `ensures result = true ↔ addr = @admin` is not
provable for any type but the unsigned integers, and the author's only recourse
is to carry an instance assumption that stands for a fact the VM guarantees
outright.

The suite does not catch this because every equality test states its
postcondition *as* `==` — `ensures result = (left == right)`
(`Tests/Generics.lean:71-105`) — which is reflexivity.  The content of equality
is never used.

This is also inconsistent with the treatment of *ordering* one screen above in
the same file: `MoveInt.less` / `lessEq` get unconditional trust-base axioms
(`Basic.lean:460,472`), no class and no assumption.  Equality needs strictly
*less* information than an order — it is fixed by the value domain alone,
whereas a structural order also needs the type's field order — yet it carries
more ceremony.

## The one real side condition: references

The law cannot be stated for every Lean type.  `MutRef α` carries `current` and
`original` (`Basic.lean:235`), so its host equality is **finer** than the VM's:
`.eq` dereferences its operands first and compares only the targets.  `Ref.get`
is opaque, so `Ref` is no better.  The IR's own guard is `refFree`; the source
side needs the same restriction.

(A model-domain phrasing — `equal l r = true ↔ l↑ = r↑` — would be faithful for
references, but it is unsafe: `ModelDomain` instances are user-extensible, and
an axiom quantified over a class *with data* is inconsistent as soon as someone
provides a non-injective projection.  A marker class carries no data and has no
such failure mode.)

## The fix

```lean
namespace Move.Compare

/-- `T`'s Lean equality is Move's structural equality: `T` is a ref-free Move
value type.  Deliberately no instance for `Ref`/`MutRef`, whose host equality
is finer than the `Eq` bytecode's. -/
class Structural (T : Type) : Prop

/-- Trust base: the `Eq` bytecode compares ref-free values structurally
(`MoveModel.IR.Oper.sem .eq`), so the marker agrees with equality of the source
value.  Companion of `MoveInt.less_eq_true_iff`. -/
axiom equal_eq_true_iff_of_structural {T : Type} [Structural T] (l r : T) :
    equal l r = true ↔ l = r

instance [Structural T] : LawfulEq T := ⟨equal_eq_true_iff_of_structural⟩

end Move.Compare
```

Instances:

- `MoveInt S W`, `Bool`, `Address`, `Unit`; `Vector α` from `Structural α`.
- Per declared `struct` / `enum`: emitted by the `module` expander next to the
  ability instances it already generates (`Export.lean`, `abilityCommands`).
- Type parameters: **not** auto-added to a bare `{T}` binder, though
  `addMoveTypeInhabitants` (`Export.lean:232`) does exactly that for
  `Inhabited`.  A `fun`'s binders are auto-added while a `spec`'s are spelled
  by the author, so adding one there changes the signature every existing
  generic `spec` must match.  Generic source keeps spelling
  `[Compare.LawfulEq T]` / `[Compare.Total T]` as it does today — the
  difference is that those binders are now *dischargeable* at a concrete
  instantiation, which before they were not.  Auto-adding the binder is a
  separate change.
- **Not** `Ref`, `MutRef`, `Signer` (`Signer.address` is uninterpreted; signer
  equality is not modeled).

Consequences:

- `logicalBEq_uint` stops being an axiom and becomes a theorem — host equality
  of `MoveInt` is `toInt` equality by `MoveInt.ext`.  Net axiom count is
  unchanged: one general statement replaces one width-specific one, and covers
  the signed widths and every other Move type that has nothing today.
- `Compare.LawfulEq` survives as the proof interface but is now *derived*, so
  existing `[Compare.Total T]` binders keep working untouched.
- `Compare.Lawful` / `Total` keep their **ordering** laws as assumptions.  That
  asymmetry is now principled rather than accidental: a structural order depends
  on the type's field order, which the source model does not fix; equality does
  not.

## Implemented

Landed as described, with three refinements found while building it:

- **`MoveInt.equal` had no law either.**  `Basic.lean` axiomatized `less` and
  `lessEq` but not `equal`, so `==` on an integer had no content in a pure
  body.  Rather than add a third integer axiom, `MoveInt.equal` now *is* the
  generic marker (`def equal := fun a b => Move.Compare.equal a b`, still
  `@[noinline]` so the backend keeps recognizing it): equality is one
  instruction in the VM, so it is one marker in the source model.  Unlike
  `less`/`lessEq`, whose instruction really is numeric.
- **`==` in a pure body never exposed the marker.**  It elaborates to
  `BEq.beq` at `Compare.genericBEq` or at `MoveInt`'s instance, which no simp
  lemma matched.  Two `@[simp] … := rfl` bridges (`Compare.beq_eq_equal`,
  `MoveInt.beq_eq_equal`) put a pure body's `==` into the same form the
  contract's `logicalBEq` has.
- **`logicalBEq_structural`** carries the law to the contract spelling, at
  `simp low` so an integer still normalizes into its model domain through
  `logicalBEq_uint` (now a theorem).

Net axiom count is unchanged — four before, four after — but the equality
axiom now covers every reference-free Move type instead of only `UInt W`:

```
Move.Compare.equal_eq_true_iff_of_structural   -- new, general
Move.MoveInt.less_eq_true_iff                  -- unchanged
Move.MoveInt.lessEq_eq_true_iff                -- unchanged
Move.Verify.logicalLT_uint                     -- unchanged
Move.Verify.logicalBEq_uint                    -- gone: now a theorem
```

Regression coverage in `Tests/Generics.lean`: `same_u64`, `same_bools`,
`same_boxes` state `result = true ↔ left = right` — the content, not the
operator — and `#print axioms` on their proofs shows exactly the one new
axiom.

## Companion (axiom-free, in the model)

`Value` is the only type in `IR/Value.lean` that derives `BEq, Ord, Repr` but
not `DecidableEq, ReflBEq, LawfulBEq` (line 432, against 34/168/305/322/348/360)
— the deriving handlers do not apply through its nested `List Value` recursion
(reproduced on 4.32.2: *"None of the deriving handlers for class `DecidableEq`
applied"*).  So even inside the model, `.eq`'s result `a == b` cannot be turned
into `a = b`.

Proving `Value.beq_iff_eq` by mutual induction over `Value` / `List Value` and
adding `DecidableEq` / `LawfulBEq` instances closes that with no axiom, and
makes the statement the source-level trust-base axiom corresponds to —
*the `Eq` bytecode decides value equality* — a theorem rather than a reading.

## Adjacent gaps (same root cause, not fixed here)

`logicalLT_uint` (`Verify/Syntax.lean:71`) is `UInt`-only, and `logicalLE` is
*defined* only for `Move.UInt W` (`:88`), so a generated source spec cannot even
express `a <= b` on a signed integer.  Ordering for `I8` … `I256` should get the
same treatment via `MoveInt.less_eq_true_iff` / `lessEq_eq_true_iff`, which
already hold for both signednesses.

## Rollout

Landed before the address work: `addr == @admin` is the canonical framework
idiom, and without this fix an address test would have to carry
`[Compare.LawfulEq Address]`.

1. ✅ `Compare.Structural`, the axiom, the derived `LawfulEq` instance, and the
   primitive / `Vector` instances.
2. ✅ Generated instances for every declared `struct` / `enum`.
3. ✅ `logicalBEq_uint` demoted to a theorem; equality-content regressions.
4. Open, independent: `Value.beq_iff_eq` and the model-side instances.
5. Open, independent: the signed-integer `<` / `≤` gap below.

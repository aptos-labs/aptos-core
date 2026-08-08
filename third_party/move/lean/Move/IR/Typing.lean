-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move.IR.Value

/-!
# Types and Value Typing: `IsValid` over the Declared Types

The declared types of the supported fragment (`Ty`, `StructDecl`) and the
semantic counterpart of the Boogie prelude's `$IsValid'T'` predicates — the
*multisorted* discipline of the Move Prover's encoding.  The dynamic
semantics stays untyped (Move Prover verifies bytecode, whose typing is
established by the bytecode verifier upstream); instead, well-formedness of
values for the *declared* types is a predicate, and the translation injects
it exactly where the real prover injects `WellFormed` assumptions:

* at function entry, for the arguments (`TypedArgs`) and for global memory
  (`TypedMemory`, both in `Syntax.lean`);
* at loop headers, for the havocked loop targets (havoc in a multisorted
  logic ranges over the sort — a havocked local is *defined* and
  well-formed for its declared type);
* at call sites, for the callee's results and the memory it may modify;
* at quantifiers, whose declared domain type bounds the range
  (`EvalSpec` in `Spec.lean`).

`u64` carries the range refinement (`n < U64_SIZE`); `signer` values are
addresses; an *immutable*-reference value is its target (dereferenced at
function boundaries in the prover's encoding); a *mutable*-reference slot
holds a mutation (`Value.mut`) with a well-formed payload — in the
eliminated world `&mut` is the mutation datum; a struct value must match
its declaration's field types pointwise; a vector value has well-formed
elements and a length representable as `u64` (its length *is* a `u64` in
Move, and the VM aborts rather than exceed it).  `IsValid` is an inductive
(least) predicate, so ill-founded recursive struct declarations — which
Move's type system excludes — denote empty types.
-/

namespace Move.IR

/-- Move types of the supported fragment.  Types are *declarations*: the
dynamic semantics is untyped (an ill-typed configuration is stuck), and
typing assumptions enter verification as well-formedness conditions over
the boundary state.  `signer` values are addresses.  Reference-typed
*parameters* are dereferenced at the boundary (`IsValid`); references in
code execute (see `Semantics.lean`) and are verified after reference
elimination (`RefElim.lean`). -/
inductive Ty where
  | bool
  | u64
  | address
  | signer
  | struct (r : ResourceId)
  | vector (t : Ty)
  | ref (t : Ty)
  | mutRef (t : Ty)
  deriving BEq, Repr

/-- A struct declaration: the field types in offset order. -/
structure StructDecl where
  fields : List Ty

/-- The struct declarations of a program, as a partial map. -/
abbrev StructDecls := ResourceId → Option StructDecl

mutual

/-- `IsValid Δ t v`: the runtime value `v` is well-formed at the declared
type `t`, relative to the struct declarations `Δ` (see module docs). -/
inductive IsValid (Δ : StructDecls) : Ty → Value → Prop where
  | bool (b : Bool) : IsValid Δ .bool (.bool b)
  | u64 {n : Nat} : n < U64_SIZE → IsValid Δ .u64 (.u64 n)
  | address (a : Address) : IsValid Δ .address (.address a)
  | signer (a : Address) : IsValid Δ .signer (.address a)
  | ref {t : Ty} {v : Value} : IsValid Δ t v → IsValid Δ (.ref t) v
  | mutRef {t : Ty} {rt : RefTarget} {v : Value} :
      IsValid Δ t v → IsValid Δ (.mutRef t) (.mut rt v)
  | struct {r : ResourceId} {d : StructDecl} {fs : List Value} :
      Δ r = some d → IsValidList Δ d.fields fs →
      IsValid Δ (.struct r) (.struct fs)
  | vector {t : Ty} {es : List Value} :
      es.length < U64_SIZE → (∀ v ∈ es, IsValid Δ t v) →
      IsValid Δ (.vector t) (.vector es)

/-- Pointwise well-formedness of a value list at a type list. -/
inductive IsValidList (Δ : StructDecls) : List Ty → List Value → Prop where
  | nil : IsValidList Δ [] []
  | cons {t : Ty} {v : Value} {ts : List Ty} {vs : List Value} :
      IsValid Δ t v → IsValidList Δ ts vs → IsValidList Δ (t :: ts) (v :: vs)

end

/-! ## Inversion pack

`iff` characterizations of `IsValid` at each type constructor, for `simp`. -/

@[simp] theorem isValid_bool_iff {Δ : StructDecls} {v : Value} :
    IsValid Δ .bool v ↔ ∃ b, v = .bool b := by
  constructor
  · intro h; cases h with | bool b => exact ⟨b, rfl⟩
  · rintro ⟨b, rfl⟩; exact .bool b

@[simp] theorem isValid_u64_iff {Δ : StructDecls} {v : Value} :
    IsValid Δ .u64 v ↔ ∃ n, v = .u64 n ∧ n < U64_SIZE := by
  constructor
  · intro h; cases h with | u64 h => exact ⟨_, rfl, h⟩
  · rintro ⟨n, rfl, h⟩; exact .u64 h

@[simp] theorem isValid_address_iff {Δ : StructDecls} {v : Value} :
    IsValid Δ .address v ↔ ∃ a, v = .address a := by
  constructor
  · intro h; cases h with | address a => exact ⟨a, rfl⟩
  · rintro ⟨a, rfl⟩; exact .address a

@[simp] theorem isValid_signer_iff {Δ : StructDecls} {v : Value} :
    IsValid Δ .signer v ↔ ∃ a, v = .address a := by
  constructor
  · intro h; cases h with | signer a => exact ⟨a, rfl⟩
  · rintro ⟨a, rfl⟩; exact .signer a

@[simp] theorem isValid_ref_iff {Δ : StructDecls} {t : Ty} {v : Value} :
    IsValid Δ (.ref t) v ↔ IsValid Δ t v := by
  constructor
  · intro h; cases h with | ref h => exact h
  · exact .ref

@[simp] theorem isValid_mutRef_iff {Δ : StructDecls} {t : Ty} {v : Value} :
    IsValid Δ (.mutRef t) v ↔
      ∃ rt w, v = .mut rt w ∧ IsValid Δ t w := by
  constructor
  · intro h
    cases h with
    | mutRef h => exact ⟨_, _, rfl, h⟩
  · rintro ⟨rt, w, rfl, h⟩
    exact .mutRef h

@[simp] theorem isValid_struct_iff {Δ : StructDecls} {r : ResourceId}
    {v : Value} :
    IsValid Δ (.struct r) v ↔
      ∃ d fs, Δ r = some d ∧ v = .struct fs ∧ IsValidList Δ d.fields fs := by
  constructor
  · intro h; cases h with | struct hd hfs => exact ⟨_, _, hd, rfl, hfs⟩
  · rintro ⟨d, fs, hd, rfl, hfs⟩; exact .struct hd hfs

/-- The payload of a mutation valid at `&mut t` is valid at `t`. -/
theorem IsValid.mutRef_payload {Δ : StructDecls} {t : Ty} {rt : RefTarget}
    {w : Value} (h : IsValid Δ (.mutRef t) (.mut rt w)) : IsValid Δ t w := by
  cases h with
  | mutRef h => exact h

@[simp] theorem isValid_vector_iff {Δ : StructDecls} {t : Ty} {v : Value} :
    IsValid Δ (.vector t) v ↔
      ∃ es, v = .vector es ∧ es.length < U64_SIZE ∧
        ∀ w ∈ es, IsValid Δ t w := by
  constructor
  · intro h; cases h with | vector hlen hes => exact ⟨_, rfl, hlen, hes⟩
  · rintro ⟨es, rfl, hlen, hes⟩; exact .vector hlen hes

/-- Reference values are never well-formed at any declared type: the
declared reference types are *transparent* (a value valid at `&t` is a
value valid at `t`), so a `Value.ref` would need an infinite derivation.
Well-typed boundaries therefore never pass reference values. -/
theorem IsValid.not_ref {Δ : StructDecls} :
    ∀ {t : Ty} {rt : RefTarget}, ¬ IsValid Δ t (.ref rt) := by
  intro t
  induction t with
  | bool => intro rt h; cases h
  | u64 => intro rt h; cases h
  | address => intro rt h; cases h
  | signer => intro rt h; cases h
  | struct r => intro rt h; cases h
  | vector t ih => intro rt h; cases h
  | ref t ih => intro rt h; cases h with | ref h => exact ih h
  | mutRef t ih => intro rt h; cases h

@[simp] theorem isValidList_nil_iff {Δ : StructDecls} {vs : List Value} :
    IsValidList Δ [] vs ↔ vs = [] := by
  constructor
  · intro h; cases h with | nil => rfl
  · rintro rfl; exact .nil

@[simp] theorem isValidList_cons_iff {Δ : StructDecls} {t : Ty}
    {ts : List Ty} {vs : List Value} :
    IsValidList Δ (t :: ts) vs ↔
      ∃ v vs', vs = v :: vs' ∧ IsValid Δ t v ∧ IsValidList Δ ts vs' := by
  constructor
  · intro h; cases h with | cons hv hvs => exact ⟨_, _, rfl, hv, hvs⟩
  · rintro ⟨v, vs', rfl, hv, hvs⟩; exact .cons hv hvs

end Move.IR

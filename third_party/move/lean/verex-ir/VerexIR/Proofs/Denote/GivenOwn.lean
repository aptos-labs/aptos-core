-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Interpretation

namespace VerexIR.Proofs.Denote

private theorem congrArg2 {α : Type u} {β : Type v} {γ : Type w} (f : α → β → γ)
    {a₁ a₂ : α} {b₁ b₂ : β} (h₁ : a₁ = a₂) (h₂ : b₁ = b₂) : f a₁ b₁ = f a₂ b₂ :=
  h₁ ▸ h₂ ▸ rfl

mutual
theorem NTy.carrier_withOwn (base : Carriers)
    (own : StructHandle → NRow → RuntimeValue → Prop) : (type : NTy) →
    @NTy.carrier { base with own } type = @NTy.carrier base type
  | .unit | .bool | .int _ _ | .num | .address | .signer | .string
  | .bytes | .char | .param _ | .function _ _ _ => rfl
  | .nominal source arguments productive =>
      congrArg (base.nominals.nominal source productive) (NRow.sorts_withOwn base own arguments)
  | .tuple elements | .struct _ _ elements => HList.carrier_withOwn base own elements
  | .enum _ _ names rows _ => variantCarrier.carrier_withOwn base own names rows
  | .slice element | .vector element =>
      congrArg SpecVector (NTy.carrier_withOwn base own element)
  | .array element length =>
      congrArg (SpecArray · length) (NTy.carrier_withOwn base own element)
  | .table key value =>
      congrArg SpecEntries (congrArg2 Prod (NTy.carrier_withOwn base own key)
        (NTy.carrier_withOwn base own value))
  | .ghost slot => NTy.carrier_withOwn base own slot
  | .observed referent => NTy.carrier_withOwn base own referent
  | .sliceRef element =>
      congrArg SpecSliceRef (NTy.carrier_withOwn base own element)
  | .ref referent =>
      let same := NTy.carrier_withOwn base own referent
      congrArg2 Prod same same

theorem NRow.sorts_withOwn (base : Carriers)
    (own : StructHandle → NRow → RuntimeValue → Prop) : (row : NRow) →
    @NRow.sorts { base with own } row = @NRow.sorts base row
  | .nil => rfl
  | .cons τ rest =>
      congrArg2 List.cons (NTy.carrier_withOwn base own τ) (NRow.sorts_withOwn base own rest)

theorem HList.carrier_withOwn (base : Carriers)
    (own : StructHandle → NRow → RuntimeValue → Prop) : (row : NRow) →
    @HList { base with own } row = @HList base row
  | .nil => rfl
  | .cons τ rest =>
      congrArg2 Prod (NTy.carrier_withOwn base own τ) (HList.carrier_withOwn base own rest)

theorem variantCarrier.carrier_withOwn (base : Carriers)
    (own : StructHandle → NRow → RuntimeValue → Prop) :
    (names : List String) → (rows : NRows) →
    @variantCarrier { base with own } names rows = @variantCarrier base names rows
  | _, .nil | [], .cons _ _ => rfl
  | _ :: names, .cons fields rest =>
      congrArg2 Sum (HList.carrier_withOwn base own fields)
        (variantCarrier.carrier_withOwn base own names rest)
end

/-- The own-independent family used only while translating declaration
clauses. It is never the family in which a function is verified: that family
installs the resulting declarations with `withDeclarations`. -/
@[reducible] def GivenFrame.clauseScaffold (frame : GivenFrame) : Carriers :=
  { frame.toCarriers with own := fun _ _ _ => True }

theorem GivenFrame.clauseScaffold_carrier (frame : GivenFrame) (type : NTy) :
    @NTy.carrier frame.clauseScaffold type = @NTy.carrier frame.toCarriers type :=
  NTy.carrier_withOwn frame.toCarriers _ type

/-- One declaration's clauses at an already substituted type in the given
parameter sorts. Clauses read plain carrier values, so their interpretation does
not depend on the declared invariants that they will define. -/
structure GivenOwnClause (frame : GivenFrame) where
  type : NTy
  holds : @NTy.carrier frame.toCarriers type → Prop

/-- The declaration clauses of a decoded value. Invalid encodings impose no
condition: a typed pack always supplies an encoding, whose decoding is exact. -/
def GivenOwnClause.atRuntime {frame : GivenFrame} (clause : GivenOwnClause frame)
    (raw : RuntimeValue) : Prop :=
  ∀ value, (@NTy.codec frame.toCarriers clause.type).decode? raw = some value →
    clause.holds value

theorem GivenOwnClause.atRuntime_encode {frame : GivenFrame}
    (clause : GivenOwnClause frame) (value : @NTy.carrier frame.toCarriers clause.type) :
    clause.atRuntime (@NTy.encode frame.toCarriers clause.type value) ↔
      clause.holds value := by
  letI : Carriers := frame.toCarriers
  simp [atRuntime]

/-- A unit supplies clauses only for declarations that state an own invariant.
The generator substitutes each declaration schema in the same given frame. -/
abbrev GivenOwnDeclarations (frame : GivenFrame) :=
  StructHandle → NRow → Option (GivenOwnClause frame)

/-- The raw own predicate selected by the declaration and its type arguments.
Nested field invariants are supplied separately by `NTy.deepHolds`. -/
def GivenOwnDeclarations.own {frame : GivenFrame}
    (declarations : GivenOwnDeclarations frame) (source : StructHandle)
    (arguments : NRow) (raw : RuntimeValue) : Prop :=
  match declarations source arguments with
  | none => True
  | some clause => clause.atRuntime raw

/-- Install generated declaration predicates without changing the given sorts,
their codecs, or their parameter well-formedness. The input frame's arbitrary
own predicate is not inherited by this verification family. -/
@[reducible] def GivenFrame.withDeclarations (frame : GivenFrame)
    (declarations : GivenOwnDeclarations frame) : GivenFrame where
  operations := { frame.operations with own := declarations.own }

theorem GivenFrame.withDeclarations_carrier (frame : GivenFrame)
    (declarations : GivenOwnDeclarations frame) (type : NTy) :
    @NTy.carrier (frame.withDeclarations declarations).toCarriers type =
      @NTy.carrier frame.toCarriers type :=
  NTy.carrier_withOwn frame.toCarriers _ type

theorem GivenFrame.withDeclarations_own (frame : GivenFrame)
    (declarations : GivenOwnDeclarations frame) (source : StructHandle)
    (arguments : NRow) (raw : RuntimeValue) :
    @Carriers.own (frame.withDeclarations declarations).toCarriers source arguments raw =
      declarations.own source arguments raw := rfl

theorem GivenFrame.withDeclarations_own_none (frame : GivenFrame)
    (declarations : GivenOwnDeclarations frame) (source : StructHandle)
    (arguments : NRow) (absent : declarations source arguments = none)
    (raw : RuntimeValue) :
    @Carriers.own (frame.withDeclarations declarations).toCarriers source arguments raw := by
  change declarations.own source arguments raw
  simp [GivenOwnDeclarations.own, absent]

theorem GivenFrame.withDeclarations_own_encode (frame : GivenFrame)
    (declarations : GivenOwnDeclarations frame) (source : StructHandle)
    (arguments : NRow) (clause : GivenOwnClause frame)
    (selected : declarations source arguments = some clause)
    (value : @NTy.carrier frame.toCarriers clause.type) :
    @Carriers.own (frame.withDeclarations declarations).toCarriers source arguments
      (@NTy.encode frame.toCarriers clause.type value) ↔
      clause.holds value := by
  change declarations.own source arguments
    (@NTy.encode frame.toCarriers clause.type value) ↔ _
  simp only [GivenOwnDeclarations.own, selected]
  exact clause.atRuntime_encode value

end VerexIR.Proofs.Denote

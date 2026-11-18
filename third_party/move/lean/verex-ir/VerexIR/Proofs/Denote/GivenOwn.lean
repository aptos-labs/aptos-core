-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Interpretation

namespace VerexIR.Proofs.Denote

/-- The own-independent family used only while translating declaration
clauses. It starts uninstalled, as does the verification stage that selects
the resulting own predicates with `withDeclarations`. -/
@[reducible] def GivenFrame.clauseScaffold (frame : GivenFrame) : Carriers :=
  { frame.toCarriers with own := fun _ _ _ => True }

/-- One declaration's clauses at an already substituted type in the given
parameter sorts. Clauses read plain carrier values, so their interpretation does
not depend on the declared invariants that they will define. -/
structure GivenOwnClause (frame : GivenFrame) where
  type : NTy
  holds : @NTy.carrier (frame.toCarriers).toSorts type → Prop

/-- The declaration clauses of a decoded value. Invalid encodings impose no
condition: a typed pack always supplies an encoding, whose decoding is exact. -/
def GivenOwnClause.atRuntime {frame : GivenFrame} (clause : GivenOwnClause frame)
    (raw : RuntimeValue) : Prop :=
  ∀ value, (@NTy.codec (frame.toCarriers).toSorts clause.type).decode? raw = some value →
    clause.holds value

theorem GivenOwnClause.atRuntime_encode {frame : GivenFrame}
    (clause : GivenOwnClause frame) (value : @NTy.carrier (frame.toCarriers).toSorts clause.type) :
    clause.atRuntime (@NTy.encode (frame.toCarriers).toSorts clause.type value) ↔
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

/-- Select the generated own predicates without changing the given sorts,
codecs or parameter well-formedness. The resulting stage has uninstalled
declared validity because the selected own predicate can differ. -/
@[reducible] def GivenFrame.withDeclarations (frame : GivenFrame)
    (declarations : GivenOwnDeclarations frame) : GivenFrame where
  operations := { frame.operations with
    own := declarations.own
    declared := fun _ _ _ _ => False }

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
    (value : @NTy.carrier (frame.toCarriers).toSorts clause.type) :
    @Carriers.own (frame.withDeclarations declarations).toCarriers source arguments
      (@NTy.encode (frame.toCarriers).toSorts clause.type value) ↔
      clause.holds value := by
  change declarations.own source arguments
    (@NTy.encode (frame.toCarriers).toSorts clause.type value) ↔ _
  simp only [GivenOwnDeclarations.own, selected]
  exact clause.atRuntime_encode value

end VerexIR.Proofs.Denote

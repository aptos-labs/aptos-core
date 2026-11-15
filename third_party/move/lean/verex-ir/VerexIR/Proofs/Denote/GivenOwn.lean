-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Interpretation

namespace VerexIR.Proofs.Denote

/-- The own-independent family used only while translating declaration
clauses. It is never the family in which a function is verified: that family
installs the resulting declarations with `withDeclarations`. -/
@[reducible] def GivenFrame.clauseScaffold (frame : GivenFrame) : Carriers :=
  { frame.toCarriers with own := fun _ _ _ => True }

/-- Clause translation may choose values without assuming the declarations
it is defining. Given parameter certification is the identity; no parameter invariant
or actual nominal invariant is introduced into this scaffold. -/
@[reducible] def GivenFrame.clauseScaffoldNonempty (frame : GivenFrame) :
    CarriersNonempty frame.clauseScaffold :=
  @CarriersNonempty.ofTrivial frame.clauseScaffold
    (fun _ _ _ => trivial) (fun _ _ => trivial)
    (fun _ value => value)

theorem GivenFrame.clauseScaffold_toPlain (frame : GivenFrame) :
    @Carriers.toPlain frame.clauseScaffold = @Carriers.toPlain frame.toCarriers := rfl

/-- One declaration's clauses at an already substituted type in the given
parameter sorts. Clauses read plain values, so their interpretation does not
depend on the nominal certificates that they will define. -/
structure GivenOwnClause (frame : GivenFrame) where
  type : NTy
  holds : @NTy.plain frame.toCarriers type → Prop

/-- The declaration clauses of a decoded value. Invalid encodings impose no
condition: a typed pack always supplies an encoding, whose decoding is exact. -/
def GivenOwnClause.atRuntime {frame : GivenFrame} (clause : GivenOwnClause frame)
    (raw : RuntimeValue) : Prop :=
  ∀ value, (@NTy.plainCodec frame.toCarriers clause.type).decode? raw = some value →
    clause.holds value

theorem GivenOwnClause.atRuntime_encode {frame : GivenFrame}
    (clause : GivenOwnClause frame) (value : @NTy.plain frame.toCarriers clause.type) :
    clause.atRuntime (@NTy.plainEncode frame.toCarriers clause.type value) ↔
      clause.holds value := by
  simp [atRuntime, NTy.plainEncode]

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

theorem GivenFrame.withDeclarations_toPlain (frame : GivenFrame)
    (declarations : GivenOwnDeclarations frame) :
    @Carriers.toPlain (frame.withDeclarations declarations).toCarriers =
      @Carriers.toPlain frame.toCarriers := rfl

theorem GivenFrame.withDeclarations_plainCodec (frame : GivenFrame)
    (declarations : GivenOwnDeclarations frame) (type : NTy) :
    @NTy.plainCodec (frame.withDeclarations declarations).toCarriers type =
      @NTy.plainCodec frame.toCarriers type := rfl

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
    (value : @NTy.plain frame.toCarriers clause.type) :
    @Carriers.own (frame.withDeclarations declarations).toCarriers source arguments
      (@NTy.plainEncode (frame.withDeclarations declarations).toCarriers clause.type value) ↔
      clause.holds value := by
  change declarations.own source arguments
    (@NTy.plainEncode frame.toCarriers clause.type value) ↔ _
  simp only [GivenOwnDeclarations.own, selected]
  exact clause.atRuntime_encode value

end VerexIR.Proofs.Denote

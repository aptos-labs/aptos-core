-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Resources

open VerexIR VerexIR.Proofs.Denote

private def source : StructHandle := ⟨⟨0⟩, 0⟩
private def kind : NominalKind := ⟨true, [], .owned, true⟩
private def arguments : NRow := .cons (.int 8 false) .nil
private def substitution : TypeArgs := ⟨arguments, rfl⟩
private def parameterRow : NRow := .cons (.param 0) .nil

private def declared (s : StructHandle) (a : NRow) (k : NominalKind)
    (raw : RuntimeValue) : Prop :=
  s = source ∧ a = arguments ∧ k = kind ∧ raw = .bool true

@[reducible] private def family : Carriers :=
  { Carriers.ground with declared := declared }

-- An omitted installation is closed, both on a standalone family and a unit.
example (s : StructHandle) (a : NRow) (k : NominalKind) (raw : RuntimeValue) :
    Carriers.ground.declared s a k raw = False := rfl

example (unit : Validation.ValidatedUnit) (s : StructHandle) (a : NRow)
    (k : NominalKind) (raw : RuntimeValue) :
    (FunctionCarriers.runtime unit).declared s a k raw = False := rfl

-- Runtime propagation preserves an actual nonconstant predicate.
example {unit : Validation.ValidatedUnit} (base : FunctionCarriers unit) :
    letI : FunctionCarriers unit := { base with declared := declared }
    (Carriers.runtime unit).declared source arguments kind (.bool true) := by
  exact ⟨rfl, rfl, rfl, rfl⟩

-- A nonidentity row substitution reaches the accepted row; source, kind and
-- raw value keep their identities. This would fail if the field were defaulted.
example : (Carriers.instantiate substitution family).declared
    source parameterRow kind (.bool true) :=
  ⟨rfl, rfl, rfl, rfl⟩

example : ¬ family.declared source parameterRow kind (.bool true) := by
  intro held
  have impossible : parameterRow = arguments := held.2.1
  cases impossible

example : ¬ (Carriers.instantiate substitution family).declared
    source parameterRow kind (.bool false) := by
  intro held
  have impossible : RuntimeValue.bool false = RuntimeValue.bool true := held.2.2.2
  cases impossible

example : ¬ (Carriers.instantiate substitution family).declared
    source parameterRow ⟨false, [], .owned, true⟩ (.bool true) := by
  intro held
  have impossible : (⟨false, [], .owned, true⟩ : NominalKind) = kind := held.2.2.1
  cases impossible

example : ¬ (Carriers.instantiate substitution family).declared
    ⟨⟨0⟩, 1⟩ parameterRow kind (.bool true) := by
  intro held
  have impossible : (⟨⟨0⟩, 1⟩ : StructHandle) = source := held.1
  cases impossible

-- Cut1 deliberately retains K1's nominal validity arm. K3 will change this
-- inhabited control to a rejection; the new field is only plumbing here.
example : @NTy.Holds Carriers.ground (.nominal source .nil kind) ⟨rfl⟩ :=
  @NTy.Holds.intro Carriers.ground _ _ trivial

example : (Carriers.instantiate substitution family).toSorts =
    Sorts.instantiate substitution family.toSorts := rfl

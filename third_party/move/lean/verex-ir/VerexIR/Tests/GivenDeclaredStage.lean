-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.GivenOwn
import VerexIR.Proofs.Denote.FunctionValues

namespace VerexIR.Tests.GivenDeclaredStage

open VerexIR VerexIR.Proofs.Denote

private def owner : StructHandle := ⟨⟨0⟩, 0⟩
private def kind : NominalKind := ⟨true, [], .owned, true⟩
private abbrev nominal : NTy := .nominal owner .nil kind

private def sample : @NTy.carrier Carriers.ground.toSorts nominal := ⟨rfl⟩

private def encoded : RuntimeValue := @NTy.encode Carriers.ground.toSorts nominal sample

-- The caller can supply genuine nominal validity even when its parameter
-- and own predicates reject. The input declared predicate depends on both
-- the declaration and the encoded value.
private abbrev input : GivenFrame :=
  ⟨{ Carriers.ground with
      holds := fun _ _ => False
      own := fun _ _ _ => False
      declared := fun source _ _ raw => source = owner ∧ raw = encoded }⟩

example : @NTy.Holds input.operations nominal sample :=
  @NTy.Holds.intro input.operations nominal sample ⟨rfl, rfl⟩

example : ¬ input.operations.declared ⟨⟨0⟩, 1⟩ .nil kind encoded := by
  intro held
  have impossible := held.1
  cases impossible

example : input.operations.holds 0 () = False := rfl

example : @NTy.Holds input.toCarriers (.param 0) () :=
  GivenFrame.holds_param input 0 ()

example : input.toCarriers.declared owner .nil kind encoded = False := rfl

example : input.clauseScaffold.own owner .nil encoded := trivial

example : input.clauseScaffold.declared owner .nil kind encoded = False := rfl

private def clause : GivenOwnClause input where
  type := .bool
  holds value := value = true

private def declarations : GivenOwnDeclarations input := fun _ _ => some clause

private abbrev selected : GivenFrame := input.withDeclarations declarations

example : input.operations.own owner .nil (.bool true) = False := rfl

example : selected.toCarriers.own owner .nil
    (@NTy.encode input.toCarriers.toSorts clause.type true) :=
  (GivenFrame.withDeclarations_own_encode input declarations owner .nil clause rfl true).mpr rfl

example : selected.operations.declared owner .nil kind encoded = False := rfl

example : selected.toCarriers.declared owner .nil kind encoded = False := rfl

-- Own selection keeps the raw and adapted operational parents. The adapted
-- parameter codec is still that of the input.
example : selected.operations.toSorts = input.operations.toSorts := rfl

example : selected.toCarriers.toSorts = input.toCarriers.toSorts := rfl

example : input.clauseScaffold.toSorts = input.toCarriers.toSorts := rfl

example (index : Nat) : input.toCarriers.codec index = input.operations.codec index := rfl

-- The same inhabited value reaches the adapted stage, but its old declared
-- certificate cannot justify nominal validity there.
example : ¬ @NTy.Holds selected.toCarriers nominal sample := by
  intro held
  exact @NTy.Holds.deepHolds selected.toCarriers _ _ held

-- These are the two fresh base constructors used by the concrete generator
-- before its own selection. Both start with closed declared validity.
example (unit : Validation.ValidatedUnit) :
    (FunctionCarriers.runtime unit).declared = (fun _ _ _ _ => False) := rfl

example {unit : Validation.ValidatedUnit} [NominalCarriers unit] (family : SiteFamily unit) :
    (family.functionCarriers (fun _ _ _ => True)).declared = (fun _ _ _ _ => False) := rfl

end VerexIR.Tests.GivenDeclaredStage

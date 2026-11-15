-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL.Proofs

open Lean Elab Command VerexSL VerexSL.Verify VerexIR.Proofs.Denote

-- A proof file adds lemmas in an unregistered copy of the source module.
-- Both that copy and its linked generic fields keep the source unit's
-- canonical nominal descriptors.
run_cmd modifyEnv (setDeferral · .all)

verex module 0x42::proof_copy_dependency where
  struct Leaf {T} has Copy, Drop where
    value : T

verex module 0x42::nominal_proof_copy where
  struct Box {T} has Copy, Drop where
    leaf : 0x42::proof_copy_dependency::Leaf<T>

run_cmd do
  modifyEnv (setDeferral · .none)
  modifyEnv (selectProofModule · `«0x42».nominal_proof_copy)

verex proofs where
  lemma same_value(box : Box<u64>)
    ensures box.leaf.value == box.leaf.value
  by
    assert box.leaf.value == box.leaf.value

  lemma same_flag(box : Box<Bool>)
    ensures box.leaf.value == box.leaf.value
  by
    assert box.leaf.value == box.leaf.value

  lemma same_values(number : Box<u64>, flag : Box<Bool>)
    ensures number.leaf.value == number.leaf.value && flag.leaf.value == flag.leaf.value
  by
    apply same_value(number)
    apply same_flag(flag)

run_cmd do
  let name := `«0x42».nominal_proof_copy
  let some unit := registeredUnit? (← getEnv) name
    | throwError "missing registered source unit"
  unless unit.namespaces[0]!.lemmas.isEmpty do
    throwError "proof-copy lemmas replaced the registered source unit"
  let unitName := Contract.unitName #["0x42", "nominal_proof_copy"]
  let descriptor := nominalDescriptorName unitName ⟨⟨0⟩, 0⟩
  for lemma in [`same_value, `same_flag, `same_values] do
    let theoremName := name ++ lemma ++ `lemma
    unless establishedLemmas.isTagged (← getEnv) theoremName do
      throwError m!"the proof-copy lemma {theoremName} was not established"
    let theoremInfo ← getConstInfo theoremName
    unless (theoremInfo.type.find? (·.isConstOf descriptor)).isSome do
      throwError m!"the proof-copy lemma {theoremName} does not use the canonical descriptor"
  let descriptorInfo ← getConstInfoDefn descriptor
  let dependency := nominalDescriptorName unitName ⟨⟨1⟩, 0⟩
  unless (descriptorInfo.value.find? (·.isConstOf dependency)).isSome do
    throwError "the proof-copy nominal field lost its linked generic descriptor"
  if (← getEnv).contains `unit then
    throwError "proof-copy verification defined an anonymous unit"

-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Validation.NominalCycles
import Lean

namespace LeanerIR.Tests.NominalCycles

open LeanerIR Import Validation

private def field (typeId : Nat) : FieldDecl := {
  name := ⟨1⟩, loc := ⟨7⟩, type := { typeId := ⟨typeId⟩, loc := ⟨7⟩ } }

private def fixture (typeId : Nat) : RawUnit := {
  profiles := #[]
  tables := { types := #[
    .unit, .nominal ⟨0⟩ #[], .vector ⟨1⟩,
    .nominal ⟨2⟩ #[.typeArg { typeId := ⟨1⟩, loc := ⟨7⟩ }],
    .function #[⟨1⟩] ⟨0⟩, .tuple #[⟨1⟩, ⟨1⟩],
    .reference { profile := .move, kind := .shared, referent := ⟨1⟩, lifetime := ⟨0⟩ }] }
  namespaces := #[{
    loc := ⟨0⟩, identity := ⟨0⟩, profile := some .move
    structs := #[{ name := ⟨0⟩, loc := ⟨7⟩, fields := #[field typeId] }] }] }

#guard (checkMoveNominalCycles (fixture 0)).isEmpty
#guard [1, 2, 3, 4, 5, 6].all fun typeId =>
  let diagnostics := checkMoveNominalCycles (fixture typeId)
  diagnostics.size == 1 && diagnostics[0]!.code == "LIR-MOVE-RECURSIVE-TYPE" &&
    diagnostics[0]!.primary == some ⟨7⟩

#guard Id.run do
  let unit := fixture 2
  let ns := unit.namespaces[0]!
  let declaration := ns.structs[0]!
  let variant := { name := ⟨3⟩, loc := ⟨7⟩, fields := declaration.fields : VariantDecl }
  return !(checkMoveNominalCycles { unit with namespaces := #[{ ns with
    structs := #[{ declaration with fields := #[], variants := #[variant] }] }] }).isEmpty

#guard Id.run do
  let unit := fixture 2
  let ns := unit.namespaces[0]!
  return (checkMoveNominalCycles { unit with
    namespaces := #[{ ns with profile := some .rust }] }).isEmpty

#guard Id.run do
  let unit := fixture 2
  let ns := unit.namespaces[0]!
  return !(checkMoveNominalCycles { unit with namespaces := #[], dependencies := #[{
    namespaceId := ⟨0⟩, profile := some .move, structs := ns.structs }] }).isEmpty

/-- Each tuple has two edges to the same preceding node: tree expansion
would visit exponentially many paths, but the interned graph stays linear. -/
private def diamond (depth : Nat) (cyclic : Bool) : RawUnit := Id.run do
  let mut types : Array Ty := #[if cyclic then .nominal ⟨0⟩ #[] else .unit]
  for index in [:depth] do
    types := types.push (.tuple #[⟨index⟩, ⟨index⟩])
  return { fixture depth with tables := { types } }

set_option maxHeartbeats 1000 in
#guard (checkMoveNominalCycles (diamond 512 false)).isEmpty

set_option maxHeartbeats 1000 in
#guard !(checkMoveNominalCycles (diamond 512 true)).isEmpty

/- Exhaust all directed graphs on three declarations against an independent
bounded path-search oracle, including self edges and disconnected cycles. -/
#guard (List.range 512).all fun mask => Id.run do
  let successors (node : Nat) : List Nat :=
    (List.range 3).filter fun target => mask.testBit (node * 3 + target)
  let rec returnsTo (start node : Nat) : Nat → Bool
    | 0 => false
    | fuel + 1 => (successors node).any fun target =>
        target == start || returnsTo start target fuel
  let cyclic := (List.range 3).any fun node => returnsTo node node 3
  let types := (Array.range 3).map fun index => Ty.nominal ⟨index⟩ #[]
  let declarations := (Array.range 3).map fun index =>
    { name := ⟨index⟩, loc := ⟨7⟩, fields := ((successors index).map field).toArray : StructDecl }
  let unit := { fixture 0 with tables := { types }, namespaces := #[{
    loc := ⟨0⟩, identity := ⟨0⟩, profile := some .move, structs := declarations }] }
  return !(checkMoveNominalCycles unit).isEmpty == cyclic

end LeanerIR.Tests.NominalCycles

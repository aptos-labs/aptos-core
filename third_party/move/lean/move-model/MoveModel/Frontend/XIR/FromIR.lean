-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.Frontend.XIR
import MoveModel.IR.Module

/-!
# Materializing semantic Move IR as XIR

The semantic IR uses bounded partial functions.  This conversion checks and
materializes those bounds into the list-based exchange representation.
-/

namespace MoveModel.Frontend.XIR

open MoveModel.IR

private def requireAt (kind : String) (i : Nat) (value : Option α) : Except String α :=
  match value with
  | some value => pure value
  | none => throw s!"missing {kind} at in-range index {i}"

private def materialize (kind : String) (count : Nat) (get : Nat → Option α) :
    Except String (List α) :=
  (List.range count).mapM fun i => requireAt kind i (get i)

private def contractToXIR (contract : Contract) : MContract where
  requires := [contract.requires]
  abortsIf := contract.aborts.toList
  ensures := [contract.ensures]
  modifies := contract.modifies

private def structToXIR (i : ResourceId) (decl : MoveModel.IR.StructDecl)
    (info : StructMeta) : Except String MStruct := do
  unless info.fieldNames.length = decl.fields.length do
    throw s!"struct {i} metadata has {info.fieldNames.length} field names, but its declaration has {decl.fields.length} fields"
  let variants ← match info.variantNames, decl.variants with
    | none, none => pure none
    | some names, some variants => do
        unless names.length = variants.length do
          throw s!"enum {i} metadata has {names.length} variants, but its declaration has {variants.length} variants"
        let result ← (names.zip variants).mapM fun ((variantName, fieldNames), fields) => do
          unless fieldNames.length = fields.length do
            throw s!"variant `{variantName}` has {fieldNames.length} field names, but its declaration has {fields.length} fields"
          return (variantName, fieldNames.zip fields)
        pure (some result)
    | _, _ => throw s!"struct {i} declaration and metadata disagree about whether it is an enum"
  return {
    name := info.name
    typeParams := decl.typeParams
    fields := info.fieldNames.zip decl.fields
    variants := variants
  }

private def funToXIR (i : FunId) (decl : MoveModel.IR.FunDecl)
    (info : FunMeta) : Except String MFun := do
  let locals ← materialize s!"local of function {i}" decl.numLocals decl.locals
  let blocks ← materialize s!"block of function {i}" decl.body.size decl.body.blocks
  for b in List.range decl.body.size do
    if (decl.loopSpecs b).isSome then
      throw s!"function {i} has loop metadata; executable IR-to-XIR loop materialization is not yet supported"
  unless decl.native || decl.body.entry < decl.body.size do
    throw s!"function {i} entry block {decl.body.entry} is outside its block range"
  unless decl.numParams ≤ decl.numLocals do
    throw s!"function {i} has {decl.numParams} parameters but only {decl.numLocals} locals"
  return {
    name := info.name
    typeParams := decl.typeParams
    params := decl.numParams
    locals := locals
    returns := decl.returns
    blocks := blocks
    entry := decl.body.entry
    loops := []
    spec := contractToXIR decl.contract
    native := decl.native
  }

/-- Materialize a finite semantic module into the exchange representation.
Missing declarations or metadata inside the declared bounds are rejected. -/
def MModule.ofIR (module : MoveModel.IR.Module) : Except String MModule := do
  let structDecls ← materialize "struct declaration" module.numStructs module.program.structs
  let structMeta ← materialize "struct metadata" module.numStructs module.structMeta
  let funDecls ← materialize "function declaration" module.numFuns module.program.funs
  let funMeta ← materialize "function metadata" module.numFuns module.funMeta
  let structs ← (structDecls.zipIdx.zip structMeta).mapM fun ((decl, i), info) =>
    structToXIR i decl info
  let funs ← (funDecls.zipIdx.zip funMeta).mapM fun ((decl, i), info) =>
    funToXIR i decl info
  return {
    structs := structs
    funs := funs
    address := module.address
    name := module.name
    dialect := module.dialect
    structMeta := structMeta
    funMeta := funMeta
    externalFuns := module.externalFuns
    friends := module.friends
  }

end MoveModel.Frontend.XIR

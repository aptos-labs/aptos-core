-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.FamilyTransport
open Lean Meta Elab Command
open VerexIR.Proofs.Denote
run_meta do
  let parameter := mkApp (mkConst ``NTy.param) (mkNatLit 0)
  let nextParameter := mkApp (mkConst ``NTy.param) (mkNatLit 1)
  let nil := mkConst ``NRow.nil
  let integer := mkApp2 (mkConst ``NTy.int) (mkNatLit 64) (toExpr false)
  withLocalDeclD `unknownRow (mkConst ``NRow) fun row => do
    unless (FamilyTransport.substituteNTy row parameter).isNone do
      throwError "an unknown row must not behave as an empty row"
    let knownPrefix := mkApp2 (mkConst ``NRow.cons) integer row
    unless FamilyTransport.substituteNTy knownPrefix parameter == some integer do
      throwError "a known prefix entry must substitute"
    unless (FamilyTransport.substituteNTy knownPrefix nextParameter).isNone do
      throwError "an unknown tail must not behave as an empty tail"
  unless FamilyTransport.substituteNTy nil parameter == some parameter do
    throwError "the actual empty row preserves a parameter"
  let known := mkApp2 (mkConst ``NRow.cons) integer nil
  unless FamilyTransport.substituteNTy known nextParameter == some nextParameter do
    throwError "known out-of-bounds parameter fallback changed"

-- Every constructor of `NTy` is spelled and substitutes as `NTy.subst`
-- defines. The substitution reads term syntax, where no match is checked for
-- its cases: a constructor added to the type fails here until it is taken.
run_meta do
  let parameter := mkApp (mkConst ``NTy.param) (mkNatLit 0)
  let integer := mkApp2 (mkConst ``NTy.int) (mkNatLit 64) (toExpr false)
  let row := mkApp2 (mkConst ``NRow.cons) parameter (mkConst ``NRow.nil)
  let rows := mkApp2 (mkConst ``NRows.cons) row (mkConst ``NRows.nil)
  let handle := mkApp2 (mkConst ``VerexIR.StructHandle.mk)
    (mkApp (mkConst ``VerexIR.NamespaceId.mk) (mkNatLit 0)) (mkNatLit 0)
  let names := toExpr ["Only"]
  let distinct ← mkDecideProof (← mkAppM ``List.Nodup #[names])
  let sharing := mkApp2 (mkConst ``Sharing.mk) (toExpr [false]) (toExpr [false])
  let samples : Array Lean.Expr := #[
    mkConst ``NTy.unit, mkConst ``NTy.bool, integer, mkConst ``NTy.num, mkConst ``NTy.address,
    mkConst ``NTy.signer, mkConst ``NTy.string, mkConst ``NTy.bytes, mkConst ``NTy.char,
    mkApp (mkConst ``NTy.tuple) row,
    mkApp3 (mkConst ``NTy.struct) handle row row,
    mkAppN (mkConst ``NTy.enum) #[handle, row, names, rows, distinct],
    mkApp (mkConst ``NTy.vector) parameter, mkApp (mkConst ``NTy.slice) parameter,
    mkApp2 (mkConst ``NTy.array) parameter (mkNatLit 2),
    mkApp2 (mkConst ``NTy.table) parameter parameter,
    mkApp (mkConst ``NTy.ghost) parameter, mkApp (mkConst ``NTy.ref) parameter,
    mkApp (mkConst ``NTy.sliceRef) parameter, parameter,
    mkApp3 (mkConst ``NTy.function) row sharing row,
    mkApp (mkConst ``NTy.observed) parameter,
    mkApp3 (mkConst ``NTy.nominal) handle row (toExpr true)]
  let some (.inductInfo type) := (← getEnv).find? ``NTy | throwError "NTy is no inductive type"
  for constructor in type.ctors do
    unless samples.any (·.getAppFn.isConstOf constructor) do
      throwError "the constructor {constructor} has no sample here"
  let arguments := mkApp2 (mkConst ``NRow.cons) integer (mkConst ``NRow.nil)
  for sample in samples do
    check sample
    unless FamilyTransport.spelledNTy sample do
      throwError "{sample} is not read as a spelled type"
    let some substituted := FamilyTransport.substituteNTy arguments sample
      | throwError "{sample} does not substitute"
    unless ← isDefEq substituted (mkApp2 (mkConst ``NTy.subst) arguments sample) do
      throwError "{sample} substitutes to {substituted}, not as `NTy.subst` defines"

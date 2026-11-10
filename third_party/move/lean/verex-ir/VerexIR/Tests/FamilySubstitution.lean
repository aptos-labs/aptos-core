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

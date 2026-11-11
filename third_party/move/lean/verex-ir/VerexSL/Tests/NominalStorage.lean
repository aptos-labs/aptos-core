-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

open Lean Meta Elab Command VerexSL VerexSL.Contract VerexIR.Proofs.Denote

-- Construction and destructuring preserve the same nominal resource index as
-- the storage operation they surround.
verex module 0x42::nominal_storage_index where
  pragma verify = false
  struct Vault {T has Store} has Key where
    value : T

  fun publish(account : &Signer, value : u64) -> Unit :=
    move_to<Vault<u64> >(account, new Vault<u64> { value })
  spec publish where
    requires !exists<Vault<u64> >(account.address)
    modifies global<Vault<u64> >(account.address)
    ensures global<Vault<u64> >(account.address).value == value
    aborts_if false

  fun take(address : Address) -> u64 := do
    let Vault<u64> { value := value } := move_from<Vault<u64> >(address)
    value
  spec take where
    requires exists<Vault<u64> >(address)
    modifies global<Vault<u64> >(address)
    ensures result == old(global<Vault<u64> >(address).value)
    ensures !exists<Vault<u64> >(address)
    aborts_if false

run_cmd do
  let some unit := registeredUnit? (← getEnv) `«0x42».nominal_storage_index
    | throwError "missing nominal storage fixture"
  let name ← ensureUnitDefinition #["0x42", "nominal_storage_index"] unit
  liftTermElabM <| withNominalUnit name do
    let some resource := structNTy unit ⟨⟨0⟩, 0⟩
      | throwError "missing nominal storage resource"
    let owner ← quoteNTy (resource.subst (.cons (.int 64 false) .nil))
    let some normalizer ← getSimpExtension? `vir_denote_norm
      | throwError "missing normalization rules"
    let metadataContext ← Simp.mkContext
      { zeta := false, beta := true, iota := false, proj := false }
      #[← normalizer.getTheorems]
    let metadataProcs ← Simp.SimprocsArray.add #[] ``nominalHandle false
    let (metadata, _) ← simp (← mkAppM ``NTy.handle? #[owner]) metadataContext metadataProcs
    unless metadata.expr.isAppOfArity ``Option.some 2 do
      throwError "resource dispatch did not observe the named declaration's handle"
    unless metadata.expr.getArg! 1 == (← nativeTypeView owner).getArg! 0 do
      throwError "resource dispatch changed declaration identity"
    let some extension ← getSimpExtension? `vir_denote
      | throwError "missing denotation rules"
    let context ← Simp.mkContext { zeta := false, beta := true, iota := false, proj := false }
      #[← extension.getTheorems]
    for (index, operation) in [(0, ``Term.packAt), (1, ``Term.letFieldsAt)] do
      let .ok compiled := compileModelFunction unit ⟨⟨0⟩, ⟨index⟩⟩
        | throwError "failed to compile nominal storage fixture"
      let quoted ← VerexSL.Verify.quoteTerm (mkConst name) compiled.body
      let some produced := quoted.find? (·.isAppOf operation)
        | throwError "nominal storage did not use {operation}"
      unless produced.getArg! 3 == owner do
        throwError "nominal storage producer lost its descriptor"
      let row ← quoteRow compiled.frame
      let shape ← quoteShape compiled.result
      let bodyType ← quoteNTy compiled.result.bodyType
      withLocalDeclD `family (mkConst ``Carriers) fun family =>
        withLocalDeclD `operations
            (mkApp3 (mkConst ``InterpreterOps) (mkConst name) family (mkConst ``Unit))
            fun operations =>
          withLocalDeclD `locals (mkApp2 (mkConst ``HEnv) family row) fun locals => do
            let call := mkAppN (mkConst ``Term.denoteIn)
              #[mkConst name, family, mkConst ``Unit, operations, shape, row, bodyType, quoted, locals]
            let (normalized, _) ← simp call context
            for operation in [``InterpreterOps.read, ``InterpreterOps.write] do
              let selects := fun e : Lean.Expr =>
                e.isAppOfArity operation (if operation == ``InterpreterOps.read then 8 else 9)
              unless (normalized.expr.find? selects).isSome do
                throwError "nominal storage did not expose {operation}"
              if let some wrong := normalized.expr.find? fun e => selects e && e.getArg! 4 != owner then
                throwError "storage operation lost its nominal type: {wrong.getArg! 4}, expected {owner}"

verify 0x42::nominal_storage_index::publish by
  on _ { }
verify 0x42::nominal_storage_index::take by
  on _ { }

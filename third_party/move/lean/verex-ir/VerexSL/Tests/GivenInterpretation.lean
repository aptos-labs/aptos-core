-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
open Lean Meta VerexSL VerexSL.Contract VerexSL.Verify VerexIR.Proofs.Denote
verex module 0x42::given_verification where
  pragma verify = false
  fun identity {T}(x : T) -> T := x
  spec identity where
    aborts_if false
    ensures result == x

run_meta do
  let some unit := registeredUnit? (← getEnv) `«0x42».given_verification
    | throwError "missing test unit"
  let some (namespaceIndex, ns, _, declaration) := findFunction? unit "identity"
    | throwError "missing identity"
  let some (params, result) := nativeSignature? unit ⟨namespaceIndex⟩ declaration
    | throwError "missing native signature"
  withUnitVariable .implicit fun unitExpr functions => do
    withLocalDeclD `G (mkConst ``GivenFrame) fun frame => do
    withLocalDeclD `State (mkSort .one) fun state => do
      let interpretation := TranslationFrame.ofGiven unitExpr functions frame state
      let signature ← quoteNativeSignatureAt interpretation params result
      let contract ← buildContract unit ⟨namespaceIndex⟩ ns declaration signature
        (types := some interpretation.types)
      unless ← isTypeCorrect contract do throwError "given contract is not well typed"
      if (contract.find? fun e => e.isConstOf ``Skolems || e.isConstOf ``Memory).isSome then
        throwError "given contract retained a concrete frame or memory"

-- Parameter map views are produced as identities in the given interpretation.
-- No concrete read/unread operation is inserted and later simplified away.
run_meta do
  withLocalDeclD `given (mkConst ``GivenFrame) fun given => do
    let carriers := mkApp (mkConst ``GivenFrame.toCarriers) given
    withLocalDecl `nonempty .instImplicit
        (mkApp (mkConst ``CarriersNonempty) carriers) fun _ => do
      let type := mkApp (mkConst ``NTy.param) (mkNatLit 0)
      for plain in [false, true] do
        let some view ← NativeMaps.typedView? carriers type plain
          | throwError "missing given parameter map view"
        unless view.identity && view.read.isLambda && view.unread == view.read do
          throwError "given parameter map view retained a conversion"
        unless ← isTypeCorrect view.read do
          throwError "given parameter identity read is not well typed"

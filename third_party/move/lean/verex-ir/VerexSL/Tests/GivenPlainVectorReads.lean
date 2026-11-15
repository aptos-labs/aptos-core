-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL.Proofs

namespace VerexSL.Tests.GivenPlainVectorReads

open Lean Meta Elab Command VerexIR VerexIR.Proofs.Denote VerexSL

run_meta withSyntheticNativeTypes do
  let procs ← Simp.SimprocsArray.add #[] ``plainParameterField false
  let context ← Simp.mkContext { zeta := false, beta := false, iota := false, proj := false }
  let parameter := mkApp (mkConst ``NTy.param) (mkNatLit 0)
  let fields := mkApp2 (mkConst ``NRow.cons) parameter (mkConst ``NRow.nil)
  let empty := mkConst ``NRow.nil
  let source := toExpr (⟨⟨0⟩, 0⟩ : StructHandle)
  let ownerType := mkApp3 (mkConst ``NTy.struct) source empty fields
  let position := mkApp2 (mkConst ``Var.here) empty parameter
  let check (family : Lean.Expr) (identity : Bool) : MetaM Unit := do
    withLocalDeclD `owner (mkApp2 (mkConst ``NTy.carrier) family ownerType) fun owner => do
      let selected := mkAppN (mkConst ``Var.selectCertified)
        #[family, source, empty, fields, parameter, position, owner]
      unless ← isTypeCorrect selected do throwError "ill typed selected field fixture"
      let (result, _) ← withReducible <| simp selected context procs
      if identity then
        unless result.expr != selected do throwError "given parameter field was not observed"
        unless ← withDefault <| isDefEq selected result.expr do
          throwError "given parameter projection changed its value"
      else
        unless result.expr == selected do
          throwError "unknown parameter family lost its certification operation"
  withLocalDeclD `unknown (mkConst ``Carriers) fun family => check family false
  withLocalDeclD `given (mkConst ``GivenFrame) fun given =>
    check (mkApp (mkConst ``GivenFrame.toCarriers) given) true

open Lean Meta Elab Command VerexIR VerexIR.Proofs.Denote

run_cmd liftTermElabM do
  let index := mkNatLit 0
  let stable := #[``Carriers.carrier, ``Carriers.codec, ``Carriers.orders]
  withLocalDeclD `given (mkConst ``GivenFrame) fun given => do
    let declarationsType := mkApp (mkConst ``GivenOwnDeclarations) given
    withLocalDeclD `declarations declarationsType fun declarations => do
      let installed := mkApp2 (mkConst ``GivenFrame.withDeclarations) given declarations
      let original := mkApp (mkConst ``GivenFrame.operations) given
      for family in #[mkApp (mkConst ``GivenFrame.toCarriers) installed,
          mkApp (mkConst ``GivenFrame.clauseScaffold) given,
          mkApp (mkConst ``GivenFrame.operations) installed] do
        for head in stable do
          let projected := mkApp2 (mkConst head) family index
          let result ← canonicalFamilyResult projected
          let expected := mkApp2 (mkConst head) original index
          unless result.expr == expected do
            throwError "stable given operation did not retain its original family: {result.expr}"
          unless ← isTypeCorrect result.expr do throwError "ill typed given operation"
          unless ← withDefault <| isDefEq projected result.expr do
            throwError "given operation observation changed its interpretation"
        if family.getAppFn.isConstOf ``GivenFrame.toCarriers ||
            family.getAppFn.isConstOf ``GivenFrame.clauseScaffold then
          let projected := mkApp (mkConst ``Carriers.toPlain) family
          let result ← canonicalFamilyResult projected
          let expected := mkApp (mkConst ``Carriers.toPlain)
            (mkApp (mkConst ``GivenFrame.toCarriers) given)
          unless result.expr == expected do
            throwError "Given plain view did not retain its canonical identity lent layer"
          unless ← withDefault <| isDefEq projected result.expr do
            throwError "Given plain observation changed its interpretation"
        for head in #[``Carriers.reads, ``Carriers.lent, ``Carriers.plainRead, ``Carriers.own] do
          let projected := mkApp (mkConst head) family
          let result ← canonicalFamilyResult projected
          unless result.expr == projected do
            throwError "operation observation erased a view-specific member: {result.expr}"
  withLocalDeclD `unknown (mkConst ``Carriers) fun family => do
    for head in stable do
      let projected := mkApp2 (mkConst head) family index
      let result ← canonicalFamilyResult projected
      unless result.expr == projected do
        throwError "unknown operation family changed"

open Lean Meta Elab Command VerexIR VerexIR.Proofs.Denote

-- A symbolic element retains its actual forgetting, including in a missing
-- read. The case's original certified proof remains available.
run_cmd liftTermElabM do
  withLocalDeclD `family (mkConst ``Carriers) fun family =>
    withLocalDeclD `element (mkConst ``NTy) fun element => do
      let empty := mkConst ``NRow.nil
      let vectorType := mkApp (mkConst ``NTy.vector) element
      let fields := mkApp2 (mkConst ``NRow.cons) vectorType empty
      let source := toExpr (⟨⟨0⟩, 0⟩ : StructHandle)
      let host := mkApp3 (mkConst ``NTy.struct) source empty fields
      let position := mkApp2 (mkConst ``Var.here) empty vectorType
      let carrier := mkApp2 (mkConst ``NTy.carrier) family element
      withLocalDeclD `owner (mkApp2 (mkConst ``NTy.carrier) family host) fun owner => do
        let vector := mkAppN (mkConst ``Var.selectCertified)
          #[family, source, empty, fields, vectorType, position, owner]
        let values ← mkAppM ``VerexIR.SpecVector.values #[vector]
        let lookup ← mkAppM ``GetElem?.getElem? #[values, mkNatLit 0]
        let check (result : Lean.Expr) (present : Bool) : MetaM Unit := do
          withLocalDeclD `originalRead (← mkEq lookup result) fun equation => do
            let goal ← mkFreshExprMVar (mkConst ``True)
            let next ← assertPlainVectorLookup goal.mvarId! equation
            next.withContext do
              unless (← getLCtx).contains equation.fvarId! do
                throwError "lookup producer discarded its certified source proof"
              let some fact := (← getLCtx).findFromUserName? `verexPlainRead
                | throwError "lookup producer omitted its plain case"
              let some (_, _, rhs) := fact.type.eq? | throwError "not a lookup equation"
              if present then
                unless rhs.isAppOfArity ``Option.some 2 &&
                    rhs.appArg!.isAppOfArity ``NTy.forget 3 &&
                    rhs.appArg!.getArg! 0 == family && rhs.appArg!.getArg! 1 == element do
                  throwError "symbolic lookup erased its actual forgetting: {rhs}"
              else unless rhs.isAppOfArity ``Option.none 1 do
                throwError "missing lookup acquired a fallback value"
              next.assign (mkConst ``True.intro)
            unless ← isTypeCorrect (← instantiateMVars goal) do
              throwError "lookup case proof is ill typed"
        check (← mkAppOptM ``Option.none #[carrier]) false
        withLocalDeclD `value carrier fun value => do
          check (← mkAppOptM ``Option.some #[carrier, value]) true

end VerexSL.Tests.GivenPlainVectorReads

-- Owned plain entries and certified program reads share one lookup form.
verex module 0x42::certified_vector_reads where
  struct Entry {K has Copy, Drop, Store} has Copy, Drop, Store where
    key : K
  struct Bag {K has Copy, Drop, Store} has Copy, Drop, Store where
    entries : Vector<Entry<K> >
  spec fun ordered {K has Copy, Drop, Store}(entries : Vector<Entry<K> >) : Bool :=
    forall (i : Int), forall (j : Int), 0 <= i && i < j && j < entries.length ==>
      core.prim.compare(entries[i].key, entries[j].key) < 0
  spec Bag where
    pragma verify = false
    invariant ordered(entries)
  fun ordered_length {K has Copy, Drop, Store}(entries : &Vector<Entry<K> >) -> u64 :=
    entries.length
  spec ordered_length where
    pragma verify = false
    requires ordered(entries)
  fun observe {K has Copy, Drop, Store}(bag : &Bag<K>) -> u64 :=
    ordered_length::<K>(&bag.entries)

  fun borrow_key {K has Copy, Drop, Store}(bag : &Bag<K>, index : u64) -> &K :=
    &bag.entries[index].key
  spec borrow_key where
    ensures result == bag.entries[index].key
    aborts_if index >= bag.entries.length

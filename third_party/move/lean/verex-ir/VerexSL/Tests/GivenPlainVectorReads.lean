-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL.Proofs

-- This test asserts what the Given route does; it selects the route.
set_option verex.givenRoute true


namespace VerexSL.Tests.GivenPlainVectorReads

open Lean Meta Elab Command VerexIR VerexIR.Proofs.Denote VerexSL

run_cmd liftTermElabM do
  let index := mkNatLit 0
  let stable := #[``Sorts.carrier, ``Sorts.codec, ``Sorts.orders]
  let sorts := VerexSL.Contract.carrierSorts
  withLocalDeclD `unknown (mkConst ``Carriers) fun family => do
    for head in stable do
      let projected := mkApp2 (mkConst head) (sorts family) index
      let result ← canonicalFamilyResult projected
      unless result.expr == projected do
        throwError "unknown operation family changed"
  withLocalDeclD `given (mkConst ``GivenFrame) fun given => do
    let declarationsType := mkApp (mkConst ``GivenOwnDeclarations) given
    withLocalDeclD `declarations declarationsType fun declarations => do
      let installed := mkApp2 (mkConst ``GivenFrame.withDeclarations) given declarations
      let original := mkApp (mkConst ``GivenFrame.operations) given
      for family in #[mkApp (mkConst ``GivenFrame.toCarriers) installed,
          mkApp (mkConst ``GivenFrame.clauseScaffold) given,
          mkApp (mkConst ``GivenFrame.operations) installed] do
        for head in #[``Sorts.reads, ``Carriers.holds, ``Carriers.own] do
          let dictionary := if head == ``Sorts.reads then sorts family else family
          let projected := mkApp (mkConst head) dictionary
          let result ← canonicalFamilyResult projected
          unless result.expr == projected do
            throwError "operation observation erased a view-specific member: {result.expr}"
        for head in stable do
          let projected := mkApp2 (mkConst head) (sorts family) index
          let result ← canonicalFamilyResult projected
          let expected := mkApp2 (mkConst head) (sorts original) index
          unless result.expr == expected do
            throwError "stable given operation did not retain its original family: {result.expr}"
          unless ← isTypeCorrect result.expr do throwError "ill typed given operation"
          unless ← withDefault <| isDefEq projected result.expr do
            throwError "given operation observation changed its interpretation"

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

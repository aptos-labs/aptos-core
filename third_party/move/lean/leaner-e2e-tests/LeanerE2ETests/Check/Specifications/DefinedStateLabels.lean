-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-! State-change definitions are memory expressions. Clauses can read a label
before its definition, and chained changes retain other types and addresses. -/

namespace LeanerLang.Tests.Check.Specifications.DefinedStateLabels

leaner module 0x42::defined_state_labels where
  struct R has Key where
    value : u64

  struct Other has Key where
    value : u64

  struct Coin {T : phantom type} has Key where
    value : u64

  fun publish_coin {T}(account : &Signer, value : u64) -> Unit :=
    move_to<Coin<T> >(account, new Coin<T> { value := value })
  spec publish_coin where
    aborts_if exists<Coin<T> >(account.address)
    ensures ..S |~ publish<Coin<T> >(account.address, new Coin<T> { value := value })
    ensures S |~ global<Coin<T> >(account.address).value == value
    ensures publish<Coin<T> >(account.address, new Coin<T> { value := value })
    modifies global<Coin<T> >(account.address)

  fun create(account : &Signer) -> Unit :=
    move_to<R>(account, new R { value := 42 })
  spec create where
    aborts_if exists<R>(account.address)
    ensures publish<R>(account.address, new R { value := 42 })
    ensures S |~ global<R>(account.address).value == 42
    ensures ..S |~ publish<R>(account.address, new R { value := 42 })
    ensures S..T |~ update<R>(account.address, new R { value := 43 })
    ensures T |~ global<R>(account.address).value == 43
    ensures (S |~ exists<Other>(account.address)) == old(exists<Other>(account.address))
    modifies global<R>(account.address)

  fun destroy(addr : Address) -> u64 := do
    let R { value := value } := move_from<R>(addr)
    value
  spec destroy where
    aborts_if !exists<R>(addr)
    ensures remove<R>(addr)
    ensures !(S |~ exists<R>(addr))
    ensures ..S |~ remove<R>(addr)
    ensures result == old(global<R>(addr).value)
    modifies global<R>(addr)

  fun replace(addr : Address, value : u64) -> Unit := do
    let r := &mut R[addr]
    r.value := value
  spec replace where
    aborts_if !exists<R>(addr)
    ensures update<R>(addr, new R { value := value })
    ensures ..S |~ update<R>(addr, new R { value := value })
    ensures S |~ global<R>(addr).value == value
    modifies global<R>(addr)

  fun recreate(account : &Signer) -> Unit := do
    let R { value := _ } := move_from<R>(account.address)
    move_to<R>(account, new R { value := 7 })
  spec recreate where
    pragma opaque
    aborts_if !exists<R>(account.address)
    ensures do
      let addr := account.address
      (S.. |~ publish<R>(addr, new R { value := 7 })) && (..S |~ remove<R>(addr))
    modifies global<R>(account.address)

  -- A definition in a different clause retains its local address binding.
  -- The call below uses the opaque contract, without the callee's points.
  fun split_recreate(account : &Signer) -> Unit := recreate(account)
  spec split_recreate where
    pragma opaque
    aborts_if !exists<R>(account.address)
    ensures S |~ !exists<R>(account.address)
    ensures do
      let addr := account.address
      S.. |~ publish<R>(addr, new R { value := 7 })
    ensures do
      let addr := account.address
      (..S |~ remove<R>(addr))
    modifies global<R>(account.address)

  fun call_split_recreate(account : &Signer) -> Unit := split_recreate(account)
  spec call_split_recreate where
    aborts_if !exists<R>(account.address)
    ensures global<R>(account.address).value == 7
    modifies global<R>(account.address)

  -- The opaque call is replaced by its contract. The caller has no point
  -- for the callee's removal; its label is a function of the call's pre-state.
  fun call_recreate(account : &Signer, other : Address) -> Unit :=
    recreate(account)
  spec call_recreate where
    requires exists<R>(account.address)
    requires other != account.address
    ensures exists<R>(account.address)
    ensures global<R>(account.address).value == 7
    ensures exists<R>(other) == old(exists<R>(other))
    ensures exists<Other>(other) == old(exists<Other>(other))
    aborts_if false
    modifies global<R>(account.address)

end LeanerLang.Tests.Check.Specifications.DefinedStateLabels

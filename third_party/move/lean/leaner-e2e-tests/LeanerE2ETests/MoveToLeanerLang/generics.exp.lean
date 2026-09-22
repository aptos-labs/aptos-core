-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner module 0x42::generics where
  struct Pair {T} {U} has Copy, Drop, Store where
    first : T
    second : U

  struct Vault {T has Store} has Key where
    value : T

  fun swap {T has Copy, Drop} {U has Copy, Drop}(
    value : Pair<T, U>
  ) -> Pair<U, T> := new Pair<U, T> {
    first := value.second, second := value.first
  }

  spec swap where
    ensures result.first == value.second
    ensures result.second == value.first

  fun publish_generic {T has Store}(account : &Signer, value : T) -> Unit := do
    move_to<Vault<T> >(account, new Vault<T> { value })

  fun has_vault {T has Store}(addr : Address) -> Bool := exists<Vault<T> >(addr)

  fun swapped(value : u64) -> Pair<u64, u64> :=
    swap(new Pair<u64, u64> { first := value, second := value + 1 })

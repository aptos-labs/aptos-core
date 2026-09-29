-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
# Generic global storage

Storage over generic resources, verified once for every instantiation, and
concrete callers through the generic theorems.
-/

namespace LeanerLang.Tests.Check.Generics.GenericStorage

set_option leaner.verifyHeartbeats 35000

leaner module 0x42::generic_storage where
  struct Vault {T has Store} has Key where
    value : T

  struct Counter has Key where
    value : u64

  fun publish_generic {T has Store}(account : &Signer, value : T) -> Unit :=
    move_to<Vault<T> >(account, new Vault<T> { value })
  spec publish_generic where
    requires !exists<Vault<T> >(account.address)
    modifies global<Vault<T> >(account.address)
    ensures global<Vault<T> >(account.address).value == value
    aborts_if false

  fun has_generic {T has Store}(address : Address) -> Bool :=
    exists<Vault<T> >(address)
  spec has_generic where
    ensures result == exists<Vault<T> >(address)
    aborts_if false

  fun has_u64_vault(address : Address) -> Bool :=
    core.call has_generic::<u64>(address)
  spec has_u64_vault where
    ensures result == exists<Vault<u64> >(address)
    aborts_if false

  fun take_generic {T has Store}(address : Address) -> T := do
    let Vault<T> { value := value } := move_from<Vault<T> >(address)
    value
  spec take_generic where
    requires exists<Vault<T> >(address)
    modifies global<Vault<T> >(address)
    ensures result == old(global<Vault<T> >(address).value) &&
      !exists<Vault<T> >(address)
    aborts_if false

  spec module where
    invariant forall (a : Address),
      global<Vault<u64> >(a).value == global<Vault<u64> >(a).value

  public entry fun bump_vault(address : Address) -> Unit := do
    let value := &mut Vault<u64>[address].value
    *value := *value + 1
  spec bump_vault where
    requires exists<Vault<u64> >(address)
    modifies global<Vault<u64> >(address)
    ensures global<Vault<u64> >(address).value ==
      old(global<Vault<u64> >(address).value) + 1
    aborts_if old(global<Vault<u64> >(address).value) + 1 >
      MAX_U64

  public entry fun mirror(address : Address) -> Unit := do
    let counter := &Counter[address]
    let current := *counter
    let value := &mut Vault<u64>[address].value
    *value := current.value
  spec mirror where
    requires exists<Counter>(address) && exists<Vault<u64> >(address)
    modifies global<Vault<u64> >(address)
    ensures global<Vault<u64> >(address).value ==
      old(global<Counter>(address).value)
    aborts_if false

  public entry fun publish_u64(account : &Signer, value : u64) -> Unit :=
    core.call publish_generic::<u64>(account, value)
  spec publish_u64 where
    requires !exists<Vault<u64> >(account.address)
    modifies global<Vault<u64> >(account.address)
    ensures global<Vault<u64> >(account.address).value == value
    aborts_if false

end LeanerLang.Tests.Check.Generics.GenericStorage

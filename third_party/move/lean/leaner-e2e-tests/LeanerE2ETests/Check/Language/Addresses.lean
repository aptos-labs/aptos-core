-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

/-! Port of v0 Language/Addresses. Package address aliases are resolved by
the source frontend before LeanerLang, so this check retains their concrete
256-bit values at the verification and execution boundary. -/

leaner module 0xCAFE::addresses where
  struct Vault has Key where
    balance : u64

  fun owner() -> Address := @0xCAFE
  fun framework() -> Address := @0x1
  fun the_admin() -> Address := @0xA550C18

  fun is_admin(addr : Address) -> Bool := addr == @0xA550C18
  spec is_admin where
    ensures result == (addr == @0xA550C18)
    aborts_if false
  verify is_admin

  fun balance_at(addr : Address) -> u64 := do
    let value := &Vault[addr].balance
    return *value
  spec balance_at where
    requires exists<Vault>(addr)
    ensures result == old(global<Vault>(addr).balance)
    aborts_if false
  verify balance_at

  fun balance_of_owner() -> u64 := balance_at(@0xCAFE)
  spec balance_of_owner where
    requires exists<Vault>(@0xCAFE)
    ensures result == old(global<Vault>(@0xCAFE).balance)
    aborts_if false

set_option leaner.route "native" in
#leaner_verify 0xCAFE::addresses::balance_of_owner

open LeanerIR LeanerE2ETests.CheckSupport in
run_cmd do
  let initial ← singleResourceState `«0xCAFE».addresses "Vault" "0xCAFE" #[.integer 7]
  assertRuns `«0xCAFE».addresses #[
    ⟨"owner", #[], .returned #[.address "0xCAFE"], {}⟩,
    ⟨"framework", #[], .returned #[.address "0x1"], {}⟩,
    ⟨"the_admin", #[], .returned #[.address "0xA550C18"], {}⟩,
    ⟨"is_admin", #[.address "0xA550C18"], .returned #[.bool true], {}⟩,
    ⟨"is_admin", #[.address "0xCAFE"], .returned #[.bool false], {}⟩,
    ⟨"balance_of_owner", #[], .returned #[.integer 7], initial⟩]

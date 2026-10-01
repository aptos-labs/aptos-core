-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
# Move address aliases

A Move path spells its address as a literal or an alias, and a module is its
address and name whatever the spelling. `@alias` is the address as a value.
A rendering spells a module with the alias it was declared with.
-/

namespace LeanerLang.Tests.AddressAliases

address_alias application = 0x42
-- Declaring an alias again at the same address changes nothing.
address_alias application = 0x0042

leaner module application::registry where
  use std::vector

  fun own_address() -> Address := @application
  fun literal_address() -> Address := @0xCAFE
  fun standard_address() -> Address := @std
  fun has_zero(values : &Vector<u64>) -> Bool := values.contains(&0)

open Lean Elab Command in
run_cmd do
  let env ← getEnv
  -- The module is registered at its address, not its alias.
  let some unit := LeanerLang.registeredUnit? env `«0x42».registry
    | throwError "the module is not registered at its address"
  unless (LeanerLang.registeredUnit? env `application.registry).isNone do
    throwError "the module is registered at its alias"
  let some self := unit.tables.namespaces[0]?
    | throwError "the module has no namespace"
  unless self.segments == #["0x42", "registry"] && self.alias == some "application" do
    throwError s!"the module's identity is {self.segments} spelled {self.alias}"
  -- A rendering spells the module and its imports as they were written,
  -- and reads back to the same unit.
  let .ok printed := LeanerLang.Print.render env unit
    | throwError "the module did not render"
  unless printed.contains "leaner module application::registry where" &&
      printed.contains "use std::vector" && printed.contains "@0x42" &&
      !printed.contains "address_alias" do
    throwError s!"the rendering lost an alias:\n{printed}"
  let .ok formatted := LeanerLang.Print.formatSource env printed
    | throwError "the rendering did not re-import"
  unless formatted == printed do
    throwError s!"the rendering is not a fixed point:\n{formatted}"

/-- error: unknown address alias `nowhere` -/
#guard_msgs in
leaner module nowhere::lost where
  fun one() -> u64 := 1

/-- error: unknown address alias `nowhere` -/
#guard_msgs in
leaner module 0x42::lost_value where
  fun where_to() -> Address := @nowhere

/-- error: module `0x1::vector` is spelled both `std` and `aptos_std` -/
#guard_msgs in
leaner module 0x42::two_spellings where
  use std::vector
  use aptos_std::vector::contains

/-- error: address alias `application` already stands for 0x42 -/
#guard_msgs in
address_alias application = 0x43

/-- error: a Move address is at most 256 bits -/
#guard_msgs in
address_alias enormous = 0x10000000000000000000000000000000000000000000000000000000000000000

end LeanerLang.Tests.AddressAliases

-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexE2ETests.CheckSupport

/-! A module calls another module's functions as Move allows: public ones
from anywhere, friend ones from the modules it names friends, package ones
from modules at its address. A call stating its result type is reserved for
a function no module of the unit declares, so it cannot reach past a
declaration's visibility. -/

verex module 0x42::library where
  friend 0x42::insider

  fun secret() -> u64 := 1

  friend fun trusted() -> u64 := 2

  package fun shared() -> u64 := 3

  public fun shown() -> u64 := 4

verex module 0x42::insider where
  use 0x42::library

  fun calls() -> u64 := library::trusted() + library::shared() + library::shown()

verex module 0x43::outsider where
  use 0x42::library

  fun calls() -> u64 := library::shown()

verex module 0x43::private_call where
  use 0x42::library

  fun call() -> u64 := library::secret()

verex module 0x43::friend_call where
  use 0x42::library

  fun call() -> u64 := library::trusted()

verex module 0x43::package_call where
  use 0x42::library

  fun call() -> u64 := library::shared()

verex module 0x43::typed_call where
  use 0x42::library

  fun call() -> u64 := (library::secret() : u64)

verex module 0x43::core_typed_call where
  use 0x42::library

  fun call() -> u64 := core.call [u64] library::secret()

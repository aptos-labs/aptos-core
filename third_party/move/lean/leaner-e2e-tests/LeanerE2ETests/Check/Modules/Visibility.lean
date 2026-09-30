-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

/-! A module calls another module's functions as Move allows: public ones
from anywhere, friend ones from the modules it names friends, package ones
from modules at its address. -/

leaner module 0x42::library where
  friend 0x42::insider

  fun secret() -> u64 := 1

  friend fun trusted() -> u64 := 2

  package fun shared() -> u64 := 3

  public fun shown() -> u64 := 4

leaner module 0x42::insider where
  use 0x42::library

  fun calls() -> u64 := library::trusted() + library::shared() + library::shown()

leaner module 0x43::outsider where
  use 0x42::library

  fun calls() -> u64 := library::shown()

leaner module 0x43::private_call where
  use 0x42::library

  fun call() -> u64 := library::secret()

leaner module 0x43::friend_call where
  use 0x42::library

  fun call() -> u64 := library::trusted()

leaner module 0x43::package_call where
  use 0x42::library

  fun call() -> u64 := library::shared()

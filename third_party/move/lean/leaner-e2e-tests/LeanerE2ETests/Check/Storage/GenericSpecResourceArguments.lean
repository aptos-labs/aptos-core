-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

/-! A specification function's type-argument row need not match the row of
the resource it reads. Instantiate the resource's arguments, including phantom
ones, with the caller's row; do not substitute those two rows in reverse. -/

leaner module 0x42::generic_spec_resource_arguments where
  struct Coin {T : phantom type} has Key where
    value : u64

  struct USD where
    dummy_field : Bool

  struct EUR where
    dummy_field : Bool

  spec fun increased_first {T} {Unused}(addr : Address) : Bool :=
    old(global<Coin<T> >(addr).value) < global<Coin<T> >(addr).value

  spec fun increased_second {Unused} {T}(addr : Address) : Bool :=
    old(global<Coin<T> >(addr).value) < global<Coin<T> >(addr).value

  fun increment_usd(addr : Address) -> Unit :=
    Coin<USD>[addr].value := Coin<USD>[addr].value + 1
  spec increment_usd where
    ensures increased_first::<USD, EUR>(addr)
    ensures increased_second::<EUR, USD>(addr)
    modifies *

  fun increment_generic {C}(addr : Address) -> Unit :=
    Coin<C>[addr].value := Coin<C>[addr].value + 1
  spec increment_generic where
    ensures increased_first::<C, EUR>(addr)
    ensures increased_second::<EUR, C>(addr)
    modifies *

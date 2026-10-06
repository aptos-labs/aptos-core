-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-! Invocation-defined labels remain meaningful in opaque caller contracts. -/

namespace LeanerLang.Tests.Check.Specifications.InvocationStateLabels

leaner module 0x42::invocation_state_labels where
  struct R has Key where
    value : u64

  fun put(addr : Address, value : u64) -> u64 := do
    let r := &mut R[addr]
    r.value := value
    value
  spec put where
    pragma opaque
    aborts_if !exists<R>(addr)
    ensures global<R>(addr).value == value
    ensures result == value
    modifies global<R>(addr)

  fun via_result(addr : Address, value : u64) -> u64 := put(addr, value)
  spec via_result where
    pragma opaque
    requires exists<R>(addr)
    aborts_if false
    -- A label can be read before its defining clause.
    ensures (S |~ global<R>(addr).value) == global<R>(addr).value
    ensures result == (..S |~ result_of<function[Fn(Address, u64) -> u64](put)>(addr, value))
    modifies global<R>(addr)

  fun via_ensures(addr : Address, value : u64) -> u64 := put(addr, value)
  spec via_ensures where
    pragma opaque
    requires exists<R>(addr)
    aborts_if false
    ensures ..S |~ ensures_of<function[Fn(Address, u64) -> u64](put)>(addr, value, result)
    ensures (S |~ global<R>(addr).value) == global<R>(addr).value
    modifies global<R>(addr)

  fun caller_result(addr : Address) -> u64 := via_result(addr, 7)
  spec caller_result where
    requires exists<R>(addr)
    aborts_if false
    ensures result == 7
    ensures global<R>(addr).value == 7
    modifies global<R>(addr)

  fun caller_ensures(addr : Address) -> u64 := via_ensures(addr, 8)
  spec caller_ensures where
    requires exists<R>(addr)
    aborts_if false
    ensures result == 8
    ensures global<R>(addr).value == 8
    modifies global<R>(addr)

  fun chained(addr : Address) -> u64 := do
    let _ := put(addr, 1)
    put(addr, 2)
  spec chained where
    requires exists<R>(addr)
    aborts_if false
    ensures ..S |~ ensures_of<function[Fn(Address, u64) -> u64](put)>(addr, 1, 1)
    ensures S..T |~ ensures_of<function[Fn(Address, u64) -> u64](put)>(addr, 2, result)
    ensures (S |~ global<R>(addr).value) == 1
    ensures (T |~ global<R>(addr).value) == global<R>(addr).value
    ensures result == 2
    modifies global<R>(addr)

  fun apply_labeled(f : Fn(Address, u64) -> u64, addr : Address, value : u64) -> u64 :=
    invoke(f, addr, value)
  spec apply_labeled where
    pragma opaque
    requires !aborts_of<f>(addr, value)
    aborts_if false
    modifies_of<f> *
    ensures ..S |~ ensures_of<f>(addr, value, result)
    ensures (S |~ global<R>(addr).value) == global<R>(addr).value
    modifies *

  fun caller_apply(addr : Address) -> u64 :=
    apply_labeled(function[Fn(Address, u64) -> u64](put), addr, 9)
  spec caller_apply where
    requires exists<R>(addr)
    aborts_if false
    ensures result == 9
    ensures global<R>(addr).value == 9
    modifies *

end LeanerLang.Tests.Check.Specifications.InvocationStateLabels

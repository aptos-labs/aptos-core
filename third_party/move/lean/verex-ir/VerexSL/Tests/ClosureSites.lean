-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-!
# Closure sites

The closure sites of a unit's link set (`closureSites`), grouped by
function type into mutual blocks: a site capturing a function value holds
its type's values, and a site inside a generic function appears once per
instantiation the link set uses, at closed rows.
-/

namespace VerexSL.Tests.ClosureSites

verex module 0x42::closure_sites where
  fun add(x : u64, y : u64) -> u64 := x + y

  fun apply(f : Fn(u64) -> u64, x : u64) -> u64 := invoke(f, x)

  fun inc(x : u64) -> u64 := apply(function[Fn(u64) -> u64](add, 1, _), x)

  -- Two types whose captures hold each other's values.
  fun via_bool(g : Fn(Bool) -> u64, x : u64) -> u64 := invoke(g, x > 0)

  fun via_u64(f : Fn(u64) -> u64, b : Bool) -> u64 := invoke(f, if b then 1 else 0)

  fun nested(x : u64) -> u64 :=
    apply(function[Fn(u64) -> u64](via_bool,
      function[Fn(Bool) -> u64](via_u64, function[Fn(u64) -> u64](add, 1, _), _), _), x)

  -- A generic site, at each instantiation used.
  fun first {T}(x : T, y : T) -> T := x

  fun fix_first {T}(x : T) -> Fn(T) -> T := function[Fn(T) -> T](first::<T>, x, _)

  fun first_u64(x : u64) -> u64 := invoke(fix_first::<u64>(x), x)

  fun first_bool(b : Bool) -> Bool := invoke(fix_first::<Bool>(b), b)

/--
info: group 0
  |bool| -> (bool)
    first<bool> mask 1 captures (bool)
group 1 (recursive)
  |u64| -> (u64)
    add mask 1 captures (u64)
    via_bool mask 1 captures (|bool| -> (u64))
    first<u64> mask 1 captures (u64)
  |bool| -> (u64)
    via_u64 mask 1 captures (|u64| -> (u64))
-/
#guard_msgs in
#verex_closure_sites 0x42::closure_sites

/-! The generated function values: a constructor per site, holding its
captures natively, a site capturing a function value holding the family
itself, and the instance the unit's theorems are stated at. -/

/--
info: site0 : |u64| -> (u64) := add mask 1 captures (u64)
site1 : |u64| -> (u64) := via_bool mask 1 captures (|bool| -> (u64))
site2 : |bool| -> (u64) := via_u64 mask 1 captures (|u64| -> (u64))
site3 : |u64| -> (u64) := first<u64> mask 1 captures (u64)
site4 : |bool| -> (bool) := first<bool> mask 1 captures (bool)
-/
#guard_msgs in
#verex_function_values 0x42::closure_sites

example : VerexIR.Proofs.Denote.FunctionCarriers.carrier «0x42».closure_sites.unit =
    «0x42».closure_sites.FunctionValue := rfl

/--
info: «0x42».closure_sites.FunctionValue.site1
  (captures :
    «0x42».closure_sites.FunctionValue «0x42».closure_sites.FunctionValue.row1
        «0x42».closure_sites.FunctionValue.sharing0 «0x42».closure_sites.FunctionValue.row0 ×
      Unit) :
  «0x42».closure_sites.FunctionValue «0x42».closure_sites.FunctionValue.row0 «0x42».closure_sites.FunctionValue.sharing0
    «0x42».closure_sites.FunctionValue.row0
-/
#guard_msgs in
#check «0x42».closure_sites.FunctionValue.site1

/--
info: '«0x42».closure_sites.functionCarriers' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms «0x42».closure_sites.functionCarriers

/--
info: '«0x42».closure_sites.FunctionValue.construct1' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms «0x42».closure_sites.FunctionValue.construct1

-- A generic site at its instantiation is its constructor.
/--
info: '«0x42».closure_sites.FunctionValue.construct3' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms «0x42».closure_sites.FunctionValue.construct3

end VerexSL.Tests.ClosureSites

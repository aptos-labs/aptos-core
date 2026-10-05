-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
# Mutable-reference parameters moved into locals

A parameter's reference moved into a local resolves where the function
leaves it, so a write through the local, or through a reborrow of it chosen
by a condition, is the parameter's final value.
-/

namespace LeanerLang.Tests.Check.References.MovedParameters

leaner module 0x42::moved_parameters where
  struct T has Key where
    x : u64

  fun through_local(l : &mut T) -> Unit := do
    let t := l
    let x := &mut t.x
    *x := 0
  spec through_local where
    ensures l.x == 0

  public fun diff_location(cond : Bool, a : Address, l : &mut T) -> Unit := do
    let x :=
      if cond then
        let t1 := &mut T[a]
        &mut t1.x
      else
        let t2 := l
        &mut t2.x
    *x := 0
  spec diff_location where
    aborts_if cond && !exists<T>(a)
    ensures if cond then global<T>(a).x == 0 else l.x == 0
    modifies *

end LeanerLang.Tests.Check.References.MovedParameters

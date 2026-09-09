-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-! Module with methods for safe memory manipulation. -/
leaner module 0x1::mem where
  /--
  Swap contents of two passed mutable references.

  Move prevents from having two mutable references to the same value,
  so `left` and `right` references are always distinct.
  -/
  public native fun swap {T}(left : &mut T, right : &mut T) -> Unit

  spec swap where
    pragma opaque
    aborts_if false
    ensures right == old(left)
    ensures left == old(right)

  /--
  Replace the value reference points to with the given new value,
  and return the value it had before.
  -/
  public fun replace {T}(ref : &mut T, mut new : T) -> T := do
    swap(ref, &mut new)
    return new

  spec replace where
    pragma opaque
    aborts_if false
    ensures result == old(ref)
    ensures ref == new

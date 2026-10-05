-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

/-! `cmp::compare`, specified only as intrinsic, is read by the Move
Prover's model: the structural order of its operands as the `Ordering`
variant less, equal, or greater, without abort. The generic native's result
reaches the caller through the transport to the caller's view, which keeps
its encoding, and a variant test of it is decided by the comparison. -/

leaner module std::cmp where
  enum Ordering has Copy, Drop where
    | Less
    | Equal
    | Greater

  public native fun compare {T}(first : &T, second : &T) -> Ordering

  spec compare where
    pragma intrinsic

leaner module 0x49::ordered where
  use std::cmp::Ordering
  use std::cmp::compare

  public fun below(a : u64, b : u64) -> Bool := compare(&a, &b) is Less

  spec below where
    aborts_if false
    ensures result == (a < b)

  public fun same(a : u64, b : u64) -> Bool := compare(&a, &b) is Equal

  spec same where
    aborts_if false
    ensures result == (a == b)

  public fun order(a : u64, b : u64) -> Ordering := compare(&a, &b)

  spec order where
    aborts_if false
    ensures result == compare(a, b)
    ensures a > b ==> result == new Ordering::Greater {}
    ensures a == b ==> result == new Ordering::Equal {}

-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

/-! An opaque specification function applied under a generic native's type
parameters. The native's contract names the function at the parameters, the
caller's clause at the concrete types, or at the caller's own parameters;
the type arguments are native types under the contract's family, which the
instantiation resolves to the caller's, so the two applications are one. -/

leaner module 0x48::opaque_generic where
  struct Point has Copy, Drop, Store where
    x : u64

  struct Box {T} has Copy, Drop, Store where
    inner : T

  native fun sum {T}(values : &Vector<T>) -> u64

  spec sum where
    pragma opaque
    aborts_if false
    ensures result == spec_sum(values)

  opaque spec fun spec_sum {T}(values : Vector<T>) : Int

  public fun total(points : &Vector<Point>) -> u64 := sum(points)

  spec total where
    aborts_if false
    ensures result == spec_sum(points)

  public fun total_weights(weights : &Vector<u64>) -> u64 := sum(weights)

  spec total_weights where
    aborts_if false
    ensures result == spec_sum(weights)

  public fun total_generic {T}(values : &Vector<T>) -> u64 := sum(values)

  spec total_generic where
    aborts_if false
    ensures result == spec_sum(values)

  public fun total_boxes {T}(boxes : &Vector<Box<T> >) -> u64 := sum(boxes)

  spec total_boxes where
    aborts_if false
    ensures result == spec_sum(boxes)

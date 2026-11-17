-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-!
# The cost of a growing sequence of map insertions

The same body at two sizes: three and six insertions of distinct literal
keys into a fresh intrinsic map, each insertion owing that its key is absent
from the map the earlier ones made, and the result read back at its last
key. prover-smoke pins the heartbeats of each target and the quotient of the
two (`scripts/prover-smoke-cost.pins`): a quotient that grows says the cost
grows faster with the number of insertions than it did. Two sizes detect
that; they bound nothing.
-/

namespace VerexSL.Tests.Cost.MapScaling

set_option Elab.async false

verex module 0x42::cost_map_scaling where
  pragma verify = false

  @[intrinsic_map]
  struct Table {K} {V} has Store where
    handle : Address

  native fun host_new {K} {V}() -> Table<K, V>
  native fun host_add {K} {V}(t : &mut Table<K, V>, k : K, v : V) -> Unit

  @[map_new (Table)]
  public fun new {K} {V}() -> Table<K, V> := host_new()
  @[map_add_no_override (Table)]
  public fun add {K} {V}(t : &mut Table<K, V>, k : K, v : V) -> Unit := host_add(t, k, v)

  @[map_spec_has_key (Table)]
  opaque spec fun has {K} {V}(t : Table<K, V>, k : K) : Bool
  @[map_spec_get (Table)]
  opaque spec fun get {K} {V}(t : Table<K, V>, k : K) : V
  @[map_spec_len (Table)]
  opaque spec fun size {K} {V}(t : Table<K, V>) : Int

  fun three() -> Table<u64, u64> := do
    let mut t := new::<u64, u64>()
    add(&mut t, 1, 10)
    add(&mut t, 2, 20)
    add(&mut t, 3, 30)
    t
  spec three where
    aborts_if false
    ensures has(result, 1) && has(result, 3)
    ensures get(result, 3) == 30
    ensures size(result) == 3

  fun six() -> Table<u64, u64> := do
    let mut t := new::<u64, u64>()
    add(&mut t, 1, 10)
    add(&mut t, 2, 20)
    add(&mut t, 3, 30)
    add(&mut t, 4, 40)
    add(&mut t, 5, 50)
    add(&mut t, 6, 60)
    t
  spec six where
    aborts_if false
    ensures has(result, 1) && has(result, 6)
    ensures get(result, 6) == 60
    ensures size(result) == 6

verify 0x42::cost_map_scaling::three
verify 0x42::cost_map_scaling::six

end VerexSL.Tests.Cost.MapScaling

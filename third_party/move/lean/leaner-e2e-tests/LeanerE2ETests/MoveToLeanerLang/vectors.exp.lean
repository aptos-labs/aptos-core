-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner module 0x42::vectors where
  fun push_pop() -> u64 := do
    let mut v := vector<u64>[]
    v := core.prim.pushVector(v, 1)
    v := core.prim.pushVector(v, 2)
    let x :=
      do
        let _t0 := &mut v
        if _t0.length == 0 then moveVectorError(2)
        let (_t1, _t2) := core.prim.removeVector(*_t0, _t0.length - 1)
        *_t0 := _t2
        _t1
    x + v.length

  fun contains_value() -> Bool := do
    let v := vector<u64>[1, 2, 3]
    core.prim.containsVector(v, 2)

  fun swap_values() -> u64 := do
    let mut v := vector<u64>[1, 2, 3]
    v := core.prim.swapVector(v, 0, 2)
    v[0]

  fun set_middle() -> u64 := do
    let mut values := vector<u64>[10, 20, 30]
    let middle := &mut values[1]
    *middle := 42
    values[1]

  fun sum(v : &Vector<u64>) -> u64 := do
    let total := 0
    let i := 0
    while v.length > i do
      total := total + v[i]
      i := i + 1
    total

  fun bytes() -> Vector<u8> := b"Move"

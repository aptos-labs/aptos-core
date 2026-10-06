-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner module 0x42::function_values where
  fun __lambda__1__add_two(x : u64) -> u64 := x + 1

  fun __lambda__2__add_two(x : u64) -> u64 := x + 1

  /--
  Construct a function value and invoke it.
  -/
  public fun add_one(value : u64) -> u64 := do
    let f := function[Fn(u64) -> u64](__lambda__1__add_one)
    invoke(f, value)

  spec add_one where
    modifies *

  fun __lambda__1__add_one(x : u64) -> u64 := x + 1

  /--
  Invoke function values that expanded inline helpers construct.
  -/
  public fun add_two(value : u64) -> u64 := do
    let value :=
      do
        let value := value
        let f := function[Fn(u64) -> u64](__lambda__1__add_two)
        invoke(f, value)
    let f := function[Fn(u64) -> u64](__lambda__2__add_two)
    invoke(f, value)

  spec add_two where
    modifies *

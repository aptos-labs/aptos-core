-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace core where
  pragma verify = false

  mod option where
    enum Option {T} has Copy where
      | None = 0
      | Some (0 : T) = 1

verex namespace reference_argument_adt where
  use core::option::Option

  struct Wrap {T} where
    0 : T

  enum Choice {T} where
    | None = 0
    | Some (0 : T) = 1

  fun wrapped(x : u32) -> u32 := do
    let w := new Wrap<&u32> { 0 := &x }
    let _4 := copy(w.0)
    *_4

  fun wrapped_mut(mut x : u32) -> u32 := do
    let w := new Wrap<&mut u32> { 0 := &mut x }
    let _4 := &mut *(w.0)
    *_4 := 1u32
    x

  fun chosen(x : u32) -> u32 := do
    let c := new Choice<&u32>::Some { 0 := &x }
    let mut _0 : u32
    match core.readDiscriminant[isize](c) with
      | 0 => _0 := 0u32
      | 1 => do
        let r := copy(c.0)
        _0 := *r
    _0

  fun optional(c : Bool, x : &u32) -> Option<&u32> :=
    if c then new Option<&u32>::Some { 0 := x } else new Option<&u32>::None {}

  fun optional_mut(o : Option<&mut u32>) -> Unit :=
    match core.readDiscriminant[isize](o) with
      | 1 => do
        let r := move(o.0)
        *r := 1u32
      | 0 => ()

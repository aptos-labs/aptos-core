-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace alloc where
  pragma verify = false

  mod boxed where
    impl Box where
      native fun new {T}(_1 : T) -> T

  mod alloc where

verex namespace boxes where
  use alloc::boxed::Box::new

  -- A box is its content: read, moved out, written through, and dropped.
  fun read(value : u32) -> u32 := do
    let _0 := value
    drop(value)
    _0

  fun read_nested(value : u32) -> u32 := do
    let _0 := value
    drop(value)
    _0

  fun read_field(value : &(u32, (u32, u32))) -> u32 := (value.1).0

  fun move_out(value : (u32, u32)) -> (u32, u32) := do
    let _0 := value
    drop(value)
    _0

  fun move_part(value : (u32, u32), flag : Bool) -> u32 := do
    let mut _0 : u32
    if flag then _0 := value.0 else _0 := value.1
    drop(value)
    _0

  fun write(value : &mut u32, other : u32) -> u32 := do
    *value := other
    *value

  fun write_field(value : &mut (u32, (u32, u32)), other : u32) -> Unit :=
    (value.1).1 := other

  fun write_through(value : &mut u32, other : u32) -> Unit := do
    let content := value
    *content := other

  fun write_owned(mut value : u32, other : u32) -> u32 := do
    value := other
    let _0 := value
    drop(value)
    _0

  fun dropped(_value : u32) -> u32 := do
    drop(_value)
    1u32

  fun make(value : u32) -> u32 := new(value)

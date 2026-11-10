-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace core where
  pragma verify = false

  mod option where
    enum Option {T} has Copy where
      | None = 0
      | Some (0 : T) = 1

  mod result where
    enum Result {T} {E} has Copy where
      | Ok (0 : T) = 0
      | Err (0 : E) = 1

verex namespace dependency_types where
  use core::option::Option
  use core::result::Result

  fun unwrap_or_zero(value : Option<u32>) -> u32 :=
    match core.readDiscriminant[isize](value) with
      | 0 => 0u32
      | 1 => value.0

  fun wrap(value : u32) -> Option<u32> := new Option<u32>::Some { 0 := value }

  fun ok_or_error(value : Result<u32, u8>) -> u32 :=
    match core.readDiscriminant[isize](value) with
      | 0 => (downcast value as Result::Ok).0
      | 1 => (downcast value as Result::Err).0 as u32

  fun keep(values : Vector<u32>) -> Vector<u32> := values

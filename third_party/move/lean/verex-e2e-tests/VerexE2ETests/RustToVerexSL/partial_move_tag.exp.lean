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

verex namespace partial_move_tag where
  use core::option::Option
  use core::result::Result

  -- Rustc reads the discriminant again after moving the Ok payload to decide
  -- whether the Err payload still needs dropping. That read must not copy the
  -- partially moved enum.
  fun take(value : Result<Vector<u8>, Vector<u8> >) -> Option<Vector<u8> > :=
    match core.readDiscriminant[isize](value) with
      | 0 => new Option<Vector<u8> >::Some {
        0 := move((downcast value as Result::Ok).0)
      }
      | 1 => new Option<Vector<u8> >::None {}

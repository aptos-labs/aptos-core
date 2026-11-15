-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace core where
  pragma verify = false

  mod cmp where
    enum Ordering has Copy where
      | Less = -1
      | Equal = 0
      | Greater = 1

    mod impls where
      trait impl «std::cmp::impls::<impl std::cmp::Ord for u64>» where
        native fun cmp(_1 : &u64, _2 : &u64) -> Ordering

verex namespace ordering where
  use core::cmp::Ordering
  use core::cmp::impls::«std::cmp::impls::<impl std::cmp::Ord for u64>»::cmp

  fun less(a : u64, b : u64) -> Bool := do
    let _3 := cmp(&a, &b)
    match core.readDiscriminant[i8](_3) with
      | -1 => true
      | _ => false

  enum Sign where
    | Negative = -1
    | Zero = 0
    | Positive = 1

  fun sign(a : i64) -> Sign := do
    let mut _0 : Sign
    if a < 0i64 then _0 := new Sign::Negative {}
    else
      if a == 0i64 then _0 := new Sign::Zero {} else _0 := new Sign::Positive {}
    _0

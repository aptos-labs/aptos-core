-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace core where
  pragma verify = false

  mod convert where
    struct Infallible has Copy where


  mod ops where
    mod control_flow where
      enum ControlFlow {B} {C} has Copy where
        | Continue (0 : C) = 0
        | Break (0 : B) = 1

  mod option where
    enum Option {T} has Copy where
      | None = 0
      | Some (0 : T) = 1

    trait impl «<std::option::Option<T> as std::ops::FromResidual<std::option::Option<std::convert::Infallible>>>» where
      native fun from_residual {T}(
        _1 : Option<convert::Infallible>
      ) -> Option<T>

    trait impl «<std::option::Option<T> as std::ops::Try>» where
      @[shape_only]
      native fun branch {T} {Residual} {Output}(
        _1 : Option<T>
      ) -> ops::control_flow::ControlFlow<Residual, Output>

verex namespace const_zero_sized where
  use core::convert::Infallible
  use core::ops::control_flow::ControlFlow
  use core::option::Option
  use core::option::«<std::option::Option<T> as std::ops::FromResidual<std::option::Option<std::convert::Infallible>>>»::from_residual
  use core::option::«<std::option::Option<T> as std::ops::Try>»::branch

  fun none() -> Option<Infallible> := new Option<Infallible>::None {}

  fun question(value : Option<u64>) -> Option<u64> := do
    let _2 := branch::<u64, Option<Infallible>, u64>(value)
    match core.readDiscriminant[isize](_2) with
      | 0 => new Option<u64>::Some {
        0 := (downcast _2 as ControlFlow::Continue).0
      }
      | 1 => from_residual::<u64>(new Option<Infallible>::None {})

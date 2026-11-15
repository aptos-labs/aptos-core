-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace closure_generics where
  fun generic {T}(x : T) -> T := do
    let g :=
      closure «generic::{closure#0}» {T} has Copy (x : T := x)
        fun call_once {T}(self : «generic::{closure#0}»<T>) -> T := move(self.x)
    «generic::{closure#0}»::call_once(g)

  struct W {T} where
    v : T

  impl W where
    fun take {T}(self : W<T>) -> T := do
      let g :=
        closure «W::<T>::take::{closure#0}» {T} has Copy (
          self__v : T := move(self.v)
        )
          fun call_once {T}(self : «W::<T>::take::{closure#0}»<T>) -> T :=
            move(self.self__v)
      «W::<T>::take::{closure#0}»::call_once(g)

-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexSL.Tests.Rendered.Support

/-!
# `library_option_map` as a frontend renders it

The namespaces below are the rendering of
`VerexE2ETests/SourceVerify/library_option_map.rs`, the items of its
`library_option_map.spec.lean` and the contracts its calls reach spliced in,
which the verification of the source renders and elaborates; its `verex
sources at` command is dropped and the diagnostics are those of
`library_option_map.exp`. Neither is edited here:
`scripts/prover-smoke-renderings.sh --write` writes this file again, and
without `--write` tells whether it is what the frontend renders. The crate's
own namespace, which the diagnostics are of, is the last command; the
namespaces before it are what it uses, and report nothing.
-/

verex namespace core where
  pragma verify = false

  mod option where
    enum Option {T} has Copy where
      | None = 0
      | Some (0 : T) = 1

    impl Option where
      native fun map {T} {U} {F}(
        _1 : Option<T>, _2 : F,
        __trait_FnOnce_call_once : Fn(F, T) -> U has Copy, Drop
      ) -> Option<U>




  mod option where
    impl Option where
















      spec map where
        ensures (result is Some) == (_1 is Some)
        ensures _1 is Some ==> ensures_of<__trait_FnOnce_call_once>(_2, _1.0, result.0)

#guard_rendering in
verex namespace library_option_map where
  use core::option::Option
  use core::option::Option::map

  fun bump(value : Option<u32>) -> Option<u32> :=
    map(
      value,
      closure «bump::{closure#0}» has Copy ()
        fun call_once(self : «bump::{closure#0}», x : u32) -> u32 := x + 1u32,
      function[Fn(«bump::{closure#0}», u32) -> u32 has Copy, Drop](«bump::{closure#0}»::call_once)
    )

  fun constant(value : Option<u32>) -> Option<u32> :=
    map(
      value,
      closure «constant::{closure#0}» has Copy ()
        fun call_once(self : «constant::{closure#0}», _x : u32) -> u32 := 7u32,
      function[Fn(«constant::{closure#0}», u32) -> u32 has Copy, Drop](«constant::{closure#0}»::call_once)
    )

  -- Copyright © Aptos Foundation
  -- SPDX-License-Identifier: Apache-2.0

  -- `Option::map` applies its argument where there is a payload, and states the
  -- result through the `FnOnce` evidence its declared bound supplies
  -- (`core/option.spec.lean`).

  impl «bump::{closure#0}» where
    spec call_once where
      pragma opaque
      ensures result == (x + 1) % 4294967296

  spec bump where
    ensures (result is Some) == (value is Some)
    ensures value is Some ==> result.0 == (value.0 + 1) % 4294967296

  impl «constant::{closure#0}» where
    spec call_once where
      pragma opaque
      ensures result == 7

  spec constant where
    ensures (result is Some) == (value is Some)
    ensures value is Some ==> result.0 == 7

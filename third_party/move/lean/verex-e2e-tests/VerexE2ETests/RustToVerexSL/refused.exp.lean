-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace refused where
  fun promoted() -> &u64 := &5u64

  @[refused = "the constant of type `u64` is the generic definition `refused::Limit::MAX`, which has no value before monomorphization"]
  native fun generic_limit {T}() -> u64

  @[refused = "call of `std::clone::Clone::clone` in `refused::generic_clone` at line 19:5 depends on a type parameter and needs trait evidence: rustc selects its implementation only at a concrete type"]
  native fun generic_clone {T}(value : &T) -> T

  fun reads_promoted() -> u64 := do
    let _1 := promoted()
    *_1

  fun reads_limit() -> u64 := plain(1u64)

  fun plain(a : u64) -> u64 :=
    if 2u64 == 0u64 then panic() else core.prim.checkedDividePanic(a, 2u64)

-- no denotation for `generic_limit`: a function without a body is not carried by the denotation
-- no denotation for `generic_clone`: a function without a body is not carried by the denotation

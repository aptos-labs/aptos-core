-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang
leaner module 0x99::invariant_behavior where
  struct Strategy has Copy, Drop, Store where
    apply : Fn(u64) -> u64 has Copy, Drop, Store
  spec Strategy where
    invariant ∀ (S : StateDomain; x : u64), !(S.. |~ aborts_of<this.apply>(x))
    invariant ∀ (S : StateDomain; x : u64), (S.. |~ result_of<this.apply>(x)) == x
  public fun identity(x : u64) -> u64 := x
  spec identity where
    pragma opaque
    aborts_if false
    ensures result == x
  fun make() -> Strategy := new Strategy { apply := function[Fn(u64) -> u64 has Copy, Drop, Store](identity) }

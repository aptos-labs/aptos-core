-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

set_option verex.sharedContinuations true

/-! A constructed certified enum is read by an inlined callee. Its plain
projection must compute before a variant's value is generalized away from
its dependent certificate. -/

verex module 0x42::variant_projection where
  enum Choice has Copy, Drop, Store where
    | Left (value : u64)
    | Right (value : u64)

  fun choose(right : Bool, value : u64) -> Choice :=
    if right then new Choice::Right { value } else new Choice::Left { value }
  spec choose where
    ensures result == if right then new Choice::Right { value } else new Choice::Left { value }

  fun score(choice : Choice) -> u64 :=
    match choice with
      | Choice::Left { value := value } => value + 1
      | Choice::Right { value := value } => value + 2
  spec score where
    ensures result == match choice with
      | Choice::Left { value := value } => value + 1
      | Choice::Right { value := value } => value + 2

  fun choose_and_score(right : Bool, value : u64) -> u64 :=
    score(choose(right, value))
  spec choose_and_score where
    ensures result == if right then value + 2 else value + 1

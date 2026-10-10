-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
-- Test category: source verification.

import Move

/-! The loose frame `modifies …, *`: the families a clause does not list are
unconstrained, as in the Move Prover's per-family `modifies` reading, while the
listed families stay closed at their addresses.  Also covers `assert!` in a
verified body, which the translator desugars to `if c then pure () else abort`. -/

namespace Tests.MovePrograms.LooseFrame

open Move
open scoped Move Move.Spec

module LooseFrame where

  struct Counter has Key where
    value : U64

  struct Ledger has Key where
    total : U64

  entry fun bump (addr : Address) : Action Unit := do
    let value ← &mut Counter[addr].value
    value := *value + 1

  -- `Counter` is closed at `addr`; every other family is unconstrained.
  spec bump (addr : Address) where
    requires existsAt<Counter>(addr) ∧ old(Counter[addr].value).toNat + 1 < U64.size;
    modifies Counter[addr], *;
    ensures Counter[addr].value = old(Counter[addr].value) + 1;
    aborts_if False

  verify bump

  entry fun guarded_bump (addr : Address) (limit : U64) : Action Unit := do
    let value ← &mut Counter[addr].value
    let current ← *value
    assert!(current < limit, 7)
    value := *value + 1

  -- The fully open frame `modifies *` states no frame at all.
  spec guarded_bump (addr : Address) (limit : U64) where
    requires existsAt<Counter>(addr);
    modifies *;
    ensures Counter[addr].value = old(Counter[addr].value) + 1;
    aborts_if limit ≤ old(Counter[addr].value) with 7

  verify guarded_bump

end Tests.MovePrograms.LooseFrame

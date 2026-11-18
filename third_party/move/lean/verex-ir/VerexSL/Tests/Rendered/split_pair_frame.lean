-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexSL.Tests.Rendered.Support

/-!
# `split_pair_frame` as a frontend renders it

The module below is the rendering of
`VerexE2ETests/SourceVerify/split_pair_frame.move` that the
source-verification driver writes to `.verex/split_pair_frame.move.lean`, its
`verex sources at` command dropped; the diagnostics are those of
`split_pair_frame.exp`. Neither is edited here:
`scripts/prover-smoke-renderings.sh --write` writes this file again, and
without `--write` tells whether it is what the frontend renders.
-/

#guard_rendering in
verex module 0x43::split_pair_frame where
  pragma proof_file = "VerexE2ETests/SourceVerify/split_pair_frame.proof.lean"

  -- Copyright © Aptos Foundation
  -- SPDX-License-Identifier: Apache-2.0
  -- A match on a pair of a resource and an argument: the split reduces
  -- constructor readings; `process` (with a closure invoke) and `plain` (without
  -- one) verify; the component-equation mechanism at the join is supported by
  -- source, while individual join leaves are not traced.
  enum Input where
    | Number (0 : u64)
    | Other

  enum State has Copy, Drop, Key where
    | Empty
    | Value (0 : u64)
    | Continuation (0 : Fn(u64) -> u64 has Copy, Drop, Store)

  fun process(s : &Signer, input : Input) -> Unit := do
    let addr := s.address
    match (move_from<State>(addr), input) with
      | (State::Empty {},
        Input::Number { 0 := x }) => move_to<State>(
        s, new State::Value { 0 := x }
      )
      | (State::Continuation { 0 := f },
        Input::Number { 0 := x }) => move_to<State>(
        s, new State::Value { 0 := invoke(f, x) }
      )
      | (_, _) => abort(1)

  spec process where
    pragma opaque
    pragma aborts_if_is_partial
    ensures [inferred] do
        let a := s.address
        (..S1 |~ remove<State>(a))
    ensures [inferred] old(global<State>(s.address)) is Empty && input is Number
        ==> (do
          let a := s.address
          S1.. |~ publish<State>(a, new State::Value { 0 := input.0 }))
    modifies global<State>(s.address)

  fun plain(s : &Signer, input : Input) -> Unit := do
    let addr := s.address
    match (move_from<State>(addr), input) with
      | (State::Value { 0 := y },
        Input::Number { 0 := x }) => move_to<State>(
        s, new State::Value { 0 := x + y }
      )
      | (_, _) => abort(1)

  spec plain where
    modifies *

-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-! A determined value can witness a clause once its certificate is proved.
Both automatic verification and the deterministic block closer use it. -/

namespace VerexSL.Tests.ConstructedWitnesses

example (p : Nat → Prop) (x : Nat) (held : p x) :
    ∃ y : { n : Nat // p n }, y.val = x := by
  verex_witness

example (p : Nat → Prop) (x : Nat) (held : p x) :
    ∃ y : { n : Nat // p n }, y.val = x := by
  verex_block_decide

-- A determined row does not supply its missing certificate.
example : True := by
  fail_if_success
    have : ∃ y : { n : Nat // n = 1 }, y.val = 2 := by
      verex_witness
  fail_if_success
    have : ∃ y : { n : Nat // n = 1 }, y.val = 2 := by
      verex_block_decide
  trivial

-- The whole clause must hold at the constructed value.
example : True := by
  fail_if_success
    have : ∃ y : { n : Nat // True }, y.val = 1 ∧ y.val = 2 := by
      verex_witness
  fail_if_success
    have : ∃ y : { n : Nat // True }, y.val = 1 ∧ y.val = 2 := by
      verex_block_decide
  trivial

verex module 0x42::constructed_witnesses where
  pragma verify = false
  enum State has Key where
    | Empty
    | Value (n : u64)

  fun init(s : &Signer) -> Unit := move_to<State>(s, new State::Empty {})
  spec init where
    modifies global<State>(s.address)
    ensures publish<State>(s.address, new State::Empty {})
    aborts_if exists<State>(s.address)

  fun put(s : &Signer, n : u64) -> Unit := move_to<State>(s, new State::Value { n })
  spec put where
    modifies global<State>(s.address)
    ensures publish<State>(s.address, new State::Value { n })
    aborts_if exists<State>(s.address)

verify 0x42::constructed_witnesses::init
verify 0x42::constructed_witnesses::put by lean =>
  verex_obligations
  verex_on
  | ensures_1 => verex_prepare; verex_block_decide
  | _ => verex_decide

end VerexSL.Tests.ConstructedWitnesses

-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang
leaner module 0x99::stored_frames where
  struct Counter has Key where
    value : u64
  enum State has Key, Copy, Drop where
    | Empty
    | Pending (f : Fn(u64) -> u64 has Copy, Drop, Store)
  public fun identity(x : u64) -> u64 := x
  spec identity where
    pragma opaque
    aborts_if false
    ensures result == x
  fun make() -> State := new State::Pending {
    f := function[Fn(u64) -> u64 has Copy, Drop, Store](identity) }
  fun apply_state(s : State, x : u64) -> u64 :=
    match s with
    | State::Empty {} => x
    | State::Pending { f := f } => invoke(f, x)
  spec apply_state where
    pragma aborts_if_is_partial
  fun consume(owner : Address, x : u64) -> u64 := do
    let state := move_from<State>(owner)
    apply_state(state, x)
  spec consume where
    pragma aborts_if_is_partial
    modifies global<State>(owner)
  fun publish(s : &Signer) -> Unit := move_to<State>(s, new State::Pending {
    f := function[Fn(u64) -> u64 has Copy, Drop, Store](identity) })
  spec publish where
    pragma aborts_if_is_partial
    modifies global<State>(s.address)

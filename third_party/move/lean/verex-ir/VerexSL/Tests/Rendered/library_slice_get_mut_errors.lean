-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexSL.Tests.Rendered.Support

/-!
# `library_slice_get_mut_errors` as a frontend renders it

The namespaces below are the rendering of
`VerexE2ETests/SourceVerify/library_slice_get_mut_errors.rs`, the items of its
`library_slice_get_mut_errors.spec.lean` and the contracts its calls reach
spliced in, which the verification of the source renders and elaborates; its
`verex sources at` command is dropped and the diagnostics are those of
`library_slice_get_mut_errors.exp`. Neither is edited here:
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

  mod primitive where
    impl slice where
      @[shape_only]
      native fun get_mut {T} {I} {Output}(
        _1 : &mut Slice<T>, _2 : I
      ) -> option::Option<&mut Output>




  mod primitive where
    impl slice where






      spec get_mut at {T} {usize} {T} where
        ensures _2 < old(_1).length ==> result is Some
        ensures result is Some ==> result.0 == old(_1)[_2]
        ensures _2 >= old(_1).length ==> result is None
        ensures _2 < old(_1).length ==> _1 == update(old(_1), _2, final(result.0))
        ensures _2 >= old(_1).length ==> _1 == old(_1)

/--
error: the specification clause `ensures i < old(values).length ==>
      values[i] == old(values)[i]` is not established
---
error: verex verification failed: the automatic verification of `preserved` failed; provide a proof in the module:
  verify preserved by lean =>
    verex_obligations
    verex_on
    | ensures_2 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_rendering in
verex namespace library_slice_get_mut_errors where
  use core::option::Option
  use core::primitive::slice::get_mut

  fun preserved(values : &mut Slice<u32>, i : usize) -> Unit := do
    let _3 := get_mut::<u32, usize, u32>(values, i)
    match core.readDiscriminant[isize](_3) with
      | 1 => do
        let x := move(_3.0)
        *x := *x + 1u32
      | 0 => ()

  fun increment(values : &mut Slice<u32>, i : usize) -> Unit := do
    let _3 := get_mut::<u32, usize, u32>(values, i)
    match core.readDiscriminant[isize](_3) with
      | 1 => do
        let x := move(_3.0)
        *x := *x + 1u32
      | 0 => ()

  -- Copyright © Aptos Foundation
  -- SPDX-License-Identifier: Apache-2.0

  -- The write through the yielded reference reaches the slice, so claiming the
  -- element is preserved is refuted. Were the contract still to carry the
  -- premise that the payload's prophecy is its current value, this would verify
  -- vacuously instead.
  spec preserved where
    ensures ∀ (k in 0 .. values.length), k != i ==> values[k] == old(values)[k]
    ensures i < old(values).length ==>
      values[i] == old(values)[i] -- error: the write through the reference lands

  -- Reading through the yielded reference before writing relates both the
  -- untouched positions and the written one to the element automatically, so
  -- `increment` verifies cleanly beside the refuted `preserved` above.
  spec increment where
    ensures ∀ (k in 0 .. values.length), k != i ==> values[k] == old(values)[k]
    ensures i < old(values).length ==>
      values[i] == (old(values)[i] + 1) % 4294967296

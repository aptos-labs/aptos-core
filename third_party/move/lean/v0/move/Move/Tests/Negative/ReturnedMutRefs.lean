-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
-- Test category: expected failures and diagnostics.

import Move

/-!
# Rejected mutable-reference result boundaries

These programs or interfaces cannot be represented by the single path-free
mutation result. They must fail with source-positioned diagnostics rather than
later generated-code elaboration errors.
-/

namespace Move.Tests.Negative

open Move
open scoped Move Move.Spec

module ReturnedMutRefs where

  fun return_local_reference : Action (&mut U64) := do
    let mut owner : U64 := 0
    let escaped ← &mut owner
    pure escaped

  /--
  error: borrow safety error `escaped`: a returned reference must derive from a reference parameter
  -/
  #guard_msgs in
  spec return_local_reference where
    ensures True;
    aborts_if False

  struct EscapedResource has Key where
    value : U64

  fun return_global_reference (address : Address) : Action (&mut U64) := do
    let escaped ← &mut EscapedResource[address].value
    pure escaped

  /--
  error: borrow safety error `escaped`: a returned reference must derive from a reference parameter
  -/
  #guard_msgs in
  spec return_global_reference (address : Address) where
    ensures True;
    aborts_if False

  fun choose_input (takeLeft : Bool) (left : &mut U64) (right : &mut U64) :
      Action (&mut U64) := do
    if takeLeft then pure left else pure right

  spec choose_input (takeLeft : Bool) (left : &mut U64) (right : &mut U64) where
    ensures True;
    aborts_if False

  fun use_suspended_input (left : &mut U64) (right : &mut U64) : Action Unit := do
    let returned ← choose_input true left right
    left := 1
    returned := 2

  /--
  error: borrow safety error `left`: reference is suspended by an active child borrow
  -/
  #guard_msgs in
  spec use_suspended_input (left : &mut U64) (right : &mut U64) where
    ensures True;
    aborts_if False

  fun return_reference_pair (left : &mut U64) (right : &mut U64) :
      Action ((&mut U64) × (&mut U64)) := do
    pure (left, right)

  spec return_reference_pair (left : &mut U64) (right : &mut U64) where
    ensures True;
    aborts_if False

  fun use_pair_input (left : &mut U64) (right : &mut U64) : Action Unit := do
    let (returnedLeft, returnedRight) ← return_reference_pair left right
    left := 1
    returnedLeft := 2
    returnedRight := 3

  /--
  error: borrow safety error `left`: reference is suspended by an active child borrow
  -/
  #guard_msgs in
  spec use_pair_input (left : &mut U64) (right : &mut U64) where
    ensures True;
    aborts_if False

  native fun native_return_reference (slot : &mut U64) : Action (&mut U64)

  spec native_return_reference (slot : &mut U64) where
    ensures True;
    aborts_if False

  fun call_native_return_reference (slot : &mut U64) : Action Unit := do
    let returned ← native_return_reference slot
    returned := 3

  /--
  error: a native or body-less opaque mutable-reference result requires an explicit mutation-level summary `Move.Tests.Negative.ReturnedMutRefs.native_return_reference.mutationSpec`
  -/
  #guard_msgs in
  spec call_native_return_reference (slot : &mut U64) where
    ensures True;
    aborts_if False

end Move.Tests.Negative

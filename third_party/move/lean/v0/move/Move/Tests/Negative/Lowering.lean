-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
-- Test category: expected failures and diagnostics.

import Move

/-! Diagnostic fixtures for invalid low-level compiler inputs. -/

namespace Tests.MovePrograms.Calls.Rejection

open Move
open scoped Move Move.Compiler Move.Spec

/--
error: `move_source` is compiler-internal; use `fun` to retain a source body
-/
#guard_msgs in
@[move_entry, move_source (Action U64, 0, "pure 0")]
def rejectedAuthoredRetainedSource : Action U64 :=
  pure 0

@[move_struct]
structure RecursiveType where
  next : RecursiveType
  deriving Copy, Drop, Store

/--
error: recursive Move type `Tests.MovePrograms.Calls.Rejection.RecursiveType` is not supported by the prototype
-/
#guard_msgs in
def rejectedRecursiveType : MoveModel.IR.Module :=
  module% "RejectedRecursiveType" structs [RecursiveType] functions []

@[move_fun]
def omittedHelper (value : U64) : U64 := value + value

@[move_entry]
def callsOmittedHelper (value : U64) : Action U64 := do
  pure (omittedHelper value)

/--
error: while compiling Move function `Tests.MovePrograms.Calls.Rejection.callsOmittedHelper`:
Move function `Tests.MovePrograms.Calls.Rejection.omittedHelper` has no enclosing `module` identity
-/
#guard_msgs in
def rejected : MoveModel.IR.Module :=
  module% "RejectedCalls" structs [] functions [callsOmittedHelper]

@[noinline]
def ordinaryHelper (value : U64) : U64 := value + value

@[move_entry]
def callsOrdinaryHelper (value : U64) : Action U64 := do
  pure (ordinaryHelper value)

/--
error: while compiling Move function `Tests.MovePrograms.Calls.Rejection.callsOrdinaryHelper`:
unsupported call `Tests.MovePrograms.Calls.Rejection.ordinaryHelper` while compiling Move function
-/
#guard_msgs in
def rejectedOrdinary : MoveModel.IR.Module :=
  module% "RejectedOrdinaryCall" structs [] functions [callsOrdinaryHelper]

@[move_fun]
partial def nonTailContinue (value : U64) : U64 :=
  if value < 1 then 0 else value + continue nonTailContinue (value - 1)

/--
error: while compiling Move function `Tests.MovePrograms.Calls.Rejection.nonTailContinue`:
`continue` must mark a direct self-call in tail position
-/
#guard_msgs in
def rejectedNonTailContinue : MoveModel.IR.Module :=
  module% "RejectedNonTailContinue" structs [] functions [nonTailContinue]

@[move_fun]
def continuedOther (value : U64) : U64 := value + value

@[move_fun]
def nonSelfContinue (value : U64) : U64 := continue continuedOther value

/--
error: while compiling Move function `Tests.MovePrograms.Calls.Rejection.nonSelfContinue`:
`continue` in `Tests.MovePrograms.Calls.Rejection.nonSelfContinue` calls `Tests.MovePrograms.Calls.Rejection.continuedOther`; only a direct self-call can become a loop
-/
#guard_msgs in
def rejectedNonSelfContinue : MoveModel.IR.Module :=
  module% "RejectedNonSelfContinue" structs [] functions [continuedOther, nonSelfContinue]

module SourceVerificationRejection where

  @[move_struct]
  structure Vault where
    value : U64
    deriving Key

  fun unmodeled_receiver_get (values : Move.Vector U64) (index : U64) : Action U64 :=
    pure (values.get index)

  spec unmodeled_receiver_get (values : Move.Vector U64) (index : U64) where
    ensures result = values.toList[index.toNat]?.getD 0;
    aborts_if ¬index.toNat < values.toList.length

  verify unmodeled_receiver_get

  namespace Vector

    fun insert (_slot : &mut Move.Vector U64) (_index value : U64) :
        Action Unit := do
      abort value

  end Vector

  fun calls_receiver_style_vector_insert : Action Unit := do
    let values : Move.Vector U64 := vector![1]
    let slot ← &mut values
    slot.insert 0 7

  spec calls_receiver_style_vector_insert where
    ensures True;
    aborts_if False

  verify calls_receiver_style_vector_insert

  @[move_struct]
  structure Other where
    value : U64
    deriving Key

  fun touches_vault (addr : Address) : Action Bool :=
    existsAt Vault addr

  /--
  error: resource `Other` is not used by the specified function
  -/
  #guard_msgs in
  spec touches_vault (addr : Address) where
    ensures existsAt<Other>(addr);
    aborts_if False

  mutual
    partial fun ping (value : U64) : U64 :=
      if value == 0 then 0 else pong (value - 1)

    partial fun pong (value : U64) : U64 :=
      if value == 0 then 1 else ping (value - 1)
  end

  fun calls_mutual (value : U64) : Action U64 :=
    pure (ping value)

  spec calls_mutual (value : U64) where
    ensures True;
    aborts_if False

end Tests.MovePrograms.Calls.Rejection

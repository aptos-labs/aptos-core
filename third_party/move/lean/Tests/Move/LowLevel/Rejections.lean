-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move

/-! Diagnostic fixtures for invalid low-level compiler inputs. -/

namespace Tests.MovePrograms.Calls.Rejection

open Move
open MoveModel.Frontend.XIR
open scoped Move Move.Compiler Move.Spec

@[move_struct]
structure RecursiveType where
  next : RecursiveType
  deriving Copy, Drop, Store

/--
error: recursive Move type `Tests.MovePrograms.Calls.Rejection.RecursiveType` is not supported by the prototype
-/
#guard_msgs in
def rejectedRecursiveType : MModule :=
  move_module% "RejectedRecursiveType" structs [RecursiveType] functions []

@[move_fun]
def omittedHelper (value : U64) : U64 := value + value

@[move_entry]
def callsOmittedHelper (value : U64) : Action U64 := do
  pure (omittedHelper value)

/--
error: while compiling Move function `Tests.MovePrograms.Calls.Rejection.callsOmittedHelper`:
Move function `Tests.MovePrograms.Calls.Rejection.omittedHelper` has no enclosing `move_module` identity
-/
#guard_msgs in
def rejected : MModule :=
  move_module% "RejectedCalls" structs [] functions [callsOmittedHelper]

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
def rejectedOrdinary : MModule :=
  move_module% "RejectedOrdinaryCall" structs [] functions [callsOrdinaryHelper]

@[move_fun]
partial def nonTailContinue (value : U64) : U64 :=
  if value < 1 then 0 else value + continue nonTailContinue (value - 1)

/--
error: while compiling Move function `Tests.MovePrograms.Calls.Rejection.nonTailContinue`:
`continue` must mark a direct self-call in tail position
-/
#guard_msgs in
def rejectedNonTailContinue : MModule :=
  move_module% "RejectedNonTailContinue" structs [] functions [nonTailContinue]

@[move_fun]
def continuedOther (value : U64) : U64 := value + value

@[move_fun]
def nonSelfContinue (value : U64) : U64 := continue continuedOther value

/--
error: while compiling Move function `Tests.MovePrograms.Calls.Rejection.nonSelfContinue`:
`continue` in `Tests.MovePrograms.Calls.Rejection.nonSelfContinue` calls `Tests.MovePrograms.Calls.Rejection.continuedOther`; only a direct self-call can become a loop
-/
#guard_msgs in
def rejectedNonSelfContinue : MModule :=
  move_module% "RejectedNonSelfContinue" structs [] functions [continuedOther, nonSelfContinue]

move_module SourceVerificationRejection where

  @[move_struct]
  structure Vault where
    value : U64
    deriving Key

  fun unmodeledMoveFrom (address : Address) : Action Vault :=
    moveFrom Vault address

  /--
  error: automatic source specifications do not yet model `Move.moveFrom`; provide an explicit `sourceSpec` or omit `verify`
  -/
  #guard_msgs in
  spec unmodeledMoveFrom (address : Address) where
    requires True;
    ensures True;
    aborts_if False

  fun unmodeledVectorGet (values : Move.Vector U64) (index : U64) : Action U64 :=
    pure (Move.Vector.get values index)

  /--
  error: automatic source specifications do not yet model `Move.Vector.get`; provide an explicit `sourceSpec` or omit `verify`
  -/
  #guard_msgs in
  spec unmodeledVectorGet (values : Move.Vector U64) (index : U64) where
    requires True;
    ensures True;
    aborts_if False

  fun arithmeticCondition (value : U64) : Action U64 := do
    if value + 1 < 2 then
      pure value
    else
      pure 0

  /--
  error: automatic source specifications do not yet support arithmetic in this context; bind it to a local first
  -/
  #guard_msgs in
  spec arithmeticCondition (value : U64) where
    requires True;
    ensures True;
    aborts_if False

  fun plusOne (value : U64) : U64 := value + 1

  fun callsPureHelper (value : U64) : Action U64 :=
    pure (plusOne value)

  /--
  error: automatic source specifications do not yet model pure Move callee `Tests.MovePrograms.Calls.Rejection.SourceVerificationRejection.plusOne`; inline it or omit `verify`
  -/
  #guard_msgs in
  spec callsPureHelper (value : U64) where
    requires True;
    ensures True;
    aborts_if False

end Tests.MovePrograms.Calls.Rejection

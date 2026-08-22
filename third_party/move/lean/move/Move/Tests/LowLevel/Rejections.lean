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
def rejected : MModule :=
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
def rejectedOrdinary : MModule :=
  module% "RejectedOrdinaryCall" structs [] functions [callsOrdinaryHelper]

@[move_fun]
partial def nonTailContinue (value : U64) : U64 :=
  if value < 1 then 0 else value + continue nonTailContinue (value - 1)

/--
error: while compiling Move function `Tests.MovePrograms.Calls.Rejection.nonTailContinue`:
`continue` must mark a direct self-call in tail position
-/
#guard_msgs in
def rejectedNonTailContinue : MModule :=
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
def rejectedNonSelfContinue : MModule :=
  module% "RejectedNonSelfContinue" structs [] functions [continuedOther, nonSelfContinue]

/--
error: `break` must be nested inside a loop
-/
#guard_msgs in
def bareBreak (n : U64) : U64 := Id.run do
  break
  n

/--
error: `continue` must be nested inside a loop
-/
#guard_msgs in
def bareContinue (n : U64) : U64 := Id.run do
  continue
  n

/--
error: `continue f` cannot appear inside `loop` / `while`; use `continue`
-/
#guard_msgs in
partial def continueCallInside (n : U64) : U64 := Id.run do
  let mut n := n
  while 0 < n do
    continue continueCallInside (n - 1)
  n

/--
error: unknown loop label `missing`
-/
#guard_msgs in
def unknownLabel (n : U64) : U64 := Id.run do
  loop@outer
    break@missing
  n

/--
error: duplicate active loop label `outer`
-/
#guard_msgs in
def duplicateLabel (n : U64) : U64 := Id.run do
  loop@outer
    loop@outer
      break
  n

/--
error: Unknown identifier `Move.loopEnter`
-/
#guard_msgs in
def forgedLoopMarker : Nat :=
  Move.loopEnter 0 0 0 ()

module SourceVerificationRejection where

  @[move_struct]
  structure Vault where
    value : U64
    deriving Key

  fun unmodeled_receiver_get (values : Move.Vector U64) (index : U64) : Action U64 :=
    pure (values.get index)

  /--
  error: automatic source specifications require fully qualified `Move.Vector.get`, `Move.Vector.set`, `Move.Vector.insert`, or `Move.Vector.remove`
  -/
  #guard_msgs in
  spec unmodeled_receiver_get (values : Move.Vector U64) (index : U64) where
    ensures True;
    aborts_if False

  fun short_circuit_arithmetic (value : U64) : Action U64 := do
    if value == 0 && value + 1 == 2 then pure value else pure 0

  /--
  error: automatic source specifications cannot sequence this operation here, where its evaluation is conditional; bind it to a local first
  -/
  #guard_msgs in
  spec short_circuit_arithmetic (value : U64) where
    ensures True;
    aborts_if False

  namespace Vector

    fun insert (_slot : &mut Move.Vector U64) (_index value : U64) :
        Action Unit := do
      abort value

  end Vector

  fun calls_receiver_style_vector_insert : Action Unit := do
    let values : Move.Vector U64 := vector![1]
    let slot ← &mut values
    slot.insert 0 7

  /--
  error: automatic source specifications require fully qualified `Move.Vector.insert` or `Move.Vector.remove`
  -/
  #guard_msgs in
  spec calls_receiver_style_vector_insert where
    ensures True;
    aborts_if False

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
      if value < 1 then 0 else pong (value - 1)

    partial fun pong (value : U64) : U64 :=
      if value < 1 then 1 else ping (value - 1)
  end

  fun calls_mutual (value : U64) : Action U64 :=
    pure (ping value)

  /--
  error: mutually recursive Move functions are not yet supported by automatic source specifications (`Tests.MovePrograms.Calls.Rejection.SourceVerificationRejection.ping`)
  -/
  #guard_msgs in
  spec calls_mutual (value : U64) where
    ensures True;
    aborts_if False

end Tests.MovePrograms.Calls.Rejection

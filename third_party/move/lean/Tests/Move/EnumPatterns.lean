-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move
import Tests.Common

/-! Nested enum patterns through Leaner lowering and the Move IR interpreter. -/

namespace Tests.MovePrograms

open Move
open MoveModel.Frontend.XIR
open scoped Move Move.Compiler Move.Spec

move_module EnumPatterns where

  @[move_enum]
  inductive Atom where
    | none
    | number (value : U64)
    deriving Copy, Drop, Store

  @[move_enum]
  inductive Envelope where
    | empty
    | one (value : Atom)
    | two (left right : Atom)
    deriving Copy, Drop, Store

  /-! ## Functions -/

  fun nestedTotal (envelope : Envelope) : U64 :=
    match envelope with
    | .one (.number value) => value
    | .two (.number left) (.number right) => left + right
    | _ => 0

  spec nestedTotal (envelope : Envelope) where
    ensures
      result =
        match envelope with
        | .one (.number value) => value
        | .two (.number left) (.number right) => left + right
        | _ => 0

  fun oneNumber (value : U64) : U64 :=
    nestedTotal (.one (.number value))

  spec oneNumber (value : U64) where
    ensures result = value

  fun oneNone : U64 :=
    nestedTotal (.one .none)

  spec oneNone where
    ensures result = 0

  fun twoNumbers (left right : U64) : U64 :=
    nestedTotal (.two (.number left) (.number right))

  spec twoNumbers (left : U64) (right : U64) where
    ensures result = left + right

  fun leftMissing (right : U64) : U64 :=
    nestedTotal (.two .none (.number right))

  spec leftMissing (right : U64) where
    ensures result = 0

  fun rightMissing (left : U64) : U64 :=
    nestedTotal (.two (.number left) .none)

  spec rightMissing (left : U64) where
    ensures result = 0

  /-! ## Proofs -/

  verify nestedTotal
  verify oneNumber
  verify oneNone
  verify twoNumbers
  verify leftMissing
  verify rightMissing

  /-! ## Tests -/

  def compiled : MModule := move_module% "EnumPatternsTest"

  private def run := Tests.run compiled

  #test run "oneNumber" [] [.u64 7] = Tests.okU64 7
  #test run "oneNone" [] [] = Tests.okU64 0
  #test run "twoNumbers" [] [.u64 4, .u64 5] = Tests.okU64 9
  #test run "leftMissing" [] [.u64 5] = Tests.okU64 0
  #test run "rightMissing" [] [.u64 4] = Tests.okU64 0

end Tests.MovePrograms

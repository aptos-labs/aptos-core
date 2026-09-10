-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
-- Test category: Move language.

import Move
import MoveModel.Tests.Common

/-! Nested enum patterns through Leaner lowering and the Move IR interpreter. -/

namespace Tests.MovePrograms

open Move
open scoped Move Move.Compiler Move.Spec

module EnumPatterns where

  enum Atom has Copy, Drop, Store where
    | None
    | Number (value : U64)

  enum Envelope has Copy, Drop, Store where
    | Empty
    | One (value : Atom)
    | Two (left right : Atom)

  /-! ## Functions -/

  fun nested_total (envelope : Envelope) : U64 :=
    match envelope with
    | .One (.Number value) => value
    | .Two (.Number left) (.Number right) => left + right
    | _ => 0

  spec nested_total (envelope : Envelope) where
    ensures
      result =
        match envelope with
        | .One (.Number value) => value
        | .Two (.Number left) (.Number right) => left + right
        | _ => 0

  fun one_number (value : U64) : U64 :=
    nested_total (.One (.Number value))

  spec one_number (value : U64) where
    ensures result = value

  fun one_none : U64 :=
    nested_total (.One .None)

  spec one_none where
    ensures result = 0

  fun two_numbers (left right : U64) : U64 :=
    nested_total (.Two (.Number left) (.Number right))

  spec two_numbers (left : U64) (right : U64) where
    ensures result = left + right

  fun left_missing (right : U64) : U64 :=
    nested_total (.Two .None (.Number right))

  spec left_missing (right : U64) where
    ensures result = 0

  fun right_missing (left : U64) : U64 :=
    nested_total (.Two (.Number left) .None)

  spec right_missing (left : U64) where
    ensures result = 0

  /-! ## Proofs -/

  verify nested_total
  verify one_number
  verify one_none
  verify two_numbers
  verify left_missing
  verify right_missing

  /-! ## Tests -/

  def compiled : MoveModel.IR.Module := lowerToIR ``Tests.MovePrograms.EnumPatterns

  private def run := Tests.run compiled

  #test run "one_number" [] [.u64 7] = Tests.okU64 7
  #test run "one_none" [] [] = Tests.okU64 0
  #test run "two_numbers" [] [.u64 4, .u64 5] = Tests.okU64 9
  #test run "left_missing" [] [.u64 5] = Tests.okU64 0
  #test run "right_missing" [] [.u64 4] = Tests.okU64 0

end Tests.MovePrograms

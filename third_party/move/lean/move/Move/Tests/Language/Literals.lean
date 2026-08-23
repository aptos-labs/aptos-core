-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
-- Test category: Move language.

import Move
import MoveModel.Tests.Common

namespace Tests.MovePrograms

open Move
open scoped Move Move.Compiler Move.Spec

module Literals where

  fun fixed_address : Address :=
    @0xCAFE

  spec fixed_address where
    ensures result = @0xCAFE;
    aborts_if False

  verify fixed_address

  fun ascii_bytes : Vector U8 :=
    b"Move"

  fun hex_bytes : Vector U8 :=
    x"DEAD00"

  fun classify_bytes (value : Vector U8) : U64 :=
    match value with
    | b"go" => 1
    | x"00" => 2
    | _ => 0

  spec classify_bytes (value : Vector U8) where
    ensures result = match value with
      | b"go" => 1
      | x"00" => 2
      | _ => 0;
    aborts_if False

  verify classify_bytes

  fun abort_message : Action Unit := do
    abort b"something went wrong"

  spec abort_message where
    ensures False;
    aborts_if True with Move.unspecifiedAbortCode

  def compiled : MoveModel.IR.Module := lowerToIR ``Tests.MovePrograms.Literals
  private def run := Tests.run compiled

  #test run "fixed_address" [] [] = Tests.okVals [.address 0xCAFE]
  #test run "ascii_bytes" [] [] =
    Tests.okVals [.vector [.int 77, .int 111, .int 118, .int 101]]
  #test run "hex_bytes" [] [] =
    Tests.okVals [.vector [.int 0xDE, .int 0xAD, .int 0]]
  #test run "classify_bytes" [] [.vector [.int 103, .int 111]] = Tests.okU64 1
  #test run "classify_bytes" [] [.vector [.int 0]] = Tests.okU64 2
  #test run "classify_bytes" [] [.vector []] = Tests.okU64 0
  #test run "abort_message" [] [] = Tests.aborted 0xCA26CBD9BE0B0000

end Tests.MovePrograms

-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
-- Test category: compiler integration.

import Move
import MoveModel.Tests.Common

namespace Tests.MovePrograms

open Move
open scoped Move Move.Compiler

module InlinePackage where

  friend 0x0::Trusted;

  native fun host_hash (x : U64) : U64

  inline fun increment (x : U64) : U64 :=
    x + 1

  package fun package_value : U64 :=
    41

  fun call_inline : U64 :=
    increment package_value

  #[test, expected_failure (abort_code 7)]
  fun attributed_test : U64 :=
    0

  def compiled : MoveModel.IR.Module := lowerToIR ``Tests.MovePrograms.InlinePackage
  private def run := Tests.run compiled

  #guard compiled.numFuns == 4
  #guard compiled.friends == [{ address := 0, moduleName := "Trusted" }]
  #guard (List.range compiled.numFuns).any fun i =>
    (compiled.funMeta i).any (fun info =>
      info.name == "package_value" && info.visibility == .friend)
  #guard (List.range compiled.numFuns).any fun i =>
    (compiled.funMeta i).any (fun info => info.name == "host_hash") &&
      (compiled.program.funs i).any (fun decl => decl.native)
  #guard (List.range compiled.numFuns).any fun i =>
    (compiled.funMeta i).any (fun info =>
      info.name == "attributed_test" && info.attributes.length == 2)
  #test run "call_inline" [] [] = Tests.okVals [.int 42]

end Tests.MovePrograms

-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
-- Test category: Move language.

import Move
import MoveModel.Tests.Common

/-! Source attributes before `struct`, `enum`, and `fun` keywords: user
provided instances are recorded as structured metadata, well-known internal
names keep desugaring to the persistent tag attributes. -/

namespace Tests.MovePrograms

open Move
open scoped Move Move.Compiler

module Attributes where

  @[resource_group (scope global)]
  struct Registry has Key where
    value : U64

  /-- A doc comment combines with an attribute list. -/
  @[resource_group_member (Registry), custom_marker]
  enum Mode has Drop where
    | Idle
    | Busy (level : U64)

  @[view]
  fun peek (x : U64) : U64 := x + 0

  @[randomness 7, lint.skip]
  entry fun act (addr : Address) : Action Unit := do
    let value ← &mut Registry[addr].value
    value := *value + 1

  -- Well-known internal names in attribute position keep their meaning and
  -- leave no user metadata behind.
  @[move_public] fun compatPublic (x : U64) : U64 := x

  def compiled : MoveModel.IR.Module := lowerToIR ``Tests.MovePrograms.Attributes

namespace Attributes

open MoveModel.IR

private def structAttributes (name : String) :=
  (compiled.structMeta? name).map (·.attributes)

private def funMeta (name : String) :=
  compiled.funMeta? name

#guard structAttributes "Registry" ==
  some [{ name := "resource_group", args := [.name "scope" [.name "global" []]] }]
#guard structAttributes "Mode" ==
  some [
    { name := "resource_group_member", args := [.name "Registry" []] },
    { name := "custom_marker", args := [] }]
#guard (funMeta "peek").map (·.attributes) ==
  some [{ name := "view", args := [] }]
#guard (funMeta "act").map (fun info => (info.isEntry, info.attributes)) ==
  some (true, [
    { name := "randomness", args := [.num 7] },
    { name := "lint.skip", args := [] }])
#guard (funMeta "compatPublic").map
    (fun info => (info.visibility, info.attributes)) ==
  some (.public_, [])

end Attributes

end Tests.MovePrograms

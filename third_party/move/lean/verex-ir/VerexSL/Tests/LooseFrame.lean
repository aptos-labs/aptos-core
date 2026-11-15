-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

namespace VerexSL.Tests.LooseFrame

-- Contextual keywords must stop expression parsing, without reserving
-- host-Lean identifiers or rejecting explicitly escaped source names.
run_cmd do
  let env ← Lean.getEnv
  for source in ["ensures", "modifies", "aborts_if"] do
    if (Lean.Parser.runParserCategory env `verexExpr source).isOk then
      throwError "reserved word accepted as identifier: {source}"
  for source in ["end", "at", "view", "«ensures»", "*value", "abort()", "abort(7)", "panic(7)"] do
    unless (Lean.Parser.runParserCategory env `verexExpr source).isOk do
      throwError "valid identifier or dereference rejected: {source}"

verex module 0x42::loose_frame_frontend where
  -- A frontend fixture: its contracts are not verified.
  pragma verify = false
  struct Counter has Key where
    value : u64
  fun touch(addr : Address) -> Unit := ()
  spec touch where
    modifies global<Counter>(addr), *
    aborts_if false
  fun open_frame() -> Unit := ()
  spec open_frame where
    modifies *
    ensures true
    aborts_if false

run_cmd do
  let env ← Lean.getEnv
  let some registered := VerexSL.registeredUnit? env `«0x42».loose_frame_frontend
    | throwError "loose-frame fixture was not registered"
  let contract := registered.namespaces[0]!.functions[0]!.contract
  unless hasLooseFrame contract && !contract.modifiesAll && contract.modifies.size == 1 do
    throwError "mixed wildcard lost its listed-family boundary"
  let openContract := registered.namespaces[0]!.functions[1]!.contract
  unless openContract.modifiesAll && !hasLooseFrame openContract do
    throwError "fully open frame consumed the following clause"
  let .ok printed := VerexSL.Print.render env registered
    | throwError "loose-frame metadata did not render"
  unless printed.contains ", *" && !printed.contains "verex_loose_frame" do
    throwError "loose-frame surface marker was lost: {printed}"
  let .ok formatted := VerexSL.Print.formatSource env printed
    | throwError "loose-frame output did not re-import"
  unless formatted == printed do
    throwError "loose-frame printing is not a fixed point: {formatted}"

-- A fully open frame ending a module leaves the next command alone: a
-- dereference's operand does not start at the command's column.
verex module 0x42::open_frame_last where
  pragma verify = false
  fun open_frame() -> Unit := ()
  spec open_frame where
    modifies *

verex module 0x42::after_open_frame where
  fun one() -> u64 := 1

run_cmd do
  let env ← Lean.getEnv
  let some registered := VerexSL.registeredUnit? env `«0x42».open_frame_last
    | throwError "the module ending in an open frame was not registered"
  unless registered.namespaces[0]!.functions[0]!.contract.modifiesAll do
    throwError "the open frame ending a module was lost"
  unless (VerexSL.registeredUnit? env `«0x42».after_open_frame).isSome do
    throwError "the module after an open frame was not registered"

end VerexSL.Tests.LooseFrame

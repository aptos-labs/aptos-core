-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

namespace LeanerLang.Tests.LooseFrame

-- Contextual keywords must stop expression parsing, without reserving
-- host-Lean identifiers or rejecting explicitly escaped source names.
run_cmd do
  let env ← Lean.getEnv
  for source in ["ensures", "modifies", "aborts_if"] do
    if (Lean.Parser.runParserCategory env `leanerExpr source).isOk then
      throwError "reserved word accepted as identifier: {source}"
  for source in ["end", "at", "view", "«ensures»", "*value", "abort()", "abort(7)", "panic(7)"] do
    unless (Lean.Parser.runParserCategory env `leanerExpr source).isOk do
      throwError "valid identifier or dereference rejected: {source}"

leaner module 0x42::loose_frame_frontend where
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
  let some registered := LeanerLang.registeredUnit? env `«0x42».loose_frame_frontend
    | throwError "loose-frame fixture was not registered"
  let contract := registered.namespaces[0]!.functions[0]!.contract
  unless hasLooseFrame contract && !contract.modifiesAll && contract.modifies.size == 1 do
    throwError "mixed wildcard lost its listed-family boundary"
  let openContract := registered.namespaces[0]!.functions[1]!.contract
  unless openContract.modifiesAll && !hasLooseFrame openContract do
    throwError "fully open frame consumed the following clause"
  let .ok printed := LeanerLang.Print.render env registered
    | throwError "loose-frame metadata did not render"
  unless printed.contains ", *" && !printed.contains "leaner_loose_frame" do
    throwError "loose-frame surface marker was lost: {printed}"
  let .ok formatted := LeanerLang.Print.formatSource env printed
    | throwError "loose-frame output did not re-import"
  unless formatted == printed do
    throwError "loose-frame printing is not a fixed point: {formatted}"

end LeanerLang.Tests.LooseFrame

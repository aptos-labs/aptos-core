-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-!
`loop k where` in a function's `spec` item places its conditions where an
in-body loop invariant goes, so that the printed unit shows them in the loop
and stays a fixed point. The checks are at the item.
-/

namespace VerexSL.Tests.LoopSpecifications

set_option verex.skip_verify true

verex namespace loop_items where
  fun whiles(n : u32) -> u32 := do
    let i := 0u32
    while i < n do
      i := core.prim.checkedAddPanic(i, 1u32)
      continue
    i

  fun exits(n : u32) -> u32 := do
    let i := 0u32
    loop if i < n then
      i := core.prim.checkedAddPanic(i, 1u32)
      continue
    else
      return i;

  fun labeled(n : u32) -> u32 := do
    let i := 0u32
    loop@outer if i < n then
      i := core.prim.checkedAddPanic(i, 1u32)
      let j := 0u32
      while j < n do
        j := core.prim.checkedAddPanic(j, 1u32)
        if j == i then continue@outer
        else continue
      continue
    else
      break
    i

  fun in_order(n : u32) -> u32 := do
    let i := 0u32
    loop do
      if i < n then
        i := core.prim.checkedAddPanic(i, 1u32)
        continue
      return i

  fun inline_exit(n : u32) -> u32 := do
    let i := 0u32
    loop@outer if i < n then
      i := core.prim.checkedAddPanic(i, 1u32)
      if i == n then continue@outer else continue
    else break
    i

  spec whiles where
    ensures result == n
    loop 1 where
      invariant i <= n

  spec exits where
    loop 1 where
      let bound := n
      invariant i <= bound

  spec labeled where
    loop 2 where
      invariant j <= n
    loop 1 where
      invariant i <= n

  spec in_order where
    loop 1 where
      invariant i <= n

  spec inline_exit where
    loop 1 where
      invariant i <= n

run_cmd do
  let env ← Lean.getEnv
  let some unit := VerexSL.registeredUnit? env `loop_items
    | throwError "missing fixture"
  let .ok printed := VerexSL.Print.render env unit
    | throwError "the fixture did not render"
  for expected in ["while i < n do\n      i := core.prim.checkedAddPanic(i, 1u32)\n      continue\n    where\n      invariant i <= n",
      "loop do\n      spec do\n        let bound := n\n        invariant i <= bound",
      "loop@outer do\n      spec invariant i <= n"] do
    unless printed.contains expected do
      throwError "the invariants are not in the loop:\n{printed}"
  if printed.contains "loop 1 where" then
    throwError "a `loop` item was printed:\n{printed}"
  let .ok formatted := VerexSL.Print.formatSource env printed
    | throwError "the rendering did not re-import:\n{printed}"
  unless formatted == printed do
    throwError "the rendering is not a fixed point:\n{printed}\n{formatted}"

/-- A namespace of one function with two loops, and a `spec` of its loops. -/
private def twoLoops (items : String) : String :=
  "verex namespace bad_loop_item where\n" ++
  "  fun f(n : u32) -> u32 := do\n" ++
  "    let i := 0u32\n" ++
  "    while i < n do\n      i := core.prim.checkedAddPanic(i, 1u32)\n      continue\n" ++
  "    for j in 0u32..n do\n      i := j\n" ++
  "    i\n" ++
  "  struct S where\n    value : u32\n" ++
  items

run_cmd do
  let env ← Lean.getEnv
  let rejected : List (String × String) := [
    ("  spec f where\n    loop 3 where\n      invariant i <= n\n", "`f` has no loop 3: its body has 2 loops"),
    ("  spec f where\n    loop 0 where\n      invariant i <= n\n", "`f` has no loop 0"),
    ("  spec f where\n    loop 1 where\n      invariant i <= n\n    loop 1 where\n      invariant i >= 0\n",
      "loop 1 of `f` has more than one `loop` item"),
    ("  spec f where\n    loop 1 where\n      assert i <= n\n", "a `loop` item holds invariants"),
    ("  spec f where\n    loop 1 where\n      let m := n\n", "a `loop` item holds invariants"),
    ("  spec f where\n    loop 2 where\n      invariant i <= n\n", "loop 2 of `f` is a `for`"),
    ("  spec f where\n    loop 1 where\n      invariant j <= n\n", "unknown local or constant `j`"),
    ("  spec S where\n    loop 1 where\n      invariant true\n", "`S` is no function")]
  for (items, expected) in rejected do
    match VerexSL.Print.formatSource env (twoLoops items) with
    | .ok _ => throwError "accepted: {items}"
    | .error error => unless (toString error).contains expected do
        throwError "wrong diagnostic for {items}: {error}"

end VerexSL.Tests.LoopSpecifications

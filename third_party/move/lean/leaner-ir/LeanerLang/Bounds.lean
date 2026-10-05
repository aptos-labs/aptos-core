-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang.Builtins

/-!
# The integer bounds as Lean constants

A specification names the integer bounds `MAX_U8` … `MAX_U256`, `MAX_I8` …
`MAX_I256`, and `MIN_I8` … `MIN_I256` (`LeanerLang.specificationBounds`).
The same names are Lean constants of type `Int` with those values, so an
authored proof, and a lemma beside it, can name a bound that its
obligations state as a literal. They are reducible: `‹x ≤ MAX_U64›` finds
a hypothesis `x ≤ 18446744073709551615`. `omega` reads a constant as an
atom; `unfold MAX_U64 at *` gives it the value.
-/

open Lean Elab Command in
run_cmd do
  for (name, width, value) in LeanerLang.specificationBounds do
    let id := mkIdent (.mkSimple name)
    let literal := Syntax.mkNumLit (toString value.natAbs)
    let term ← if value < 0 then `(-$literal) else `($literal)
    elabCommand (← `(command| abbrev $id : Int := $term))
    let kind := if name.startsWith "MAX_U" then s!"largest value of `u{width}`"
      else if name.startsWith "MAX_I" then s!"largest value of `i{width}`"
      else s!"least value of `i{width}`"
    addDocStringCore (.mkSimple name) s!"The {kind}, `{value}`."

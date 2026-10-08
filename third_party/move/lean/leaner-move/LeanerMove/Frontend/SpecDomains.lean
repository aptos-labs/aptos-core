-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerMove.Frontend.BitVectorConversion

/-!
A specification function is partial: it is defined where each of its integer
parameters fits the fixed-width type it is declared with, and its value
elsewhere is unspecified. Specification typing lets a caller pass any integer
to such a parameter. A specification function cannot abort, so outside its
domain the body is an aborting branch, which a specification reads as an
unspecified value of the result type. The Move Prover's specification
functions are total over the integers; see G15 in
`designs/prover-test-problems.md`.
-/

namespace LeanerMove.Frontend.SpecDomains

open Xast

/-- The least and greatest value of a fixed-width integer type. -/
def range? (ty : Ty) : Option (Int × Int) := do
  let (bits, signed) ← BitVectorConversion.fixedWidth? ty
  if signed then pure (-(2 ^ (bits - 1)), 2 ^ (bits - 1) - 1) else pure (0, 2 ^ bits - 1)

private def condition (loc : Loc) (operation : Operation) (arguments : List Exp) : Exp :=
  .mk .bool loc (.call operation [] arguments none)

private def number (loc : Loc) (value : Int) : Exp :=
  .mk .num loc (.value (.number value) none false)

/-- Whether every integer parameter of `params` fits its declared type, or
`none` when no parameter is declared with a fixed-width integer type. -/
def domain? (loc : Loc) (params : List Param) : Option Exp :=
  let fits := params.filterMap fun parameter => do
    let (low, high) ← range? parameter.ty
    let value : Exp := .mk parameter.ty loc (.«local» parameter.name)
    pure (condition loc .and
      [condition loc .le [number loc low, value], condition loc .le [value, number loc high]])
  match fits with
  | [] => none
  | first :: rest => some (rest.foldl (fun all fit => condition loc .and [all, fit]) first)

/-- The body of a specification function with parameters `params`: `body`
inside its domain, an unspecified value outside it. -/
def partialBody (params : List Param) (body : Exp) : Exp :=
  let loc := body.loc
  match domain? loc params with
  | none => body
  | some domain =>
      let code : Exp := .mk .u64 loc (.value (.number 0) none false)
      let outside : Exp := .mk body.ty loc (.call (.abort .code) [] [code] none)
      .mk body.ty loc (.ite domain body outside)

end LeanerMove.Frontend.SpecDomains

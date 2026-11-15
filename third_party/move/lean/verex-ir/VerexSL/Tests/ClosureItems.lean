-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-!
A closure expression declares the struct of its captures and the body in the struct's
scope. Lowering makes them items of the module around it, so a unit with closures is
a fixed point of printing, whose contracts of the body stand in the struct's scope.
-/

open Lean
open Lean.Elab.Command

private def header : String :=
  "-- Copyright © Aptos Foundation\n-- SPDX-License-Identifier: Apache-2.0\n\n" ++
    "import VerexSL\n\n"

/-- A closure in a module and one in a function of an impl, whose struct stands in the
module; the first of two constructions in its parent spells the closure, and one in
another function does not, even before its parent. -/
private def canonical : String := header ++ "verex namespace closure_items where
  mod a where
    fun f(c : u32) -> u32 := do
      let g :=
        closure «f::{closure#0}» {u0 : lifetime} has Copy (c : &[u0] u32 := &c)
          fun call(self : &«f::{closure#0}»<lifetime _>, x : u32) -> u32 := x
      let h := new «f::{closure#0}»<lifetime _> { c := &c }
      «f::{closure#0}»::call(&h, «f::{closure#0}»::call(&g, c))

    impl «f::{closure#0}» where
      spec call where
        ensures result == x

    fun caller(x : u32) -> u32 :=
      «make::{closure#0}»::call(&new «make::{closure#0}» {}, x)

    fun make() -> «make::{closure#0}» :=
      closure «make::{closure#0}» has Copy ()
        fun call(self : &«make::{closure#0}», x : u32) -> u32 := x

    struct S where
      v : u32

    impl S where
      fun get(self : S) -> u32 := do
        let k :=
          closure «S::get::{closure#0}» (v : u32 := self.v)
            fun call_once(self : «S::get::{closure#0}») -> u32 := self.v
        «S::get::{closure#0}»::call_once(k)
"

run_cmd do
  let env ← getEnv
  let printed ← match VerexSL.Print.formatSource env canonical with
    | .ok printed => pure printed
    | .error error => throwError "a unit with closures did not format: {error}"
  unless printed == canonical do
    throwError "a unit with closures is not a fixed point:\n{printed}"

-- What a closure expression does not admit.
run_cmd do
  let env ← getEnv
  let rejected : List (String × String) := [
    ("verex module 0x42::move_closure where\n  fun f() -> u64 := do\n    let g := " ++
      "closure «c» () fun call(self : &«c») -> u64 := 1\n    1\n", "VEREX-CLOSURE-PLACE"),
    ("verex namespace in_constant where\n  const K : u32 := " ++
      "closure «c» () fun call(self : &«c») -> u32 := 1u32\n", "VEREX-CLOSURE-PLACE"),
    ("verex namespace twice where\n  fun f() -> u32 := do\n    let g := " ++
      "closure «c» () fun call(self : &«c») -> u32 := 1u32\n    let h := " ++
      "closure «c» () fun call(self : &«c») -> u32 := 2u32\n    1u32\n", "VEREX-CLOSURE-DUPLICATE"),
    ("verex namespace shaped where\n  fun f() -> u32 := do\n    let g := " ++
      "closure «c» {T} () fun call {T}(self : &«c»<T>) -> u32 := 1u32\n    1u32\n" ++
      "  impl «c» where\n    spec call at {u32} where\n      ensures true\n",
      "unknown function or nominal"),
    ("verex namespace unknown where\n  fun f() -> u32 := do\n    let g := " ++
      "closure «c» () fun call(self : &«c») -> u32 := 1u32\n    1u32\n" ++
      "  impl «c» where\n    spec call_mut where\n      ensures true\n",
      "unknown function or nominal")]
  for (source, expected) in rejected do
    match VerexSL.Print.formatSource env source with
    | .ok printed => throwError "`{expected}` was not reported for:\n{printed}"
    | .error error =>
        unless ((toString error).splitOn expected).length > 1 do
          throwError "wrong diagnostic, expecting `{expected}`: {error}"

-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-!
A scope is a reopenable item of a Rust namespace. A path resolves from the
innermost scope outward, and the rendering prints the shortest path that resolves
back to its target, so a unit with scopes is a fixed point of printing.
-/

open Lean
open Lean.Elab.Command

private def header : String :=
  "-- Copyright © Aptos Foundation\n-- SPDX-License-Identifier: Apache-2.0\n\n" ++
    "import VerexSL\n\n"

/-- Every kind of scope, and paths the innermost scope, an outer one and only
the namespace's own path resolve. -/
private def canonical : String := header ++ "verex namespace scopes where
  fun f() -> u32 := 1u32

  mod a where
    fun f() -> u32 := 2u32

    fun inner() -> u32 := f()

    mod b where
      fun outward() -> u32 := f()

      struct S where
        value : u32

      fun make() -> S := new S { value := f() }

    fun sibling() -> b::S := b::make()

  fun qualified() -> u32 := a::f()

  fun deep() -> a::b::S := a::b::make()

  mod c where
    mod a where
      fun f() -> u32 := 3u32

    fun shadowed() -> u32 := a::f()

    fun hidden() -> u32 := scopes::a::f()

    fun root() -> u32 := f()

  struct Counter where
    value : u32

  impl Counter where
    fun get(self : Counter) -> u32 := self.value

    fun twice(self : Counter) -> u32 := get(self)

  fun read(counter : Counter) -> u32 := Counter::get(counter)

  trait Step where
    native fun step(value : u32) -> u32

  trait impl «<scopes::Counter as std::ops::Drop>» where
    fun finish(value : u32) -> u32 := value

  opaque struct Handle where

  mod empty where
"

run_cmd do
  let env ← getEnv
  let printed ← match VerexSL.Print.formatSource env canonical with
    | .ok printed => pure printed
    | .error error => throwError "a unit with scopes did not format: {error}"
  unless printed == canonical do
    throwError "a unit with scopes is not a fixed point:\n{printed}"

-- A reopened block merges with the first, and a `spec` item in it attaches to
-- the declaration of its own scope.
run_cmd do
  let env ← getEnv
  let source := "verex namespace reopened where
  mod a where
    fun f() -> u32 := 1u32

  mod b where
    fun f() -> u32 := 2u32

  mod a where
    fun g() -> u32 := f()

    spec f where
      ensures result == 1

  mod b where
    spec f where
      ensures result == 2
"
  let printed ← match VerexSL.Print.formatSource env source with
    | .ok printed => pure printed
    | .error error => throwError "reopened scopes did not format: {error}"
  let expected := header ++ "verex namespace reopened where
  mod a where
    fun f() -> u32 := 1u32

    spec f where
      ensures result == 1

    fun g() -> u32 := f()

  mod b where
    fun f() -> u32 := 2u32

    spec f where
      ensures result == 2
"
  unless printed == expected do
    throwError "reopened scopes printed otherwise:\n{printed}"

-- What a scope does not admit.
run_cmd do
  let env ← getEnv
  let rejected : List (String × String) := [
    ("verex module 0x42::move_scope where\n  mod a where\n    fun f() -> u64 := 1\n",
      "VEREX-SCOPE-PROFILE"),
    ("verex namespace kinds where\n  mod a where\n    fun f() -> u32 := 1u32\n" ++
      "  impl a where\n    fun g() -> u32 := 1u32\n", "VEREX-SCOPE-KIND"),
    ("verex namespace nesting where\n  impl a where\n    mod b where\n" ++
      "      fun f() -> u32 := 1u32\n", "VIR-SCOPE"),
    ("verex namespace outer_spec where\n  mod a where\n    fun f() -> u32 := 1u32\n" ++
      "  spec f where\n    ensures result == 1\n", "unknown function or nominal"),
    ("verex namespace twice where\n  mod a where\n    fun f() -> u32 := 1u32\n" ++
      "    spec f where\n      ensures true\n  mod a where\n    spec f where\n      ensures true\n",
      "more than one"),
    ("verex namespace uses where\n  mod a where\n    use core::option\n", "belongs to the namespace"),
    ("verex namespace unknown where\n  mod a where\n    fun f() -> u32 := 1u32\n" ++
      "  fun g() -> u32 := f()\n", "unknown function"),
    ("verex module 0x42::move_opaque where\n  opaque struct S where\n", "VEREX-STRUCT-OPAQUE")]
  for (source, expected) in rejected do
    match VerexSL.Print.formatSource env source with
    | .ok printed => throwError "accepted, expecting `{expected}`:\n{printed}"
    | .error error =>
        unless ((toString error).splitOn expected).length > 1 do
          throwError "wrong diagnostic, expecting `{expected}`: {error}"

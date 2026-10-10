-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Transpiler.Xast

/-!
# Intrinsic models

The Move Prover models the standard library's natives in its Boogie prelude
(`prelude.bpl`, `native.bpl`): a hash is an otherwise uninterpreted injection
onto 32-byte vectors, `bcs::serialize` an uninterpreted injection onto
non-empty vectors, `signer::borrow_address` the signer's address.  A
transpiled native has no body, so its Leaner contract is all its callers
know; where the Move source states none, the model here supplies it, in the
prelude's terms: an uninterpreted spec function (`opaque`) with its axioms,
and the native's `spec` relating the call to it.  A model is printed only
when the source declares no `spec` for the native.

The axioms are about the uninterpreted spec functions, never about the native
Lean functions themselves (whose placeholder bodies would make such axioms
inconsistent); the report lists every axiom.  A model also declares the
native's *specification version* (`spec fun`), what a call of the native in a
specification denotes — a native has no body for Leaner to derive it from.
-/

namespace Transpiler.Intrinsics

open Transpiler.Xast

/-- The model of one native: Lean declarations to emit before its `spec`
(uninterpreted spec functions, axioms), and the clauses of the `spec`. -/
structure Model where
  decls : List String := []
  clauses : List String
  /-- The axioms among `decls`, by name, for the report. -/
  axioms : List String := []

/-- The model of a standard-library native, by module and name; the clauses
use the source's parameter names. -/
def model? (name : QualifiedName) : Option Model :=
  if name.module.address != "0x1" then none else
  match name.module.name, name.name with
  | "signer", "borrow_address" =>
    some {
      decls := ["spec fun borrow_address (self : &Signer) : Address := self.address"]
      clauses := ["ensures result = self.address", "aborts_if False"] }
  | "hash", "sha2_256" => some (hashModel "sha2_256")
  | "hash", "sha3_256" => some (hashModel "sha3_256")
  | "bcs", "to_bytes" =>
    -- `serialize` is the source's own uninterpreted spec function.
    some {
      decls := [
        "axiom serialize_injective {MoveValue} [Inhabited MoveValue] (a b : MoveValue) : serialize a = serialize b → a = b",
        "axiom serialize_nonempty {MoveValue} [Inhabited MoveValue] (v : MoveValue) : 0 < (serialize v).toList.length",
        "spec fun to_bytes {MoveValue} (v : MoveValue) : Vector U8 := serialize v" ]
      clauses := ["ensures result = serialize v", "aborts_if False"]
      axioms := ["serialize_injective", "serialize_nonempty"] }
  | _, _ => none
where
  hashModel (fn : String) : Model := {
    decls := [
      s!"/-- The hash, an uninterpreted injection onto 32-byte vectors (the prover's prelude). -/",
      s!"opaque spec_{fn} : Vector U8 → Vector U8",
      s!"axiom spec_{fn}_injective (a b : Vector U8) : spec_{fn} a = spec_{fn} b → a = b",
      s!"axiom spec_{fn}_length (data : Vector U8) : (spec_{fn} data).toList.length = 32",
      s!"spec fun {fn} (data : Vector U8) : Vector U8 := spec_{fn} data" ]
    clauses := [s!"ensures result = spec_{fn} data", "aborts_if False"]
    axioms := [s!"spec_{fn}_injective", s!"spec_{fn}_length"] }

end Transpiler.Intrinsics

-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

/-!
# Verifier results

UNTRUSTED. The model of the Rust bytecode verifier (plan §9.1) returns `ok`
or one of three kinds of error:

- `status`: a rejection with the Rust `StatusCode` name;
- `panic`: a point where the Rust code would panic, typically an index the
  earlier passes are expected to have bounds-checked. Rust turns a panic into
  a rejection (`catch_unwind` in `verifier.rs`), so this is a rejection too;
- `fuel`: the model's loop bound ran out where the Rust loop terminates. A
  rejection the Rust verifier does not make; differential testing (A2) must
  never see it.

Every error is a rejection, so soundness results only need acceptance.
-/

namespace MoveBytecode.Model

inductive VerifierError where
  | status (code : String)
  | panic (what : String)
  | fuel (what : String)
  deriving DecidableEq, Repr, Inhabited

abbrev VResult := Except VerifierError

def reject {α : Type} (code : String) : VResult α := .error (.status code)

def panicAt {α : Type} (what : String) : VResult α := .error (.panic what)

def outOfFuel {α : Type} (what : String) : VResult α := .error (.fuel what)

/-- Reject with `code` if `c` holds: Rust's `if c { return Err(..) }`. -/
def rejectIf (c : Bool) (code : String) : VResult Unit := if c then reject code else pure ()

/-- A table lookup the Rust code performs with `[]`, which panics when out of
bounds. -/
def lookup {α : Type} (table : Array α) (i : Nat) (what : String) : VResult α :=
  match table[i]? with
  | some a => pure a
  | none => panicAt what

end MoveBytecode.Model

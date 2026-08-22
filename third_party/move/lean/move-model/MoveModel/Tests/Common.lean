-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean
import MoveModel.IR.Interp.Exec
import MoveModel.IR.Module

/-!
# Test Infrastructure

`#test` — a `#guard` that reports what actually happened:

* `#test e = v` checks `e == v` (`BEq`) and on failure prints expected and
  actual, both through the compact `TestShow` rendering;
* `#test e matches pat` checks the pattern (for expectations with
  wildcards, e.g. ignoring a `stuck` reason) and on failure prints the
  actual value.

Expected interpreter outcomes are written with the helper constructors
(`okU64 1` instead of constructing an `IWorld` explicitly):

* `okU64 n` / `okBool b` / `okUnit` / `okVals vs` — normal return, empty
  memory;
* `okRet mem vs` — normal return with the expected memory;
* `aborted code` / `abortedIn mem code` — abort outcomes.
-/

namespace Tests

open MoveModel.IR

/-! ## Expected-outcome constructors -/

/-- The outcome of one interpreter run. -/
abbrev Outcome := Except InterpError IOutcome

/-- Compare observable outcomes, ignoring retired interpreter frame storage. -/
instance : BEq Outcome where
  beq
    | .ok (.ret world vals), .ok (.ret world' vals') =>
        world.memory == world'.memory && vals == vals'
    | .ok (.abort mem code), .ok (.abort mem' code') =>
        mem == mem' && code == code'
    | .error a, .error b => a == b
    | _, _ => false

/-- Normal return with values `vs`, empty global memory. -/
def okVals (vs : List Value) : Outcome :=
  .ok (.ret { frames := [], memory := [] } vs)

/-- Normal return of a single `u64`, empty global memory. -/
def okU64 (n : Nat) : Outcome := okVals [.u64 n]

/-- Normal return of a single `bool`, empty global memory. -/
def okBool (b : Bool) : Outcome := okVals [.bool b]

/-- Normal return without values, empty global memory. -/
def okUnit : Outcome := okVals []

/-- Normal return with values `vs` and global memory `mem`. -/
def okRet (mem : IMem) (vs : List Value) : Outcome :=
  .ok (.ret { frames := [], memory := mem } vs)

/-- Abort with `code`, empty global memory. -/
def aborted (code : Nat) : Outcome := .ok (.abort [] code)

/-- Abort with `code` and global memory `mem`. -/
def abortedIn (mem : IMem) (code : Nat) : Outcome := .ok (.abort mem code)

/-- Runs a function of a semantic finite module by name, with the suite's
standard fuel. Test files partially apply this to their module. -/
def run (m : MoveModel.IR.Module) (f : String) (mem : IMem)
    (args : List Value) : Outcome :=
  interpFun m.program 1000 (m.funId f) mem args

/-! ## Compact rendering of outcomes -/

/-- Rendering of values in test-failure messages; the fallback is `Repr`,
with a compact anonymous-constructor form for interpreter outcomes. -/
class TestShow (α : Type) where
  render : α → String

instance (priority := low) [Repr α] : TestShow α := ⟨reprStr⟩

private partial def valueStr : Value → String
  | .int i => s!".int {i}"
  | .bool b => s!".bool {b}"
  | .address a => s!".address {a}"
  | .struct fs => ".struct [" ++ ", ".intercalate (fs.map valueStr) ++ "]"
  | .variant tag fs => s!".variant {tag} [" ++ ", ".intercalate (fs.map valueStr) ++ "]"
  | .vector es => ".vector [" ++ ", ".intercalate (es.map valueStr) ++ "]"
  | .ref t => s!".ref ({reprStr t})"
  | .mut t v => s!".mut ({reprStr t}) ({valueStr v})"

private def valsStr (vs : List Value) : String :=
  "[" ++ ", ".intercalate (vs.map valueStr) ++ "]"

private def memStr (m : IMem) : String :=
  "[" ++ ", ".intercalate
    (m.map fun (r, a, v) => s!"({reprStr r}, {a}, {valueStr v})") ++ "]"

instance : TestShow Outcome where
  render
    | .ok (.ret world vs) =>
        s!".ok (.ret {memStr world.memory} {valsStr vs})"
    | .ok (.abort m c) => s!".ok (.abort {memStr m} {c})"
    | .error (.stuck r) => s!".error (.stuck \"{r}\")"
    | .error .outOfFuel => ".error .outOfFuel"

/-! ## The `#test` command -/

open Lean Elab Command Term

private unsafe def elabTestMatchesUnsafe (e p : Lean.Term) :
    CommandElabM Unit :=
  liftTermElabM do
    let ok ← evalTerm Bool (mkConst ``Bool) (← `(term| (($e matches $p) : Bool)))
    unless ok do
      let actual ← evalTerm String (mkConst ``String)
        (← `(term| TestShow.render $e))
      throwErrorAt e
        "test failed\n  pattern: {p}\n  actual:  {actual}"

@[implemented_by elabTestMatchesUnsafe]
private opaque elabTestMatches (e p : Lean.Term) : CommandElabM Unit

private unsafe def elabTestEqUnsafe (e v : Lean.Term) : CommandElabM Unit :=
  liftTermElabM do
    let ok ← evalTerm Bool (mkConst ``Bool) (← `(term| (($e == $v) : Bool)))
    unless ok do
      let actual ← evalTerm String (mkConst ``String)
        (← `(term| TestShow.render $e))
      let expected ← evalTerm String (mkConst ``String)
        (← `(term| TestShow.render $v))
      throwErrorAt e
        "test failed\n  expected: {expected}\n  actual:   {actual}"

@[implemented_by elabTestEqUnsafe]
private opaque elabTestEq (e v : Lean.Term) : CommandElabM Unit

/-- `#guard e matches pat`, but reporting the actual value on failure. -/
elab "#test " e:term:51 " matches " p:term:51 : command => elabTestMatches e p

/-- `#guard e == v`, but reporting expected and actual on failure. -/
elab "#test " e:term:51 " = " v:term:51 : command => elabTestEq e v

end Tests

-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Transpiler.Driver

/-!
# Source-axiom printer check

The compiler exchange frontend currently does not expose module axiom
declarations. This focused unit check therefore constructs the corresponding
XAST directly; it is intentionally an exception to the source-baseline tests.
-/

namespace Transpiler.Tests.Print.Axiom

open Transpiler Transpiler.Driver Transpiler.Effects Transpiler.Print
open Transpiler.Xast

private def loc : Loc := { file := 0, start := 0, stop := 0 }
private def moduleRef : ModuleRef := { address := "0x42", addressAlias := none, name := "axioms" }

private def localExp (name : String) (ty : Ty) : Exp := .mk ty loc (.local name)
private def number (n : Int) : Exp := .mk .num loc (.value (.number n) none)
private def typeDomain (ty : Ty) : Exp := .mk (.typeDomain ty) loc (.call .typeDomain [] [] none)
private def varPattern (name : String) (ty : Ty) : Pattern := .mk ty loc (.var name)
private def callSpec (name : String) (args : List Exp) (ty : Ty) : Exp :=
  .mk ty loc (.call (.specFunction { module := moduleRef, name } default) [] args none)
private def equal (left right : Exp) : Exp := .mk .bool loc (.call .eq [] [left, right] none)
private def add (left right : Exp) : Exp := .mk .num loc (.call .add [] [left, right] none)
private def forall' (name : String) (ty : Ty) (body : Exp) : Exp :=
  .mk .bool loc (.quant .forall [.mk (varPattern name ty) (typeDomain ty)] [] none body)

private def increment : SpecFun := {
  name := "increment", doc := "", loc,
  typeParams := [], params := [{ name := "x", ty := .num }], result := .num,
  uninterpreted := true, isNative := false, isMoveFun := false, usesOld := false,
  body := none, spec := .empty
}

private def identity : SpecFun := {
  name := "identity", doc := "", loc,
  typeParams := [{ name := "T", abilities := [], isPhantom := false }],
  params := [{ name := "x", ty := .typeParam 0 }], result := .typeParam 0,
  uninterpreted := true, isNative := false, isMoveFun := false, usesOld := false,
  body := none, spec := .empty
}

private def axiomModule : Module := {
  address := "0x42", addressAlias := none, name := "axioms", doc := "", loc,
  namedAddresses := [], friends := [], pragmas := [], constants := [], structs := [], functions := [],
  specFuns := [increment, identity], specVars := [],
  invariants := [
    { kind := .axiom, loc, typeParams := [], properties := [],
      exp := forall' "x" .num (equal (callSpec "increment" [localExp "x" .num] .num)
        (add (localExp "x" .num) (number 1))) },
    { kind := .axiom, loc, typeParams := ["T"], properties := [],
      exp := forall' "x" (.typeParam 0) (equal
        (callSpec "identity" [localExp "x" (.typeParam 0)] (.typeParam 0))
        (localExp "x" (.typeParam 0))) }
  ],
  sources := ["axioms.move"], comments := []
}

private def checkAxiomPrinter : IO Unit := do
  let input : Package := { modules := [axiomModule] }
  let checked ← match Transpiler.LIR.Backend.fromXast input with
    | .ok checked => pure checked
    | .error error => throw <| IO.userError error
  let effects ← match Transpiler.LIR.Effects.computePrinterTable checked with
    | .ok effects => pure effects
    | .error error => throw <| IO.userError error
  let package ← match Transpiler.LIR.Backend.toPrinterPackage .move checked with
    | .ok package => pure package
    | .error error => throw <| IO.userError error
  let some sourceModule := package.modules.head?
    | throw <| IO.userError "the axiom fixture produced no printer module"
  let (output, report) ← match printModule package sourceModule effects with
    | .ok result => pure result
    | .error error => throw <| IO.userError error
  unless output.contains "axiom move_axiom : ∀ x : Int, increment x = x + 1" do
    throw <| IO.userError "the monomorphic source axiom was not printed"
  unless output.contains
      "axiom move_axiom_1 {T} [Inhabited T] : ∀ x : T, identity x = x" do
    throw <| IO.userError "the generic source axiom was not printed"
  unless report.axioms ==
      ["`move_axiom` (source axiom)", "`move_axiom_1` (source axiom)"] do
    throw <| IO.userError s!"unexpected source-axiom report: {report.axioms}"
  unless report.unsupported.isEmpty do
    throw <| IO.userError s!"source axioms were reported unsupported: {report.unsupported}"

#guard_msgs in
#eval checkAxiomPrinter

end Transpiler.Tests.Print.Axiom

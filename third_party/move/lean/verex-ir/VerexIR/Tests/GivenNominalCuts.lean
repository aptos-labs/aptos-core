-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Solve
import VerexIR.Proofs.Denote.GivenOwn

/-!
Construction and mutation check a declared nominal invariant in an arbitrary
Given frame. The payload parameter has no invariant premise. A mutation uses
the explicit certificate of its current row; a false declaration rejects every
check kind.
-/

open VerexIR VerexIR.Proofs VerexIR.Proofs.Denote
namespace VerexIR.Tests.GivenNominalCuts
-- The Given emitter registers this rule in the same local scope.
attribute [local vir_denote_norm] GivenFrame.holds_param

abbrev source : StructHandle := ⟨⟨0⟩, 0⟩
abbrev arguments : NRow := .cons (.param 0) .nil
abbrev fields : NRow := .cons (.param 0) (.cons .num .nil)
abbrev nominal : NTy := .struct source arguments fields

def clauses (G : GivenFrame) : GivenOwnDeclarations G := fun _ _ =>
  some { type := nominal, holds := fun row => 0 ≤ row.2.1.val }

abbrev num (n : Int) : SpecInt .unbounded true := ⟨n, by trivial⟩

def falseClauses (G : GivenFrame) : GivenOwnDeclarations G := fun _ _ =>
  some { type := nominal, holds := fun _ => False }

/-- An arbitrary predicate of a declaration, as a frame installs it. -/
def predicateClauses (G : GivenFrame)
    (predicate : @NTy.carrier G.toCarriers nominal → Prop) : GivenOwnDeclarations G :=
  fun _ _ => some ⟨nominal, predicate⟩

-- Negative control first: a false declared invariant forbids every check kind,
-- including both construction and mutation end, even though parameters hold freely.
theorem false_declaration_rejected (G : GivenFrame) (State : Type) (initial : State)
    (payload : @NTy.carrier (G.withDeclarations (falseClauses G)).toCarriers (.param 0))
    (kind : CertifyKind) :
    ¬ wp (@checkingAt (G.withDeclarations (falseClauses G)).toCarriers State 3 kind nominal
        (payload, (num 0, ())))
      (fun _ final => final = initial) (fun _ => False) initial := by
  letI : Carriers := (G.withDeclarations (falseClauses G)).toCarriers
  intro verified
  have checked := (@wp_checkingAt (G.withDeclarations (falseClauses G)).toCarriers State nominal
    3 kind
    (payload, (num 0, ())) (fun _ final => final = initial) (fun _ => False) initial).mp verified
  exact (GivenFrame.withDeclarations_own_encode G (falseClauses G) source arguments
    ⟨nominal, fun _ => False⟩ rfl (payload, (num 0, ()))).mp checked.1.deepHolds.1

-- This is the cut taken by an actual pack in the shared interpreter.
theorem construction_valid (G : GivenFrame) (State : Type) (initial : State)
    (payload : @NTy.carrier (G.withDeclarations (clauses G)).toCarriers (.param 0)) :
    wp (@checkingAt (G.withDeclarations (clauses G)).toCarriers State 1 .construction nominal
        (payload, (num 0, ())))
      (fun _ final => final = initial) (fun _ => False) initial := by
  verex_obligations []
  all_goals verex_solve

-- The ended loan must establish the declaration for its new current value.
theorem mutation_valid (G : GivenFrame) (State : Type) (initial : State)
    (payload : @NTy.carrier (G.withDeclarations (clauses G)).toCarriers (.param 0)) (n : Int)
    (held : @Carriers.own (G.withDeclarations (clauses G)).toCarriers source arguments
      (@NTy.encode (G.withDeclarations (clauses G)).toCarriers nominal (payload, (num n, ())))) :
    wp (@checkingAt (G.withDeclarations (clauses G)).toCarriers State 2 .mutationEnd nominal
        (payload, (num (n + 1), ())))
      (fun _ final => final = initial) (fun _ => False) initial := by
  verex_obligations []
  all_goals verex_solve

-- Opening a declaration preserves an arbitrary predicate of the decoded row.
-- In particular, it cannot replace the declared predicate by parameter validity.
theorem predicate_construction (G : GivenFrame) (State : Type) (initial : State)
    (P : @NTy.carrier G.toCarriers nominal → Prop)
    (row : @NTy.carrier (G.withDeclarations (predicateClauses G P)).toCarriers nominal)
    (held : P row) :
    wp (@checkingAt (G.withDeclarations (predicateClauses G P)).toCarriers State
      4 .construction nominal row)
      (fun _ final => final = initial) (fun _ => False) initial := by
  verex_obligations []
  all_goals verex_solve

open Lean Elab Command in
run_cmd do
  for name in [``false_declaration_rejected, ``construction_valid, ``mutation_valid, ``predicate_construction] do
    let axioms ← collectAxioms name
    unless axioms.all ([``propext, ``Classical.choice, ``Quot.sound].contains ·) do
      throwError "unexpected axioms in {name}: {axioms}"

end VerexIR.Tests.GivenNominalCuts

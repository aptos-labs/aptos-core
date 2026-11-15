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
-- The Given emitter exposes this accessor in the same local scope.
attribute [local vir_denote_norm] Carriers.holds

abbrev source : StructHandle := ⟨⟨0⟩, 0⟩
abbrev arguments : NRow := .cons (.param 0) .nil
abbrev fields : NRow := .cons (.param 0) (.cons .num .nil)
abbrev nominal : NTy := .struct source arguments fields

abbrev clauses (G : GivenFrame) : GivenOwnDeclarations G := fun _ _ =>
  some { type := nominal, holds := fun row => 0 ≤ row.2.1.val }

abbrev num (n : Int) : SpecInt .unbounded true := ⟨n, by trivial⟩

abbrev family (G : GivenFrame) := (G.withDeclarations (clauses G)).toCarriers

abbrev falseClauses (G : GivenFrame) : GivenOwnDeclarations G := fun _ _ =>
  some { type := nominal, holds := fun _ => False }
abbrev falseFamily (G : GivenFrame) := (G.withDeclarations (falseClauses G)).toCarriers

-- Negative control first: a false declared invariant forbids every check kind,
-- including both construction and mutation end, even though parameters hold freely.
theorem false_declaration_rejected (G : GivenFrame) (State : Type) (initial : State)
    (payload : @NTy.carrier G.toCarriers (.param 0)) (kind : CertifyKind) :
    ¬ wp (@checkingAt (falseFamily G) State 3 kind nominal (payload, (num 0, ())))
      (fun _ final => final = initial) (fun _ => False) initial := by
  letI : Carriers := falseFamily G
  intro verified
  have checked := (@wp_checkingAt (falseFamily G) State nominal 3 kind
    (payload, (num 0, ())) (fun _ final => final = initial) (fun _ => False) initial).mp verified
  exact (GivenFrame.withDeclarations_own_encode G (falseClauses G) source arguments
    ⟨nominal, fun _ => False⟩ rfl (payload, (num 0, ()))).mp checked.1.deepHolds.1

-- This is the cut taken by an actual pack in the shared interpreter.
theorem construction_valid (G : GivenFrame) (State : Type) (initial : State)
    (payload : @NTy.carrier G.toCarriers (.param 0)) :
    wp (@checkingAt (family G) State 1 .construction nominal (payload, (num 0, ())))
      (fun _ final => final = initial) (fun _ => False) initial := by
  verex_obligations []
  all_goals verex_solve

-- The ended loan must establish the declaration for its new current value.
theorem mutation_valid (G : GivenFrame) (State : Type) (initial : State)
    (payload : @NTy.carrier G.toCarriers (.param 0)) (n : Int)
    (held : @Carriers.own (family G) source arguments
      (@NTy.encode (family G) nominal (payload, (num n, ())))) :
    wp (@checkingAt (family G) State 2 .mutationEnd nominal (payload, (num (n + 1), ())))
      (fun _ final => final = initial) (fun _ => False) initial := by
  verex_obligations []
  all_goals verex_solve

-- Opening a declaration preserves an arbitrary predicate of the decoded row.
-- In particular, it cannot replace the declared predicate by parameter validity.
theorem predicate_construction (G : GivenFrame) (State : Type) (initial : State)
    (P : @NTy.carrier G.toCarriers nominal → Prop)
    (row : @NTy.carrier G.toCarriers nominal) (held : P row) :
    let declarations : GivenOwnDeclarations G := fun _ _ => some ⟨nominal, P⟩
    wp (@checkingAt (G.withDeclarations declarations).toCarriers State
      4 .construction nominal row)
      (fun _ final => final = initial) (fun _ => False) initial := by
  dsimp only
  verex_obligations []
  all_goals verex_solve

open Lean Elab Command in
run_cmd do
  for name in [``false_declaration_rejected, ``construction_valid, ``mutation_valid, ``predicate_construction] do
    let axioms ← collectAxioms name
    unless axioms.all ([``propext, ``Classical.choice, ``Quot.sound].contains ·) do
      throwError "unexpected axioms in {name}: {axioms}"

end VerexIR.Tests.GivenNominalCuts

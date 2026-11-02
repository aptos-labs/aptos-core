-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.PreparationCache

open Lean Meta Elab Tactic
open VerexIR.Proofs.Denote.PreparationCache

private abbrev RequestCache := VerexIR.Proofs.Denote.PreparationCache.Cache

private def expectMissing (cache : RequestCache) (statement : Lean.Expr) : MetaM Unit := do
  match ← lookup cache statement with
  | .missing => pure ()
  | .found _ => throwError "a preparation result escaped its dependency scope"

-- A proof's dependencies include its hypothesis even when the requested
-- proposition does not mention that hypothesis. An unchanged child reuses
-- the proof, a replacement or a sibling outside its scope cannot.
example : True := by
  run_tac
    withLocalDeclD `P (mkSort Level.zero) fun proposition => do
      let cache ← IO.mkRef ({} : RequestCache)
      let attempts ← IO.mkRef 0
      withLocalDeclD `evidence proposition fun evidence => do
        withLetDecl `derived proposition evidence fun derived => do
          let search := do attempts.modify (· + 1); pure (some derived)
          discard <| VerexIR.Proofs.Denote.PreparationCache.run cache proposition search
          withLocalDeclD `extra (mkConst ``True) fun _ => do
            discard <| VerexIR.Proofs.Denote.PreparationCache.run cache proposition search
          unless (← attempts.get) == 1 do throwError "a child repeated a proved request"
          let changed := (← getLCtx).modifyLocalDecl evidence.fvarId! (·.setType (mkConst ``False))
          withLCtx changed (← getLocalInstances) do
            expectMissing (← cache.get) proposition
          let inhabited ← mkAppM ``Inhabited #[mkConst ``Nat]
          withLocalDecl `instance .instImplicit inhabited fun instanceValue => do
            let localInstances := (← getLocalInstances).push {
              className := ``Inhabited, fvar := instanceValue }
            withLCtx (← getLCtx) localInstances do expectMissing (← cache.get) proposition
      expectMissing (← cache.get) proposition
  trivial

-- Dependency compatibility alone does not pay for a derivation. A common
-- request is reusable by its participating owners and their proof cases;
-- another clause, or a mixed group containing it, must do its own work.
example : True := by
  run_tac
    let statement := mkConst ``True
    let cache ← IO.mkRef ({} : RequestCache)
    let attempts ← IO.mkRef 0
    let request := do
      attempts.modify (· + 1)
      pure (some (mkConst ``True.intro))
    let run (owners : Array Nat) :=
      VerexIR.Proofs.Denote.PreparationCache.run cache statement request (owners := owners)
    discard <| run #[0, 1]
    discard <| run #[1]
    discard <| run #[0, 0]
    unless (← attempts.get) == 1 do throwError "participating owners repeated common work"
    discard <| run #[2]
    unless (← attempts.get) == 2 do throwError "a late owner inherited uncharged work"
    discard <| run #[1, 2]
    unless (← attempts.get) == 3 do throwError "a mixed group inherited an unpaid request"
    discard <| run #[]
    unless (← attempts.get) == 4 do throwError "an unowned request inherited charged work"
    discard <| run #[3]
    unless (← attempts.get) == 5 do throwError "an owner inherited uncharged work"
  trivial

-- Failed searches have the same debit boundary as successful requests.
example : True := by
  run_tac
    let statement := mkConst ``False
    let cache ← remember {} statement none (owners := #[0])
    match ← lookup cache statement (owners := #[1]) with
    | .missing => pure ()
    | _ => throwError "a late owner inherited an unpaid failed search"
    match ← lookup cache statement (owners := #[0]) with
    | .found none => pure ()
    | _ => throwError "an owner repeated its unchanged failed search"
  trivial

-- No-progress is specific to the whole context revision. A new assumption
-- can make the same request provable; it must not inherit a failed result.
example : True := by
  run_tac
    withLocalDeclD `P (mkSort Level.zero) fun proposition => do
      let cache ← IO.mkRef ({} : RequestCache)
      let attempts ← IO.mkRef 0
      let fail := do attempts.modify (· + 1); pure none
      discard <| VerexIR.Proofs.Denote.PreparationCache.run cache proposition fail
      discard <| VerexIR.Proofs.Denote.PreparationCache.run cache proposition fail
      unless (← attempts.get) == 1 do throwError "an unchanged failed request ran again"
      match ← lookup (← cache.get) proposition (stage := `anotherStage) with
      | .missing => pure ()
      | _ => throwError "a failed request suppressed a different preparation stage"
      withOptions (·.setBool `preparationTest.changed true) do
        expectMissing (← cache.get) proposition
      withLocalDeclD `evidence proposition fun evidence => do
        let found ← VerexIR.Proofs.Denote.PreparationCache.run cache proposition do
          attempts.modify (· + 1)
          return some evidence
        unless found == some evidence && (← attempts.get) == 2 do
          throwError "new evidence did not resume a failed request"
      match ← lookup (← cache.get) proposition with
      | .found none => pure ()
      | _ => throwError "branch evidence leaked to the original context"
  trivial

-- Keeping an FVarId while changing its definition is not context extension.
example : True := by
  run_tac
    withLetDecl `value (mkConst ``Nat) (mkNatLit 0) fun value => do
      let statement ← mkEq value value
      let cache ← remember {} statement (some (← mkEqRefl value))
      let changed := (← getLCtx).modifyLocalDecl value.fvarId! (·.setValue (mkNatLit 1))
      withLCtx changed (← getLocalInstances) do expectMissing cache statement
  trivial

-- An unresolved proof or request cannot become reusable merely because a
-- later tactic assigned its metavariable.
example : True := by
  run_tac
    let statement := mkConst ``True
    let proof ← mkFreshExprMVar statement
    let cache ← remember {} statement (some proof)
    proof.mvarId!.assign (mkConst ``True.intro)
    expectMissing cache statement
    let request ← mkFreshExprMVar (mkSort Level.zero)
    let cache ← remember cache request none
    request.mvarId!.assign statement
    expectMissing cache statement
  trivial

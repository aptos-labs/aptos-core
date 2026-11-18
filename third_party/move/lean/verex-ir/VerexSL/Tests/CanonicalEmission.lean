-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
import VerexSL
import VerexIR.Tests.CanonicalEmissionState

set_option Elab.async false

/-! Native values and freshly exposed invariant facts share their caller's
representation, including implicit annotations, before a leaf is decided. -/
namespace VerexSL.Tests.CanonicalEmission
open Lean Meta Elab Tactic VerexIR.Proofs.Denote
open VerexIR.Tests.CanonicalEmissionState

-- Fresh case equations can expose primitive projections. Emit the same
-- inferred applications as source clauses, while preserving held caller terms.
example (record : VerexIR.SpecInt (.bits 64) false × Bool) : True := by
  run_tac withMainContext do
    let record := (← getLocalDeclFromUserName `record).toExpr
    let raw := mkProj ``VerexIR.SpecInt 0 (mkProj ``Prod 0 record)
    unless (← canonicalValueProjection? (mkProj ``Prod 0 (mkBVar 0))).isNone do
      throwError "unopened projection binder was interpreted in the local context"
    let expected ← mkAppM ``VerexIR.SpecInt.val #[← mkAppM ``Prod.fst #[record]]
    let canonical ← canonicalEmission {} none
    let output ← canonical raw
    unless output == expected do throwError "primitive value projection was not canonical"
    check output
    unless (← canonical output) == output do throwError "projection spelling is not stable"
    let held ← canonicalEmission (({} : Std.HashSet Lean.Expr).insert raw) none
    unless (← held raw) == raw do throwError "held projection was rebuilt"
    let methods ← canonicalEmissionMethods (Simp.mkDefaultMethodsCore #[]) {} none
    let context ← Simp.mkContext { zeta := false }
    let (simplified, _) ← Simp.SimpM.run context {} methods (Simp.simp raw)
    unless simplified.expr == expected do throwError "simp changed canonical projection spelling"
  trivial

example [family : Carriers] : True := by
  run_tac
    let family := (← getLocalDeclFromUserName `family).fvarId
    let canonical ← canonicalEmission {} none
    let semantic ← elabTerm (← `(@Flow
      (Carriers.instantiate (⟨.cons .bool .nil, rfl⟩ : TypeArgs) $(mkIdent `family))
      (.one (.param 0)) .nil
      (@HList ((Carriers.instantiate (⟨.cons .bool .nil, rfl⟩ : TypeArgs) $(mkIdent `family))).toSorts
        (NRow.replicate 0 (.param 0))))) none
    let semantic ← instantiateMVars semantic
    let rebuilt ← canonical semantic
    unless rebuilt.isAppOfArity ``Flow 4 do throwError "lost semantic Flow type"
    unless rebuilt.getArg! 0 == semantic.getArg! 0 && rebuilt.getArg! 1 == semantic.getArg! 1 do
      throwError "changed semantic Flow family or result shape"
    let alpha := rebuilt.getArg! 3
    unless alpha.isAppOfArity ``HList 2 && alpha.getArg! 0 == VerexSL.Contract.carrierSorts (mkFVar family) &&
        (alpha.getArg! 1).isConstOf ``NRow.nil do
      throwError "parent Flow alpha did not acquire caller HList spelling: {alpha}"
    check rebuilt
    unless ← withDefault (isDefEq semantic rebuilt) do throwError "Flow annotation changed meaning"
  trivial

example (_given : GivenFrame) : True := by
  run_tac
    let inner ← elabTerm (← `(@Carriers.instantiate
      (⟨.cons (.param 0) (.cons (.param 1) .nil), rfl⟩ : TypeArgs)
      (GivenFrame.toCarriers $(mkIdent `_given)))) none
    let outer ← elabTerm (← `(GivenFrame.toCarriers $(mkIdent `_given))) none
    let row ← elabTerm (← `(NRow.cons (.param 0) (.cons (.param 1) .nil))) none
    let rows ← elabTerm (← `(NRows.cons (.cons (.param 0) (.cons (.param 1) .nil)) .nil)) none
    let names ← elabTerm (← `(["Entry"])) none
    let proof ← mkEqRefl (mkConst ``Bool.true)
    let nativeType := mkApp (mkConst ``NTy.tuple) row
    let innerSorts := VerexSL.Contract.carrierSorts inner
    let outerSorts := VerexSL.Contract.carrierSorts outer
    let input := #[
      mkApp3 (mkConst ``NTy.plainFallback) innerSorts nativeType proof,
      mkApp3 (mkConst ``NTy.inhabitant) innerSorts nativeType proof,
      mkApp3 (mkConst ``HList.inhabitant) innerSorts row proof,
      mkApp4 (mkConst ``variantCarrier.inhabitant) innerSorts names rows proof]
    let expected := #[
      mkApp3 (mkConst ``NTy.plainFallback) outerSorts nativeType proof,
      mkApp3 (mkConst ``NTy.inhabitant) outerSorts nativeType proof,
      mkApp3 (mkConst ``HList.inhabitant) outerSorts row proof,
      mkApp4 (mkConst ``variantCarrier.inhabitant) outerSorts names rows proof]
    let canonical ← canonicalEmission {} none
    for i in [:input.size] do
      let output ← canonical input[i]!
      if output == input[i]! then throwError "default operation {i} was not respelled"
      unless output == expected[i]! do
        throwError "default operation {i} has a noncanonical spelling: {output}"
      check output
      unless ← withDefault (isDefEq input[i]! output) do
        throwError "default operation {i} changed meaning"
  trivial

-- A deferred `NTy.inhabitantOf` operation whose `Carriers` family is bound only
-- inside an opened lambda does not convert definitionally across `Carriers.instantiate`,
-- so canonical emission preserves its original family spelling untouched.
example : True := by
  run_tac
    let arguments ← elabTerm (← `((⟨.cons .bool .nil, rfl⟩ : TypeArgs))) none
    let proof ← mkEqRefl (mkConst ``Bool.true)
    let canonical ← canonicalEmission {} none
    let input ← withLocalDecl `inner .instImplicit (mkConst ``Carriers) fun inner => do
      let induced := mkApp2 (mkConst ``Carriers.instantiate) arguments inner
      let value := mkApp3 (mkConst ``NTy.inhabitantOf) (VerexSL.Contract.carrierSorts induced)
        (mkConst ``NTy.bool) proof
      mkLambdaFVars #[inner] value
    check input
    let output ← canonical input
    unless output == input do throwError "binder-bound family changed a deferred operation"
    check output
  trivial

run_cmd do observation.set {}

/- Assert representation at a concrete caller read, independently of proof
   success. The observer neither rewrites terms nor inspects generic compiled
   bodies. Earlier callee-local intermediate representations are permitted. -/
namespace CallerState
open Lean Meta Elab Command VerexIR.Proofs.Denote

private partial def hasStaleCarrier (e : Lean.Expr) : MetaM Bool := do
  if ← isProof e then return false
  if e.isAppOfArity ``NTy.carrier 2 &&
      (FamilyTransport.inducedCarriers? (e.getArg! 0)).isSome then return true
  match e with
  | .app f a => return (← hasStaleCarrier f) || (← hasStaleCarrier a)
  | .mdata _ e => hasStaleCarrier e
  | .proj _ _ e => hasStaleCarrier e
  | _ => return false

elab "observe_return_boundary" : tactic => Lean.Elab.Tactic.withMainContext do
  let findLiteral (e : Lean.Expr) := e.find? fun node => node.isAppOfArity ``List.toArray 2 &&
    (node.getArg! 1).isAppOfArity ``List.cons 3
  let recordLiteral (literal : Lean.Expr) : Lean.Elab.Tactic.TacticM Unit := do
    let stale ← hasStaleCarrier literal
    let previous ← observation.get
    observation.set {
      count := previous.count + 1
      badCount := previous.badCount + (if stale then 1 else 0)
      firstBad := if stale then "retained caller state has induced carrier" else previous.firstBad }
  for decl in ← getLCtx do
    let type ← instantiateMVars decl.type
    if type.consumeMData.isAppOfArity ``NTy.Holds 3 then
      if let some literal := findLiteral (type.consumeMData.getArg! 2) then
        recordLiteral literal
      continue
    let some (_, lhs, rhs) := type.eq? | continue
    let (some literal, none) := (findLiteral lhs, findLiteral rhs) | continue
    unless rhs.hasFVar do continue
    recordLiteral literal
  if let some literal := findLiteral (← instantiateMVars (← getMainTarget)) then
    recordLiteral literal

verex module 0x42::canonical_construction where
  pragma verify = false
  struct Entry {K has Copy, Drop, Store} {V has Copy, Drop, Store} has Copy, Drop, Store where
    key : K
    value : V
  struct Map {K has Copy, Drop, Store} {V has Copy, Drop, Store} has Copy, Drop, Store where
    entries : Vector<Entry<K, V> >
  fun empty {K has Copy, Drop, Store} {V has Copy, Drop, Store}() -> Map<K, V> :=
    new Map<K, V> { entries := vector<Entry<K, V> >[] }
  fun add {K has Copy, Drop, Store} {V has Copy, Drop, Store}(map : &mut Map<K,V>, key : K, value : V) -> Unit := do
    let entries := &mut map.entries
    *entries := core.prim.insertVector(*entries, 0, new Entry<K,V> { key, value })
  fun direct_read() -> u64 := do
    let mut map := empty::<Bool,u64>()
    add::<Bool,u64>(&mut map, true, 10)
    map.entries[0].value
  spec direct_read where
    ensures result == 10
    aborts_if false
  verify direct_read by lean =>
    verex_obligations
    all_goals observe_return_boundary
    all_goals verex_prepare
    all_goals verex_decide

run_cmd do
  let result ← observation.get
  unless result.count > 0 do throwError "no retained caller state equation observed"
  unless result.badCount == 0 do throwError "noncanonical caller state {result.badCount}/{result.count}: {result.firstBad}"
end CallerState

run_cmd do observation.set {}
open VerexIR.Tests.CanonicalEmissionState
elab "observe_canonical_entry_invariant" : tactic => withMainContext do
  let target ← instantiateMVars (← getMainTarget)
  unless target.isForall &&
      (target.find? fun e => e.isAppOf ``plainFallback || e.isAppOf ``NTy.plainFallback).isSome do
    return
  for decl in ← getLCtx do
    let type ← instantiateMVars decl.type
    unless type.isForall do continue
    let saved ← saveState
    let convertible ← withDefault (isDefEq type target)
    saved.restore
    unless convertible do continue
    let exactSpelling := type == target
    let previous ← observation.get
    observation.set {
      count := previous.count + 1
      badCount := previous.badCount + (if exactSpelling then 0 else 1)
      firstBad := previous.firstBad }
verex module 0x42::read_only_invariant_loan where
  pragma verify = false
  -- ## Representation

  struct Entry {K has Copy, Drop, Store} {V has Copy, Drop, Store} has Copy, Drop, Store where
    key : K
    value : V

  struct Map {K has Copy, Drop, Store} {V has Copy, Drop, Store} has Copy, Drop, Store where
    entries : Vector<Entry<K, V> >

  spec Map where
    invariant sorted_keys(entries)

  -- Keys strictly increase.
  spec fun sorted_keys {K has Copy, Drop, Store} {V has Copy, Drop, Store}(entries : Vector<Entry<K, V> >) : Bool :=
    forall (i : Int), forall (j : Int), 0 <= i && i < j && j < entries.length ==>
      core.prim.compare(entries[i].key, entries[j].key) < 0

  public fun copy_entries {K has Copy, Drop, Store} {V has Copy, Drop, Store}(map : &mut Map<K, V>) -> Vector<Entry<K, V> > :=
    map.entries
  spec copy_entries where
    aborts_if false
    ensures result.length == old(map).entries.length

  verify copy_entries by lean =>
    verex_obligations
    all_goals verex_prepare
    all_goals observe_canonical_entry_invariant
    all_goals verex_decide

  -- A mutation that writes back what it read owes the invariant of the value
  -- it wrote, which the entry states one for: the owed spelling and the held
  -- one are compared here, where a read-only loan's owed invariant is closed
  -- by the value's own fact before any spelling is read (`openHoldsGoal`).
  public fun reseat {K has Copy, Drop, Store} {V has Copy, Drop, Store}(map : &mut Map<K, V>) -> Unit := do
    let entries := &mut map.entries
    *entries := *entries
  spec reseat where
    aborts_if false
    ensures map.entries.length == old(map).entries.length

  verify reseat by lean =>
    verex_obligations
    all_goals verex_prepare
    all_goals observe_canonical_entry_invariant
    all_goals verex_decide

run_cmd do
  let result ← observation.get
  unless result.count > 0 do throwError "no entry invariant comparison observed"
  unless result.badCount == 0 do throwError "different entry invariant spellings: {result.badCount}/{result.count}"

-- Negative (first): why the conversion of a whole projection is no evidence
-- for the projection's erased structure parameter. The rewritten spelling
-- built here is never handed to the comparator; it stands beside the original
-- to show that the two convert while the parameter the primitive projection
-- discards does not. `canonicalFamilyResult` is then invoked on the unchanged
-- original, and what it returns is that original: the respelling is preserved,
-- which is what this example asserts, not a rejection by the comparator.
example [family : Carriers] (pair : Nat × Unit) : True := by
  run_tac withMainContext do
    let pair := (← getLocalDeclFromUserName `pair).toExpr
    let family := (← getLocalDeclFromUserName `family).toExpr
    let arguments ← elabTerm (← `((⟨.cons .bool .nil, rfl⟩ : TypeArgs))) none
    let induced ← instantiateMVars (mkApp2 (mkConst ``Carriers.instantiate) arguments family)
    let parameter := mkApp (mkConst ``NTy.param) (mkNatLit 0)
    let owner := mkConst ``Nat
    let respelled := mkApp2 (mkConst ``NTy.carrier) (VerexSL.Contract.carrierSorts induced) parameter
    let original ← mkAppM ``Prod.fst #[pair]
    let rewritten := original.withApp fun head args => mkAppN head (args.set! 0 respelled)
    -- The conversion of the whole projection is no evidence: both sides
    -- reduce to the primitive projection of the same pair.
    unless ← withDefault (isDefEq original rewritten) do
      throwError "the two projections did not convert, so this is not the case under test"
    -- And the parameter the projection erases does not convert.
    if ← withDefault (isDefEq owner respelled) then
      throwError "the erased parameter converts, so this is not the case under test"
    -- The respelling does not produce that rewrite from the original.
    unless (← canonicalFamilyResult original).expr == original do
      throwError "the projection was rewritten over a parameter that does not convert: \
        {(← canonicalFamilyResult original).expr}"
  trivial

-- Preservation control: a well-typed input whose owner the traversal holds.
-- What the emission returns is type-correct and converts with its input, in
-- that position and inside an argument a dependent parent keeps. It does not
-- assert that a rewrite was refused: this instance's substituted carrier
-- converts, so the contract holds whether or not the emission rewrites here.
example [family : Carriers]
    (pair : @NTy.carrier ((Carriers.instantiate (⟨.cons .bool .nil, rfl⟩ : TypeArgs) family)).toSorts
      (.param 0) × Unit) : True := by
  run_tac withMainContext do
    let pair := (← getLocalDeclFromUserName `pair).toExpr
    let projection ← mkAppM ``Prod.fst #[pair]
    let held ← canonicalEmission (({} : Std.HashSet Lean.Expr).insert pair) none
    let emitted ← held projection
    check emitted
    unless ← withDefault (isDefEq projection emitted) do
      throwError "the emitted projection over a held owner changed meaning"
    let parent ← mkAppM ``Prod.mk #[projection, mkConst ``Unit.unit]
    let emittedParent ← held parent
    check emittedParent
    unless ← withDefault (isDefEq parent emittedParent) do
      throwError "the emitted parent of a held owner's projection changed meaning"
  trivial

-- What the emission actually hands on is type-correct and means what its
-- input means: a dependent constructor that keeps a proof over a changed
-- field, and a projection of a value at an induced family. The kernel checks
-- the emitted term here, which is where a check of a whole term belongs.
example [family : Carriers] (value : @NTy.carrier (family).toSorts (.int 64 false)) : True := by
  run_tac withMainContext do
    let canonical ← canonicalEmission {} none
    let vector ← instantiateMVars (← elabTerm (← `((⟨#[$(mkIdent `value)], by simp⟩ :
      @NTy.carrier
        ((Carriers.instantiate (⟨.cons (.int 64 false) .nil, rfl⟩ : TypeArgs) $(mkIdent `family))).toSorts
        (.vector (.param 0))))) none)
    let rebuilt ← canonical vector
    check rebuilt
    unless ← withDefault (isDefEq vector rebuilt) do
      throwError "the emitted vector changed meaning"
    let pair ← instantiateMVars (← elabTerm (← `(((($(mkIdent `value)), true) :
      @NTy.carrier
          ((Carriers.instantiate (⟨.cons (.int 64 false) .nil, rfl⟩ : TypeArgs) $(mkIdent `family))).toSorts
          (.param 0) × Bool).fst)) none)
    let projected ← canonical pair
    check projected
    unless ← withDefault (isDefEq pair projected) do
      throwError "the emitted projection changed meaning"
    unless (← canonical projected) == projected do
      throwError "the emitted projection is not stable: {projected}"
  trivial

end VerexSL.Tests.CanonicalEmission

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
example (record : { row : Nat × Bool // row.2 = true }) : True := by
  run_tac withMainContext do
    let record := (← getLocalDeclFromUserName `record).toExpr
    let raw := mkProj ``Prod 0 (mkProj ``Subtype 0 record)
    unless (← canonicalValueProjection? (mkProj ``Prod 0 (mkBVar 0))).isNone do
      throwError "unopened projection binder was interpreted in the local context"
    let expected ← mkAppM ``Prod.fst #[← mkAppM ``Subtype.val #[record]]
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

example [family : Carriers] [_inherited : CarriersNonempty family] : True := by
  run_tac
    let source ← elabTerm (← `(@NTy.toSkolem
      (Carriers.instantiate (⟨.cons .bool .nil, rfl⟩ : TypeArgs) $(mkIdent `family))
      (CarriersNonempty.instantiate (⟨.cons .bool .nil, rfl⟩ : TypeArgs) $(mkIdent `family))
      (⟨.cons .bool .nil, rfl⟩ : TypeArgs) .bool true)) none
    let source ← instantiateMVars source
    let family := (← getLocalDeclFromUserName `family).fvarId
    let instanceType := mkApp (mkConst ``CarriersNonempty) (mkFVar family)
    let make (name : Name) := withLocalDecl name .instImplicit instanceType fun local_ =>
      mkLambdaFVars #[local_] source
    let left ← make `first
    let right ← make `second
    let input ← mkAppM ``Prod.mk #[left, right]
    let canonical ← canonicalEmission {} none
    let output ← canonical input
    if output == input then throwError "binder test observed no canonical conversion"
    let allowed := (collectFVars {} input).fvarIds
    unless (collectFVars {} output).fvarIds.all allowed.contains do
      throwError "canonical conversion leaked a temporary instance binder"
    check output
    unless ← withDefault (isDefEq input output) do
      throwError "canonical conversion changed the expression"
    let again ← canonical input
    unless again == output do throwError "canonical cache result changed"
    check again
    let semantic ← elabTerm (← `(@Flow
      (Carriers.instantiate (⟨.cons .bool .nil, rfl⟩ : TypeArgs) $(mkIdent `family))
      (.one (.param 0)) .nil
      (@HList (Carriers.instantiate (⟨.cons .bool .nil, rfl⟩ : TypeArgs) $(mkIdent `family))
        (NRow.replicate 0 (.param 0))))) none
    let semantic ← instantiateMVars semantic
    let rebuilt ← canonical semantic
    unless rebuilt.isAppOfArity ``Flow 4 do throwError "lost semantic Flow type"
    unless rebuilt.getArg! 0 == semantic.getArg! 0 && rebuilt.getArg! 1 == semantic.getArg! 1 do
      throwError "changed semantic Flow family or result shape"
    let alpha := rebuilt.getArg! 3
    unless alpha.isAppOfArity ``HList 2 && alpha.getArg! 0 == mkFVar family &&
        (alpha.getArg! 1).isConstOf ``NRow.nil do
      throwError "parent Flow alpha did not acquire caller HList spelling: {alpha}"
    check rebuilt
    unless ← withDefault (isDefEq semantic rebuilt) do throwError "Flow annotation changed meaning"
  trivial


example (_given : GivenFrame) : True := by
  run_tac
    let inner ← elabTerm (← `(@Carriers.toPlain
      (@Carriers.instantiate (⟨.cons (.param 0) (.cons (.param 1) .nil), rfl⟩ : TypeArgs)
        (GivenFrame.clauseScaffold $(mkIdent `_given))
        (GivenFrame.clauseScaffoldNonempty $(mkIdent `_given))))) none
    let outer ← elabTerm (← `(@Carriers.toPlain (GivenFrame.toCarriers $(mkIdent `_given)))) none
    let row ← elabTerm (← `(NRow.cons (.param 0) (.cons (.param 1) .nil))) none
    let rows ← elabTerm (← `(NRows.cons (.cons (.param 0) (.cons (.param 1) .nil)) .nil)) none
    let names ← elabTerm (← `(["Entry"])) none
    let proof := mkEqRefl (mkConst ``Bool.true)
    let proof ← proof
    let nativeType := mkApp (mkConst ``NTy.tuple) row
    let input := #[
      mkApp3 (mkConst ``NTy.plainInhabitantOf) inner nativeType proof,
      mkApp3 (mkConst ``HListPlainOf.inhabitant) inner row proof,
      mkApp4 (mkConst ``variantPlainOf.inhabitant) inner names rows proof]
    let expected := #[
      mkApp3 (mkConst ``NTy.plainInhabitantOf) outer nativeType proof,
      mkApp3 (mkConst ``HListPlainOf.inhabitant) outer row proof,
      mkApp4 (mkConst ``variantPlainOf.inhabitant) outer names rows proof]
    let canonical ← canonicalEmission {} none
    for i in [:input.size] do
      let output ← canonical input[i]!
      if output == input[i]! then throwError "plain operation {i} was not respelled"
      unless output == expected[i]! do
        throwError "plain operation {i} has a noncanonical spelling: {output}"
      check output
      unless ← withDefault (isDefEq input[i]! output) do
        throwError "plain operation {i} changed meaning"
  trivial

-- An instance available only inside an opened binder cannot escape through
-- a cached conversion; that operation keeps its original family spelling.
example [_family : Carriers] : True := by
  run_tac
    let family := mkFVar (← getLocalDeclFromUserName `_family).fvarId
    let arguments ← elabTerm (← `((⟨.cons .bool .nil, rfl⟩ : TypeArgs))) none
    let proof ← mkEqRefl (mkConst ``Bool.true)
    let input ← withLocalDecl `inner .instImplicit
        (mkApp (mkConst ``CarriersNonempty) family) fun inner => do
      let induced := mkApp3 (mkConst ``Carriers.instantiate) arguments family inner
      let nonempty := mkApp3 (mkConst ``CarriersNonempty.instantiate) arguments family inner
      let value := mkApp4 (mkConst ``NTy.inhabitantOf) induced nonempty (mkConst ``NTy.bool) proof
      mkLambdaFVars #[inner] value
    check input
    let canonical ← canonicalEmission {} none
    let output ← canonical input
    unless output == input do throwError "binder-only instance changed a deferred operation"
    check output
  trivial

run_cmd do observation.set {}

/- Assert representation at a concrete caller read, independently of proof
   success. The observer neither rewrites terms nor inspects generic compiled
   bodies. Earlier callee-local intermediate representations are permitted. -/
namespace CallerState
open Lean Meta Elab Command VerexIR.Proofs.Denote

-- Inspect computational values only: proof certificates may mention the
-- old generic construction without retaining a wrapper in the runtime value.
private partial def hasLentValue (e : Lean.Expr) : MetaM Bool := do
  if ← isProof e then return false
  if (← whnf (← inferType e)).isSort then return false
  if e.isAppOf ``ParamPlain.of then return true
  match e with
  | .app f a => return (← hasLentValue f) || (← hasLentValue a)
  | .mdata _ e => hasLentValue e
  | .proj _ _ e => hasLentValue e
  | _ => return false

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
  for decl in ← getLCtx do
    let type ← instantiateMVars decl.type
    let some (_, lhs, rhs) := type.eq? | continue
    let findLiteral (e : Lean.Expr) := e.find? fun node => node.isAppOfArity ``List.toArray 2 &&
      (node.getArg! 1).isAppOfArity ``List.cons 3
    let (some literal, none) := (findLiteral lhs, findLiteral rhs) | continue
    unless rhs.hasFVar do continue
    let lent ← hasLentValue (literal.getArg! 1)
    let stale ← hasStaleCarrier literal
    let previous ← observation.get
    observation.set {
      count := previous.count + 1
      badCount := previous.badCount + (if lent || stale then 1 else 0)
      firstBad := if lent || stale then "retained caller state has parameter wrapper or induced carrier" else previous.firstBad }

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
  unless target.isForall && (target.find? fun e => e.isAppOf ``NTy.plainInhabitantOf).isSome do return
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

run_cmd do
  let result ← observation.get
  unless result.count > 0 do throwError "no entry invariant comparison observed"
  unless result.badCount == 0 do throwError "different entry invariant spellings: {result.badCount}/{result.count}"

end VerexSL.Tests.CanonicalEmission

-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Certify
import LeanerIR.Proofs.NativeBitwise
import LeanerIR.Proofs.NativeShift
import LeanerIR.Proofs.NativeVector
import LeanerLang.NativeDataAttrs

/-! Small data normalization at native contract boundaries. In particular,
literal payload selection must reduce before an arithmetic premise is used.
This does not unfold native callee computations or execution relations. -/

attribute [lir_data_norm] LeanerIR.Proofs.NativeArithmetic.bitwise_and_unsigned
  LeanerIR.Proofs.NativeArithmetic.shiftValue_left_unsigned
  LeanerIR.Proofs.NativeArithmetic.shiftValue_right_unsigned

-- Constant shift distances must use the same atom in native value equations
-- and authored specifications before the arithmetic decision procedure runs.
attribute [lir_spec_norm] Int.reduceToNat decide_eq_true_eq
  LeanerIR.Proofs.NativeVector.length_val

/-- A joined typed integer exposes one arithmetic choice, not a conditional
record projection that the arithmetic solver would treat as an opaque atom. -/
@[simp, lir_spec_norm, leaner_native_data_norm] theorem leaner_native_integer_choice
    (condition : Prop) [Decidable condition]
    (left right : LeanerIR.SpecInt width signed) :
    (if condition then left else right).val =
      if condition then left.val else right.val := by
  split <;> rfl

-- Keep a caller's indexed observation in the same form as the callee's
-- contract fact. This is a specification-boundary reduction, not a read
-- from the native computation or an unfolding of the callee body.
@[leaner_native_data_norm, lir_spec_norm] theorem leaner_native_vector_field
    (values : Array LeanerIR.RuntimeValue) (index : Nat) :
    (LeanerIR.RuntimeValue.vector values).field index =
      values[index]?.getD .unit := rfl

attribute [leaner_native_data_norm]
  Array.size_map
  List.getElem?_toArray List.getElem?_cons_zero List.getElem?_cons_succ List.getElem?_nil
  Option.getD_some Option.getD_none String.reduceBEq beq_self_eq_true
  Bool.false_eq_true Bool.true_eq_false Bool.and_false Bool.and_true Bool.false_and Bool.true_and
  ite_true ite_false LeanerIR.RuntimeValue.nominal.injEq LeanerIR.RuntimeValue.integer.injEq
  Option.some.injEq Array.mk.injEq List.cons.injEq reduceCtorEq and_true true_and and_false false_and

open Lean Parser Tactic in
macro "leaner_native_data" loc:(location)? : tactic =>
  `(tactic| simp (config := { failIfUnchanged := false }) only
    [lir_data_norm, leaner_native_data_norm] $[$loc]?)

/- Normalize only hypotheses containing specification vector lengths. A
callee precondition and a caller premise must use the same native array size;
neither a callee body nor the whole continuation needs unfolding. -/
open Lean Meta Elab Tactic in
elab "leaner_native_lengths" : tactic => do
  let mut goal ← getMainGoal
  let remaining := (← getGoals).drop 1
  let hypotheses ← goal.withContext do
    return (← getLCtx).foldl (init := #[]) fun found declaration =>
      if !declaration.isImplementationDetail &&
          declaration.type.getUsedConstants.contains `LeanerLang.Contract.lengthVector then
        found.push declaration.fvarId
      else found
  if hypotheses.isEmpty then return
  let invocation ← `(tactic| simp only [lir_data_norm, leaner_native_data_norm])
  let context ← mkSimpContext invocation (eraseLocal := false)
  for hypothesis in hypotheses do
    setGoals (goal :: remaining)
    let next ← goal.withContext do
      if ((← getLCtx).find? hypothesis).isNone then return some goal
      let (next, _) ← simpLocalDecl goal hypothesis context.ctx context.simprocs
        (mayCloseGoal := false)
      return next.map (·.2)
    match next with
    | none => setGoals remaining; return
    | some next => goal := next
  setGoals (goal :: remaining)

/- Normalize selected-path payload premises without repeatedly visiting the
whole WP and its nested postcondition. Locals are addressed by identity, not
their potentially shared inaccessible names. -/
open Lean Meta Elab Tactic in
elab "leaner_native_hypotheses" : tactic => do
  let mut goal ← getMainGoal
  let remaining := (← getGoals).drop 1
  let candidates ← goal.withContext do
    let mut found := #[]
    for declaration in ← getLCtx do
      if declaration.isImplementationDetail then continue
      if declaration.type.getUsedConstants.contains ``LeanerIR.RuntimeValue.nominal then
        found := found.push declaration.fvarId
    return found
  let invocation ← `(tactic| simp only [lir_data_norm, leaner_native_data_norm])
  let context ← mkSimpContext invocation (eraseLocal := false)
  for candidate in candidates do
    setGoals (goal :: remaining)
    let next ← goal.withContext do
      if ((← getLCtx).find? candidate).isNone then return some goal
      let mut ctx := context.ctx
      let position := (← candidate.getDecl).index
      -- Only earlier runtime-value equations are rewrite material. Arithmetic
      -- facts must not rewrite numeral literals throughout a continuation.
      for declaration in ← getLCtx do
        if declaration.isImplementationDetail || declaration.index ≥ position then continue
        let type := declaration.type.consumeMData
        unless type.isAppOfArity ``Eq 3 &&
            (type.getArg! 0).isConstOf ``LeanerIR.RuntimeValue do continue
        let rules ← ctx.simpTheorems.addTheorem (.fvar declaration.fvarId)
          (mkFVar declaration.fvarId) (config := ctx.indexConfig)
        ctx := ctx.setSimpTheorems rules
      let (next, _) ← simpLocalDecl goal candidate ctx context.simprocs
        (mayCloseGoal := false)
      return next.map (·.2)
    match next with
    | none => setGoals remaining; return
    | some next => goal := next
  -- A proof-producing hypothesis rewrite assigns the old goal. Replacing
  -- its head after that would discard it before installing its successor.
  setGoals (goal :: remaining)

/- Preserve size information from a callee's encoded-array equality without
decoding its elements or unfolding its computation. Each equality is processed
once; unrelated execution and contract hypotheses are not rewrite material. -/
open Lean Meta Elab Tactic in
elab "leaner_native_array_sizes" : tactic => do
  let mut goal ← getMainGoal
  let tail := (← getGoals).drop 1
  let candidates ← goal.withContext do
    let mut found := #[]
    for declaration in ← getLCtx do
      if declaration.isImplementationDetail then continue
      let type := declaration.type.consumeMData
      if type.isAppOfArity ``Eq 3 && (type.getArg! 0).isAppOfArity ``Array 1 then
        found := found.push declaration.fvarId
    return found
  for candidate in candidates do
    let name := Name.str `nativeVectorSize candidate.name.toString
    let next ← goal.withContext do
      if (← getLCtx).any (·.userName == name) then return none
      let declaration ← candidate.getDecl
      let arrayType := declaration.type.consumeMData.getArg! 0
      let size ← withLocalDeclD `values arrayType fun values => do
        mkLambdaFVars #[values] (← mkAppM ``Array.size #[values])
      let proof ← mkAppM ``congrArg #[size, declaration.toExpr]
      return some (← (← goal.assert name (← inferType proof) proof).intro1P).2
    if let some next := next then
      goal := next
      setGoals (goal :: tail)
      evalTactic (← `(tactic|
        simp (config := { failIfUnchanged := false }) only
          [Array.size_map, Array.size_push, Array.size_empty, List.size_toArray,
            List.length_cons, List.length_nil, Nat.reduceAdd] at $(mkIdent name):ident))
      if (← getGoals).length == tail.length then return
      goal ← getMainGoal
  setGoals (goal :: tail)

/- A native branch uses its Boolean guard and the callee's logical
equivalences. Simplify only that guard, never the continuation or arithmetic
value equations elsewhere in context. -/
open Lean Meta Elab Tactic in
elab "leaner_native_guard" guard:ident : tactic => do
  let goal ← getMainGoal
  let remaining := (← getGoals).drop 1
  let next ← goal.withContext do
    let declaration ← getLocalDeclFromUserName guard.getId
    let invocation ← `(tactic| simp only
      [LeanerIR.Proofs.NativeArithmetic.bitwise_and_unsigned,
        LeanerIR.Proofs.NativeArithmetic.shiftValue_left_unsigned,
        LeanerIR.Proofs.NativeArithmetic.shiftValue_right_unsigned,
        beq_true, beq_false, Bool.true_beq, Bool.false_beq,
        Bool.bne_true, Bool.bne_false, Bool.true_bne, Bool.false_bne,
        LeanerIR.Proofs.Certify.boolean_beq_true, LeanerIR.Proofs.Certify.boolean_bne_true,
        Bool.and_eq_true, Bool.or_eq_true, Bool.not_eq_true', Bool.eq_false_iff,
        ne_eq, decide_eq_true_eq, Decidable.not_not, not_or,
        eq_self_iff_true, Bool.false_eq_true, true_iff, iff_true, false_iff, iff_false,
        not_true_eq_false, not_false_eq_true])
    let context ← mkSimpContext invocation (eraseLocal := false)
    let mut ctx := context.ctx
    for earlier in ← getLCtx do
      if earlier.isImplementationDetail || earlier.index ≥ declaration.index then continue
      let type := earlier.type.consumeMData
      let namedBoolean := type.isAppOfArity ``Eq 3 &&
        (type.getArg! 0).isConstOf ``Bool && (type.getArg! 1).isFVar
      -- Named Boolean results may come from arithmetic conditions. Do not
      -- turn literal or integer value equations into global rewrite rules.
      unless type.isAppOfArity ``Iff 2 || namedBoolean do continue
      let rules ← ctx.simpTheorems.addTheorem (.fvar earlier.fvarId)
        (mkFVar earlier.fvarId) (config := ctx.indexConfig)
      ctx := ctx.setSimpTheorems rules
    let (next, _) ← simpLocalDecl goal declaration.fvarId ctx context.simprocs
    return next.map (·.2)
  setGoals (next.toList ++ remaining)

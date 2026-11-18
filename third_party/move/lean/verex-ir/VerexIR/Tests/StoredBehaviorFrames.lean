-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Solve
open VerexIR VerexIR.Proofs VerexIR.Proofs.Denote

/-! # A stored function value's clauses at the frame that reads them

The invariant of a generic declaration's stored value is stated at the frame
its resource's type arguments induce, of the value moved there.  Where a
case is selected at an abstract frame, its clauses of a function-valued
field are handed over at that frame, of the field itself (`callerClause?`):
the case's definition exposed, the field of the moved value read as the
field moved, and each behavioral clause transported (`behaviorAtFrame?`,
`ValidResultsAt.reframe_iff`, `FramedAt.reframe_iff`).  The clauses' markers
stay, a frame that reads its arguments reads them moved, and the conversion
of its own result converts nothing.

Negatives first: the validity of results at a row with a reference is not
converted, and stays the clause it was.
-/

namespace VerexIR.Tests.StoredBehaviorFrames

open Lean Elab Command Meta

/-- The declaration's one type parameter. -/
abbrev params : NRow := .cons (.param 0) .nil

theorem own : params.isParamsFrom 0 = true := rfl

abbrev scalar : NRow := .cons (.int 64 false) .nil
abbrev generic : NRow := .cons (.param 0) .nil
abbrev borrowed : NRow := .cons (.ref (.int 64 false)) .nil
abbrev shared : Sharing := ⟨[false], [false]⟩

/-- A declaration over the parameter holding a function value and a count. -/
abbrev Holder : NTy :=
  .struct ⟨⟨0⟩, 0⟩ params (.cons (.function scalar shared generic) (.cons (.int 64 false) .nil))

/-- A declaration holding a function value that takes a reference. -/
abbrev Lender : NTy :=
  .struct ⟨⟨0⟩, 1⟩ params (.cons (.function borrowed shared generic) .nil)

/-- A declaration holding both. -/
abbrev Both : NTy :=
  .struct ⟨⟨0⟩, 2⟩ params
    (.cons (.function borrowed shared generic) (.cons (.function scalar shared generic) .nil))

section
variable (unit : Validation.ValidatedUnit) [FunctionCarriers unit]
  (executable : Validation.ExecutableUnit unit) (θ : TypeArgs)

/-- The case of `Holder`'s stored invariant, as a unit's generated constant
states one: over an instance's type arguments and a value at the frame they
induce, each clause under its marker. -/
def holderCase
    (value : @NTy.carrier ((Skolems.instantiate θ (Skolems.runtime unit)).toCarriers).toSorts Holder) : Prop :=
  Obligation "holder" 1 2
      (@ValidResultsAt unit _ executable (Skolems.instantiate θ (Skolems.runtime unit)) scalar
        shared (.one (.param 0)) value.1) ∧
    Obligation "holder" 3 4
      (@KeepsMemoryAt unit _ executable (Skolems.instantiate θ (Skolems.runtime unit)) scalar
        shared (.one (.param 0)) value.1)

/-- The case of `Lender`: the validity of results at a row with a reference. -/
def lenderCase
    (value : @NTy.carrier ((Skolems.instantiate θ (Skolems.runtime unit)).toCarriers).toSorts Lender) : Prop :=
  Obligation "lender" 1 2
    (@ValidResultsAt unit _ executable (Skolems.instantiate θ (Skolems.runtime unit)) borrowed
      shared (.one (.param 0)) value.1)

/-- The case of `Both`: one clause at a row with a reference, one without. -/
def bothCase
    (value : @NTy.carrier ((Skolems.instantiate θ (Skolems.runtime unit)).toCarriers).toSorts Both) : Prop :=
  Obligation "both" 1 2
      (@ValidResultsAt unit _ executable (Skolems.instantiate θ (Skolems.runtime unit)) borrowed
        shared (.one (.param 0)) value.1) ∧
    Obligation "both" 3 4
      (@ValidResultsAt unit _ executable (Skolems.instantiate θ (Skolems.runtime unit)) scalar
        shared (.one (.param 0)) value.2.1)

end

/-- The behavioral clauses a statement makes: each with whether it is stated
at the frame `reader` and of a value that is not one moved. -/
def behaviors (reader statement : Lean.Expr) : Array (Lean.Expr × Bool) :=
  (sitesWhere (fun term => term.isAppOfArity ``ValidResultsAt 8 ||
      term.isAppOfArity ``KeepsMemoryAt 8 || term.isAppOfArity ``FramedAt 9)
    #[statement]).map fun clause =>
      (clause, clause.getArg! 3 == reader &&
        (clause.find? (·.isAppOf ``Skolems.reframe)).isNone)

/-- The markers a statement keeps, each with its label. -/
def markers (statement : Lean.Expr) : Array (Lean.Expr × Lean.Expr × Lean.Expr) :=
  (sitesWhere (·.isAppOfArity ``Obligation 4) #[statement]).map fun marked =>
    (marked.getArg! 0, marked.getArg! 1, marked.getArg! 2)

/-- Runs the conversion of a selected case's clause on the clause `statement`
concludes, under its binders, of which `Θ` is the reading frame.  `none`
expects no conversion.  `some reads` expects one whose proof states it, that
keeps every marker with its label, whose behavioral clauses are at the
reader's frame of the field itself exactly where `reads` says so, in the
statement's order, and that converts nothing when run on its own result. -/
elab "#guard_caller_clause " expected:term " in " statement:term : command => liftTermElabM do
  let statement ← instantiateMVars (← Term.elabType statement)
  Term.synthesizeSyntheticMVarsNoPostponing
  let expected ← unsafe Term.evalTerm (Option (List Bool)) (mkApp (mkConst ``Option [0])
    (mkApp (mkConst ``List [0]) (mkConst ``Bool))) expected
  forallTelescope statement fun binders clause => do
    let some reader ← binders.findM? fun binder => do
        return (← inferType binder).isAppOf ``Skolems
      | throwError "the statement names no reading frame"
    match ← callerClause? clause, expected with
    | none, none => pure ()
    | none, some _ => throwError "the clause was not converted"
    | some (converted, _), none => throwError "the clause was converted to{indentExpr converted}"
    | some (converted, same), some reads =>
        Meta.check same
        unless ← isDefEq (← inferType same) (← mkEq clause converted) do
          throwError "the conversion's proof does not state it"
        let given := markers ((← unfoldDefinition? clause).getD clause)
        unless markers converted == given && !given.isEmpty do
          throwError "the conversion changed the clauses' markers:{indentExpr converted}"
        let found := (behaviors reader converted).toList.map (·.2)
        unless found == reads do
          throwError "expected the clauses at the reader's frame {reads}, got {found} in\
            {indentExpr converted}"
        if let some (again, _) ← callerClause? converted then
          throwError "the conversion converts its own result to{indentExpr again}"
        for (behavior, _) in behaviors reader converted do
          if (← behaviorAtFrame? behavior).isSome && behavior.getArg! 3 == reader then
            throwError "a clause at the reader's frame was converted again"

variable {unit : Validation.ValidatedUnit} [FunctionCarriers unit]

/-! ## A row with a reference

The validity of results at such a row has no law here: the case stays the
clause it was, at the induced frame, of the moved holder. -/

#guard_caller_clause none in
  ∀ (unit : Validation.ValidatedUnit) [FunctionCarriers unit]
    (executable : Validation.ExecutableUnit unit) (Θ : Skolems unit)
    (bounded : Lender.paramsAll (fun index => decide (index < params.length)) = true)
    (value : @NTy.carrier (Θ.toCarriers).toSorts Lender),
    lenderCase unit executable (Θ.typeArgs params own) (Θ.reframe own bounded value)

/-! Beside a clause that is converted, it is kept as it was: at the induced
frame, of the field moved. -/

#guard_caller_clause some [false, true] in
  ∀ (unit : Validation.ValidatedUnit) [FunctionCarriers unit]
    (executable : Validation.ExecutableUnit unit) (Θ : Skolems unit)
    (bounded : Both.paramsAll (fun index => decide (index < params.length)) = true)
    (value : @NTy.carrier (Θ.toCarriers).toSorts Both),
    bothCase unit executable (Θ.typeArgs params own) (Θ.reframe own bounded value)

/-! ## The clauses of a function field

Both clauses of the holder's field, the validity of its results and the
memory it keeps, are stated at the reader's frame of the field, under their
markers. -/

#guard_caller_clause some [true, true] in
  ∀ (unit : Validation.ValidatedUnit) [FunctionCarriers unit]
    (executable : Validation.ExecutableUnit unit) (Θ : Skolems unit)
    (bounded : Holder.paramsAll (fun index => decide (index < params.length)) = true)
    (value : @NTy.carrier (Θ.toCarriers).toSorts Holder),
    holderCase unit executable (Θ.typeArgs params own) (Θ.reframe own bounded value)

/-! A statement that is no case's clause is not converted. -/

#guard_caller_clause none in
  ∀ (unit : Validation.ValidatedUnit) [FunctionCarriers unit]
    (executable : Validation.ExecutableUnit unit) (Θ : Skolems unit)
    (function : @NTy.carrier (Θ.toCarriers).toSorts (.function scalar shared generic)),
    @ValidResultsAt unit _ executable Θ scalar shared (.one (.param 0)) function

/-! ## A frame that reads its arguments

The frame of a moved function value, stated at the induced frame over that
frame's arguments, is read at the reader's frame at the arguments moved
there: the converted clause's frame applies the predicate to the row moved,
not to the row itself, which is of another carrier family. -/

run_meta do
  let statement ← Term.TermElabM.run' <| do
    let statement ← Term.elabType (← `(
      ∀ (unit : Validation.ValidatedUnit) [FunctionCarriers unit]
        (executable : Validation.ExecutableUnit unit) (Θ : Skolems unit)
        (bounded : (NTy.function generic shared generic).paramsAll
          (fun index => decide (index < params.length)) = true)
        (observed : @HList ((Θ.instance params own).toCarriers).toSorts generic → Memory unit → Prop)
        (function : @NTy.carrier (Θ.toCarriers).toSorts (.function generic shared generic)),
        @FramedAt unit _ executable (Θ.instance params own) generic shared (.one (.param 0))
          (fun args _ post => observed args post) (Θ.reframe own bounded function)))
    Term.synthesizeSyntheticMVarsNoPostponing
    instantiateMVars statement
  forallTelescope statement fun binders clause => do
    let (reader, observed) := (binders[3]!, binders[5]!)
    let some converted ← behaviorAtFrame? clause | throwError "the frame was not converted"
    Meta.check (← converted.getProof)
    unless converted.expr.isAppOfArity ``FramedAt 9 && converted.expr.getArg! 3 == reader &&
        converted.expr.appArg! == binders[6]! do
      throwError "the frame is not stated at the reader's frame of the value:\
        {indentExpr converted.expr}"
    let .lam _ _ (.lam _ _ (.lam _ _ body _) _) _ := converted.expr.getArg! 7
      | throwError "the converted frame is no predicate of its arguments"
    -- `observed (Θ.reframe … args) post`, `args` the predicate's first binder.
    unless body.getAppFn == observed do
      throwError "the converted frame is not the predicate given:{indentExpr body}"
    let moved := body.getArg! 0
    unless moved.isAppOfArity ``Skolems.reframe 8 && moved.appArg! == .bvar 2 &&
        body.getArg! 1 == .bvar 0 do
      throwError "the converted frame does not read its arguments moved:{indentExpr body}"
    if (← behaviorAtFrame? converted.expr).isSome then
      throwError "a frame at the reader's frame was converted again"

end VerexIR.Tests.StoredBehaviorFrames

-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Solve.Base

/-!
# The solver: the normal form

The simprocs and lemmas of the normalization sets (`vir_denote_norm`,
`vir_denote_eval`): literal evaluation, canonical family spellings, and the
encodings' normal forms. The deciders read these sets by name, so this
module does not import them.
-/

namespace VerexIR.Proofs.Denote

open FamilyTransport (respellOperation? inducedCarriers? spelledNTy spelledNRow spelledNRows substitutionRow?
  rowEntry? substituteNTy substituteNRow substituteNRows)

open Lean Elab Tactic Meta

/-- A structural update of an array literal at literal positions: the
positions move, the elements are not inspected. -/
private def reduceArrayLiteral (array : Lean.Expr) (positions : Array Lean.Expr)
    (update : List Lean.Expr → Array Nat → Option (List Lean.Expr)) : MetaM Simp.DStep := do
  let some (type, elements) := array.arrayLit? | return .continue
  let some positions := positions.mapM (·.nat?) | return .continue
  let some updated := update elements positions | return .continue
  return .visit (← mkArrayLit type updated)

/-- An in-bounds write into an array literal. -/
dsimproc [vir_denote_norm] reduceSetIfInBounds (Array.setIfInBounds _ _ _) := fun e => do
  let_expr Array.setIfInBounds _ array index value := e | return .continue
  reduceArrayLiteral array #[index] fun elements positions =>
    let index := positions[0]!
    some (if index < elements.length then elements.set index value else elements)

/-- An in-bounds exchange in an array literal. -/
dsimproc [vir_denote_norm] reduceSwapIfInBounds (Array.swapIfInBounds _ _ _) := fun e => do
  let_expr Array.swapIfInBounds _ array left right := e | return .continue
  reduceArrayLiteral array #[left, right] fun elements positions =>
    match elements[positions[0]!]?, elements[positions[1]!]? with
    | some first, some second =>
        some ((elements.set positions[0]! second).set positions[1]! first)
    | _, _ => some elements

/-- A slice of an array literal, its end clamped to the size. -/
dsimproc [vir_denote_norm] reduceExtract (Array.extract _ _ _) := fun e => do
  let_expr Array.extract _ array start stop := e | return .continue
  reduceArrayLiteral array #[start, stop] fun elements positions =>
    some ((elements.take positions[1]!).drop positions[0]!)

/-- A range reversal of an array literal. -/
dsimproc [vir_denote_norm] reduceReverseSlice (VerexIR.Proofs.Denote.reverseSlice _ _ _) :=
  fun e => do
    let_expr VerexIR.Proofs.Denote.reverseSlice _ array left right := e | return .continue
    reduceArrayLiteral array #[left, right] fun elements positions =>
      let left := positions[0]!
      let right := positions[1]!
      if right ≤ elements.length then
        some (elements.take left ++ ((elements.take right).drop left).reverse ++
          elements.drop right)
      else some elements

/-- Normalize only a bind's action. Its continuation is the rest of the
program: the solver reaches it through a folded continuation when the action
completes, so the normalizer never traverses it while a head is processed.
An action that completed hands the bind back to the rewrite rules that
reduce it. -/
private def bindActionStep (e : Lean.Expr) : Simp.SimpM Simp.Step := do
  let arguments := e.getAppArgs
  if arguments.size < 2 then return .continue
  let action := arguments[arguments.size - 2]!
  let next := arguments[arguments.size - 1]!
  let result ← Simp.simp action
  let head := mkAppN e.getAppFn (arguments.extract 0 (arguments.size - 2))
  let expr := mkApp2 head result.expr next
  let proof? ← result.proof?.mapM fun proof => do mkCongrFun (← mkCongrArg head proof) next
  let completed := match result.expr.getAppFn with
    | .const name _ => name == ``VerexIR.Proofs.Spec.pure || name == ``VerexIR.Proofs.Spec.abort ||
        name == ``VerexIR.Proofs.Denote.abortAt
    | _ => false
  if completed then return .continue (some { expr, proof? })
  return .done { expr, proof? }

simproc ↓ [vir_denote_norm] bindAction (VerexIR.Proofs.Spec.bind _ _) := bindActionStep

simproc ↓ [vir_denote_norm] flowBindAction (@VerexIR.Proofs.Denote.Flow.bind _ ?functions ?skolems _ _ _ _ _ _) :=
  bindActionStep



/-- An equation between values under the family a call's type arguments
induce, stated as the caller reads them: the equation of their caller views
(`NTy.ofSkolem`), the view in which the caller holds a callee's results. -/
simproc ↓ [vir_denote_norm] instantiatedEqInCallerView (@Eq _ _ _) :=
  fun e => do
    let_expr Eq carrier left right := e | return .continue
    let some (outer, θ, τ) ← (do
        match carrier.getAppFn.constName?, carrier.getAppArgs with
        | some ``VerexIR.Proofs.Denote.Carriers.carrier, #[family, index] =>
            let some (θ, outer) := inducedCarriers? family | pure none
            pure (some (outer, θ, mkApp (mkConst ``VerexIR.Proofs.Denote.NTy.param) index))
        | some ``VerexIR.Proofs.Denote.NTy.carrier, #[family, τ] =>
            let some (θ, outer) := inducedCarriers? family | pure none
            pure (some (outer, θ, τ))
        | _, _ => pure none : MetaM (Option (Lean.Expr × Lean.Expr × Lean.Expr)))
      | return .continue
    -- Without type parameters the two views hold the same values.
    if (τ.find? (·.isAppOf ``VerexIR.Proofs.Denote.NTy.param)).isNone then return .continue
    let view (value : Lean.Expr) :=
      mkAppN (mkConst ``VerexIR.Proofs.Denote.NTy.ofSkolem) #[outer, θ, τ, value]
    let equation ← mkEq (view left) (view right)
    let injective := mkAppN (mkConst ``VerexIR.Proofs.Denote.NTy.ofSkolem_inj)
      #[outer, θ, τ, left, right]
    let proof ← mkAppM ``propext #[← mkAppM ``Iff.symm #[injective]]
    return .visit { expr := equation, proof? := some proof }



/-- Whether a row is spelled to its end, so that a position past it is known
to be past it. -/
private partial def spelledSpine (row : Lean.Expr) : Bool :=
  row.isConstOf ``VerexIR.Proofs.Denote.NRow.nil ||
    (row.isAppOfArity ``VerexIR.Proofs.Denote.NRow.cons 2 && spelledSpine (row.getArg! 1))

/-- The row a substitution is by, as spelled. -/
private def spelledRow (row : Lean.Expr) : MetaM Lean.Expr := do
  if row.isAppOfArity ``Subtype.val 3 then
    let inner ← whnfR (row.getArg! 2)
    if let some spelled := substitutionRow? inner then return spelled
  return row

/-- A substitution by a literal row, evaluated by the given evaluator or,
failing it, by reduction. -/
private def reduceSubst (substitute : Lean.Expr → Lean.Expr → Option Lean.Expr)
    (e row subject : Lean.Expr) : MetaM Simp.DStep := do
  if let some substituted := substitute (← spelledRow row) subject then
    if substituted != e then return .visit substituted
  let reduced ← whnfR e
  if reduced == e then return .continue
  return .visit reduced

/-- A substitution by a literal row evaluates, the whole type at once, so no
substitution is left in an argument the normalizer does not visit. -/
dsimproc ↓ [vir_denote, vir_denote_norm] reduceParameterSubst
    (VerexIR.Proofs.Denote.NTy.subst _ _) := fun e => do
  let_expr VerexIR.Proofs.Denote.NTy.subst row τ := e | return .continue
  reduceSubst substituteNTy e row τ

/-- The substitution of a literal row evaluates (`reduceParameterSubst`). -/
dsimproc ↓ [vir_denote, vir_denote_norm] reduceRowSubst
    (VerexIR.Proofs.Denote.NRow.subst _ _) := fun e => do
  let_expr VerexIR.Proofs.Denote.NRow.subst row fields := e | return .continue
  reduceSubst substituteNRow e row fields

/-- The substitution of literal variant rows evaluates (`reduceParameterSubst`). -/
dsimproc ↓ [vir_denote, vir_denote_norm] reduceRowsSubst
    (VerexIR.Proofs.Denote.NRows.subst _ _) := fun e => do
  let_expr VerexIR.Proofs.Denote.NRows.subst row rows := e | return .continue
  reduceSubst substituteNRows e row rows

/-- The typed reads of a type at a family (`NTy.read` and its carrier,
instances, write, read-back, and view). -/
private def typedReads : List Name :=
  [``VerexIR.Proofs.Denote.NTy.readCarrier, ``VerexIR.Proofs.Denote.NTy.read,
   ``VerexIR.Proofs.Denote.NTy.readDecEq, ``VerexIR.Proofs.Denote.NTy.readInhabited]

/-- A typed read of a type at a family, applied to `rest`, as the normal
form spells it: at an integer type or `num` the integers' (values, `Int`,
its instances), at another concrete type the type's own (values as they
are, its carrier, `NTy.decEq`, `nativeInhabited`), at a parameter the
family's. The spellings are those of the typed views
(`NativeMaps.typedView?`) and the typed-read lemmas (`NTy.read_int`, …),
by definition; here they reach implicit arguments and instances, which
rewriting does not. -/
private def typedReadSpelling (head : Name) (family τ : Lean.Expr) (rest : Array Lean.Expr) :
    MetaM (Option Lean.Expr) := do
  let τ := τ.consumeMData
  let atFamily (τ : Lean.Expr) : Lean.Expr := mkAppN (mkConst head) (#[family, τ] ++ rest)
  if τ.isAppOfArity ``VerexIR.Proofs.Denote.NTy.param 1 then return some (atFamily τ)
  let int := τ.isAppOfArity ``VerexIR.Proofs.Denote.NTy.int 2
  let num := τ.isConstOf ``VerexIR.Proofs.Denote.NTy.num
  unless int || num || τ.getAppFn.constName?.any (·.getPrefix == ``VerexIR.Proofs.Denote.NTy) do
    return none
  let integer := int || num
  let intType := mkConst ``Int
  let carrier := mkApp2 (mkConst ``VerexIR.Proofs.Denote.NTy.carrier) family τ
  -- The reads taking the type's inhabitability: the proof, and the arguments after it.
  let inhabited? : Option (Lean.Expr × Array Lean.Expr) := do
    let proof ← rest[0]?
    pure (proof, rest.extract 1 rest.size)
  let nativeInhabited (proof : Lean.Expr) : Lean.Expr :=
    let fallback := mkApp3 (mkConst ``VerexIR.Proofs.Denote.NTy.inhabitantOf) family τ proof
    mkApp3 (mkConst ``VerexIR.Proofs.Denote.nativeInhabited) family τ fallback
  if head == ``VerexIR.Proofs.Denote.NTy.readCarrier then
    return some (mkAppN (if integer then intType else carrier) rest)
  if head == ``VerexIR.Proofs.Denote.NTy.read then
    let read := if int then
        mkApp2 (mkConst ``VerexIR.SpecInt.val)
          (mkApp (mkConst ``VerexIR.IntWidth.bits) (τ.getArg! 0)) (τ.getArg! 1)
      else if num then
        mkApp2 (mkConst ``VerexIR.SpecInt.val) (mkConst ``VerexIR.IntWidth.unbounded)
          (mkConst ``Bool.true)
      else .lam `value carrier (.bvar 0) .default
    return some (mkAppN read rest).headBeta
  if head == ``VerexIR.Proofs.Denote.NTy.readDecEq then
    let inst ← if integer then synthInstance (← mkAppM ``DecidableEq #[intType])
      else pure (mkApp2 (mkConst ``VerexIR.Proofs.Denote.NTy.decEq) family τ)
    return some (mkAppN inst rest)
  if head == ``VerexIR.Proofs.Denote.NTy.readInhabited then
    let some (proof, rest) := inhabited? | return none
    let inst ← if integer then synthInstance (← mkAppM ``Inhabited #[intType])
      else pure (nativeInhabited proof)
    return some (mkAppN inst rest)
  return some (atFamily τ)

/-- A candidate outer spelling, including implicit type arguments ordinary
rewriting does not reach. The caller checks conversion of the complete
result. -/
private def canonicalFamilies (e : Lean.Expr) : MetaM Lean.Expr := do
  unless (e.find? fun t => t.isConstOf ``VerexIR.Proofs.Denote.Skolems.instantiate ||
      t.isConstOf ``VerexIR.Proofs.Denote.Carriers.instantiate ||
      t.isConstOf ``VerexIR.Proofs.Denote.NTy.subst || t.isConstOf ``VerexIR.Proofs.Denote.NRow.subst ||
      t.isConstOf ``VerexIR.Proofs.Denote.NRows.subst ||
      t.constName?.any typedReads.contains).isSome do return e
  -- This is a syntactic pass. Keep binders closed instead of rebuilding their
  -- telescopes; only closed expressions may reach a meta-level reduction.
  Core.transform e (post := fun t => do
    let args := t.getAppArgs
    let some head := t.getAppFn.constName? | return .continue
    -- A substitution by a row spelled to its end, such as a transport's
    -- result type, evaluated as the caller spells it.
    if head == ``VerexIR.Proofs.Denote.NTy.subst || head == ``VerexIR.Proofs.Denote.NRow.subst ||
        head == ``VerexIR.Proofs.Denote.NRows.subst then
      let #[θ, operand] := args | return .continue
      let row ← if θ.hasLooseBVars then pure θ else spelledRow θ
      unless spelledSpine row do return .continue
      let evaluated := if head == ``VerexIR.Proofs.Denote.NTy.subst then substituteNTy row operand
        else if head == ``VerexIR.Proofs.Denote.NRow.subst then substituteNRow row operand
        else substituteNRows row operand
      return match evaluated with
        | some evaluated => .done evaluated
        | none => .continue
    let induced (family : Lean.Expr) : Option (Lean.Expr × Lean.Expr) := do
      let (θ, outer) ← inducedCarriers? family
      (substitutionRow? θ).map (outer, ·)
    -- A typed read at a concrete type, or at a parameter of an induced
    -- family, spelled as the normal form spells it (`typedReadSpelling`).
    if typedReads.contains head then
      let #[family, τ] := args.extract 0 2 | return .continue
      let (family, τ) ← match induced family with
        | some (outer, row) =>
            let some τ' := substituteNTy row τ | return .continue
            pure (outer, τ')
        | none => pure (family, τ)
      match ← typedReadSpelling head family τ (args.extract 2 args.size) with
      | some t' => return .done t'
      | none => return .continue
    let rebuilt : Option Lean.Expr := do
      let family ← args[0]?
      let (outer, row) ← induced family
      if head == ``VerexIR.Proofs.Denote.NTy.carrier then
        let τ ← args[1]?
        let τ' ← substituteNTy row τ
        pure (mkAppN t.getAppFn (#[outer, τ'] ++ args.extract 2 args.size))
      else if head == ``VerexIR.Proofs.Denote.HList then
        let fields ← args[1]?
        let fields' ← substituteNRow row fields
        pure (mkAppN t.getAppFn (#[outer, fields'] ++ args.extract 2 args.size))
      else if head == ``VerexIR.Proofs.Denote.variantCarrier then
        let names ← args[1]?
        let rows ← args[2]?
        let rows' ← substituteNRows row rows
        pure (mkAppN t.getAppFn (#[outer, names, rows'] ++ args.extract 3 args.size))
      else none
    match rebuilt with
    | some t' => return .done t'
    | none =>
    -- Any other native operation at an induced family: at the outer family,
    -- each type and index it takes substituted.
    if t.hasLooseBVars then return .continue
    match ← respellOperation? t with
    | some t' => return .done t'
    | none => return .continue)

/-- The canonical spelling of an expression's families, where it converts.
An expression the search left as it was is not searched again. -/
private def canonicalFamilyResult (e : Lean.Expr) : MetaM Simp.Result := do
  if (← (← targetState).canonicalForms.get).contains e then return { expr := e }
  let canonical ← canonicalFamilies e
  if canonical == e then
    unless e.hasMVar do (← targetState).canonicalForms.modify (·.insert e)
    return { expr := e }
  if ← isDefEq e canonical then return { expr := canonical }
  return { expr := e }

/-- Make the family spellings of the goals canonical, their targets too
where `target` holds (`canonicalFamilies`). -/
def canonicalFamiliesOf (target : Bool) : TacticM Unit := do
  let goals ← getGoals
  let mut result := #[]
  for goal in goals do
    if ← goal.isAssigned then continue
    let mut goal := goal
    if target then
      let type ← goal.withContext (instantiateMVars (← goal.getType))
      let canonical ← goal.withContext (canonicalFamilyResult type)
      if canonical.expr != type then
        goal ← goal.withContext (applySimpResultToTarget goal type canonical)
    for decl in ← goal.withContext getLCtx do
      if decl.isImplementationDetail then continue
      let type ← goal.withContext (instantiateMVars decl.type)
      let canonical ← goal.withContext (canonicalFamilyResult type)
      if canonical.expr != type then
        let some (_, next) ← goal.withContext
            (applySimpResultToLocalDecl goal decl.fvarId canonical false) | continue
        goal := next
    result := result.push goal
  setGoals result.toList

elab_rules : tactic
  | `(tactic| verex_canonical_families) => canonicalFamiliesOf true

elab_rules : tactic
  | `(tactic| verex_canonical_hypotheses) => canonicalFamiliesOf false

/-- The outer frame, the type arguments, and their spelled row of a frame a
call's type arguments induce. -/
private def inducedFrame? (frame : Lean.Expr) : Option (Lean.Expr × Lean.Expr × Lean.Expr) := do
  let frame := frame.consumeMData
  guard (frame.isAppOfArity ``VerexIR.Proofs.Denote.Skolems.instantiate 4)
  let θ := frame.getArg! 2
  pure (θ, frame.getArg! 3, ← substitutionRow? θ)

/-- A literal failure, as compiled code throws one (a pattern mismatch), in
the form the clause naming its abort code reads (`Failure.ofInts`). -/
dsimproc [vir_denote] literalFailure (Prod.mk _ _) := fun e => do
  let_expr Prod.mk kindType _ kind payload := e | return .continue
  unless kindType.isConstOf ``VerexIR.ThrowKind do return .continue
  let some (elementType, _) := payload.arrayLit? | return .continue
  unless elementType.isConstOf ``Int do return .continue
  return .done (mkApp2 (mkConst ``VerexIR.Proofs.Failure.ofInts) kind payload)

/-- A value carried from the runtime family to an induced frame, as the
outer frame carries it at the type's substitution, evaluated
(`Skolems.ofRuntime_instantiate`): it then meets the outer frame's
transports, also where the value's type needs the substitution evaluated. -/
dsimproc ↓ [vir_denote, vir_denote_norm] ofRuntimeOuterFrame
    (@VerexIR.Proofs.Denote.Skolems.ofRuntime _ ?functions ?frame _ _) := fun e => do
  let_expr VerexIR.Proofs.Denote.Skolems.ofRuntime unit functions frame τ value := e
    | return .continue
  let some (θ, outer, row) := inducedFrame? frame | return .continue
  let some substituted := substituteNTy row τ | return .continue
  let carriers := mkApp3 (mkConst ``VerexIR.Proofs.Denote.Skolems.toCarriers) unit functions outer
  return .visit (mkApp4 (mkConst ``VerexIR.Proofs.Denote.NTy.toSkolem) carriers θ τ
    (mkApp5 e.getAppFn unit functions outer substituted value))

/-- A value carried from an induced frame to the runtime family, as the
outer frame carries it (`Skolems.toRuntime_instantiate`, `ofRuntimeOuterFrame`). -/
dsimproc ↓ [vir_denote, vir_denote_norm] toRuntimeOuterFrame
    (@VerexIR.Proofs.Denote.Skolems.toRuntime _ ?functions ?frame _ _) := fun e => do
  let_expr VerexIR.Proofs.Denote.Skolems.toRuntime unit functions frame τ value := e
    | return .continue
  let some (θ, outer, row) := inducedFrame? frame | return .continue
  let some substituted := substituteNTy row τ | return .continue
  let carriers := mkApp3 (mkConst ``VerexIR.Proofs.Denote.Skolems.toCarriers) unit functions outer
  return .visit (mkApp5 e.getAppFn unit functions outer substituted
    (mkApp4 (mkConst ``VerexIR.Proofs.Denote.NTy.ofSkolem) carriers θ τ value))

/-- Whether a term rebuilds a value from its components: the value itself,
or a pair of its first component and a rebuild of its second, down to the
unit a row ends with. -/
private partial def rebuilds (term value : Lean.Expr) : Bool :=
  term == value ||
    (term.isAppOfArity ``Prod.mk 4 &&
      (let first := term.getArg! 2
       first.isAppOfArity ``Prod.fst 3 && first.getArg! 2 == value &&
         (let rest := term.getArg! 3
          let tail := mkApp3 (mkConst ``Prod.snd first.getAppFn.constLevels!) (first.getArg! 0)
            (first.getArg! 1) value
          rebuilds rest tail ||
            ((rest.isConstOf ``Unit.unit || rest.isAppOfArity ``PUnit.unit 0) &&
              let rowType := first.getArg! 1
              rowType.isConstOf ``Unit || rowType.isAppOfArity ``PUnit 0 ||
                (rowType.isAppOfArity ``VerexIR.Proofs.Denote.HList 2 &&
                  (rowType.getArg! 1).isConstOf ``VerexIR.Proofs.Denote.NRow.nil)))))

/-- Mapping the identity over an array is the array: a function that
returns its argument, or rebuilds it component by component as the
transport of a row whose types need no substitution does, also where the
element type is spelled differently on the two sides. -/
simproc [vir_denote, vir_denote_norm] mapIdentity (Array.map _ _) := fun e => do
  let_expr Array.map element _ function array := e | return .continue
  let .lam name domain body info := function | return .continue
  unless rebuilds body (.bvar 0) do return .continue
  let levels := [e.getAppFn.constLevels!.head!]
  let identity := mkApp (mkConst ``Array.map_id'' levels) element
  let proof ← withLocalDecl name info domain fun x => do
    let reflexive ← mkEqRefl x
    let pointwise ← mkLambdaFVars #[x] reflexive
    return mkApp3 identity function pointwise array
  let proof ← mkExpectedTypeHint proof (← mkEq e array)
  return .done { expr := array, proof? := some proof }

/-- The transport a function carries values with, `NTy.toSkolem θ τ`, also
eta-expanded: its family, type arguments and callee type. -/
private def valueTransport? (function : Lean.Expr) : Option (Lean.Expr × Lean.Expr × Lean.Expr) :=
  let function := function.consumeMData
  let body := match function with
    | .lam _ _ body _ =>
        if body.isAppOfArity ``VerexIR.Proofs.Denote.NTy.toSkolem 4 && body.appArg! == Lean.mkBVar 0 then
          some body.appFn!
        else none
    | _ => some function
  match body with
  | some transport =>
      if transport.isAppOfArity ``VerexIR.Proofs.Denote.NTy.toSkolem 3 then
        some (transport.getArg! 0, transport.getArg! 1, transport.getArg! 2)
      else none
  | none => none

/-- A caller's values carried into a callee's view element by element are
the caller's values (`FamilyTransport.toSkolemIdentity`): what reading an
encoding through a transport states of encodings (`encodeTransport`), of
the values themselves. -/
simproc [vir_denote, vir_denote_norm] mapTransportIdentity (Array.map _ _) := fun e => do
  let_expr Array.map element _ function array := e | return .continue
  let some (outer, θ, τ) := valueTransport? function | return .continue
  let some identity ← (try some <$> FamilyTransport.toSkolemIdentity outer θ τ element
      catch _ => pure none) | return .continue
  let pointwise ← withLocalDeclD `value element fun value => do
    mkLambdaFVars #[value] (← mkAppM ``congrFun #[identity, value])
  let proof := mkApp4 (mkConst ``Array.map_id'' [e.getAppFn.constLevels!.head!]) element function
    pointwise array
  let proof ← mkExpectedTypeHint proof (← mkEq e array)
  return .visit { expr := array, proof? := some proof }

/-- An optional caller's value carried into a callee's view is the caller's
value (`mapTransportIdentity`). -/
simproc [vir_denote, vir_denote_norm] optionTransportIdentity (Option.map _ _) := fun e => do
  let_expr Option.map element _ function option := e | return .continue
  let some (outer, θ, τ) := valueTransport? function | return .continue
  let some identity ← (try some <$> FamilyTransport.toSkolemIdentity outer θ τ element
      catch _ => pure none) | return .continue
  let mapped ← withLocalDeclD `function (← inferType function) fun g => do
    mkLambdaFVars #[g] (← mkAppM ``Option.map #[g, option])
  let congruence ← mkAppM ``congrArg #[mapped, identity]
  let proof ← mkEqTrans congruence (← mkAppOptM ``Option.map_id' #[element, option])
  let proof ← mkExpectedTypeHint proof (← mkEq e option)
  return .visit { expr := option, proof? := some proof }

/-- A callee's value of a type without type parameters, read in the
caller's view, is the value itself: both views hold the same values
(`FamilyTransport.pointwiseIdentity`). The transport's head, its family and
type arguments, and the types it reads at. -/
private def paramFreeTransportStep (head : Name) (typeArguments : Nat) : Simp.Simproc := fun e => do
  unless e.isAppOfArity head (3 + typeArguments) do return .continue
  let arguments := e.getAppArgs
  let types := arguments.extract 2 (2 + typeArguments)
  if types.any (fun type => (type.find? (·.isAppOf ``VerexIR.Proofs.Denote.NTy.param)).isSome) then
    return .continue
  let value := arguments.back!
  let transport := fun (held : Lean.Expr) => mkAppN e.getAppFn (arguments.pop.push held)
  let some identity ← (try some <$> FamilyTransport.pointwiseIdentity (← inferType value) transport
      catch _ => pure none) | return .continue
  let proof ← mkAppM ``congrFun #[identity, value]
  return .visit { expr := value, proof? := some (← mkExpectedTypeHint proof (← mkEq e value)) }

simproc [vir_denote_norm] paramFreeValueTransport
    (@VerexIR.Proofs.Denote.NTy.ofSkolem ?carriers _ _ _) :=
  paramFreeTransportStep ``VerexIR.Proofs.Denote.NTy.ofSkolem 1
simproc [vir_denote_norm] paramFreeVariantTransport
    (@VerexIR.Proofs.Denote.variantCarrier.ofSkolem ?carriers _ _ _ _) :=
  paramFreeTransportStep ``VerexIR.Proofs.Denote.variantCarrier.ofSkolem 2
simproc [vir_denote_norm] paramFreeRowTransport
    (@VerexIR.Proofs.Denote.HList.ofSkolem ?carriers _ _ _) :=
  paramFreeTransportStep ``VerexIR.Proofs.Denote.HList.ofSkolem 1

/-- A callee's value in the caller's view holds the variant it holds at the
callee's family (`variantName_ofSkolem`). -/
simproc ↓ [vir_denote, vir_denote_norm] variantNameTransport
    (@VerexIR.Proofs.Denote.variantName ?carriers _ _ _) :=
  fun e => do
    let_expr VerexIR.Proofs.Denote.variantName carriers _ _ value := e | return .continue
    let_expr VerexIR.Proofs.Denote.Skolems.toCarriers unit functions outer := carriers.consumeMData
      | return .continue
    let_expr VerexIR.Proofs.Denote.variantCarrier.ofSkolem _ θ names rows v := value
      | return .continue
    let proof := mkAppN (mkConst ``VerexIR.Proofs.Denote.variantName_ofSkolem)
      #[unit, functions, outer, θ, names, rows, v]
    let some (_, _, name) := (← inferType proof).eq? | return .continue
    return .visit { expr := name, proof? := some proof }

/-- A variant observation (`variantName`, `variantPayload`) of a value at
the family a call induces, as the outer family spells it: the value is the
value at the outer family (`variantCarrier.ofSkolem_heq`), whose
observation is the one at the callee's (`variantName_ofSkolem`,
`variantPayload_ofSkolem`). The rows are substituted as spelled. -/
private def variantOuterFamily (transport : Name) (e : Lean.Expr) : SimpM Simp.Step := do
  let e := e.consumeMData
  -- `variantName` takes the family's carriers, `variantPayload` the frame.
  let (unit, functions, skolems, names, rows, value, carried) ← match e.getAppArgs with
    | #[unit, functions, skolems, names, rows, value] =>
        pure (unit, functions, skolems, names, rows, value, false)
    | #[carriers, names, rows, value] =>
        let_expr VerexIR.Proofs.Denote.Skolems.toCarriers unit functions skolems :=
            carriers.consumeMData
          | return .continue
        pure (unit, functions, skolems, names, rows, value, true)
    | _ => return .continue
  let_expr VerexIR.Proofs.Denote.Skolems.instantiate _ _ θ outer := skolems.consumeMData
    | return .continue
  let some row := substitutionRow? θ | return .continue
  let some substituted := substituteNRows row rows | return .continue
  let some (result, equation) ← (try withTransparency .default do
      let transported := mkAppN (mkConst transport) #[unit, functions, outer, θ, names, rows, value]
      let some (_, atOuter, _) := (← inferType transported).eq? | return none
      let ofSkolem := atOuter.appArg!
      let_expr VerexIR.Proofs.Denote.variantCarrier.ofSkolem carriers _ _ _ _ := ofSkolem
        | return none
      let carrier ← inferType ofSkolem
      unless ← isDefEq carrier (← inferType value) do return none
      let heq ← mkAppOptM ``VerexIR.Proofs.Denote.variantCarrier.ofSkolem_heq
        #[some carriers, some θ, some names, some rows, some value]
      let identity ← mkAppOptM ``eq_of_heq #[some carrier, some ofSkolem, some value, some heq]
      let observe ← withLocalDeclD `value carrier fun value =>
        mkLambdaFVars #[value] (mkApp atOuter.appFn! value)
      let proof ← mkEqTrans (← mkEqSymm transported) (← mkCongrArg observe identity)
      let result := if carried then
          mkAppN e.getAppFn
            #[mkApp3 (mkConst ``VerexIR.Proofs.Denote.Skolems.toCarriers) unit functions outer,
              names, substituted, value]
        else mkAppN e.getAppFn #[unit, functions, outer, names, substituted, value]
      pure (some (result, ← mkExpectedTypeHint proof (← mkEq e result)))
    catch _ => pure none)
    | return .continue
  return .visit { expr := result, proof? := some equation }

simproc ↓ [vir_denote_norm] variantNameOuterFamily
    (@VerexIR.Proofs.Denote.variantName ?carriers _ _ _) :=
  variantOuterFamily ``VerexIR.Proofs.Denote.variantName_ofSkolem

/-- Native equality at the family a call's type arguments induce, as the
outer family spells it (`NTy.equal_ofSkolem`): of the values read in the
caller's view, at the substituted type. A type without structs compares
alike whether or not a handle identifies a value, as only a struct's
comparison reads it. -/
simproc ↓ [vir_denote_norm] equalOuterFamily
    (@VerexIR.Proofs.Denote.NTy.equal ?carriers _ _ _ _) := fun e => do
  let e := e.consumeMData
  let_expr VerexIR.Proofs.Denote.NTy.equal family handles τ left right := e | return .continue
  let some (θ, outer) := inducedCarriers? family.consumeMData | return .continue
  let structFree := (τ.find? (·.isAppOf ``VerexIR.Proofs.Denote.NTy.struct)).isNone
  unless handles.isConstOf ``Bool.false || structFree do return .continue
  let some row := substitutionRow? θ | return .continue
  let some substituted := substituteNTy row τ | return .continue
  let some (value, equation) ← (try withTransparency .default do
      let atFalse := mkAppN (mkConst ``VerexIR.Proofs.Denote.NTy.equal)
        #[family, mkConst ``Bool.false, τ, left, right]
      let first ← mkExpectedTypeHint (← mkEqRefl e) (← mkEq e atFalse)
      let transported := mkAppN (mkConst ``VerexIR.Proofs.Denote.NTy.equal_ofSkolem)
        #[outer, θ, τ, left, right]
      let view (value : Lean.Expr) :=
        mkAppN (mkConst ``VerexIR.Proofs.Denote.NTy.ofSkolem) #[outer, θ, τ, value]
      let value := mkAppN (mkConst ``VerexIR.Proofs.Denote.NTy.equal)
        #[outer, mkConst ``Bool.false, substituted, view left, view right]
      let proof ← mkEqTrans first transported
      pure (some (value, ← mkExpectedTypeHint proof (← mkEq e value)))
    catch _ => pure none)
    | return .continue
  return .visit { expr := value, proof? := some equation }

/-- The name at a position of a literal list of names. -/
private partial def literalNameAt? (list : Lean.Expr) (position : Nat) : Option String :=
  let list := list.consumeMData
  if list.isAppOfArity ``List.cons 3 then
    if position == 0 then
      match (list.getArg! 1).consumeMData with
      | .lit (.strVal name) => some name
      | _ => none
    else literalNameAt? (list.getArg! 2) (position - 1)
  else none

/-- The variant a constructed enum value holds, read off its injections
(`variantName_inl`, `variantName_inr`) definitionally: the decided
inhabitation of an induced family's type arguments defeats simp's matching
of the lemmas at a callee's instance. -/
simproc [vir_denote, vir_denote_norm] variantNameConstructor
    (@VerexIR.Proofs.Denote.variantName ?carriers _ _ _) := fun e => do
  let_expr VerexIR.Proofs.Denote.variantName _ names _ value := e | return .continue
  let mut position := 0
  let mut current := value.consumeMData
  while current.isAppOfArity ``Sum.inr 3 do
    position := position + 1
    current := (current.getArg! 2).consumeMData
  unless current.isAppOfArity ``Sum.inl 3 do return .continue
  let some name := literalNameAt? names position | return .continue
  return .visit { expr := mkStrLit name }

/-- The row a native row expression denotes, in constructor form. -/
private def rowConstructor (row : Lean.Expr) : MetaM Lean.Expr := do
  let row := row.consumeMData
  if row.isAppOfArity ``VerexIR.Proofs.Denote.NRow.cons 2 || row.isConstOf ``VerexIR.Proofs.Denote.NRow.nil then
    return row
  whnfD row

/-- The rows a native rows expression denotes, in constructor form. -/
private def rowsConstructor (rows : Lean.Expr) : MetaM Lean.Expr := do
  let rows := rows.consumeMData
  if rows.isAppOfArity ``VerexIR.Proofs.Denote.NRows.cons 2 || rows.isConstOf ``VerexIR.Proofs.Denote.NRows.nil then
    return rows
  whnfD rows

/-- The components of a value of a product carrier: a pair's, or its
projections. -/
private def pairComponents (value : Lean.Expr) : Lean.Expr × Lean.Expr :=
  let value := value.consumeMData
  if value.isAppOfArity ``Prod.mk 4 then (value.getArg! 2, value.getArg! 3)
  else (.proj ``Prod 0 value, .proj ``Prod 1 value)

/-- The injections a constructed enum value is made of: its variant's
position and payload. -/
private def injections? (value : Lean.Expr) : Option (Nat × Lean.Expr) := Id.run do
  let mut position := 0
  let mut current := value.consumeMData
  while current.isAppOfArity ``Sum.inr 3 do
    position := position + 1
    current := (current.getArg! 2).consumeMData
  unless current.isAppOfArity ``Sum.inl 3 do return none
  return some (position, current.getArg! 2)

/-- The names and rows after the first `count` variants, with the row at
that position. -/
private def variantAt? (names rows : Lean.Expr) (count : Nat) :
    MetaM (Option (Lean.Expr × Lean.Expr × Lean.Expr)) := do
  let mut names := names.consumeMData
  let mut rows ← rowsConstructor rows
  for _ in [:count] do
    unless names.isAppOfArity ``List.cons 3 do return none
    let_expr VerexIR.Proofs.Denote.NRows.cons _ rest := rows | return none
    names := (names.getArg! 2).consumeMData
    rows ← rowsConstructor rest
  unless names.isAppOfArity ``List.cons 3 do return none
  let_expr VerexIR.Proofs.Denote.NRows.cons fields _ := rows | return none
  return some (names, rows, fields)

/-- A constructed enum value transported to a callee's family, injection by
injection (`variantCarrier.toSkolem_inl`, `variantCarrier.toSkolem_inr`)
definitionally. -/
dsimproc [vir_denote, vir_denote_norm] variantToSkolemConstructor
    (@VerexIR.Proofs.Denote.variantCarrier.toSkolem ?outer _ _ _ _) := fun e => do
  let e := e.consumeMData
  let_expr VerexIR.Proofs.Denote.variantCarrier.toSkolem outer θ names rows value := e
    | return .continue
  let some (position, payload) := injections? value | return .continue
  let inner := mkApp2 (mkConst ``VerexIR.Proofs.Denote.Carriers.instantiate) θ outer
  -- The variants from the innermost injection outwards.
  let mut levels : Array (Lean.Expr × Lean.Expr × Lean.Expr) := #[]
  for i in [:position + 1] do
    let some level ← variantAt? names rows i | return .continue
    levels := levels.push level
  let some (_, _, fields) := levels[position]? | return .continue
  let mut built := mkApp4 (mkConst ``VerexIR.Proofs.Denote.HList.toSkolem) outer θ fields payload
  for i in (List.range (position + 1)).reverse do
    let some (names, rows, fields) := levels[i]? | return .continue
    let_expr VerexIR.Proofs.Denote.NRows.cons _ rest := rows | return .continue
    let left := mkApp2 (mkConst ``VerexIR.Proofs.Denote.HList) inner fields
    let right := mkApp3 (mkConst ``VerexIR.Proofs.Denote.variantCarrier) inner
      (names.getArg! 2) rest
    built := if i == position then
      mkApp3 (mkConst ``Sum.inl [Level.zero, Level.zero]) left right built
    else mkApp3 (mkConst ``Sum.inr [Level.zero, Level.zero]) left right built
  return .visit built

/-- A row's values transported to a callee's family, component by
component (`HList.toSkolem_mk`) definitionally. -/
dsimproc [vir_denote, vir_denote_norm] hlistToSkolemConstructor
    (@VerexIR.Proofs.Denote.HList.toSkolem ?outer _ _ _) := fun e => do
  let e := e.consumeMData
  let_expr VerexIR.Proofs.Denote.HList.toSkolem outer θ row values := e | return .continue
  let row ← rowConstructor row
  if row.isConstOf ``VerexIR.Proofs.Denote.NRow.nil then
    return .done (mkConst ``Unit.unit)
  let_expr VerexIR.Proofs.Denote.NRow.cons τ rest := row | return .continue
  let inner := mkApp2 (mkConst ``VerexIR.Proofs.Denote.Carriers.instantiate) θ outer
  let (head, tail) := pairComponents values
  let headType := mkApp2 (mkConst ``VerexIR.Proofs.Denote.NTy.carrier) inner τ
  let tailType := mkApp2 (mkConst ``VerexIR.Proofs.Denote.HList) inner rest
  return .visit (mkApp4 (mkConst ``Prod.mk [Level.zero, Level.zero]) headType tailType
    (mkApp4 (mkConst ``VerexIR.Proofs.Denote.NTy.toSkolem) outer θ τ head)
    (mkApp4 (mkConst ``VerexIR.Proofs.Denote.HList.toSkolem) outer θ rest tail))

/-- A transport of a value between a caller's family and a callee's is the
value, where the two carriers are one type: at a concrete instantiation
they unfold alike (`NTy.ofSkolem_heq` and its kin). The equation is stated
at the transport's type, which the value's unfolds to. -/
private def transportIdentity (identity : Name) (arguments : Array Lean.Expr) (value e : Lean.Expr) :
    SimpM Simp.Step := do
  let some equation ← (try withTransparency .default do
      let type ← inferType e
      unless ← isDefEq type (← inferType value) do return none
      let heq ← mkAppOptM identity (arguments.map some ++ #[some value])
      let proof ← mkAppOptM ``eq_of_heq #[some type, some e, some value, some heq]
      pure (some (← mkExpectedTypeHint proof (← mkEq e value)))
    catch _ => pure none)
    | return .continue
  return .visit { expr := value, proof? := some equation }

simproc ↓ [vir_denote, vir_denote_norm] ofSkolemIdentity
    (@VerexIR.Proofs.Denote.NTy.ofSkolem ?outer _ _ _) := fun e => do
  let e := e.consumeMData
  let_expr VerexIR.Proofs.Denote.NTy.ofSkolem outer θ τ value := e | return .continue
  transportIdentity ``VerexIR.Proofs.Denote.NTy.ofSkolem_heq #[outer, θ, τ] value e
simproc ↓ [vir_denote, vir_denote_norm] toSkolemIdentity
    (@VerexIR.Proofs.Denote.NTy.toSkolem ?outer _ _ _) := fun e => do
  let e := e.consumeMData
  let_expr VerexIR.Proofs.Denote.NTy.toSkolem outer θ τ value := e | return .continue
  transportIdentity ``VerexIR.Proofs.Denote.NTy.toSkolem_heq #[outer, θ, τ] value e
simproc ↓ [vir_denote, vir_denote_norm] hlistOfSkolemIdentity
    (@VerexIR.Proofs.Denote.HList.ofSkolem ?outer _ _ _) := fun e => do
  let e := e.consumeMData
  let_expr VerexIR.Proofs.Denote.HList.ofSkolem outer θ row values := e | return .continue
  transportIdentity ``VerexIR.Proofs.Denote.HList.ofSkolem_heq #[outer, θ, row] values e
simproc ↓ [vir_denote, vir_denote_norm] hlistToSkolemIdentity
    (@VerexIR.Proofs.Denote.HList.toSkolem ?outer _ _ _) := fun e => do
  let e := e.consumeMData
  let_expr VerexIR.Proofs.Denote.HList.toSkolem outer θ row values := e | return .continue
  transportIdentity ``VerexIR.Proofs.Denote.HList.toSkolem_heq #[outer, θ, row] values e
simproc ↓ [vir_denote, vir_denote_norm] variantOfSkolemIdentity
    (@VerexIR.Proofs.Denote.variantCarrier.ofSkolem ?outer _ _ _ _) := fun e => do
  let e := e.consumeMData
  let_expr VerexIR.Proofs.Denote.variantCarrier.ofSkolem outer θ names rows value := e
    | return .continue
  transportIdentity ``VerexIR.Proofs.Denote.variantCarrier.ofSkolem_heq #[outer, θ, names, rows]
    value e
simproc ↓ [vir_denote, vir_denote_norm] variantToSkolemIdentity
    (@VerexIR.Proofs.Denote.variantCarrier.toSkolem ?outer _ _ _ _) := fun e => do
  let e := e.consumeMData
  let_expr VerexIR.Proofs.Denote.variantCarrier.toSkolem outer θ names rows value := e
    | return .continue
  transportIdentity ``VerexIR.Proofs.Denote.variantCarrier.toSkolem_heq #[outer, θ, names, rows]
    value e

/-- The default of a type at the family a call induces, as the outer family
spells it (`NTy.inhabitantOf_instantiate`): a typed map's reads stated by a
callee then name the map's own instance. The default is irreducible, so the
two spellings are equal by the lemma, not by unfolding. -/
simproc ↓ [vir_denote_norm] inhabitantOuterFamily
    (@VerexIR.Proofs.Denote.NTy.inhabitantOf ?family _ _) := fun e => do
  let e := e.consumeMData
  let_expr VerexIR.Proofs.Denote.NTy.inhabitantOf family τ inhabitable := e | return .continue
  let some (θ, outer) := inducedCarriers? family | return .continue
  let some row := substitutionRow? θ | return .continue
  let some substituted := substituteNTy row τ | return .continue
  let some (value, equation) ← (try withTransparency .default do
      let decided ← mkExpectedTypeHint (← mkEqRefl (mkConst ``Bool.true))
        (← mkEq (mkApp (mkConst ``VerexIR.Proofs.Denote.NTy.inhabitable) substituted)
          (mkConst ``Bool.true))
      let value := mkApp3 (mkConst ``VerexIR.Proofs.Denote.NTy.inhabitantOf) outer substituted
        decided
      let type ← inferType e
      unless ← isDefEq type (← inferType value) do return none
      let heq ← mkAppOptM ``VerexIR.Proofs.Denote.NTy.inhabitantOf_instantiate
        #[some outer, some θ, some τ, some inhabitable, some decided]
      let proof ← mkAppOptM ``eq_of_heq #[some type, some e, some value, some heq]
      pure (some (value, ← mkExpectedTypeHint proof (← mkEq e value)))
    catch _ => pure none)
    | return .continue
  return .visit { expr := value, proof? := some equation }

/-- The typed view of a map's key or value type, at the map's own family:
`carrier` is that type, as the map's type spells it, and `inhabited` the
default its view names. -/
private def ownView? (carrier : Lean.Expr) : MetaM (Option NativeMaps.TypedView) := do
  let_expr VerexIR.Proofs.Denote.NTy.carrier carriers τ := carrier.consumeMData | return none
  let some view ← NativeMaps.typedView? carriers τ | return none
  unless view.carrier == carrier do return none
  return some view

/-- The key and value types of a map term, as its type names them. -/
private def mapTypes? (map : Lean.Expr) : MetaM (Option (Lean.Expr × Lean.Expr)) := do
  let type ← instantiateMVars (← inferType map)
  if let .app (.app (.const ``VerexIR.SpecMap _) K) V := type.consumeMData then return some (K, V)
  let_expr List pair := ← whnfR type | return none
  let_expr Prod K V := pair.consumeMData | return none
  return some (K, V)

/-- The instance of a typed map's key or value type a read names, related to
the map's own (`ownView?`): `some none` when the two are one instance by
unfolding, `some (some proof)` when the read's is spelled at the family a
call induces and the defaults are equal by `NTy.inhabitantOf_instantiate`,
which the instance position hides from rewriting. -/
private def ownInstance? (inhabited : Lean.Expr) (view : NativeMaps.TypedView) :
    MetaM (Option (Option Lean.Expr)) := do
  -- Unfolding the two instances compares their types' codecs; only an
  -- instance at an induced family is related by the lemma, any other by
  -- reducible unfolding.
  let induced? : Option (Lean.Expr × Lean.Expr × Lean.Expr × Lean.Expr) := do
    let_expr VerexIR.Proofs.Denote.nativeInhabited family τ fallback := inhabited | none
    let_expr VerexIR.Proofs.Denote.NTy.inhabitantOf _ _ inhabitable := fallback | none
    let (θ, induced) ← inducedCarriers? family
    some (θ, induced, τ, inhabitable)
  try withTransparency .default do
    let some (θ, induced, τ, inhabitable) := induced?
      | if ← withReducible (isDefEq inhabited view.inhabited) then return some none else return none
    let_expr VerexIR.Proofs.Denote.nativeInhabited outer substituted own := view.inhabited
      | return none
    let_expr VerexIR.Proofs.Denote.NTy.inhabitantOf _ _ decided := own | return none
    let fallback := inhabited.appArg!
    let heq ← mkAppOptM ``VerexIR.Proofs.Denote.NTy.inhabitantOf_instantiate
      #[some induced, some θ, some τ, some inhabitable, some decided]
    let carrier ← inferType own
    let defaults ← mkAppOptM ``eq_of_heq #[some carrier, some fallback, some own, some heq]
    let instanceOf ← withLocalDeclD `fallback carrier fun fallback =>
      mkLambdaFVars #[fallback]
        (mkApp3 (mkConst ``VerexIR.Proofs.Denote.nativeInhabited) outer substituted fallback)
    let proof ← mkCongrArg instanceOf defaults
    return some (some (← mkExpectedTypeHint proof (← mkEq inhabited view.inhabited)))
  catch _ => return none

/-- A map read respelled at the map's own type and instance for one argument
position, with the proof of the instance equation when it is not
definitional. -/
private def ownReadStep (e : Lean.Expr) (typePosition instancePosition : Nat) (own : Lean.Expr)
    (view : Unit → MetaM (Option NativeMaps.TypedView)) : SimpM Simp.Step := do
  let args := e.getAppArgs
  let inhabited := args[instancePosition]!
  -- A read at the map's own type whose instance is not at an induced family
  -- is as the map spells it; deciding the view costs more than most reads.
  let induced := match inhabited.consumeMData with
    | .app (.app (.app (.const ``VerexIR.Proofs.Denote.nativeInhabited _) family) _) _ =>
        (inducedCarriers? family).isSome
    | _ => false
  if args[typePosition]! == own && !induced then return .continue
  let some view ← view () | return .continue
  if args[typePosition]! == own && inhabited == view.inhabited then return .continue
  let some proof? ← ownInstance? inhabited view | return .continue
  let value := mkAppN e.getAppFn
    ((args.set! typePosition own).set! instancePosition view.inhabited)
  let some instanceEq := proof? | return .visit { expr := value }
  let some equation ← (try withTransparency .default do
      let read ← withLocalDeclD `inhabited (← mkAppM ``Inhabited #[own]) fun inhabited =>
        mkLambdaFVars #[inhabited]
          (mkAppN e.getAppFn ((args.set! typePosition own).set! instancePosition inhabited))
      let proof ← mkCongrArg read instanceEq
      pure (some (← mkExpectedTypeHint proof (← mkEq e value)))
    catch _ => pure none)
    | return .continue
  return .visit { expr := value, proof? := some equation }

/-- A typed map's value read at the map's own value type and default: a
contract names the value type and its default at the family its call
induces, while the map, named by the caller, is at the caller's; the two
spellings are one type, and the defaults one value (`ownInstance?`). -/
simproc [vir_denote_norm] valueAtOwnInstance (VerexIR.SpecMap.valueAt _ _) :=
  fun e => do
  let e := e.consumeMData
  let_expr VerexIR.SpecMap.valueAt _ _ _ _ map _ := e | return .continue
  let some (_, V) ← mapTypes? map | return .continue
  ownReadStep e 1 3 V fun _ => ownView? V

/-- A typed map's key read at the map's own key type and default
(`valueAtOwnInstance`). -/
simproc [vir_denote_norm] keyAtOwnInstance (VerexIR.SpecMap.keyAt _ _) :=
  fun e => do
  let e := e.consumeMData
  let_expr VerexIR.SpecMap.keyAt _ _ _ map _ := e | return .continue
  let some (K, _) ← mapTypes? map | return .continue
  ownReadStep e 0 2 K fun _ => ownView? K

/-- The component a variable selects from a row of values, by the
variable's constructors (`Var.select`) definitionally. -/
dsimproc [vir_denote, vir_denote_norm] varSelectConstructor
    (@VerexIR.Proofs.Denote.Var.select ?inst _ _ _ _) := fun e => do
  let e := e.consumeMData
  let_expr VerexIR.Proofs.Denote.Var.select _ _ _ selector values := e | return .continue
  let mut selector := selector.consumeMData
  let mut values := values
  while selector.isAppOfArity ``VerexIR.Proofs.Denote.Var.there 4 do
    selector := (selector.getArg! 3).consumeMData
    values := (pairComponents values).2
  unless selector.isAppOfArity ``VerexIR.Proofs.Denote.Var.here 2 do return .continue
  return .visit (pairComponents values).1

/-- A choice's projection of a constructed enum value, by the choice's and
the value's constructors (`Which.project?`) definitionally: the payload
where the injections agree, nothing otherwise. -/
dsimproc [vir_denote, vir_denote_norm] whichProjectConstructor
    (@VerexIR.Proofs.Denote.Which.project? _ ?functions ?skolems _ _ _ _ _) := fun e => do
  let e := e.consumeMData
  let_expr VerexIR.Proofs.Denote.Which.project? unit _ skolems _ _ σs choice value := e
    | return .continue
  let mut depth := 0
  let mut choice := choice.consumeMData
  while choice.isAppOfArity ``VerexIR.Proofs.Denote.Which.there 6 do
    depth := depth + 1
    choice := (choice.getArg! 5).consumeMData
  unless choice.isAppOfArity ``VerexIR.Proofs.Denote.Which.here 4 do return .continue
  let some (position, payload) := injections? value | return .continue
  -- The row's type as the projection states it, so that the lemmas on
  -- the option match the context's spelling.
  let _ := (unit, skolems, σs)
  let_expr Option row := (← instantiateMVars (← inferType e)) | return .continue
  if position == depth then
    return .visit (mkApp2 (mkConst ``Option.some [Level.zero]) row payload)
  return .visit (mkApp (mkConst ``Option.none [Level.zero]) row)

/-- Close `goal`, an equation between two projections of one variant value
`value`, case by case on the value: at a constructor both sides reduce. -/
private partial def projectionsAgree (goal : MVarId) (value : FVarId) : MetaM Bool := do
  try goal.refl; return true catch _ => pure ()
  -- A variant carrier is a sum by its definition, which `cases` does not unfold.
  let goal ← goal.withContext do
    goal.replaceLocalDeclDefEq value (← whnfD (← value.getType))
  let some subgoals ← (some <$> goal.cases value) <|> pure none | return false
  for subgoal in subgoals do
    if ← (do subgoal.mvarId.refl; pure true) <|> pure false then continue
    -- The rest of the variants, where the value is not the first.
    let some field := subgoal.fields.back? | return false
    unless field.isFVar do return false
    let sum ← subgoal.mvarId.withContext do
      return (← whnfD (← inferType field)).isAppOfArity ``Sum 2
    unless sum && (← projectionsAgree subgoal.mvarId field.fvarId!) do return false
  return true

open Lean Meta Simp in
/-- A choice's projection at a frame type arguments induce, at the outer
frame with the types substituted: equal, case by case on the value, though
not convertible, since the projection matches on the choice and the value
together, which keeps it an atom where the value is unknown. A step proof's
leaf reads a callee's results so (`verex_step_obligation`); the automatic
normal form keeps the callee's spelling. -/
simproc_decl whichProjectFamily
    (@VerexIR.Proofs.Denote.Which.project? _ ?functions ?skolems _ _ _ _ _) := fun e => do
  let e := e.consumeMData
  unless e.isAppOfArity ``VerexIR.Proofs.Denote.Which.project? 8 && !e.hasLooseBVars do
    return .continue
  let some (respelled, _) ← FamilyTransport.respelledOperation? e | return .continue
  let valueType ← inferType e.appArg!
  let statement ← withLocalDeclD `value valueType fun value => do
    mkForallFVars #[value] (← mkEq (mkApp e.appFn! value) (mkApp respelled.appFn! value))
  let proof ← mkFreshExprMVar statement
  let (value, goal) ← proof.mvarId!.intro1
  -- The carriers of the two frames agree by unfolding their definitions.
  unless ← withTransparency .default (projectionsAgree goal value) do return .continue
  return .visit { expr := respelled, proof? := some (mkApp (← instantiateMVars proof) e.appArg!) }

/-- A choice's projection of a constructed enum value, mapped: the mapped
payload, or nothing, typed as the map spells its result. A contract spells
the map at the caller's family and the projection at the callee's, which
no lemma on the option matches across. -/
dsimproc ↓ [vir_denote, vir_denote_norm] optionMapProjectConstructor
    (Option.map _ (@VerexIR.Proofs.Denote.Which.project? _ ?functions ?skolems _ _ _ _ _)) := fun e => do
  let e := e.consumeMData
  let_expr Option.map _ β function projection := e | return .continue
  let_expr VerexIR.Proofs.Denote.Which.project? _ _ _ _ _ _ choice value := projection
    | return .continue
  let mut depth := 0
  let mut choice := choice.consumeMData
  while choice.isAppOfArity ``VerexIR.Proofs.Denote.Which.there 6 do
    depth := depth + 1
    choice := (choice.getArg! 5).consumeMData
  unless choice.isAppOfArity ``VerexIR.Proofs.Denote.Which.here 4 do return .continue
  let some (position, payload) := injections? value | return .continue
  if position == depth then
    return .visit (mkApp2 (mkConst ``Option.some [Level.zero]) β (mkApp function payload).headBeta)
  return .visit (mkApp (mkConst ``Option.none [Level.zero]) β)

/-- An option's map and default at a constructor, definitionally: where the
option's element type is spelled at one family and the map's at another,
the lemmas do not match. -/
dsimproc [vir_denote, vir_denote_norm] optionMapSomeConstructor (Option.map _ (some _)) := fun e => do
  let e := e.consumeMData
  let_expr Option.map _ β function option := e | return .continue
  let_expr Option.some _ value := option.consumeMData | return .continue
  return .visit (mkApp2 (mkConst ``Option.some [Level.zero]) β (mkApp function value).headBeta)
dsimproc [vir_denote, vir_denote_norm] optionGetDSomeConstructor (Option.getD (some _) _) := fun e => do
  let e := e.consumeMData
  let_expr Option.getD _ option _ := e | return .continue
  let_expr Option.some _ value := option.consumeMData | return .continue
  return .visit value
dsimproc [vir_denote, vir_denote_norm] optionGetDNoneConstructor (Option.getD none _) := fun e => do
  let e := e.consumeMData
  let_expr Option.getD _ option fallback := e | return .continue
  unless option.consumeMData.isAppOfArity ``Option.none 1 do return .continue
  return .visit fallback

/-- Two injections compared, at whatever family each side spells its sum:
the same injection compares its payloads, different ones refute
(`Sum.inr.injEq`, `Sum.inl_ne_inr`), which the lemmas state at one spelling.
Keyed on every equation: a pattern naming the injections would fix the
equation's type to a sum, where a contract states it at the enum's carrier. -/
simproc [vir_denote, vir_denote_norm] sumInjectionFamilies (@Eq _ _ _) := fun e => do
  let e := e.consumeMData
  let_expr Eq _ lhs rhs := e | return .continue
  let lhs := lhs.consumeMData
  let rhs := rhs.consumeMData
  let left := if lhs.isAppOfArity ``Sum.inl 3 then some true
    else if lhs.isAppOfArity ``Sum.inr 3 then some false else none
  let right := if rhs.isAppOfArity ``Sum.inl 3 then some true
    else if rhs.isAppOfArity ``Sum.inr 3 then some false else none
  let (some leftInl, some rightInl) := (left, right) | return .continue
  let arguments := #[some (lhs.getArg! 0), some (lhs.getArg! 1), some (lhs.getArg! 2), some (rhs.getArg! 2)]
  let some equation ← (try withTransparency .default do
      if leftInl == rightInl then
        let proof ← mkAppOptM (if leftInl then ``Sum.inl.injEq else ``Sum.inr.injEq) arguments
        let some (_, _, inner) := (← inferType proof).eq? | return none
        pure (some (inner, ← mkExpectedTypeHint proof (← mkEq e inner)))
      else
        let proof ← mkAppM ``eq_false
          #[← mkAppOptM (if leftInl then ``Sum.inl_ne_inr else ``Sum.inr_ne_inl) arguments]
        pure (some (mkConst ``False, ← mkExpectedTypeHint proof (← mkEq e (mkConst ``False))))
    catch _ => pure none)
    | return .continue
  return .visit { expr := equation.1, proof? := some equation.2 }

/-- Two pairs compared, as rows of values are: the components compare
(`Prod.mk.injEq`), which the lemma states at a product type where a row
states the equation at its own, `HList (.cons τ rest)`, which unfolds to
the product only beyond the matching's transparency. -/
simproc [vir_denote_norm] pairInjectionRows (@Eq _ _ _) := fun e => do
  let e := e.consumeMData
  let_expr Eq type lhs rhs := e | return .continue
  let lhs := lhs.consumeMData
  let rhs := rhs.consumeMData
  unless lhs.isAppOfArity ``Prod.mk 4 && rhs.isAppOfArity ``Prod.mk 4 do return .continue
  if type.consumeMData.isAppOfArity ``Prod 2 then return .continue
  let some equation ← (try withTransparency .default do
      let proof ← mkAppOptM ``Prod.mk.injEq #[some (lhs.getArg! 0), some (lhs.getArg! 1),
        some (lhs.getArg! 2), some (lhs.getArg! 3), some (rhs.getArg! 2), some (rhs.getArg! 3)]
      let some (_, _, components) := (← inferType proof).eq? | return none
      pure (some (components, ← mkExpectedTypeHint proof (← mkEq e components)))
    catch _ => pure none)
    | return .continue
  return .visit { expr := equation.1, proof? := some equation.2 }

-- Constructed values compared: a different constructor refutes, the same
-- one compares its fields.
attribute [vir_denote_norm] Sum.inl.injEq Sum.inr.injEq reduceCtorEq

/- The normalization of a verification condition: the denotation's rules,
the weakest-precondition rules, and the propositional normal forms a leaf
is decided in. -/
attribute [vir_denote_norm] VerexIR.Proofs.Denote.variantName_inl
  VerexIR.Proofs.Denote.variantName_inr VerexIR.Proofs.Denote.ghostValueOf_same
  VerexIR.Proofs.Denote.ghostInitOf_same Weave.supplying_cons Weave.supplying_nil NRow.length
attribute [vir_denote_norm] VerexIR.Proofs.wp_choose VerexIR.Proofs.wp_assume
  VerexIR.Proofs.wp_certifyState VerexIR.Proofs.Denote.wp_prophecyOf
attribute [vir_denote_norm] VerexIR.Proofs.Denote.Weave.compose
-- A value a function states, before the equation is taken apart.
attribute [vir_denote_norm ↓] exists_apply_eq_apply

/-- A conjunction stated true, as a lemma's Boolean equation gives it where
its other side is decided: each conjunct true. -/
@[vir_denote_norm] theorem true_eq_and (a b : Bool) : (true = (a && b)) = (a = true ∧ b = true) := by
  cases a <;> cases b <;> decide

attribute [vir_denote_norm] decide_false decide_true Bool.decide_eq_true Bool.decide_eq_false
-- A branch on a condition neither arm reads, as `omega` splits it.
attribute [vir_denote_norm] dite_eq_ite
/-- The sequence of integers literal elements hold, if each element is
one. -/
private def literalIntegers? (elements : Lean.Expr) (value? : Lean.Expr → Option Lean.Expr) :
    MetaM (Option Lean.Expr) := do
  let some (_, elements) := elements.listLit? | return none
  let some values := elements.mapM value? | return none
  return some (← mkListLit (mkConst ``Int) values)

/-- `[a, …][k]?.getD 0`, the read of a literal sequence of integers. -/
private def literalIntegerRead (values index : Lean.Expr) : MetaM Lean.Expr := do
  mkAppM ``Option.getD #[← mkAppM ``GetElem?.getElem? #[values, index], mkIntLit 0]

/-- The program's read of a literal sequence of certified integers at an
arbitrary position (`[⟨a, _⟩, …][k].val`), as the read of the sequence of
their values, `[a, …][k]?.getD 0`. -/
simproc [vir_denote_norm] literalCertifiedRead (VerexIR.SpecInt.val _) := fun e => do
  if e.hasLooseBVars || !e.isAppOfArity ``VerexIR.SpecInt.val 3 then return .continue
  let element := e.getArg! 2
  unless element.isAppOfArity ``GetElem.getElem 8 do return .continue
  let some values ← literalIntegers? (element.getArg! 5) fun element =>
      if element.isAppOfArity ``VerexIR.SpecInt.mk 4 then some (element.getArg! 2) else none
    | return .continue
  let index := element.getArg! 6
  let rhs ← literalIntegerRead values index
  -- The read's bound may state the length as a literal.
  let proof := mkAppN (mkConst ``val_getElem_eq_getD_map_val)
    #[e.getArg! 0, e.getArg! 1, element.getArg! 5, index, element.getArg! 7]
  return .done { expr := rhs, proof? := some (← mkExpectedTypeHint proof (← mkEq e rhs)) }

/-- Optional and certified reads of the same integer literals must share a
normal form. Keep the actual fallback, including at out-of-bounds positions. -/
simproc [vir_denote_norm] literalOptionalCertifiedRead (VerexIR.SpecInt.val _) := fun e => do
  if e.hasLooseBVars || !e.isAppOfArity ``VerexIR.SpecInt.val 3 then return .continue
  let element := e.getArg! 2
  unless element.isAppOfArity ``Option.getD 3 do return .continue
  let lookup := element.getArg! 1
  unless lookup.isAppOfArity ``GetElem?.getElem? 7 do return .continue
  let some values ← literalIntegers? (lookup.getArg! 5) fun element =>
      if element.isAppOfArity ``VerexIR.SpecInt.mk 4 then some (element.getArg! 2) else none
    | return .continue
  let fallback := element.getArg! 2
  unless fallback.isAppOfArity ``VerexIR.SpecInt.mk 4 do return .continue
  let rhs ← mkAppM ``Option.getD #[
    ← mkAppM ``GetElem?.getElem? #[values, lookup.getArg! 6], fallback.getArg! 2]
  let proof ← mkAppM ``val_getD_getElem?_map_val #[lookup.getArg! 5, lookup.getArg! 6, fallback]
  return .done { expr := rhs, proof? := some (← mkExpectedTypeHint proof (← mkEq e rhs)) }

attribute [vir_denote_norm] Option.bind_fun_some

/-- A quantifier over a range with literal bounds is its instances, one per
position, when the range is short (at most 64 positions): the first position
is split off until the range is empty, each step decided by the kernel. -/
simproc [vir_denote_norm] expandLiteralRange (∀ _ : Int, _ ≤ _ → _ < _ → _) :=
  fun e => do
    let .forallE binder (.const ``Int []) body _ := e | return .continue
    let .forallE _ lowerBound rest _ := body | return .continue
    let .forallE _ upperBound predicate _ := rest | return .continue
    -- The premises' proofs are unused by the clause.
    if predicate.hasLooseBVar 0 || predicate.hasLooseBVar 1 then return .continue
    let_expr LE.le _ _ low lowVariable := lowerBound | return .continue
    let_expr LT.lt _ _ highVariable high := upperBound | return .continue
    unless lowVariable == .bvar 0 && highVariable == .bvar 1 do return .continue
    let some lowValue := low.int? | return .continue
    let some highValue := high.int? | return .continue
    unless lowValue < highValue && highValue ≤ lowValue + 64 do return .continue
    let intType : Lean.Expr := .const ``Int []
    -- The clause with the range's variable free at `#0`, and as a predicate.
    let clause := predicate.lowerLooseBVars 2 2
    let clauseAt (position : Lean.Expr) : Lean.Expr := clause.instantiate1 position
    let asPredicate : Lean.Expr := .lam binder intType clause .default
    -- From the last position back: `(∀ i, k ≤ i → i < high → P i) ↔ conjunction`.
    let mut conjunction : Lean.Expr := mkConst ``True
    let mut proof ← mkAppOptM ``VerexIR.Proofs.Denote.forall_int_range_empty
      #[asPredicate, high, high, ← mkDecideProof (← mkAppM ``LE.le #[high, high])]
    let mut position := highValue
    while position > lowValue do
      position := position - 1
      let here : Lean.Expr := toExpr position
      let short ← mkAppM ``LE.le #[high, ← mkAppM ``HAdd.hAdd #[here, (toExpr (64 : Int) : Lean.Expr)]]
      let split ← mkAppOptM ``VerexIR.Proofs.Denote.forall_int_range_split
        #[asPredicate, here, high, ← mkDecideProof (← mkAppM ``LT.lt #[here, high]),
          ← mkDecideProof short]
      let restCongruent ← mkAppM ``and_congr #[← mkAppM ``Iff.refl #[clauseAt here], proof]
      proof ← mkAppM ``Iff.trans #[split, restCongruent]
      conjunction := mkAnd (clauseAt here) conjunction
    return .visit { expr := conjunction, proof? := some (← mkAppM ``propext #[proof]) }
-- Typed memory: a slot read after a write, a frame's resource types at the
-- runtime frame, and the values it carries as they are.
attribute [vir_denote, vir_denote_norm] VerexIR.Proofs.Denote.Memory.set_same
  VerexIR.Proofs.Denote.ite_some_some VerexIR.Proofs.Denote.isSome_ite
  Bool.ite_eq_true_distrib if_false_left VerexIR.Proofs.Denote.ite_true_or
-- A bitwise operation of a value with itself or with zero.
attribute [vir_denote_norm] VerexIR.Proofs.IntegerArithmetic.bitwiseAnd_self
  VerexIR.Proofs.IntegerArithmetic.bitwiseOr_self Nat.and_self Nat.or_self Nat.xor_self
  VerexIR.Proofs.IntegerArithmetic.bitwiseAnd_zero VerexIR.Proofs.IntegerArithmetic.zero_bitwiseAnd
  VerexIR.Proofs.IntegerArithmetic.bitwiseOr_zero VerexIR.Proofs.IntegerArithmetic.zero_bitwiseOr
  VerexIR.Proofs.IntegerArithmetic.bitwiseXor_zero VerexIR.Proofs.IntegerArithmetic.zero_bitwiseXor
  Nat.and_zero Nat.zero_and Nat.or_zero Nat.zero_or Nat.xor_zero Nat.zero_xor
-- A specification's bitwise operation on nonnegative operands, as the runtime computes it.
attribute [vir_denote_norm] VerexIR.Proofs.IntegerArithmetic.bitwiseOr_nonnegative
  VerexIR.Proofs.IntegerArithmetic.bitwiseXor_nonnegative
-- An empty vector, by its size.
attribute [vir_denote_norm] Array.isEmpty_iff_size_eq_zero
  VerexIR.Proofs.Denote.array_isEmpty_eq_false
-- The data invariant a stored value's resource type selects.
attribute [vir_denote_norm] VerexIR.Proofs.Denote.NTy.handle?_struct
  VerexIR.Proofs.Denote.NTy.handle?_enum VerexIR.Proofs.Denote.NTy.handle?_resolve_struct
  VerexIR.Proofs.Denote.NTy.handle?_resolve_enum
  VerexIR.Proofs.Denote.Memory.set_other VerexIR.Proofs.Denote.Memory.set_other_resource
  VerexIR.Proofs.Denote.Skolems.resource VerexIR.Proofs.Denote.NRow.map
  VerexIR.Proofs.Denote.Skolems.resolve_runtime VerexIR.Proofs.Denote.Skolems.ofRuntime_runtime
  VerexIR.Proofs.Denote.Skolems.toRuntime_runtime VerexIR.Proofs.Denote.ResourceType.mk.injEq
  VerexIR.Proofs.Denote.Skolems.resolve_instantiate VerexIR.Proofs.Denote.Skolems.ofRuntime_instantiate
  VerexIR.Proofs.Denote.Skolems.toRuntime_instantiate
  VerexIR.Proofs.Denote.NTy.struct.injEq VerexIR.Proofs.Denote.NTy.enum.injEq
  VerexIR.Proofs.Denote.NRow.cons.injEq VerexIR.StructHandle.mk.injEq
  VerexIR.Proofs.Denote.NTy.int.injEq VerexIR.Proofs.Denote.NTy.vector.injEq
  VerexIR.StorageKey.address.injEq VerexIR.Proofs.Denote.NTy.storageKey_address
  VerexIR.Proofs.Denote.NTy.storageKey_signer VerexIR.Proofs.Denote.NTy.storageKey_int
  VerexIR.Proofs.Denote.NTy.storageKey_bool VerexIR.Proofs.Denote.NTy.storageKey_string
-- A literal instantiation keys a family as the runtime does.
attribute [vir_denote_norm] VerexIR.Proofs.Denote.instantiatedTypeId_cons

-- The function value naming a target spells its weave as a closure literal does.
attribute [vir_denote_norm] VerexIR.Proofs.Denote.Weave.supplying_nil
  VerexIR.Proofs.Denote.Weave.supplying_cons
  VerexIR.Proofs.Denote.instantiatedTypeId_nil VerexIR.TypeId.mk.injEq
-- A type parameter's value defaults under an induced family to its
-- argument's value.
attribute [vir_denote_norm] VerexIR.Proofs.Denote.Carriers.default_instantiate
  VerexIR.Proofs.Denote.NTy.inhabitant VerexIR.Proofs.Denote.HList.inhabitant
-- The type arguments of an opaque specification function, at an
-- instantiated family the caller's, at the public family themselves.
-- Before `resolve_instantiate`, which a parameter's type also matches.
attribute [vir_denote_norm high] VerexIR.Proofs.Denote.Skolems.type_instantiate
attribute [vir_denote_norm] VerexIR.Proofs.Denote.Skolems.type_runtime VerexIR.Proofs.Denote.NTy.substWith
  VerexIR.Proofs.Denote.NRow.substWith VerexIR.Proofs.Denote.NRows.substWith
-- A resource type at an induced frame is its substitution, evaluated, so
-- that a callee's slot and the caller's are one term.
attribute [vir_denote, vir_denote_norm] VerexIR.Proofs.Denote.NTy.subst
  VerexIR.Proofs.Denote.NRow.subst VerexIR.Proofs.Denote.NRows.subst
  VerexIR.Proofs.Denote.NRow.getD
attribute [vir_denote_norm high] VerexIR.Proofs.Denote.NTy.substWith_type_runtime
-- A callee's value in the caller's view reduces constructor by constructor.
-- A transported entry list of scalar keys and values is the list itself.
attribute [vir_denote_norm] Prod.eta VerexIR.SpecVector.eta List.map_id' List.map_id

-- A vector's mapped element conversion is still unapplied when the
-- normalizer visits it. Its identity must simplify before vector equality
-- can substitute the caller's value into the search facts.
attribute [vir_denote_norm] VerexIR.Proofs.Denote.NTy.ofSkolem_unit_fun
  VerexIR.Proofs.Denote.NTy.ofSkolem_bool_fun VerexIR.Proofs.Denote.NTy.ofSkolem_int_fun
  VerexIR.Proofs.Denote.NTy.ofSkolem_num_fun VerexIR.Proofs.Denote.NTy.ofSkolem_address_fun
  VerexIR.Proofs.Denote.NTy.ofSkolem_signer_fun VerexIR.Proofs.Denote.NTy.ofSkolem_string_fun
  VerexIR.Proofs.Denote.NTy.ofSkolem_bytes_fun VerexIR.Proofs.Denote.NTy.ofSkolem_param_fun

attribute [vir_denote_norm] VerexIR.Proofs.Denote.NTy.ofSkolem_unit
  VerexIR.Proofs.Denote.NTy.ofSkolem_bool VerexIR.Proofs.Denote.NTy.ofSkolem_int
  VerexIR.Proofs.Denote.NTy.ofSkolem_address VerexIR.Proofs.Denote.NTy.ofSkolem_signer
  VerexIR.Proofs.Denote.NTy.ofSkolem_string VerexIR.Proofs.Denote.NTy.ofSkolem_bytes
  VerexIR.Proofs.Denote.NTy.ofSkolem_param VerexIR.Proofs.Denote.NTy.ofSkolem_tuple
  VerexIR.Proofs.Denote.NTy.ofSkolem_struct VerexIR.Proofs.Denote.NTy.ofSkolem_enum
  VerexIR.Proofs.Denote.NTy.ofSkolem_vector VerexIR.Proofs.Denote.NTy.ofSkolem_ref
  VerexIR.Proofs.Denote.NTy.ofSkolem_ghost
  VerexIR.Proofs.Denote.HList.ofSkolem_nil VerexIR.Proofs.Denote.HList.ofSkolem_cons
  VerexIR.Proofs.Denote.variantCarrier.ofSkolem_inl
  VerexIR.Proofs.Denote.variantCarrier.ofSkolem_inr
  VerexIR.Proofs.Denote.ResultShape.ofSkolem_none VerexIR.Proofs.Denote.ResultShape.ofSkolem_one
-- A function calling itself, or a member of a cycle of calls: the calls
-- routed to the cycle are `self`, every other call its callee's meaning.
attribute [vir_denote_norm] VerexIR.Proofs.Denote.routeMeaning_self
  VerexIR.Proofs.Denote.routeMeaning_other VerexIR.Proofs.Denote.routeGeneric_self
  VerexIR.Proofs.Denote.routeGeneric_other VerexIR.FunctionHandle.mk.injEq
  VerexIR.NamespaceId.mk.injEq VerexIR.FunctionId.mk.injEq
/-- The certified locals the context defines (`x.val = e`), with their
definitions. -/
private def namedValues : MetaM (Array (Lean.LocalDecl × Lean.Expr × Lean.Expr)) := do
  (← getLCtx).foldlM (init := #[]) fun found decl => do
    if decl.isImplementationDetail then return found
    let some (_, lhs, rhs) := (← instantiateMVars decl.type).eq? | return found
    unless lhs.isAppOfArity ``VerexIR.SpecInt.val 3 && lhs.appArg!.isFVar do return found
    if rhs.containsFVar lhs.appArg!.fvarId! then return found
    return found.push (decl, lhs.appArg!, rhs)

/-- A term with every named value it reads replaced by its definition, read
the same way: as the clause that cut it states the value. -/
private partial def expandNamed (named : Array (Lean.LocalDecl × Lean.Expr × Lean.Expr))
    (fuel : Nat) (e : Lean.Expr) : Lean.Expr :=
  if fuel == 0 then e else
  e.replace fun sub =>
    if sub.isAppOfArity ``VerexIR.SpecInt.val 3 then
      (named.find? (·.2.1 == sub.appArg!)).map fun (_, _, d) => expandNamed named (fuel - 1) d
    else none

/-- A certified local of `type` the context defines as `value` (`x.val = e`),
directly or through the locals `e` names, with the proof of its definition. -/
private def namedLocal? (type value : Lean.Expr) : MetaM (Option (Lean.Expr × Lean.Expr)) := do
  if value.hasLooseBVars then return none
  let named ← namedValues
  let expandedValue := expandNamed named named.size value
  for (decl, certified, rhs) in named do
    unless ← isDefEq (← inferType certified) type do continue
    if rhs == value then return some (certified, decl.toExpr)
    unless expandNamed named named.size rhs == expandedValue do continue
    -- The definition and the value, each read through the other names'
    -- definitions, are one term.
    let mut theorems : SimpTheorems := {}
    for (other, _, _) in named do
      unless other.fvarId == decl.fvarId do
        theorems ← theorems.add (.fvar other.fvarId) #[] other.toExpr
    let ctx ← Simp.mkContext (simpTheorems := #[theorems])
    let methods := Simp.mkDefaultMethodsCore #[]
    let (fromDefinition, _) ← Simp.main rhs ctx (methods := methods)
    let (fromValue, _) ← Simp.main value ctx (methods := methods)
    unless fromDefinition.expr == fromValue.expr do continue
    return some (certified, ← mkEqTrans (← mkEqTrans decl.toExpr (← fromDefinition.getProof))
      (← mkEqSymm (← fromValue.getProof)))
  return none

/-- A specification integer cut to a width (`SpecInt.ofInt`) of a value a
certified local has by the context: that local. -/
simproc ↓ [vir_denote_norm] cutNamedValue (VerexIR.SpecInt.ofInt _ _) := fun e => do
  let_expr VerexIR.SpecInt.ofInt _ _ default value := e | return .continue
  let some (certified, definition) ← namedLocal? (← inferType e) value | return .continue
  let proof ← mkAppM ``VerexIR.SpecInt.ofInt_eq_of_val #[default, certified, definition]
  return .done { expr := certified, proof? := some proof }

/-- A certified integer a clause defines by a value (`∃ x, x.val = e ∧ p x`)
a certified local has by the context: that local. -/
simproc ↓ [vir_denote_norm] existsNamedValue (∃ _ : VerexIR.SpecInt _ _, _) := fun e => do
  let_expr Exists domain body := e | return .continue
  let .lam name _ (.app (.app (.const ``And _) definition) rest) info := body | return .continue
  let some (_, lhs, value) := definition.eq? | return .continue
  unless lhs.isAppOfArity ``VerexIR.SpecInt.val 3 && lhs.appArg! == .bvar 0 do return .continue
  let some (certified, named) ← namedLocal? domain value | return .continue
  let proof ← mkAppOptM ``VerexIR.Proofs.Denote.SpecInt.exists_val_eq
    #[none, none, none, some (.lam name domain rest info), certified, named]
  return .visit { expr := rest.instantiate1 certified, proof? := some (← mkPropExt proof) }

/-- Whether an expression states a value a clause defines (`bindCuts`): a
certified integer by its value, or the check of a literal function value. -/
private def statesDefinedValue (e : Lean.Expr) : Bool :=
  (e.find? fun sub => (sub.isAppOfArity ``Exists 2 &&
    (sub.getArg! 0).isAppOfArity ``VerexIR.SpecInt 2) ||
      sub.isAppOfArity ``closureTypingCheck 7).isSome

open Lean Meta Simp in
/-- The check of a literal function value a leaf's clause binds, where the
literal is a closure site's: the site's generated check (`siteCheckProof?`). -/
simproc ↓ [vir_denote_norm] existsClosureChecked (@Exists _ _) := fun e => do
  let_expr Exists domain body := e | return .continue
  let some proof ← siteCheckProof? domain | return .continue
  let iff := mkApp3 (mkConst ``exists_prop_of_true) domain body proof
  return .visit { expr := body.beta #[proof], proof? := some (← mkPropExt iff) }

open Lean Meta Simp in
/-- A behavioral predicate of a closure site's constructor capturing values,
as of the function value naming its target at the woven arguments
(`ensuresOf_closureOf_woven`, `abortsOf_closureOf_woven`): the named value
a literal of the target capturing nothing, typed by the site's generated
`named` theorem. -/
private def wovenBehavior (rule : Name) (e : Lean.Expr) : SimpM Step := withDefault do
  unless e.getAppNumArgs > 7 do return .continue
  let function := (e.getArg! 7).consumeMData
  let .const constructor _ := function.getAppFn | return .continue
  let some entry := siteEquationsAt? (← getEnv) constructor | return .continue
  let some namedTyping := entry.named | return .continue
  let some literal ← literalValue? function | return .continue
  unless (literal.captured.consumeMData).isAppOfArity ``NRow.cons 2 do return .continue
  let typing ← unfoldLiterals (← inferType (mkConst namedTyping))
  let typed ← mkExpectedTypeHint (mkConst namedTyping) typing
  unless typing.isAppOfArity ``SiteTyped 10 do return .continue
  let namedArguments := (typing.getAppArgs.extract 0 9).push (mkConst ``NRow.nil)
    |>.push (mkConst ``Unit.unit) |>.push typed
  let named := mkAppN (mkConst ``Skolems.construct) namedArguments
  let naming := mkAppN (mkConst ``Skolems.closure_construct) namedArguments
  -- The reference-freedom premise `free` decided; no other premise is open.
  let premise (name : Name) (condition : Lean.Expr) : MetaM (Option Lean.Expr) := do
    if name == `free then mkDecideProof condition else return none
  let some (rhs, proof) ← rewriteByEquationCore? rule e premise
      #[(`closed, literal.closed), (`named, named), (`naming, naming)]
    | return .continue
  return .visit { expr := rhs, proof? := some proof }

open Lean Meta Simp in
/-- `ensures_of` of a literal function value capturing values, as of the
function value naming its target (`wovenBehavior`). -/
simproc [vir_denote_norm] wovenEnsuresOf
    (@VerexIR.Proofs.EnsuresOf _ ?functions ?frame _ _ _ _ _ _ _ _ _) :=
  wovenBehavior ``VerexIR.Proofs.ensuresOf_closureOf_woven

open Lean Meta Simp in
/-- `aborts_of` of a literal function value capturing values, as of the
function value naming its target (`wovenBehavior`). -/
simproc [vir_denote_norm] wovenAbortsOf
    (@VerexIR.Proofs.AbortsOf _ ?functions ?frame _ _ _ _ _ _ _) :=
  wovenBehavior ``VerexIR.Proofs.abortsOf_closureOf_woven

/-- The values a leaf's clauses define (`bindCuts`): an integer by its value
(`∃ x, x.val = e ∧ p x`) as a certified local of that value where the path
names one (`existsNamedValue`), otherwise as the value cut to the width with
its range a conjunct (`SpecInt.exists_val_cut_unsigned`); a literal function
value's check, a closure site's, by its generated check (`existsClosureChecked`). At the
leaf, after the path has named its locals; the normal form keeps a clause's
as stated. -/
elab "verex_defined_values" : tactic => focusGoal do
  let goal ← getMainGoal
  goal.withContext do
  let mut stating := #[]
  for decl in ← getLCtx do
    if decl.isImplementationDetail then continue
    if statesDefinedValue (← instantiateMVars decl.type) then stating := stating.push decl.fvarId
  let target := statesDefinedValue (← instantiateMVars (← goal.getType))
  if stating.isEmpty && !target then return
  let mut theorems : SimpTheorems := {}
  for name in [``SpecInt.exists_val, ``SpecInt.exists_unbounded, ``SpecInt.exists_val_unsigned,
      ``SpecInt.exists_val_signed, ``SpecInt.exists_val_unbounded, ``SpecInt.exists_val_cut_unsigned,
      ``SpecInt.exists_val_cut_signed] do
    theorems ← theorems.addConst name
  let simprocs ← ({} : Simprocs).add ``existsNamedValue (post := false)
  let simprocs ← simprocs.add ``existsClosureChecked (post := false)
  let ctx ← Simp.mkContext (config := { failIfUnchanged := false }) (simpTheorems := #[theorems])
    (congrTheorems := ← getSimpCongrTheorems)
  match ← simpGoal goal ctx #[simprocs] (simplifyTarget := target) (fvarIdsToSimp := stating) with
  | (none, _) => replaceMainGoal []
  | (some (_, next), _) => replaceMainGoal [next]

-- A certified integer cut to its own width is itself; a cut compared reads
-- by its value.
attribute [vir_denote_norm] VerexIR.SpecInt.ofInt_val VerexIR.SpecInt.ofInt_eq_iff
  VerexIR.SpecInt.eq_ofInt_iff
-- A vector read at a wider element type (`coerceNative`): its values mapped,
-- a literal's element by element.
attribute [vir_denote_norm] VerexIR.SpecVector.map List.map_toArray List.map_cons
  List.map_nil
-- The value of a bitwise operation at its operator.
attribute [vir_denote_norm] VerexIR.Proofs.Denote.BitOp.eval_and VerexIR.Proofs.Denote.BitOp.eval_or
  VerexIR.Proofs.Denote.BitOp.eval_xor
-- The structural order, as the ordering it computes.
attribute [vir_denote_norm] VerexIR.Proofs.Denote.NTy.compareResult_val VerexIR.orderValue_lt_zero
  VerexIR.orderValue_eq_zero VerexIR.zero_lt_orderValue VerexIR.orderValue_le_zero
  VerexIR.zero_le_orderValue VerexIR.orderValue_eq_neg_one VerexIR.orderValue_eq_one
  VerexIR.Proofs.Denote.NTy.order VerexIR.Proofs.Denote.HList.order
  VerexIR.Proofs.Denote.variantCarrier.order Int.compare_eq_lt Int.compare_eq_gt
  Int.compare_eq_eq List.toList_toArray VerexIR.SpecMap.intOrder_compare
-- A vector construction whose representability is decided, and the sizes
-- of in-bounds insertion and removal. Whether an insertion or removal is
-- in bounds is an arithmetic side condition, decided at a leaf by omega
-- (`verex_lookups_after_writes`), not by the normalization, whose
-- default discharger would run it over the whole set.
attribute [vir_denote_norm] Option.dite_none_right_eq_some
  Array.size_insertIdx Array.size_eraseIdx
theorem self_eq_extract_iff {α : Type} {as : Array α} {i j : Nat} :
    as = as.extract i j ↔ as.size = 0 ∨ i = 0 ∧ as.size ≤ j := by
  rw [eq_comm]; exact Array.extract_eq_self_iff

-- A slice, read by its size and elements.
attribute [vir_denote_norm] Array.extract_size Array.size_extract Array.getElem?_extract
  Array.extract_eq_self_iff self_eq_extract_iff Array.extract_size_left
-- An operation on a mapped array, as the mapped result.
attribute [vir_denote_norm] VerexIR.Proofs.Denote.setIfInBounds_map
  VerexIR.Proofs.Denote.extract_map VerexIR.Proofs.Denote.push_map
  VerexIR.Proofs.Denote.append_map
-- A range reversal, read by its size and elements.
attribute [vir_denote_norm] VerexIR.Proofs.Denote.size_reverseSlice
  VerexIR.Proofs.Denote.getElem?_reverseSlice VerexIR.Proofs.Denote.reverseSlice_reverseSlice
attribute [vir_denote_norm] VerexIR.Proofs.Denote.findIndex?_eq_none_iff
  VerexIR.Proofs.Denote.findIndex?_isSome_iff VerexIR.Proofs.Denote.forall_imp_ne_iff
  VerexIR.Proofs.Denote.forall_imp_ne_iff' VerexIR.Proofs.Denote.exists_some_val_eq_iff
  VerexIR.Proofs.Denote.forall_some_val_ne_iff VerexIR.Proofs.Denote.findIndex?_zero
  VerexIR.Proofs.Denote.findIndex?_succ VerexIR.Proofs.Denote.NTy.eqb_int
  VerexIR.Proofs.Denote.NTy.eqb_bool VerexIR.Proofs.Denote.NTy.eqb_address
  VerexIR.Proofs.Denote.NTy.eqb_unit VerexIR.Proofs.Denote.NTy.eqb_param
-- A wrapping shift whose value cannot reach the modulus, and the truncating
-- remainder of a nonnegative shift, the runtime's.
attribute [vir_denote_norm] VerexIR.Proofs.Denote.shiftLeft_emod_of_fits
  VerexIR.Proofs.Denote.shiftLeft_tmod_of_fits VerexIR.Proofs.Denote.shiftLeft_tmod_of_nonneg
/-- A proof of `0 ≤ e` through sums, products and quotients of terms the
normalization's discharger shows nonnegative. -/
private partial def nonnegative? (e : Lean.Expr) : Simp.SimpM (Option Lean.Expr) := do
  let parts (lemma : Name) (left right : Lean.Expr) : Simp.SimpM (Option Lean.Expr) := do
    let some first ← nonnegative? left | return none
    let some second ← nonnegative? right | return none
    return some (mkApp4 (mkConst lemma) left right first second)
  let whole : Simp.SimpM (Option Lean.Expr) := do
    (← Simp.getMethods).discharge? (mkIntLE (mkIntLit 0) e)
  let structural ← match_expr e with
    | HAdd.hAdd _ _ _ _ left right => parts ``Int.add_nonneg left right
    | HMul.hMul _ _ _ _ left right => parts ``Int.mul_nonneg left right
    | HDiv.hDiv _ _ _ _ left right => parts ``Int.ediv_nonneg left right
    | Nat.cast _ _ n => return some (mkApp (mkConst ``Int.natCast_nonneg) n)
    -- A certified unsigned value by its bounds, not by a search.
    | VerexIR.SpecInt.val width signed value =>
        if signed.isConstOf ``Bool.false && width.isAppOfArity ``VerexIR.IntWidth.bits 1 then
          return some (← mkAppM ``And.left #[← mkAppM ``VerexIR.SpecInt.unsigned_bounds #[value]])
        else whole
    | Int.shiftLeft value distance => do
        let some first ← nonnegative? value | whole
        return some (mkApp3 (mkConst ``VerexIR.Proofs.Denote.shiftLeft_nonneg) value distance first)
    | _ => whole
  if structural.isSome || !(e.isAppOfArity ``HAdd.hAdd 6 || e.isAppOfArity ``HMul.hMul 6 ||
      e.isAppOfArity ``HDiv.hDiv 6) then
    return structural
  -- A sum, product or quotient whose sign a fact states as a whole.
  whole

/-- One spelling of a quotient (`doc/plan/proof-language.md`, "One spelling of
a quotient"): a truncating quotient or remainder of a nonnegative dividend is
the Euclidean one, `/` and `%` of unbounded integers; `Int.tdiv` and
`Int.tmod` stay only where the dividend's sign is open. -/
simproc [vir_denote_norm] tdivOfNonnegative (Int.tdiv _ _) := fun e => do
  let_expr Int.tdiv dividend divisor := e | return .continue
  let some nonnegative ← nonnegative? dividend | return .continue
  let quotient := mkIntDiv dividend divisor
  let proof := mkApp3 (mkConst ``Int.tdiv_eq_ediv_of_nonneg) dividend divisor nonnegative
  return .visit { expr := quotient, proof? := some proof }

simproc [vir_denote_norm] tmodOfNonnegative (Int.tmod _ _) := fun e => do
  let_expr Int.tmod dividend divisor := e | return .continue
  let some nonnegative ← nonnegative? dividend | return .continue
  let remainder := mkIntMod dividend divisor
  let proof := mkApp3 (mkConst ``Int.tmod_eq_emod_of_nonneg) dividend divisor nonnegative
  return .visit { expr := remainder, proof? := some proof }

-- The identities of zero and one, which a substitution or a case split
-- leaves (`r - 0`, `r * 0`): the arithmetic in its simplest spelling.
attribute [vir_denote_norm] Int.add_zero Int.zero_add Int.sub_zero Int.mul_zero Int.zero_mul
  Int.mul_one Int.one_mul Int.zero_ediv Int.ediv_zero Int.ediv_one Int.zero_emod Int.emod_one

/-! Membership (`doc/plan/proof-language.md`, N2): the leaf's deciders read
`e ∈ v` by its positions (`mem_iff_exists_int_index`); the step normal form
keeps it an atom with its own algebra, and reads a search for an element as
that atom (`VerexSL.Verify`'s step simp context). -/

@[vir_denote_norm] theorem mem_swapIfInBounds {α : Type} (xs : Array α) (i j : Nat) (a : α) :
    a ∈ xs.swapIfInBounds i j ↔ a ∈ xs := by
  unfold Array.swapIfInBounds
  split
  · split
    · exact (Array.swap_perm ‹_› ‹_›).mem_iff
    · rfl
  · rfl

/-- A statement or its negation holds, whatever spelling it has. -/
@[vir_denote_norm] theorem or_not_self_iff (p : Prop) : (p ∨ ¬p) ↔ True :=
  iff_true_intro (Classical.em p)

@[vir_denote_norm] theorem not_or_self_iff (p : Prop) : (¬p ∨ p) ↔ True :=
  iff_true_intro ((Classical.em p).symm)

/-- An integer read at its own width is itself: the identity coercion of a
vector's elements, and of a needle, which membership reads. -/
@[vir_denote_norm] theorem SpecInt.mk_val_eq {width : IntWidth} {signed : Bool}
    (x : SpecInt width signed) (fits : IntegerValueFits width signed x.val) :
    (⟨x.val, fits⟩ : SpecInt width signed) = x := rfl

@[vir_denote_norm] theorem Array.map_mk_val {width : IntWidth} {signed : Bool}
    (xs : Array (SpecInt width signed)) (fits : ∀ x : SpecInt width signed,
      IntegerValueFits width signed x.val) :
    xs.map (fun x => (⟨x.val, fits x⟩ : SpecInt width signed)) = xs := by
  have : (fun x => (⟨x.val, fits x⟩ : SpecInt width signed)) = id := rfl
  rw [this, Array.map_id]

/-- A search for an element is membership, by its value for an integer, or
over a vector's positions whatever its reads fall back to: the spellings the
step normal form reads as the atom (`doc/plan/proof-language.md`, N2). -/
theorem exists_mem_val_eq_iff {width : IntWidth} {signed : Bool}
    (xs : Array (SpecInt width signed)) (e : SpecInt width signed) :
    (∃ x, x ∈ xs ∧ x.val = e.val) ↔ e ∈ xs :=
  ⟨fun ⟨_, mem, same⟩ => SpecInt.ext same ▸ mem, fun mem => ⟨e, mem, rfl⟩⟩

theorem forall_mem_val_ne_iff {width : IntWidth} {signed : Bool}
    (xs : Array (SpecInt width signed)) (e : SpecInt width signed) :
    (∀ x, x ∈ xs → ¬x.val = e.val) ↔ ¬e ∈ xs :=
  ⟨fun all mem => all e mem rfl, fun absent _ mem same => absent (SpecInt.ext same ▸ mem)⟩

theorem exists_position_some_iff {α : Type} (xs : Array α) (e : α) :
    (∃ i : Int, 0 ≤ i ∧ i < xs.size ∧ xs[i.toNat]? = some e) ↔ e ∈ xs :=
  (mem_iff_exists_int_index xs e).symm

theorem forall_position_some_ne_iff {α : Type} (xs : Array α) (e : α) :
    (∀ i : Int, 0 ≤ i → i < xs.size → ¬xs[i.toNat]? = some e) ↔ ¬e ∈ xs := by
  rw [mem_iff_exists_int_index]
  exact ⟨fun all ⟨i, low, high, read⟩ => all i low high read,
    fun absent i low high read => absent ⟨i, low, high, read⟩⟩

theorem exists_position_eq_iff {α : Type} (xs : Array α) (d e : α) :
    (∃ i : Int, 0 ≤ i ∧ i < xs.size ∧ xs[i.toNat]?.getD d = e) ↔ e ∈ xs := by
  rw [mem_iff_exists_int_index]
  refine exists_congr fun i => ⟨fun ⟨low, high, read⟩ => ⟨low, high, ?_⟩,
    fun ⟨low, high, found⟩ => ⟨low, high, by rw [found, Option.getD_some]⟩⟩
  rw [Array.getElem?_eq_getElem (by omega : i.toNat < xs.size), Option.getD_some] at read
  rw [Array.getElem?_eq_getElem (by omega : i.toNat < xs.size), read]

theorem forall_position_ne_iff {α : Type} (xs : Array α) (d e : α) :
    (∀ i : Int, 0 ≤ i → i < xs.size → ¬xs[i.toNat]?.getD d = e) ↔ ¬e ∈ xs := by
  rw [← exists_position_eq_iff xs d e]
  exact ⟨fun all ⟨i, low, high, read⟩ => all i low high read,
    fun absent i low high read => absent ⟨i, low, high, read⟩⟩

attribute [vir_denote_norm] Array.mem_push Array.mem_append Array.mem_singleton List.mem_toArray
  Array.mem_map

-- A resolved prophecy brings an operation's result into a leaf.
attribute [vir_denote_norm] VerexIR.Proofs.Denote.wrapInt_unsigned
  VerexIR.Proofs.Denote.wrapInt_signed VerexIR.Proofs.Denote.ModularOp.run_val
  VerexIR.Proofs.Denote.BitOp.run_val
attribute [vir_denote_norm] VerexIR.Proofs.wp_pure
  VerexIR.Proofs.wp_abort VerexIR.Proofs.wp_given VerexIR.Proofs.Spec.pure_bind
  VerexIR.Proofs.Spec.abort_bind VerexIR.Proofs.Denote.wp_abortAt VerexIR.Proofs.Denote.wp_opAt
  VerexIR.Proofs.Denote.wp_opAfterAt VerexIR.Proofs.Denote.AfterAt_true
  VerexIR.Proofs.Denote.opAt_pure VerexIR.Proofs.Denote.opAt_abort
  VerexIR.Proofs.Denote.wp_branchAt VerexIR.Proofs.Denote.branchIn_some
  VerexIR.Proofs.Denote.branchIn_none VerexIR.Proofs.Denote.AbortsAt_true VerexIR.Proofs.Denote.Taken_true
  VerexIR.Proofs.Denote.wp_armAt VerexIR.Proofs.Denote.armIn_some VerexIR.Proofs.Denote.armIn_none
  VerexIR.Proofs.Denote.wp_dispatchAt VerexIR.Proofs.Denote.Dispatched_true
  VerexIR.Proofs.Denote.Matched_true
  VerexIR.Proofs.Denote.abortAt_bind VerexIR.Proofs.Denote.Flow.bind_abortAt
  VerexIR.Proofs.Denote.ResultShape.bodyType Bool.not_eq_true decide_eq_true_eq Bool.and_eq_true
  Bool.or_eq_true and_assoc exists_and_left exists_and_right exists_eq_left exists_eq_left'
  exists_true_left
  and_true true_and and_imp forall_and forall_eq forall_eq' Prod.mk.injEq exists_eq exists_eq'
  and_false false_and not_false_eq_true not_true_eq_false true_implies false_implies implies_true
  true_or or_true false_or or_false
  imp_self eq_self_iff_true ite_true ite_false Bool.false_eq_true Bool.true_eq_false
  Bool.not_eq_false decide_eq_false_iff_not Bool.and_eq_false_imp Bool.not_true Bool.not_false
  Bool.not_not Bool.not_eq_false' Bool.not_eq_true' Option.some.injEq Option.elim_some Option.elim_none
  List.getElem?_toArray List.getElem?_cons_zero
  List.getElem?_cons_succ Option.getD_some
  List.cons.injEq List.nil_eq
  beq_self_eq_true VerexIR.Proofs.Denote.Array.push_eq_push_iff
  Nat.lt_succ_self Nat.lt_add_one Nat.le_refl
  Option.map_some Option.map_none
  VerexIR.Proofs.Denote.nat_eq_add_succ_iff
  VerexIR.Proofs.Denote.nat_add_succ_eq_iff VerexIR.Proofs.Denote.nat_eq_succ_iff
  VerexIR.Proofs.Denote.nat_succ_eq_iff
  Option.bind_some Option.bind_none
  Option.getD_none
  VerexIR.SemanticOperations.instantiatedTypeId_empty
  Nat.add_assoc VerexIR.Proofs.Denote.nat_self_eq_add_iff
  VerexIR.Proofs.Denote.nat_add_eq_self_iff
  VerexIR.SpecVector.ofArray?_eq VerexIR.SpecVector.values_set VerexIR.SpecVector.values_mk
  Array.toList_map
  VerexIR.Proofs.Denote.exists_mem_map_iff Array.mem_toList_iff
  exists_eq_right exists_eq_right' VerexIR.Proofs.Denote.exists_range_eq_iff
  VerexIR.Proofs.Denote.mem_iff_exists_int_index
  VerexIR.Proofs.Denote.not_exists_range_iff VerexIR.Proofs.Denote.not_forall_range_iff
  VerexIR.Proofs.Denote.forall_exists_range_iff
  Option.isSome_eq_false_iff Option.isNone_iff_eq_none true_iff false_iff iff_true iff_false
  VerexIR.Proofs.Denote.not_eq_none_iff_isSome
  Int.toNat_natCast Array.toArray_toList List.toList_toArray Array.length_toList Array.size_set!
  Array.size_push Array.size_map VerexIR.Proofs.Denote.toArray_inj_iff
  VerexIR.SpecVector.values_push VerexIR.SpecVector.values_append Array.append_singleton
  -- A field update a clause states (`nativeFieldUpdate`).
  VerexIR.Proofs.Denote.Var.update_here VerexIR.Proofs.Denote.Var.update_there
  List.map_toArray List.map_cons List.map_nil List.push_toArray
  List.size_toArray List.length_cons List.length_nil List.getElem?_eq_some_iff
  Array.getElem?_eq_none_iff Array.size_setIfInBounds Array.getElem?_map
  -- The transport of a value at the identity instantiation.
  Option.map_id' Array.map_id' Int.sub_add_cancel
  Option.map_map Function.comp_def Array.set!_eq_setIfInBounds List.setIfInBounds_toArray
  List.set_cons_zero List.set_cons_succ List.insertIdxIfInBounds_toArray List.insertIdx_zero
  List.insertIdx_succ_cons List.eraseIdxIfInBounds_toArray List.eraseIdx_cons_zero
  List.eraseIdx_cons_succ List.toList_toArray Option.bind_eq_some_iff Option.map_eq_some_iff
  -- A literal vector updated at a position not known literally, and a
  -- literal vector transported elementwise.
  List.length_insertIdx List.getElem?_insertIdx List.length_eraseIdx List.getElem?_eraseIdx
  List.length_map List.getElem?_map List.getElem_cons_zero List.getElem_cons_succ
  List.getElem_toArray
  List.getElem?_nil List.getElem?_cons_zero List.getElem?_cons_succ
  Int.toNat_natCast VerexIR.Proofs.Denote.vectorLength_val
  VerexIR.Proofs.Denote.NTy.eqb_vector_decide
  VerexIR.Proofs.Denote.NTy.eqb_table_decide
  VerexIR.Proofs.Denote.NTy.refFree VerexIR.Proofs.Denote.NRow.refFree
  VerexIR.Proofs.Denote.NRows.refFree VerexIR.Proofs.Denote.NTy.ghostFree
  VerexIR.Proofs.Denote.NRow.ghostFree VerexIR.Proofs.Denote.NRows.ghostFree Option.isSome_some Option.isSome_none Option.isSome_map
  VerexIR.GlobalKey.mk.injEq VerexIR.StorageKey.address.injEq Option.map_eq_none_iff
  VerexIR.StructHandle.mk.injEq VerexIR.Proofs.Contract.typed
  List.toArray_eq_iff exists_prop exists_const Int.not_lt Int.not_le ne_eq Decidable.not_not
  Bool.true_or Bool.false_or Bool.or_true Bool.or_false

private partial def isLiteralList (e : Lean.Expr) : Bool :=
  let e := e.consumeMData
  if e.isAppOfArity ``List.nil 1 then true
  else if e.isAppOfArity ``List.cons 3 then isLiteralList (e.getArg! 2)
  else false

/-- A read from a constant list needs no enumeration of the index. -/
theorem getElem_replicated {α : Type u} (value : α) (n i : Nat) (h : i < n) :
    (List.replicate n value)[i]'(by simpa only [List.length_replicate] using h) = value := by
  apply List.getElem_replicate

/-- Literal vectors often contain equal values while their index comes from
a callee. Recognize the constant list and use its lookup lemma, without case
search over the index. Definitional equality also ignores range certificates
on otherwise equal bounded values. -/
simproc [vir_denote_norm] uniformListRead ((_ : List _)[_]'_) := fun e => do
  unless e.isAppOfArity ``GetElem.getElem 8 do return .continue
  let some (type, elements) := (e.getArg! 5).listLit? | return .continue
  let some value := elements.head? | return .continue
  if elements.any (·.hasExprMVar) then return .continue
  -- A skolem-family conversion may expose literal elements before the
  -- list's type is reduced. Leave those reads to the ordinary rewrites.
  unless ← withReducible (isDefEq type (← inferType value)) do return .continue
  unless ← elements.allM fun element => withReducible (isDefEq value element) do
    return .continue
  -- Keep the bound's existing type. The application builder does not
  -- unfold List.length when checking a bound against a numeric literal.
  let proof ← mkAppOptM ``getElem_replicated
    #[type, value, ← mkAppM ``List.length #[e.getArg! 5], e.getArg! 6, e.getArg! 7]
  return .visit { expr := value, proof? := some proof }

/-- Read a swapped literal vector before invoking the leaf solver. Expanding
reads of symbolic arrays introduces conditionals which obstruct their authored
array proofs, so those arrays retain the swap operation. The rewrite itself
uses the standard, kernel-checked array lookup theorem. -/
simproc [vir_denote_norm] literalSwapRead (((_ : Array _).swapIfInBounds _ _)[_]'_) := fun e => do
  unless e.isAppOfArity ``GetElem.getElem 8 do return .continue
  let swapped := e.getArg! 5
  unless swapped.isAppOfArity ``Array.swapIfInBounds 4 do return .continue
  let xs := (swapped.getArg! 1).consumeMData
  unless (xs.isAppOfArity ``List.toArray 2 || xs.isAppOfArity ``Array.mk 2) &&
      isLiteralList (xs.getArg! 1) do return .continue
  let rules ← mkSimpTheoremFromConst ``Array.getElem_swapIfInBounds
  if let some result ← Simp.tryTheorem? e rules[0]! then return .visit result
  return .continue


-- Variant names are literals, decided where they are compared.
attribute [vir_denote_norm] String.reduceBEq String.reduceEq String.reduceBNe String.reduceNe
-- A value transported into a callee's view and back is the value, also
-- element by element.
attribute [vir_denote_norm] VerexIR.Proofs.Denote.NTy.ofSkolem_toSkolem
  VerexIR.Proofs.Denote.NTy.toSkolem_ofSkolem VerexIR.Proofs.Denote.HList.ofSkolem_toSkolem
  VerexIR.Proofs.Denote.HList.toSkolem_ofSkolem
  VerexIR.Proofs.Denote.variantCarrier.ofSkolem_toSkolem
  VerexIR.Proofs.Denote.variantCarrier.toSkolem_ofSkolem Array.map_map List.map_map
-- The order of booleans is their comparison, computed on literals.
@[vir_denote_norm] theorem compare_false_false : compare false false = Ordering.eq := rfl
@[vir_denote_norm] theorem compare_false_true : compare false true = Ordering.lt := rfl
@[vir_denote_norm] theorem compare_true_false : compare true false = Ordering.gt := rfl
@[vir_denote_norm] theorem compare_true_true : compare true true = Ordering.eq := rfl
-- A position a pinned integer gives is a literal, and so are the
-- comparisons of the lengths it computes; literal keys compare equal or
-- not, as a map's precondition over literal vectors states them.
attribute [vir_denote_norm] Int.reduceToNat Nat.reduceLeDiff Nat.reduceLT Int.reduceLT
  Int.reduceLE Int.reduceEq Int.reduceNe

/-- Fold literal shifts before collecting arithmetic bounds. Leaving `1 << k`
symbolic makes each use of that constant acquire separate quotient/remainder
facts. As with literal powers, respect Lean's evaluation threshold. Symbolic
operands keep their bit-vector representation and existing arithmetic lemmas. -/
private def shiftLiteralStep (operation : Int → Nat → Int) : Simp.DSimproc := fun e => do
  let args := e.getAppArgs
  let (some value, some distance) := (args[0]?.bind Lean.Expr.int?, args[1]?.bind Lean.Expr.nat?)
    | return .continue
  unless ← checkExponent distance (warning := false) do return .continue
  return .done (mkIntLit (operation value distance))

dsimproc_decl reduceIntShiftLeft (Int.shiftLeft _ _) := shiftLiteralStep Int.shiftLeft
dsimproc_decl reduceIntShiftRight (Int.shiftRight _ _) := shiftLiteralStep Int.shiftRight
attribute [vir_denote, vir_denote_norm] reduceIntShiftLeft reduceIntShiftRight

/-- A bitwise operation on integer literals, evaluated: a value a program
computes from literals, as `3 & 7`, is a literal to the observations of
what holds it. The kernel decides the equation by evaluation. -/
private def bitwiseLiteralStep (operation : Int → Int → Int) : Lean.Meta.Simp.Simproc := fun e => do
  let args := e.getAppArgs
  let (some left, some right) := (args[0]?.bind Lean.Expr.int?, args[1]?.bind Lean.Expr.int?)
    | return .continue
  let result := mkIntLit (operation left right)
  let proof ← mkExpectedTypeHint (← mkEqRefl e) (← mkEq e result)
  return .done { expr := result, proof? := some proof }

simproc_decl reduceBitwiseAnd (VerexIR.Proofs.IntegerArithmetic.bitwiseAnd _ _) :=
  bitwiseLiteralStep VerexIR.Proofs.IntegerArithmetic.bitwiseAnd
simproc_decl reduceBitwiseOr (VerexIR.Proofs.IntegerArithmetic.bitwiseOr _ _) :=
  bitwiseLiteralStep VerexIR.Proofs.IntegerArithmetic.bitwiseOr
simproc_decl reduceBitwiseXor (VerexIR.Proofs.IntegerArithmetic.bitwiseXor _ _) :=
  bitwiseLiteralStep VerexIR.Proofs.IntegerArithmetic.bitwiseXor
/-- A lifted bitwise operation (`BitOp.eval`) on integer literals, evaluated
as its operation. -/
simproc_decl reduceBitOpEval (VerexIR.Proofs.Denote.BitOp.eval _ _ _) := fun e => do
  let args := e.getAppArgs
  let some op := args[0]? | return .continue
  let operation : Option VerexIR.Proofs.Denote.BitOp :=
    if op.isConstOf ``VerexIR.Proofs.Denote.BitOp.and then some .and
    else if op.isConstOf ``VerexIR.Proofs.Denote.BitOp.or then some .or
    else if op.isConstOf ``VerexIR.Proofs.Denote.BitOp.xor then some .xor
    else none
  let some operation := operation | return .continue
  let (some left, some right) := (args[1]?.bind Lean.Expr.int?, args[2]?.bind Lean.Expr.int?)
    | return .continue
  let result := mkIntLit (operation.eval left right)
  let proof ← mkExpectedTypeHint (← mkEqRefl e) (← mkEq e result)
  return .done { expr := result, proof? := some proof }
attribute [vir_denote_norm, vir_denote_eval] reduceBitwiseAnd reduceBitwiseOr reduceBitwiseXor
  reduceBitOpEval
-- Concatenated vector literals, as a specification builds a key with
-- `concat(vec(1), vec(2))`, are the literal of the concatenation.
attribute [vir_denote_norm] List.append_toArray List.cons_append List.nil_append

/-- The order of two integer literals, as the structural order of literal
values compares their components. -/
simproc [vir_denote_eval] reduceIntCompare (@compare Int _ _ _) := fun e => do
  let_expr compare _ _ left right := e | return .continue
  let (some a, some b) := (left.int?, right.int?) | return .continue
  let result := match Ord.compare a b with
    | .lt => mkConst ``Ordering.lt
    | .eq => mkConst ``Ordering.eq
    | .gt => mkConst ``Ordering.gt
  let proof ← mkExpectedTypeHint (← mkEqRefl e) (← mkEq e result)
  return .done { expr := result, proof? := some proof }

/-- A literal certified at a width it fits, as a specification cuts an integer
into a fixed-width position (`SpecInt.ofInt`). -/
simproc [vir_denote_eval] reduceSpecIntOfInt (VerexIR.SpecInt.ofInt _ _) := fun e => do
  let_expr VerexIR.SpecInt.ofInt width signed default value := e | return .continue
  let some literal := value.int? | return .continue
  let some widthValue := (match_expr width.consumeMData with
      | VerexIR.IntWidth.bits bits => (bits.nat? <|> bits.rawNatLit?).map VerexIR.IntWidth.bits
      | VerexIR.IntWidth.unbounded => some VerexIR.IntWidth.unbounded
      | _ => none) | return .continue
  let some signedValue := (if signed.isConstOf ``Bool.true then some true
      else if signed.isConstOf ``Bool.false then some false else none) | return .continue
  unless VerexIR.Ty.integerValueFits? (.integer widthValue signedValue) literal == some true do
    return .continue
  let fits ← VerexIR.Proofs.Denote.decidedLemma
    (← mkAppM ``VerexIR.IntegerValueFits #[width, signed, value])
  let certified := mkAppN (mkConst ``VerexIR.SpecInt.mk) #[width, signed, value, fits]
  let proof ← mkAppOptM ``VerexIR.SpecInt.ofInt_eq_of_val
    #[width, signed, default, certified, value, ← mkEqRefl value]
  return .done { expr := certified, proof? := some proof }

-- The neighbours of a key in a map literal, entry by entry, and its keys
-- and values.
attribute [vir_denote_eval] VerexIR.SpecMap.nextKey?_nil VerexIR.SpecMap.nextKey?_cons
  VerexIR.SpecMap.prevKey?_nil VerexIR.SpecMap.prevKey?_cons Option.some_or Option.none_or
  VerexIR.SpecMap.keys_nil VerexIR.SpecMap.keys_cons VerexIR.SpecMap.values_nil
  VerexIR.SpecMap.values_cons

/-- The unit value spelled `()` (`Unit.unit`), as contracts build it, where
reduction leaves its constructor (`PUnit.unit`), so that a value read in two
places reaches the deciders as one term. -/
dsimproc ↓ [vir_denote_norm, vir_transport] unitSpelling (Unit.unit) := fun e => do
  unless e.isConstOf ``PUnit.unit && e.constLevels! == [levelOne] do return .continue
  return .done (mkConst ``Unit.unit)

/-- A map's order of a constructed type's typed keys, as a contract at the
type spells it: integers by `SpecMap.intOrder`, any other by the structural
order (`NTy.keyOrder`); a type parameter's stays its family's. The order is
reducible, so a simp key reduces it away: the orders are respelled where a
clause states them, in a discipline and in a map's neighbours. -/
def reducedOrderArguments : Simp.DSimproc := fun e => do
  let arguments := e.getAppArgs
  let mut changed := false
  let mut respelled := arguments
  for i in [:arguments.size] do
    let argument := arguments[i]!
    let_expr NTy.readOrder _ _ τ := argument | continue
    let .const name _ := τ.consumeMData.getAppFn | continue
    unless (← getEnv).isConstructor name && name != ``NTy.param do continue
    let reduced ← whnfR argument
    unless reduced.isConstOf ``VerexIR.SpecMap.intOrder || reduced.isAppOf ``NTy.keyOrder do continue
    respelled := respelled.set! i reduced
    changed := true
  unless changed do return .continue
  return .done (mkAppN e.getAppFn respelled)

dsimproc ↓ [vir_denote_norm, vir_transport] orderedReducedOrder
    (VerexIR.SpecMap.Discipline.ordered _) := reducedOrderArguments
dsimproc ↓ [vir_denote_norm, vir_transport] nextKeyReducedOrder
    (VerexIR.SpecMap.nextKey? _ _ _) := reducedOrderArguments
dsimproc ↓ [vir_denote_norm, vir_transport] prevKeyReducedOrder
    (VerexIR.SpecMap.prevKey? _ _ _) := reducedOrderArguments

/-- Integer keys compare as integers, at every rank. -/
@[vir_denote_eval] theorem NTy.readOrder_compare_int [Carriers] (rank : VerexIR.ValueRanks)
    (width : Nat) (signed : Bool) (left right : Int) :
    (NTy.readOrder rank (.int width signed)).compare left right = compare left right := rfl

-- The integer order itself, as the order of a type argument's integer keys
-- reads at the caller.
attribute [vir_denote_eval] VerexIR.SpecMap.intOrder_compare

-- The lexicographic order of literal lists, as the structural order of
-- literal vectors unfolds to.
attribute [vir_denote_eval] List.compareLex_nil_nil List.compareLex_nil_cons
  List.compareLex_cons_nil List.compareLex_cons_cons Ordering.lt_then Ordering.gt_then
  Ordering.eq_then

/-- A comparison of two orderings is decided: what the navigation of a
literal map (`nextKey?`, `prevKey?`) unfolds to at its entries. -/
theorem Ordering.beq_eq_decide (a b : Ordering) : (a == b) = decide (a = b) := rfl

attribute [vir_denote_eval] Ordering.beq_eq_decide decide_true decide_false

attribute [vir_denote_eval] reduceCtorEq Nat.reduceAdd Nat.reduceSub Nat.reduceDiv Nat.reducePow
  Nat.reduceLT Nat.reduceEqDiff Int.reduceAdd Int.reduceSub Int.reducePow Int.reduceLT Int.reduceLE
  Int.reduceEq Int.reduceNe
  Int.reduceNatCast' Int.reduceToNat String.reduceBEq String.reduceEq String.reduceBNe
  String.reduceNe dite_true dite_false

-- The transport normal form (`vir_transport`): values, operations and
-- contracts at a call's induced frame read at the caller's.
attribute [vir_transport] instantiatedEqInCallerView mapTransportIdentity optionTransportIdentity
  paramFreeValueTransport paramFreeVariantTransport paramFreeRowTransport variantNameTransport
  variantNameOuterFamily equalOuterFamily variantToSkolemConstructor hlistToSkolemConstructor
  ofSkolemIdentity
  toSkolemIdentity hlistOfSkolemIdentity hlistToSkolemIdentity variantOfSkolemIdentity
  variantToSkolemIdentity inhabitantOuterFamily NativeMaps.NTy.read_param_instantiate
  NativeMaps.NTy.unread_param_instantiate
  NativeMaps.ofEntriesTransport
  Carriers.orders_instantiate Carriers.erasures_instantiate NTy.ofSkolem_toSkolem
  NTy.toSkolem_unit NTy.toSkolem_bool NTy.toSkolem_int NTy.toSkolem_num NTy.toSkolem_param
  NTy.toSkolem_tuple NTy.toSkolem_struct NTy.toSkolem_enum NTy.toSkolem_ref NTy.toSkolem_ghost
  HList.toSkolem_nil HList.toSkolem_fst HList.toSkolem_snd HList.toSkolem_mk
  variantCarrier.toSkolem_inl variantCarrier.toSkolem_inr Skolems.resolve_instantiate
  Skolems.ofRuntime_instantiate Skolems.toRuntime_instantiate ResultShape.toSkolem
/-- A quantifier over certified integers, spelled as a clause quantifies over
an integer type (`Clauses`, a type's domain): over the integers the type
admits, its native value certified by the membership
(`SpecInt.forall_mk`). A specialized contract quantifies so at a type
argument's integers as a direct contract does. -/
simproc ↓ [vir_transport] forallCertified (∀ _ : _, _) := fun e => do
  let .forallE name domain body _ := e | return .continue
  if ← isProp domain then return .continue
  unless ← isProp e do return .continue
  -- The domain in the caller's spelling, as a frame a contract is
  -- specialized at induces it; the key cannot read it so.
  let domain' ← whnfD (← FamilyTransport.respellInducedFamily domain)
  let_expr VerexIR.SpecInt width signed := domain' | return .continue
  let Q := Lean.mkLambda name .default domain' body
  let unbounded := (← whnfR width).isConstOf ``VerexIR.IntWidth.unbounded
  let proof ← if unbounded then
      mkAppM ``VerexIR.Proofs.Denote.SpecInt.forall_mk_unbounded #[Q]
    else mkAppOptM ``VerexIR.Proofs.Denote.SpecInt.forall_mk #[width, signed, Q]
  let some (_, rhs) := (← whnfR (← inferType proof)).iff? | return .continue
  let expr ← Core.betaReduce rhs
  let proof ← mkExpectedTypeHint (← mkPropExt proof) (← mkEq e expr)
  return .visit { expr, proof? := some proof }

/-- Whether a closed row holds a reference, decided: the type arguments of a
transport (`InvokeMeaning.Coherent`). -/
dsimproc [vir_transport] reduceRowRefFree (NRow.refFree _) := fun e => do
  if e.hasFVar || e.hasMVar then return .continue
  let reduced ← withDefault (whnf e)
  if reduced.isConstOf ``Bool.true || reduced.isConstOf ``Bool.false then return .done reduced
  return .continue

open Lean Meta Simp in
/-- A behavioral predicate at the frame a call's type arguments induce, at
the outer frame (`rewriteInducedFrame`): by the executable's coherence or a
model's. -/
private def behaviorInducedFrame (frameIndex : Nat) (rules : List Name) : Simproc := fun e => do
  unless e.getAppNumArgs > frameIndex &&
      (e.getArg! frameIndex).consumeMData.isAppOf ``Skolems.instantiate do
    return .continue
  for rule in rules do
    let step ← rewriteInducedFrame rule e
    unless step matches .continue .. do return step
  return .continue

simproc [vir_transport] ensuresOfInducedFrame (@VerexIR.Proofs.EnsuresOf _ ?functions ?frame _ _ _ _ _ _ _ _ _) :=
  behaviorInducedFrame 2 [``VerexIR.Proofs.closureMeaning_ensuresOf_instantiate,
    ``VerexIR.Proofs.ModelUnit.ensuresOf_instantiate]
simproc [vir_transport] abortsOfInducedFrame (@VerexIR.Proofs.AbortsOf _ ?functions ?frame _ _ _ _ _ _ _) :=
  behaviorInducedFrame 2 [``VerexIR.Proofs.closureMeaning_abortsOf_instantiate,
    ``VerexIR.Proofs.ModelUnit.abortsOf_instantiate]
simproc [vir_transport] resultOfInducedFrame
    (@VerexIR.Proofs.ResultOf _ ?functions ?frame _ _ _ _ _ _ _ _) :=
  behaviorInducedFrame 2 [``VerexIR.Proofs.closureMeaning_resultOf_instantiate,
    ``VerexIR.Proofs.ModelUnit.resultOf_instantiate]
simproc [vir_transport] stateOfInducedFrame (@VerexIR.Proofs.StateOf _ ?functions ?frame _ _ _ _ _ _ _) :=
  behaviorInducedFrame 2 [``VerexIR.Proofs.closureMeaning_stateOf_instantiate,
    ``VerexIR.Proofs.ModelUnit.stateOf_instantiate]
simproc [vir_transport] requiresOfInducedFrame
    (@VerexIR.Proofs.RequiresOf _ ?functions ?frame _ _ _ _ _ _ _) :=
  behaviorInducedFrame 2 [``VerexIR.Proofs.requiresOf_instantiate]

-- Behavioral predicates at the outer frame (above), the substitutions they
-- read evaluated, and the type arguments' freedom from references decided.
attribute [vir_transport] reduceParameterSubst reduceRowSubst reduceRowsSubst ResultShape.subst
  ResultShape.ofSkolem ResultShape.functionOfSkolem

end VerexIR.Proofs.Denote

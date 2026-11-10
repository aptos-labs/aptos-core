-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Solve.VectorSizes
import VerexIR.Proofs.Denote.NominalQuote
import VerexIR.Proofs.Denote.GivenOwn

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

private theorem differentNominalDeclarations (left right : NTy)
    (leftHandle rightHandle : Option StructHandle)
    (leftObserved : left.handle? = leftHandle) (rightObserved : right.handle? = rightHandle)
    (distinct : leftHandle ≠ rightHandle) : left ≠ right := by
  intro same
  apply distinct
  rw [← leftObserved, ← rightObserved, same]

/-- A resource key observes a declaration's identity at its own frame. Resolving
its parameters preserves that identity, without exposing its field layout. -/
private def nominalResourceHandle? (type : Lean.Expr) : MetaM (Option (Lean.Expr × Lean.Expr)) := do
  let type := type.consumeMData
  let resolved := type.isAppOfArity ``Skolems.resolve 4
  let owner := if resolved then type.getArg! 3 else type
  unless isNominalType (← getEnv) owner do return none
  let view ← nativeTypeView owner
  let isStruct := view.isAppOfArity ``NTy.struct 3
  let theoremName := if resolved then
      if isStruct then ``NTy.handle?_resolve_struct else ``NTy.handle?_resolve_enum
    else if isStruct then ``NTy.handle?_struct else ``NTy.handle?_enum
  let arguments := if resolved then #[type.getArg! 2] ++ view.getAppArgs else view.getAppArgs
  let proof ← mkAppM theoremName arguments
  let statement ← inferType proof
  let some (_, _, handle) := statement.eq? | return none
  return some (handle, proof)

/-- Distinct declaration identities suffice to separate resource types.
Inspect their handles, never their field layouts. -/
simproc [vir_denote_norm] nominalDeclarationEquality ((_ : NTy) = _) := fun e => do
  let_expr Eq _ left right := e | return .continue
  let some (leftHandle, leftObserved) ← nominalResourceHandle? left | return .continue
  let some (rightHandle, rightObserved) ← nominalResourceHandle? right | return .continue
  if leftHandle == rightHandle then return .continue
  let condition ← mkAppM ``Ne #[leftHandle, rightHandle]
  let distinct ← withDefault (mkDecideProof condition)
  let proof ← mkAppM ``differentNominalDeclarations
    #[left, right, leftHandle, rightHandle, leftObserved, rightObserved, distinct]
  return .done { expr := mkConst ``False, proof? := some (← mkAppM ``eq_false #[proof]) }

/-- A certificate observation emits the encoding at the same nominal owner
it observed. The structural equation exposes the own conjunct, but its raw
layout is only a view of that owner. Preserve every other argument verbatim. -/
private def nominalCertificateOwner (rhs view owner : Lean.Expr) : Lean.Expr := Id.run do
  unless rhs.isAppOfArity ``And 2 do return rhs
  let own := rhs.getArg! 0
  unless own.isAppOfArity ``Carriers.own 4 do return rhs
  let encoded := own.getArg! 3
  unless encoded.isAppOfArity ``NTy.plainEncode 3 && encoded.getArg! 1 == view do return rhs
  let encoded := mkAppN encoded.getAppFn (encoded.getAppArgs.set! 1 owner)
  let own := mkAppN own.getAppFn (own.getAppArgs.set! 3 encoded)
  return mkApp2 rhs.getAppFn own (rhs.getArg! 1)

/-- Observe a named declaration at an operation boundary. Only the selected
operation's existing equations may inspect the descriptor; the surrounding
carrier types and the declaration's nested fields keep their names. -/
private def nominalOperation (e type : Lean.Expr)
    (structEquations enumEquations : Array Name) :
    SimpM Simp.Step := do
  unless isNominalType (← getEnv) type do return .continue
  let view ← nativeTypeView type
  let equations := if view.isAppOfArity ``NTy.struct 3 then structEquations else enumEquations
  for equation in equations do
    let result? ← withNewMCtxDepth do
      let proof ← mkConstWithFreshMVarLevels equation
      let (arguments, _, statement) ← forallMetaTelescopeReducing (← inferType proof)
      let (lhs, rhs) := if let some (_, lhs, rhs) := statement.eq? then (lhs, rhs)
        else if statement.isAppOfArity ``Iff 2 then (statement.getArg! 0, statement.getArg! 1)
        else (statement, mkConst ``True)
      unless ← withDefault (isDefEq lhs e) do return none
      -- Equations selecting handle equality carry a structural Boolean
      -- condition. Resolve only reflexive equations after observing the
      -- declaration, without asking the solver to inspect its values.
      for argument in arguments do
        unless argument.isMVar && !(← argument.mvarId!.isAssigned) do continue
        let some (_, left, right) := (← inferType argument).eq? | continue
        if ← withDefault (isDefEq left right) then
          argument.mvarId!.assign (← mkEqRefl left)
      let proof ← instantiateMVars (mkAppN proof arguments)
      let rhs ← instantiateMVars rhs
      let rhs := nominalCertificateOwner rhs view type
      -- A field write returns the original owner's plain value. Its row
      -- equation must construct Option.map at that same named result type,
      -- rather than introducing a structural row spelling in the map alone.
      let rhs ← if e.isAppOf ``Proj.set? && rhs.isAppOfArity ``Option.map 4 then do
          let resultType ← whnfR (← inferType e)
          if resultType.isAppOfArity ``Option 1 then
            let canonical := mkAppN rhs.getAppFn
              (rhs.getAppArgs.set! 1 resultType.appArg!)
            if ← withDefault (isDefEq rhs canonical) then pure canonical else pure rhs
          else pure rhs
        else pure rhs
      if proof.hasExprMVar || rhs.hasExprMVar || rhs == e then return none
      -- Use the existing theorem directly. Building a simp theorem here
      -- would publish an auxiliary lemma on every observation of an Iff.
      let proof ← if statement.eq?.isSome then pure proof
        else if statement.isAppOfArity ``Iff 2 then mkAppM ``propext #[proof]
        else mkAppM ``eq_true #[proof]
      return some ({ expr := rhs, proof? := some proof } : Simp.Result)
    if let some result := result? then
      return .visit result
  return .continue

/-- Equality observes the named declaration's established comparison rule.
A table's handle rule keeps its contents and cached fields unobserved. -/
simproc [vir_denote_norm] nominalPlainEquality
    (@NTy.plainEqual ?carriers _ _ _ _) := fun e => do
  nominalOperation e (e.getArg! 2)
    #[``NTy.plainEqual_struct_handle, ``NTy.plainEqual_struct] #[``NTy.plainEqual_enum]

simproc [vir_denote_norm] nominalEquality
    (@NTy.equal ?carriers _ _ _ _) := fun e => do
  nominalOperation e (e.getArg! 2)
    #[``NTy.equal_struct_handle, ``NTy.equal_struct] #[``NTy.equal_enum]

/-- A named declaration is a constructed type. Select its structural order
before the generic order's parameter alternative can become a proof case.
Keep the nominal owner on both forgetful reads. -/
dsimproc ↓ [vir_denote_norm] nominalOrder
    (@NTy.order ?carriers _ _ _ _) := fun e => do
  let_expr NTy.order carriers rank type left right := e | return .continue
  unless isNominalType (← getEnv) type do return .continue
  let forget := mkApp2 (mkConst ``NTy.forget) carriers type
  let result := mkAppN (mkConst ``NTy.plainOrder)
    #[carriers, rank, type, mkApp forget left, mkApp forget right]
  unless ← withDefault (isDefEq e result) do return .continue
  return .visit result

/-- The plain order observes one declaration layer through the definition's
existing equations; field types remain named. -/
simproc ↓ [vir_denote_norm] nominalPlainOrder
    (NTy.plainOrderOf _ _ _ _ _) := fun e => do
  nominalOperation e (e.getArg! 2)
    #[``NTy.plainOrderOf.eq_10] #[``NTy.plainOrderOf.eq_11]

/-- Resource dispatch observes only the identity of a named declaration. -/
simproc [vir_denote_norm] nominalHandle (NTy.handle? _) := fun e => do
  nominalOperation e (e.getArg! 0) #[``NTy.handle?_struct] #[``NTy.handle?_enum]

simproc [vir_denote_norm] nominalDeepHolds
    (@NTy.deepHolds ?carriers _ _) := fun e => do
  nominalOperation e (e.getArg! 1)
    #[``NTy.deepHolds_struct_val, ``NTy.deepHolds_struct]
    #[``NTy.deepHolds_enum_val, ``NTy.deepHolds_enum]

simproc [vir_denote, vir_denote_norm] nominalForget
    (@NTy.forget ?carriers _ _) := fun e => do
  nominalOperation e (e.getArg! 1) #[``NTy.forget_struct] #[``NTy.forget_enum]

simproc [vir_denote, vir_denote_norm] nominalPlaceRead
    (@Proj.get? ?carriers _ _ _ _ _ _) := fun e => do
  nominalOperation e (e.getArg! 3) #[``Proj.get?_field] #[``Proj.get?_variant]

simproc [vir_denote, vir_denote_norm] nominalPlaceWrite
    (@Proj.set? ?carriers _ _ _ _ _ _ _) := fun e => do
  nominalOperation e (e.getArg! 3) #[``Proj.set?_field] #[``Proj.set?_variant]

/-- Updating a local owner retains its mutation certificate at the named
owner, while the selected projection follows the declaration's field rule. -/
simproc [vir_denote, vir_denote_norm] nominalOwnerPlaceWrite
    (@Proj.setOwner? ?carriers _ _ _ _ _ _ _ _ _) := fun e => do
  let owner := e.getArg! 5
  let step ← nominalOperation e owner
    #[``Proj.setOwner?_field] #[``Proj.setOwner?_variant]
  let .visit result := step | return step
  let view ← nativeTypeView owner
  let expr := result.expr.replace fun term => if term == view then some owner else none
  return .visit { result with expr }


/-- A declaration pattern observes its own layer through the established
binding equations. Its context and nested field patterns remain named. -/
simproc [vir_denote, vir_denote_norm] nominalPatternBinding
    (@Pat.bindValue ?carriers _ _ _ _ _) := fun e => do
  nominalOperation e (e.getArg! 2) #[``Pat.bindValue_struct] #[``Pat.bindValue_variant]

simproc [vir_denote, vir_denote_norm] nominalPlainPatternBinding
    (@Pat.bindPlain ?carriers _ _ _ _ _ _) := fun e => do
  nominalOperation e (e.getArg! 2) #[``Pat.bindPlain_struct] #[``Pat.bindPlain_variant]

simproc [vir_denote, vir_denote_norm] nominalCertifiedPlaceRead
    (@Proj.getCertified? ?carriers _ _ _ _ _ _) := fun e => do
  nominalOperation e (e.getArg! 3) #[``Proj.getCertified?_field] #[``Proj.getCertified?_variant]

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

simproc ↓ [vir_denote_norm] flowBindAction (@VerexIR.Proofs.Denote.Flow.bind ?carriers _ _ _ _ _ _ _) :=
  bindActionStep



/-- An equation between values under the family a call's type arguments
induce, stated as the caller reads them: the equation of their caller views
(`NTy.ofSkolem`), the view in which the caller holds a callee's results. -/
simproc ↓ [vir_denote_norm] instantiatedEqInCallerView (@Eq _ _ _) :=
  fun e => do
    let_expr Eq carrier left right := e | return .continue
    -- Optional observations retain their underlying memory view. Expanding
    -- a mapped option equation exposes the value equation below; transporting
    -- the whole option first would recreate the same equation under an
    -- existential witness and make normalization repeat indefinitely.
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
    -- The transports of an induced family take its inhabitants
    -- (`Carriers.instantiate`), which the equation's own family carries.
    let some nonempty ← (try
        pure (some (← synthInstance
          (mkApp (mkConst ``VerexIR.Proofs.Denote.CarriersNonempty) outer)))
      catch _ => pure none)
      | return .continue
    let transport := mkAppN (mkConst ``VerexIR.Proofs.Denote.NTy.ofSkolem)
      #[outer, nonempty, θ, τ]
    let restated ← (try
        let equation ← mkEq (mkApp transport left) (mkApp transport right)
        let injective := mkAppN (mkConst ``VerexIR.Proofs.Denote.NTy.ofSkolem_inj)
          #[outer, nonempty, θ, τ, left, right]
        let proof ← mkAppM ``propext #[← mkAppM ``Iff.symm #[injective]]
        if ← isTypeCorrect proof then pure (some (equation, proof)) else pure none
      catch _ => pure none)
    -- A side spelled at another type would make an ill-typed rewrite: say
    -- so and leave the equation as stated, rather than restate it wrongly.
    let some (equation, proof) := restated
      | do
        trace[Elab.definition] "verex: an equation in the caller's view \
          was left as stated: {e}"
        return .continue
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
  let env ← getEnv
  reduceSubst (fun row subject => substituteNTy row subject (some env)) e row τ

/-- A frame's parameter replacement constructs the same named declaration
at its replaced arguments. Only the explicit arguments are traversed; the
field layout stays behind the descriptor. Unknown argument syntax retains
its substitution rather than being assumed independent of parameters. -/
dsimproc ↓ [vir_denote, vir_denote_norm] reduceNominalParameterReplacement
    (NTy.substWith _ _) := fun e => do
  let_expr NTy.substWith types owner := e | return .continue
  let env ← getEnv
  unless isNominalType env owner && spelledNTy owner (some env) do return .continue
  let replaced := owner.replace fun term =>
    if term.isAppOfArity ``NTy.param 1 then some (mkApp types term.appArg!).headBeta else none
  return .visit replaced

/-- The substitution of a literal row evaluates (`reduceParameterSubst`). -/
dsimproc ↓ [vir_denote, vir_denote_norm] reduceRowSubst
    (VerexIR.Proofs.Denote.NRow.subst _ _) := fun e => do
  let_expr VerexIR.Proofs.Denote.NRow.subst row fields := e | return .continue
  let env ← getEnv
  reduceSubst (fun row subject => substituteNRow row subject (some env)) e row fields

/-- The substitution of literal variant rows evaluates (`reduceParameterSubst`). -/
dsimproc ↓ [vir_denote, vir_denote_norm] reduceRowsSubst
    (VerexIR.Proofs.Denote.NRows.subst _ _) := fun e => do
  let_expr VerexIR.Proofs.Denote.NRows.subst row rows := e | return .continue
  let env ← getEnv
  reduceSubst (fun row subject => substituteNRows row subject (some env)) e row rows

/-- A transport between types that agree, such as one by a generic site's
compile-time evidence (`Term.callGeneric`, `Term.closureGeneric`): the value
itself. -/
dsimproc ↓ [vir_denote, vir_denote_norm] resolvedCast (cast _ _) := fun e => do
  let_expr cast α β _ value := e | return .continue
  unless ← withTransparency .default (isDefEq α β) do return .continue
  return .visit value

/-- Forgetting the certificates of a vector or a table whose type carries
none: the value itself.  At a certificate-free type the certified and the
plain carrier are one type and the forgetting is the identity on it
(`NTy.forget_eq_cast_of_certificateFree`), whose transport then resolves
(`resolvedCast`).

Read before the structural equations, and only at the two types whose
equations distribute the forgetting into a map of an *unapplied*
`NTy.forget` over the elements (`NTy.forget_vector`, `NTy.forget_table`).
No equation reaches such a map: the ones that close a forgetting
(`NTy.forget_int`, …) are stated at an applied value.  So a vector of
integers would keep two spellings, the value and the map over it, and a
clause stating one would never meet the other.  At every other type the
structural equation closes the forgetting itself, by reflexivity and so
without a transport, which is why this is not read there. -/
simproc ↓ [vir_denote, vir_denote_norm] forgottenCertificateFree
    (@VerexIR.Proofs.Denote.NTy.forget ?carriers _ _) := fun e => do
  let e := e.consumeMData
  let #[carriers, τ, value] := e.getAppArgs | return .continue
  let τ := τ.consumeMData
  if τ.hasExprMVar || τ.hasFVar || τ.hasLooseBVars then return .continue
  unless τ.isAppOfArity ``VerexIR.Proofs.Denote.NTy.vector 1 ||
      τ.isAppOfArity ``VerexIR.Proofs.Denote.NTy.table 2 do
    return .continue
  -- The equation is not reflexive at a certificate-free vector or table,
  -- where it states that a map of the identity is the array mapped, so
  -- this is a proof-building reader and not a `dsimproc`.
  -- The carriers of a certificate-free type agree only by unfolding their
  -- definitions, which simp's reducible transparency does not do.
  let some (transported, proof) ← (try withNewMCtxDepth <| withTransparency .default do
      -- The side condition is decided here rather than left to the
      -- application's unification, so that a type carrying a certificate
      -- declines instead of raising.
      unless (← whnf (mkApp (mkConst ``VerexIR.Proofs.Denote.NTy.certificateFree) τ)).isConstOf
          ``Bool.true do
        return none
      let proof ← mkAppOptM ``VerexIR.Proofs.Denote.NTy.forget_eq_cast_of_certificateFree
        #[carriers, τ, ← mkEqRefl (mkConst ``Bool.true), value]
      let some (_, _, transported) := (← instantiateMVars (← inferType proof)).eq?
        | return none
      return some (transported, proof)
    catch _ => pure none)
    | return .continue
  return .visit { expr := transported, proof? := some proof }

/-- Certifying a certificate-free aggregate is the same transport as its
forgetting, rather than a rebuilt vector or table. -/
simproc ↓ [vir_denote, vir_denote_norm] certifiedCertificateFree
    (@VerexIR.Proofs.Denote.NTy.ofDeepHolds ?carriers _ _ _) := fun e => do
  let_expr VerexIR.Proofs.Denote.NTy.ofDeepHolds carriers τ value _ := e | return .continue
  let τ := τ.consumeMData
  if τ.hasExprMVar || τ.hasFVar || τ.hasLooseBVars then return .continue
  unless τ.isAppOfArity ``VerexIR.Proofs.Denote.NTy.vector 1 ||
      τ.isAppOfArity ``VerexIR.Proofs.Denote.NTy.table 2 do
    return .continue
  let some (transported, proof) ← (try withNewMCtxDepth <| withTransparency .default do
      unless (← whnf (mkApp (mkConst ``VerexIR.Proofs.Denote.NTy.certificateFree) τ)).isConstOf
          ``Bool.true do
        return none
      let reverse ← mkAppOptM ``VerexIR.Proofs.Denote.NTy.cast_eq_ofDeepHolds_of_certificateFree
        #[carriers, τ, ← mkEqRefl (mkConst ``Bool.true), value]
      let proof ← mkEqSymm reverse
      let some (_, lhs, transported) := (← instantiateMVars (← inferType proof)).eq? | return none
      unless ← isDefEq lhs e do return none
      return some (transported, proof)
    catch _ => pure none) | return .continue
  return .visit { expr := transported, proof? := some proof }

private theorem forgottenVectorBounded {α β : Type} (f : α → β) (entries : Array α)
    (bounded : (entries.map f).size < 2 ^ 64) : entries.size < 2 ^ 64 := by
  simpa only [Array.size_map] using bounded

/-- Recover the certified vector behind a structurally expanded forgetting.
The vector equation is read before an outer certification or invariant, so
its map spelling must be accepted by the round-trip readers as well. -/
private def forgottenVector? (carriers τ value : Lean.Expr) : MetaM (Option Lean.Expr) := do
  let_expr VerexIR.Proofs.Denote.NTy.vector element := τ | return none
  let_expr VerexIR.SpecVector.mk _ values bounded := value | return none
  let_expr Array.map _ _ forget entries := values | return none
  withTransparency .default do
    unless ← isDefEq forget (mkApp2 (mkConst ``VerexIR.Proofs.Denote.NTy.forget) carriers element) do
      return none
    let bound ← mkAppM ``forgottenVectorBounded #[forget, entries, bounded]
    let vector ← mkAppM ``VerexIR.SpecVector.mk #[entries, bound]
    unless ← isDefEq value (mkApp3 (mkConst ``VerexIR.Proofs.Denote.NTy.forget) carriers τ vector) do
      return none
    return some vector

-- Preserve whole forgotten values and cancel their common injective map.
attribute [vir_denote_norm] NTy.forget_eq_forget

/-- Certification reads an existing certificate when given its forgotten value. -/
@[vir_denote, vir_denote_norm] private theorem certifyingAt_forget [Carriers] {State : Type} (site : Nat) (kind : CertifyKind)
    (τ : NTy) (value : τ.carrier) :
    certifyingAt (State := State) site kind τ (τ.forget value) = Spec.pure value := by
  apply certifying_eq_pure (τ.deepHolds_forget value)
  intro held
  exact NTy.forget_injective τ (NTy.forget_ofDeepHolds τ (τ.forget value) held)

/-- Recover a vector at a certification boundary, including a field read
whose owner was constructed from that vector's forgotten value. -/
private def recertifiedForgottenVectorStep (e carriers τ value : Lean.Expr) : MetaM Simp.Step := do
  let some (vector, proof) ← (try withNewMCtxDepth <| withTransparency .default do
      let some vector ← forgottenVector? carriers τ value | return none
      let proof := mkApp3 (mkConst ``VerexIR.Proofs.Denote.NTy.ofDeepHolds_forget) carriers τ vector
      let some (_, lhs, _) := (← inferType proof).eq? | return none
      unless ← isDefEq lhs e do return none
      return some (vector, proof)
    catch _ => pure none) | return .continue
  return .visit { expr := vector, proof? := some proof }

/-- Re-certifying a vector after forgetting its certificates returns the
original vector, also after the forgetting's structural equation fired. -/
simproc ↓ [vir_denote, vir_denote_norm] recertifiedForgottenVector
    (@VerexIR.Proofs.Denote.NTy.ofDeepHolds ?carriers _ _ _) := fun e => do
  let_expr VerexIR.Proofs.Denote.NTy.ofDeepHolds carriers τ value _ := e | return .continue
  recertifiedForgottenVectorStep e carriers τ value

/-- A field constructed from a forgotten certified value recovers that
value. Observe the projection without exposing a dependent certificate to
simplification; vectors also admit their already-expanded map spelling. -/
simproc ↓ [vir_denote, vir_denote_norm] selectedForgottenValue
    (@VerexIR.Proofs.Denote.Var.selectCertified ?carriers _ _ _ _ _ _) := fun e => do
  let_expr VerexIR.Proofs.Denote.Var.selectCertified _ _ _ _ τ _ owner := e
    | return .continue
  -- Unknown owners need no projection evaluation. The vector reader also
  -- recognizes a mapped forgetting, so retain its established observation path.
  if τ.isAppOfArity ``NTy.vector 1 then
    let_expr NTy.ofDeepHolds carriers τ value _ := ← NativeMaps.certificationView e
      | return .continue
    return ← recertifiedForgottenVectorStep e carriers τ (← whnfD value)
  let_expr NTy.ofDeepHolds _ _ fields _ := owner | return .continue
  unless fields.isAppOfArity ``Prod.mk 4 do return .continue
  let_expr NTy.ofDeepHolds carriers τ value _ := ← NativeMaps.certificationView e
    | return .continue
  let isForgetting (value : Lean.Expr) : Bool :=
    value.isAppOfArity ``NTy.forget 3 || value.isAppOf ``ParamPlain.of ||
      value.isAppOfArity ``Subtype.val 3
  let observed ← withDefault <| whnfHeadPred value (fun value => pure (!isForgetting value))
  unless isForgetting observed do return .continue
  let candidate := observed.appArg!
  let some proof ← (try withNewMCtxDepth <| withDefault do
      unless ← isDefEq (← inferType candidate) (mkApp2 (mkConst ``NTy.carrier) carriers τ) do
        return none
      let proof := mkApp3 (mkConst ``NTy.ofDeepHolds_forget) carriers τ candidate
      let some (_, lhs, _) := (← inferType proof).eq? | return none
      unless ← isDefEq lhs e do return none
      return some proof
    catch _ => pure none) | return .continue
  return .visit { expr := candidate, proof? := some proof }

/-- A forgotten certified vector retains its element invariants. -/
simproc ↓ [vir_denote, vir_denote_norm] deepHoldsForgottenVector
    (@VerexIR.Proofs.Denote.NTy.deepHolds ?carriers _ _) := fun e => do
  let_expr VerexIR.Proofs.Denote.NTy.deepHolds carriers τ value := e | return .continue
  let some proof ← (try withNewMCtxDepth <| withTransparency .default do
      let some vector ← forgottenVector? carriers τ value | return none
      let held := mkApp3 (mkConst ``VerexIR.Proofs.Denote.NTy.deepHolds_forget) carriers τ vector
      unless ← isDefEq (← inferType held) e do return none
      return some (← mkAppM ``eq_true #[held])
    catch _ => pure none) | return .continue
  return .done { expr := mkConst ``True, proof? := some proof }

/-- The parameter-family equation can carry an induced carrier spelling,
which the theorem matcher at reducible transparency cannot identify. -/
simproc ↓ [vir_denote, vir_denote_norm] forgottenParameterEquation (@Eq _ _ _) := fun e => do
  let_expr Eq _ left right := e | return .continue
  let_expr VerexIR.Proofs.Denote.ParamPlain.of _ _ lent leftValue := left | return .continue
  let_expr VerexIR.Proofs.Denote.Carriers.lent carriers index := lent | return .continue
  let_expr VerexIR.Proofs.Denote.ParamPlain.of _ _ other rightValue := right | return .continue
  let some (result, proof) ← (try withNewMCtxDepth <| withTransparency .default do
      unless ← isDefEq lent other do return none
      let proof ← mkAppOptM ``VerexIR.Proofs.Denote.Carriers.lent_of_eq_of
        #[carriers, index, leftValue, rightValue]
      let some (_, lhs, result) := (← instantiateMVars (← inferType proof)).eq? | return none
      unless ← isDefEq lhs e do return none
      return some (result, proof)
    catch _ => pure none) | return .continue
  return .visit { expr := result, proof? := some proof }

/-- Forgetting certificates is injective element by element, so a vector's
resolution equation still identifies the vector after the structural map
normal form has been exposed. -/
@[vir_denote_norm] theorem NTy.map_forget_eq_map_forget [Carriers] (τ : NTy)
    (left right : Array τ.carrier) :
    left.map (NTy.forget τ) = right.map (NTy.forget τ) ↔ left = right :=
  Array.map_inj_right fun _ _ equal => NTy.forget_injective τ equal

/-- The map spelling of forgetting is injective, including across the
induced-family types that only default transparency identifies. -/
simproc ↓ [vir_denote, vir_denote_norm] forgottenVectorEquation (@Eq _ _ _) := fun e => do
  let_expr Eq _ left right := e | return .continue
  let_expr Array.map _ _ f leftValues := left | return .continue
  let_expr Array.map _ _ g rightValues := right | return .continue
  let_expr VerexIR.Proofs.Denote.NTy.forget carriers τ := f | return .continue
  let some (result, proof) ← (try withNewMCtxDepth <| withTransparency .default do
      unless ← isDefEq f g do return none
      let equivalence ← mkAppOptM ``VerexIR.Proofs.Denote.NTy.map_forget_eq_map_forget
        #[carriers, τ, leftValues, rightValues]
      let proof ← mkAppM ``propext #[equivalence]
      let some (_, lhs, result) := (← instantiateMVars (← inferType proof)).eq? | return none
      unless ← isDefEq lhs e do return none
      return some (result, proof)
    catch _ => pure none) | return .continue
  return .visit { expr := result, proof? := some proof }

/-- The typed reads of a type at a family (`NTy.read` and its carrier,
instances, write, read-back, and view). -/
private def typedReads : List Name :=
  [``VerexIR.Proofs.Denote.NTy.readCarrier, ``VerexIR.Proofs.Denote.NTy.read,
   ``VerexIR.Proofs.Denote.NTy.readDecEq, ``VerexIR.Proofs.Denote.NTy.readInhabited,
   ``VerexIR.Proofs.Denote.NTy.readPlain]

/-- A typed read of a type at a family, applied to `rest`, as the normal
form spells it: at an integer type or `num` the integers' (values, `Int`,
its instances), at another concrete type the type's own (values as they
are, its carrier, `NTy.decEq`, `nativeInhabited`), at a parameter the
family's. The spellings are those of the typed views
(`NativeMaps.typedView?`) and the typed-read lemmas (`NTy.read_int`, …),
by definition; here they reach implicit arguments and instances, which
rewriting does not. -/
private def typedReadSpelling (head : Name) (family : Lean.Expr)
    (nonempty? : Option Lean.Expr) (τ : Lean.Expr) (rest : Array Lean.Expr) :
    MetaM (Option Lean.Expr) := do
  let τ := τ.consumeMData
  -- The inhabitants of the family, which `NTy.readInhabited` and the
  -- fallback inhabitant take (`CarriersNonempty`): the one the read carries,
  -- or the one the context gives at this family.
  let inhabitants : MetaM Lean.Expr := do
    match nonempty? with
    | some inst => pure inst
    | none =>
        synthInstance (mkApp (mkConst ``VerexIR.Proofs.Denote.CarriersNonempty) family)
  let takesInhabitants := head == ``VerexIR.Proofs.Denote.NTy.readInhabited ||
      head == ``VerexIR.Proofs.Denote.NTy.readPlain
  let atFamily (τ : Lean.Expr) : MetaM Lean.Expr := do
    let leading ← if takesInhabitants then pure #[family, ← inhabitants] else pure #[family]
    pure (mkAppN (mkConst head) ((leading.push τ) ++ rest))
  if head == ``VerexIR.Proofs.Denote.NTy.readPlain then
    let some (held, values) := rest[0]?.map (·, rest.extract 1 rest.size) | return none
    if let some reader ← NativeMaps.canonicalPlainReader? family τ then
      return some (mkAppN reader values).headBeta
    -- A specialized parameter's proof must be stated at its resolved type,
    -- just as the reader and its carrier are. An old expected-type hint can
    -- otherwise keep the parameter's inhabitability annotation here.
    let inhabitable := mkApp (mkConst ``VerexIR.Proofs.Denote.NTy.inhabitable) τ
    let held ← if ← withDefault (isDefEq inhabitable (mkConst ``Bool.true)) then
        mkExpectedTypeHint (← mkEqRefl (mkConst ``Bool.true))
          (← mkEq inhabitable (mkConst ``Bool.true))
      else pure held
    return some (mkAppN (mkConst head) (#[family, ← inhabitants, τ, held] ++ values))
  if τ.isAppOfArity ``VerexIR.Proofs.Denote.NTy.param 1 then return some (← atFamily τ)
  let int := τ.isAppOfArity ``VerexIR.Proofs.Denote.NTy.int 2
  let num := τ.isConstOf ``VerexIR.Proofs.Denote.NTy.num
  unless int || num || isNominalType (← getEnv) τ ||
      τ.getAppFn.constName?.any (·.getPrefix == ``VerexIR.Proofs.Denote.NTy) do
    return none
  let integer := int || num
  let intType := mkConst ``Int
  let carrier := mkApp2 (mkConst ``VerexIR.Proofs.Denote.NTy.carrier) family τ
  -- The reads taking the type's inhabitability: the proof, and the arguments after it.
  let inhabited? : Option (Lean.Expr × Array Lean.Expr) := do
    let proof ← rest[0]?
    pure (proof, rest.extract 1 rest.size)
  let nativeInhabited (proof : Lean.Expr) : MetaM Lean.Expr := do
    let inhabitable := mkApp (mkConst ``VerexIR.Proofs.Denote.NTy.inhabitable) τ
    let proof ← if ← withDefault (isDefEq inhabitable (mkConst ``Bool.true)) then
        mkExpectedTypeHint (← mkEqRefl (mkConst ``Bool.true))
          (← mkEq inhabitable (mkConst ``Bool.true))
      else pure proof
    let fallback := mkApp4 (mkConst ``VerexIR.Proofs.Denote.NTy.inhabitantOf) family
      (← inhabitants) τ proof
    pure (mkApp3 (mkConst ``VerexIR.Proofs.Denote.nativeInhabited) family τ fallback)
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
      else nativeInhabited proof
    return some (mkAppN inst rest)
  return some (← atFamily τ)

/-- Evaluate a typed read from its original native type. Constructor equations
can unfold a nominal descriptor while selecting their identity branch; retain
that descriptor in the identity's binder instead. -/
dsimproc ↓ [vir_denote, vir_denote_norm] typedReadValue
    (@VerexIR.Proofs.Denote.NTy.read ?carriers _) := fun e => do
  let args := e.getAppArgs
  let some family := args[0]? | return .continue
  let some τ := args[1]? | return .continue
  let env ← getEnv
  let τ ← NativeMaps.nativeTypeHead τ
  unless isNominalType env τ ||
      τ.isAppOfArity ``VerexIR.Proofs.Denote.NTy.struct 3 ||
      τ.isAppOfArity ``VerexIR.Proofs.Denote.NTy.enum 5 do return .continue
  let some read ← typedReadSpelling ``VerexIR.Proofs.Denote.NTy.read family none τ
      (args.extract 2 args.size) | return .continue
  if read == e then return .continue
  return .visit read

/-- A certificate-free field is its plain projection cast to the identical
carrier, without rebuilding containers or visiting a dependent certificate. -/
simproc ↓ [vir_denote, vir_denote_norm] freeCertifiedField
    (@VerexIR.Proofs.Denote.Var.selectCertified ?carriers _ _ _ _ _ _) := fun e => do
  let_expr VerexIR.Proofs.Denote.Var.selectCertified _carriers _source _arguments _fields field _position _value := e
    | return .continue
  let free := mkApp (mkConst ``VerexIR.Proofs.Denote.NTy.certificateFree) field
  unless (← whnfD free).isConstOf ``Bool.true do return .continue
  let freeProof ← mkExpectedTypeHint (← mkEqRefl (mkConst ``Bool.true))
    (← mkEq free (mkConst ``Bool.true))
  let proof := mkAppN (mkConst ``VerexIR.Proofs.Denote.Var.selectCertified_of_certificateFree)
    (e.getAppArgs.push freeProof)
  let some (_, _, rhs) := (← inferType proof).eq? | return .continue
  return .visit { expr := rhs, proof? := some proof }

private theorem NTy.forget_vector_getD [Carriers] (element : NTy)
    (vector : (NTy.vector element).carrier) (plain : (NTy.vector element).plain)
    (forgotten : NTy.forget (.vector element) vector = plain) (index : Nat)
    (certifiedDefault : element.carrier) (plainDefault : element.plain)
    (bounded : index < plain.values.size) :
    NTy.forget element (vector.values[index]?.getD certifiedDefault) =
      plain.values[index]?.getD plainDefault := by
  have values := congrArg (fun vector : (NTy.vector element).plain => vector.values) forgotten
  simp only [NTy.forget_vector_values] at values
  have size : vector.values.size = plain.values.size := by
    simpa only [Array.size_map] using congrArg Array.size values
  have bound : index < vector.values.size := by
    rw [size]
    exact bounded
  rw [← values, Array.getElem?_map, Array.getElem?_eq_getElem bound,
    Option.map_some, Option.getD_some, Option.getD_some]

private theorem NTy.forget_vector_lookup [Carriers] (element : NTy)
    (vector : (NTy.vector element).carrier) (plain : (NTy.vector element).plain)
    (forgotten : NTy.forget (.vector element) vector = plain) (index : Nat)
    (result : Option element.carrier) (read : vector.values[index]? = result) :
    plain.values[index]? = result.map (NTy.forget element) := by
  have values := congrArg (fun vector : (NTy.vector element).plain => vector.values) forgotten
  simp only [NTy.forget_vector_values] at values
  rw [← values, Array.getElem?_map, read]

/-- A nominal field's plain row comes from the owner's plain field. The
certificate remains inside the shared field operation until this observation. -/
simproc ↓ [vir_denote_norm] certifiedFieldPlain (@Subtype.val _ _ _) := fun e => do
  let_expr Subtype.val _ _ value := e | return .continue
  if value.isAppOfArity ``NTy.ofDeepHolds 4 then
    return ← nominalOperation e (value.getArg! 1)
      #[``NTy.ofDeepHolds_struct_val] #[``NTy.ofDeepHolds_enum_val]
  unless value.isAppOfArity ``VerexIR.Proofs.Denote.Var.selectCertified 7 do return .continue
  let proof := mkAppN (mkConst ``VerexIR.Proofs.Denote.Var.forget_selectCertified) value.getAppArgs
  let some (_, lhs, rhs) := (← inferType proof).eq? | return .continue
  unless ← withDefault (isDefEq lhs e) do return .continue
  return .visit { expr := rhs, proof? := some proof }

/-- A result shape as the constructor it reduces to, with that
constructor's arguments: the shape a function's result type names is the
generated `<fn>.shape` abbreviation, which reduction unfolds and matching
does not. -/
private def resultShape? (shape : Lean.Expr) : MetaM (Option (Name × Array Lean.Expr)) := do
  if shape.hasLooseBVars then return none
  let reduced ← whnfD shape
  let some constructor := reduced.getAppFn.constName? | return none
  unless constructor == ``VerexIR.Proofs.Denote.ResultShape.none ||
      constructor == ``VerexIR.Proofs.Denote.ResultShape.one do
    return none
  return some (constructor, reduced.getAppArgs)

/-- Literal scalar arguments of a map construction, without unfolding named
values or maps. These are the closed constructions the typed map evaluator
can materialize at a call's specialization boundary. -/
private def primitiveMapLiteral (type value : Lean.Expr) : Bool :=
  let value := value.consumeMData
  if type.isConstOf ``Int then
    value.int?.isSome ||
      ((value.isAppOfArity ``Int.ofNat 1 || value.isAppOfArity ``Int.negSucc 1) &&
        value.appArg!.isRawNatLit)
  else if type.isConstOf ``Nat then value.nat?.isSome || value.isRawNatLit
  else if type.isConstOf ``Bool then value.isConstOf ``Bool.true || value.isConstOf ``Bool.false
  else type.isConstOf ``String && value.isStringLit

private def literalMapConstruction (e : Lean.Expr) : Bool := Id.run do
  unless e.isAppOfArity ``VerexIR.SpecMap.updateAll 7 &&
      !e.hasFVar && !e.hasMVar && !e.hasLooseBVars do return false
  let args := e.getAppArgs
  unless [``Int, ``Nat, ``Bool, ``String].any args[0]!.isConstOf do return false
  let discipline := args[3]!
  unless discipline.isAppOfArity ``VerexIR.SpecMap.Discipline.sequence 1 ||
      (discipline.isAppOfArity ``VerexIR.SpecMap.Discipline.ordered 2 &&
        discipline.appArg!.isConstOf ``VerexIR.SpecMap.intOrder) do return false
  let some (_, entries) := args[4]!.listLit? | return false
  let some (_, keys) := args[5]!.listLit? | return false
  let some (_, values) := args[6]!.listLit? | return false
  return keys.all (primitiveMapLiteral args[0]!) &&
    values.all (primitiveMapLiteral args[1]!) &&
    entries.all fun entry => entry.isAppOfArity ``Prod.mk 4 &&
      primitiveMapLiteral args[0]! (entry.getArg! 2) &&
      primitiveMapLiteral args[1]! (entry.getArg! 3)

/-- A candidate outer spelling, including implicit type arguments ordinary
rewriting does not reach, and the canonical spelling of the plain layer
(`PlainSpelling`): the wrapper abbreviations are reducible, so a leaf's
congruence closure unfolds them — and goes on to unfold the plain family
into its instance literal — where it keeps the worker's spelling as the
goal states it.  A term read once at each spelling enters the closure
twice, and no equation of the leaf relates the two, so the pass that makes
a goal's families canonical makes their spelling canonical with them. -/
private def canonicalFamilies (e : Lean.Expr) : MetaM Lean.Expr := do
  let env ← getEnv
  let substituteNTy := fun row τ => FamilyTransport.substituteNTy row τ (some env)
  let substituteNRow := fun row fields => FamilyTransport.substituteNRow row fields (some env)
  let substituteNRows := fun row rows => FamilyTransport.substituteNRows row rows (some env)
  -- The two reasons to walk a term, kept apart: a family to take to its
  -- outer spelling, and a plain layer to respell.  A term with only the
  -- latter is respelled and nothing else, so that widening the pass to
  -- the spelling does not also run the family branches over terms they
  -- used to be spared.
  let instantiated := (e.find? fun t => t.isConstOf ``VerexIR.Proofs.Denote.Skolems.instantiate ||
      t.isConstOf ``VerexIR.Proofs.Denote.Carriers.instantiate ||
      t.isConstOf ``VerexIR.Proofs.Denote.NTy.subst || t.isConstOf ``VerexIR.Proofs.Denote.NRow.subst ||
      t.isConstOf ``VerexIR.Proofs.Denote.NRows.subst ||
      t.isConstOf ``VerexIR.Proofs.Denote.ResultShape.subst ||
      t.constName?.any typedReads.contains).isSome
  let spelled := (e.find? fun t => t.isConstOf ``VerexIR.Proofs.Denote.NTy.plain ||
      t.isConstOf ``VerexIR.Proofs.Denote.HListPlain ||
      t.isConstOf ``VerexIR.Proofs.Denote.variantPlain).isSome
  let mapInputs := (e.find? fun t =>
    t.isAppOfArity ``VerexIR.SpecMap.ofEntries 7 &&
      ((t.getArg! 6).isAppOfArity ``Prod.fst 3 ||
        (t.getArg! 6).isAppOfArity ``Prod.snd 3 ||
        #[t.getArg! 4, t.getArg! 5].any fun reader =>
          match reader.consumeMData with
          | .lam _ domain body _ => body == .bvar 0 &&
              (domain.isAppOfArity ``VerexIR.Proofs.Denote.NTy.plain 2 ||
                domain.isAppOfArity ``VerexIR.Proofs.Denote.NTy.plainOf 2)
          | _ => false)).isSome
  let literalMaps := (e.find? literalMapConstruction).isSome
  let givenOperations := (e.find? fun t => (givenOperationProjection? t).isSome).isSome
  unless instantiated || spelled || mapInputs || literalMaps || givenOperations do return e
  -- This is a syntactic pass. Keep binders closed instead of rebuilding their
  -- telescopes; only closed expressions may reach a meta-level reduction.
  Core.transform e (post := fun t => do
    let args := t.getAppArgs
    let some head := t.getAppFn.constName? | return .continue
    if let some original := givenOperationProjection? t then return .done original
    if literalMapConstruction t then
      if let some literal ← (try NativeMaps.groundLiteral? t catch _ => pure none) then
        if literal != t then return .done literal
    if head == ``VerexIR.SpecMap.ofEntries then
      if let some canonical ← NativeMaps.canonicalMapOfEntries? t then
        return .done canonical
    let induced (family : Lean.Expr) : Option (Lean.Expr × Lean.Expr) := do
      let (θ, outer) ← inducedCarriers? family
      (substitutionRow? θ).map (outer, ·)
    -- The plain layer in its canonical spelling, at the outer family where
    -- an induced one spells it (`PlainSpelling`).  A wrapper at the outer
    -- family is respelled where it stands: the spelling is the pass's own
    -- business, and the two are the same type.
    let plain : Option Lean.Expr := do
      let spelling ← plainSpelling? t
      let carriers ← if spelling.worker then
          (if spelling.family.isAppOfArity ``VerexIR.Proofs.Denote.Carriers.toPlain 1
            then some spelling.family.appArg! else none)
        else some spelling.family
      let (outerSpelling, substituted) := match induced carriers with
        | some (outer, row) => ({ spelling with family := outer, worker := false }, some row)
        | none => ({ spelling with family := carriers, worker := false }, none)
      let payload (substitute : Lean.Expr → Lean.Expr → Option Lean.Expr)
          (operand : Lean.Expr) : Option Lean.Expr :=
        match substituted with
          | some row => substitute row operand
          | none => some operand
      let rebuilt ← match spelling.payload with
        | .ty τ => (payload substituteNTy τ).map outerSpelling.tyOf
        | .row fields => (payload substituteNRow fields).map outerSpelling.rowOf
        | .variant names rows =>
            (payload substituteNRows rows).map (outerSpelling.variantOf names)
      guard (rebuilt != t)
      pure rebuilt
    if let some t' := plain then return .done t'
    unless instantiated do return .continue
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
    -- A result shape substituted by a spelled row, and the carrier it
    -- reads: the canonical spelling of a result's type is the type's own,
    -- not the shape's.  `ResultShape.subst` and `ResultShape.carrier` are
    -- reducible, but a leaf's congruence closure and simp's unifier stop
    -- at the generated `<fn>.shape` abbreviation, so a result type enters
    -- the closure as a spelling unrelated to the type it is, and a lemma
    -- stated of that type never unifies with it.
    if head == ``VerexIR.Proofs.Denote.ResultShape.subst then
      let #[θ, shape] := args | return .continue
      let row ← if θ.hasLooseBVars then pure θ else spelledRow θ
      unless spelledSpine row do return .continue
      let some (constructor, shapeArgs) := (← resultShape? shape) | return .continue
      if constructor == ``VerexIR.Proofs.Denote.ResultShape.none then
        return .done (mkConst ``VerexIR.Proofs.Denote.ResultShape.none)
      let #[τ] := shapeArgs | return .continue
      return match substituteNTy row τ with
        | some τ' => .done (mkApp (mkConst ``VerexIR.Proofs.Denote.ResultShape.one) τ')
        | none => .continue
    if head == ``VerexIR.Proofs.Denote.ResultShape.carrier then
      let #[family, shape] := args | return .continue
      let some (constructor, shapeArgs) := (← resultShape? shape) | return .continue
      if constructor == ``VerexIR.Proofs.Denote.ResultShape.none then
        return .done (mkConst ``Unit)
      let #[τ] := shapeArgs | return .continue
      return .done (mkApp2 (mkConst ``VerexIR.Proofs.Denote.NTy.carrier) family τ)
    -- A typed read at a concrete type, or at a parameter of an induced
    -- family, spelled as the normal form spells it (`typedReadSpelling`).
    if typedReads.contains head then
      -- `NTy.readInhabited` and `NTy.readPlain` carry the family's
      -- inhabitants between the family and the type.
      let takesInhabitants := head == ``VerexIR.Proofs.Denote.NTy.readInhabited ||
          head == ``VerexIR.Proofs.Denote.NTy.readPlain
      let leading := if takesInhabitants then 3 else 2
      let some family := args[0]? | return .continue
      let some τ := args[if takesInhabitants then 2 else 1]? | return .continue
      let nonempty? := if takesInhabitants then args[1]? else none
      let rest := args.extract leading args.size
      -- A native type's default stays at its family: the outer family's
      -- default is equal by a lemma (`ownInstance?`), not by definition.
      let τ₀ := τ.consumeMData
      if takesInhabitants && (induced family).isSome &&
          !(τ₀.isAppOfArity ``VerexIR.Proofs.Denote.NTy.param 1 ||
            τ₀.isAppOfArity ``VerexIR.Proofs.Denote.NTy.int 2 ||
            τ₀.isConstOf ``VerexIR.Proofs.Denote.NTy.num) then
        match ← typedReadSpelling head family nonempty? τ rest with
        | some t' => return .done t'
        | none => return .continue
      -- At the outer family the inhabitants are the outer family's, which
      -- the context gives; the read's own instance is this family's.
      let (family, nonempty?, τ) ← match induced family with
        | some (outer, row) =>
            let some τ' := substituteNTy row τ | return .continue
            pure (outer, none, τ')
        | none => pure (family, nonempty?, τ)
      match ← typedReadSpelling head family nonempty? τ rest with
      | some t' => return .done t'
      | none => return .continue
    let rebuilt : Option Lean.Expr := do
      let family ← args[0]?
      let (outer, row) ← induced family
      if head == ``VerexIR.Proofs.Denote.NTy.carrier ||
          head == ``VerexIR.Proofs.Denote.NTy.deepHolds then
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

/-- Whether a term and its respelling are definitionally equal, by `isDefEq` on
the smallest applications they differ in, each distinct pair once per target. A
family converts only under the application it instantiates. On the whole term,
`isDefEq` fails on that argument at every enclosing application and unfolds it
instead, which on a term of shared reads costs the term's unshared size. -/
private partial def convertsAtSites (left right : Lean.Expr) : MetaM Bool := do
  if left == right then return true
  let conversions := (← targetState).familyConversions
  if let some known := (← conversions.get)[(left, right)]? then return known
  let result ← match left, right with
    | .mdata _ inner, _ => convertsAtSites inner right
    | _, .mdata _ inner => convertsAtSites left inner
    | .lam name type body info, .lam _ type' body' _
    | .forallE name type body info, .forallE _ type' body' _ =>
      if ← convertsAtSites type type' <&&> withLocalDecl name info type fun local_ =>
          convertsAtSites (body.instantiate1 local_) (body'.instantiate1 local_) then
        pure true
      else isDefEq left right
    | .app .., .app .. =>
      let (arguments, arguments') := (left.getAppArgs, right.getAppArgs)
      let sameHead := left.getAppFn == right.getAppFn && arguments.size == arguments'.size
      if ← pure sameHead <&&> arguments.size.allM fun index _ =>
          convertsAtSites arguments[index]! arguments'[index]! then
        pure true
      else isDefEq left right
    | _, _ => isDefEq left right
  unless left.hasMVar || right.hasMVar do conversions.modify (·.insert (left, right) result)
  return result

/-- The canonical spelling of an expression's families, where it converts.
An expression the search left as it was is not searched again. Conversion uses
default transparency so a caller restricting simp transparency cannot suppress
a valid conversion of nominal types. -/
def canonicalFamilyResult (e : Lean.Expr) : MetaM Simp.Result := withDefault do
  if (← (← targetState).canonicalForms.get).contains e then return { expr := e }
  -- A pass takes off one level of induced families: a callee's callee at
  -- generic types is two levels below the caller.
  let mut canonical ← canonicalFamilies e
  if canonical == e then
    unless e.hasMVar do (← targetState).canonicalForms.modify (·.insert e)
    return { expr := e }
  unless ← convertsAtSites e canonical do return { expr := e }
  repeat
    let next ← canonicalFamilies canonical
    if next == canonical then break
    unless ← convertsAtSites canonical next do break
    canonical := next
  return { expr := canonical }

/-- An order projection carries its parameter sort and codec as implicit
arguments, and its fields' plain sorts below the arguments. Give these
projections their common Given spelling without evaluating the comparison. -/
dsimproc ↓ [vir_denote, vir_denote_norm] givenOrderProjection
    (@TypedOrder.order _ _ _ _ _ _) := fun e => withDefault do
  if e.hasLooseBVars then return .continue
  unless (e.find? fun expression => (givenOperationProjection? expression).isSome).isSome do
    return .continue
  let canonical ← Core.transform e (post := fun expression => do
    if let some original := givenOperationProjection? expression then return .done original
    return .continue)
  unless ← isDefEq e canonical do return .continue
  return .visit canonical

/-- A parameter field whose family already reads its plain values by identity
is the owner's plain projection. The identity is observed from this family;
an arbitrary parameter family retains its certification operation. -/
dsimproc ↓ [vir_denote, vir_denote_norm] plainParameterField
    (@VerexIR.Proofs.Denote.Var.selectCertified ?carriers _ _ _ _ _ _) := fun e => withDefault do
  if e.hasLooseBVars then return .continue
  let_expr Var.selectCertified carriers _source _arguments fields field position owner := e
    | return .continue
  unless field.isAppOfArity ``NTy.param 1 do return .continue
  let some (.lam _ _ body _) ← NativeMaps.canonicalPlainReader? carriers field
    | return .continue
  unless body == .bvar 0 do return .continue
  let row ← mkAppM ``Subtype.val #[owner]
  let projection := mkAppN (mkConst ``Var.selectPlain) #[carriers, fields, field, position, row]
  unless ← isDefEq e projection do return .continue
  return .visit (← canonicalFamilyResult projection).expr

/-- Observe the actual plain vector of a certified field or a value
recertified from its plain vector. Both consumers use the same forgetting
law; arbitrary vector families and constructions remain unobserved. -/
private def certifiedVectorObservation? (vector : Lean.Expr) :
    MetaM (Option (Lean.Expr × Lean.Expr × Lean.Expr × Lean.Expr)) := do
  let (carriers, element, forgotten) ← match_expr vector with
    | Var.selectCertified carriers source arguments fields vectorType position owner => do
        let_expr NTy.vector element := ← NativeMaps.nativeTypeHead vectorType | return none
        pure (carriers, element, mkAppN (mkConst ``Var.forget_selectCertified)
          #[carriers, source, arguments, fields, vectorType, position, owner])
    | NTy.ofDeepHolds carriers vectorType value held => do
        let_expr NTy.vector element := ← NativeMaps.nativeTypeHead vectorType | return none
        pure (carriers, element, mkAppN (mkConst ``NTy.forget_ofDeepHolds)
          #[carriers, vectorType, value, held])
    | _ => return none
  let some (_, _, plain) := (← inferType forgotten).eq? | return none
  return some (carriers, element, plain, forgotten)

/-- The plain row read from a certified vector field is the same bounded
read from its plain field. Bounds keep unrelated fallback inhabitants out of
this observation. -/
simproc ↓ [vir_denote_norm] selectedVectorPlainRead (@Subtype.val _ _ _) := fun e => withDefault do
  if e.hasLooseBVars then return .continue
  let_expr Subtype.val _ _ value := e | return .continue
  let_expr Option.getD _ lookup fallback := value | return .continue
  unless lookup.isAppOfArity ``GetElem?.getElem? 7 do return .continue
  let values := lookup.getArg! 5
  let_expr VerexIR.SpecVector.values _ vector := values | return .continue
  let some (carriers, element, plain, forgotten) ← certifiedVectorObservation? vector
    | return .continue
  unless fallback.isAppOfArity ``NTy.inhabitant 4 do return .continue
  unless ← isDefEq (fallback.getArg! 2) element do return .continue
  let plainDefault := mkApp3 (mkConst ``NTy.plainInhabitant) carriers element (fallback.getArg! 3)
  let index := lookup.getArg! 6
  let plainVector := (← Simp.simp plain).expr
  let plainValues ← mkAppM ``VerexIR.SpecVector.values #[plainVector]
  let premise ← mkAppM ``LT.lt #[index, ← mkAppM ``Array.size #[plainValues]]
  let canonicalPremise := (← canonicalFamilyResult premise).expr
  let some bounded ← (← Simp.getMethods).discharge? canonicalPremise | return .continue
  let bounded ← mkExpectedTypeHint bounded premise
  let proof := mkAppN (mkConst ``NTy.forget_vector_getD)
    #[carriers, element, vector, plain, forgotten, index, fallback, plainDefault, bounded]
  let some (_, lhs, rhs) := (← inferType proof).eq? | return .continue
  unless ← isDefEq lhs e do return .continue
  let rhs := (← canonicalFamilyResult rhs).expr
  return .visit { expr := rhs, proof? := some (← mkExpectedTypeHint proof (← mkEq e rhs)) }

/-- A certified vector read's case also states the corresponding plain
lookup. Keep the original case equation and its certified value available;
consumers of plain fields read this additional, canonical equation. -/
def assertPlainVectorLookup (goal : MVarId) (equation : Lean.Expr) : MetaM MVarId := goal.withContext do
  let some (_, lookup, result) := (← inferType equation).eq? | return goal
  unless lookup.isAppOfArity ``GetElem?.getElem? 7 do return goal
  let_expr VerexIR.SpecVector.values _ vector := lookup.getArg! 5 | return goal
  let some (carriers, element, plain, forgotten) ← certifiedVectorObservation? vector
    | return goal
  unless result.isAppOfArity ``Option.some 2 || result.isAppOfArity ``Option.none 1 do return goal
  let proof := mkAppN (mkConst ``NTy.forget_vector_lookup)
    #[carriers, element, vector, plain, forgotten, lookup.getArg! 6, result, equation]
  let some (_, left, right) := (← inferType proof).eq? | return goal
  let statement ← mkEq left (← whnfD right)
  let statement := (← canonicalFamilyResult statement).expr
  let proof ← mkExpectedTypeHint proof statement
  let (_, next) ← (← goal.assert `verexPlainRead statement proof).intro1P
  return next

elab_rules : tactic
  | `(tactic| verex_split_hypotheses) => splitHypotheses assertPlainVectorLookup

@[inherit_doc assertCertifiedClauses]
elab_rules : tactic
  | `(tactic| verex_certified_clauses) =>
      assertCertifiedClauses (fun statement => return (← canonicalFamilyResult statement).expr)

/-- Make the family spellings of the goals canonical, their targets too
where `target` holds (`canonicalFamilies`). -/
def canonicalFamiliesOf (target : Bool) (select : FVarId → Bool := fun _ => true) : TacticM Unit := do
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
      if decl.isImplementationDetail || !select decl.fvarId then continue
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

elab_rules : tactic
  | `(tactic| verex_canonical_hypotheses [$hypotheses,*]) => focusGoal do
      let selected ← withMainContext do
        hypotheses.getElems.mapM fun hypothesis => do
          let .fvar id ← instantiateMVars (← Lean.Elab.Term.elabTerm hypothesis none)
            | throwError "expected a local hypothesis"
          return id
      canonicalFamiliesOf false selected.contains

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
  let some substituted := substituteNTy row τ (some (← getEnv)) | return .continue
  let carriers := mkApp3 (mkConst ``VerexIR.Proofs.Denote.Skolems.toCarriers) unit functions outer
  let some transported ← transportAt? ``VerexIR.Proofs.Denote.NTy.toSkolem carriers
      #[θ, τ, mkApp5 e.getAppFn unit functions outer substituted value]
    | return .continue
  return .visit transported

/-- A value carried from an induced frame to the runtime family, as the
outer frame carries it (`Skolems.toRuntime_instantiate`, `ofRuntimeOuterFrame`). -/
dsimproc ↓ [vir_denote, vir_denote_norm] toRuntimeOuterFrame
    (@VerexIR.Proofs.Denote.Skolems.toRuntime _ ?functions ?frame _ _) := fun e => do
  let_expr VerexIR.Proofs.Denote.Skolems.toRuntime unit functions frame τ value := e
    | return .continue
  let some (θ, outer, row) := inducedFrame? frame | return .continue
  let some substituted := substituteNTy row τ (some (← getEnv)) | return .continue
  let carriers := mkApp3 (mkConst ``VerexIR.Proofs.Denote.Skolems.toCarriers) unit functions outer
  let some view ← transportAt? ``VerexIR.Proofs.Denote.NTy.ofSkolem carriers #[θ, τ, value]
    | return .continue
  return .visit (mkApp5 e.getAppFn unit functions outer substituted view)

/-- Whether a term rebuilds a value from its components: the value itself,
a certified integer from its value, or a pair of its first component and a
rebuild of its second, down to the unit a row ends with. -/
private partial def rebuilds (term value : Lean.Expr) : Bool :=
  term == value ||
    (term.isAppOfArity ``VerexIR.SpecInt.mk 4 &&
      (let first := term.getArg! 2
       first.isAppOfArity ``VerexIR.SpecInt.val 3 && first.appArg! == value)) ||
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
  let expanded ← if function.isAppOfArity ``VerexIR.Proofs.Denote.NTy.forget 2 then
      withTransparency .default (whnf function)
    else pure function
  let .lam name domain body info := expanded | return .continue
  unless rebuilds body (.bvar 0) do return .continue
  unless ← withTransparency .default (isDefEq (← inferType e) (← inferType array)) do
    return .continue
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
private def valueTransport? (function : Lean.Expr) : Option (Lean.Expr × Lean.Expr × Lean.Expr × Lean.Expr) :=
  let function := function.consumeMData
  let body := match function with
    | .lam _ _ body _ =>
        if body.isAppOfArity ``VerexIR.Proofs.Denote.NTy.toSkolem 5 && body.appArg! == Lean.mkBVar 0 then
          some body.appFn!
        else none
    | _ => some function
  match body with
  | some transport =>
      if transport.isAppOfArity ``VerexIR.Proofs.Denote.NTy.toSkolem 4 then
        some (transport.getArg! 0, transport.getArg! 1, transport.getArg! 2, transport.getArg! 3)
      else none
  | none => none

/-- A caller's values carried into a callee's view element by element are
the caller's values (`FamilyTransport.toSkolemIdentity`): what reading an
encoding through a transport states of encodings (`encodeTransport`), of
the values themselves. -/
simproc [vir_denote, vir_denote_norm] mapTransportIdentity (Array.map _ _) := fun e => do
  let_expr Array.map element _ function array := e | return .continue
  let some (outer, nonempty, θ, τ) := valueTransport? function | return .continue
  let some identity ← (try some <$> FamilyTransport.toSkolemIdentity outer nonempty θ τ element
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
  let some (outer, nonempty, θ, τ) := valueTransport? function | return .continue
  let some identity ← (try some <$> FamilyTransport.toSkolemIdentity outer nonempty θ τ element
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
  unless e.isAppOfArity head (4 + typeArguments) do return .continue
  let arguments := e.getAppArgs
  let types := arguments.extract 3 (3 + typeArguments)
  if types.any (fun type => (type.find? (·.isAppOf ``VerexIR.Proofs.Denote.NTy.param)).isSome) then
    return .continue
  let value := arguments.back!
  let transport := fun (held : Lean.Expr) => mkAppN e.getAppFn (arguments.pop.push held)
  let some identity ← (try some <$> FamilyTransport.pointwiseIdentity (← inferType value) transport
      catch _ => pure none) | return .continue
  let proof ← mkAppM ``congrFun #[identity, value]
  return .visit { expr := value, proof? := some (← mkExpectedTypeHint proof (← mkEq e value)) }

simproc [vir_denote_norm] paramFreeValueTransport
    (@VerexIR.Proofs.Denote.NTy.ofSkolem ?carriers _ _ _ _) :=
  paramFreeTransportStep ``VerexIR.Proofs.Denote.NTy.ofSkolem 1
simproc [vir_denote_norm] paramFreeVariantTransport
    (@VerexIR.Proofs.Denote.variantCarrier.ofSkolem ?carriers _ _ _ _ _) :=
  paramFreeTransportStep ``VerexIR.Proofs.Denote.variantCarrier.ofSkolem 2
simproc [vir_denote_norm] paramFreeRowTransport
    (@VerexIR.Proofs.Denote.HList.ofSkolem ?carriers _ _ _ _) :=
  paramFreeTransportStep ``VerexIR.Proofs.Denote.HList.ofSkolem 1

/-- A callee's value in the caller's view holds the variant it holds at the
callee's family (`variantPlainName_ofSkolem`). -/
simproc ↓ [vir_denote, vir_denote_norm] variantNameTransport
    (@VerexIR.Proofs.Denote.variantPlainName ?carriers _ _ _) :=
  fun e => do
    let_expr VerexIR.Proofs.Denote.variantPlainName carriers _ _ value := e | return .continue
    let_expr VerexIR.Proofs.Denote.Skolems.toCarriers unit functions outer := carriers.consumeMData
      | return .continue
    let_expr VerexIR.Proofs.Denote.variantPlain.ofSkolem _ _ θ names rows v := value
      | return .continue
    let proof := mkAppN (mkConst ``VerexIR.Proofs.Denote.variantPlainName_ofSkolem)
      #[unit, functions, outer, θ, names, rows, v]
    let some (_, _, name) := (← inferType proof).eq? | return .continue
    return .visit { expr := name, proof? := some proof }

/-- A variant observation (`variantName`, `variantPayload`) of a value at
the family a call induces, as the outer family spells it: the value is the
plain value at the outer family (`variantPlain.ofSkolem_heq`), whose
name is the one at the callee's (`variantPlainName_ofSkolem`). The rows
are substituted as spelled. -/
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
  let some substituted := substituteNRows row rows (some (← getEnv)) | return .continue
  let some (result, equation) ← (try withTransparency .default do
      let transported := mkAppN (mkConst transport) #[unit, functions, outer, θ, names, rows, value]
      let some (_, atOuter, _) := (← inferType transported).eq? | return none
      let ofSkolem := atOuter.appArg!
      let_expr VerexIR.Proofs.Denote.variantPlain.ofSkolem carriers nonempty _ _ _ _ := ofSkolem
        | return none
      let carrier ← inferType ofSkolem
      unless ← isDefEq carrier (← inferType value) do return none
      let heq ← mkAppOptM ``VerexIR.Proofs.Denote.variantPlain.ofSkolem_heq
        #[some carriers, some nonempty, some θ, some names, some rows, some value]
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
    (@VerexIR.Proofs.Denote.variantPlainName ?carriers _ _ _) :=
  variantOuterFamily ``VerexIR.Proofs.Denote.variantPlainName_ofSkolem

/-- Inspect named descriptor bodies as well as their explicit children. -/
private partial def containsStructQuoted (τ : Lean.Expr) : MetaM Bool := do
  let τView ← nativeTypeView τ
  if τView.isAppOf ``NTy.struct then return true
  τView.getAppArgs.anyM containsStructQuoted

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
  let structFree := !(← containsStructQuoted τ)
  unless handles.isConstOf ``Bool.false || structFree do return .continue
  let some row := substitutionRow? θ | return .continue
  let some substituted := substituteNTy row τ (some (← getEnv)) | return .continue
  let some (value, equation) ← (try withTransparency .default do
      let atFalse := mkAppN (mkConst ``VerexIR.Proofs.Denote.NTy.equal)
        #[family, mkConst ``Bool.false, τ, left, right]
      let first ← mkExpectedTypeHint (← mkEqRefl e) (← mkEq e atFalse)
      let nonempty ← if family.isAppOfArity ``VerexIR.Proofs.Denote.Carriers.instantiate 3 then
          pure (family.getArg! 2)
        else synthInstance (mkApp (mkConst ``VerexIR.Proofs.Denote.CarriersNonempty) outer)
      let transported ← mkAppOptM ``VerexIR.Proofs.Denote.NTy.equal_ofSkolem
        #[some outer, some nonempty, some θ, some τ, some left, some right]
      let view (value : Lean.Expr) : MetaM Lean.Expr := do
        mkAppOptM ``VerexIR.Proofs.Denote.NTy.ofSkolem
          #[some outer, some nonempty, some θ, some τ, some value]
      let value := mkAppN (mkConst ``VerexIR.Proofs.Denote.NTy.equal)
        #[outer, mkConst ``Bool.false, substituted, ← view left, ← view right]
      let proof ← mkEqTrans first transported
      pure (some (value, ← mkExpectedTypeHint proof (← mkEq e value)))
    catch _ => pure none)
    | return .continue
  return .visit { expr := value, proof? := some equation }

mutual
/-- Native equality at a type that compares a value by every part, directly:
none of its parts is a type parameter's or a function value's, compared by an
erasure, or a ghost slot, which equality ignores. -/
def NTy.comparesByValue : NTy → Bool
  | .unit | .bool | .int _ _ | .num | .address | .signer | .string | .bytes => true
  | .tuple row | .struct _ _ row => NRow.comparesByValue row
  | .enum _ _ _ rows _ => NRows.comparesByValue rows
  | .vector element => element.comparesByValue
  | .table key value => key.comparesByValue && value.comparesByValue
  | .ref referent => referent.comparesByValue
  | .ghost _ | .param _ | .function _ _ _ => false

def NRow.comparesByValue : NRow → Bool
  | .nil => true
  | .cons τ rest => τ.comparesByValue && NRow.comparesByValue rest

def NRows.comparesByValue : NRows → Bool
  | .nil => true
  | .cons row rest => NRow.comparesByValue row && NRows.comparesByValue rest
end

section EqualAsEquation
variable [Carriers]

private theorem listEqual_iff_eq {α : Type} (equal : α → α → Bool)
    (same : ∀ a b, equal a b = true ↔ a = b) (left right : List α) :
    listEqual equal left right = true ↔ left = right := by
  simpa using NativeMaps.listEqual_iff_map equal id (by simpa using same) left right

mutual
/-- At a type comparing by value, the equality of lent values is their
equation: a struct's and a variant's certified equality is this one of their
rows (`NTy.equal_iff_eq`), and so is a reference's current. -/
theorem NTy.plainEqual_iff_eq : (τ : NTy) → τ.comparesByValue = true → (left right : τ.plain) →
    (NTy.plainEqual false τ left right = true ↔ left = right)
  | .unit, _, _, _ => by simp [NTy.plainEqual, NTy.plainEqualOf]
  | .bool, _, _, _ => by simp [NTy.plainEqual, NTy.plainEqualOf]
  | .int _ _, _, left, right => by
      simp [NTy.plainEqual, NTy.plainEqualOf, SpecInt.eq_iff_val left right]
  | .num, _, left, right => by
      simp [NTy.plainEqual, NTy.plainEqualOf, SpecInt.eq_iff_val left right]
  | .address, _, _, _ => by simp [NTy.plainEqual, NTy.plainEqualOf]
  | .signer, _, _, _ => by simp [NTy.plainEqual, NTy.plainEqualOf]
  | .string, _, _, _ => by simp [NTy.plainEqual, NTy.plainEqualOf]
  | .bytes, _, _, _ => by simp [NTy.plainEqual, NTy.plainEqualOf]
  | .tuple row, plain, left, right => by
      show HListPlain.equal false row left right = true ↔ _
      exact HListPlain.equal_iff_eq row (by simpa [NTy.comparesByValue] using plain) left right
  | .struct _ _ row, plain, left, right => by
      show (if false && row.ownsHandle then HListPlain.headEqual false row left right
        else HListPlain.equal false row left right) = true ↔ _
      simp only [Bool.false_and, Bool.false_eq_true, ↓reduceIte]
      exact HListPlain.equal_iff_eq row (by simpa [NTy.comparesByValue] using plain) left right
  | .enum _ _ names rows _, plain, left, right => by
      show variantPlain.equal false names rows left right = true ↔ _
      exact variantPlain.equal_iff_eq names rows (by simpa [NTy.comparesByValue] using plain)
        left right
  | .vector element, plain, left, right => by
      show listEqual (NTy.plainEqual false element) left.values.toList right.values.toList = true ↔ _
      rw [listEqual_iff_eq _
        (NTy.plainEqual_iff_eq element (by simpa [NTy.comparesByValue] using plain))]
      exact ⟨fun same => SpecVector.ext (Array.toList_inj.mp same),
        fun same => congrArg (·.values.toList) same⟩
  | .table key value, plain, left, right => by
      have plains : key.comparesByValue = true ∧ value.comparesByValue = true := by
        simpa [NTy.comparesByValue] using plain
      show listEqual (fun l r => NTy.plainEqual false key l.1 r.1 &&
        NTy.plainEqual false value l.2 r.2) left.entries right.entries = true ↔ _
      rw [listEqual_iff_eq _ fun a b => by
        simp only [Bool.and_eq_true, NTy.plainEqual_iff_eq key plains.1,
          NTy.plainEqual_iff_eq value plains.2, Prod.ext_iff]]
      exact ⟨fun same => SpecEntries.ext same, fun same => congrArg (·.entries) same⟩
  | .ref referent, plain, left, right => by
      have plain : referent.comparesByValue = true := by simpa [NTy.comparesByValue] using plain
      show (NTy.plainEqual false referent left.1 right.1 &&
        NTy.plainEqual false referent left.2 right.2) = true ↔ _
      simp only [Bool.and_eq_true, NTy.plainEqual_iff_eq referent plain, Prod.ext_iff]
  | .ghost _, plain, _, _ | .param _, plain, _, _ | .function _ _ _, plain, _, _ => by
      simp [NTy.comparesByValue] at plain

/-- Rows of lent values comparing by value compare as their equation. -/
theorem HListPlain.equal_iff_eq : (row : NRow) → row.comparesByValue = true →
    (left right : HListPlain row) →
    (HListPlain.equal false row left right = true ↔ left = right)
  | .nil, _, _, _ => by
      show (true = true) ↔ _
      exact ⟨fun _ => rfl, fun _ => rfl⟩
  | .cons τ rest, plain, left, right => by
      have plains : τ.comparesByValue = true ∧ rest.comparesByValue = true := by
        simpa [NRow.comparesByValue] using plain
      show (NTy.plainEqual false τ left.1 right.1 &&
        HListPlain.equal false rest left.2 right.2) = true ↔ _
      simp only [Bool.and_eq_true, NTy.plainEqual_iff_eq τ plains.1,
        HListPlain.equal_iff_eq rest plains.2]
      exact ⟨fun same => Prod.ext same.1 same.2, fun same => same ▸ ⟨rfl, rfl⟩⟩

/-- Lent variant values comparing by value compare as their equation. -/
theorem variantPlain.equal_iff_eq : (names : List String) → (rows : NRows) →
    rows.comparesByValue = true → (left right : variantPlain names rows) →
    (variantPlain.equal false names rows left right = true ↔ left = right)
  | _, .nil, _, left, _ => nomatch left
  | [], .cons _ _, _, left, _ => nomatch left
  | _ :: _, .cons fields rest, plain, .inl left, .inl right => by
      have plains : fields.comparesByValue = true ∧ rest.comparesByValue = true := by
        simpa [NRows.comparesByValue] using plain
      show HListPlain.equal false fields left right = true ↔ _
      exact (HListPlain.equal_iff_eq fields plains.1 left right).trans
        ⟨congrArg Sum.inl, Sum.inl.inj⟩
  | _ :: _, .cons _ _, _, .inl _, .inr _ => by simp [variantPlain.equal, variantPlainOf.equal]
  | _ :: _, .cons _ _, _, .inr _, .inl _ => by simp [variantPlain.equal, variantPlainOf.equal]
  | _ :: names, .cons fields rest, plain, .inr left, .inr right => by
      have plains : fields.comparesByValue = true ∧ rest.comparesByValue = true := by
        simpa [NRows.comparesByValue] using plain
      show variantPlain.equal false names rest left right = true ↔ _
      exact (variantPlain.equal_iff_eq names rest plains.2 left right).trans
        ⟨congrArg Sum.inr, Sum.inr.inj⟩
end

mutual
/-- At a type comparing by value, native equality is the equation of the values. -/
theorem NTy.equal_iff_eq : (τ : NTy) → τ.comparesByValue = true → (left right : τ.carrier) →
    (NTy.equal false τ left right = true ↔ left = right)
  | .unit, _, _, _ => by simp [NTy.equal]
  | .bool, _, _, _ => by simp [NTy.equal]
  | .int _ _, _, left, right => by simp [NTy.equal, SpecInt.eq_iff_val left right]
  | .num, _, left, right => by simp [NTy.equal, SpecInt.eq_iff_val left right]
  | .address, _, _, _ => by simp [NTy.equal]
  | .signer, _, _, _ => by simp [NTy.equal]
  | .string, _, _, _ => by simp [NTy.equal]
  | .bytes, _, _, _ => by simp [NTy.equal]
  | .tuple row, plain, left, right => by
      simp only [NTy.equal]
      exact HList.equal_iff_eq row (by simpa [NTy.comparesByValue] using plain) left right
  | .struct _ _ row, plain, left, right => by
      simp only [NTy.equal, Bool.false_and, Bool.false_eq_true, ↓reduceIte]
      rw [HListPlain.equal_iff_eq row (by simpa [NTy.comparesByValue] using plain)
        left.val right.val]
      exact Subtype.ext_iff.symm
  | .enum _ _ names rows _, plain, left, right => by
      simp only [NTy.equal]
      rw [variantPlain.equal_iff_eq names rows (by simpa [NTy.comparesByValue] using plain)
        left.val right.val]
      exact Subtype.ext_iff.symm
  | .vector element, plain, left, right => by
      simp only [NTy.equal]
      rw [listEqual_iff_eq _
        (NTy.equal_iff_eq element (by simpa [NTy.comparesByValue] using plain))]
      exact ⟨fun same => SpecVector.ext (Array.toList_inj.mp same),
        fun same => congrArg (·.values.toList) same⟩
  | .table key value, plain, left, right => by
      have plains : key.comparesByValue = true ∧ value.comparesByValue = true := by
        simpa [NTy.comparesByValue] using plain
      simp only [NTy.equal]
      rw [listEqual_iff_eq _ fun a b => by
        simp only [Bool.and_eq_true, NTy.equal_iff_eq key plains.1,
          NTy.equal_iff_eq value plains.2, Prod.ext_iff]]
      exact ⟨fun same => SpecEntries.ext same, fun same => congrArg (·.entries) same⟩
  | .ref referent, plain, left, right => by
      have plain : referent.comparesByValue = true := by simpa [NTy.comparesByValue] using plain
      simp only [NTy.equal, Bool.and_eq_true, NTy.plainEqual_iff_eq referent plain,
        NTy.equal_iff_eq referent plain, Prod.ext_iff]
  | .ghost _, plain, _, _ | .param _, plain, _, _ | .function _ _ _, plain, _, _ => by
      simp [NTy.comparesByValue] at plain

/-- Rows comparing by value compare as the equation of the rows. -/
theorem HList.equal_iff_eq : (row : NRow) → row.comparesByValue = true →
    (left right : HList row) → (HList.equal false row left right = true ↔ left = right)
  | .nil, _, _, _ => by simp only [HList.equal]; exact ⟨fun _ => rfl, fun _ => trivial⟩
  | .cons τ rest, plain, left, right => by
      have plains : τ.comparesByValue = true ∧ rest.comparesByValue = true := by
        simpa [NRow.comparesByValue] using plain
      simp only [HList.equal, Bool.and_eq_true, NTy.equal_iff_eq τ plains.1,
        HList.equal_iff_eq rest plains.2]
      exact ⟨fun same => Prod.ext same.1 same.2, fun same => same ▸ ⟨rfl, rfl⟩⟩

/-- Variants comparing by value compare as the equation of the variants. -/
theorem variantCarrier.equal_iff_eq : (names : List String) → (rows : NRows) →
    rows.comparesByValue = true → (left right : variantCarrier names rows) →
    (variantCarrier.equal false names rows left right = true ↔ left = right)
  | _, .nil, _, left, _ => nomatch left
  | [], .cons _ _, _, left, _ => nomatch left
  | _ :: _, .cons fields rest, plain, .inl left, .inl right => by
      have plains : fields.comparesByValue = true ∧ rest.comparesByValue = true := by
        simpa [NRows.comparesByValue] using plain
      simp only [variantCarrier.equal]
      exact (HList.equal_iff_eq fields plains.1 left right).trans
        ⟨congrArg Sum.inl, Sum.inl.inj⟩
  | _ :: _, .cons _ _, _, .inl _, .inr _ => by simp [variantCarrier.equal]
  | _ :: _, .cons _ _, _, .inr _, .inl _ => by simp [variantCarrier.equal]
  | _ :: names, .cons fields rest, plain, .inr left, .inr right => by
      have plains : fields.comparesByValue = true ∧ rest.comparesByValue = true := by
        simpa [NRows.comparesByValue] using plain
      simp only [variantCarrier.equal]
      exact (variantCarrier.equal_iff_eq names rest plains.2 left right).trans
        ⟨congrArg Sum.inr, Sum.inr.inj⟩
end

/-- `NTy.equal_iff_eq` as a Boolean, in the form of native equality at a scalar. -/
theorem NTy.equal_eq_decide (τ : NTy) (plain : τ.comparesByValue = true) (left right : τ.carrier) :
    NTy.equal false τ left right = @decide (left = right) (Classical.propDecidable _) := by
  rw [Bool.eq_iff_iff, NTy.equal_iff_eq τ plain]
  simp only [decide_eq_true_eq]

end EqualAsEquation

/-- Whether a quoted type compares by value, read off its constructors; a type
not literal to its leaves does not count. -/
private partial def comparesByValueQuoted (τ : Lean.Expr) : MetaM Bool := do
  let τ ← nativeTypeView τ
  match τ.getAppFn.constName?, τ.getAppArgs with
  | some ``NTy.unit, _ | some ``NTy.bool, _ | some ``NTy.int, _ | some ``NTy.num, _
  | some ``NTy.address, _ | some ``NTy.signer, _ | some ``NTy.string, _ | some ``NTy.bytes, _ => pure true
  | some ``NTy.tuple, #[elements] => rowCompares elements
  | some ``NTy.struct, #[_, _, fields] => rowCompares fields
  | some ``NTy.enum, #[_, _, _, rows, _] => rowsCompare rows
  | some ``NTy.vector, #[element] => comparesByValueQuoted element
  | some ``NTy.table, #[key, value] => comparesByValueQuoted key <&&> comparesByValueQuoted value
  | some ``NTy.ref, #[referent] => comparesByValueQuoted referent
  | _, _ => pure false
where
  rowCompares (row : Lean.Expr) : MetaM Bool := do
    let row := row.consumeMData
    match row.getAppFn.constName?, row.getAppArgs with
    | some ``NRow.nil, _ => pure true
    | some ``NRow.cons, #[τ, rest] => comparesByValueQuoted τ <&&> rowCompares rest
    | _, _ => pure false
  rowsCompare (rows : Lean.Expr) : MetaM Bool := do
    let rows := rows.consumeMData
    match rows.getAppFn.constName?, rows.getAppArgs with
    | some ``NRows.nil, _ => pure true
    | some ``NRows.cons, #[row, rest] => rowCompares row <&&> rowsCompare rest
    | _, _ => pure false

/-- Native equality between values, one of them not built, at a compound type
comparing by value: the equation of the values, which rewrites where a
comparison of an opaque value by its parts is stuck. Built values compare part
by part (`HList.equal_cons`, `variantCarrier.equal_inr_inr`). -/
simproc ↓ [vir_denote_norm] equalAsEquation
    (@VerexIR.Proofs.Denote.NTy.equal ?carriers _ _ _ _) := fun e => do
  let e := e.consumeMData
  let_expr VerexIR.Proofs.Denote.NTy.equal family handles τ left right := e | return .continue
  unless handles.isConstOf ``Bool.false do return .continue
  if (inducedCarriers? family.consumeMData).isSome then return .continue
  let τView ← nativeTypeView τ
  if τView.getAppFn.constName?.any (· matches ``NTy.unit | ``NTy.bool | ``NTy.int | ``NTy.num
      | ``NTy.address | ``NTy.signer | ``NTy.string | ``NTy.bytes) then return .continue
  unless ← comparesByValueQuoted τ do return .continue
  let built (value : Lean.Expr) : MetaM Bool :=
    return (← isConstructorApp? value.consumeMData).isSome
  if ← (built left <&&> built right) then return .continue
  let plain ← mkExpectedTypeHint (mkApp2 (mkConst ``Eq.refl [1]) (mkConst ``Bool) (mkConst ``Bool.true))
    (← mkEq (mkApp (mkConst ``NTy.comparesByValue) τ) (mkConst ``Bool.true))
  let proof := mkAppN (mkConst ``NTy.equal_eq_decide) #[family, τ, plain, left, right]
  let some (_, _, value) := (← inferType proof).eq? | return .continue
  return .visit { expr := value, proof? := some proof }

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
    (@VerexIR.Proofs.Denote.variantPlainName ?carriers _ _ _) := fun e => do
  let_expr VerexIR.Proofs.Denote.variantPlainName _ names _ value := e | return .continue
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

/-- The row a certified value certifies: a built one is the row under its
certificate (`Subtype.mk`, `NTy.ofDeepHolds`), and any other is read
through `.val`; a value that carries no certificate is itself
(`doc/designs/data-invariants.md`). -/
private def certifiedRow (value : Lean.Expr) : MetaM Lean.Expr := do
  let value := value.consumeMData
  if value.isAppOfArity ``Subtype.mk 4 then return (value.getArg! 2).consumeMData
  if value.isAppOfArity ``VerexIR.Proofs.Denote.NTy.ofDeepHolds 4 then
    return (value.getArg! 2).consumeMData
  if (← whnfD (← inferType value)).isAppOfArity ``Subtype 2 then
    return ← mkAppM ``Subtype.val #[value]
  return value

/-- The components of a value of a product carrier: a pair's, or its
projections. -/
private def pairComponents (value : Lean.Expr) : MetaM (Lean.Expr × Lean.Expr) := do
  let value ← certifiedRow value
  if value.isAppOfArity ``Prod.mk 4 then return (value.getArg! 2, value.getArg! 3)
  -- Keep the same projection spelling as clause and write-back producers.
  -- Infer the product parameters from this row rather than re-quoting them:
  -- a nominal owner's fields may carry a different, convertible spelling.
  return (← mkAppM ``Prod.fst #[value], ← mkAppM ``Prod.snd #[value])

/-- The injections a constructed enum value is made of: its variant's
position and payload. -/
private def injections? (value : Lean.Expr) : MetaM (Option (Nat × Lean.Expr)) := do
  let mut position := 0
  let mut current ← certifiedRow value
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

/-- The family a transport's type arguments induce over the family it
carries.  `Carriers.instantiate` takes the outer family's inhabitants as
well, so a family assembled without them is a function into `Carriers` and
the kernel rejects every type it was put in (`NTy.carrier inner τ`,
`HListPlain inner fields`).  A transport carries that instance beside its
family, so the induced family is spelled from the matched term's own pair
here and nowhere else; a step with no term to read it from synthesizes the
instance instead and declines where there is none (`transportAt?`). -/
private def inducedFamily (outer nonempty θ : Lean.Expr) : Lean.Expr :=
  mkApp3 (mkConst ``VerexIR.Proofs.Denote.Carriers.instantiate) θ outer nonempty

/-- A constructed enum value transported to a callee's family, injection by
injection (`variantCarrier.toSkolem_inl`, `variantCarrier.toSkolem_inr`)
definitionally. -/
dsimproc [vir_denote, vir_denote_norm] variantToSkolemConstructor
    (@VerexIR.Proofs.Denote.variantCarrier.toSkolem ?outer _ _ _ _ _) := fun e => do
  let e := e.consumeMData
  let_expr VerexIR.Proofs.Denote.variantCarrier.toSkolem outer nonempty θ names rows value := e
    | return .continue
  let some (position, payload) ← injections? value | return .continue
  let inner := inducedFamily outer nonempty θ
  -- The variants from the innermost injection outwards.
  let mut levels : Array (Lean.Expr × Lean.Expr × Lean.Expr) := #[]
  for i in [:position + 1] do
    let some level ← variantAt? names rows i | return .continue
    levels := levels.push level
  let some (_, _, fields) := levels[position]? | return .continue
  let mut built := mkAppN (mkConst ``VerexIR.Proofs.Denote.HList.toSkolem)
    #[outer, nonempty, θ, fields, payload]
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
    (@VerexIR.Proofs.Denote.HList.toSkolem ?outer _ _ _ _) := fun e => do
  let e := e.consumeMData
  let_expr VerexIR.Proofs.Denote.HList.toSkolem outer nonempty θ row values := e
    | return .continue
  let row ← rowConstructor row
  if row.isConstOf ``VerexIR.Proofs.Denote.NRow.nil then
    return .done (mkConst ``Unit.unit)
  let_expr VerexIR.Proofs.Denote.NRow.cons τ rest := row | return .continue
  let inner := inducedFamily outer nonempty θ
  let (head, tail) ← pairComponents values
  let headType := mkApp2 (mkConst ``VerexIR.Proofs.Denote.NTy.carrier) inner τ
  let tailType := mkApp2 (mkConst ``VerexIR.Proofs.Denote.HList) inner rest
  return .visit (mkApp4 (mkConst ``Prod.mk [Level.zero, Level.zero]) headType tailType
    (mkAppN (mkConst ``VerexIR.Proofs.Denote.NTy.toSkolem) #[outer, nonempty, θ, τ, head])
    (mkAppN (mkConst ``VerexIR.Proofs.Denote.HList.toSkolem)
      #[outer, nonempty, θ, rest, tail]))

/-- A constructed lent variant value transported to a callee's family,
injection by injection (`variantPlain.toSkolem_inl`, `_inr`)
definitionally: a struct's row and a variant's value cross a call as a
certified value does. -/
dsimproc [vir_denote, vir_denote_norm] variantPlainToSkolemConstructor
    (@VerexIR.Proofs.Denote.variantPlain.toSkolem ?outer _ _ _ _ _) := fun e => do
  let e := e.consumeMData
  let_expr VerexIR.Proofs.Denote.variantPlain.toSkolem outer nonempty θ names rows value := e
    | return .continue
  let some (position, payload) ← injections? value | return .continue
  let inner := inducedFamily outer nonempty θ
  let mut levels : Array (Lean.Expr × Lean.Expr × Lean.Expr) := #[]
  for i in [:position + 1] do
    let some level ← variantAt? names rows i | return .continue
    levels := levels.push level
  let some (_, _, fields) := levels[position]? | return .continue
  let mut built := mkAppN (mkConst ``VerexIR.Proofs.Denote.HListPlain.toSkolem)
    #[outer, nonempty, θ, fields, payload]
  for i in (List.range (position + 1)).reverse do
    let some (names, rows, fields) := levels[i]? | return .continue
    let_expr VerexIR.Proofs.Denote.NRows.cons _ rest := rows | return .continue
    let plain := mkApp (mkConst ``VerexIR.Proofs.Denote.Carriers.toPlain) inner
    let left := mkApp2 (mkConst ``VerexIR.Proofs.Denote.HListPlainOf) plain fields
    let right := mkApp3 (mkConst ``VerexIR.Proofs.Denote.variantPlainOf) plain
      (names.getArg! 2) rest
    built := if i == position then
      mkApp3 (mkConst ``Sum.inl [Level.zero, Level.zero]) left right built
    else mkApp3 (mkConst ``Sum.inr [Level.zero, Level.zero]) left right built
  return .visit built

/-- A row of lent values transported to a callee's family, component by
component.  The components' types are the canonical spelling of the plain
layer (`PlainSpelling`), the one a leaf's closure keeps. -/
dsimproc [vir_denote, vir_denote_norm] hlistPlainToSkolemConstructor
    (@VerexIR.Proofs.Denote.HListPlain.toSkolem ?outer _ _ _ _) := fun e => do
  let e := e.consumeMData
  let_expr VerexIR.Proofs.Denote.HListPlain.toSkolem outer nonempty θ row values := e
    | return .continue
  let row ← rowConstructor row
  if row.isConstOf ``VerexIR.Proofs.Denote.NRow.nil then
    return .done (mkConst ``Unit.unit)
  let_expr VerexIR.Proofs.Denote.NRow.cons τ rest := row | return .continue
  let inner := inducedFamily outer nonempty θ
  let (head, tail) ← pairComponents values
  let plain := mkApp (mkConst ``VerexIR.Proofs.Denote.Carriers.toPlain) inner
  let headType := mkApp2 (mkConst ``VerexIR.Proofs.Denote.NTy.plainOf) plain τ
  let tailType := mkApp2 (mkConst ``VerexIR.Proofs.Denote.HListPlainOf) plain rest
  return .visit (mkApp4 (mkConst ``Prod.mk [Level.zero, Level.zero]) headType tailType
    (mkAppN (mkConst ``VerexIR.Proofs.Denote.NTy.plainToSkolem) #[outer, nonempty, θ, τ, head])
    (mkAppN (mkConst ``VerexIR.Proofs.Denote.HListPlain.toSkolem)
      #[outer, nonempty, θ, rest, tail]))

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
    (@VerexIR.Proofs.Denote.NTy.ofSkolem ?outer _ _ _ _) := fun e => do
  let e := e.consumeMData
  let_expr VerexIR.Proofs.Denote.NTy.ofSkolem outer nonempty θ τ value := e
    | return .continue
  transportIdentity ``VerexIR.Proofs.Denote.NTy.ofSkolem_heq #[outer, nonempty, θ, τ] value e
simproc ↓ [vir_denote, vir_denote_norm] toSkolemIdentity
    (@VerexIR.Proofs.Denote.NTy.toSkolem ?outer _ _ _ _) := fun e => do
  let e := e.consumeMData
  let_expr VerexIR.Proofs.Denote.NTy.toSkolem outer nonempty θ τ value := e
    | return .continue
  transportIdentity ``VerexIR.Proofs.Denote.NTy.toSkolem_heq #[outer, nonempty, θ, τ] value e
simproc ↓ [vir_denote, vir_denote_norm] hlistOfSkolemIdentity
    (@VerexIR.Proofs.Denote.HList.ofSkolem ?outer _ _ _ _) := fun e => do
  let e := e.consumeMData
  let_expr VerexIR.Proofs.Denote.HList.ofSkolem outer nonempty θ row values := e
    | return .continue
  transportIdentity ``VerexIR.Proofs.Denote.HList.ofSkolem_heq #[outer, nonempty, θ, row] values e
simproc ↓ [vir_denote, vir_denote_norm] hlistToSkolemIdentity
    (@VerexIR.Proofs.Denote.HList.toSkolem ?outer _ _ _ _) := fun e => do
  let e := e.consumeMData
  let_expr VerexIR.Proofs.Denote.HList.toSkolem outer nonempty θ row values := e
    | return .continue
  transportIdentity ``VerexIR.Proofs.Denote.HList.toSkolem_heq #[outer, nonempty, θ, row] values e
simproc ↓ [vir_denote, vir_denote_norm] variantOfSkolemIdentity
    (@VerexIR.Proofs.Denote.variantCarrier.ofSkolem ?outer _ _ _ _ _) := fun e => do
  let e := e.consumeMData
  let_expr VerexIR.Proofs.Denote.variantCarrier.ofSkolem outer nonempty θ names rows value := e
    | return .continue
  transportIdentity ``VerexIR.Proofs.Denote.variantCarrier.ofSkolem_heq #[outer, nonempty, θ, names, rows] value e
simproc ↓ [vir_denote, vir_denote_norm] variantToSkolemIdentity
    (@VerexIR.Proofs.Denote.variantCarrier.toSkolem ?outer _ _ _ _ _) := fun e => do
  let e := e.consumeMData
  let_expr VerexIR.Proofs.Denote.variantCarrier.toSkolem outer nonempty θ names rows value := e
    | return .continue
  transportIdentity ``VerexIR.Proofs.Denote.variantCarrier.toSkolem_heq #[outer, nonempty, θ, names, rows] value e
simproc ↓ [vir_denote, vir_denote_norm] plainOfSkolemIdentity
    (@VerexIR.Proofs.Denote.NTy.plainOfSkolem ?outer _ _ _ _) := fun e => do
  let e := e.consumeMData
  let_expr VerexIR.Proofs.Denote.NTy.plainOfSkolem outer nonempty θ τ value := e
    | return .continue
  transportIdentity ``VerexIR.Proofs.Denote.NTy.plainOfSkolem_heq #[outer, nonempty, θ, τ] value e
simproc ↓ [vir_denote, vir_denote_norm] plainToSkolemIdentity
    (@VerexIR.Proofs.Denote.NTy.plainToSkolem ?outer _ _ _ _) := fun e => do
  let e := e.consumeMData
  let_expr VerexIR.Proofs.Denote.NTy.plainToSkolem outer nonempty θ τ value := e
    | return .continue
  transportIdentity ``VerexIR.Proofs.Denote.NTy.plainToSkolem_heq #[outer, nonempty, θ, τ] value e
simproc ↓ [vir_denote, vir_denote_norm] hlistPlainOfSkolemIdentity
    (@VerexIR.Proofs.Denote.HListPlain.ofSkolem ?outer _ _ _ _) := fun e => do
  let e := e.consumeMData
  let_expr VerexIR.Proofs.Denote.HListPlain.ofSkolem outer nonempty θ row values := e
    | return .continue
  transportIdentity ``VerexIR.Proofs.Denote.HListPlain.ofSkolem_heq #[outer, nonempty, θ, row] values e
simproc ↓ [vir_denote, vir_denote_norm] hlistPlainToSkolemIdentity
    (@VerexIR.Proofs.Denote.HListPlain.toSkolem ?outer _ _ _ _) := fun e => do
  let e := e.consumeMData
  let_expr VerexIR.Proofs.Denote.HListPlain.toSkolem outer nonempty θ row values := e
    | return .continue
  transportIdentity ``VerexIR.Proofs.Denote.HListPlain.toSkolem_heq #[outer, nonempty, θ, row] values e
simproc ↓ [vir_denote, vir_denote_norm] variantPlainOfSkolemIdentity
    (@VerexIR.Proofs.Denote.variantPlain.ofSkolem ?outer _ _ _ _ _) := fun e => do
  let e := e.consumeMData
  let_expr VerexIR.Proofs.Denote.variantPlain.ofSkolem outer nonempty θ names rows value := e
    | return .continue
  transportIdentity ``VerexIR.Proofs.Denote.variantPlain.ofSkolem_heq #[outer, nonempty, θ, names, rows] value e
simproc ↓ [vir_denote, vir_denote_norm] variantPlainToSkolemIdentity
    (@VerexIR.Proofs.Denote.variantPlain.toSkolem ?outer _ _ _ _ _) := fun e => do
  let e := e.consumeMData
  let_expr VerexIR.Proofs.Denote.variantPlain.toSkolem outer nonempty θ names rows value := e
    | return .continue
  transportIdentity ``VerexIR.Proofs.Denote.variantPlain.toSkolem_heq #[outer, nonempty, θ, names, rows] value e

/-- The default of a type at the family a call induces, as the outer family
spells it: a typed map's reads stated by a callee then name the map's own
instance. The default is irreducible, so the two spellings are equal by the
lemma (`FamilyTransport.inhabitantOf_instantiate_heq`), not by unfolding;
and the lemma is heterogeneous, because the two defaults share a type only
where the carrier does not read the family, which is checked here. The
alternative — exchanging the families in the term and calling the result a
conversion — states what does not hold in a bare family position. -/
simproc ↓ [vir_denote_norm] inhabitantOuterFamily
    (@VerexIR.Proofs.Denote.NTy.inhabitantOf ?family _ _ _) := fun e => do
  let e := e.consumeMData
  let_expr VerexIR.Proofs.Denote.NTy.inhabitantOf family _nonempty τ inhabitable := e
    | return .continue
  if e.hasLooseBVars then return .continue
  let some (θ, outer) := inducedCarriers? family.consumeMData | return .continue
  let some row := substitutionRow? θ | return .continue
  let some substituted := substituteNTy row τ (some (← getEnv)) | return .continue
  let some (value, equation) ← (try withTransparency .default do
      -- The outer family's own inhabitants: the default belongs to the
      -- family it is read from (`respelledOperation?`).
      let some nonempty ←
          synthInstance? (mkApp (mkConst ``VerexIR.Proofs.Denote.CarriersNonempty) outer)
        | return none
      let inhabitable' := mkApp (mkConst ``VerexIR.Proofs.Denote.NTy.inhabitable) substituted
      unless (← whnf inhabitable').isConstOf ``Bool.true do return none
      let decided ← mkExpectedTypeHint (← mkEqRefl (mkConst ``Bool.true))
        (← mkEq inhabitable' (mkConst ``Bool.true))
      let value := mkApp4 (mkConst ``VerexIR.Proofs.Denote.NTy.inhabitantOf) outer nonempty
        substituted decided
      let type ← inferType e
      unless ← isDefEq type (← inferType value) do return none
      let heq ← mkAppOptM
        ``VerexIR.Proofs.Denote.FamilyTransport.inhabitantOf_instantiate_heq
        #[some outer, none, some θ, some τ, some inhabitable, some decided]
      let proof ← mkAppOptM ``eq_of_heq #[some type, some e, some value, some heq]
      pure (some (value, proof))
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
call induces and the defaults are equal by
`FamilyTransport.inhabitantOf_instantiate_heq`, which the instance position
hides from rewriting. -/
private def ownInstance? (inhabited : Lean.Expr) (view : NativeMaps.TypedView) :
    MetaM (Option (Option Lean.Expr)) := do
  -- Unfolding the two instances compares their types' codecs; only an
  -- instance at an induced family is related by the lemma, any other by
  -- reducible unfolding.
  let induced? : Option (Lean.Expr × Lean.Expr × Lean.Expr) := do
    let_expr VerexIR.Proofs.Denote.nativeInhabited _ _ fallback := inhabited | none
    let_expr VerexIR.Proofs.Denote.NTy.inhabitantOf family _ τ inhabitable := fallback | none
    let _ ← inducedCarriers? family
    some (family, τ, inhabitable)
  try withTransparency .default do
    let some (family, τ, inhabitable) := induced?
      | if ← withReducible (isDefEq inhabited view.inhabited) then return some none else return none
    let_expr VerexIR.Proofs.Denote.nativeInhabited outer substituted own := view.inhabited
      | return none
    let_expr VerexIR.Proofs.Denote.NTy.inhabitantOf ownFamily _ _ _ := own | return none
    let fallback := inhabited.appArg!
    -- Each level of induced families by the lemma, down to the map's own.
    let mut family := family
    let mut τ := τ
    let mut inhabitable := inhabitable
    let mut chain : Option Lean.Expr := none
    repeat
      let some (θ, below) := inducedCarriers? family | return none
      let some row := substitutionRow? θ | return none
      let some next := substituteNTy row τ (some (← getEnv)) | return none
      let decided ← mkExpectedTypeHint (← mkEqRefl (mkConst ``Bool.true))
        (← mkEq (mkApp (mkConst ``VerexIR.Proofs.Denote.NTy.inhabitable) next)
          (mkConst ``Bool.true))
      let nonempty ← if family.isAppOfArity ``VerexIR.Proofs.Denote.Carriers.instantiate 3 then
          pure (family.getArg! 2)
        else synthInstance (mkApp (mkConst ``VerexIR.Proofs.Denote.CarriersNonempty) below)
      let heq ← mkAppOptM ``VerexIR.Proofs.Denote.FamilyTransport.inhabitantOf_instantiate_heq
        #[some below, some nonempty, some θ, some τ, some inhabitable, some decided]
      chain ← match chain with
        | some known => pure (some (← mkHEqTrans known heq))
        | none => pure (some heq)
      family := below
      τ := next
      inhabitable := decided
      if below == ownFamily then break
    let some equal := chain | return none
    let carrier ← inferType own
    let defaults ← mkAppOptM ``eq_of_heq #[some carrier, some fallback, some own, some equal]
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
    | .app (.app (.app (.const ``VerexIR.Proofs.Denote.nativeInhabited _) _) _) fallback =>
        match fallback.consumeMData with
        | .app (.app (.app (.app (.const ``VerexIR.Proofs.Denote.NTy.inhabitantOf _) family) _) _) _ =>
            (inducedCarriers? family).isSome
        | _ => false
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
    values := (← pairComponents values).2
  unless selector.isAppOfArity ``VerexIR.Proofs.Denote.Var.here 2 do return .continue
  return .visit (← pairComponents values).1

/-- A choice's projection of a constructed enum value, by the choice's and
the value's constructors (`Which.project?`) definitionally: the payload
where the injections agree, nothing otherwise. -/
dsimproc [vir_denote, vir_denote_norm] whichProjectConstructor
    (@VerexIR.Proofs.Denote.Which.project? ?carriers _ _ _ _ _) := fun e => do
  let e := e.consumeMData
  let_expr VerexIR.Proofs.Denote.Which.project? _ _ _ _ choice value := e
    | return .continue
  let mut depth := 0
  let mut choice := choice.consumeMData
  while choice.isAppOfArity ``VerexIR.Proofs.Denote.Which.there 6 do
    depth := depth + 1
    choice := (choice.getArg! 5).consumeMData
  unless choice.isAppOfArity ``VerexIR.Proofs.Denote.Which.here 4 do return .continue
  let some (position, payload) ← injections? value | return .continue
  -- The row's type as the projection states it, so that the lemmas on
  -- the option match the context's spelling.
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
    -- The value inside the case: the rest of the variants, where the value is
    -- not the first, or the plain side of a certificate, which stands between
    -- an enum's value and its variants (`NTy.carrier`) and which the
    -- recursion looks past to the sum.  A certificate itself is a field of
    -- the case too, and is not what the projections read.
    let mut inside := none
    for field in subgoal.fields do
      unless field.isFVar do continue
      let shape ← subgoal.mvarId.withContext do whnfD (← inferType field)
      if shape.isAppOfArity ``Sum 2 || shape.isAppOfArity ``Subtype 2 then
        inside := some field.fvarId!
        break
    let some value := inside | return false
    unless ← projectionsAgree subgoal.mvarId value do return false
  return true

open Lean Meta Simp in
/-- A choice's projection at a frame type arguments induce, at the outer
frame with the types substituted: equal, case by case on the value, though
not convertible, since the projection matches on the choice and the value
together, which keeps it an atom where the value is unknown. A step proof's
leaf reads a callee's results so (`verex_step_obligation`); the automatic
normal form keeps the callee's spelling. -/
simproc_decl whichProjectFamily
    (@VerexIR.Proofs.Denote.Which.project? ?carriers _ _ _ _ _) := fun e => do
  let e := e.consumeMData
  unless e.isAppOfArity ``VerexIR.Proofs.Denote.Which.project? 6 && !e.hasLooseBVars do
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

open Lean Meta Simp in
/-- A certified selection at a frame type arguments induce, at the outer
frame with the types substituted: equal, case by case on the value, as a
plain projection is (`whichProjectFamily`), and not convertible for the same
reason — the projection inside it matches on the choice and the value
together, and a position witness's rows are data, which no transparency
respells.

In the normal form, not only in a step proof, because the reading that
collapses a field whose type carries no certificate names that type
(`Choices.selectCertified?_single_bool`), and a generic callee's clause
spells the field as the type parameter it is at the induced frame.  The
caller's leaf reads the value's plain side; until the selection is spelled
at the caller's frame no equation of a certificate-free type can reach it. -/
simproc ↓ [vir_denote, vir_denote_norm] choicesSelectCertifiedFamily
    (@VerexIR.Proofs.Denote.Choices.selectCertified? ?carriers _ _ _ _ _ _ _ _) :=
    fun e => do
  let e := e.consumeMData
  unless e.isAppOfArity ``VerexIR.Proofs.Denote.Choices.selectCertified? 9 &&
      !e.hasLooseBVars do
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
    (Option.map _ (@VerexIR.Proofs.Denote.Which.project? ?carriers _ _ _ _ _)) := fun e => do
  let e := e.consumeMData
  let_expr Option.map _ β function projection := e | return .continue
  let_expr VerexIR.Proofs.Denote.Which.project? _ _ _ _ choice value := projection
    | return .continue
  let mut depth := 0
  let mut choice := choice.consumeMData
  while choice.isAppOfArity ``VerexIR.Proofs.Denote.Which.there 6 do
    depth := depth + 1
    choice := (choice.getArg! 5).consumeMData
  unless choice.isAppOfArity ``VerexIR.Proofs.Denote.Which.here 4 do return .continue
  let some (position, payload) ← injections? value | return .continue
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

/-- The components of a side of an equation at `carrier`, a product: a pair's,
or its reads, as the normal form spells them. -/
private def rowComponents (carrier side : Lean.Expr) : Lean.Expr × Lean.Expr :=
  if side.isAppOfArity ``Prod.mk 4 then (side.getArg! 2, side.getArg! 3)
  else
    let levels := carrier.getAppFn.constLevels!
    let (α, β) := (carrier.getArg! 0, carrier.getArg! 1)
    (mkApp3 (mkConst ``Prod.fst levels) α β side, mkApp3 (mkConst ``Prod.snd levels) α β side)

/-- What a type spells of a row of values: the unit, or the product it
unfolds to. -/
private inductive RowSpelling where
  | unit
  | pair (carrier : Lean.Expr)

/-- The row a type spells, read from the spelling alone: `Prod α β`, `Unit`,
`HList row` and `NTy.carrier _ (.tuple row)`. The row needs no more than
reducible unfolding; any other type is not a row here. -/
private def rowSpelling? (spelled : Lean.Expr) : MetaM (Option RowSpelling) := do
  if spelled.isAppOfArity ``Prod 2 then return some (.pair spelled)
  if spelled.isConstOf ``Unit || spelled.isAppOf ``PUnit then return some .unit
  -- A row of certified values, and a row of lent ones: a struct's row is
  -- the latter (`NTy.carrier` at a struct is its row under a certificate),
  -- and splits as a tuple's does.
  -- A lent row is spelled either way (`plainSpelling?`); the components are
  -- rebuilt in the worker's spelling, which the wrapper's unfolds to, so
  -- that one type has one spelling wherever the components are read.  Two
  -- spellings of one type are equal only by unfolding, and a congruence
  -- closure compares the spellings: a leaf whose link facts read a value
  -- once at each spelling has them as unrelated terms.
  let carrierView ← if spelled.isAppOfArity ``NTy.carrier 2 then
      nativeTypeView (spelled.getArg! 1) else pure spelled
  let row? : Option (Lean.Expr × Lean.Expr × Option PlainSpelling) ←
    if spelled.isAppOfArity ``HList 2 then pure (some (spelled.getArg! 0, spelled.getArg! 1, none))
    else if spelled.isAppOfArity ``NTy.carrier 2 &&
        carrierView.isAppOfArity ``NTy.tuple 1 then
      pure (some (spelled.getArg! 0, carrierView.getArg! 0, none))
    else match plainSpelling? spelled with
      | some spelling => match spelling.payload with
        | .row row => pure (some (spelling.family, row, some spelling))
        | .ty τ => do
          let τ ← nativeTypeView τ
          -- A lent struct value is its row: no certificate stands between
          -- them, unlike the certified side (`NTy.plainOf`).
          if τ.isAppOfArity ``NTy.tuple 1 then
            pure (some (spelling.family, τ.getArg! 0, some spelling))
          else if τ.isAppOfArity ``NTy.struct 3 then
            pure (some (spelling.family, τ.getArg! 2, some spelling))
          else pure none
        | .variant _ _ => pure none
      | none => pure none
  let some (carriers, row, lent) := row? | return none
  let row ← withReducible (whnf row)
  if row.isConstOf ``NRow.nil then return some .unit
  unless row.isAppOfArity ``NRow.cons 2 do return none
  let (head, tail) := match lent with
    | some spelling => (spelling.tyOf (row.getArg! 0), spelling.rowOf (row.getArg! 1))
    | none => (mkApp2 (mkConst ``NTy.carrier) carriers (row.getArg! 0),
        mkApp2 (mkConst ``HList) carriers (row.getArg! 1))
  return some (.pair (mkApp2 (mkConst ``Prod [levelZero, levelZero]) head tail))

/-- A certified value compared with one built from its parts: the parts
compare.  A certificate carries nothing beyond the value it certifies, so an
equation one side of which is a constructor application — a certified
integer `{ val := 1, fits := … }`, a certified row `⟨row, held⟩` — states
the carried value of the other side, which is the scalar form every decider
reads.  Two sides both built state the equation of the values they carry,
the certificates dropped (`Subtype.mk.injEq`), which is how a goal stating
two certified values reaches the equations of its parts.  The other
direction is `certifiedValueEq`: two values compared through their
certificates compare as the values.  Nothing is oriented by which side is
the goal's: the rule is the shape of the sides (`SpecInt.eq_iff_val`,
`Subtype.ext_iff`, `Subtype.mk.injEq`). -/
simproc [vir_denote_norm] certifiedByConstruction (@Eq _ _ _) := fun e => do
  let e := e.consumeMData
  let_expr Eq type lhs rhs := e | return .continue
  let lhs := lhs.consumeMData
  let rhs := rhs.consumeMData
  -- A nominal certification is a built subtype even when its producer
  -- preserves the named ofDeepHolds operation. Compute that constructor at
  -- this equality observation, then let the existing certificate comparison
  -- below read its row. Other uses retain the named operation.
  let computeNominal (side : Lean.Expr) : MetaM Lean.Expr := do
    let_expr VerexIR.Proofs.Denote.NTy.ofDeepHolds _ τ _ _ := side | return side
    let τView ← nativeTypeView τ
    unless τView.isAppOfArity ``VerexIR.Proofs.Denote.NTy.struct 3 ||
        τView.isAppOfArity ``VerexIR.Proofs.Denote.NTy.enum 5 do return side
    whnfD side
  let left ← computeNominal lhs
  let right ← computeNominal rhs
  if left != lhs || right != rhs then
    return .visit { expr := mkApp3 e.getAppFn type left right }
  let constructed (side : Lean.Expr) : Bool :=
    side.isAppOfArity ``VerexIR.SpecInt.mk 4 || side.isAppOfArity ``Subtype.mk 4
  let type ← whnfR type
  let type ← if type.isAppOfArity ``NTy.carrier 2 &&
      isNominalType (← getEnv) (type.getArg! 1) then whnfD type else pure type
  -- A scalar local can be substituted as a whole. Projecting its equality
  -- first prevents that substitution and makes every use re-establish it.
  if type.isAppOfArity ``VerexIR.SpecInt 2 then
    let substitutable (side other : Lean.Expr) := match side with
      | .fvar id => !other.containsFVar id
      | _ => false
    if substitutable lhs rhs || substitutable rhs lhs then return .continue
  -- Two certified values both built: the equation of the values they carry,
  -- the certificates dropped as irrelevant (`Subtype.mk.injEq`), after
  -- which the values split as a row's do (`pairInjectionRows`).  This is
  -- the direction a goal needs, and the dual of the splitting of such an
  -- equation among the hypotheses: a goal stating two certified values
  -- cannot be split by rewriting, because a certificate's type mentions the
  -- value it certifies and rewriting under it is not type-correct, so the
  -- equation is reduced here instead
  -- (`doc/designs/data-invariants.md`).
  if constructed lhs && constructed rhs && type.isAppOfArity ``Subtype 2 &&
      lhs.isAppOfArity ``Subtype.mk 4 && rhs.isAppOfArity ``Subtype.mk 4 then
    let values ← mkEq (lhs.getArg! 2) (rhs.getArg! 2)
    let injected := mkAppN (mkConst ``Subtype.mk.injEq type.getAppFn.constLevels!)
      #[type.getArg! 0, type.getArg! 1, lhs.getArg! 2, lhs.getArg! 3,
        rhs.getArg! 2, rhs.getArg! 3]
    let proof? ← (try
        let hinted ← mkExpectedTypeHint injected (← mkEq e values)
        if ← isTypeCorrect hinted then pure (some hinted) else pure none
      catch _ => pure none)
    if let some proof := proof? then
      return .visit { expr := values, proof? := some proof }
  -- One side built, the other read: two read sides are `certifiedValueEq`'s.
  unless constructed lhs != constructed rhs do return .continue
  let equivalence ←
    if type.isAppOfArity ``VerexIR.SpecInt 2 then
      (try pure (some (← mkAppM ``VerexIR.SpecInt.eq_iff_val #[lhs, rhs])) catch _ => pure none)
    else if type.isAppOfArity ``Subtype 2 then
      (try
        pure (some (← mkAppOptM ``Subtype.ext_iff #[none, none, some lhs, some rhs]))
      catch _ => pure none)
    else pure none
  let some equivalence := equivalence | return .continue
  let reduced ← (try
      let statement ← whnfR (← inferType equivalence)
      let_expr Iff stated values := statement | pure none
      if !(← isDefEq stated e) then pure none
      else
        let proof ← mkAppM ``propext #[equivalence]
        if ← isTypeCorrect proof then pure (some (values, proof)) else pure none
    catch _ => pure none)
  let some (values, proof) := reduced | return .continue
  return .visit { expr := values, proof? := some proof }

/-- Two certified values compared through their certificates: the values
compare (`Subtype.ext_iff`). A certificate is proof-irrelevant, so the two
equations are the same; the equation of the values is the one that
substitutes, and a leaf reads the components of one value through the other
(`doc/designs/data-invariants.md`). -/
simproc [vir_denote_norm] certifiedValueEq (@Eq _ (@Subtype.val _ _ _) (@Subtype.val _ _ _)) :=
  fun e => withDefault do
    let e := e.consumeMData
    let_expr Eq _ lhs rhs := e | return .continue
    let lhs := lhs.consumeMData
    let rhs := rhs.consumeMData
    unless lhs.isAppOfArity ``Subtype.val 3 && rhs.isAppOfArity ``Subtype.val 3 do
      return .continue
    let (left, right) := (lhs.getArg! 2, rhs.getArg! 2)
    unless ← isDefEq (← inferType left) (← inferType right) do return .continue
    let equation ← mkEq left right
    let proof ← (try
        let extensional ← mkAppOptM ``Subtype.ext_iff #[none, none, some left, some right]
        pure (some (← mkAppM ``propext #[← mkAppM ``Iff.symm #[extensional]]))
      catch _ => pure none)
    let some proof := proof | return .continue
    unless ← isTypeCorrect proof do return .continue
    return .visit { expr := equation, proof? := some proof }

/-- Two pairs compared, as rows of values are: the components compare, whichever
the sides (`Prod.ext_iff`). Decided on the spelling of the equation: a side
constructed, or a tuple's type, so that a struct's value compared whole stays
whole. A local the other side does not mention is left to its substitution. A
run's observation (`result_of`, `state_of`) stays whole: the run deciders read
it as stated; what they derive of a local decomposes. The lemma `Prod.mk.injEq`
covers two constructors at a product type; a row states the equation at its
own, `HList (.cons τ rest)`, which unfolds to the product only beyond the
matching's transparency. -/
simproc [vir_denote_norm] pairInjectionRows (@Eq _ _ _) := fun e => do
  let e := e.consumeMData
  let_expr Eq type lhs rhs := e | return .continue
  let lhs := lhs.consumeMData
  let rhs := rhs.consumeMData
  let observation (side : Lean.Expr) : Bool :=
    side.isAppOf ``VerexIR.Proofs.ResultOf || side.isAppOf ``VerexIR.Proofs.StateOf
  if observation lhs || observation rhs then return .continue
  let spelled := type.consumeMData
  let (leftPair, rightPair) := (lhs.isAppOfArity ``Prod.mk 4, rhs.isAppOfArity ``Prod.mk 4)
  if leftPair && rightPair && spelled.isAppOfArity ``Prod 2 then return .continue
  -- A row's own type says the equation is a row's: a tuple's row of
  -- certified values, and any row of lent ones — a lent struct value is its
  -- row, with no certificate between them, so an equation of two currents
  -- decomposes as the clauses read them
  -- (`doc/designs/data-invariants.md`).
  let lentRow ← match plainSpelling? spelled with
    | some spelling => match spelling.payload with
      | .row _ => pure true
      | .ty τ => do
          let τView ← nativeTypeView τ
          pure (τView.isAppOfArity ``NTy.tuple 1 || τView.isAppOfArity ``NTy.struct 3)
      | .variant _ _ => pure false
    | none => pure false
  let carrierView ← if spelled.isAppOfArity ``NTy.carrier 2 then
      nativeTypeView (spelled.getArg! 1) else pure spelled
  let tupleRow := spelled.isAppOf ``HList || lentRow ||
    (spelled.isAppOfArity ``NTy.carrier 2 && carrierView.isAppOf ``NTy.tuple)
  let unitSide := lhs.isConstOf ``Unit.unit || lhs.isConstOf ``PUnit.unit ||
    rhs.isConstOf ``Unit.unit || rhs.isConstOf ``PUnit.unit
  unless leftPair || rightPair || tupleRow || unitSide do return .continue
  let substitutable (side other : Lean.Expr) : Bool := match side with
    | .fvar x => !other.containsFVar x
    | _ => false
  if substitutable lhs rhs || substitutable rhs lhs then return .continue
  -- The rows of two certified values compared: their equation is the
  -- equation of the values (`certifiedValueEq`), which substitutes; its
  -- components would read through the certificates instead.
  if lhs.isAppOfArity ``Subtype.val 3 && rhs.isAppOfArity ``Subtype.val 3 then
    return .continue
  match ← rowSpelling? spelled with
  | none => return .continue
  | some .unit =>
    let same ← mkExpectedTypeHint (← mkEqRefl lhs) e
    return .visit { expr := mkConst ``True, proof? := some (mkApp2 (mkConst ``eq_true) e same) }
  | some (.pair carrier) =>
    let (leftFirst, leftSecond) := rowComponents carrier lhs
    let (rightFirst, rightSecond) := rowComponents carrier rhs
    let components ← mkAppM ``And #[← mkEq leftFirst rightFirst, ← mkEq leftSecond rightSecond]
    let split := mkApp4 (mkConst ``Prod.ext_iff carrier.getAppFn.constLevels!)
      (carrier.getArg! 0) (carrier.getArg! 1) lhs rhs
    return .visit { expr := components, proof? := some (mkApp3 (mkConst ``propext) e components split) }

/-- The value and position a pair's component reads, as a read of a value. -/
private def pairRead? (component : Lean.Expr) : Option (Lean.Expr × Bool) :=
  let component := component.consumeMData
  if component.isAppOfArity ``Prod.fst 3 then some (component.getArg! 2, true)
  else if component.isAppOfArity ``Prod.snd 3 then some (component.getArg! 2, false)
  else match component with
    | .proj ``Prod index value => some (value, index == 0)
    | _ => none

/-- Whether a type's spelling is a row's end: a row with no entry, at
whatever layer the type spells it (`rowSpelling?`) — `Unit`, the empty row
of certified values, the empty plain row.  The one reader of row spellings
answers this, so that a layer it reads is a row's end for every rule that
ends a row through this. -/
private def spelledRowEnd (type : Lean.Expr) : MetaM Bool := do
  match ← rowSpelling? (← whnfR type) with
  | some .unit => return true
  | _ => return false

/-- Whether a type's spelling is a row of one entry: the row the type spells
has an entry (`rowSpelling?`) and the rest of it is the row's end. -/
private def spelledSingleton (type : Lean.Expr) : MetaM Bool := do
  match ← rowSpelling? (← whnfR type) with
  | some (.pair carrier) => spelledRowEnd (carrier.getArg! 1)
  | _ => return false

/-- A row of the reads of one value is that value, up to its unit end
(`(e.1, e.2)`, `(e.1, ())` where the spelling of `e`'s type shows a unit second
component): the form the components of an equation leave where a value is rebuilt
from them. Definitional, by structure eta. -/
dsimproc [vir_denote_norm] rowOfReads (Prod.mk _ _) := fun e => do
  let e := e.consumeMData
  unless e.isAppOfArity ``Prod.mk 4 do return .continue
  let some (value, true) := pairRead? (e.getArg! 2) | return .continue
  -- Eta stays within the row's layer. Forgetting a certificate-free
  -- singleton can leave `(row.1, ())` at the plain row; replacing it with
  -- the certified row would reintroduce its implicit component types.
  let some (.pair carrier) ← rowSpelling? (← inferType value) | return .continue
  unless ← withReducible (isDefEq carrier (← inferType e)) do return .continue
  let second := (e.getArg! 3).consumeMData
  if let some (other, false) := pairRead? second then
    return if other == value then .visit value else .continue
  unless second.isConstOf ``Unit.unit || second.isConstOf ``PUnit.unit do return .continue
  unless ← spelledSingleton (← inferType value) do return .continue
  return .visit value

/-- The first component of a pair whose second component has one value
determines the pair: the two statements are the same proposition.  A row of
one entry is such a pair — the rest of it is the row's end, which has one
value — so the entry's equation and the row's say the same thing. -/
theorem pairEntryEq_eq_eq {α β : Type} (single : ∀ l r : β, l = r) (left right : α × β) :
    (left.1 = right.1) = (left = right) :=
  propext ⟨fun entry => Prod.ext entry (single _ _), fun same => same ▸ rfl⟩

open Lean Meta in
/-- An equation of the one entry of the rows of two certified values: the
equation of the rows (`pairEntryEq_eq_eq`), which is the equation of the
values themselves (`certifiedValueEq`) and substitutes.  Both sides read the
entry of a row whose type spells one entry and the row's end (`pairRead?`,
`rowSpelling?`), and both rows are read through a certificate: that is the
reason the row's equation is wanted over the entry's, since a certificate's
type mentions the row it certifies and only the row's equation reaches the
value.  A row carrying no certificate is left to the rule that splits it
(`pairInjectionRows`), whose components the deciders read.  Nothing is
oriented by which side is the goal's: the rule is the shape of the sides, as
`certifiedByConstruction`'s is.  A leaf states such an equation of a `&mut`
parameter's final value and of the prophecy a call returned, and only the
values' equation relates the two
(`doc/designs/data-invariants.md`). -/
simproc [vir_denote_norm] singletonRowEntryEq (@Eq _ _ _) := fun e => do
  let e := e.consumeMData
  let_expr Eq type lhs rhs := e | return .continue
  let some (left, true) := pairRead? lhs | return .continue
  let some (right, true) := pairRead? rhs | return .continue
  unless left.isAppOfArity ``Subtype.val 3 && right.isAppOfArity ``Subtype.val 3 do
    return .continue
  -- The row's entry and rest as its own type spells them: a row type is not
  -- reducible, so they are read off the row it names and never by unfolding
  -- it (`rowSpelling?`).
  let rowType ← whnfR (← instantiateMVars (← inferType left))
  let some (.pair carrier) ← rowSpelling? rowType | return .continue
  let (α, β) := (carrier.getArg! 0, carrier.getArg! 1)
  unless ← spelledRowEnd β do return .continue
  unless ← isDefEq (← inferType right) rowType do return .continue
  unless ← isDefEq type α do return .continue
  let value ← mkEq left right
  let proof? ← (try withDefault do
      let single ← withLocalDeclD `l β fun l => withLocalDeclD `r β fun r => do
        mkLambdaFVars #[l, r] (← mkExpectedTypeHint (← mkEqRefl l) (← mkEq l r))
      pure (some (← mkExpectedTypeHint
        (mkAppN (mkConst ``pairEntryEq_eq_eq) #[α, β, single, left, right])
        (← mkEq e value)))
    catch _ => pure none)
  let some proof := proof? | return .continue
  unless ← isTypeCorrect proof do return .continue
  return .visit { expr := value, proof? := some proof }

open Lean Meta in
/-- A declaration's data invariant of a value where the declaration states
none: `True`.  The unit's own clauses are a field of its function values
(`FunctionCarriers.own`), so a pack of a declaration without an invariant
leaves a cut over the trivial predicate, which closes by unfolding the
field alone.  Reduced here, the cut closes in the normal form and no leaf
is spent on it; a declaration that does state an invariant keeps its
clauses, which the step that reports the obligation reads
(`doc/designs/data-invariants.md`). -/
dsimproc [vir_denote, vir_denote_norm] ownWithoutInvariant
    (@VerexIR.Proofs.Denote.FunctionCarriers.own _ ?functions _ _ _) := fun e => do
  unless e.isAppOfArity ``VerexIR.Proofs.Denote.FunctionCarriers.own 5 do return .continue
  -- The field alone, not the application: a unit that states an invariant
  -- keeps a predicate here, and reducing the application would unfold its
  -- clauses at every occurrence for nothing.
  let field := mkAppN e.getAppFn (e.getAppArgs.extract 0 2)
  let reduced ← (try withDefault (whnf field) catch _ => pure field)
  let mut body := reduced
  for _ in [0:3] do
    let .lam _ _ next _ := body | return .continue
    body := next
  unless body.isConstOf ``True do return .continue
  return .done (mkConst ``True)

/-- Selecting an absent declaration in an installed Given frame owes no
own clause. Inspect only the declaration table at this handle and argument
row; a present or unknown declaration retains its predicate and encoding. -/
simproc [vir_denote, vir_denote_norm] givenOwnWithoutInvariant
    (@Carriers.own ?carriers _ _ _) := fun e => do
  let_expr Carriers.own family source arguments encoded := e | return .continue
  let_expr GivenFrame.toCarriers installed := family | return .continue
  let_expr GivenFrame.withDeclarations given declarations := installed | return .continue
  let selected := mkApp2 declarations source arguments
  let reduced ← (try withDefault (whnf selected) catch _ => pure selected)
  unless reduced.isAppOfArity ``Option.none 1 do return .continue
  let absent ← mkExpectedTypeHint (← mkEqRefl reduced) (← mkEq selected reduced)
  let proof := mkAppN (mkConst ``GivenFrame.withDeclarations_own_none)
    #[given, declarations, source, arguments, absent, encoded]
  return .done { expr := mkConst ``True, proof? := some (← mkAppM ``eq_true #[proof]) }

open Lean Meta in
/-- A variant declaration's data invariants of a value whose variants carry
no certificate: `True`.  The invariants of a variant value are the ones of
the variant it is, which the normal form reads of a constructed value
(`variantPlain.deepHolds_inl`) and not of a value whose variant is unknown —
the current of a `&mut` to an enum is such a value, and the cut at a copy
out of it would otherwise be split per variant for a conjunction trivially
true at each.  Where no row of the declaration carries a certificate the
step closes it whole (`variantPlain.deepHolds_of_certificateFree`,
`NRows.certificateFree`); a declaration whose rows do carry one keeps its
clauses, which the split at the cut reads
(`doc/designs/data-invariants.md`). -/
simproc [vir_denote_norm] variantWithoutCertificates
    (@VerexIR.Proofs.Denote.variantPlain.deepHolds ?carriers _ _ _) := fun e => do
  let e := e.consumeMData
  unless e.isAppOfArity ``VerexIR.Proofs.Denote.variantPlain.deepHolds 4 do return .continue
  let parts := e.getAppArgs
  -- Only a declaration whose rows the term spells: the criterion is read by
  -- evaluation, and a row given by a substitution or a family's field would
  -- be unfolded for an answer it cannot give.
  unless spelledNRows parts[2]! (some (← getEnv)) do return .continue
  let free := mkApp (mkConst ``VerexIR.Proofs.Denote.NRows.certificateFree) parts[2]!
  unless (← (try withDefault (whnf free) catch _ => pure free)).isConstOf ``Bool.true do
    return .continue
  let decided ← mkExpectedTypeHint (← mkEqRefl (mkConst ``Bool.true))
    (← mkEq free (mkConst ``Bool.true))
  let holds := mkAppN
    (mkConst ``VerexIR.Proofs.Denote.variantPlain.deepHolds_of_certificateFree)
    #[parts[0]!, parts[1]!, parts[2]!, decided, parts[3]!]
  return .done { expr := mkConst ``True, proof? := some (← mkAppM ``eq_true #[holds]) }

-- Constructed values compared: a different constructor refutes, the same
-- one compares its fields.
attribute [vir_denote_norm] Sum.inl.injEq Sum.inr.injEq reduceCtorEq

/- The normalization of a verification condition: the denotation's rules,
the weakest-precondition rules, and the propositional normal forms a leaf
is decided in. -/
attribute [vir_denote_norm] VerexIR.Proofs.Denote.variantPlainName_inl
  VerexIR.Proofs.Denote.variantPlainName_inr VerexIR.Proofs.Denote.ghostValueOf_same
  VerexIR.Proofs.Denote.ghostInitOf_same Weave.supplying_cons Weave.supplying_nil NRow.length
attribute [vir_denote_norm] VerexIR.Proofs.wp_choose VerexIR.Proofs.wp_assume
  VerexIR.Proofs.wp_certifyState VerexIR.Proofs.Denote.wp_prophecyOf
-- A certifying step: the data invariants of the value it builds, and the
-- continuation under them (`Spec.certified`,
-- `doc/designs/data-invariants.md`). The obligation keeps its site
-- through the enclosing `constructedAt`/`mutationEndAt` marker, which the
-- step that cuts here reads (`dataInvariantStep`).
attribute [vir_denote_norm] VerexIR.Proofs.wp_certified
  VerexIR.Proofs.Denote.wp_certifying
  VerexIR.Proofs.Denote.certifying_struct VerexIR.Proofs.Denote.certifying_enum
-- A value's data invariants at every depth: a declaration's clauses of its
-- encoding and its fields' (`NTy.deepHolds`), and the certification of a
-- lent value that satisfies them.
attribute [vir_denote_norm] VerexIR.Proofs.Denote.NTy.deepHolds_struct
  VerexIR.Proofs.Denote.NTy.deepHolds_enum
  VerexIR.Proofs.Denote.NTy.forget_ofDeepHolds VerexIR.Proofs.Denote.NTy.ofDeepHolds_forget
  VerexIR.Proofs.Denote.NTy.plainEncode_ofDeepHolds
-- The ghost initializers a construction applies, and the lender a loan's
-- death resolves (`Term.pack`, `Term.resolve`).
attribute [vir_denote_norm] VerexIR.Proofs.Denote.GhostInits.initialize_nil
  VerexIR.Proofs.Denote.GhostInits.initialize_cons
  VerexIR.Proofs.Denote.Lender.resolve_slot VerexIR.Proofs.Denote.Lender.resolve_place
  VerexIR.Proofs.Denote.Lender.resolve_candidate
  VerexIR.Proofs.Denote.Lender.resolve_referent VerexIR.Proofs.Denote.Lender.resolve_equation
-- Whether a path crosses a dereference, which decides where a read reads
-- out of a plain current (`readPlace`).
attribute [vir_denote_norm] VerexIR.Proofs.Denote.Proj.hasDeref
  VerexIR.Proofs.Denote.Proj.hasDeref_nil VerexIR.Proofs.Denote.Proj.hasDeref_deref
-- A struct's or a variant's value is its row: a certificate is a
-- proposition and two of them are equal, so an equation of two built
-- values is the equation of their rows.
attribute [vir_denote_norm] Subtype.mk.injEq
-- A struct's row at the frame of its type arguments (`Skolems.reframe`).
attribute [vir_denote_norm] VerexIR.Proofs.Denote.Skolems.reframe_struct_val
  VerexIR.Proofs.Denote.Skolems.plainReframe_struct_fst
  VerexIR.Proofs.Denote.Skolems.plainReframe_struct_snd
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
  VerexIR.Proofs.Denote.ResourceType.mk.injEq
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
  VerexIR.Proofs.Denote.NTy.plainInhabitant VerexIR.Proofs.Denote.HListPlain.inhabitant
  VerexIR.Proofs.Denote.variantPlain.inhabitant
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

/-- Handles differ where their literal indices do: a check the kernel
evaluates, in place of rewriting the handles' equality. -/
theorem handle_ne_of_indices {n m f g : Nat} (differ : (Nat.beq n m && Nat.beq f g) = false) :
    (VerexIR.FunctionHandle.mk ⟨n⟩ ⟨f⟩) ≠ (VerexIR.FunctionHandle.mk ⟨m⟩ ⟨g⟩) := by
  intro same
  simp only [VerexIR.FunctionHandle.mk.injEq, VerexIR.NamespaceId.mk.injEq,
    VerexIR.FunctionId.mk.injEq] at same
  simp [same.1, same.2, Nat.beq_refl] at differ

/-- The indices of a function handle, where they are literals. -/
private def handleIndices? (handle : Lean.Expr) :
    MetaM (Option (Lean.Expr × Lean.Expr × Nat × Nat)) := do
  let handle := handle.consumeMData
  unless handle.isAppOfArity ``VerexIR.FunctionHandle.mk 2 do return none
  let spaceId := (handle.getArg! 0).consumeMData
  let functionId := (handle.getArg! 1).consumeMData
  unless spaceId.isAppOfArity ``VerexIR.NamespaceId.mk 1 &&
      functionId.isAppOfArity ``VerexIR.FunctionId.mk 1 do return none
  let some space ← getNatValue? spaceId.appArg! | return none
  let some function ← getNatValue? functionId.appArg! | return none
  return some (spaceId.appArg!, functionId.appArg!, space, function)

/-- A proof that `callee` is not the handle of literal indices `entry`. -/
private def handlesDiffer (callee entry : Lean.Expr × Lean.Expr × Nat × Nat) : Lean.Expr :=
  let (n, f, _, _) := callee
  let (m, g, _, _) := entry
  mkApp5 (mkConst ``handle_ne_of_indices) n m f g
    (mkApp2 (mkConst ``Eq.refl [levelOne]) (mkConst ``Bool) (mkConst ``Bool.false))

/-- Resolve a call along its route chain in one step: every entry of another
literal handle is passed by `other`, a lemma application with a check the
kernel evaluates, and the entry of the call's own signature is applied by
`self`. `entries` is the number of the chain's arguments up to its `rest`,
`callArguments` the number of the call's. The chain's arguments are not
visited, so the cost of a call is its position in the chain. -/
private def routeCall (route other self : Name) (entries callArguments : Nat)
    (selfArguments : Array Lean.Expr → Array Lean.Expr → Array Lean.Expr)
    (otherArguments : Array Lean.Expr → Array Lean.Expr → Lean.Expr → Array Lean.Expr)
    (e : Lean.Expr) : SimpM Simp.Step := do
  let arity := 3 + entries + callArguments
  unless e.isAppOfArity route arity do return .continue
  let levels := e.getAppFn.constLevels!
  let start := e.getAppArgs
  let call := start.extract (3 + entries) arity
  let some callee ← handleIndices? call[0]! | return .continue
  let σs := call[callArguments - 3]!
  let shape := call[callArguments - 2]!
  let type ← inferType e
  let level ← getLevel type
  let trans (first second : Lean.Expr) (left middle right : Lean.Expr) : Lean.Expr :=
    mkApp6 (mkConst ``Eq.trans [level]) type left middle right first second
  let mut current := e
  let mut proof? : Option Lean.Expr := none
  repeat
    unless current.isAppOfArity route arity do break
    let arguments := current.getAppArgs
    let entry := arguments.extract 3 (3 + entries)
    let some literal ← handleIndices? entry[0]! | break
    let prefixArguments := arguments.extract 0 3
    if literal.2.2.1 == callee.2.2.1 && literal.2.2.2 == callee.2.2.2 then
      -- The call's own entry, at its own signature.
      unless ← withReducible (isDefEq σs entry[1]! <&&> isDefEq shape entry[2]!) do break
      let step := mkAppN (mkConst self levels) (prefixArguments ++ selfArguments entry call)
      let some (_, _, result) := (← inferType step).eq? | break
      let proof := match proof? with
        | some earlier => trans earlier step e current result
        | none => step
      return .visit { expr := result, proof? := some proof }
    let rest := entry.back!
    let next := mkAppN rest call
    let step := mkAppN (mkConst other levels) (prefixArguments ++
      otherArguments entry call (handlesDiffer callee literal))
    proof? := some <| match proof? with
      | some earlier => trans earlier step e current next
      | none => step
    current := next
  match proof? with
  | some proof => return .visit { expr := current, proof? := some proof }
  | none => return .continue

/-- `routeMeaning handle π ρ self rest callee σs shape args`. -/
simproc ↓ [vir_denote_norm] resolveCalleeRoute
    (@VerexIR.Proofs.Denote.routeMeaning _ ?functions ?skolems _ _ _ _ _ _ _ _ _) :=
  routeCall ``VerexIR.Proofs.Denote.routeMeaning ``VerexIR.Proofs.Denote.routeMeaning_other
    ``VerexIR.Proofs.Denote.routeMeaning_self 5 4
    (fun entry call => entry ++ #[call[3]!])
    (fun entry call differ => #[entry[0]!, call[0]!, entry[1]!, call[1]!, entry[2]!, call[2]!,
      entry[3]!, entry[4]!, call[3]!, differ])

/-- `routeGeneric handle π ρ self frame rest callee typeArgs θ σs shape args`. -/
simproc ↓ [vir_denote_norm] resolveGenericRoute
    (@VerexIR.Proofs.Denote.routeGeneric _ ?functions ?skolems _ _ _ _ _ _ _ _ _ _ _ _) :=
  routeCall ``VerexIR.Proofs.Denote.routeGeneric ``VerexIR.Proofs.Denote.routeGeneric_other
    ``VerexIR.Proofs.Denote.routeGeneric_self 6 6
    (fun entry call => entry ++ #[call[1]!, call[2]!, call[5]!])
    (fun entry call differ => #[entry[0]!, call[0]!, entry[1]!, call[3]!, entry[2]!, call[4]!,
      entry[3]!, entry[4]!, entry[5]!, call[1]!, call[2]!, call[5]!, differ])

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

/-- A certified integer defined by its value and read only by its value is the
value, its range a conjunct: no cut is left to read back. -/
theorem SpecInt.exists_val_read_unsigned (n : Nat) {value : Int} {p : Int → Prop} :
    (∃ c : VerexIR.SpecInt (.bits (n + 1)) false, c.val = value ∧ p c.val) ↔
      (0 ≤ value ∧ value ≤ 2 ^ (n + 1) - 1) ∧ p value :=
  ⟨fun ⟨c, defined, held⟩ =>
    ⟨(IntegerValueFits_unsigned_succ n value).mp (defined ▸ c.fits), defined ▸ held⟩,
    fun ⟨bounds, held⟩ =>
      ⟨⟨value, (IntegerValueFits_unsigned_succ n value).mpr bounds⟩, rfl, held⟩⟩

theorem SpecInt.exists_val_read_signed (n : Nat) {value : Int} {p : Int → Prop} :
    (∃ c : VerexIR.SpecInt (.bits (n + 1)) true, c.val = value ∧ p c.val) ↔
      (-(2 : Int) ^ n ≤ value ∧ value ≤ 2 ^ n - 1) ∧ p value :=
  ⟨fun ⟨c, defined, held⟩ =>
    ⟨(IntegerValueFits_signed_succ n value).mp (defined ▸ c.fits), defined ▸ held⟩,
    fun ⟨bounds, held⟩ =>
      ⟨⟨value, (IntegerValueFits_signed_succ n value).mpr bounds⟩, rfl, held⟩⟩

open Lean Meta Simp in
/-- One spelling of a defined value: `∃ c, c.val = v ∧ p c` at a fixed width is
the range of `v` and `p` at the value `v` cut to the width, wherever it occurs;
where `p` reads `c` only by its value, `p` at `v` itself. -/
simproc [vir_denote_norm] existsDefinedValue (∃ _ : VerexIR.SpecInt _ _, _) := fun e => do
  let_expr Exists domain body := e | return .continue
  let .lam name _ (.app (.app (.const ``And _) definition) rest) info := body | return .continue
  let some (_, lhs, value) := definition.eq? | return .continue
  unless lhs.isAppOfArity ``VerexIR.SpecInt.val 3 && lhs.appArg! == .bvar 0 do return .continue
  if value.hasLooseBVars then return .continue
  let_expr VerexIR.SpecInt width signed := domain | return .continue
  let_expr VerexIR.IntWidth.bits bits := width | return .continue
  let some bits := bits.nat? | return .continue
  let .succ n := bits | return .continue
  let signed? := signed.isConstOf ``Bool.true
  -- `c`'s reads by its value abstracted; `c` left is another read.
  let (theorem_, predicate) ← withLocalDeclD name domain fun c => do
    let held := rest.instantiate1 c
    withLocalDeclD name (mkConst ``Int) fun read => do
      let reading := held.replace fun sub =>
        if sub.isAppOfArity ``VerexIR.SpecInt.val 3 && sub.appArg! == c then some read else none
      if reading.containsFVar c.fvarId! then
        return (if signed? then ``SpecInt.exists_val_cut_signed else ``SpecInt.exists_val_cut_unsigned,
          Expr.lam name domain rest info)
      return (if signed? then ``SpecInt.exists_val_read_signed else ``SpecInt.exists_val_read_unsigned,
        ← mkLambdaFVars #[read] reading)
  let iff ← mkAppOptM theorem_ #[mkNatLit n, some value, some predicate]
  let some (_, rhs) := (← instantiateMVars (← inferType iff)).iff? | return .continue
  return .visit { expr := rhs.headBeta, proof? := some (← mkPropExt iff) }

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
    if name == `free then decidedFrameFree? condition else return none
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
      ``SpecInt.exists_val_signed, ``SpecInt.exists_val_unbounded] do
    theorems ← theorems.addConst name
  let simprocs ← ({} : Simprocs).add ``existsNamedValue (post := false)
  let simprocs ← simprocs.add ``existsDefinedValue (post := true)
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
  VerexIR.Proofs.Denote.variantCarrier.order VerexIR.Proofs.Denote.NTy.plainOrder
  VerexIR.Proofs.Denote.HListPlain.order VerexIR.Proofs.Denote.variantPlain.order
  VerexIR.Proofs.Denote.NTy.plainOrderOf VerexIR.Proofs.Denote.HListPlainOf.order
  VerexIR.Proofs.Denote.variantPlainOf.order
  Int.compare_eq_lt Int.compare_eq_gt
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

/-- One spelling of a quotient (`doc/designs/proof-language.md`, "One spelling of
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

-- A concatenation's size, linear in its parts'.
attribute [vir_denote_norm] Array.size_append

-- The identities of zero and one, which a substitution or a case split
-- leaves (`r - 0`, `r * 0`): the arithmetic in its simplest spelling.
attribute [vir_denote_norm] Int.add_zero Int.zero_add Int.sub_zero Int.mul_zero Int.zero_mul
  Int.mul_one Int.one_mul Int.zero_ediv Int.ediv_zero Int.ediv_one Int.zero_emod Int.emod_one

/-! Membership (`doc/plan/proof-language.md`, "The deciders' own theory of
membership"): the leaf's deciders read `e ∈ v` by its positions
(`mem_iff_exists_int_index`); the step normal form keeps it an atom with its own algebra,
and reads a search for an element as that atom (`VerexSL.Verify`'s step simp context). -/

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

/-- A search for an element is membership, by its value for an integer, or over a
vector's positions whatever its reads fall back to: the spellings the step normal
form reads as the atom (`doc/plan/proof-language.md`, "The deciders' own theory of membership"). -/
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

open Lean Meta Simp in
/-- `∀ x, p x ∧ q x` as `(∀ x, p x) ∧ (∀ x, q x)`, `forall_and`, its binder's name
kept: a clause's binder stays in a proof's scope. -/
def forallAndKeepingName : Simproc := fun e => do
  let .forallE name domain body info := e | return .continue
  let body' := body.consumeMData
  unless body'.isAppOfArity ``And 2 do return .continue
  let (left, right) := (body'.getArg! 0, body'.getArg! 1)
  let iff ← mkAppOptM ``forall_and #[domain, some (.lam name domain left info), some (.lam name domain right info)]
  let result := mkAnd (.forallE name domain left info) (.forallE name domain right info)
  return .visit { expr := result, proof? := some (← mkPropExt iff) }

open Lean Meta Simp in
simproc [vir_denote_norm] forallAndDependent (∀ x : ?α, ?p x) := forallAndKeepingName

open Lean Meta Simp in
simproc [vir_denote_norm] forallAndArrow (_ → _) := forallAndKeepingName

open Lean Meta Simp in
/-- A literal position: the normal form evaluates it wherever a substitution leaves
one. -/
simproc [vir_denote_norm] literalToNat (Int.toNat _) := fun e => do
  let some value := e.appArg!.int? | return .continue
  return .done { expr := toExpr value.toNat }

/-- The lengths of a nest of concatenations from `start`, as integers, in the order
of its leaves: `start + len(b) + len(c) + …`. -/
def SpecVector.lengthsFrom {α : Type} (start : Int) (rest : List (VerexIR.SpecVector α)) : Int :=
  rest.foldl (fun sum vector => sum + (vector.values.size : Int)) start

private theorem SpecVector.le_lengthsFrom {α : Type} (rest : List (VerexIR.SpecVector α)) (start : Int) :
    start ≤ SpecVector.lengthsFrom start rest := by
  induction rest generalizing start with
  | nil => exact Int.le_refl _
  | cons vector rest induction =>
      have := induction (start + (vector.values.size : Int))
      simp only [SpecVector.lengthsFrom, List.foldl_cons] at this ⊢
      omega

/-- A nest of concatenations is one concatenation under one premise on its
leaves' lengths: the depth-one lemma, leaf by leaf. -/
theorem SpecVector.values_nest {α : Type} (rest : List (VerexIR.SpecVector α))
    (first : VerexIR.SpecVector α)
    (bounded : SpecVector.lengthsFrom (first.values.size : Int) rest ≤ 18446744073709551615) :
    (rest.foldl VerexIR.SpecVector.append first).values =
      rest.foldl (fun values vector => values ++ vector.values) first.values := by
  induction rest generalizing first with
  | nil => rfl
  | cons vector rest induction =>
      have below : (first.values.size : Int) + (vector.values.size : Int) ≤
          SpecVector.lengthsFrom (first.values.size : Int) (vector :: rest) :=
        SpecVector.le_lengthsFrom rest _
      have joined : (first.append vector).values = first.values ++ vector.values :=
        VerexIR.SpecVector.values_append (by omega)
      have size : (((first.append vector).values.size : Nat) : Int) =
          (first.values.size : Int) + (vector.values.size : Int) := by
        rw [joined, Array.size_append]
        omega
      have step := induction (first.append vector) (by
        rw [size]
        exact bounded)
      simp only [List.foldl_cons]
      rw [step, joined]

open Lean Meta Simp in
/-- A nest of concatenations is one concatenation: the values of a left nest of
`SpecVector.append` are the concatenation of its leaves' under one premise, the
sum of the leaves' lengths as integers (a literal vector's as a literal), in their
order, at most `MAX_U64`. A proof states it (`len(a) + len(b) + … <= MAX_U64`), matched by spelling in the normal
form; `omega` proves it at a leaf. -/
simproc [vir_denote_norm] valuesOfNest
    (VerexIR.SpecVector.values (VerexIR.SpecVector.append _ _)) := fun e => do
  let_expr VerexIR.SpecVector.values α nest := e | return .continue
  let mut first := nest
  let mut rest : List Lean.Expr := []
  while first.isAppOfArity ``VerexIR.SpecVector.append 3 do
    rest := first.appArg! :: rest
    first := first.getArg! 1
  let int := mkConst ``Int
  -- A literal vector's length is its literal, which `omega` reads.
  let length (vector : Lean.Expr) : MetaM Lean.Expr := do
    let vector := vector.consumeMData
    if vector.isAppOfArity ``VerexIR.SpecVector.mk 3 then
      let values := (vector.getArg! 1).consumeMData
      if values.isAppOfArity ``List.toArray 2 then
        if let some (_, elements) := values.appArg!.listLit? then
          return toExpr (Int.ofNat elements.length)
    mkAppOptM ``Nat.cast #[int, none, ← mkAppM ``Array.size #[← mkAppM ``VerexIR.SpecVector.values #[vector]]]
  let sum ← rest.foldlM (init := ← length first) fun sum vector => do
    mkAppM ``HAdd.hAdd #[sum, ← length vector]
  let leaves ← mkListLit (mkApp (mkConst ``VerexIR.SpecVector) α) rest
  let values ← rest.foldlM (init := ← mkAppM ``VerexIR.SpecVector.values #[first]) fun values vector => do
    mkAppM ``HAppend.hAppend #[values, ← mkAppM ``VerexIR.SpecVector.values #[vector]]
  let premise ← mkAppM ``LE.le #[sum, toExpr (18446744073709551615 : Int)]
  let some bounded ← (← getMethods).discharge? premise | return .continue
  let proof := mkApp4 (mkConst ``SpecVector.values_nest) α leaves first bounded
  let statement ← mkEq e values
  return .visit { expr := values, proof? := some (← mkExpectedTypeHint proof statement) }

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
  and_true true_and and_imp forall_eq forall_eq' Prod.mk.injEq exists_eq exists_eq'
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
  VerexIR.SpecVector.values_push Array.append_singleton
  -- A field update a clause states (`nativeFieldUpdate`).
  VerexIR.Proofs.Denote.Var.update_here VerexIR.Proofs.Denote.Var.update_there
  List.map_toArray List.map_cons List.map_nil List.push_toArray
  List.size_toArray List.length_cons List.length_nil List.getElem?_eq_some_iff
  List.getElem?_eq_none_iff Array.getElem?_eq_none_iff Array.size_setIfInBounds Array.getElem?_map
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
  VerexIR.Proofs.Denote.variantCarrier.toSkolem_ofSkolem
  VerexIR.Proofs.Denote.NTy.plainOfSkolem_toSkolem
  VerexIR.Proofs.Denote.NTy.plainToSkolem_ofSkolem
  VerexIR.Proofs.Denote.HListPlain.ofSkolem_toSkolem
  VerexIR.Proofs.Denote.HListPlain.toSkolem_ofSkolem
  VerexIR.Proofs.Denote.variantPlain.ofSkolem_toSkolem
  VerexIR.Proofs.Denote.variantPlain.toSkolem_ofSkolem Array.map_map List.map_map
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

/-- A shift right of a multiple of the shift's power of two is the quotient's
multiple, exactly. -/
theorem shiftRight_mul_pow {multiplier quotient : Int} (value : Int) (distance : Nat)
    (isMultiple : multiplier = quotient * 2 ^ distance) :
    Int.shiftRight (multiplier * value) distance = quotient * value := by
  subst isMultiple
  rw [shiftRight_eq_div, Int.mul_right_comm,
    Int.mul_ediv_cancel _ (Int.ne_of_gt (Int.pow_pos (by decide)))]

/-- A shift right of the shift's power of two times a value is that value:
`(16 * x) >> 4 = x`. -/
theorem shiftRight_pow_mul {multiplier : Int} (value : Int) (distance : Nat)
    (isPower : multiplier = 2 ^ distance) : Int.shiftRight (multiplier * value) distance = value := by
  rw [shiftRight_mul_pow value distance (quotient := 1) (by rw [isPower, Int.one_mul]), Int.one_mul]

/-- A shift right of a literal multiple of the shift's power of two, as the
quotient's multiple: `(16 * x) >> 4` is `x`, syntactically, where a product
with `x` is then one atom to the deciders. The kernel decides the literal
equation. -/
simproc_decl reduceShiftRightMul (Int.shiftRight (HMul.hMul _ _) _) := fun e => do
  let args := e.getAppArgs
  let product := args[0]!
  unless product.isAppOfArity ``HMul.hMul 6 do return .continue
  let (some multiplier, some distance) := (product.getArg! 4 |>.int?, args[1]?.bind Lean.Expr.nat?)
    | return .continue
  unless ← checkExponent distance (warning := false) do return .continue
  let power : Int := 2 ^ distance
  unless multiplier % power == 0 do return .continue
  let quotient := multiplier / power
  let value := product.getArg! 5
  let powerExpr ← mkAppM ``HPow.hPow #[mkIntLit 2, mkNatLit distance]
  if quotient == 1 then
    let some isPower ← decidedFrameFree? (← mkEq (product.getArg! 4) powerExpr)
      | return .continue
    let proof ← mkAppM ``shiftRight_pow_mul #[value, mkNatLit distance, isPower]
    return .done { expr := value, proof? := some proof }
  let some isMultiple ← decidedFrameFree?
      (← mkEq (product.getArg! 4) (← mkAppM ``HMul.hMul #[mkIntLit quotient, powerExpr]))
    | return .continue
  let proof ← mkAppM ``shiftRight_mul_pow #[value, mkNatLit distance, isMultiple]
  return .done { expr := ← mkAppM ``HMul.hMul #[mkIntLit quotient, value], proof? := some proof }

attribute [vir_denote, vir_denote_norm] reduceShiftRightMul

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
    let τView ← nativeTypeView τ
    let .const name _ := τView.getAppFn | continue
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
  variantPlainToSkolemConstructor hlistPlainToSkolemConstructor
  ofSkolemIdentity
  toSkolemIdentity hlistOfSkolemIdentity hlistToSkolemIdentity variantOfSkolemIdentity
  variantToSkolemIdentity plainOfSkolemIdentity plainToSkolemIdentity
  hlistPlainOfSkolemIdentity hlistPlainToSkolemIdentity variantPlainOfSkolemIdentity
  variantPlainToSkolemIdentity inhabitantOuterFamily NativeMaps.NTy.read_param_instantiate
  NativeMaps.NTy.unread_param_instantiate
  NativeMaps.ofEntriesTransport
  Carriers.orders_instantiate Carriers.erasures_instantiate NTy.ofSkolem_toSkolem
  NTy.toSkolem_unit NTy.toSkolem_bool NTy.toSkolem_int NTy.toSkolem_num NTy.toSkolem_param
  NTy.toSkolem_tuple NTy.toSkolem_struct NTy.toSkolem_enum NTy.toSkolem_ref NTy.toSkolem_ghost
  HList.toSkolem_nil HList.toSkolem_fst HList.toSkolem_snd HList.toSkolem_mk
  variantCarrier.toSkolem_inl variantCarrier.toSkolem_inr ResultShape.toSkolem
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

-- Complete an action before traversing the continuation it selects.
attribute [vir_denote ↓] VerexIR.Proofs.Denote.Flow.bind_pure_value
  VerexIR.Proofs.Denote.Flow.bind_pure_return VerexIR.Proofs.Denote.Flow.bind_pure_break
  VerexIR.Proofs.Denote.Flow.bind_pure_continue VerexIR.Proofs.Denote.Flow.bind_abort
  VerexIR.Proofs.Denote.Flow.bind_abortAt VerexIR.Proofs.Spec.pure_bind

open Lean Meta Simp in
/-- A completed action's `wp` before its continuation is traversed:
`wp_pure` and `wp_abort` at a `wp` itself. Not as rules taken before the
arguments: a path's markers (`Taken`, `Matched`, `Dispatched`, `AfterAt`,
`AbortsAt`) are reducible, so such a rule matches a marked `wp` at the
marker and rewrites the marker away with it, and the path loses its
decision or point. -/
def completedAction (e : Lean.Expr) : SimpM Step := do
  unless e.isAppOfArity ``VerexIR.Proofs.wp 7 do return .continue
  let action := (e.getArg! 3).consumeMData
  let ensures := e.getArg! 4
  let aborts := e.getArg! 5
  let state := e.getArg! 6
  let (rule, result) ←
    if action.isAppOfArity ``VerexIR.Proofs.Spec.pure 4 then
      let value := action.getArg! 3
      pure (← mkAppM ``VerexIR.Proofs.wp_pure #[value, state, ensures, aborts],
        (mkApp2 ensures value state).headBeta)
    else if action.isAppOfArity ``VerexIR.Proofs.Spec.abort 4 then
      let error := action.getArg! 3
      pure (← mkAppM ``VerexIR.Proofs.wp_abort #[error, state, ensures, aborts],
        (mkApp aborts error).headBeta)
    else return .continue
  return .visit { expr := result, proof? := some (← mkPropExt rule) }

simproc ↓ [vir_denote] completedPure (VerexIR.Proofs.wp (VerexIR.Proofs.Spec.pure _) _ _ _) :=
  completedAction
simproc ↓ [vir_denote] completedAbort (VerexIR.Proofs.wp (VerexIR.Proofs.Spec.abort _) _ _ _) :=
  completedAction

end VerexIR.Proofs.Denote

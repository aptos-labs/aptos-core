-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Oracle
import LeanerIR.Proofs.Denotation
import LeanerIR.Proofs.DenotationWP
import LeanerIR.Proofs.Typed

/-!
# Recursive native denotations

A recursive function's body has no callee relation to unfold into: its
denotation is the least body closed under one unfolding (`fixBody`), where
the generated body places `nativeFunctionRelation unit shape self` at every
recursive call.  Two facts connect it to the rest of the stack:

* **Agreement.** The body agrees with the open semantics under the oracle
  that answers the function's own handle with the relation of `self` and
  every other handle with the closed semantics (`oracle`), for every
  `self`.  That parametric agreement is what the generator proves with the
  ordinary combinator lemmas; `fixBody_agrees` turns it into exact
  agreement of the fixed point with the closed semantics, by the
  least-fixed-point induction of the rules (`EvalFunction.induction`).
* **Verification.** A contract holds of the fixed point when it holds of
  the body under the hypothesis that it holds of the recursive calls
  (`satisfies_typed_fixBody`) — the recursive hypothesis a row script
  consumes at a recursive call exactly as it consumes a verified callee.

`fixBody` is impredicative on purpose: no fuel is threaded through the
denotation and no continuity of the combinators is needed.
-/

namespace LeanerIR.Proofs.Denotation

open LeanerIR.Validation
open LeanerIR.BigStep
open LeanerIR.SemanticOperations

/-! ## The oracle of a recursive body -/

/-- The callee oracle a recursive function's body is denoted against: its
own handle answered by `self`, every other handle by the closed
semantics. -/
def oracle (unit : ExecutableUnit) (handle : FunctionHandle)
    (self : FunctionDenotation) : CalleeRelation :=
  fun callee => if callee = handle then self else EvalFunction unit callee

theorem oracle_self (unit : ExecutableUnit) (handle : FunctionHandle)
    (self : FunctionDenotation) : oracle unit handle self handle = self := by
  simp [oracle]

theorem oracle_other (unit : ExecutableUnit) {handle callee : FunctionHandle}
    (self : FunctionDenotation) (ne : callee ≠ handle) :
    oracle unit handle self callee = EvalFunction unit callee := by
  simp [oracle, ne]

theorem oracle_le {unit : ExecutableUnit} {handle : FunctionHandle}
    {self self' : FunctionDenotation}
    (le : ∀ initial arguments final outcome,
      self initial arguments final outcome → self' initial arguments final outcome) :
    ∀ callee initial arguments final outcome,
      oracle unit handle self callee initial arguments final outcome →
        oracle unit handle self' callee initial arguments final outcome := by
  intro callee initial arguments final outcome fact
  by_cases eq : callee = handle
  · subst eq
    rw [oracle_self] at fact ⊢
    exact le _ _ _ _ fact
  · rw [oracle_other unit _ eq] at fact ⊢
    exact fact

/-- The recursive call agrees with the oracle by construction. -/
theorem FunctionDenotation.agreesWith_oracle_self (unit : ExecutableUnit)
    (handle : FunctionHandle) (self : FunctionDenotation) :
    FunctionDenotation.AgreesWith (oracle unit handle self) handle self := by
  intro initial arguments final outcome
  rw [oracle_self]

/-- Any other callee keeps its closed agreement under the oracle. -/
theorem FunctionDenotation.agreesWith_oracle_other {unit : ExecutableUnit}
    {handle other : FunctionHandle} (self : FunctionDenotation)
    {denotation : FunctionDenotation} (ne : other ≠ handle)
    (agrees : FunctionDenotation.Agrees unit other denotation) :
    FunctionDenotation.AgreesWith (oracle unit handle self) other denotation := by
  intro initial arguments final outcome
  rw [oracle_other unit _ ne]
  exact agrees initial arguments final outcome

theorem nativeFunctionRelation_mono {unit : ExecutableUnit} {shape : FunctionShape}
    {body body' : ExprDenotation}
    (le : ∀ frame state finalFrame finalState control,
      body frame state finalFrame finalState control →
        body' frame state finalFrame finalState control) :
    ∀ initial arguments final outcome,
      nativeFunctionRelation unit shape body initial arguments final outcome →
        nativeFunctionRelation unit shape body' initial arguments final outcome := by
  rintro initial arguments final outcome
    ⟨frame, finalFrame, evaluatedState, control, frame_eq, step, outcome_eq, finalize_eq⟩
  exact ⟨frame, finalFrame, evaluatedState, control, frame_eq,
    le _ _ _ _ _ step, outcome_eq, finalize_eq⟩

/-! ## The fixed point -/

/-- The least body closed under one unfolding of `body`. -/
def fixBody (body : ExprDenotation → ExprDenotation) : ExprDenotation :=
  fun frame state finalFrame finalState control =>
    ∀ closed : ExprDenotation,
      (∀ frame state finalFrame finalState control,
        body closed frame state finalFrame finalState control →
          closed frame state finalFrame finalState control) →
      closed frame state finalFrame finalState control

theorem fixBody_le {body : ExprDenotation → ExprDenotation} {closed : ExprDenotation}
    (closure : ∀ frame state finalFrame finalState control,
      body closed frame state finalFrame finalState control →
        closed frame state finalFrame finalState control) :
    ∀ frame state finalFrame finalState control,
      fixBody body frame state finalFrame finalState control →
        closed frame state finalFrame finalState control :=
  fun _ _ _ _ _ step => step closed closure

theorem fixBody_fold {body : ExprDenotation → ExprDenotation}
    (mono : ∀ inner outer : ExprDenotation,
      (∀ frame state finalFrame finalState control,
        inner frame state finalFrame finalState control →
          outer frame state finalFrame finalState control) →
      ∀ frame state finalFrame finalState control,
        body inner frame state finalFrame finalState control →
          body outer frame state finalFrame finalState control) :
    ∀ frame state finalFrame finalState control,
      body (fixBody body) frame state finalFrame finalState control →
        fixBody body frame state finalFrame finalState control := by
  intro frame state finalFrame finalState control step closed closure
  exact closure _ _ _ _ _
    (mono (fixBody body) closed (fixBody_le closure) _ _ _ _ _ step)

/-- The executions admitted by some body with a property. -/
def bodiesUnion (property : ExprDenotation → Prop) : ExprDenotation :=
  fun frame state finalFrame finalState control =>
    ∃ body, property body ∧ body frame state finalFrame finalState control

/-- Induction for a pointwise property of bodies: one that is antitone and
holds of the union of the bodies having it, such as contract satisfaction.
The step may assume the property of the recursive calls. -/
theorem fixBody_induction {body : ExprDenotation → ExprDenotation}
    (property : ExprDenotation → Prop)
    (antitone : ∀ inner outer : ExprDenotation,
      (∀ frame state finalFrame finalState control,
        inner frame state finalFrame finalState control →
          outer frame state finalFrame finalState control) →
      property outer → property inner)
    (union : property (bodiesUnion property))
    (step : ∀ self, property self → property (body self)) :
    property (fixBody body) := by
  apply antitone _ (bodiesUnion property) _ union
  apply fixBody_le
  intro frame state finalFrame finalState control unfolded
  exact ⟨body (bodiesUnion property), step _ union, unfolded⟩

/-! ## Agreement of the fixed point with the closed semantics -/

theorem fixBody_agrees {unit : ExecutableUnit} {handle : FunctionHandle}
    {ns : ValidatedNamespace} {declaration : FunctionDecl FunctionBody}
    {root : ExprId} {shape : FunctionShape}
    {body : ExprDenotation → ExprDenotation}
    (namespace_eq : unit.unit.namespaces[handle.namespaceId.index]? = some ns)
    (declaration_eq : ns.functions[handle.functionId.index]? = some declaration)
    (body_eq : declaration.body = .structured root)
    (shape_eq : shape = .ofDeclaration declaration)
    (parametric : ∀ self : ExprDenotation,
      ExprDenotation.AgreesWith unit
        (oracle unit handle (nativeFunctionRelation unit shape self))
        handle.namespaceId root (body self)) :
    FunctionDenotation.Agrees unit handle
      (nativeFunctionRelation unit shape (fixBody body)) := by
  subst shape_eq
  have bodyMono : ∀ inner outer : ExprDenotation,
      (∀ frame state finalFrame finalState control,
        inner frame state finalFrame finalState control →
          outer frame state finalFrame finalState control) →
      ∀ frame state finalFrame finalState control,
        body inner frame state finalFrame finalState control →
          body outer frame state finalFrame finalState control := by
    intro inner outer le frame state finalFrame finalState control step
    exact (parametric outer _ _ _ _ _).mpr
      (EvalExprWith.mono (oracle_le (nativeFunctionRelation_mono le))
        ((parametric inner _ _ _ _ _).mp step))
  let deepBody : ExprDenotation := fun frame state finalFrame finalState control =>
    EvalExpr unit handle.namespaceId frame state root finalFrame finalState control
  have forward : ∀ initial arguments final outcome,
      nativeFunctionRelation unit (.ofDeclaration declaration) (fixBody body)
        initial arguments final outcome →
      EvalFunction unit handle initial arguments final outcome := by
    rintro initial arguments final outcome
      ⟨frame, finalFrame, evaluatedState, control, frame_eq, step, outcome_eq,
        finalize_eq⟩
    have closure : ∀ frame state finalFrame finalState control,
        body deepBody frame state finalFrame finalState control →
          deepBody frame state finalFrame finalState control := by
      intro frame state finalFrame finalState control unfolded
      refine EvalExprWith.mono ?_ ((parametric deepBody _ _ _ _ _).mp unfolded)
      intro callee initial arguments final outcome fact
      by_cases eq : callee = handle
      · subst eq
        rw [oracle_self] at fact
        obtain ⟨frame', finalFrame', evaluatedState', control', frame_eq', step',
          outcome_eq', finalize_eq'⟩ := fact
        exact .body callee initial arguments ns declaration frame' root finalFrame'
          evaluatedState' final control' outcome namespace_eq declaration_eq
          frame_eq' body_eq step' outcome_eq' finalize_eq'
      · rw [oracle_other unit _ eq] at fact
        exact fact
    exact .body handle initial arguments ns declaration frame root finalFrame
      evaluatedState final control outcome namespace_eq declaration_eq frame_eq
      body_eq (fixBody_le closure _ _ _ _ _ step) outcome_eq finalize_eq
  have reverse : ∀ initial arguments final outcome,
      EvalFunction unit handle initial arguments final outcome →
      nativeFunctionRelation unit (.ofDeclaration declaration) (fixBody body)
        initial arguments final outcome := by
    intro initial arguments final outcome step
    have below := EvalFunction.induction
      (oracle unit handle
        (nativeFunctionRelation unit (.ofDeclaration declaration) (fixBody body)))
      ?closed step
    · rw [oracle_self] at below
      exact below
    intro callee initial arguments final outcome opened
    by_cases eq : callee = handle
    · subst eq
      rw [oracle_self]
      obtain ⟨ns', declaration', frame, root', finalFrame, evaluatedState, control,
        namespace_eq', declaration_eq', frame_eq, body_eq', bodyStep, outcome_eq,
        finalize_eq⟩ := opened
      rw [namespace_eq, Option.some.injEq] at namespace_eq'
      subst namespace_eq'
      rw [declaration_eq, Option.some.injEq] at declaration_eq'
      subst declaration_eq'
      rw [body_eq, FunctionBody.structured.injEq] at body_eq'
      subst body_eq'
      exact ⟨frame, finalFrame, evaluatedState, control, frame_eq,
        fixBody_fold bodyMono _ _ _ _ _
          ((parametric (fixBody body) _ _ _ _ _).mpr bodyStep),
        outcome_eq, finalize_eq⟩
    · rw [oracle_other unit _ eq]
      apply EvalFunction.fold
      refine EvalFunctionWith.mono ?_ opened
      intro callee' initial arguments final outcome fact
      by_cases eq' : callee' = handle
      · subst eq'
        rw [oracle_self] at fact
        exact forward _ _ _ _ fact
      · rw [oracle_other unit _ eq'] at fact
        exact fact
  intro initial arguments final outcome
  exact ⟨forward initial arguments final outcome, reverse initial arguments final outcome⟩

/-! ## Verification of the fixed point -/

/-- `Satisfies` is the weakest precondition of every permitted call. -/
theorem satisfies_iff_wp {function : Args → Spec σ ε Result}
    {contract : Contract σ ε Args Result} :
    Satisfies function contract ↔
      ∀ args initial, contract.requires args initial →
        wp (function args)
          (fun result final =>
            (¬contract.mayAbort args initial →
              contract.ensures args initial result final) ∧
            contract.frame args initial final ∧
            ¬contract.mustAbort args initial)
          (contract.aborts args initial)
          initial :=
  Iff.rfl

section Typed

variable {unit : ExecutableUnit} {shape : FunctionShape}
  {NativeArgs NativeResult : Type}
  {argumentsCodec : Codec NativeArgs (Array RuntimeValue)}
  {resultsCodec : Codec NativeResult (Array RuntimeValue)}
  {contract : Contract RuntimeState Failure NativeArgs NativeResult}

theorem satisfies_typed_nativeFunction_antitone {inner outer : ExprDenotation}
    (le : ∀ frame state finalFrame finalState control,
      inner frame state finalFrame finalState control →
        outer frame state finalFrame finalState control)
    (satisfied : Satisfies
      (typedFunction argumentsCodec resultsCodec (nativeFunction unit shape outer))
      contract) :
    Satisfies
      (typedFunction argumentsCodec resultsCodec (nativeFunction unit shape inner))
      contract := by
  rw [satisfies_iff_wp] at satisfied ⊢
  intro args initial permitted
  have established := satisfied args initial permitted
  rw [wp_typedFunction, wp_nativeFunction] at established ⊢
  unfold wpExpr at established ⊢
  intro frame frame_eq finalFrame finalState control step
  exact established frame frame_eq finalFrame finalState control
    (le _ _ _ _ _ step)

theorem satisfies_typed_nativeFunction_union :
    Satisfies
      (typedFunction argumentsCodec resultsCodec
        (nativeFunction unit shape (bodiesUnion fun self =>
          Satisfies
            (typedFunction argumentsCodec resultsCodec (nativeFunction unit shape self))
            contract)))
      contract := by
  rw [satisfies_iff_wp]
  intro args initial permitted
  rw [wp_typedFunction, wp_nativeFunction]
  unfold wpExpr
  intro frame frame_eq finalFrame finalState control step
  obtain ⟨self, satisfied, step⟩ := step
  have established := (satisfies_iff_wp.mp satisfied) args initial permitted
  rw [wp_typedFunction, wp_nativeFunction] at established
  unfold wpExpr at established
  exact established frame frame_eq finalFrame finalState control step

/-- Fixed-point induction for the typed contract of a recursive function:
prove the body under the hypothesis that the recursive calls satisfy the
contract. -/
theorem satisfies_typed_fixBody {body : ExprDenotation → ExprDenotation}
    (step : ∀ self : ExprDenotation,
      Satisfies
        (typedFunction argumentsCodec resultsCodec (nativeFunction unit shape self))
        contract →
      Satisfies
        (typedFunction argumentsCodec resultsCodec
          (nativeFunction unit shape (body self)))
        contract) :
    Satisfies
      (typedFunction argumentsCodec resultsCodec
        (nativeFunction unit shape (fixBody body)))
      contract :=
  fixBody_induction
    (fun self =>
      Satisfies
        (typedFunction argumentsCodec resultsCodec (nativeFunction unit shape self))
        contract)
    (fun _ _ le satisfied => satisfies_typed_nativeFunction_antitone le satisfied)
    satisfies_typed_nativeFunction_union step

end Typed

end LeanerIR.Proofs.Denotation

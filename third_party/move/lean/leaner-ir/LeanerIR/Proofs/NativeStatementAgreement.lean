-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.NativeValueAgreement

/-! Statement-to-expression agreement. These laws retain slot availability,
evaluation order, and abrupt control while native locals use typed bindings. -/

namespace LeanerIR.Proofs.ComputationAgreement
open LeanerIR.SemanticOperations LeanerIR.BigStep Denotation

theorem block_nil (body : ExprDenotation)
    (entry : RuntimeFrame) (initial : RuntimeState) (finalFrame : RuntimeFrame)
    (finalState : RuntimeState) (control : Control) :
    blockResult statementsNil body entry initial finalFrame finalState control ↔
      body entry initial finalFrame finalState control := by
  unfold blockResult statementsNil
  constructor
  · rintro (impossible | ⟨_, _, equal, executed⟩)
    · cases impossible
    · cases equal; exact executed
  · intro executed; exact Or.inr ⟨entry, initial, rfl, executed⟩

/-- Assignment followed by a continuation is a checked rebinding, not a
native runtime-frame update. A failing initializer never enters the tail. -/
theorem block_assign (localId : LocalId) (value body : ExprDenotation)
    (tail : StatementsDenotation)
    (entry : RuntimeFrame) (initial : RuntimeState) (finalFrame : RuntimeFrame)
    (finalState : RuntimeState) (control : Control) :
    blockResult (statementsCons (nativeAssignLocal localId value) tail) body
      entry initial finalFrame finalState control ↔
    letNativeValue ⟨1, .variable localId⟩ value (blockResult tail body)
      entry initial finalFrame finalState control := by
  unfold blockResult statementsCons nativeAssignLocal letNativeValue
  constructor
  · rintro ((⟨frame, state, actual, assigned, abrupt, equal⟩ |
        ⟨frame, state, actual, assigned, continued⟩) |
      ⟨frame, state, (⟨_, _, _, _, _, impossible⟩ |
        ⟨headFrame, headState, actual, assigned, continued⟩), result⟩)
    · cases equal
      rcases assigned with ⟨executed, _⟩ | ⟨_, _, _, _, _, _, _, rfl⟩
      · exact Or.inl ⟨executed, abrupt⟩
      · cases abrupt
    · rcases assigned with ⟨_, abrupt⟩ | ⟨valueFrame, valueState, value, executed, bound, rfl, rfl, equal⟩
      · cases abrupt
      · cases equal
        exact Or.inr ⟨valueFrame, state, value, _, executed,
          by simp [NativePatternBinder.bind, bindNativePatternFuel, bound], Or.inl continued⟩
    · cases impossible
    · rcases assigned with ⟨_, abrupt⟩ | ⟨valueFrame, valueState, value, executed, bound, rfl, rfl, equal⟩
      · cases abrupt
      · cases equal
        exact Or.inr ⟨valueFrame, headState, value, _, executed,
          by simp [NativePatternBinder.bind, bindNativePatternFuel, bound],
          Or.inr ⟨frame, state, continued, result⟩⟩
  · rintro (⟨executed, abrupt⟩ | ⟨frame, state, value, boundFrame, executed, binding, continued⟩)
    · exact Or.inl (Or.inl ⟨finalFrame, finalState, control, Or.inl ⟨executed, abrupt⟩, abrupt, rfl⟩)
    · simp only [NativePatternBinder.bind, bindNativePatternFuel] at binding
      split at binding
      next bound =>
        cases binding
        rcases continued with continued | ⟨lastFrame, lastState, continued, result⟩
        · exact Or.inl (Or.inr ⟨_, state, .unit,
            Or.inr ⟨frame, state, value, executed, bound, rfl, rfl, rfl⟩, continued⟩)
        · exact Or.inr ⟨lastFrame, lastState,
            Or.inr ⟨_, state, .unit,
              Or.inr ⟨frame, state, value, executed, bound, rfl, rfl, rfl⟩, continued⟩, result⟩
      next => cases binding

theorem unit_single_assign (localId : LocalId) (value : ExprDenotation) :
    blockUnit (statementsCons (nativeAssignLocal localId value) statementsNil) =
      nativeAssignLocal localId value := by
  funext entry initial finalFrame finalState control
  apply propext
  simp only [blockUnit, statementsCons, statementsNil, nativeAssignLocal]
  grind [Abrupt]

theorem branch_assign (localId : LocalId) (condition yes no : ExprDenotation) :
    nativeBranch condition (nativeAssignLocal localId yes) (some (nativeAssignLocal localId no)) =
      nativeAssignLocal localId (nativeBranch condition yes (some no)) := by
  funext entry initial finalFrame finalState control
  apply propext
  simp only [nativeBranch, nativeAssignLocal]
  constructor
  · rintro (⟨tested, abrupt⟩ | ⟨frame, state, tested, assigned⟩ | ⟨frame, state, tested, assigned⟩)
    · exact Or.inl ⟨Or.inl ⟨tested, abrupt⟩, abrupt⟩
    · rcases assigned with ⟨ran, abrupt⟩ | ⟨vf, vs, value, ran, bound, feq, seq, equal⟩
      · exact Or.inl ⟨Or.inr (Or.inl ⟨frame, state, tested, ran⟩), abrupt⟩
      · exact Or.inr ⟨vf, vs, value, Or.inr (Or.inl ⟨frame, state, tested, ran⟩),
          bound, feq, seq, equal⟩
    · rcases assigned with ⟨ran, abrupt⟩ | ⟨vf, vs, value, ran, bound, feq, seq, equal⟩
      · exact Or.inl ⟨Or.inr (Or.inr ⟨frame, state, tested, ran⟩), abrupt⟩
      · exact Or.inr ⟨vf, vs, value, Or.inr (Or.inr ⟨frame, state, tested, ran⟩),
          bound, feq, seq, equal⟩
  · rintro (⟨(⟨tested, stopped⟩ | ⟨frame, state, tested, ran⟩ | ⟨frame, state, tested, ran⟩), abrupt⟩ |
      ⟨vf, vs, value, (⟨_, stopped⟩ | ⟨frame, state, tested, ran⟩ | ⟨frame, state, tested, ran⟩),
        bound, feq, seq, equal⟩)
    · exact Or.inl ⟨tested, stopped⟩
    · exact Or.inr (Or.inl ⟨frame, state, tested, Or.inl ⟨ran, abrupt⟩⟩)
    · exact Or.inr (Or.inr ⟨frame, state, tested, Or.inl ⟨ran, abrupt⟩⟩)
    · cases stopped
    · exact Or.inr (Or.inl ⟨frame, state, tested, Or.inr ⟨vf, vs, value, ran, bound, feq, seq, equal⟩⟩)
    · exact Or.inr (Or.inr ⟨frame, state, tested, Or.inr ⟨vf, vs, value, ran, bound, feq, seq, equal⟩⟩)

/-- Normalize only the statement head; leave the continuation opaque. -/
theorem block_assign_head (head : ExprDenotation) (localId : LocalId)
    (value body : ExprDenotation) (tail : StatementsDenotation)
    (equal : head = nativeAssignLocal localId value)
    (entry : RuntimeFrame) (initial : RuntimeState) (finalFrame : RuntimeFrame)
    (finalState : RuntimeState) (control : Control) :
    blockResult (statementsCons head tail) body entry initial finalFrame finalState control ↔
      letNativeValue ⟨1, .variable localId⟩ value (blockResult tail body)
        entry initial finalFrame finalState control := by
  rw [equal]
  exact block_assign _ _ _ _ _ _ _ _ _

theorem block_discard (head body : ExprDenotation) (tail : StatementsDenotation) :
    blockResult (statementsCons head tail) body =
      letNativeValue ⟨1, .wildcard⟩ head (blockResult tail body) := by
  funext entry initial finalFrame finalState control
  apply propext
  simp only [blockResult, statementsCons, letNativeValue,
    NativePatternBinder.bind, bindNativePatternFuel]
  grind [Abrupt]

theorem block_unit_result (statements : StatementsDenotation) :
    blockUnit statements = blockResult statements (value .unit) := by
  funext entry initial finalFrame finalState control
  apply propext
  simp [blockUnit, blockResult, value]
  grind

theorem branch_none (condition yes : ExprDenotation) :
    nativeBranch condition yes none = nativeBranch condition yes (some (value .unit)) := rfl

theorem block_nil_eq (body : ExprDenotation) : blockResult statementsNil body = body := by
  funext entry initial finalFrame finalState control
  exact propext (block_nil _ _ _ _ _ _)

end LeanerIR.Proofs.ComputationAgreement

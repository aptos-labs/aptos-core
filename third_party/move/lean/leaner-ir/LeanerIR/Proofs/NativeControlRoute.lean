-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.NativeObservedFlow
import LeanerIR.Proofs.NativeNestedFlow

namespace LeanerIR.Proofs.ComputationAgreement

open LeanerIR.BigStep Denotation

/-- Control encoding belongs only to execution agreement. The native target
type may be a heterogeneous sum of outer headers or a typed return value. -/
structure ControlRoute (Target : Type) where
  continue_ : Target → Control
  break_ : Target → Control
  continueAbrupt : ∀ target, Abrupt (continue_ target)
  breakAbrupt : ∀ target, Abrupt (break_ target)
  continueValid : ∀ target, NonThrow (continue_ target)
  breakValid : ∀ target, NonThrow (break_ target)

namespace ControlRoute

/-- The outermost source loop keeps the compact single-header encoding. -/
def root : ControlRoute Locals where
  continue_ := fun _ => .continue_ 0
  break_ := fun _ => .break_ 0 none
  continueAbrupt := fun _ => .continue_ _
  breakAbrupt := fun _ => .break_ _ _
  continueValid := fun _ => trivial
  breakValid := fun _ => trivial

def encode (route : ControlRoute Target) : NativeFlow.Flow Locals Target → Control
  | .normal _ => .value .unit
  | .continue_ target => route.continue_ target
  | .break_ target => route.break_ target

theorem valid (route : ControlRoute Target) (flow : NativeFlow.Flow Locals Target) :
    NonThrow (route.encode flow) := by
  cases flow with
  | normal _ => trivial
  | continue_ target => exact route.continueValid target
  | break_ target => exact route.breakValid target

theorem root_encode : (root : ControlRoute Target).encode =
    (flowControl : NativeFlow.Flow Locals Target → Control) := by
  funext flow
  cases flow <;> rfl

def raise : Control → Control
  | .continue_ depth => .continue_ (depth + 1)
  | .break_ depth value => .break_ (depth + 1) value
  | control => control

theorem raise_abrupt (control : Control) (abrupt : Abrupt control) : Abrupt (raise control) := by
  cases abrupt <;> constructor

theorem raise_valid (control : Control) (valid : NonThrow control) : NonThrow (raise control) := by
  cases control <;> exact valid

def push (route : ControlRoute Outer) : ControlRoute (Sum Locals Outer) where
  continue_ := fun | .inl _ => .continue_ 0 | .inr target => raise (route.continue_ target)
  break_ := fun | .inl _ => .break_ 0 none | .inr target => raise (route.break_ target)
  continueAbrupt := by rintro (locals | target); exact .continue_ _; exact raise_abrupt _ (route.continueAbrupt target)
  breakAbrupt := by rintro (locals | target); exact .break_ _ _; exact raise_abrupt _ (route.breakAbrupt target)
  continueValid := by rintro (locals | target); trivial; exact raise_valid _ (route.continueValid target)
  breakValid := by rintro (locals | target); trivial; exact raise_valid _ (route.breakValid target)

def empty : ControlRoute Empty where
  continue_ := fun target => nomatch target
  break_ := fun target => nomatch target
  continueAbrupt := fun target => nomatch target
  breakAbrupt := fun target => nomatch target
  continueValid := fun target => nomatch target
  breakValid := fun target => nomatch target

theorem raised_loop (body : ExprDenotation) (control : Control)
    (abrupt : Abrupt control) (valid : NonThrow control)
    (ran : body frame initial finalFrame finalState (raise control)) :
    Denotation.NativeLoop body frame initial finalFrame finalState control := by
  cases abrupt with
  | continue_ depth => exact .outerContinue depth ran
  | break_ depth value => exact .outerBreak depth value ran
  | return_ values => exact .return_ values ran
  | throw_ kind values => exact valid.elim

end ControlRoute

theorem observed_routed_sequence (route : ControlRoute LoopLocals) (head body : ExprDenotation) (tail : StatementsDenotation)
    (entry : RuntimeFrame) (joinFrames : Join → RuntimeFrame → Prop)
    (frames : Locals → RuntimeFrame → Prop) (loopFrames : LoopLocals → RuntimeFrame → Prop)
    (first : Spec RuntimeState Failure (NativeFlow.Flow Join LoopLocals))
    (next : Join → Spec RuntimeState Failure (NativeFlow.Flow Locals LoopLocals))
    (headAgreement : Observed head entry (flowFrames joinFrames loopFrames) route.encode first)
    (continuation : ∀ locals frame, joinFrames locals frame →
      Observed (blockResult tail body) frame (flowFrames frames loopFrames) route.encode (next locals)) :
    Observed (blockResult (statementsCons head tail) body) entry
      (flowFrames frames loopFrames) route.encode (NativeFlow.sequence first next) := by
  rw [block_discard]
  constructor
  · intro initial finalFrame finalState control nonThrow
    rintro (⟨ran, abrupt⟩ | ⟨headFrame, headState, actual, boundFrame, ran, binds, continued⟩)
    · obtain ⟨flow, step, same, equal⟩ := headAgreement.sound _ _ _ _ nonThrow ran
      cases flow with
      | normal locals => cases equal; cases abrupt
      | continue_ locals =>
        exact ⟨.continue_ locals, ⟨.continue_ locals, finalState, step, rfl, rfl⟩, same, equal⟩
      | break_ locals =>
        exact ⟨.break_ locals, ⟨.break_ locals, finalState, step, rfl, rfl⟩, same, equal⟩
    · obtain ⟨flow, step, same, equal⟩ := headAgreement.sound _ _ _ (.value actual) trivial ran
      cases binds
      cases flow with
      | normal locals =>
        cases equal
        obtain ⟨result, rest, same, equal⟩ :=
          (continuation locals headFrame same).sound _ _ _ _ nonThrow continued
        exact ⟨result, ⟨.normal locals, headState, step, rest⟩, same, equal⟩
      | continue_ target =>
        have bad : Abrupt (.value actual) := equal.symm ▸ route.continueAbrupt target
        cases bad
      | break_ target =>
        have bad : Abrupt (.value actual) := equal.symm ▸ route.breakAbrupt target
        cases bad
  · rintro initial result finalState ⟨flow, middle, step, rest⟩
    obtain ⟨headFrame, ran, same⟩ := headAgreement.complete _ _ _ step
    cases flow with
    | normal locals =>
      obtain ⟨finalFrame, continued, observes⟩ := (continuation locals headFrame same).complete _ _ _ rest
      exact ⟨finalFrame, Or.inr ⟨headFrame, middle, .unit, headFrame, ran, rfl, continued⟩, observes⟩
    | continue_ locals =>
      obtain ⟨rfl, rfl⟩ := rest
      exact ⟨headFrame, Or.inl ⟨ran, route.continueAbrupt locals⟩, same⟩
    | break_ locals =>
      obtain ⟨rfl, rfl⟩ := rest
      exact ⟨headFrame, Or.inl ⟨ran, route.breakAbrupt locals⟩, same⟩
  · intro initial error
    constructor
    · rintro ⟨finalFrame, finalState, (⟨ran, _⟩ |
        ⟨headFrame, headState, actual, boundFrame, ran, binds, continued⟩)⟩
      · exact Or.inl ((headAgreement.aborts _ _).mp ⟨_, _, ran⟩)
      · obtain ⟨flow, step, same, equal⟩ := headAgreement.sound _ _ _ (.value actual) trivial ran
        cases binds
        cases flow with
        | normal locals =>
          cases equal
          exact Or.inr ⟨.normal locals, headState, step,
            (continuation locals headFrame same).aborts _ _ |>.mp ⟨_, _, continued⟩⟩
        | continue_ target =>
          have bad : Abrupt (.value actual) := equal.symm ▸ route.continueAbrupt target
          cases bad
        | break_ target =>
          have bad : Abrupt (.value actual) := equal.symm ▸ route.breakAbrupt target
          cases bad
    · rintro (failed | ⟨flow, middle, step, failed⟩)
      · obtain ⟨frame, state, ran⟩ := (headAgreement.aborts _ _).mpr failed
        exact ⟨frame, state, Or.inl ⟨ran, .throw_ _ _⟩⟩
      · obtain ⟨headFrame, step, same⟩ := headAgreement.complete _ _ _ step
        cases flow with
        | normal locals =>
          obtain ⟨lastFrame, state, ran⟩ := (continuation locals headFrame same).aborts _ _ |>.mpr failed
          exact ⟨lastFrame, state, Or.inr ⟨headFrame, middle, .unit, headFrame, step, rfl, ran⟩⟩
        | continue_ _ => cases failed
        | break_ _ => cases failed
  · intro initial undefined
    rcases undefined with undefined | ⟨flow, middle, ran, undefined⟩
    · exact headAgreement.defined _ undefined
    · obtain ⟨frame, _, same⟩ := headAgreement.complete _ _ _ ran
      cases flow with
      | normal locals => exact (continuation locals frame same).defined _ undefined
      | continue_ _ => cases undefined
      | break_ _ => cases undefined
  · exact route.valid

end LeanerIR.Proofs.ComputationAgreement

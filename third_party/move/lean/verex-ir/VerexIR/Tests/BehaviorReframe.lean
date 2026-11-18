-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.BehaviorReframe
open VerexIR VerexIR.Proofs VerexIR.Proofs.Denote

/-! # A function value's behavior at the frame of a declaration's type arguments

The behavioral predicates of a function value moved to the frame of a
declaration's type arguments (`Skolems.reframe`) are those of the value at
the frame it was moved from (`ValidResultsAt.reframe_iff`,
`FramedAt.reframe_iff`, `KeepsMemoryAt.reframe_iff`), at rows without
references, where a run reads its arguments and its result through their
encodings (`closureRun_ok_iff_of_encode`).

The laws are instantiated at a closed row, at a row of the declaration's
parameter, and at a function returning nothing.  A frame that reads its
arguments is read, at the frame, at the arguments moved to the induced one.

Negatives first: a row with a reference gives no premise for the law of
valid results, and the frame predicate states nothing there, at either
frame.
-/

namespace VerexIR.Tests.BehaviorReframe

variable {unit : Validation.ValidatedUnit} [FunctionCarriers unit]
  (executable : Validation.ExecutableUnit unit) (Θ : Skolems unit)

/-- The declaration's one type parameter. -/
abbrev params : NRow := .cons (.param 0) .nil

theorem own : params.isParamsFrom 0 = true := rfl

abbrev scalar : NRow := .cons (.int 64 false) .nil
abbrev generic : NRow := .cons (.param 0) .nil
abbrev borrowed : NRow := .cons (.ref (.int 64 false)) .nil
abbrev shared : Sharing := ⟨[false], [false]⟩

/-! ## A row with a reference -/

/-- The row gives no premise for the law of valid results. -/
example : borrowed.refFree = false := rfl

/-- The frame predicate states nothing at such a row, whatever the frame and
the value. -/
example (frame : @HList (Θ.toCarriers).toSorts borrowed → Memory unit → Memory unit → Prop)
    (function : @NTy.carrier (Θ.toCarriers).toSorts (.function borrowed shared (.cons (.int 64 false) .nil))) :
    @FramedAt unit _ executable Θ borrowed shared (.one (.int 64 false)) frame function :=
  fun free => nomatch free

/-! ## A closed row -/

example (function : @NTy.carrier (Θ.toCarriers).toSorts (.function scalar shared scalar))
    (bounded : (NTy.function scalar shared scalar).paramsAll
      (fun index => decide (index < params.length)) = true) :
    @ValidResultsAt unit _ executable (Θ.instance params own) scalar shared (.one (.int 64 false))
        (Θ.reframe own bounded function) ↔
      @ValidResultsAt unit _ executable Θ scalar shared (.one (.int 64 false)) function :=
  ValidResultsAt.reframe_iff (parameters := scalar) (shared := shared)
    (shape := .one (.int 64 false)) Θ own executable rfl rfl bounded function

example (function : @NTy.carrier (Θ.toCarriers).toSorts (.function scalar shared scalar))
    (bounded : (NTy.function scalar shared scalar).paramsAll
      (fun index => decide (index < params.length)) = true) :
    @KeepsMemoryAt unit _ executable (Θ.instance params own) scalar shared (.one (.int 64 false))
        (Θ.reframe own bounded function) ↔
      @KeepsMemoryAt unit _ executable Θ scalar shared (.one (.int 64 false)) function :=
  KeepsMemoryAt.reframe_iff (parameters := scalar) (shared := shared)
    (shape := .one (.int 64 false)) Θ own executable bounded function

/-! ## A row of the declaration's parameter -/

example (function : @NTy.carrier (Θ.toCarriers).toSorts (.function generic shared generic))
    (bounded : (NTy.function generic shared generic).paramsAll
      (fun index => decide (index < params.length)) = true) :
    @ValidResultsAt unit _ executable (Θ.instance params own) generic shared (.one (.param 0))
        (Θ.reframe own bounded function) ↔
      @ValidResultsAt unit _ executable Θ generic shared (.one (.param 0)) function :=
  ValidResultsAt.reframe_iff (parameters := generic) (shared := shared) (shape := .one (.param 0))
    Θ own executable rfl rfl bounded function

example (function : @NTy.carrier (Θ.toCarriers).toSorts (.function generic shared generic))
    (bounded : (NTy.function generic shared generic).paramsAll
      (fun index => decide (index < params.length)) = true) :
    @KeepsMemoryAt unit _ executable (Θ.instance params own) generic shared (.one (.param 0))
        (Θ.reframe own bounded function) ↔
      @KeepsMemoryAt unit _ executable Θ generic shared (.one (.param 0)) function :=
  KeepsMemoryAt.reframe_iff (parameters := generic) (shared := shared) (shape := .one (.param 0))
    Θ own executable bounded function

/-- The bound of such a function type is decided. -/
example : (NTy.function scalar shared generic).paramsAll
    (fun index => decide (index < params.length)) = true := by decide

/-! ## A function returning nothing -/

example (function : @NTy.carrier (Θ.toCarriers).toSorts (.function generic ⟨[false], []⟩ .nil))
    (bounded : (NTy.function generic ⟨[false], []⟩ .nil).paramsAll
      (fun index => decide (index < params.length)) = true) :
    @ValidResultsAt unit _ executable (Θ.instance params own) generic ⟨[false], []⟩ .none
        (Θ.reframe own bounded function) ↔
      @ValidResultsAt unit _ executable Θ generic ⟨[false], []⟩ .none function :=
  ValidResultsAt.reframe_iff (parameters := generic) (shared := ⟨[false], []⟩) (shape := .none)
    Θ own executable rfl rfl bounded function

/-! ## A frame that reads its arguments

A frame stated at the induced frame is a predicate of that frame's
arguments.  At the frame the value was moved from it is read at the
arguments moved there: a run at the frame on `args` keeps it at
`Θ.reframe … args`, the induced frame's view of the same arguments. -/

example (function : @NTy.carrier (Θ.toCarriers).toSorts (.function generic shared generic))
    (bounded : (NTy.function generic shared generic).paramsAll
      (fun index => decide (index < params.length)) = true)
    (observed : @HList ((Θ.instance params own).toCarriers).toSorts generic → Memory unit → Prop)
    (framed : @FramedAt unit _ executable (Θ.instance params own) generic shared (.one (.param 0))
      (fun args _ post => observed args post) (Θ.reframe own bounded function))
    (args : @HList (Θ.toCarriers).toSorts generic) (result : @NTy.carrier (Θ.toCarriers).toSorts (.param 0))
    (pre post : Memory unit) (valid : @HList.Holds Θ.toCarriers generic args)
    (held : @NTy.Holds Θ.toCarriers (.param 0) result) (typed : MemoryTyped unit pre)
    (ran : @EnsuresOf unit _ Θ (@closureMeaning unit _ executable Θ) generic shared
      (.one (.param 0)) function args result pre post) :
    observed (Θ.reframe own (τ := .tuple generic) (NTy.paramsAll_function_parameters bounded) args)
      post :=
  (FramedAt.reframe_iff (parameters := generic) (shared := shared) (shape := .one (.param 0))
    Θ own executable bounded (fun args _ post => observed args post)
    function).mp framed rfl rfl args result pre post valid held typed ran

/-- And conversely the induced frame's predicate, at any of its arguments,
from the frame the value keeps where it was moved from: every argument row
of the induced frame is a row moved there. -/
example (function : @NTy.carrier (Θ.toCarriers).toSorts (.function generic shared generic))
    (bounded : (NTy.function generic shared generic).paramsAll
      (fun index => decide (index < params.length)) = true)
    (observed : @HList ((Θ.instance params own).toCarriers).toSorts generic → Memory unit → Prop)
    (framed : @FramedAt unit _ executable Θ generic shared (.one (.param 0))
      (fun args _ post => observed
        (Θ.reframe own (τ := .tuple generic) (NTy.paramsAll_function_parameters bounded) args) post)
      function) :
    @FramedAt unit _ executable (Θ.instance params own) generic shared (.one (.param 0))
      (fun args _ post => observed args post) (Θ.reframe own bounded function) :=
  (FramedAt.reframe_iff (parameters := generic) (shared := shared) (shape := .one (.param 0))
    Θ own executable bounded (fun args _ post => observed args post) function).mpr framed

/-! ## The run two frames share

Two frames, one closure, arguments and results that encode alike: the runs
are the same proposition's two readings. -/

example (Θ₁ Θ₂ : Skolems unit) (closure : ClosureValue)
    (args₁ : @HList (Θ₁.toCarriers).toSorts generic) (args₂ : @HList (Θ₂.toCarriers).toSorts generic)
    (sameArgs : @HList.encode (Θ₁.toCarriers).toSorts generic args₁ = @HList.encode (Θ₂.toCarriers).toSorts generic args₂)
    (result₁ : @NTy.carrier (Θ₁.toCarriers).toSorts (.param 0)) (result₂ : @NTy.carrier (Θ₂.toCarriers).toSorts (.param 0))
    (sameResult : @NTy.encode (Θ₁.toCarriers).toSorts (.param 0) result₁ =
      @NTy.encode (Θ₂.toCarriers).toSorts (.param 0) result₂)
    (pre post : Memory unit) :
    (@closureRun unit _ executable Θ₁ closure generic (.one (.param 0)) args₁).ok pre result₁ post ↔
      (@closureRun unit _ executable Θ₂ closure generic (.one (.param 0)) args₂).ok pre result₂ post :=
  closureRun_ok_iff_of_encode (σs := generic) (shape := .one (.param 0)) executable Θ₁ Θ₂
    closure rfl rfl sameArgs (congrArg (fun raw => #[raw]) sameResult) pre post

/--
info: 'VerexIR.Proofs.closureRun_ok_iff_of_encode' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms closureRun_ok_iff_of_encode

/--
info: 'VerexIR.Proofs.ValidResultsAt.reframe_iff' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms ValidResultsAt.reframe_iff

/--
info: 'VerexIR.Proofs.FramedAt.reframe_iff' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms FramedAt.reframe_iff

/--
info: 'VerexIR.Proofs.KeepsMemoryAt.reframe_iff' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms KeepsMemoryAt.reframe_iff

end VerexIR.Tests.BehaviorReframe

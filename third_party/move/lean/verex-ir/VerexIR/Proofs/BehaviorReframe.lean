-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Invocation
import VerexIR.Proofs.Denote.Resources

/-!
# The behavior of a function value at the frame of a declaration's type arguments

A stored value of a generic declaration is stated at the frame its resource's
type arguments induce over the runtime frame (`Skolems.instance`), where a
reader's frame holds the value moved there (`Skolems.reframe`).  A function
value keeps its encoding across the move (`Skolems.encode_reframe`), so it is
the same runtime closure, and an invocation of it runs alike at both frames.

A run of a closure at rows without references reads its arguments and its
result through their encodings alone (`closureRun_ok_iff_of_encode`): the
lent arguments are the encoded row (`NRow.lend_refFree`) and the boundary
correspondence states the encodings (`NRow.corresponds_refFree_iff`).  The
storage, the admissible start, the target's run and the returned storage read
no frame.  A row with references lends and corresponds by recursion over the
referents at the frame, with loans, and is not stated here.

The behavioral predicates of a function value at the induced frame, of the
value moved there, are therefore those at the frame, of the value
(`ValidResultsAt.reframe_iff`, `FramedAt.reframe_iff`,
`KeepsMemoryAt.reframe_iff`): the arguments and the result move with
`Skolems.reframe`, which keeps their encodings and their data invariants
(`Skolems.holds_reframe_iff`), and every value of the induced frame is one
moved there (`Skolems.reframe_surjective`).
-/

namespace VerexIR.Proofs

open Denote Validation

section
variable {unit : ValidatedUnit} [FunctionCarriers unit]

/-- A result of a shape without references corresponds to the runtime results
that are its encoding. -/
theorem Denote.ResultShape.corresponds_refFree_iff [Carriers] (loans : LoanCorrespondence) :
    (shape : ResultShape) → shape.row.refFree = true → (result : shape.carrier) →
    (results : Array RuntimeValue) →
    (shape.corresponds loans result results ↔ results = (resultCodec shape).encode result)
  | .none, _, _, _ => Iff.rfl
  | .one τ, free, result, results => by
      simp only [NRow.refFree, Bool.and_true] at free
      simp only [ResultShape.corresponds, NTy.corresponds_refFree_iff loans τ free]
      constructor
      · rintro ⟨raw, rfl, rfl⟩
        rfl
      · intro same
        exact ⟨_, same, rfl⟩

private theorem closureRun_ok_of_encode (executable : ExecutableUnit unit)
    (Θ₁ Θ₂ : Skolems unit) (closure : ClosureValue) {σs : NRow} {shape : ResultShape}
    (parametersFree : σs.refFree = true) (resultsFree : shape.row.refFree = true)
    {args₁ : @HList (Θ₁.toCarriers).toSorts σs} {args₂ : @HList (Θ₂.toCarriers).toSorts σs}
    (sameArgs : @HList.encode (Θ₁.toCarriers).toSorts σs args₁ = @HList.encode (Θ₂.toCarriers).toSorts σs args₂)
    {result₁ : @ResultShape.carrier (Θ₁.toCarriers).toSorts shape}
    {result₂ : @ResultShape.carrier (Θ₂.toCarriers).toSorts shape}
    (sameResult : (@resultCodec Θ₁.toCarriers.toSorts shape).encode result₁ =
      (@resultCodec Θ₂.toCarriers.toSorts shape).encode result₂)
    {pre post : Memory unit}
    (run : (@closureRun unit _ executable Θ₁ closure σs shape args₁).ok pre result₁ post) :
    (@closureRun unit _ executable Θ₂ closure σs shape args₂).ok pre result₂ post := by
  obtain ⟨start, loans, supplied, arguments, results, exit, correspondence, globals,
    admissible, lends, composes, runs, relates, encoded, agree⟩ := run
  refine ⟨start, loans, supplied, arguments, results, exit, correspondence, globals,
    admissible, ?_, composes, runs, ?_, encoded, agree⟩
  · unfold lendArguments at lends ⊢
    rw [@NRow.lend_refFree unit _ Θ₁ σs parametersFree false args₁ loans] at lends
    rw [@NRow.lend_refFree unit _ Θ₂ σs parametersFree false args₂ loans, ← sameArgs]
    exact lends
  · obtain ⟨valid, arguments', result', settles⟩ := relates
    refine @BoundaryCorrespondence.mk Θ₂.toCarriers σs args₂ shape result₂ supplied results loans
      _ correspondence valid ?_ ?_ settles
    · rw [@NRow.corresponds_refFree_iff Θ₂.toCarriers correspondence σs parametersFree, ← sameArgs]
      exact (@NRow.corresponds_refFree_iff Θ₁.toCarriers correspondence σs parametersFree
        args₁ _).mp arguments'
    · rw [@ResultShape.corresponds_refFree_iff Θ₂.toCarriers correspondence shape resultsFree,
        ← sameResult]
      exact (@ResultShape.corresponds_refFree_iff Θ₁.toCarriers correspondence shape resultsFree
        result₁ _).mp result'

/-- A run of a closure at rows without references is determined by the
encodings of its arguments and of its result: two frames whose arguments
encode alike run the closure alike, to results that encode alike. -/
theorem closureRun_ok_iff_of_encode (executable : ExecutableUnit unit)
    (Θ₁ Θ₂ : Skolems unit) (closure : ClosureValue) {σs : NRow} {shape : ResultShape}
    (parametersFree : σs.refFree = true) (resultsFree : shape.row.refFree = true)
    {args₁ : @HList (Θ₁.toCarriers).toSorts σs} {args₂ : @HList (Θ₂.toCarriers).toSorts σs}
    (sameArgs : @HList.encode (Θ₁.toCarriers).toSorts σs args₁ = @HList.encode (Θ₂.toCarriers).toSorts σs args₂)
    {result₁ : @ResultShape.carrier (Θ₁.toCarriers).toSorts shape}
    {result₂ : @ResultShape.carrier (Θ₂.toCarriers).toSorts shape}
    (sameResult : (@resultCodec Θ₁.toCarriers.toSorts shape).encode result₁ =
      (@resultCodec Θ₂.toCarriers.toSorts shape).encode result₂)
    (pre post : Memory unit) :
    (@closureRun unit _ executable Θ₁ closure σs shape args₁).ok pre result₁ post ↔
      (@closureRun unit _ executable Θ₂ closure σs shape args₂).ok pre result₂ post :=
  ⟨closureRun_ok_of_encode executable Θ₁ Θ₂ closure parametersFree resultsFree sameArgs
      sameResult,
    closureRun_ok_of_encode executable Θ₂ Θ₁ closure parametersFree resultsFree sameArgs.symm
      sameResult.symm⟩

variable (Θ : Skolems unit) {params : NRow} (own : params.isParamsFrom 0 = true)

/-- Every value of the frame of a declaration's type arguments is a value of
the frame moved there. -/
theorem Denote.Skolems.reframe_surjective {τ : NTy}
    (bounded : τ.paramsAll (fun index => decide (index < params.length)) = true)
    (moved : @NTy.carrier ((Θ.instance params own).toCarriers).toSorts τ) :
    ∃ value : @NTy.carrier (Θ.toCarriers).toSorts τ, Θ.reframe own bounded value = moved := by
  refine ⟨Θ.ofRuntime τ
    (cast (congrArg (@NTy.carrier ((Carriers.runtime unit)).toSorts) (Θ.resolve_instance_params own bounded))
      ((Θ.instance params own).toRuntime τ moved)), ?_⟩
  apply @NTy.encode_injective ((Θ.instance params own).toCarriers).toSorts τ
  rw [Θ.encode_reframe, ← Θ.encode_toRuntime, Θ.toRuntime_ofRuntime]
  exact (NTy.encode_castCarrier (Θ.resolve_instance_params own bounded) _ _).trans
    ((Θ.instance params own).encode_toRuntime τ moved)

/-- The parameters of a function type over a declaration's parameters are
over them, as a tuple. -/
theorem Denote.NTy.paramsAll_function_parameters {allowed : Nat → Bool} {parameters : NRow}
    {shared : Sharing} {results : NRow}
    (bounded : (NTy.function parameters shared results).paramsAll allowed = true) :
    (NTy.tuple parameters).paramsAll allowed = true := by
  simp only [NTy.paramsAll, Bool.and_eq_true] at bounded
  simpa only [NTy.paramsAll] using bounded.1

/-- The result of a function type over a declaration's parameters is over
them. -/
theorem Denote.NTy.paramsAll_function_result {allowed : Nat → Bool} {parameters : NRow}
    {shared : Sharing} {τ : NTy} {rest : NRow}
    (bounded : (NTy.function parameters shared (.cons τ rest)).paramsAll allowed = true) :
    τ.paramsAll allowed = true := by
  simp only [NTy.paramsAll, NRow.paramsAll, Bool.and_eq_true] at bounded
  exact bounded.2.1

/-- A row moved to the frame of its type arguments keeps its encoding. -/
theorem Denote.Skolems.encode_reframe_row {row : NRow}
    (bounded : (NTy.tuple row).paramsAll (fun index => decide (index < params.length)) = true)
    (values : @HList (Θ.toCarriers).toSorts row) :
    @HList.encode ((Θ.instance params own).toCarriers).toSorts row
        (Θ.reframe own (τ := .tuple row) bounded values) =
      @HList.encode (Θ.toCarriers).toSorts row values := by
  have encoded := Θ.encode_reframe own (τ := .tuple row) bounded values
  rw [@NTy.encode_tuple ((Θ.instance params own).toCarriers).toSorts, @NTy.encode_tuple (Θ.toCarriers).toSorts] at encoded
  simp only [RuntimeValue.tuple.injEq] at encoded
  exact List.toArray_inj encoded

/-- A row moved to the frame of its type arguments keeps its data
invariants. -/
theorem Denote.Skolems.holds_reframe_row_iff {row : NRow}
    (bounded : (NTy.tuple row).paramsAll (fun index => decide (index < params.length)) = true)
    (values : @HList (Θ.toCarriers).toSorts row) :
    @HList.Holds (Θ.instance params own).toCarriers row
        (Θ.reframe own (τ := .tuple row) bounded values) ↔
      @HList.Holds Θ.toCarriers row values := by
  rw [← @NTy.holds_tuple (Θ.instance params own).toCarriers, ← @NTy.holds_tuple Θ.toCarriers]
  exact Θ.holds_reframe_iff own bounded values

/-- A function value moved to the frame of its type arguments is the same
runtime closure. -/
theorem Denote.Skolems.closure_reframe {parameters : NRow} {shared : Sharing} {results : NRow}
    (bounded : (NTy.function parameters shared results).paramsAll
      (fun index => decide (index < params.length)) = true)
    (function : @NTy.carrier (Θ.toCarriers).toSorts (.function parameters shared results)) :
    (@Sorts.functions ((Θ.instance params own).toCarriers).toSorts parameters shared results).closure
        (Θ.reframe own bounded function) =
      (@Sorts.functions (Θ.toCarriers).toSorts parameters shared results).closure function :=
  ClosureValue.encode_injective (Θ.encode_reframe own bounded function)

/-- A run of a function value at the frame of its type arguments, of the
value, the arguments and the result moved there, is its run at the frame. -/
private theorem ensuresOf_reframe (executable : ExecutableUnit unit) {parameters : NRow}
    {shared : Sharing} {shape : ResultShape}
    (parametersFree : parameters.refFree = true) (resultsFree : shape.row.refFree = true)
    (bounded : (NTy.function parameters shared shape.row).paramsAll
      (fun index => decide (index < params.length)) = true)
    (function : @NTy.carrier (Θ.toCarriers).toSorts (.function parameters shared shape.row))
    (args : @HList (Θ.toCarriers).toSorts parameters)
    {result : @ResultShape.carrier (Θ.toCarriers).toSorts shape}
    {moved : @ResultShape.carrier ((Θ.instance params own).toCarriers).toSorts shape}
    (sameResult : (@resultCodec (Θ.instance params own).toCarriers.toSorts shape).encode moved =
      (@resultCodec Θ.toCarriers.toSorts shape).encode result)
    (pre post : Memory unit) :
    @EnsuresOf unit _ (Θ.instance params own) (@closureMeaning unit _ executable (Θ.instance params own))
        parameters shared shape (Θ.reframe own bounded function)
        (Θ.reframe own (τ := .tuple parameters) (NTy.paramsAll_function_parameters bounded) args)
        moved pre post ↔
      @EnsuresOf unit _ Θ (@closureMeaning unit _ executable Θ) parameters shared shape function
        args result pre post := by
  unfold EnsuresOf closureMeaning
  rw [Θ.closure_reframe own bounded function]
  exact closureRun_ok_iff_of_encode executable (Θ.instance params own) Θ _ parametersFree
    resultsFree (Θ.encode_reframe_row own _ args) sameResult pre post

/-- A result of the frame of a declaration's type arguments is a result of
the frame with the same encoding, which holds its data invariants where the
other does. -/
private theorem result_reframe {parameters : NRow} {shared : Sharing} :
    (shape : ResultShape) →
    (NTy.function parameters shared shape.row).paramsAll
      (fun index => decide (index < params.length)) = true →
    (moved : @ResultShape.carrier ((Θ.instance params own).toCarriers).toSorts shape) →
    ∃ result : @ResultShape.carrier (Θ.toCarriers).toSorts shape,
      (@resultCodec (Θ.instance params own).toCarriers.toSorts shape).encode moved =
        (@resultCodec Θ.toCarriers.toSorts shape).encode result ∧
      (@ResultShape.Holds (Θ.instance params own).toCarriers shape moved ↔
        @ResultShape.Holds Θ.toCarriers shape result)
  | .none, _, moved => ⟨moved, rfl, Iff.rfl⟩
  | .one τ, bounded, moved => by
      obtain ⟨result, rfl⟩ :=
        Θ.reframe_surjective own (NTy.paramsAll_function_result bounded) moved
      exact ⟨result, congrArg (fun raw => #[raw]) (Θ.encode_reframe own _ result),
        Θ.holds_reframe_iff own _ result⟩

/-- A result of the frame moves to the frame of a declaration's type
arguments, keeping its encoding and its data invariants. -/
private theorem result_moved {parameters : NRow} {shared : Sharing} :
    (shape : ResultShape) →
    (NTy.function parameters shared shape.row).paramsAll
      (fun index => decide (index < params.length)) = true →
    (result : @ResultShape.carrier (Θ.toCarriers).toSorts shape) →
    ∃ moved : @ResultShape.carrier ((Θ.instance params own).toCarriers).toSorts shape,
      (@resultCodec (Θ.instance params own).toCarriers.toSorts shape).encode moved =
        (@resultCodec Θ.toCarriers.toSorts shape).encode result ∧
      (@ResultShape.Holds (Θ.instance params own).toCarriers shape moved ↔
        @ResultShape.Holds Θ.toCarriers shape result)
  | .none, _, result => ⟨result, rfl, Iff.rfl⟩
  | .one _, bounded, result =>
      ⟨Θ.reframe own (NTy.paramsAll_function_result bounded) result,
        congrArg (fun raw => #[raw]) (Θ.encode_reframe own _ result),
        Θ.holds_reframe_iff own _ result⟩

/-- The frame a function value keeps at the frame of a declaration's type
arguments, of the value moved there, is the frame it keeps at the frame, read
at the arguments moved there.  The rows without references the predicate
states its runs at are those the run is determined by encodings at; the
arguments' data invariants and the result's move with them
(`Skolems.holds_reframe_iff`), and the typing of the memory reads no frame. -/
theorem FramedAt.reframe_iff (executable : ExecutableUnit unit) {parameters : NRow}
    {shared : Sharing} {shape : ResultShape}
    (bounded : (NTy.function parameters shared shape.row).paramsAll
      (fun index => decide (index < params.length)) = true)
    (frame : @HList ((Θ.instance params own).toCarriers).toSorts parameters →
      Memory unit → Memory unit → Prop)
    (function : @NTy.carrier (Θ.toCarriers).toSorts (.function parameters shared shape.row)) :
    @FramedAt unit _ executable (Θ.instance params own) parameters shared shape frame
        (Θ.reframe own bounded function) ↔
      @FramedAt unit _ executable Θ parameters shared shape
        (fun args pre post =>
          frame (Θ.reframe own (τ := .tuple parameters)
            (NTy.paramsAll_function_parameters bounded) args) pre post)
        function := by
  constructor
  · intro framed parametersFree resultsFree args result pre post valid held typed ensures
    obtain ⟨moved, sameResult, sameHolds⟩ := result_moved Θ own shape bounded result
    exact framed parametersFree resultsFree _ moved pre post
      ((Θ.holds_reframe_row_iff own _ args).mpr valid) (sameHolds.mpr held) typed
      ((ensuresOf_reframe Θ own executable parametersFree resultsFree bounded function args
        sameResult pre post).mpr ensures)
  · intro framed parametersFree resultsFree movedArgs moved pre post valid held typed ensures
    obtain ⟨args, rfl⟩ := Θ.reframe_surjective own (τ := .tuple parameters)
      (NTy.paramsAll_function_parameters bounded) movedArgs
    obtain ⟨result, sameResult, sameHolds⟩ := result_reframe Θ own shape bounded moved
    exact framed parametersFree resultsFree args result pre post
      ((Θ.holds_reframe_row_iff own _ args).mp valid) (sameHolds.mp held) typed
      ((ensuresOf_reframe Θ own executable parametersFree resultsFree bounded function args
        sameResult pre post).mp ensures)

/-- A function value keeps memory at the frame of a declaration's type
arguments, moved there, where it keeps it at the frame: the frame that
leaves memory as it was reads no argument. -/
theorem KeepsMemoryAt.reframe_iff (executable : ExecutableUnit unit) {parameters : NRow}
    {shared : Sharing} {shape : ResultShape}
    (bounded : (NTy.function parameters shared shape.row).paramsAll
      (fun index => decide (index < params.length)) = true)
    (function : @NTy.carrier (Θ.toCarriers).toSorts (.function parameters shared shape.row)) :
    @KeepsMemoryAt unit _ executable (Θ.instance params own) parameters shared shape
        (Θ.reframe own bounded function) ↔
      @KeepsMemoryAt unit _ executable Θ parameters shared shape function :=
  FramedAt.reframe_iff Θ own executable bounded (fun _ pre post => post = pre) function

/-- The results of a function value are valid at the frame of a declaration's
type arguments, of the value moved there, where they are at the frame.  The
predicate states its runs at every row, so the rows without references the
run is determined by encodings at are hypotheses here; the arguments' data
invariants and the result's move with them (`Skolems.holds_reframe_iff`). -/
theorem ValidResultsAt.reframe_iff (executable : ExecutableUnit unit) {parameters : NRow}
    {shared : Sharing} {shape : ResultShape}
    (parametersFree : parameters.refFree = true) (resultsFree : shape.row.refFree = true)
    (bounded : (NTy.function parameters shared shape.row).paramsAll
      (fun index => decide (index < params.length)) = true)
    (function : @NTy.carrier (Θ.toCarriers).toSorts (.function parameters shared shape.row)) :
    @ValidResultsAt unit _ executable (Θ.instance params own) parameters shared shape
        (Θ.reframe own bounded function) ↔
      @ValidResultsAt unit _ executable Θ parameters shared shape function := by
  constructor
  · intro valid args result pre post held ensures
    obtain ⟨moved, sameResult, sameHolds⟩ := result_moved Θ own shape bounded result
    exact sameHolds.mp (valid _ moved pre post ((Θ.holds_reframe_row_iff own _ args).mpr held)
      ((ensuresOf_reframe Θ own executable parametersFree resultsFree bounded function args
        sameResult pre post).mpr ensures))
  · intro valid movedArgs moved pre post held ensures
    obtain ⟨args, rfl⟩ := Θ.reframe_surjective own (τ := .tuple parameters)
      (NTy.paramsAll_function_parameters bounded) movedArgs
    obtain ⟨result, sameResult, sameHolds⟩ := result_reframe Θ own shape bounded moved
    exact sameHolds.mpr (valid args result pre post ((Θ.holds_reframe_row_iff own _ args).mp held)
      ((ensuresOf_reframe Θ own executable parametersFree resultsFree bounded function args
        sameResult pre post).mp ensures))

end

end VerexIR.Proofs

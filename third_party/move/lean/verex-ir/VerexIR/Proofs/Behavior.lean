-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Meaning
import VerexIR.Proofs.Denote.Closures

/-!
# Behavioral predicates

The behavior of a function value as a specification states it
(`doc/designs/higher-order-functions.md`, "Behavioral predicates and state
labels"): `aborts_of`, `ensures_of`, `result_of`, and `requires_of`, over an
invocation meaning (`Denote.InvokeMeaning`) on native arguments and
results. The executable meaning is `Denote.closureMeaning`, the big-step
run of the value's closure from the starts a call runs from; a unit
verified through its model invokes through the model's meaning. Of a
function value whose target, weave, and captures a proof sees, the
executable meaning is a run of the target's prophetic meaning
(`Denote.closureRun_closureOf`), which its theorem speaks about.

A mutable reference argument is its entry and final values, a returned one
its value at return and its prophecy, as everywhere in the denotation.
-/

namespace VerexIR.Proofs

open VerexIR.Validation (ExecutableUnit ValidatedUnit)
open Denote

section
variable [Carriers]

/-- A result whose returned mutable reference has its current value as its
prophecy, as the reference's death leaves it (`Term.resolve`). -/
def Denote.ResultShape.Settled : (shape : ResultShape) → shape.carrier → Prop
  | .one (.ref _), value => value.1 = value.2
  | _, _ => True

/-- A value with a mutable reference's prophecy replaced by its entry value:
an argument as `aborts_of` and `result_of` read it, which no final value
names. -/
def Denote.NTy.settle : (τ : NTy) → τ.carrier → τ.carrier
  | .ref _, value => (value.1, value.1)
  | .unit, value => value
  | .bool, value => value
  | .int _ _, value => value
  | .num, value => value
  | .address, value => value
  | .signer, value => value
  | .string, value => value
  | .bytes, value => value
  | .tuple _, value => value
  | .struct _ _ _, value => value
  | .enum _ _ _ _ _, value => value
  | .vector _, value => value
  | .table _ _, value => value
  | .ghost _, value => value
  | .param _, value => value
  | .function _ _ _, value => value

/-- A row of arguments, each settled. -/
def Denote.HList.settle : (row : NRow) → HList row → HList row
  | .nil, values => values
  | .cons τ rest, values => HList.cons (τ.settle values.1) (HList.settle rest values.2)

end

section
variable {unit : ValidatedUnit} [FunctionCarriers unit] [Skolems unit]

/-- `ensures_of<f>(x, r)`: the invocation of `function` on `args` runs from
`pre` to `post`, returning `result`. -/
def EnsuresOf (invoke : InvokeMeaning) (σs : NRow) (shared : Sharing) (shape : ResultShape)
    (function : (NTy.function σs shared shape.row).carrier) (args : HList σs) (result : shape.carrier)
    (pre post : Memory unit) : Prop :=
  (invoke σs shared shape function args).ok pre result post

/-- `aborts_of<f>(x)`: the invocation of `function` on `args` aborts from
`state`. -/
def AbortsOf (invoke : InvokeMeaning) (σs : NRow) (shared : Sharing) (shape : ResultShape)
    (function : (NTy.function σs shared shape.row).carrier) (args : HList σs) (state : Memory unit) : Prop :=
  ∃ error, (invoke σs shared shape function args).aborts state error

/-- A run `result_of` and `state_of` read: from `state`, on arguments of the
entry values of `args`, returning a settled result. -/
def SettledRun (invoke : InvokeMeaning) (σs : NRow) (shared : Sharing) (shape : ResultShape)
    (function : (NTy.function σs shared shape.row).carrier) (args : HList σs) (state : Memory unit)
    (run : HList σs × shape.carrier × Memory unit) : Prop :=
  HList.settle σs run.1 = HList.settle σs args ∧ shape.Settled run.2.1 ∧
    EnsuresOf invoke σs shared shape function run.1 run.2.1 state run.2.2

/-- A run's encodings: its arguments', its result's, and its final memory,
which every view of a frame holds alike. -/
def runEncoding (σs : NRow) (shape : ResultShape) (run : HList σs × shape.carrier × Memory unit) :
    List RuntimeValue × Array RuntimeValue × Memory unit :=
  (HList.encode run.1, (Denote.resultCodec shape).encode run.2.1, run.2.2)

/-- The encodings of the settled runs `result_of` and `state_of` read. -/
def SettledEncoding (invoke : InvokeMeaning) (σs : NRow) (shared : Sharing)
    (shape : ResultShape) (function : (NTy.function σs shared shape.row).carrier) (args : HList σs)
    (state : Memory unit) (encoded : List RuntimeValue × Array RuntimeValue × Memory unit) : Prop :=
  ∃ run, SettledRun invoke σs shared shape function args state run ∧
    runEncoding σs shape run = encoded

open Classical in
/-- `result_of<f>(x)`: the result of a settled run on the entry values of
`args` from `state`, chosen among the runs' encodings, so that every view of
a frame names the same run; `default` where there is none. -/
noncomputable def ResultOf (invoke : InvokeMeaning) (σs : NRow) (shared : Sharing)
    (shape : ResultShape) (function : (NTy.function σs shared shape.row).carrier) (args : HList σs)
    (state : Memory unit) (default : shape.carrier) : shape.carrier :=
  if returns : ∃ encoded, SettledEncoding invoke σs shared shape function args state encoded then
    ((Denote.resultCodec shape).decode? returns.choose.2.1).getD default
  else default

open Classical in
/-- The post-state of the run `result_of` reads; the pre-state where there is
none, so that using the label does not assume that the invocation returns. -/
noncomputable def StateOf (invoke : InvokeMeaning) (σs : NRow) (shared : Sharing)
    (shape : ResultShape) (function : (NTy.function σs shared shape.row).carrier) (args : HList σs)
    (state : Memory unit) : Memory unit :=
  if returns : ∃ encoded, SettledEncoding invoke σs shared shape function args state encoded then
    returns.choose.2.2
  else state

/-- The run `result_of` and `state_of` read, where a settled run returns:
one whose encodings are the chosen ones. -/
theorem settledEncoding_choose {invoke : InvokeMeaning} {σs : NRow} {shared : Sharing}
    {shape : ResultShape} {function : (NTy.function σs shared shape.row).carrier} {args : HList σs}
    {state : Memory unit}
    (returns : ∃ encoded, SettledEncoding invoke σs shared shape function args state encoded)
    (default : shape.carrier) :
    ∃ run, SettledRun invoke σs shared shape function args state run ∧
      ResultOf invoke σs shared shape function args state default = run.2.1 ∧
      StateOf invoke σs shared shape function args state = run.2.2 := by
  obtain ⟨run, settled, encoded⟩ := returns.choose_spec
  refine ⟨run, settled, ?_, ?_⟩
  · unfold ResultOf
    rw [dif_pos returns, ← encoded]
    simp only [runEncoding, Codec.decode_encode, Option.getD_some]
  · unfold StateOf
    rw [dif_pos returns, ← encoded]
    rfl

/-- Where a settled run returns, `result_of` and `state_of` name one. -/
theorem settledRun_resultOf {invoke : InvokeMeaning} {σs : NRow} {shared : Sharing}
    {shape : ResultShape} {function : (NTy.function σs shared shape.row).carrier} {args : HList σs}
    {state : Memory unit} {run : HList σs × shape.carrier × Memory unit}
    (returns : SettledRun invoke σs shared shape function args state run) (default : shape.carrier) :
    ∃ args', SettledRun invoke σs shared shape function args state
      (args', ResultOf invoke σs shared shape function args state default,
        StateOf invoke σs shared shape function args state) := by
  obtain ⟨chosen, settled, result, final⟩ :=
    settledEncoding_choose ⟨runEncoding σs shape run, run, returns, rfl⟩ default
  refine ⟨chosen.1, ?_⟩
  rw [result, final]
  exact settled

end

/-- Declared preconditions of a unit's function values at the base frame:
of a value, at native arguments of its parameter row, from a state. A link
set's generated meaning reads a closure site's value as its target's
declared precondition at the woven arguments and any other value by an
uninterpreted predicate (`requiresForeign`, or a model's hypothesis). -/
abbrev RequiresMeaning (unit : Validation.ValidatedUnit) [FunctionCarriers unit] :=
  (parameters : NRow) → (shared : Sharing) → (results : NRow) →
    FunctionCarriers.carrier unit parameters shared results →
    @HList (Carriers.runtime unit) parameters → Memory unit → Prop

/-- The declared precondition of a function value no closure site of the
unit encodes: uninterpreted. Defining it as `True` would make every such
value invocable at every argument. -/
opaque requiresForeign (unit : Validation.ValidatedUnit) [FunctionCarriers unit] :
    RequiresMeaning unit

/-- A requires meaning at equal rows, values and arguments. -/
theorem RequiresMeaning.congr {unit : Validation.ValidatedUnit} [FunctionCarriers unit]
    (requires : RequiresMeaning unit) {parameters parameters' results results' : NRow}
    (shared : Sharing) (sameParameters : parameters = parameters') (sameResults : results = results')
    {value : FunctionCarriers.carrier unit parameters shared results}
    {value' : FunctionCarriers.carrier unit parameters' shared results'} (sameValue : HEq value value')
    {args : @HList (Carriers.runtime unit) parameters} {args' : @HList (Carriers.runtime unit) parameters'}
    (sameArgs : HEq args args') (state : Memory unit) :
    requires parameters shared results value args state =
      requires parameters' shared results' value' args' state := by
  subst sameParameters sameResults
  cases sameValue
  cases sameArgs
  rfl

section
variable {unit : ValidatedUnit} [FunctionCarriers unit] [Θ : Skolems unit]

/-- A frame resolves a function type to the function type of its rows'
resolutions. -/
theorem Skolems.resolve_function (parameters : NRow) (shared : Sharing) (results : NRow) :
    Θ.resolve (.function parameters shared results) =
      .function parameters.resolved shared results.resolved := by
  rw [Θ.resolve_eq, NRow.resolved_eq_substWith, NRow.resolved_eq_substWith]
  rfl

/-- A function value at the frame as the base frame holds it. -/
def Skolems.functionToRuntime (parameters : NRow) (shared : Sharing) (results : NRow)
    (function : (NTy.function parameters shared results).carrier) :
    FunctionCarriers.carrier unit parameters.resolved shared results.resolved :=
  cast (congrArg (@NTy.carrier (Carriers.runtime unit))
      (Skolems.resolve_function parameters shared results))
    (Θ.toRuntime (.function parameters shared results) function)

/-- `requires_of<f>(x)`: the declared precondition of the function value at
the arguments, read by the unit's requires meaning at the base frame, where
the frame carries the value and the arguments. -/
def RequiresOf (requires : RequiresMeaning unit) (σs : NRow) (shared : Sharing) (results : NRow)
    (function : (NTy.function σs shared results).carrier) (args : HList σs) (state : Memory unit) :
    Prop :=
  requires σs.resolved shared results.resolved (Skolems.functionToRuntime σs shared results function)
    (HList.toRuntime σs args) state

omit Θ in
/-- At the base frame, `requires_of` is the requires meaning of the value at
the arguments. -/
theorem requiresOf_runtime (requires : RequiresMeaning unit) (σs : NRow) (shared : Sharing)
    (results : NRow) (function : @NTy.carrier (Carriers.runtime unit) (.function σs shared results))
    (args : @HList (Carriers.runtime unit) σs) (state : Memory unit) :
    @RequiresOf unit _ (Skolems.runtime unit) requires σs shared results function args state =
      requires σs shared results function args state := by
  unfold RequiresOf Skolems.functionToRuntime
  exact RequiresMeaning.congr requires shared (NRow.resolved_runtime σs)
    (NRow.resolved_runtime results) (cast_heq _ _) (HList.toRuntime_runtime σs args) state

attribute [vir_denote_norm] requiresOf_runtime

/-- A closure site typed at the frame is typed at the base frame at the
resolved rows. -/
theorem Denote.SiteTyped.atRuntime {parameters results : NRow} {shared : Sharing}
    {handle : FunctionHandle} {mask : Nat} {typeInstantiation : Array (TypeId × TypeId)}
    {σs : NRow} (typed : SiteTyped parameters shared results handle mask typeInstantiation σs) :
    @SiteTyped unit _ (Skolems.runtime unit) parameters.resolved shared results.resolved handle
      mask typeInstantiation σs.resolved :=
  by
    refine @SiteTyped.mk unit _ (Skolems.runtime unit) _ _ _ _ _ _ _ ?_ typed.sharing typed.faithful
    simpa only [NRow.resolved_runtime] using typed.rows

/-- The base frame holds a construction at the frame as the construction
at the base frame of the captures it holds. -/
theorem Skolems.functionToRuntime_construct (parameters : NRow) (shared : Sharing)
    (results : NRow) (handle : FunctionHandle) (mask : Nat)
    (typeInstantiation : Array (TypeId × TypeId)) {σs : NRow} (captures : HList σs)
    (typed : SiteTyped parameters shared results handle mask typeInstantiation σs) :
    Skolems.functionToRuntime parameters shared results
        (Skolems.construct parameters shared results handle mask typeInstantiation captures typed) =
      @Skolems.construct unit _ (Skolems.runtime unit) parameters.resolved shared results.resolved
        handle mask typeInstantiation σs.resolved (HList.toRuntime σs captures) typed.atRuntime := by
  apply @NTy.encode_injective (Carriers.runtime unit)
    (.function parameters.resolved shared results.resolved)
  unfold Skolems.functionToRuntime
  rw [NTy.encode_castCarrier (Skolems.resolve_function parameters shared results),
    Θ.encode_toRuntime]
  show ((Carriers.functions parameters shared results).closure _).encode =
    ((FunctionCarriers.functions parameters.resolved shared results.resolved).closure _).encode
  rw [Skolems.closure_construct]
  erw [@Skolems.closure_construct unit _ (Skolems.runtime unit) parameters.resolved shared
    results.resolved handle mask typeInstantiation σs.resolved (HList.toRuntime σs captures)
    typed.atRuntime]
  congr 1
  refine ClosureValue.ext rfl rfl rfl ?_
  show (HList.encode captures).toArray =
    (@HList.encode (Carriers.runtime unit) σs.resolved (HList.toRuntime σs captures)).toArray
  rw [HList.encode_toRuntime]

/-- `requires_of` of a construction at the frame is the requires meaning of
the construction at the base frame. -/
theorem requiresOf_construct (requires : RequiresMeaning unit) (parameters : NRow)
    (shared : Sharing) (results : NRow) (handle : FunctionHandle) (mask : Nat)
    (typeInstantiation : Array (TypeId × TypeId)) {σs : NRow} (captures : HList σs)
    (typed : SiteTyped parameters shared results handle mask typeInstantiation σs)
    (args : HList parameters) (state : Memory unit) :
    RequiresOf requires parameters shared results
        (Skolems.construct parameters shared results handle mask typeInstantiation captures typed)
        args state =
      requires parameters.resolved shared results.resolved
        (@Skolems.construct unit _ (Skolems.runtime unit) parameters.resolved shared
          results.resolved handle mask typeInstantiation σs.resolved (HList.toRuntime σs captures)
          typed.atRuntime)
        (HList.toRuntime parameters args) state := by
  rw [RequiresOf, Skolems.functionToRuntime_construct]

/-- A value at the frame of a type without type parameters, at the base
frame at the same type. -/
def Denote.Skolems.toRuntimeFree (τ : NTy) (free : τ.paramFree = true) (value : τ.carrier) :
    @NTy.carrier (Carriers.runtime unit) τ :=
  cast (congrArg (@NTy.carrier (Carriers.runtime unit)) (Skolems.resolve_paramFree free))
    (Θ.toRuntime τ value)

theorem Denote.Skolems.encode_toRuntimeFree (τ : NTy) (free : τ.paramFree = true) (value : τ.carrier) :
    @NTy.encode (Carriers.runtime unit) τ (Θ.toRuntimeFree τ free value) = NTy.encode τ value := by
  unfold Skolems.toRuntimeFree
  rw [NTy.encode_castCarrier (Skolems.resolve_paramFree free), Θ.encode_toRuntime]

/-! A scalar is the same value at the base frame; a reference is the pair of
its current value and prophecy there. -/

@[vir_denote_norm] theorem Denote.Skolems.toRuntimeFree_unit (free) (value : Unit) :
    Θ.toRuntimeFree .unit free value = value :=
  @NTy.encode_injective (Carriers.runtime unit) .unit _ _ (Θ.encode_toRuntimeFree _ free value)
@[vir_denote_norm] theorem Denote.Skolems.toRuntimeFree_bool (free) (value : Bool) :
    Θ.toRuntimeFree .bool free value = value :=
  @NTy.encode_injective (Carriers.runtime unit) .bool _ _ (Θ.encode_toRuntimeFree _ free value)
@[vir_denote_norm] theorem Denote.Skolems.toRuntimeFree_int {width : Nat} {signed : Bool} (free)
    (value : SpecInt (.bits width) signed) :
    Θ.toRuntimeFree (.int width signed) free value = value :=
  @NTy.encode_injective (Carriers.runtime unit) (.int width signed) _ _
    (Θ.encode_toRuntimeFree _ free value)
@[vir_denote_norm] theorem Denote.Skolems.toRuntimeFree_num (free) (value : SpecInt .unbounded true) :
    Θ.toRuntimeFree .num free value = value :=
  @NTy.encode_injective (Carriers.runtime unit) .num _ _ (Θ.encode_toRuntimeFree _ free value)
@[vir_denote_norm] theorem Denote.Skolems.toRuntimeFree_address (free) (value : String) :
    Θ.toRuntimeFree .address free value = value :=
  @NTy.encode_injective (Carriers.runtime unit) .address _ _ (Θ.encode_toRuntimeFree _ free value)
@[vir_denote_norm] theorem Denote.Skolems.toRuntimeFree_signer (free) (value : String) :
    Θ.toRuntimeFree .signer free value = value :=
  @NTy.encode_injective (Carriers.runtime unit) .signer _ _ (Θ.encode_toRuntimeFree _ free value)
@[vir_denote_norm] theorem Denote.Skolems.toRuntimeFree_string (free) (value : String) :
    Θ.toRuntimeFree .string free value = value :=
  @NTy.encode_injective (Carriers.runtime unit) .string _ _ (Θ.encode_toRuntimeFree _ free value)
@[vir_denote_norm] theorem Denote.Skolems.toRuntimeFree_bytes (free) (value : Array UInt8) :
    Θ.toRuntimeFree .bytes free value = value :=
  @NTy.encode_injective (Carriers.runtime unit) .bytes _ _ (Θ.encode_toRuntimeFree _ free value)

@[vir_denote_norm] theorem Denote.Skolems.toRuntimeFree_ref {τ : NTy} (free : (NTy.ref τ).paramFree = true)
    (value : τ.carrier × τ.carrier) :
    Θ.toRuntimeFree (.ref τ) free value =
      (Θ.toRuntimeFree τ free value.1, Θ.toRuntimeFree τ free value.2) := by
  apply @NTy.encode_injective (Carriers.runtime unit) (.ref τ)
  rw [Θ.encode_toRuntimeFree]
  show RuntimeValue.tuple #[τ.encode value.1, τ.encode value.2] =
    RuntimeValue.tuple #[@NTy.encode (Carriers.runtime unit) τ (Θ.toRuntimeFree τ free value.1),
      @NTy.encode (Carriers.runtime unit) τ (Θ.toRuntimeFree τ free value.2)]
  rw [Θ.encode_toRuntimeFree, Θ.encode_toRuntimeFree]

/-- Values of a row without type parameters at the frame, at the base frame
at the same row. -/
def HList.toRuntimeFree (row : NRow) (free : row.paramFree = true) (values : HList row) :
    @HList (Carriers.runtime unit) row :=
  cast (congrArg (@HList (Carriers.runtime unit)) (NRow.resolved_paramFree row free))
    (HList.toRuntime row values)

theorem HList.encode_toRuntimeFree (row : NRow) (free : row.paramFree = true) (values : HList row) :
    @HList.encode (Carriers.runtime unit) row (HList.toRuntimeFree row free values) =
      HList.encode values := by
  unfold HList.toRuntimeFree
  rw [HList.encode_castCarrier (NRow.resolved_paramFree row free), HList.encode_toRuntime]

@[vir_denote_norm] theorem HList.toRuntimeFree_nil (free : NRow.nil.paramFree = true)
    (values : HList .nil) : HList.toRuntimeFree (unit := unit) .nil free values = () := rfl

@[vir_denote_norm] theorem HList.toRuntimeFree_cons (τ : NTy) (rest : NRow)
    (free : (NRow.cons τ rest).paramFree = true) (values : HList (.cons τ rest)) :
    HList.toRuntimeFree (.cons τ rest) free values =
      (Θ.toRuntimeFree τ (by simp only [NRow.paramFree, Bool.and_eq_true] at free; exact free.1)
          values.1,
        HList.toRuntimeFree rest (by simp only [NRow.paramFree, Bool.and_eq_true] at free; exact free.2)
          values.2) := by
  apply (@rowCodec (Carriers.runtime unit) (.cons τ rest)).encode_injective
  show @HList.encode (Carriers.runtime unit) _ _ = @HList.encode (Carriers.runtime unit) _ _
  rw [HList.encode_toRuntimeFree, HList.encode_cons]
  show _ = @NTy.encode (Carriers.runtime unit) τ _ :: @HList.encode (Carriers.runtime unit) rest _
  rw [Θ.encode_toRuntimeFree, HList.encode_toRuntimeFree]

omit Θ in
/-- A construction at the base frame at equal rows and captures. -/
theorem Skolems.construct_heq_runtime {parameters parameters' results results' captured captured' : NRow}
    (shared : Sharing) (handle : FunctionHandle) (mask : Nat)
    (typeInstantiation : Array (TypeId × TypeId)) (sameParameters : parameters = parameters')
    (sameResults : results = results') (sameCaptured : captured = captured')
    {captures : @HList (Carriers.runtime unit) captured}
    {captures' : @HList (Carriers.runtime unit) captured'} (sameCaptures : HEq captures captures')
    (typed : @SiteTyped unit _ (Skolems.runtime unit) parameters shared results handle mask
      typeInstantiation captured)
    (typed' : @SiteTyped unit _ (Skolems.runtime unit) parameters' shared results' handle mask
      typeInstantiation captured') :
    HEq (@Skolems.construct unit _ (Skolems.runtime unit) parameters shared results handle mask
        typeInstantiation captured captures typed)
      (@Skolems.construct unit _ (Skolems.runtime unit) parameters' shared results' handle mask
        typeInstantiation captured' captures' typed') := by
  subst sameParameters sameResults sameCaptured
  cases sameCaptures
  rfl

/-- `requires_of` of a construction at the frame of rows without type
parameters: the requires meaning of the construction at the base frame at
the same rows, which a closure site's generated equations read. -/
theorem requiresOf_construct_paramFree (requires : RequiresMeaning unit) (parameters : NRow)
    (shared : Sharing) (results : NRow) (handle : FunctionHandle) (mask : Nat)
    (typeInstantiation : Array (TypeId × TypeId)) {σs : NRow} (captures : HList σs)
    (typed : SiteTyped parameters shared results handle mask typeInstantiation σs)
    (args : HList parameters) (state : Memory unit) (parametersFree : parameters.paramFree = true)
    (resultsFree : results.paramFree = true) (capturedFree : σs.paramFree = true) :
    RequiresOf requires parameters shared results
        (Skolems.construct parameters shared results handle mask typeInstantiation captures typed)
        args state =
      requires parameters shared results
        (@Skolems.construct unit _ (Skolems.runtime unit) parameters shared results handle mask
          typeInstantiation σs
          (HList.toRuntimeFree σs capturedFree captures)
          (by
            have rows := typed.rows
            rw [NRow.resolved_paramFree σs capturedFree,
              NRow.resolved_paramFree parameters parametersFree,
              NRow.resolved_paramFree results resultsFree] at rows
            exact @SiteTyped.mk unit _ (Skolems.runtime unit) _ _ _ _ _ _ _
              (by simpa only [NRow.resolved_runtime] using rows) typed.sharing typed.faithful))
        (HList.toRuntimeFree parameters parametersFree args) state := by
  rw [requiresOf_construct]
  exact RequiresMeaning.congr requires shared (NRow.resolved_paramFree parameters parametersFree)
    (NRow.resolved_paramFree results resultsFree)
    (Skolems.construct_heq_runtime shared handle mask typeInstantiation
      (NRow.resolved_paramFree parameters parametersFree) (NRow.resolved_paramFree results resultsFree)
      (NRow.resolved_paramFree σs capturedFree) (cast_heq _ _).symm _ _)
    (cast_heq _ _).symm state

end

/-! ## Named targets

A call of a target is an invocation of the function value naming it, which
captures nothing (`Weave.supplying`): the continuations of a call may assume
its behavioral predicates. -/

section
variable {unit : ValidatedUnit} [FunctionCarriers unit] [Skolems unit]

/-- Invoking a function value whose target, weave, and captures a proof sees,
at a frame coherent with its instantiation, is the target's prophetic meaning
on the woven arguments. -/
theorem closureMeaning_closureOf_coherent (executable : ExecutableUnit unit)
    {handle : FunctionHandle} {full captured supplied : NRow} (weave : Weave full captured supplied)
    (capturedFree : captured.refFree = true) {typeInstantiation : Array (TypeId × TypeId)}
    (coherent : Coherent unit handle typeInstantiation) (captures : HList captured)
    (shared : Sharing) (shape : ResultShape)
    (function : (NTy.function supplied shared shape.row).carrier)
    (closed : (Carriers.functions supplied shared shape.row).closure function =
      closureOf handle weave.mask typeInstantiation captures)
    (args : HList supplied) :
    closureMeaning executable supplied shared shape
        function
        args =
      propheticMeaning executable typeInstantiation handle full shape (weave.compose captures args) := by
  simp only [closureMeaning, closed]
  rw [closureRun_closureOf executable handle weave capturedFree
    typeInstantiation captures shape args, propheticMeaning_of_coherent coherent]

/-- `ensures_of` of a function value whose target, weave, and captures a proof
sees is a run of the target's prophetic meaning on the woven arguments. -/
theorem ensuresOf_closureOf {executable : ExecutableUnit unit} {handle : FunctionHandle}
    {full captured supplied : NRow} (weave : Weave full captured supplied)
    (capturedFree : captured.refFree = true) {typeInstantiation : Array (TypeId × TypeId)}
    (coherent : Coherent unit handle typeInstantiation) (captures : HList captured)
    {shared : Sharing} {shape : ResultShape}
    {function : (NTy.function supplied shared shape.row).carrier}
    (args : HList supplied) {result : shape.carrier} {pre post : Memory unit}
    (ensures : EnsuresOf (closureMeaning executable) supplied shared shape
      function
      args result pre post)
    (closed : (Carriers.functions supplied shared shape.row).closure function =
      closureOf handle weave.mask typeInstantiation captures) :
    (propheticMeaning executable typeInstantiation handle full shape
      (weave.compose captures args)).ok pre result post := by
  rw [← closureMeaning_closureOf_coherent executable weave capturedFree coherent captures shared
    shape function closed args]
  exact ensures

/-- `aborts_of` of a function value whose target, weave, and captures a proof
sees is an abort of the target's prophetic meaning on the woven arguments. -/
theorem abortsOf_closureOf {executable : ExecutableUnit unit} {handle : FunctionHandle}
    {full captured supplied : NRow} (weave : Weave full captured supplied)
    (capturedFree : captured.refFree = true) {typeInstantiation : Array (TypeId × TypeId)}
    (coherent : Coherent unit handle typeInstantiation) (captures : HList captured)
    {shared : Sharing} {shape : ResultShape}
    {function : (NTy.function supplied shared shape.row).carrier}
    (args : HList supplied) {state : Memory unit}
    (aborts : AbortsOf (closureMeaning executable) supplied shared shape
      function
      args state)
    (closed : (Carriers.functions supplied shared shape.row).closure function =
      closureOf handle weave.mask typeInstantiation captures) :
    ∃ failure, (propheticMeaning executable typeInstantiation handle full shape
      (weave.compose captures args)).aborts state failure := by
  rw [← closureMeaning_closureOf_coherent executable weave capturedFree coherent captures shared
    shape function closed args]
  exact aborts

/-- `ensures_of` of a function value whose target is verified: where what the
target's theorem assumes and its precondition hold at the woven arguments,
what its contract ensures of the run, its frame, and that no condition under
which it must abort held. -/
theorem ensuresOf_closureOf_verified {executable : ExecutableUnit unit} {handle : FunctionHandle}
    {full captured supplied : NRow} (weave : Weave full captured supplied)
    (capturedFree : captured.refFree = true) {typeInstantiation : Array (TypeId × TypeId)}
    (coherent : Coherent unit handle typeInstantiation) (captures : HList captured)
    {shared : Sharing} {shape : ResultShape}
    {function : (NTy.function supplied shared shape.row).carrier}
    (args : HList supplied) {result : shape.carrier} {pre post : Memory unit}
    {contract : Contract (Memory unit) Failure (HList full) shape.carrier}
    (verified : Satisfies (propheticMeaning executable typeInstantiation handle full shape) contract)
    (ensures : EnsuresOf (closureMeaning executable) supplied shared shape
      function
      args result pre post)
    (closed : (Carriers.functions supplied shared shape.row).closure function =
      closureOf handle weave.mask typeInstantiation captures)
    (assumed : contract.assumes (weave.compose captures args) pre)
    (permitted : contract.requires (weave.compose captures args) pre) :
    (¬contract.mayAbort (weave.compose captures args) pre →
        contract.ensures (weave.compose captures args) pre result post) ∧
      contract.frame (weave.compose captures args) pre result post ∧
      ¬contract.mustAbort (weave.compose captures args) pre :=
  (verified _ pre assumed permitted).1 result post
    (ensuresOf_closureOf weave capturedFree coherent captures args ensures closed)

/-- `aborts_of` of a function value whose target is verified: where what the
target's theorem assumes and its precondition hold at the woven arguments,
a failure its contract permits. -/
theorem abortsOf_closureOf_verified {executable : ExecutableUnit unit} {handle : FunctionHandle}
    {full captured supplied : NRow} (weave : Weave full captured supplied)
    (capturedFree : captured.refFree = true) {typeInstantiation : Array (TypeId × TypeId)}
    (coherent : Coherent unit handle typeInstantiation) (captures : HList captured)
    {shared : Sharing} {shape : ResultShape}
    {function : (NTy.function supplied shared shape.row).carrier}
    (args : HList supplied) {state : Memory unit}
    {contract : Contract (Memory unit) Failure (HList full) shape.carrier}
    (verified : Satisfies (propheticMeaning executable typeInstantiation handle full shape) contract)
    (aborts : AbortsOf (closureMeaning executable) supplied shared shape
      function
      args state)
    (closed : (Carriers.functions supplied shared shape.row).closure function =
      closureOf handle weave.mask typeInstantiation captures)
    (assumed : contract.assumes (weave.compose captures args) state)
    (permitted : contract.requires (weave.compose captures args) state) :
    ∃ failure, contract.aborts (weave.compose captures args) state failure := by
  obtain ⟨failure, runs⟩ := abortsOf_closureOf weave capturedFree coherent captures args aborts closed
  exact ⟨failure, (verified _ state assumed permitted).2.1 failure runs⟩

/-- `aborts_of` of a function value capturing values, as of a function value
naming its target (`named`), at the woven arguments. -/
theorem abortsOf_closureOf_woven {executable : ExecutableUnit unit} {handle : FunctionHandle}
    {full : NRow} {τ : NTy} {captured supplied : NRow}
    (weave : Weave full (.cons τ captured) supplied) (free : (NRow.cons τ captured).refFree = true)
    (typeInstantiation : Array (TypeId × TypeId)) (captures : HList (.cons τ captured))
    (shared : Sharing) (shape : ResultShape)
    (function : (NTy.function supplied shared shape.row).carrier)
    (closed : (Carriers.functions supplied shared shape.row).closure function =
      closureOf handle weave.mask typeInstantiation captures)
    {named_shared : Sharing} (named : (NTy.function full named_shared shape.row).carrier)
    (naming : (Carriers.functions full named_shared shape.row).closure named =
      closureOf handle (Weave.supplying full).mask typeInstantiation (σs := .nil) ())
    (args : HList supplied) (state : Memory unit) :
    AbortsOf (closureMeaning executable) supplied shared shape
        function
        args state =
      AbortsOf (closureMeaning executable) full named_shared shape named
        (weave.compose captures args) state := by
  simp only [AbortsOf, closureMeaning, closed, naming,
    closureRun_closureOf_woven executable handle weave free typeInstantiation captures shape args]

/-- `ensures_of` of a function value capturing values, as of a function value
naming its target (`named`), at the woven arguments. -/
theorem ensuresOf_closureOf_woven {executable : ExecutableUnit unit} {handle : FunctionHandle}
    {full : NRow} {τ : NTy} {captured supplied : NRow}
    (weave : Weave full (.cons τ captured) supplied) (free : (NRow.cons τ captured).refFree = true)
    (typeInstantiation : Array (TypeId × TypeId)) (captures : HList (.cons τ captured))
    (shared : Sharing) (shape : ResultShape)
    (function : (NTy.function supplied shared shape.row).carrier)
    (closed : (Carriers.functions supplied shared shape.row).closure function =
      closureOf handle weave.mask typeInstantiation captures)
    {named_shared : Sharing} (named : (NTy.function full named_shared shape.row).carrier)
    (naming : (Carriers.functions full named_shared shape.row).closure named =
      closureOf handle (Weave.supplying full).mask typeInstantiation (σs := .nil) ())
    (args : HList supplied) (result : shape.carrier) (pre post : Memory unit) :
    EnsuresOf (closureMeaning executable) supplied shared shape
        function
        args result pre post =
      EnsuresOf (closureMeaning executable) full named_shared shape named
        (weave.compose captures args) result pre post := by
  simp only [EnsuresOf, closureMeaning, closed, naming,
    closureRun_closureOf_woven executable handle weave free typeInstantiation captures shape args]

/-- Invoking a function value naming a target is running the target. -/
theorem closureMeaning_named (executable : ExecutableUnit unit) {handle : FunctionHandle}
    {typeInstantiation : Array (TypeId × TypeId)} (coherent : Coherent unit handle typeInstantiation)
    (full : NRow) (shared : Sharing) (shape : ResultShape)
    (named : (NTy.function full shared shape.row).carrier)
    (naming : (Carriers.functions full shared shape.row).closure named =
      closureOf handle (Weave.supplying full).mask typeInstantiation (σs := .nil) ())
    (args : HList full) :
    closureMeaning executable full shared shape named args =
      propheticMeaning executable typeInstantiation handle full shape args := by
  simp only [closureMeaning, naming]
  rw [closureRun_closureOf executable handle (Weave.supplying full) rfl typeInstantiation () shape
    args, Weave.compose_supplying, propheticMeaning_of_coherent coherent]

/-- Where a target is called, the run is one of a function value naming it
(`named`): the continuations may assume its `ensures_of` where the call
returns and its `aborts_of` where it aborts. -/
theorem wp_named {executable : ExecutableUnit unit} {handle : FunctionHandle}
    {typeInstantiation : Array (TypeId × TypeId)} {full : NRow} {shape : ResultShape}
    {shared : Sharing} {named : (NTy.function full shared shape.row).carrier}
    (naming : (Carriers.functions full shared shape.row).closure named =
      closureOf handle (Weave.supplying full).mask typeInstantiation (σs := .nil) ())
    {args : HList full} {ensures : shape.carrier → Memory unit → Prop} {aborts : Failure → Prop}
    {initial : Memory unit}
    (continued : wp (propheticMeaning executable typeInstantiation handle full shape args)
      (fun result final =>
        EnsuresOf (closureMeaning executable) full shared shape named args result initial final →
        ensures result final)
      (fun error =>
        AbortsOf (closureMeaning executable) full shared shape named args initial → aborts error)
      initial) :
    wp (propheticMeaning executable typeInstantiation handle full shape args) ensures aborts
      initial :=
  ⟨fun result final execution => continued.1 result final execution (by
      unfold EnsuresOf
      rw [closureMeaning_named executable execution.1 full shared shape named naming]
      exact execution),
    fun error execution => continued.2.1 error execution ⟨error, by
      rw [closureMeaning_named executable execution.1 full shared shape named naming]
      exact execution⟩,
    continued.2.2⟩

end

/-! ## Units verified through their models

In a unit verified through its model (`doc/designs/ghost-fields.md`), an
invocation means what the model's invocation meaning says, and the
behavioral predicates are stated over it. Of a closure site's constructor
of a routed target, at the base frame, they are the target's meaning on the
woven arguments (the theorem's routing hypotheses). -/

/-- A unit verified through its model (`doc/designs/ghost-fields.md`): its
executable, which its contracts read the unit from, and the meaning of
invoking a function value at every family, coherent across the families type
arguments induce as the executable's is (`closureMeaning_coherent`). A
contract of such a unit that reads the unit takes a model where another
takes the executable. -/
structure ModelUnit (unit : ValidatedUnit) [FunctionCarriers unit] where
  executable : ExecutableUnit unit
  invoke : (Θ : Skolems unit) → @Denote.InvokeMeaning unit _ Θ
  coherent : Denote.InvokeMeaning.Coherent invoke

/-! ## Behavioral predicates at induced frames

A behavioral predicate a contract states at the frame a call's type
arguments induce (`Skolems.instantiate`) is the predicate at the outer frame,
of the arguments, results and runs read in its view, by the coherence of
the invocation meaning (`InvokeMeaning.Coherent`); `result_of` and
`state_of` choose among the runs' encodings, which both views hold alike. -/

section Induced
variable {unit : ValidatedUnit} [FunctionCarriers unit]

/-- `ensures_of` at an induced frame is `ensures_of` at the outer frame, of
the arguments and result read in its view. -/
theorem ensuresOf_instantiate {invoke : (Θ : Skolems unit) → @InvokeMeaning unit _ Θ}
    (coherent : InvokeMeaning.Coherent invoke) (Θ : Skolems unit) (θ : TypeArgs)
    (free : θ.1.refFree = true) (σs : NRow) (shared : Sharing) (shape : ResultShape)
    (function : @NTy.carrier (Skolems.instantiate θ Θ).toCarriers (.function σs shared shape.row))
    (args : @HList (Skolems.instantiate θ Θ).toCarriers σs)
    (result : @ResultShape.carrier (Skolems.instantiate θ Θ).toCarriers shape)
    (pre post : Memory unit) :
    @EnsuresOf unit _ (Skolems.instantiate θ Θ) (invoke (Skolems.instantiate θ Θ)) σs shared shape
        function args result pre post =
      @EnsuresOf unit _ Θ (invoke Θ) (NRow.subst θ.1 σs) shared (shape.subst θ.1)
        (shape.functionOfSkolem θ σs shared function) (HList.ofSkolem θ σs args)
        (ResultShape.ofSkolem θ shape result) pre post := by
  apply propext
  unfold EnsuresOf
  rw [coherent Θ θ free]
  simp only [Spec.bind_ok, Spec.pure_ok]
  constructor
  · rintro ⟨value, middle, ran, rfl, rfl⟩
    rw [ResultShape.ofSkolem_toSkolem]
    exact ran
  · intro ran
    exact ⟨_, post, ran, (@NTy.toSkolem_ofSkolem_shape unit _ Θ θ shape result).symm, rfl⟩

/-- `aborts_of` at an induced frame, at the outer frame. -/
theorem abortsOf_instantiate {invoke : (Θ : Skolems unit) → @InvokeMeaning unit _ Θ}
    (coherent : InvokeMeaning.Coherent invoke) (Θ : Skolems unit) (θ : TypeArgs)
    (free : θ.1.refFree = true) (σs : NRow) (shared : Sharing) (shape : ResultShape)
    (function : @NTy.carrier (Skolems.instantiate θ Θ).toCarriers (.function σs shared shape.row))
    (args : @HList (Skolems.instantiate θ Θ).toCarriers σs) (state : Memory unit) :
    @AbortsOf unit _ (Skolems.instantiate θ Θ) (invoke (Skolems.instantiate θ Θ)) σs shared shape
        function args state =
      @AbortsOf unit _ Θ (invoke Θ) (NRow.subst θ.1 σs) shared (shape.subst θ.1)
        (shape.functionOfSkolem θ σs shared function) (HList.ofSkolem θ σs args) state := by
  apply propext
  unfold AbortsOf
  rw [coherent Θ θ free]
  simp only [Spec.bind_aborts, Spec.pure_aborts, and_false, exists_false, or_false]

/-- `requires_of` at an induced frame is `requires_of` at the outer frame, of
the arguments read in its view: both carry the value and the arguments to
the base frame alike. -/
theorem requiresOf_instantiate (Θ : Skolems unit) (θ : TypeArgs) (requires : RequiresMeaning unit)
    (σs : NRow) (shared : Sharing) (results : NRow)
    (function : @NTy.carrier (Skolems.instantiate θ Θ).toCarriers (.function σs shared results))
    (args : @HList (Skolems.instantiate θ Θ).toCarriers σs) (state : Memory unit) :
    @RequiresOf unit _ (Skolems.instantiate θ Θ) requires σs shared results function args state =
      @RequiresOf unit _ Θ requires (NRow.subst θ.1 σs) shared (NRow.subst θ.1 results) function
        (HList.ofSkolem θ σs args) state := by
  unfold RequiresOf Skolems.functionToRuntime
  apply RequiresMeaning.congr requires shared (NRow.resolved_instantiate θ σs)
    (NRow.resolved_instantiate θ results)
  · exact (cast_heq _ _).trans (cast_heq _ _).symm
  · exact HList.toRuntime_instantiate θ Θ σs args

section Settle

/-- Settling a value without a reference leaves it. -/
theorem Denote.NTy.settle_self_of_refFree [Carriers] (τ : NTy) (free : τ.refFree = true)
    (value : τ.carrier) : NTy.settle τ value = value := by
  cases τ <;> first | rfl | simp [NTy.refFree] at free

/-- Settling commutes with reading a value in the outer view, where the
type arguments hold no reference. -/
theorem Denote.NTy.settle_ofSkolem [Θ : Carriers] (θ : TypeArgs) (free : θ.1.refFree = true) :
    (τ : NTy) → (value : @NTy.carrier (Carriers.instantiate θ Θ) τ) →
    NTy.settle (τ.subst θ.1) (NTy.ofSkolem θ τ value) =
      NTy.ofSkolem θ τ (@NTy.settle (Carriers.instantiate θ Θ) τ value)
  | .param index, _ =>
      NTy.settle_self_of_refFree _ (NRow.getD_refFree θ.1 free index _ rfl) _
  | .ref _, _ | .unit, _ | .bool, _ | .int _ _, _ | .num, _ | .address, _ | .signer, _
  | .string, _ | .bytes, _ | .tuple _, _ | .struct _ _ _, _ | .enum _ _ _ _ _, _ | .vector _, _
  | .table _ _, _ | .ghost _, _ | .function _ _ _, _ => rfl

theorem Denote.HList.settle_ofSkolem [Θ : Carriers] (θ : TypeArgs) (free : θ.1.refFree = true) :
    (row : NRow) → (values : @HList (Carriers.instantiate θ Θ) row) →
    HList.settle (NRow.subst θ.1 row) (HList.ofSkolem θ row values) =
      HList.ofSkolem θ row (@HList.settle (Carriers.instantiate θ Θ) row values)
  | .nil, _ => rfl
  | .cons τ rest, values => by
      show (NTy.settle (τ.subst θ.1) (NTy.ofSkolem θ τ values.1),
          HList.settle (NRow.subst θ.1 rest) (HList.ofSkolem θ rest values.2)) =
        (NTy.ofSkolem θ τ (@NTy.settle (Carriers.instantiate θ Θ) τ values.1),
          HList.ofSkolem θ rest (@HList.settle (Carriers.instantiate θ Θ) rest values.2))
      rw [NTy.settle_ofSkolem θ free τ, HList.settle_ofSkolem θ free rest]

/-- A value without a reference is settled. -/
theorem Denote.ResultShape.settled_one_of_refFree [Carriers] (τ : NTy) (free : τ.refFree = true)
    (value : τ.carrier) : (ResultShape.one τ).Settled value := by
  cases τ <;> first | trivial | simp [NTy.refFree] at free

/-- A result is settled in either view alike. -/
theorem Denote.ResultShape.settled_ofSkolem [Θ : Carriers] (θ : TypeArgs)
    (free : θ.1.refFree = true) : (shape : ResultShape) →
    (value : @ResultShape.carrier (Carriers.instantiate θ Θ) shape) →
    ((shape.subst θ.1).Settled (ResultShape.ofSkolem θ shape value) ↔
      @ResultShape.Settled (Carriers.instantiate θ Θ) shape value)
  | .none, _ => Iff.rfl
  | .one (.ref τ), value => by
      show NTy.ofSkolem θ τ value.1 = NTy.ofSkolem θ τ value.2 ↔ value.1 = value.2
      constructor
      · intro same
        have := congrArg (NTy.toSkolem θ τ) same
        rwa [NTy.toSkolem_ofSkolem, NTy.toSkolem_ofSkolem] at this
      · intro same; rw [same]
  | .one (.param index), value => by
      have settled := ResultShape.settled_one_of_refFree ((NTy.param index).subst θ.1)
        (NRow.getD_refFree θ.1 free index _ rfl) (ResultShape.ofSkolem θ (.one (.param index)) value)
      exact ⟨fun _ => trivial, fun _ => settled⟩
  | .one .unit, _ | .one .bool, _ | .one (.int _ _), _ | .one .num, _ | .one .address, _
  | .one .signer, _ | .one .string, _ | .one .bytes, _ | .one (.tuple _), _
  | .one (.struct _ _ _), _ | .one (.enum _ _ _ _ _), _ | .one (.vector _), _
  | .one (.table _ _), _ | .one (.ghost _), _ | .one (.function _ _ _), _ => Iff.rfl

/-- A result encodes alike in either view. -/
theorem Denote.resultCodec_encode_ofSkolem [Θ : Carriers] (θ : TypeArgs) : (shape : ResultShape) →
    (value : @ResultShape.carrier (Carriers.instantiate θ Θ) shape) →
    (resultCodec (shape.subst θ.1)).encode (ResultShape.ofSkolem θ shape value) =
      (@resultCodec (Carriers.instantiate θ Θ) shape).encode value
  | .none, _ => rfl
  | .one τ, value => by
      show #[NTy.encode (τ.subst θ.1) (NTy.ofSkolem θ τ value)] =
        #[@NTy.encode (Carriers.instantiate θ Θ) τ value]
      rw [NTy.encode_ofSkolem]

end Settle

section Runs
variable {invoke : (Θ : Skolems unit) → @InvokeMeaning unit _ Θ}

/-- The settled runs' encodings are the same at the induced frame as at the
outer frame on the arguments read in its view. -/
theorem settledEncoding_instantiate (coherent : InvokeMeaning.Coherent invoke) (Θ : Skolems unit)
    (θ : TypeArgs) (free : θ.1.refFree = true) (σs : NRow) (shared : Sharing)
    (shape : ResultShape) (function : @NTy.carrier (Skolems.instantiate θ Θ).toCarriers (.function σs shared shape.row))
    (args : @HList (Skolems.instantiate θ Θ).toCarriers σs) (state : Memory unit) :
    @SettledEncoding unit _ (Skolems.instantiate θ Θ) (invoke (Skolems.instantiate θ Θ)) σs shared
        shape function args state =
      @SettledEncoding unit _ Θ (invoke Θ) (NRow.subst θ.1 σs) shared (shape.subst θ.1)
        (shape.functionOfSkolem θ σs shared function) (HList.ofSkolem θ σs args) state := by
  funext encoded
  apply propext
  constructor
  · rintro ⟨⟨values, result, final⟩, ⟨settles, settled, ran⟩, rfl⟩
    refine ⟨(HList.ofSkolem θ σs values, ResultShape.ofSkolem θ shape result, final),
      ⟨?_, ?_, ?_⟩, ?_⟩
    · show HList.settle _ (HList.ofSkolem θ σs values) = HList.settle _ (HList.ofSkolem θ σs args)
      rw [HList.settle_ofSkolem θ free, HList.settle_ofSkolem θ free]
      exact congrArg (HList.ofSkolem θ σs) settles
    · exact (ResultShape.settled_ofSkolem θ free shape result).mpr settled
    · rw [← ensuresOf_instantiate coherent Θ θ free]; exact ran
    · show (HList.encode (HList.ofSkolem θ σs values),
          (resultCodec (shape.subst θ.1)).encode (ResultShape.ofSkolem θ shape result), final) = _
      rw [HList.encode_ofSkolem, resultCodec_encode_ofSkolem]
      rfl
  · rintro ⟨⟨values, result, final⟩, ⟨settles, settled, ran⟩, rfl⟩
    refine ⟨(HList.toSkolem θ σs values, ResultShape.toSkolem θ shape result, final),
      ⟨?_, ?_, ?_⟩, ?_⟩
    · have := congrArg (HList.toSkolem θ σs) settles
      rw [← HList.ofSkolem_toSkolem θ σs values, HList.settle_ofSkolem θ free,
        HList.settle_ofSkolem θ free, HList.toSkolem_ofSkolem, HList.toSkolem_ofSkolem] at this
      exact this
    · rw [← ResultShape.settled_ofSkolem θ free, ResultShape.ofSkolem_toSkolem]; exact settled
    · rw [ensuresOf_instantiate coherent Θ θ free, HList.ofSkolem_toSkolem,
        ResultShape.ofSkolem_toSkolem]
      exact ran
    · show (@HList.encode (Skolems.instantiate θ Θ).toCarriers σs (HList.toSkolem θ σs values),
          (@resultCodec (Skolems.instantiate θ Θ).toCarriers shape).encode
            (ResultShape.toSkolem θ shape result), final) = _
      rw [← HList.encode_ofSkolem, ← resultCodec_encode_ofSkolem, HList.ofSkolem_toSkolem,
        ResultShape.ofSkolem_toSkolem]
      rfl

/-- `state_of` at an induced frame is `state_of` at the outer frame. -/
theorem stateOf_instantiate (coherent : InvokeMeaning.Coherent invoke) (Θ : Skolems unit)
    (θ : TypeArgs) (free : θ.1.refFree = true) (σs : NRow) (shared : Sharing)
    (shape : ResultShape) (function : @NTy.carrier (Skolems.instantiate θ Θ).toCarriers (.function σs shared shape.row))
    (args : @HList (Skolems.instantiate θ Θ).toCarriers σs) (state : Memory unit) :
    @StateOf unit _ (Skolems.instantiate θ Θ) (invoke (Skolems.instantiate θ Θ)) σs shared shape
        function args state =
      @StateOf unit _ Θ (invoke Θ) (NRow.subst θ.1 σs) shared (shape.subst θ.1)
        (shape.functionOfSkolem θ σs shared function) (HList.ofSkolem θ σs args) state := by
  unfold StateOf
  rw [settledEncoding_instantiate coherent Θ θ free]

/-- `result_of` at an induced frame is `result_of` at the outer frame, carried
back. -/
theorem resultOf_instantiate (coherent : InvokeMeaning.Coherent invoke) (Θ : Skolems unit)
    (θ : TypeArgs) (free : θ.1.refFree = true) (σs : NRow) (shared : Sharing)
    (shape : ResultShape) (function : @NTy.carrier (Skolems.instantiate θ Θ).toCarriers (.function σs shared shape.row))
    (args : @HList (Skolems.instantiate θ Θ).toCarriers σs) (state : Memory unit)
    (default : @ResultShape.carrier (Skolems.instantiate θ Θ).toCarriers shape) :
    @ResultOf unit _ (Skolems.instantiate θ Θ) (invoke (Skolems.instantiate θ Θ)) σs shared shape
        function args state default =
      ResultShape.toSkolem θ shape (@ResultOf unit _ Θ (invoke Θ) (NRow.subst θ.1 σs) shared
        (shape.subst θ.1) (shape.functionOfSkolem θ σs shared function) (HList.ofSkolem θ σs args) state
        (ResultShape.ofSkolem θ shape default)) := by
  unfold ResultOf
  rw [settledEncoding_instantiate coherent Θ θ free]
  split
  · rename_i returns
    obtain ⟨⟨values, result, final⟩, _, chosen⟩ := returns.choose_spec
    rw [← chosen]
    show ((@resultCodec (Skolems.instantiate θ Θ).toCarriers shape).decode?
        ((resultCodec (shape.subst θ.1)).encode result)).getD default =
      ResultShape.toSkolem θ shape (((resultCodec (shape.subst θ.1)).decode?
        ((resultCodec (shape.subst θ.1)).encode result)).getD (ResultShape.ofSkolem θ shape default))
    rw [Codec.decode_encode, Option.getD_some, ← ResultShape.ofSkolem_toSkolem θ shape result,
      resultCodec_encode_ofSkolem, Codec.decode_encode, Option.getD_some,
      ResultShape.ofSkolem_toSkolem]
  · exact (@NTy.toSkolem_ofSkolem_shape unit _ Θ θ shape default).symm

end Runs


/-! The behavioral predicates of the executable's invocations and of a
model's at an induced frame, at the outer frame: the rewrite rules of the
transport's normal form (`vir_transport`), which reads each coherence. -/

section Instances
variable (executable : ExecutableUnit unit) (model : ModelUnit unit) (Θ : Skolems unit)
  (θ : TypeArgs) (σs : NRow) (shared : Sharing) (shape : ResultShape)
  (function : @NTy.carrier (Skolems.instantiate θ Θ).toCarriers (.function σs shared shape.row))
  (args : @HList (Skolems.instantiate θ Θ).toCarriers σs) (state : Memory unit)

theorem closureMeaning_ensuresOf_instantiate (free : θ.1.refFree = true)
    (result : @ResultShape.carrier (Skolems.instantiate θ Θ).toCarriers shape) (post : Memory unit) :
    @EnsuresOf unit _ (Skolems.instantiate θ Θ) (@closureMeaning unit _ executable (Skolems.instantiate θ Θ))
        σs shared shape function args result state post =
      @EnsuresOf unit _ Θ (@closureMeaning unit _ executable Θ) (NRow.subst θ.1 σs) shared
        (shape.subst θ.1) (shape.functionOfSkolem θ σs shared function) (HList.ofSkolem θ σs args)
        (ResultShape.ofSkolem θ shape result) state post :=
  ensuresOf_instantiate (closureMeaning_coherent executable) Θ θ free σs shared shape function args
    result state post

theorem closureMeaning_abortsOf_instantiate (free : θ.1.refFree = true) :
    @AbortsOf unit _ (Skolems.instantiate θ Θ) (@closureMeaning unit _ executable (Skolems.instantiate θ Θ))
        σs shared shape function args state =
      @AbortsOf unit _ Θ (@closureMeaning unit _ executable Θ) (NRow.subst θ.1 σs) shared
        (shape.subst θ.1) (shape.functionOfSkolem θ σs shared function) (HList.ofSkolem θ σs args)
        state :=
  abortsOf_instantiate (closureMeaning_coherent executable) Θ θ free σs shared shape function args
    state

theorem closureMeaning_resultOf_instantiate (free : θ.1.refFree = true)
    (default : @ResultShape.carrier (Skolems.instantiate θ Θ).toCarriers shape) :
    @ResultOf unit _ (Skolems.instantiate θ Θ) (@closureMeaning unit _ executable (Skolems.instantiate θ Θ))
        σs shared shape function args state default =
      ResultShape.toSkolem θ shape (@ResultOf unit _ Θ (@closureMeaning unit _ executable Θ)
        (NRow.subst θ.1 σs) shared (shape.subst θ.1) (shape.functionOfSkolem θ σs shared function)
        (HList.ofSkolem θ σs args) state (ResultShape.ofSkolem θ shape default)) :=
  resultOf_instantiate (closureMeaning_coherent executable) Θ θ free σs shared shape function args
    state default

theorem closureMeaning_stateOf_instantiate (free : θ.1.refFree = true) :
    @StateOf unit _ (Skolems.instantiate θ Θ) (@closureMeaning unit _ executable (Skolems.instantiate θ Θ))
        σs shared shape function args state =
      @StateOf unit _ Θ (@closureMeaning unit _ executable Θ) (NRow.subst θ.1 σs) shared
        (shape.subst θ.1) (shape.functionOfSkolem θ σs shared function) (HList.ofSkolem θ σs args)
        state :=
  stateOf_instantiate (closureMeaning_coherent executable) Θ θ free σs shared shape function args
    state

theorem ModelUnit.ensuresOf_instantiate (free : θ.1.refFree = true)
    (result : @ResultShape.carrier (Skolems.instantiate θ Θ).toCarriers shape) (post : Memory unit) :
    @EnsuresOf unit _ (Skolems.instantiate θ Θ) (model.invoke (Skolems.instantiate θ Θ))
        σs shared shape function args result state post =
      @EnsuresOf unit _ Θ (model.invoke Θ) (NRow.subst θ.1 σs) shared (shape.subst θ.1)
        (shape.functionOfSkolem θ σs shared function) (HList.ofSkolem θ σs args)
        (ResultShape.ofSkolem θ shape result) state post :=
  VerexIR.Proofs.ensuresOf_instantiate model.coherent Θ θ free σs shared shape function args result
    state post

theorem ModelUnit.abortsOf_instantiate (free : θ.1.refFree = true) :
    @AbortsOf unit _ (Skolems.instantiate θ Θ) (model.invoke (Skolems.instantiate θ Θ))
        σs shared shape function args state =
      @AbortsOf unit _ Θ (model.invoke Θ) (NRow.subst θ.1 σs) shared (shape.subst θ.1)
        (shape.functionOfSkolem θ σs shared function) (HList.ofSkolem θ σs args) state :=
  VerexIR.Proofs.abortsOf_instantiate model.coherent Θ θ free σs shared shape function args state

theorem ModelUnit.resultOf_instantiate (free : θ.1.refFree = true)
    (default : @ResultShape.carrier (Skolems.instantiate θ Θ).toCarriers shape) :
    @ResultOf unit _ (Skolems.instantiate θ Θ) (model.invoke (Skolems.instantiate θ Θ))
        σs shared shape function args state default =
      ResultShape.toSkolem θ shape (@ResultOf unit _ Θ (model.invoke Θ) (NRow.subst θ.1 σs) shared
        (shape.subst θ.1) (shape.functionOfSkolem θ σs shared function) (HList.ofSkolem θ σs args)
        state (ResultShape.ofSkolem θ shape default)) :=
  VerexIR.Proofs.resultOf_instantiate model.coherent Θ θ free σs shared shape function args state
    default

theorem ModelUnit.stateOf_instantiate (free : θ.1.refFree = true) :
    @StateOf unit _ (Skolems.instantiate θ Θ) (model.invoke (Skolems.instantiate θ Θ))
        σs shared shape function args state =
      @StateOf unit _ Θ (model.invoke Θ) (NRow.subst θ.1 σs) shared (shape.subst θ.1)
        (shape.functionOfSkolem θ σs shared function) (HList.ofSkolem θ σs args) state :=
  VerexIR.Proofs.stateOf_instantiate model.coherent Θ θ free σs shared shape function args state

end Instances

end Induced

-- The solver reads behavioral predicates at concrete rows and results.
attribute [vir_denote_norm] Denote.NTy.refFree Denote.NRow.refFree Denote.NRows.refFree
attribute [vir_denote_norm] Denote.ResultShape.Settled Denote.NTy.settle Denote.HList.settle

end VerexIR.Proofs

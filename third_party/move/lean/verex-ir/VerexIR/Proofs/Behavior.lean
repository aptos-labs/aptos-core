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
variable {unit : ValidatedUnit} [Skolems unit]

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

/-- Declared preconditions by target: a function's at its runtime
arguments. A unit's table names the targets its closures have whose
contracts state no behavioral predicate, and holds `True` for every other
function. -/
abbrev RequiresTable (unit : Validation.ValidatedUnit) :=
  FunctionHandle → Array RuntimeValue → Memory unit → Prop

section
variable {unit : ValidatedUnit} [Skolems unit]

/-- `requires_of<f>(x)`: the declared precondition of the target of the
function value's closure, read from a table, at the captures and the
arguments composed by its mask; `True` where there is none to read. -/
def RequiresOf (table : RequiresTable unit) (σs : NRow) (shared : Sharing) (results : NRow)
    (function : (NTy.function σs shared results).carrier) (args : HList σs) (state : Memory unit) : Prop :=
  let closure := (Carriers.functions σs shared results).closure function
  match ClosureMask.compose closure.mask closure.captures.toList (HList.encode args) with
  | some composed => table closure.function composed.toArray state
  | none => True

/-- A table entry's precondition, stated of the rows of native values the
runtime arguments encode, at one row's encoding: the precondition of that
row, as encodings are injective. Stated at the list equation the normal
form makes of the entry's array equation (`toArray_eq_iff`). -/
theorem requiresEntry_encode {row : NRow} (requires : HList row → Prop) (values : HList row) :
    (∀ args : HList row, HList.encode args = HList.encode values → requires args) ↔
      requires values :=
  ⟨fun held => held values rfl, fun held args equal => by
    rw [(HList.encode_inj args values).mp equal]; exact held⟩

/-- `requires_of` holds of every function value where the table declares no
precondition of any target. -/
theorem requiresOf_of_table (table : RequiresTable unit)
    (everywhere : ∀ handle arguments state, table handle arguments state)
    (σs : NRow) (shared : Sharing) (results : NRow) (function : (NTy.function σs shared results).carrier)
    (args : HList σs) (state : Memory unit) :
    RequiresOf table σs shared results function args state := by
  dsimp only [RequiresOf]
  split
  · exact everywhere _ _ _
  · trivial

/-- `requires_of` of a function value whose target, weave, and captures a
proof sees is the target's declared precondition at its woven arguments. -/
theorem requiresOf_closureOf (table : RequiresTable unit) (handle : FunctionHandle)
    {full captured supplied : NRow} (weave : Weave full captured supplied)
    (typeInstantiation : Array (TypeId × TypeId)) (captures : HList captured)
    (shared : Sharing) (results : NRow)
    (typed : FunctionTyped supplied shared results
      handle weave.mask typeInstantiation captures)
    (args : HList supplied) (state : Memory unit) :
    RequiresOf table supplied shared results
        (functionOf supplied shared results handle weave.mask typeInstantiation captures typed)
        args state =
      table handle (HList.encode (weave.compose captures args)).toArray state := by
  simp only [RequiresOf, functionOf_closure, closureOf_captures, closureOf_mask, closureOf_function]
  rw [weave.compose_mask _ _ (HList.encode_length captured captures)
    (HList.encode_length supplied args), weave.encode_compose]

end

/-! ## Named targets

A call of a target is an invocation of the function value naming it, which
captures nothing (`Weave.supplying`): the continuations of a call may assume
its behavioral predicates. -/

section
variable {unit : ValidatedUnit} [Skolems unit]

/-- Invoking a function value whose target, weave, and captures a proof sees,
at a frame coherent with its instantiation, is the target's prophetic meaning
on the woven arguments. -/
theorem closureMeaning_closureOf_coherent (executable : ExecutableUnit unit)
    {handle : FunctionHandle} {full captured supplied : NRow} (weave : Weave full captured supplied)
    (capturedFree : captured.refFree = true) {typeInstantiation : Array (TypeId × TypeId)}
    (coherent : Coherent unit handle typeInstantiation) (captures : HList captured)
    (shared : Sharing) (shape : ResultShape)
    (typed : FunctionTyped supplied shared shape.row
      handle weave.mask typeInstantiation captures)
    (args : HList supplied) :
    closureMeaning executable supplied shared shape
        (functionOf supplied shared shape.row handle weave.mask typeInstantiation captures typed)
        args =
      propheticMeaning executable typeInstantiation handle full shape (weave.compose captures args) := by
  rw [closureMeaning_functionOf, closureRun_closureOf executable handle weave capturedFree
    typeInstantiation captures shape args, propheticMeaning_of_coherent coherent]

/-- `ensures_of` of a function value whose target, weave, and captures a proof
sees is a run of the target's prophetic meaning on the woven arguments. -/
theorem ensuresOf_closureOf {executable : ExecutableUnit unit} {handle : FunctionHandle}
    {full captured supplied : NRow} (weave : Weave full captured supplied)
    (capturedFree : captured.refFree = true) {typeInstantiation : Array (TypeId × TypeId)}
    (coherent : Coherent unit handle typeInstantiation) (captures : HList captured)
    {shared : Sharing} {shape : ResultShape}
    {typed : FunctionTyped supplied shared shape.row
      handle weave.mask typeInstantiation captures}
    (args : HList supplied) {result : shape.carrier} {pre post : Memory unit}
    (ensures : EnsuresOf (closureMeaning executable) supplied shared shape
      (functionOf supplied shared shape.row handle weave.mask typeInstantiation captures typed)
      args result pre post) :
    (propheticMeaning executable typeInstantiation handle full shape
      (weave.compose captures args)).ok pre result post := by
  rw [← closureMeaning_closureOf_coherent executable weave capturedFree coherent captures shared
    shape typed args]
  exact ensures

/-- `aborts_of` of a function value whose target, weave, and captures a proof
sees is an abort of the target's prophetic meaning on the woven arguments. -/
theorem abortsOf_closureOf {executable : ExecutableUnit unit} {handle : FunctionHandle}
    {full captured supplied : NRow} (weave : Weave full captured supplied)
    (capturedFree : captured.refFree = true) {typeInstantiation : Array (TypeId × TypeId)}
    (coherent : Coherent unit handle typeInstantiation) (captures : HList captured)
    {shared : Sharing} {shape : ResultShape}
    {typed : FunctionTyped supplied shared shape.row
      handle weave.mask typeInstantiation captures}
    (args : HList supplied) {state : Memory unit}
    (aborts : AbortsOf (closureMeaning executable) supplied shared shape
      (functionOf supplied shared shape.row handle weave.mask typeInstantiation captures typed)
      args state) :
    ∃ failure, (propheticMeaning executable typeInstantiation handle full shape
      (weave.compose captures args)).aborts state failure := by
  rw [← closureMeaning_closureOf_coherent executable weave capturedFree coherent captures shared
    shape typed args]
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
    {typed : FunctionTyped supplied shared shape.row
      handle weave.mask typeInstantiation captures}
    (args : HList supplied) {result : shape.carrier} {pre post : Memory unit}
    {contract : Contract (Memory unit) Failure (HList full) shape.carrier}
    (verified : Satisfies (propheticMeaning executable typeInstantiation handle full shape) contract)
    (ensures : EnsuresOf (closureMeaning executable) supplied shared shape
      (functionOf supplied shared shape.row handle weave.mask typeInstantiation captures typed)
      args result pre post)
    (assumed : contract.assumes (weave.compose captures args) pre)
    (permitted : contract.requires (weave.compose captures args) pre) :
    (¬contract.mayAbort (weave.compose captures args) pre →
        contract.ensures (weave.compose captures args) pre result post) ∧
      contract.frame (weave.compose captures args) pre result post ∧
      ¬contract.mustAbort (weave.compose captures args) pre :=
  (verified _ pre assumed permitted).1 result post
    (ensuresOf_closureOf weave capturedFree coherent captures args ensures)

/-- `aborts_of` of a function value whose target is verified: where what the
target's theorem assumes and its precondition hold at the woven arguments,
a failure its contract permits. -/
theorem abortsOf_closureOf_verified {executable : ExecutableUnit unit} {handle : FunctionHandle}
    {full captured supplied : NRow} (weave : Weave full captured supplied)
    (capturedFree : captured.refFree = true) {typeInstantiation : Array (TypeId × TypeId)}
    (coherent : Coherent unit handle typeInstantiation) (captures : HList captured)
    {shared : Sharing} {shape : ResultShape}
    {typed : FunctionTyped supplied shared shape.row
      handle weave.mask typeInstantiation captures}
    (args : HList supplied) {state : Memory unit}
    {contract : Contract (Memory unit) Failure (HList full) shape.carrier}
    (verified : Satisfies (propheticMeaning executable typeInstantiation handle full shape) contract)
    (aborts : AbortsOf (closureMeaning executable) supplied shared shape
      (functionOf supplied shared shape.row handle weave.mask typeInstantiation captures typed)
      args state)
    (assumed : contract.assumes (weave.compose captures args) state)
    (permitted : contract.requires (weave.compose captures args) state) :
    ∃ failure, contract.aborts (weave.compose captures args) state failure := by
  obtain ⟨failure, runs⟩ := abortsOf_closureOf weave capturedFree coherent captures args aborts
  exact ⟨failure, (verified _ state assumed permitted).2.1 failure runs⟩

/-- `aborts_of` of a function value capturing values, as of a function value
naming its target (`named`), at the woven arguments. -/
theorem abortsOf_closureOf_woven {executable : ExecutableUnit unit} {handle : FunctionHandle}
    {full : NRow} {τ : NTy} {captured supplied : NRow}
    (weave : Weave full (.cons τ captured) supplied) (free : (NRow.cons τ captured).refFree = true)
    (typeInstantiation : Array (TypeId × TypeId)) (captures : HList (.cons τ captured))
    (shared : Sharing) (shape : ResultShape)
    (typed : FunctionTyped supplied shared shape.row
      handle weave.mask typeInstantiation captures)
    {named_shared : Sharing} (named : (NTy.function full named_shared shape.row).carrier)
    (naming : (Carriers.functions full named_shared shape.row).closure named =
      closureOf handle (Weave.supplying full).mask typeInstantiation (σs := .nil) ())
    (args : HList supplied) (state : Memory unit) :
    AbortsOf (closureMeaning executable) supplied shared shape
        (functionOf supplied shared shape.row handle weave.mask typeInstantiation captures typed)
        args state =
      AbortsOf (closureMeaning executable) full named_shared shape named
        (weave.compose captures args) state := by
  simp only [AbortsOf, closureMeaning, functionOf_closure, naming,
    closureRun_closureOf_woven executable handle weave free typeInstantiation captures shape args]

/-- `ensures_of` of a function value capturing values, as of a function value
naming its target (`named`), at the woven arguments. -/
theorem ensuresOf_closureOf_woven {executable : ExecutableUnit unit} {handle : FunctionHandle}
    {full : NRow} {τ : NTy} {captured supplied : NRow}
    (weave : Weave full (.cons τ captured) supplied) (free : (NRow.cons τ captured).refFree = true)
    (typeInstantiation : Array (TypeId × TypeId)) (captures : HList (.cons τ captured))
    (shared : Sharing) (shape : ResultShape)
    (typed : FunctionTyped supplied shared shape.row
      handle weave.mask typeInstantiation captures)
    {named_shared : Sharing} (named : (NTy.function full named_shared shape.row).carrier)
    (naming : (Carriers.functions full named_shared shape.row).closure named =
      closureOf handle (Weave.supplying full).mask typeInstantiation (σs := .nil) ())
    (args : HList supplied) (result : shape.carrier) (pre post : Memory unit) :
    EnsuresOf (closureMeaning executable) supplied shared shape
        (functionOf supplied shared shape.row handle weave.mask typeInstantiation captures typed)
        args result pre post =
      EnsuresOf (closureMeaning executable) full named_shared shape named
        (weave.compose captures args) result pre post := by
  simp only [EnsuresOf, closureMeaning, functionOf_closure, naming,
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
behavioral predicates are stated over it. Of a literal closure of a routed
target they are the target's meaning on the woven arguments
(`Denote.invokeAt`). -/

/-- A unit verified through its model (`doc/designs/ghost-fields.md`): its
executable, which its contracts read the unit from, and the meaning of
invoking a function value at every family, coherent across the families type
arguments induce as the executable's is (`closureMeaning_coherent`). A
contract of such a unit that reads the unit takes a model where another
takes the executable. -/
structure ModelUnit (unit : ValidatedUnit) where
  executable : ExecutableUnit unit
  invoke : (Θ : Skolems unit) → @Denote.InvokeMeaning unit Θ
  coherent : Denote.InvokeMeaning.Coherent invoke

/-! ## Behavioral predicates at induced frames

A behavioral predicate a contract states at the frame a call's type
arguments induce (`Skolems.instantiate`) is the predicate at the outer frame,
of the arguments, results and runs read in its view, by the coherence of
the invocation meaning (`InvokeMeaning.Coherent`); `result_of` and
`state_of` choose among the runs' encodings, which both views hold alike. -/

section Induced
variable {unit : ValidatedUnit}

/-- `ensures_of` at an induced frame is `ensures_of` at the outer frame, of
the arguments and result read in its view. -/
theorem ensuresOf_instantiate {invoke : (Θ : Skolems unit) → @InvokeMeaning unit Θ}
    (coherent : InvokeMeaning.Coherent invoke) (Θ : Skolems unit) (θ : TypeArgs)
    (free : θ.1.refFree = true) (σs : NRow) (shared : Sharing) (shape : ResultShape)
    (function : @NTy.carrier (Skolems.instantiate θ Θ).toCarriers (.function σs shared shape.row))
    (args : @HList (Skolems.instantiate θ Θ).toCarriers σs)
    (result : @ResultShape.carrier (Skolems.instantiate θ Θ).toCarriers shape)
    (pre post : Memory unit) :
    @EnsuresOf unit (Skolems.instantiate θ Θ) (invoke (Skolems.instantiate θ Θ)) σs shared shape
        function args result pre post =
      @EnsuresOf unit Θ (invoke Θ) (NRow.subst θ.1 σs) shared (shape.subst θ.1)
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
    exact ⟨_, post, ran, (@NTy.toSkolem_ofSkolem_shape unit Θ θ shape result).symm, rfl⟩

/-- `aborts_of` at an induced frame, at the outer frame. -/
theorem abortsOf_instantiate {invoke : (Θ : Skolems unit) → @InvokeMeaning unit Θ}
    (coherent : InvokeMeaning.Coherent invoke) (Θ : Skolems unit) (θ : TypeArgs)
    (free : θ.1.refFree = true) (σs : NRow) (shared : Sharing) (shape : ResultShape)
    (function : @NTy.carrier (Skolems.instantiate θ Θ).toCarriers (.function σs shared shape.row))
    (args : @HList (Skolems.instantiate θ Θ).toCarriers σs) (state : Memory unit) :
    @AbortsOf unit (Skolems.instantiate θ Θ) (invoke (Skolems.instantiate θ Θ)) σs shared shape
        function args state =
      @AbortsOf unit Θ (invoke Θ) (NRow.subst θ.1 σs) shared (shape.subst θ.1)
        (shape.functionOfSkolem θ σs shared function) (HList.ofSkolem θ σs args) state := by
  apply propext
  unfold AbortsOf
  rw [coherent Θ θ free]
  simp only [Spec.bind_aborts, Spec.pure_aborts, and_false, exists_false, or_false]

/-- `requires_of` reads the arguments' encodings, which both views hold
alike. -/
theorem requiresOf_instantiate (Θ : Skolems unit) (θ : TypeArgs) (table : RequiresTable unit)
    (σs : NRow) (shared : Sharing) (results : NRow)
    (function : @NTy.carrier (Skolems.instantiate θ Θ).toCarriers (.function σs shared results))
    (args : @HList (Skolems.instantiate θ Θ).toCarriers σs) (state : Memory unit) :
    @RequiresOf unit (Skolems.instantiate θ Θ) table σs shared results function args state =
      @RequiresOf unit Θ table (NRow.subst θ.1 σs) shared (NRow.subst θ.1 results) function
        (HList.ofSkolem θ σs args) state := by
  dsimp only [RequiresOf]
  rw [@HList.encode_ofSkolem Θ.toCarriers θ σs args]
  rfl


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
variable {invoke : (Θ : Skolems unit) → @InvokeMeaning unit Θ}

/-- The settled runs' encodings are the same at the induced frame as at the
outer frame on the arguments read in its view. -/
theorem settledEncoding_instantiate (coherent : InvokeMeaning.Coherent invoke) (Θ : Skolems unit)
    (θ : TypeArgs) (free : θ.1.refFree = true) (σs : NRow) (shared : Sharing)
    (shape : ResultShape) (function : @NTy.carrier (Skolems.instantiate θ Θ).toCarriers (.function σs shared shape.row))
    (args : @HList (Skolems.instantiate θ Θ).toCarriers σs) (state : Memory unit) :
    @SettledEncoding unit (Skolems.instantiate θ Θ) (invoke (Skolems.instantiate θ Θ)) σs shared
        shape function args state =
      @SettledEncoding unit Θ (invoke Θ) (NRow.subst θ.1 σs) shared (shape.subst θ.1)
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
    @StateOf unit (Skolems.instantiate θ Θ) (invoke (Skolems.instantiate θ Θ)) σs shared shape
        function args state =
      @StateOf unit Θ (invoke Θ) (NRow.subst θ.1 σs) shared (shape.subst θ.1)
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
    @ResultOf unit (Skolems.instantiate θ Θ) (invoke (Skolems.instantiate θ Θ)) σs shared shape
        function args state default =
      ResultShape.toSkolem θ shape (@ResultOf unit Θ (invoke Θ) (NRow.subst θ.1 σs) shared
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
  · exact (@NTy.toSkolem_ofSkolem_shape unit Θ θ shape default).symm

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
    @EnsuresOf unit (Skolems.instantiate θ Θ) (@closureMeaning unit executable (Skolems.instantiate θ Θ))
        σs shared shape function args result state post =
      @EnsuresOf unit Θ (@closureMeaning unit executable Θ) (NRow.subst θ.1 σs) shared
        (shape.subst θ.1) (shape.functionOfSkolem θ σs shared function) (HList.ofSkolem θ σs args)
        (ResultShape.ofSkolem θ shape result) state post :=
  ensuresOf_instantiate (closureMeaning_coherent executable) Θ θ free σs shared shape function args
    result state post

theorem closureMeaning_abortsOf_instantiate (free : θ.1.refFree = true) :
    @AbortsOf unit (Skolems.instantiate θ Θ) (@closureMeaning unit executable (Skolems.instantiate θ Θ))
        σs shared shape function args state =
      @AbortsOf unit Θ (@closureMeaning unit executable Θ) (NRow.subst θ.1 σs) shared
        (shape.subst θ.1) (shape.functionOfSkolem θ σs shared function) (HList.ofSkolem θ σs args)
        state :=
  abortsOf_instantiate (closureMeaning_coherent executable) Θ θ free σs shared shape function args
    state

theorem closureMeaning_resultOf_instantiate (free : θ.1.refFree = true)
    (default : @ResultShape.carrier (Skolems.instantiate θ Θ).toCarriers shape) :
    @ResultOf unit (Skolems.instantiate θ Θ) (@closureMeaning unit executable (Skolems.instantiate θ Θ))
        σs shared shape function args state default =
      ResultShape.toSkolem θ shape (@ResultOf unit Θ (@closureMeaning unit executable Θ)
        (NRow.subst θ.1 σs) shared (shape.subst θ.1) (shape.functionOfSkolem θ σs shared function)
        (HList.ofSkolem θ σs args) state (ResultShape.ofSkolem θ shape default)) :=
  resultOf_instantiate (closureMeaning_coherent executable) Θ θ free σs shared shape function args
    state default

theorem closureMeaning_stateOf_instantiate (free : θ.1.refFree = true) :
    @StateOf unit (Skolems.instantiate θ Θ) (@closureMeaning unit executable (Skolems.instantiate θ Θ))
        σs shared shape function args state =
      @StateOf unit Θ (@closureMeaning unit executable Θ) (NRow.subst θ.1 σs) shared
        (shape.subst θ.1) (shape.functionOfSkolem θ σs shared function) (HList.ofSkolem θ σs args)
        state :=
  stateOf_instantiate (closureMeaning_coherent executable) Θ θ free σs shared shape function args
    state

theorem ModelUnit.ensuresOf_instantiate (free : θ.1.refFree = true)
    (result : @ResultShape.carrier (Skolems.instantiate θ Θ).toCarriers shape) (post : Memory unit) :
    @EnsuresOf unit (Skolems.instantiate θ Θ) (model.invoke (Skolems.instantiate θ Θ))
        σs shared shape function args result state post =
      @EnsuresOf unit Θ (model.invoke Θ) (NRow.subst θ.1 σs) shared (shape.subst θ.1)
        (shape.functionOfSkolem θ σs shared function) (HList.ofSkolem θ σs args)
        (ResultShape.ofSkolem θ shape result) state post :=
  VerexIR.Proofs.ensuresOf_instantiate model.coherent Θ θ free σs shared shape function args result
    state post

theorem ModelUnit.abortsOf_instantiate (free : θ.1.refFree = true) :
    @AbortsOf unit (Skolems.instantiate θ Θ) (model.invoke (Skolems.instantiate θ Θ))
        σs shared shape function args state =
      @AbortsOf unit Θ (model.invoke Θ) (NRow.subst θ.1 σs) shared (shape.subst θ.1)
        (shape.functionOfSkolem θ σs shared function) (HList.ofSkolem θ σs args) state :=
  VerexIR.Proofs.abortsOf_instantiate model.coherent Θ θ free σs shared shape function args state

theorem ModelUnit.resultOf_instantiate (free : θ.1.refFree = true)
    (default : @ResultShape.carrier (Skolems.instantiate θ Θ).toCarriers shape) :
    @ResultOf unit (Skolems.instantiate θ Θ) (model.invoke (Skolems.instantiate θ Θ))
        σs shared shape function args state default =
      ResultShape.toSkolem θ shape (@ResultOf unit Θ (model.invoke Θ) (NRow.subst θ.1 σs) shared
        (shape.subst θ.1) (shape.functionOfSkolem θ σs shared function) (HList.ofSkolem θ σs args)
        state (ResultShape.ofSkolem θ shape default)) :=
  VerexIR.Proofs.resultOf_instantiate model.coherent Θ θ free σs shared shape function args state
    default

theorem ModelUnit.stateOf_instantiate (free : θ.1.refFree = true) :
    @StateOf unit (Skolems.instantiate θ Θ) (model.invoke (Skolems.instantiate θ Θ))
        σs shared shape function args state =
      @StateOf unit Θ (model.invoke Θ) (NRow.subst θ.1 σs) shared (shape.subst θ.1)
        (shape.functionOfSkolem θ σs shared function) (HList.ofSkolem θ σs args) state :=
  VerexIR.Proofs.stateOf_instantiate model.coherent Θ θ free σs shared shape function args state

end Instances

end Induced

-- The solver reads behavioral predicates at concrete rows and results.
attribute [vir_denote_norm] Denote.NTy.refFree Denote.NRow.refFree Denote.NRows.refFree
attribute [vir_denote_norm] Denote.ResultShape.Settled Denote.NTy.settle Denote.HList.settle

end VerexIR.Proofs

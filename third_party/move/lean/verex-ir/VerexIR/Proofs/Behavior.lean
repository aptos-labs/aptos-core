-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Meaning
import VerexIR.Proofs.Denote.Closures

/-!
# Behavioral predicates

The behavior of a function value as a specification states it
(`doc/designs/higher-order-functions.md`, "Behavioral predicates"):
`aborts_of`, `ensures_of`, `result_of`, and `requires_of`, over an
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
  | .one (.sliceRef _), value => value.currentVector = value.finalVector
  | _, _ => True

/-- A value with a mutable reference's prophecy replaced by its entry value:
an argument as `aborts_of` and `result_of` read it, which no final value
names. -/
def Denote.NTy.settle : (τ : NTy) → τ.carrier → τ.carrier
  | .ref _, value => (value.1, value.1)
  | .sliceRef _, value => SpecSliceRef.ofArrays value.current value.current
  | .unit, value => value
  | .bool, value => value
  | .int _ _, value => value
  | .num, value => value
  | .address, value => value
  | .signer, value => value
  | .string, value => value
  | .bytes, value => value
  | .char, value => value
  | .tuple _, value => value
  | .struct _ _ _, value => value
  | .enum _ _ _ _ _, value => value
  | .vector _, value => value
  | .slice _, value => value
  | .array _ _, value => value
  | .table _ _, value => value
  | .ghost _, value => value
  | .param _, value => value
  | .function _ _ _, value => value
  | .nominal _ _ _, value => value
  -- A substitution's marked entry stands for a reference-free type, as a
  -- parameter does: settling leaves its value.
  | .observed _, value => value

/-- A row of arguments, each settled. -/
def Denote.HList.settle : (row : NRow) → HList row → HList row
  | .nil, _ => ()
  | .cons τ rest, values => (τ.settle values.1, HList.settle rest values.2)

/-- The referents of a row's mutable references, in the row's order: the
row of final values a clause names of them (`ensures_of`). -/
def Denote.NRow.finals : NRow → NRow
  | .nil => .nil
  | .cons (.ref σ) rest => .cons σ (NRow.finals rest)
  -- A mutable slice reference's final value is its contents at the loan's
  -- death, read as a slice: the operand `ensures_of` names of it
  -- (`doc/designs/higher-order-functions.md`, tuple-returning callbacks).
  | .cons (.sliceRef element) rest => .cons (.slice element) (NRow.finals rest)
  | .cons .unit rest | .cons .bool rest | .cons (.int _ _) rest | .cons .num rest
  | .cons .address rest | .cons .signer rest | .cons .string rest | .cons .bytes rest
  | .cons .char rest | .cons (.tuple _) rest | .cons (.struct _ _ _) rest
  | .cons (.enum _ _ _ _ _) rest | .cons (.vector _) rest | .cons (.slice _) rest
  | .cons (.array _ _) rest | .cons (.table _ _) rest | .cons (.ghost _) rest
  | .cons (.param _) rest | .cons (.function _ _ _) rest | .cons (.nominal _ _ _) rest
  | .cons (.observed _) rest => NRow.finals rest

/-- The prophecies a row of arguments holds, in the order of its mutable
references: the final values a clause names of them, which settling replaces
with entry values (`HList.settle`).  Every type of the row is named, as
`NTy.settle` names them, so that the row of finals reduces with the row. -/
def Denote.HList.finals : (row : NRow) → HList row → HList (NRow.finals row)
  | .nil, _ => ()
  | .cons (.ref _) rest, values => (values.1.2, HList.finals rest values.2)
  | .cons (.sliceRef _) rest, values => (values.1.finalVector, HList.finals rest values.2)
  | .cons .unit rest, values => HList.finals rest values.2
  | .cons .char rest, values => HList.finals rest values.2
  | .cons (.slice _) rest, values => HList.finals rest values.2
  | .cons (.array _ _) rest, values => HList.finals rest values.2
  | .cons (.nominal _ _ _) rest, values => HList.finals rest values.2
  | .cons (.observed _) rest, values => HList.finals rest values.2
  | .cons .bool rest, values => HList.finals rest values.2
  | .cons (.int _ _) rest, values => HList.finals rest values.2
  | .cons .num rest, values => HList.finals rest values.2
  | .cons .address rest, values => HList.finals rest values.2
  | .cons .signer rest, values => HList.finals rest values.2
  | .cons .string rest, values => HList.finals rest values.2
  | .cons .bytes rest, values => HList.finals rest values.2
  | .cons (.tuple _) rest, values => HList.finals rest values.2
  | .cons (.struct _ _ _) rest, values => HList.finals rest values.2
  | .cons (.enum _ _ _ _ _) rest, values => HList.finals rest values.2
  | .cons (.vector _) rest, values => HList.finals rest values.2
  | .cons (.table _ _) rest, values => HList.finals rest values.2
  | .cons (.ghost _) rest, values => HList.finals rest values.2
  | .cons (.param _) rest, values => HList.finals rest values.2
  | .cons (.function _ _ _) rest, values => HList.finals rest values.2

/-- A property of an arguments row as a clause states it of their entry
values: it holds of some row that passes them.  A clause names no prophecy
of a mutable reference, and of two rows of the same entry values an
invocation aborts alike (`not_abortsOf_of_ok`), so `aborts_of` is stated
this way — of an invocation (`AbortsOfEntries`) and of a known target read
directly, whose call or whose contract takes the row itself.

The row is one of valid values (`HList.Holds`): an invocation passes values
that hold their data invariants, a mutable reference's prophecy among them,
and the row a clause quantifies ranges over those and no others.  The entry
value of a reference is its current, whose validity is the hand-over's and
is not stated here. -/
def Denote.AtEntries (σs : NRow) (holds : HList σs → Prop) (entries : HList σs) : Prop :=
  ∃ args, HList.Holds σs args ∧ HList.settle σs args = entries ∧ holds args

/-- A property of a valid arguments row holds of their entry values. -/
theorem Denote.atEntries_of_holds {σs : NRow} {holds : HList σs → Prop} {args : HList σs}
    (valid : HList.Holds σs args) (held : holds args) :
    AtEntries σs holds (HList.settle σs args) :=
  ⟨args, valid, rfl, held⟩

/-- A property of an arguments row as a clause states it of their entry
values and of the finals of its mutable references: it holds of some row of
valid values that passes both (`AtEntries`).  Settling replaces the
prophecies with entry values, so the finals a clause names of them pin them
(`HList.finals`); a final value a clause names is therefore a valid one. -/
def Denote.AtEntryFinals (σs : NRow) (holds : HList σs → Prop) (entries : HList σs)
    (finals : HList (NRow.finals σs)) : Prop :=
  ∃ args, HList.Holds σs args ∧ HList.settle σs args = entries ∧
    HList.finals σs args = finals ∧ holds args

/-- A property of a valid arguments row holds of their entry values and of
the finals of its mutable references. -/
theorem Denote.atEntryFinals_of_holds {σs : NRow} {holds : HList σs → Prop} {args : HList σs}
    (valid : HList.Holds σs args) (held : holds args) :
    AtEntryFinals σs holds (HList.settle σs args) (HList.finals σs args) :=
  ⟨args, valid, rfl, rfl, held⟩

/-- The two components a settled row's equation gives: the settled head value
and the settled tail. -/
theorem Denote.HList.settle_cons_inj {τ : NTy} {rest : NRow} {left right : HList (.cons τ rest)}
    (settled : HList.settle (.cons τ rest) left = HList.settle (.cons τ rest) right) :
    NTy.settle τ left.1 = NTy.settle τ right.1 ∧
      HList.settle rest left.2 = HList.settle rest right.2 := by
  have pair : ((NTy.settle τ left.1, HList.settle rest left.2) : HList (.cons τ rest)) =
      (NTy.settle τ right.1, HList.settle rest right.2) := settled
  rw [Prod.mk.injEq] at pair
  exact pair

/-- A row of arguments is determined by its entry values and the finals of
its mutable references: settling keeps every value a clause can name, and
the one value it drops — the prophecy of a top-level mutable reference, and
of a mutable slice reference its final reading — is exactly what
`HList.finals` returns. -/
theorem Denote.HList.settle_finals_injective : (row : NRow) → ∀ {left right : HList row},
    HList.settle row left = HList.settle row right →
    HList.finals row left = HList.finals row right → left = right
  | .nil, _, _, _, _ => rfl
  | .cons (.ref σ) rest, left, right, settled, finals => by
      have currents : ((left.1.1, left.1.1) : (NTy.ref σ).carrier) = (right.1.1, right.1.1) :=
        (HList.settle_cons_inj settled).1
      rw [Prod.mk.injEq] at currents
      have parts :
          ((left.1.2, HList.finals rest left.2) :
            HList (NRow.finals (.cons (.ref σ) rest))) =
            (right.1.2, HList.finals rest right.2) := finals
      rw [Prod.mk.injEq] at parts
      exact Prod.ext (Prod.ext currents.1 parts.1)
        (HList.settle_finals_injective rest (HList.settle_cons_inj settled).2 parts.2)
  | .cons (.sliceRef element) rest, left, right, settled, finals => by
      have same : SpecSliceRef.ofArrays left.1.current left.1.current =
          SpecSliceRef.ofArrays right.1.current right.1.current :=
        (HList.settle_cons_inj settled).1
      have current : (SpecSliceRef.ofArrays left.1.current left.1.current).currentVector =
          (SpecSliceRef.ofArrays right.1.current right.1.current).currentVector :=
        congrArg SpecSliceRef.currentVector same
      have parts :
          ((left.1.finalVector, HList.finals rest left.2) :
            HList (NRow.finals (.cons (.sliceRef element) rest))) =
            (right.1.finalVector, HList.finals rest right.2) := finals
      rw [Prod.mk.injEq] at parts
      exact Prod.ext (SpecSliceRef.ext current parts.1)
        (HList.settle_finals_injective rest (HList.settle_cons_inj settled).2 parts.2)
  | .cons .unit rest, _, _, settled, finals
  | .cons .bool rest, _, _, settled, finals
  | .cons (.int _ _) rest, _, _, settled, finals
  | .cons .num rest, _, _, settled, finals
  | .cons .address rest, _, _, settled, finals
  | .cons .signer rest, _, _, settled, finals
  | .cons .string rest, _, _, settled, finals
  | .cons .bytes rest, _, _, settled, finals
  | .cons .char rest, _, _, settled, finals
  | .cons (.tuple _) rest, _, _, settled, finals
  | .cons (.struct _ _ _) rest, _, _, settled, finals
  | .cons (.enum _ _ _ _ _) rest, _, _, settled, finals
  | .cons (.vector _) rest, _, _, settled, finals
  | .cons (.slice _) rest, _, _, settled, finals
  | .cons (.array _ _) rest, _, _, settled, finals
  | .cons (.table _ _) rest, _, _, settled, finals
  | .cons (.ghost _) rest, _, _, settled, finals
  | .cons (.param _) rest, _, _, settled, finals
  | .cons (.function _ _ _) rest, _, _, settled, finals
  | .cons (.nominal _ _ _) rest, _, _, settled, finals
  | .cons (.observed _) rest, _, _, settled, finals =>
      Prod.ext (HList.settle_cons_inj settled).1
        (HList.settle_finals_injective rest (HList.settle_cons_inj settled).2 finals)

/-- The row a statement over entry values and finals quantifies is the row
whose settling and finals they are: where a row of those entry values and
finals is known, the statement's property and validity are that row's
(`HList.settle_finals_injective`). -/
theorem Denote.AtEntryFinals.at_row {σs : NRow} {holds : HList σs → Prop} {args : HList σs}
    (stated : AtEntryFinals σs holds (HList.settle σs args) (HList.finals σs args)) :
    HList.Holds σs args ∧ holds args := by
  obtain ⟨row, valid, settled, finals, held⟩ := stated
  obtain rfl := Denote.HList.settle_finals_injective σs settled finals
  exact ⟨valid, held⟩

end

section
variable {unit : ValidatedUnit} [FunctionCarriers unit] [Skolems unit]

/-- `ensures_of<f>(x, r)`: the invocation of `function` on `args` runs from
`pre` to `post`, returning `result`. -/
def EnsuresOf (invoke : InvokeMeaning) (σs : NRow) (shared : Sharing) (shape : ResultShape)
    (function : (NTy.function σs shared shape.row).carrier) (args : HList σs) (result : shape.carrier)
    (pre post : Memory unit) : Prop :=
  (invoke σs shared shape function args).ok pre result post

/-- `ensures_of<f>(x, r, f̄)` as a clause states it: the invocation runs
from `pre` to `post` returning `result` on valid arguments of the entry
values `entries` whose mutable references end at the final values `finals`.

The predicate observes a run and ranges over plain results: it states the
validity of the row it quantifies (`AtEntryFinals`), from which the finals'
validity follows as the witness's own (`AtEntryFinals.at_row`), and nothing
of `result`.  The result's fact is the function value's
(`ValidResultsAt`, a target's `Contract.validResult`); the run `result_of`
reads and a call need that evidence and do not take it from here. -/
def EnsuresOfEntries (invoke : InvokeMeaning) (σs : NRow) (shared : Sharing) (shape : ResultShape)
    (function : (NTy.function σs shared shape.row).carrier) (entries : HList σs)
    (finals : HList (NRow.finals σs)) (result : shape.carrier)
    (pre post : Memory unit) : Prop :=
  AtEntryFinals σs
    (fun args => EnsuresOf invoke σs shared shape function args result pre post) entries finals

/-- A run of an invocation on valid arguments is a run on the entry values
of its arguments and the finals of its mutable references. -/
theorem ensuresOfEntries_of_ensuresOf {invoke : InvokeMeaning} {σs : NRow} {shared : Sharing}
    {shape : ResultShape} {function : (NTy.function σs shared shape.row).carrier}
    {args : HList σs} {result : shape.carrier} {pre post : Memory unit}
    (valid : HList.Holds σs args)
    (ensures : EnsuresOf invoke σs shared shape function args result pre post) :
    EnsuresOfEntries invoke σs shared shape function (HList.settle σs args)
      (HList.finals σs args) result pre post :=
  Denote.atEntryFinals_of_holds valid ensures

/-- `ensures_of<f>(x, r, f̄)` where `f` returns a mutable reference: the
run returns a settled reference (`ResultShape.Settled`) whose current value
is `current`, the one the clause names.  A clause sees a returned reference
as its current value and holds no prophecy of it, as it holds none of a
mutable argument, so the prophecy the run returns is quantified here and
identified with `current` (`EnsuresOfEntries`).  The prophecy is a valid
value (`NTy.Holds`), as every prophecy is by its choice; its equation with
`current` states which value it is and not that it holds. -/
def EnsuresOfSettled (invoke : InvokeMeaning) (σs : NRow) (shared : Sharing) (σ : NTy)
    (function : (NTy.function σs shared (ResultShape.one (.ref σ)).row).carrier)
    (entries : HList σs) (finals : HList (NRow.finals σs)) (current : σ.carrier)
    (pre post : Memory unit) : Prop :=
  ∃ prophecy : σ.carrier, NTy.Holds σ prophecy ∧ prophecy = current ∧
    EnsuresOfEntries invoke σs shared (.one (.ref σ)) function entries finals
      (prophecy, prophecy) pre post

/-- A run returning a settled reference of a valid prophecy is an
`EnsuresOfSettled` run at that prophecy. -/
theorem ensuresOfSettled_of_ensuresOfEntries {invoke : InvokeMeaning} {σs : NRow}
    {shared : Sharing} {σ : NTy}
    {function : (NTy.function σs shared (ResultShape.one (.ref σ)).row).carrier}
    {entries : HList σs} {finals : HList (NRow.finals σs)} {prophecy : σ.carrier}
    {pre post : Memory unit}
    (valid : NTy.Holds σ prophecy)
    (ensures : EnsuresOfEntries invoke σs shared (.one (.ref σ)) function entries finals
      (prophecy, prophecy) pre post) :
    EnsuresOfSettled invoke σs shared σ function entries finals prophecy
      pre post :=
  ⟨prophecy, valid, rfl, ensures⟩

/-- `aborts_of<f>(x)`: the invocation of `function` on `args` aborts from
`state`. -/
def AbortsOf (invoke : InvokeMeaning) (σs : NRow) (shared : Sharing) (shape : ResultShape)
    (function : (NTy.function σs shared shape.row).carrier) (args : HList σs) (state : Memory unit) : Prop :=
  ∃ error, (invoke σs shared shape function args).aborts state error

/-- `aborts_of<f>(x)` as a clause states it: the invocation aborts on valid
arguments of the entry values of `x` (`AtEntries`). -/
def AbortsOfEntries (invoke : InvokeMeaning) (σs : NRow) (shared : Sharing) (shape : ResultShape)
    (function : (NTy.function σs shared shape.row).carrier) (entries : HList σs)
    (state : Memory unit) : Prop :=
  AtEntries σs (fun args => AbortsOf invoke σs shared shape function args state) entries

/-- An abort of an invocation on valid arguments is an abort on the entry
values of its arguments. -/
theorem abortsOfEntries_of_abortsOf {invoke : InvokeMeaning} {σs : NRow} {shared : Sharing}
    {shape : ResultShape} {function : (NTy.function σs shared shape.row).carrier} {args : HList σs}
    {state : Memory unit} (valid : HList.Holds σs args)
    (aborts : AbortsOf invoke σs shared shape function args state) :
    AbortsOfEntries invoke σs shared shape function (HList.settle σs args) state :=
  Denote.atEntries_of_holds valid aborts

/-- A run `result_of` and `state_of` read: from `state`, on valid arguments
of the entry values `entries`, returning a valid, settled result.  A row of
entry values is settled, as the clause layer builds it
(`Invocation.entryArguments`): no run's settled arguments meet an unsettled
row, and the labels then read their defaults.  A run on arguments or to a
result that does not hold its data invariants is no candidate, so the result
`result_of` reads of a run holds its own (`holds_resultOf`). -/
def SettledRun (invoke : InvokeMeaning) (σs : NRow) (shared : Sharing) (shape : ResultShape)
    (function : (NTy.function σs shared shape.row).carrier) (entries : HList σs)
    (state : Memory unit) (run : HList σs × shape.carrier × Memory unit) : Prop :=
  HList.Holds σs run.1 ∧ shape.Holds run.2.1 ∧
    HList.settle σs run.1 = entries ∧ shape.Settled run.2.1 ∧
    EnsuresOf invoke σs shared shape function run.1 run.2.1 state run.2.2

/-- A run's encodings: its arguments', its result's, and its final memory,
which every view of a frame holds alike. -/
noncomputable def runEncoding (σs : NRow) (shape : ResultShape) (run : HList σs × shape.carrier × Memory unit) :
    List RuntimeValue × Array RuntimeValue × Memory unit :=
  (HList.encode run.1, (Denote.resultCodec shape).encode run.2.1, run.2.2)

/-- The encodings of the settled runs `result_of` and `state_of` read. -/
def SettledEncoding (invoke : InvokeMeaning) (σs : NRow) (shared : Sharing)
    (shape : ResultShape) (function : (NTy.function σs shared shape.row).carrier)
    (entries : HList σs)
    (state : Memory unit) (encoded : List RuntimeValue × Array RuntimeValue × Memory unit) : Prop :=
  ∃ run, SettledRun invoke σs shared shape function entries state run ∧
    runEncoding σs shape run = encoded

open Classical in
/-- `result_of<f>(x)`: the result of a settled run on the entry values
`entries` from `state`, chosen among the runs' encodings, so that every view
of a frame names the same run; `default` where there is none.  The run it
chooses is a candidate (`SettledRun`), on valid arguments and to a valid
result, so the result it reads of one holds its data invariants
(`holds_resultOf`); the default it falls back to is the value it was given,
of which no validity is stated. -/
noncomputable def ResultOf (invoke : InvokeMeaning) (σs : NRow) (shared : Sharing)
    (shape : ResultShape) (function : (NTy.function σs shared shape.row).carrier)
    (entries : HList σs)
    (state : Memory unit) (default : shape.carrier) : shape.carrier :=
  if returns : ∃ encoded, SettledEncoding invoke σs shared shape function entries state encoded then
    ((Denote.resultCodec shape).decode? returns.choose.2.1).getD default
  else default

open Classical in
/-- The post-state of the run `result_of` reads; the pre-state where there is
none, so that using the label does not assume that the invocation returns. -/
noncomputable def StateOf (invoke : InvokeMeaning) (σs : NRow) (shared : Sharing)
    (shape : ResultShape) (function : (NTy.function σs shared shape.row).carrier)
    (entries : HList σs) (state : Memory unit) : Memory unit :=
  if returns : ∃ encoded, SettledEncoding invoke σs shared shape function entries state encoded then
    returns.choose.2.2
  else state

/-- The run `result_of` and `state_of` read, where a settled run returns:
one whose encodings are the chosen ones. -/
theorem settledEncoding_choose {invoke : InvokeMeaning} {σs : NRow} {shared : Sharing}
    {shape : ResultShape} {function : (NTy.function σs shared shape.row).carrier}
    {entries : HList σs} {state : Memory unit}
    (returns : ∃ encoded, SettledEncoding invoke σs shared shape function entries state encoded)
    (default : shape.carrier) :
    ∃ run, SettledRun invoke σs shared shape function entries state run ∧
      ResultOf invoke σs shared shape function entries state default = run.2.1 ∧
      StateOf invoke σs shared shape function entries state = run.2.2 := by
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
    {shape : ResultShape} {function : (NTy.function σs shared shape.row).carrier}
    {entries : HList σs}
    {state : Memory unit} {run : HList σs × shape.carrier × Memory unit}
    (returns : SettledRun invoke σs shared shape function entries state run)
    (default : shape.carrier) :
    ∃ args', SettledRun invoke σs shared shape function entries state
      (args', ResultOf invoke σs shared shape function entries state default,
        StateOf invoke σs shared shape function entries state) := by
  obtain ⟨chosen, settled, result, final⟩ :=
    settledEncoding_choose ⟨runEncoding σs shape run, run, returns, rfl⟩ default
  refine ⟨chosen.1, ?_⟩
  rw [result, final]
  exact settled

/-- Where a settled run returns, `result_of` holds its data invariants: it
is the result of a candidate run (`SettledRun`).  Where none returns it is
the default it was given, of which nothing is stated. -/
theorem holds_resultOf {invoke : InvokeMeaning} {σs : NRow} {shared : Sharing}
    {shape : ResultShape} {function : (NTy.function σs shared shape.row).carrier}
    {entries : HList σs} {state : Memory unit}
    (returns : ∃ encoded, SettledEncoding invoke σs shared shape function entries state encoded)
    (default : shape.carrier) :
    shape.Holds (ResultOf invoke σs shared shape function entries state default) := by
  obtain ⟨run, settled, result, -⟩ := settledEncoding_choose returns default
  rw [result]
  exact settled.2.1

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
  rw [Θ.resolve_eq_type, NRow.resolved_eq_substWith, NRow.resolved_eq_substWith]
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

/-- `requires_of<f>(x)` as a clause states it: the declared precondition of
the function value holds at every valid arguments row whose entry values are
`entries`.  A clause names no prophecy of a mutable reference it passes, and
a precondition reads a reference's entry value, so it holds alike at two
rows of the same entry values; stated so, the predicate is provable where
the entry values are known and usable at a call, which an existential form
would not be.  The rows it ranges over are those an invocation passes: rows
of values that hold their data invariants (`HList.Holds`). -/
def RequiresOfEntries (requires : RequiresMeaning unit) (σs : NRow) (shared : Sharing)
    (results : NRow) (function : (NTy.function σs shared results).carrier)
    (entries : HList σs) (state : Memory unit) : Prop :=
  ∀ args : HList σs, HList.Holds σs args → HList.settle σs args = entries →
    RequiresOf requires σs shared results function args state

/-- The declared precondition at a valid arguments row, where it holds at
every valid row of the row's entry values: what a call reads of a clause's
`requires_of`. -/
theorem requiresOf_of_entries {requires : RequiresMeaning unit} {σs : NRow} {shared : Sharing}
    {results : NRow} {function : (NTy.function σs shared results).carrier} {args : HList σs}
    {state : Memory unit} (valid : HList.Holds σs args)
    (holds : RequiresOfEntries requires σs shared results function (HList.settle σs args) state) :
    RequiresOf requires σs shared results function args state :=
  holds args valid rfl

omit Θ in
/-- At the base frame, `requires_of` is the requires meaning of the value at
the arguments. -/
theorem requiresOf_runtime
    (requires : RequiresMeaning unit) (σs : NRow) (shared : Sharing)
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

/-- A mutable reference at the base frame pairs its current value with its
prophecy there (`Skolems.toRuntimeFree`). -/
@[vir_denote_norm] theorem Denote.Skolems.toRuntimeFree_ref {σ : NTy}
    (free : (NTy.ref σ).paramFree = true) (value : (NTy.ref σ).carrier) :
    Θ.toRuntimeFree (.ref σ) free value =
      (Θ.toRuntimeFree σ (by simpa only [NTy.paramFree] using free) value.1,
        Θ.toRuntimeFree σ (by simpa only [NTy.paramFree] using free) value.2) := by
  apply @NTy.encode_injective (Carriers.runtime unit) (.ref σ)
  rw [Θ.encode_toRuntimeFree]
  show RuntimeValue.tuple #[σ.encode value.1, σ.encode value.2] =
    RuntimeValue.tuple #[@NTy.encode (Carriers.runtime unit) σ _,
      @NTy.encode (Carriers.runtime unit) σ _]
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
theorem Skolems.construct_heq_runtime
    {parameters parameters' results results' captured captured' : NRow}
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

omit [Skolems unit] in
/-- `aborts_of<f<τ̄>>(x)` of a known target with type arguments, read directly: the
call of the target at the frame its type arguments induce aborts from `state`
(`closureMeaning_closureOfGeneric` reduces an invocation of the literal to it). -/
def CallAborts {α : Type} (call : Comp unit α) (state : Memory unit) : Prop :=
  ∃ error, call.aborts state error

omit [Skolems unit] in
/-- `ensures_of<f<τ̄>>(x)` of a known target with type arguments, read directly: the
call of the target at the frame its type arguments induce runs from `pre` to
`post`, returning `result`. -/
def CallEnsures {α : Type} (call : Comp unit α) (result : α) (pre post : Memory unit) : Prop :=
  call.ok pre result post

omit [Skolems unit] in
/-- Where a call is made, its continuations may assume its run where it returns
and its abort where it aborts. -/
theorem wp_call_runs {α : Type} {call : Comp unit α} {ensures : α → Memory unit → Prop}
    {aborts : Failure → Prop} {initial : Memory unit}
    (continued : wp call
      (fun result final => CallEnsures call result initial final → ensures result final)
      (fun error => CallAborts call initial → aborts error) initial) :
    wp call ensures aborts initial :=
  ⟨fun result final run => continued.1 result final run run,
    fun error failed => continued.2.1 error failed ⟨error, failed⟩, continued.2.2⟩

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
its result validity, what its contract ensures of the run, its frame, and
that no condition under which it must abort held. -/
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
    (permitted : contract.requires (weave.compose captures args) pre)
    (valid : contract.validArgs (weave.compose captures args)) :
    contract.validResult result ∧
      (¬contract.mayAbort (weave.compose captures args) pre →
          contract.ensures (weave.compose captures args) pre result post) ∧
        contract.frame (weave.compose captures args) pre result post ∧
        ¬contract.mustAbort (weave.compose captures args) pre :=
  (verified _ pre valid assumed permitted).1 result post
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
    (permitted : contract.requires (weave.compose captures args) state)
    (valid : contract.validArgs (weave.compose captures args)) :
    ∃ failure, contract.aborts (weave.compose captures args) state failure := by
  obtain ⟨failure, runs⟩ := abortsOf_closureOf weave capturedFree coherent captures args aborts closed
  exact ⟨failure, (verified _ state valid assumed permitted).2.1 failure runs⟩

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
structure ModelUnit (unit : ValidatedUnit) [FunctionCarriers unit]
    where
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
  apply RequiresMeaning.congr requires shared (NRow.resolved_instantiate θ Θ σs)
    (NRow.resolved_instantiate θ Θ results)
  · exact (cast_heq _ _).trans (cast_heq _ _).symm
  · exact HList.toRuntime_instantiate θ Θ σs args

section Settle

/-- Settling a value without a reference leaves it unchanged. -/
theorem Denote.NTy.settle_self_of_refFree [Carriers] (τ : NTy) (free : τ.refFree = true)
    (value : τ.carrier) : NTy.settle τ value = value := by
  cases τ <;> first | rfl | simp [NTy.refFree] at free

theorem Denote.NTy.settle_of_refFree [Carriers] (τ : NTy) (free : τ.refFree = true)
    (value : τ.carrier) : NTy.settle τ value = value :=
  NTy.settle_self_of_refFree τ free value

/-- Settling a row without a reference leaves its values unchanged. -/
theorem Denote.HList.settle_of_refFree [Carriers] : (row : NRow) → row.refFree = true →
    (values : HList row) → HList.settle row values = values
  | .nil, _, () => rfl
  | .cons τ rest, free, (head, tail) => by
      simp only [NRow.refFree, Bool.and_eq_true] at free
      exact Prod.ext (NTy.settle_self_of_refFree τ free.1 head)
        (HList.settle_of_refFree rest free.2 tail)

/-- Settling commutes with reading a value in the outer view, where the
type arguments hold no reference. -/
theorem Denote.NTy.settle_ofSkolem [Θ : Carriers] (θ : TypeArgs)
    (free : θ.1.refFree = true) :
    (τ : NTy) → (value : @NTy.carrier (Carriers.instantiate θ Θ) τ) →
    NTy.settle (τ.subst θ.1) (NTy.ofSkolem θ τ value) =
      NTy.ofSkolem θ τ (@NTy.settle (Carriers.instantiate θ Θ) τ value)
  | .param index, value => by
      rw [NTy.settle_self_of_refFree _
          ((NTy.refFree_unmark _).trans (NRow.getD_refFree θ.1 free index (.param index) rfl))]
      rfl
  | .sliceRef _, _ => by
      apply SpecSliceRef.ext <;> rfl
  | .ref _, _ | .unit, _ | .bool, _ | .int _ _, _ | .num, _ | .address, _ | .signer, _
  | .string, _ | .bytes, _ | .char, _ | .tuple _, _ | .struct _ _ _, _ | .enum _ _ _ _ _, _
  | .vector _, _ | .slice _, _ | .array _ _, _ | .table _ _, _ | .ghost _, _
  | .function _ _ _, _ | .nominal _ _ _, _ | .observed _, _ => rfl

theorem Denote.HList.settle_ofSkolem [Θ : Carriers] (θ : TypeArgs)
    (free : θ.1.refFree = true) :
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
theorem Denote.ResultShape.settled_one_of_refFree [Carriers] : (τ : NTy) → τ.refFree = true →
    (value : τ.carrier) → (ResultShape.one τ).Settled value
  | .ref _, free, _ | .sliceRef _, free, _ => by simp [NTy.refFree] at free
  | .unit, _, _ | .bool, _, _ | .int _ _, _, _ | .num, _, _ | .address, _, _ | .signer, _, _
  | .string, _, _ | .bytes, _, _ | .char, _, _ | .tuple _, _, _ | .struct _ _ _, _, _
  | .enum _ _ _ _ _, _, _ | .vector _, _, _ | .slice _, _, _ | .array _ _, _, _
  | .table _ _, _, _ | .ghost _, _, _ | .param _, _, _ | .function _ _ _, _, _
  | .nominal _ _ _, _, _ | .observed _, _, _ => trivial

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
        ((NTy.refFree_unmark _).trans (NRow.getD_refFree θ.1 free index (.param index) rfl))
        (ResultShape.ofSkolem θ (.one (.param index)) value)
      exact ⟨fun _ => trivial, fun _ => settled⟩
  | .one (.sliceRef τ), value => by
      change NTy.ofSkolem θ (.vector τ) value.currentVector =
          NTy.ofSkolem θ (.vector τ) value.finalVector ↔
        value.currentVector = value.finalVector
      constructor
      · intro same
        have := congrArg (NTy.toSkolem θ (.vector τ)) same
        rwa [NTy.toSkolem_ofSkolem, NTy.toSkolem_ofSkolem] at this
      · intro same; rw [same]
  | .one .unit, _ | .one .bool, _ | .one (.int _ _), _ | .one .num, _ | .one .address, _
  | .one .signer, _ | .one .string, _ | .one .bytes, _ | .one .char, _ | .one (.tuple _), _
  | .one (.struct _ _ _), _ | .one (.enum _ _ _ _ _), _ | .one (.vector _), _ | .one (.array _ _), _
  | .one (.slice _), _ | .one (.table _ _), _ | .one (.ghost _), _ | .one (.function _ _ _), _
  | .one (.nominal _ _ _), _ | .one (.observed _), _ => Iff.rfl

/-- A result holds its data invariants in either view alike. -/
theorem Denote.ResultShape.holds_ofSkolem_iff [Θ : Carriers] (θ : TypeArgs) :
    (shape : ResultShape) →
    (value : @ResultShape.carrier (Carriers.instantiate θ Θ) shape) →
    ((shape.subst θ.1).Holds (ResultShape.ofSkolem θ shape value) ↔
      @ResultShape.Holds (Carriers.instantiate θ Θ) shape value)
  | .none, _ => Iff.rfl
  | .one τ, value => NTy.holds_ofSkolem_iff θ τ value

/-- A result read at the induced frame holds its data invariants where it
does at the outer frame. -/
theorem Denote.ResultShape.holds_toSkolem_iff [Θ : Skolems unit] (θ : TypeArgs) :
    (shape : ResultShape) → (value : (shape.subst θ.1).carrier) →
    (@ResultShape.Holds (Skolems.instantiate θ Θ).toCarriers shape
        (ResultShape.toSkolem θ shape value) ↔
      (shape.subst θ.1).Holds value)
  | .none, _ => Iff.rfl
  | .one τ, value => NTy.holds_toSkolem_iff θ τ value

/-- A result encodes alike in either view. -/
theorem Denote.resultCodec_encode_ofSkolem [Θ : Carriers] (θ : TypeArgs) :
    (shape : ResultShape) →
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
outer frame on the entry values read in its view. -/
theorem settledEncoding_instantiate (coherent : InvokeMeaning.Coherent invoke) (Θ : Skolems unit)
    (θ : TypeArgs) (free : θ.1.refFree = true) (σs : NRow) (shared : Sharing)
    (shape : ResultShape) (function : @NTy.carrier (Skolems.instantiate θ Θ).toCarriers (.function σs shared shape.row))
    (entries : @HList (Skolems.instantiate θ Θ).toCarriers σs) (state : Memory unit) :
    @SettledEncoding unit _ (Skolems.instantiate θ Θ) (invoke (Skolems.instantiate θ Θ)) σs shared
        shape function entries state =
      @SettledEncoding unit _ Θ (invoke Θ) (NRow.subst θ.1 σs) shared (shape.subst θ.1)
        (shape.functionOfSkolem θ σs shared function) (HList.ofSkolem θ σs entries) state := by
  funext encoded
  apply propext
  constructor
  · rintro ⟨⟨values, result, final⟩, ⟨valid, validResult, settles, settled, ran⟩, rfl⟩
    refine ⟨(HList.ofSkolem θ σs values, ResultShape.ofSkolem θ shape result, final),
      ⟨?_, ?_, ?_, ?_, ?_⟩, ?_⟩
    · exact (HList.holds_ofSkolem_iff θ σs values).mpr valid
    · exact (ResultShape.holds_ofSkolem_iff θ shape result).mpr validResult
    · show HList.settle _ (HList.ofSkolem θ σs values) = HList.ofSkolem θ σs entries
      rw [HList.settle_ofSkolem θ free]
      exact congrArg (HList.ofSkolem θ σs) settles
    · exact (ResultShape.settled_ofSkolem θ free shape result).mpr settled
    · rw [← ensuresOf_instantiate coherent Θ θ free]; exact ran
    · show (HList.encode (HList.ofSkolem θ σs values),
          (resultCodec (shape.subst θ.1)).encode (ResultShape.ofSkolem θ shape result), final) = _
      rw [HList.encode_ofSkolem, resultCodec_encode_ofSkolem]
      rfl
  · rintro ⟨⟨values, result, final⟩, ⟨valid, validResult, settles, settled, ran⟩, rfl⟩
    refine ⟨(HList.toSkolem θ σs values, ResultShape.toSkolem θ shape result, final),
      ⟨?_, ?_, ?_, ?_, ?_⟩, ?_⟩
    · exact (HList.holds_toSkolem_iff θ σs values).mpr valid
    · exact (ResultShape.holds_toSkolem_iff θ shape result).mpr validResult
    · have := congrArg (HList.toSkolem θ σs) settles
      rw [← HList.ofSkolem_toSkolem θ σs values, HList.settle_ofSkolem θ free,
        HList.toSkolem_ofSkolem, HList.toSkolem_ofSkolem] at this
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
    (entries : @HList (Skolems.instantiate θ Θ).toCarriers σs) (state : Memory unit) :
    @StateOf unit _ (Skolems.instantiate θ Θ) (invoke (Skolems.instantiate θ Θ)) σs shared shape
        function entries state =
      @StateOf unit _ Θ (invoke Θ) (NRow.subst θ.1 σs) shared (shape.subst θ.1)
        (shape.functionOfSkolem θ σs shared function) (HList.ofSkolem θ σs entries) state := by
  unfold StateOf
  rw [settledEncoding_instantiate coherent Θ θ free]

/-- `result_of` at an induced frame is `result_of` at the outer frame, carried
back. -/
theorem resultOf_instantiate (coherent : InvokeMeaning.Coherent invoke) (Θ : Skolems unit)
    (θ : TypeArgs) (free : θ.1.refFree = true) (σs : NRow) (shared : Sharing)
    (shape : ResultShape) (function : @NTy.carrier (Skolems.instantiate θ Θ).toCarriers (.function σs shared shape.row))
    (entries : @HList (Skolems.instantiate θ Θ).toCarriers σs) (state : Memory unit)
    (default : @ResultShape.carrier (Skolems.instantiate θ Θ).toCarriers shape) :
    @ResultOf unit _ (Skolems.instantiate θ Θ) (invoke (Skolems.instantiate θ Θ)) σs shared shape
        function entries state default =
      ResultShape.toSkolem θ shape (@ResultOf unit _ Θ (invoke Θ) (NRow.subst θ.1 σs) shared
        (shape.subst θ.1) (shape.functionOfSkolem θ σs shared function)
        (HList.ofSkolem θ σs entries) state
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
  (args : @HList (Skolems.instantiate θ Θ).toCarriers σs)
  (entries : @HList (Skolems.instantiate θ Θ).toCarriers σs) (state : Memory unit)

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
        σs shared shape function entries state default =
      ResultShape.toSkolem θ shape (@ResultOf unit _ Θ (@closureMeaning unit _ executable Θ)
        (NRow.subst θ.1 σs) shared (shape.subst θ.1) (shape.functionOfSkolem θ σs shared function)
        (HList.ofSkolem θ σs entries) state (ResultShape.ofSkolem θ shape default)) :=
  resultOf_instantiate (closureMeaning_coherent executable) Θ θ free σs shared shape function entries
    state default

theorem closureMeaning_stateOf_instantiate (free : θ.1.refFree = true) :
    @StateOf unit _ (Skolems.instantiate θ Θ) (@closureMeaning unit _ executable (Skolems.instantiate θ Θ))
        σs shared shape function entries state =
      @StateOf unit _ Θ (@closureMeaning unit _ executable Θ) (NRow.subst θ.1 σs) shared
        (shape.subst θ.1) (shape.functionOfSkolem θ σs shared function) (HList.ofSkolem θ σs entries)
        state :=
  stateOf_instantiate (closureMeaning_coherent executable) Θ θ free σs shared shape function entries
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
        σs shared shape function entries state default =
      ResultShape.toSkolem θ shape (@ResultOf unit _ Θ (model.invoke Θ) (NRow.subst θ.1 σs) shared
        (shape.subst θ.1) (shape.functionOfSkolem θ σs shared function) (HList.ofSkolem θ σs entries)
        state (ResultShape.ofSkolem θ shape default)) :=
  VerexIR.Proofs.resultOf_instantiate model.coherent Θ θ free σs shared shape function entries state
    default

theorem ModelUnit.stateOf_instantiate (free : θ.1.refFree = true) :
    @StateOf unit _ (Skolems.instantiate θ Θ) (model.invoke (Skolems.instantiate θ Θ))
        σs shared shape function entries state =
      @StateOf unit _ Θ (model.invoke Θ) (NRow.subst θ.1 σs) shared (shape.subst θ.1)
        (shape.functionOfSkolem θ σs shared function) (HList.ofSkolem θ σs entries) state :=
  VerexIR.Proofs.stateOf_instantiate model.coherent Θ θ free σs shared shape function entries state

end Instances

end Induced

/-- What holds of every run of an action that ends at the state it started
from, where the action's weakest precondition holds at every state. A leaf
reads a run this way where its clause names one local for the invocation's
entry and exit state, as a clause over a memory the run leaves unchanged
does (`denotedRun`): the start is then the state the fact is quantified
over, so the precondition cannot be read at a start the leaf names
(`forall_ok_of_wp`), and the state the goal names is the one the rule
quantifies. -/
theorem forall_ok_self_of_wp {σ ε Result : Type} {action : Spec σ ε Result}
    {ensures : Result → σ → Prop}
    (established : ∀ initial, wp action ensures (fun _ => True) initial) :
    ∀ result state, action.ok state result state → ensures result state :=
  fun result state => (established state).1 result state

/-- What holds of every run of an action that returns nothing and ends at
the state it started from, where its weakest precondition holds at every
state (`forall_ok_self_of_wp`). -/
theorem forall_ok_unit_self_of_wp {σ ε : Type} {action : Spec σ ε Unit}
    {ensures : σ → Prop}
    (established : ∀ initial, wp action (fun _ final => ensures final) (fun _ => True) initial) :
    ∀ state, action.ok state () state → ensures state :=
  fun state => (established state).1 () state

-- The solver reads behavioral predicates at concrete rows and results.
-- `requires_of` of a clause keeps its head through the normal form: the
-- step at it introduces the arguments and the entry values they pass before
-- reading the precondition (`dispatchBehavior`), which an unfolding in the
-- normal form would reach first and leave quantified.
attribute [vir_denote_norm] Denote.NTy.refFree Denote.NRow.refFree Denote.NRows.refFree
attribute [vir_denote_norm] Denote.ResultShape.Settled Denote.NTy.settle Denote.HList.settle

/-- A generic closure's verified target applies at the induced frame, with
captured dictionaries woven into its ordinary arguments. -/
theorem ensuresOf_generic_verified {unit : ValidatedUnit} [FunctionCarriers unit]
    {executable : ExecutableUnit unit} [Θ : Skolems unit]
    (θ : TypeArgs) (free : θ.1.refFree = true) {full captured supplied : NRow}
    (weave : Weave full captured supplied) (capturedFree : captured.refFree = true)
    (shape : ResultShape) (handle : FunctionHandle) (outer : Array (TypeId × TypeId))
    (typeArgs : Array GenericArgument)
    (coherent : @Coherent unit _ (Skolems.instantiate θ Θ) handle
      (frameInstantiation unit handle outer typeArgs))
    (captures : HList (NRow.subst θ.1 captured)) (shared : Sharing)
    (typed : SiteTyped (NRow.subst θ.1 supplied) shared (shape.subst θ.1).row
      handle weave.mask (frameInstantiation unit handle outer typeArgs) (NRow.subst θ.1 captured))
    (args : HList (NRow.subst θ.1 supplied))
    {result : (shape.subst θ.1).carrier} {pre post : Memory unit}
    {contract : Contract (Memory unit) Failure
      (@HList (Skolems.instantiate θ Θ).toCarriers full)
      (@ResultShape.carrier (Skolems.instantiate θ Θ).toCarriers shape)}
    (verified : Satisfies
      (@propheticMeaning unit _ executable (Skolems.instantiate θ Θ)
        (frameInstantiation unit handle outer typeArgs) handle full shape) contract)
    (ensures : EnsuresOf (closureMeaning executable) (NRow.subst θ.1 supplied) shared
      (shape.subst θ.1) (functionOfGeneric unit outer typeArgs θ weave shape shared handle captures typed)
      args result pre post)
    (assumed : contract.assumes (HList.toSkolem θ full ((weave.subst θ.1).compose captures args)) pre)
    (permitted : contract.requires (HList.toSkolem θ full ((weave.subst θ.1).compose captures args)) pre)
    (valid : contract.validArgs (HList.toSkolem θ full ((weave.subst θ.1).compose captures args))) :
    contract.validResult (ResultShape.toSkolem θ shape result) ∧
      (¬contract.mayAbort (HList.toSkolem θ full ((weave.subst θ.1).compose captures args)) pre →
        contract.ensures (HList.toSkolem θ full ((weave.subst θ.1).compose captures args)) pre
          (ResultShape.toSkolem θ shape result) post) ∧
      contract.frame (HList.toSkolem θ full ((weave.subst θ.1).compose captures args)) pre
        (ResultShape.toSkolem θ shape result) post ∧
      ¬contract.mustAbort (HList.toSkolem θ full ((weave.subst θ.1).compose captures args)) pre := by
  unfold EnsuresOf at ensures
  rw [closureMeaning_closureOfGeneric executable θ free weave capturedFree shape handle outer typeArgs coherent] at ensures
  rcases ensures with ⟨value, middle, run, same, state⟩
  subst post
  subst result
  rw [NTy.toSkolem_ofSkolem_shape]
  exact (verified _ pre valid assumed permitted).1 value middle run

/-! ## A behavior at a row

A behavior of a *meaning at a row* (`AbortsOfMeaning`, `EnsuresOfMeaning`): what a clause
states of entry values, of the run a proof knows rather than of a function
value.  A function value an invocation meaning reads is the meaning at the
value's own row (`abortsOfEntries_eq_abortsOfMeaning`); a value whose target a proof
sees is the target's meaning at the target's full row, its captures entries of
that row (`abortsOfMeaning_woven`), so a closure at any mask and the target named
bare state one predicate
(`doc/designs/higher-order-functions.md`, "Behavioral predicates"). -/

section

/-- A pair is a pair of its components, at types read as equal. -/
theorem Denote.heq_pair {α β γ δ : Type} {left : α} {right : β} {left' : γ} {right' : δ}
    (heads : HEq left left') (tails : HEq right right') :
    HEq ((left, right) : α × β) ((left', right') : γ × δ) := by
  cases heads
  cases tails
  rfl

/-- A woven row names the final values the supplied arguments name: a capture
holds no reference, so it names none. -/
theorem Denote.Weave.finals_eq : {full captured supplied : NRow} →
    (weave : Weave full captured supplied) → captured.refFree = true →
    NRow.finals full = NRow.finals supplied
  | _, _, _, .nil, _ => rfl
  | _, _, _, .captured (τ := τ) rest, free => by
      simp only [NRow.refFree, Bool.and_eq_true] at free
      have inner := rest.finals_eq free.2
      cases τ <;> first
        | exact inner
        | simp [NTy.refFree] at free
  | _, _, _, .supplied (τ := τ) rest, free => by
      have inner := rest.finals_eq free
      cases τ <;> simp only [NRow.finals, inner]

end

section
variable {unit : ValidatedUnit} [FunctionCarriers unit] [Skolems unit]

@[simp, vir_denote, vir_denote_norm] theorem Denote.Weave.compose_nil
    (captures args : HList .nil) : Weave.nil.compose (unit := unit) captures args = () := rfl

@[simp, vir_denote, vir_denote_norm] theorem Denote.Weave.compose_captured {τ : NTy}
    {full captured supplied : NRow} (rest : Weave full captured supplied)
    (captures : HList (.cons τ captured)) (args : HList supplied) :
    (Weave.captured rest).compose (unit := unit) captures args =
      (captures.1, rest.compose captures.2 args) := rfl

@[simp, vir_denote, vir_denote_norm] theorem Denote.Weave.compose_supplied {τ : NTy}
    {full captured supplied : NRow} (rest : Weave full captured supplied)
    (captures : HList captured) (args : HList (.cons τ supplied)) :
    (Weave.supplied rest).compose (unit := unit) captures args =
      (args.1, rest.compose captures args.2) := rfl

/-- A woven row determines its captures and its arguments. -/
theorem Denote.Weave.compose_injective : {full captured supplied : NRow} →
    (weave : Weave full captured supplied) →
    {captures captures' : HList captured} → {args args' : HList supplied} →
    weave.compose (unit := unit) captures args = weave.compose (unit := unit) captures' args' →
      captures = captures' ∧ args = args'
  | _, _, _, .nil, (), (), (), (), _ => ⟨rfl, rfl⟩
  | _, _, _, .captured rest, (c, cs), (c', cs'), _, _, equal => by
      have heads : c = c' := congrArg Prod.fst equal
      obtain ⟨rests, argsEq⟩ := rest.compose_injective (congrArg Prod.snd equal)
      have rests : cs = cs' := rests
      exact ⟨by subst heads; subst rests; rfl, argsEq⟩
  | _, _, _, .supplied rest, _, _, (a, as), (a', as'), equal => by
      have heads : a = a' := congrArg Prod.fst equal
      obtain ⟨capturesEq, rests⟩ := rest.compose_injective (congrArg Prod.snd equal)
      have rests : as = as' := rests
      exact ⟨capturesEq, by subst heads; subst rests; rfl⟩

/-- Settling a woven row of reference-free captures is weaving the captures
with the settled arguments. -/
theorem Denote.Weave.settle_compose : {full captured supplied : NRow} →
    (weave : Weave full captured supplied) → captured.refFree = true →
    (captures : HList captured) → (args : HList supplied) →
    HList.settle full (weave.compose captures args) =
      weave.compose captures (HList.settle supplied args)
  | _, _, _, .nil, _, _, _ => rfl
  | _, _, _, .captured (τ := τ) rest, free, captures, args => by
      simp only [NRow.refFree, Bool.and_eq_true] at free
      simp only [Weave.compose, HList.settle,
        NTy.settle_self_of_refFree τ free.1, rest.settle_compose free.2]
  | _, _, _, .supplied rest, free, captures, args => by
      simp only [Weave.compose, HList.settle, rest.settle_compose free]

/-- The final values a woven row names are the supplied arguments': a capture
holds no reference (`Weave.finals_eq`). -/
theorem Denote.HList.heq_finals_compose : {full captured supplied : NRow} →
    (weave : Weave full captured supplied) → captured.refFree = true →
    (captures : HList captured) → (args : HList supplied) →
    HEq (HList.finals full (weave.compose captures args)) (HList.finals supplied args)
  | _, _, _, .nil, _, _, _ => HEq.rfl
  | _, _, _, .captured (τ := τ) rest, free, captures, args => by
      simp only [NRow.refFree, Bool.and_eq_true] at free
      have inner := HList.heq_finals_compose rest free.2 captures.2 args
      cases τ <;> first
        | exact inner
        | simp [NTy.refFree] at free
  | _, _, _, .supplied (τ := τ) rest, free, captures, args => by
      have inner := HList.heq_finals_compose rest free captures args.2
      cases τ <;> first
        | exact inner
        | exact Denote.heq_pair HEq.rfl inner

/-- The final values a clause names of a woven row are the supplied
arguments' (`HList.heq_finals_compose`). -/
theorem Denote.Weave.finals_compose_iff {full captured supplied : NRow}
    (weave : Weave full captured supplied) (free : captured.refFree = true)
    (captures : HList captured) (args : HList supplied)
    {finals : HList (NRow.finals supplied)} {finals' : HList (NRow.finals full)}
    (same : HEq finals' finals) :
    (HList.finals full (weave.compose captures args) = finals') ↔
      (HList.finals supplied args = finals) := by
  have woven := HList.heq_finals_compose weave free captures args
  constructor
  · intro held
    exact eq_of_heq ((woven.symm.trans (heq_of_eq held)).trans same)
  · intro held
    exact eq_of_heq ((woven.trans (heq_of_eq held)).trans same.symm)

/-- `aborts_of` of a meaning at a row, as a clause states it: the run on
valid arguments of the entry values `entries` aborts from `state`
(`AtEntries`).  Of an invocation
meaning at a function value (`abortsOfEntries_eq_abortsOfMeaning`) and of a known
target's meaning at its full row alike. -/
def AbortsOfMeaning (σs : NRow) (shape : ResultShape) (run : HList σs → Comp unit shape.carrier)
    (entries : HList σs) (state : Memory unit) : Prop :=
  AtEntries σs (fun args => CallAborts (run args) state) entries

/-- `ensures_of` of a meaning at a row, as a clause states it: the run on
valid arguments of the entry values `entries` runs from `pre` to `post`, its
mutable references ending at `finals` and returning `result`
(`AtEntryFinals`, `EnsuresOfEntries`).  As there, `result` is a plain value
of which the predicate states no validity. -/
def EnsuresOfMeaning (σs : NRow) (shape : ResultShape) (run : HList σs → Comp unit shape.carrier)
    (entries : HList σs) (finals : HList (NRow.finals σs)) (result : shape.carrier)
    (pre post : Memory unit) : Prop :=
  AtEntryFinals σs (fun args => CallEnsures (run args) result pre post) entries finals

/-- `aborts_of` of a function value is its invocation meaning at the value,
read at the value's own row. -/
theorem abortsOfEntries_eq_abortsOfMeaning (invoke : InvokeMeaning) (σs : NRow) (shared : Sharing)
    (shape : ResultShape) (function : (NTy.function σs shared shape.row).carrier)
    (entries : HList σs) (state : Memory unit) :
    AbortsOfEntries invoke σs shared shape function entries state =
      AbortsOfMeaning σs shape (invoke σs shared shape function) entries state := rfl

/-- `aborts_of` of a known target read directly is its call at the row. -/
theorem atEntries_callAborts_eq_abortsOfMeaning {σs : NRow} {shape : ResultShape}
    (run : HList σs → Comp unit shape.carrier) (entries : HList σs)
    (state : Memory unit) :
    AtEntries σs (fun args => CallAborts (run args) state) entries =
      AbortsOfMeaning σs shape run entries state := rfl

/-- `ensures_of` of a function value is its invocation meaning at the value. -/
theorem ensuresOfEntries_eq_ensuresOfMeaning (invoke : InvokeMeaning) (σs : NRow) (shared : Sharing)
    (shape : ResultShape) (function : (NTy.function σs shared shape.row).carrier)
    (entries : HList σs) (finals : HList (NRow.finals σs)) (result : shape.carrier)
    (pre post : Memory unit) :
    EnsuresOfEntries invoke σs shared shape function entries finals result pre post =
      EnsuresOfMeaning σs shape (invoke σs shared shape function) entries finals result pre post := rfl

/-- `ensures_of` of a known target read directly is its call at the row. -/
theorem atEntryFinals_callEnsures_eq_ensuresOfMeaning {σs : NRow} {shape : ResultShape}
    (run : HList σs → Comp unit shape.carrier) (entries : HList σs)
    (finals : HList (NRow.finals σs)) (result : shape.carrier) (pre post : Memory unit) :
    AtEntryFinals σs (fun args => CallEnsures (run args) result pre post) entries finals =
      EnsuresOfMeaning σs shape run entries finals result pre post := rfl

/-- `aborts_of` of a meaning at a row the captures of a weave fill in part,
at the woven entry values: a valid row of the supplied arguments is, woven
with valid captures, a valid row of the target's. -/
theorem abortsOfMeaning_weave {full captured supplied : NRow} (weave : Weave full captured supplied)
    (free : captured.refFree = true) {shape : ResultShape}
    (run : HList full → Comp unit shape.carrier) (captures : HList captured)
    (valid : HList.Holds captured captures) (entries : HList supplied) (state : Memory unit)
    (stated : AbortsOfMeaning supplied shape (fun args => run (weave.compose captures args))
      entries state) :
    AbortsOfMeaning full shape run (weave.compose captures entries) state := by
  obtain ⟨args, holds, settled, aborts⟩ := stated
  exact ⟨weave.compose captures args, (weave.holds_compose captures args).mpr ⟨valid, holds⟩,
    by rw [weave.settle_compose free, settled], aborts⟩

/-- `aborts_of` of a meaning at the woven entry values, read at the supplied
row: the supplied arguments of a valid woven row are valid
(`Weave.holds_split`), whatever the captures. -/
theorem abortsOfMeaning_unweave {full captured supplied : NRow}
    (weave : Weave full captured supplied) (free : captured.refFree = true) {shape : ResultShape}
    (run : HList full → Comp unit shape.carrier) (captures : HList captured)
    (entries : HList supplied) (state : Memory unit)
    (stated : AbortsOfMeaning full shape run (weave.compose captures entries) state) :
    AbortsOfMeaning supplied shape (fun args => run (weave.compose captures args)) entries
      state := by
  obtain ⟨woven, holds, settled, aborts⟩ := stated
  have split := weave.compose_split woven
  rw [← split, weave.settle_compose free] at settled
  obtain ⟨capturesEq, argsEq⟩ := weave.compose_injective settled
  refine ⟨(weave.split woven).2, ((weave.holds_split woven).mp holds).2, argsEq, ?_⟩
  show CallAborts (run (weave.compose captures (weave.split woven).2)) state
  rw [← capturesEq, split]
  exact aborts

/-- A behavior stated at woven entry values states that the captures hold
their data invariants: a capture holds no reference, so the valid row the
statement quantifies carries the capture itself.  The equations of a
closure's behavior at its target therefore hold of valid captures only
(`abortsOfMeaning_woven`). -/
theorem holds_captures_of_abortsOfMeaning {full captured supplied : NRow}
    (weave : Weave full captured supplied) (free : captured.refFree = true) {shape : ResultShape}
    (run : HList full → Comp unit shape.carrier) (captures : HList captured)
    (entries : HList supplied) (state : Memory unit)
    (stated : AbortsOfMeaning full shape run (weave.compose captures entries) state) :
    HList.Holds captured captures := by
  obtain ⟨woven, holds, settled, -⟩ := stated
  have split := weave.compose_split woven
  rw [← split, weave.settle_compose free] at settled
  obtain ⟨capturesEq, -⟩ := weave.compose_injective settled
  rw [← capturesEq]
  exact ((weave.holds_split woven).mp holds).1

/-- `aborts_of` of a meaning at a row, read at a row the captures of a weave
fill in part, is that meaning at the woven entry values: the equation of a
closure's behavior at its target, in the meaning and with no closure in it.
It holds of valid captures (`holds_captures_of_abortsOfMeaning`). -/
theorem abortsOfMeaning_woven {full captured supplied : NRow} (weave : Weave full captured supplied)
    (free : captured.refFree = true) {shape : ResultShape}
    (run : HList full → Comp unit shape.carrier) (captures : HList captured)
    (valid : HList.Holds captured captures) (entries : HList supplied) (state : Memory unit) :
    AbortsOfMeaning supplied shape (fun args => run (weave.compose captures args)) entries state =
      AbortsOfMeaning full shape run (weave.compose captures entries) state :=
  propext ⟨abortsOfMeaning_weave weave free run captures valid entries state,
    abortsOfMeaning_unweave weave free run captures entries state⟩

/-- `ensures_of` of a meaning at a row the captures of a weave fill in part,
at the woven entry values (`abortsOfMeaning_weave`); the finals are the
supplied arguments', a capture naming none (`Weave.finals_compose_iff`). -/
theorem ensuresOfMeaning_weave {full captured supplied : NRow} (weave : Weave full captured supplied)
    (free : captured.refFree = true) {shape : ResultShape}
    (run : HList full → Comp unit shape.carrier) (captures : HList captured)
    (valid : HList.Holds captured captures)
    (entries : HList supplied) {finals : HList (NRow.finals supplied)}
    {finals' : HList (NRow.finals full)} (same : HEq finals' finals)
    (result : shape.carrier) (pre post : Memory unit)
    (stated : EnsuresOfMeaning supplied shape (fun args => run (weave.compose captures args))
      entries finals result pre post) :
    EnsuresOfMeaning full shape run (weave.compose captures entries) finals' result pre post := by
  obtain ⟨args, holds, settled, finalsEq, ensures⟩ := stated
  refine ⟨weave.compose captures args, (weave.holds_compose captures args).mpr ⟨valid, holds⟩,
    by rw [weave.settle_compose free, settled], ?_, ensures⟩
  exact (weave.finals_compose_iff free captures args same).mpr finalsEq

/-- `ensures_of` of a meaning at the woven entry values, read at the
supplied row (`abortsOfMeaning_unweave`). -/
theorem ensuresOfMeaning_unweave {full captured supplied : NRow}
    (weave : Weave full captured supplied) (free : captured.refFree = true) {shape : ResultShape}
    (run : HList full → Comp unit shape.carrier) (captures : HList captured)
    (entries : HList supplied) {finals : HList (NRow.finals supplied)}
    {finals' : HList (NRow.finals full)} (same : HEq finals' finals)
    (result : shape.carrier) (pre post : Memory unit)
    (stated : EnsuresOfMeaning full shape run (weave.compose captures entries) finals' result
      pre post) :
    EnsuresOfMeaning supplied shape (fun args => run (weave.compose captures args)) entries finals
      result pre post := by
  obtain ⟨woven, holds, settled, finalsEq, ensures⟩ := stated
  have split := weave.compose_split woven
  rw [← split, weave.settle_compose free] at settled
  obtain ⟨capturesEq, argsEq⟩ := weave.compose_injective settled
  rw [← split, capturesEq] at finalsEq ensures
  refine ⟨(weave.split woven).2, ((weave.holds_split woven).mp holds).2, argsEq, ?_, ensures⟩
  exact (weave.finals_compose_iff free captures (weave.split woven).2 same).mp finalsEq

/-- `ensures_of` of a meaning at a row, at the woven entry values
(`abortsOfMeaning_woven`), of valid captures. -/
theorem ensuresOfMeaning_woven {full captured supplied : NRow} (weave : Weave full captured supplied)
    (free : captured.refFree = true) {shape : ResultShape}
    (run : HList full → Comp unit shape.carrier) (captures : HList captured)
    (valid : HList.Holds captured captures)
    (entries : HList supplied) {finals : HList (NRow.finals supplied)}
    {finals' : HList (NRow.finals full)} (same : HEq finals' finals)
    (result : shape.carrier) (pre post : Memory unit) :
    EnsuresOfMeaning supplied shape (fun args => run (weave.compose captures args)) entries finals
        result pre post =
      EnsuresOfMeaning full shape run (weave.compose captures entries)
        finals' result pre post :=
  propext ⟨ensuresOfMeaning_weave weave free run captures valid entries same result pre post,
    ensuresOfMeaning_unweave weave free run captures entries same result pre post⟩

/-- Invoking a function value whose target, weave, and captures a proof sees
is invoking the function value naming its target, at the woven arguments. -/
theorem closureMeaning_closureOf_named {executable : ExecutableUnit unit} {handle : FunctionHandle}
    {full captured supplied : NRow} (weave : Weave full captured supplied)
    (free : captured.refFree = true) (typeInstantiation : Array (TypeId × TypeId))
    (captures : HList captured) (shared : Sharing) (shape : ResultShape)
    (function : (NTy.function supplied shared shape.row).carrier)
    (closed : (Carriers.functions supplied shared shape.row).closure function =
      closureOf handle weave.mask typeInstantiation captures)
    {named_shared : Sharing} (named : (NTy.function full named_shared shape.row).carrier)
    (naming : (Carriers.functions full named_shared shape.row).closure named =
      closureOf handle (Weave.supplying full).mask typeInstantiation (σs := .nil) ())
    (args : HList supplied) :
    closureMeaning executable supplied shared shape function args =
      closureMeaning executable full named_shared shape named (weave.compose captures args) := by
  simp only [closureMeaning, closed, naming,
    closureRun_closureOf_woven executable handle weave free typeInstantiation captures shape args]

/-- `aborts_of` as a clause states it (`AbortsOfEntries`) of a function value
capturing valid values, as of the function value naming its target (`named`)
at the woven entry values. -/
theorem abortsOfEntries_closureOf_woven {executable : ExecutableUnit unit}
    {handle : FunctionHandle} {full captured supplied : NRow}
    (weave : Weave full captured supplied) (free : captured.refFree = true)
    (typeInstantiation : Array (TypeId × TypeId)) (captures : HList captured)
    (valid : HList.Holds captured captures)
    (shared : Sharing) (shape : ResultShape)
    (function : (NTy.function supplied shared shape.row).carrier)
    (closed : (Carriers.functions supplied shared shape.row).closure function =
      closureOf handle weave.mask typeInstantiation captures)
    {named_shared : Sharing} (named : (NTy.function full named_shared shape.row).carrier)
    (naming : (Carriers.functions full named_shared shape.row).closure named =
      closureOf handle (Weave.supplying full).mask typeInstantiation (σs := .nil) ())
    (entries : HList supplied) (state : Memory unit) :
    AbortsOfEntries (closureMeaning executable) supplied shared shape function entries state =
      AbortsOfEntries (closureMeaning executable) full named_shared shape named
        (weave.compose captures entries) state := by
  rw [abortsOfEntries_eq_abortsOfMeaning, abortsOfEntries_eq_abortsOfMeaning,
    show (closureMeaning executable supplied shared shape function) =
        fun args => closureMeaning executable full named_shared shape named
          (weave.compose captures args) from
      funext fun args => closureMeaning_closureOf_named weave free typeInstantiation captures
        shared shape function closed named naming args]
  exact abortsOfMeaning_woven weave free _ captures valid entries state

/-- `ensures_of` as a clause states it (`EnsuresOfEntries`) of a function
value capturing valid values, as of the function value naming its target at
the woven entry values; the two rows name the same final values
(`Weave.finals_compose_iff`). -/
theorem ensuresOfEntries_closureOf_woven {executable : ExecutableUnit unit}
    {handle : FunctionHandle} {full captured supplied : NRow}
    (weave : Weave full captured supplied) (free : captured.refFree = true)
    (typeInstantiation : Array (TypeId × TypeId)) (captures : HList captured)
    (valid : HList.Holds captured captures)
    (shared : Sharing) (shape : ResultShape)
    (function : (NTy.function supplied shared shape.row).carrier)
    (closed : (Carriers.functions supplied shared shape.row).closure function =
      closureOf handle weave.mask typeInstantiation captures)
    {named_shared : Sharing} (named : (NTy.function full named_shared shape.row).carrier)
    (naming : (Carriers.functions full named_shared shape.row).closure named =
      closureOf handle (Weave.supplying full).mask typeInstantiation (σs := .nil) ())
    (entries : HList supplied) {finals : HList (NRow.finals supplied)}
    {finals' : HList (NRow.finals full)} (same : HEq finals' finals)
    (result : shape.carrier) (pre post : Memory unit) :
    EnsuresOfEntries (closureMeaning executable) supplied shared shape function entries finals
        result pre post =
      EnsuresOfEntries (closureMeaning executable) full named_shared shape named
        (weave.compose captures entries) finals' result pre post := by
  rw [ensuresOfEntries_eq_ensuresOfMeaning, ensuresOfEntries_eq_ensuresOfMeaning,
    show (closureMeaning executable supplied shared shape function) =
        fun args => closureMeaning executable full named_shared shape named
          (weave.compose captures args) from
      funext fun args => closureMeaning_closureOf_named weave free typeInstantiation captures
        shared shape function closed named naming args]
  exact ensuresOfMeaning_woven weave free _ captures valid entries same result pre post

/-- `ensures_of` of a function value returning a mutable reference
(`EnsuresOfSettled`) and capturing valid values, as of the function value
naming its target at the woven entry values
(`ensuresOfEntries_closureOf_woven`). -/
theorem ensuresOfSettled_closureOf_woven {executable : ExecutableUnit unit}
    {handle : FunctionHandle} {full captured supplied : NRow}
    (weave : Weave full captured supplied) (free : captured.refFree = true)
    (typeInstantiation : Array (TypeId × TypeId)) (captures : HList captured)
    (valid : HList.Holds captured captures)
    (shared : Sharing) (σ : NTy)
    (function : (NTy.function supplied shared (ResultShape.one (.ref σ)).row).carrier)
    (closed : (Carriers.functions supplied shared (ResultShape.one (.ref σ)).row).closure
      function = closureOf handle weave.mask typeInstantiation captures)
    {named_shared : Sharing}
    (named : (NTy.function full named_shared (ResultShape.one (.ref σ)).row).carrier)
    (naming : (Carriers.functions full named_shared (ResultShape.one (.ref σ)).row).closure
      named = closureOf handle (Weave.supplying full).mask typeInstantiation (σs := .nil) ())
    (entries : HList supplied) {finals : HList (NRow.finals supplied)}
    {finals' : HList (NRow.finals full)} (same : HEq finals' finals)
    (current : σ.carrier) (pre post : Memory unit) :
    EnsuresOfSettled (closureMeaning executable) supplied shared σ function entries finals
        current pre post =
      EnsuresOfSettled (closureMeaning executable) full named_shared σ named
        (weave.compose captures entries) finals' current
        pre post := by
  unfold EnsuresOfSettled
  apply propext
  constructor
  · rintro ⟨prophecy, holds, settled, ensures⟩
    exact ⟨prophecy, holds, settled, (ensuresOfEntries_closureOf_woven weave free
      typeInstantiation captures valid shared _ function closed named naming entries same _ pre
      post) ▸ ensures⟩
  · rintro ⟨prophecy, holds, settled, ensures⟩
    exact ⟨prophecy, holds, settled, (ensuresOfEntries_closureOf_woven weave free
      typeInstantiation captures valid shared _ function closed named naming entries same _ pre
      post).symm ▸ ensures⟩

end

end VerexIR.Proofs

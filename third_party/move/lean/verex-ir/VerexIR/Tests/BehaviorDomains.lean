-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.ClosureTyping
open VerexIR VerexIR.Proofs VerexIR.Proofs.Denote

/-! # The domain of the rows a behavioral predicate quantifies

A behavioral predicate of a clause quantifies the arguments row an invocation
passes (`AtEntries`, `AtEntryFinals`), the prophecy a run returns
(`EnsuresOfSettled`) and the run `result_of` reads (`SettledRun`).  Each
ranges over values that hold their data invariants: at a mutable reference
the prophecy's, the current being the hand-over's matter.

The refutations are stated of a family's carriers alone, with a type
parameter whose predicate is the family's own (`Carriers.holds`), so they
assume no frame and nothing about which values a frame admits.  They say
that the predicate has no witness outside its domain; they do not say that
no raw run exists on such values.
-/

namespace VerexIR.Tests.BehaviorDomains

/-! ## Entry values and finals -/

section Entries
variable [Carriers]

/-- A row of one mutable reference to a value of a type parameter. -/
abbrev receiverRow : NRow := .cons (.ref (.param 0)) .nil

/-- A row of one value of a type parameter. -/
abbrev valueRow : NRow := .cons (.param 0) .nil

/-- Finals that pin a prophecy outside its type's predicate leave the
statement without a witness, whatever property it states of the row and
though the reference's current value holds. -/
theorem invalid_prophecy_refutes_atEntryFinals (current prophecy : (NTy.param 0).carrier)
    (_validCurrent : Carriers.holds 0 current) (invalid : ¬Carriers.holds 0 prophecy) :
    ¬AtEntryFinals receiverRow (fun _ => True) ((current, current), ()) (prophecy, ()) := by
  rintro ⟨args, valid, -, finals, -⟩
  have pinned : args.1.2 = prophecy := congrArg Prod.fst finals
  have held : NTy.Holds (.ref (.param 0)) args.1 := valid.1
  rw [NTy.holds_ref, NTy.holds_param, pinned] at held
  exact invalid held

/-- Finals that pin a valid prophecy have the witness they name; the
current value's validity is not asked. -/
theorem valid_prophecy_gives_atEntryFinals (current prophecy : (NTy.param 0).carrier)
    (valid : Carriers.holds 0 prophecy) :
    AtEntryFinals receiverRow (fun _ => True) ((current, current), ()) (prophecy, ()) := by
  refine ⟨((current, prophecy), ()), ⟨?_, trivial⟩, rfl, rfl, trivial⟩
  rw [NTy.holds_ref, NTy.holds_param]
  exact valid

/-- The witness of a statement over the entry values and finals of a row is
that row: the statement gives the row's validity and its property. -/
theorem atEntryFinals_gives_prophecy_fact (current prophecy : (NTy.param 0).carrier)
    {holds : HList receiverRow → Prop}
    (stated : AtEntryFinals receiverRow holds ((current, current), ()) (prophecy, ())) :
    Carriers.holds 0 prophecy ∧ holds ((current, prophecy), ()) := by
  have read := AtEntryFinals.at_row (σs := receiverRow) (args := ((current, prophecy), ())) stated
  have held : NTy.Holds (.ref (.param 0)) (current, prophecy) := read.1.1
  rw [NTy.holds_ref, NTy.holds_param] at held
  exact ⟨held, read.2⟩

/-- A statement over entry values has no witness at a value outside its
type's predicate: the row it quantifies passes that value. -/
theorem invalid_value_refutes_atEntries (value : (NTy.param 0).carrier)
    (invalid : ¬Carriers.holds 0 value) (holds : HList valueRow → Prop) :
    ¬AtEntries valueRow holds (value, ()) := by
  rintro ⟨args, valid, settled, -⟩
  have same : args.1 = value := congrArg Prod.fst settled
  have held : NTy.Holds (.param 0) args.1 := valid.1
  rw [NTy.holds_param, same] at held
  exact invalid held

/-- A property of a valid row is stated of its entry values. -/
theorem valid_value_gives_atEntries (value : (NTy.param 0).carrier)
    (valid : Carriers.holds 0 value) {holds : HList valueRow → Prop} (held : holds (value, ())) :
    AtEntries valueRow holds (value, ()) :=
  atEntries_of_holds (σs := valueRow) (args := (value, ()))
    ⟨by rw [NTy.holds_param]; exact valid, trivial⟩ held

end Entries

/-! ## Aborts, settled results and the run `result_of` reads -/

section Invocations
variable {unit : Validation.ValidatedUnit} [FunctionCarriers unit] [Skolems unit]

/-- `aborts_of` of a clause is the statement over entry values of the
invocation's abort, with its domain. -/
example (invoke : InvokeMeaning) (σs : NRow) (shared : Sharing) (shape : ResultShape)
    (function : (NTy.function σs shared shape.row).carrier) (entries : HList σs)
    (state : Memory unit) :
    AbortsOfEntries invoke σs shared shape function entries state =
      AtEntries σs (fun args => AbortsOf invoke σs shared shape function args state) entries :=
  rfl

/-- An abort on valid arguments is an abort at their entry values. -/
theorem abort_at_valid_arguments {invoke : InvokeMeaning} {σs : NRow} {shared : Sharing}
    {shape : ResultShape} {function : (NTy.function σs shared shape.row).carrier}
    {args : HList σs} {state : Memory unit} (valid : HList.Holds σs args)
    (aborts : AbortsOf invoke σs shared shape function args state) :
    AbortsOfEntries invoke σs shared shape function (HList.settle σs args) state :=
  abortsOfEntries_of_abortsOf valid aborts

/-- An abort at entry values is one on valid arguments. -/
theorem abort_at_entries_is_valid {invoke : InvokeMeaning} {σs : NRow} {shared : Sharing}
    {shape : ResultShape} {function : (NTy.function σs shared shape.row).carrier}
    {entries : HList σs} {state : Memory unit}
    (aborts : AbortsOfEntries invoke σs shared shape function entries state) :
    ∃ args, HList.Holds σs args ∧ HList.settle σs args = entries ∧
      AbortsOf invoke σs shared shape function args state :=
  aborts

/-- A run excludes the abort at its entry values without any fact of its
own arguments: the exclusion reads the witness's run and not its validity. -/
theorem run_excludes_abort {executable : Validation.ExecutableUnit unit}
    (agrees : RunsAgree executable) {σs : NRow} (flat : σs.lentFlat = true)
    {shape : ResultShape} {shared : Sharing}
    {function : (NTy.function σs shared shape.row).carrier} {args : HList σs}
    {state post : Memory unit} {result : shape.carrier}
    (ok : EnsuresOf (closureMeaning executable) σs shared shape function args result state post) :
    ¬AbortsOfEntries (closureMeaning executable) σs shared shape function (HList.settle σs args)
      state :=
  not_abortsOfEntries_of_ok agrees flat rfl ok

/-- A returned reference named by a current value outside its type's
predicate has no settled run: the equation with the prophecy says which
value it is and gives it no fact. -/
theorem invalid_current_refutes_ensuresOfSettled {invoke : InvokeMeaning} {σs : NRow}
    {shared : Sharing} {σ : NTy}
    {function : (NTy.function σs shared (ResultShape.one (.ref σ)).row).carrier}
    {entries : HList σs} {finals : HList (NRow.finals σs)} {current : σ.carrier}
    {pre post : Memory unit} (invalid : ¬NTy.Holds σ current) :
    ¬EnsuresOfSettled invoke σs shared σ function entries finals current pre post := by
  rintro ⟨prophecy, valid, rfl, -⟩
  exact invalid valid

/-- A run on valid arguments returning a settled reference of a valid
prophecy is the settled run at that value. -/
theorem valid_prophecy_gives_ensuresOfSettled {invoke : InvokeMeaning} {σs : NRow}
    {shared : Sharing} {σ : NTy}
    {function : (NTy.function σs shared (ResultShape.one (.ref σ)).row).carrier}
    {args : HList σs} {prophecy : σ.carrier} {pre post : Memory unit}
    (validArgs : HList.Holds σs args) (valid : NTy.Holds σ prophecy)
    (ensures : EnsuresOf invoke σs shared (.one (.ref σ)) function args (prophecy, prophecy)
      pre post) :
    EnsuresOfSettled invoke σs shared σ function (HList.settle σs args) (HList.finals σs args)
      prophecy pre post :=
  ensuresOfSettled_of_ensuresOfEntries valid (ensuresOfEntries_of_ensuresOf validArgs ensures)

/-- A run whose result lies outside its type's predicate is no run
`result_of` reads. -/
theorem invalid_result_is_no_candidate {invoke : InvokeMeaning} {σs : NRow} {shared : Sharing}
    {shape : ResultShape} {function : (NTy.function σs shared shape.row).carrier}
    {entries : HList σs} {state : Memory unit} {run : HList σs × shape.carrier × Memory unit}
    (invalid : ¬shape.Holds run.2.1) :
    ¬SettledRun invoke σs shared shape function entries state run :=
  fun candidate => invalid candidate.2.1

/-- Nor is a run on arguments outside their types' predicates. -/
theorem invalid_arguments_are_no_candidate {invoke : InvokeMeaning} {σs : NRow}
    {shared : Sharing} {shape : ResultShape}
    {function : (NTy.function σs shared shape.row).carrier}
    {entries : HList σs} {state : Memory unit} {run : HList σs × shape.carrier × Memory unit}
    (invalid : ¬HList.Holds σs run.1) :
    ¬SettledRun invoke σs shared shape function entries state run :=
  fun candidate => invalid candidate.1

/-- Where a candidate run returns, `result_of` holds its data invariants. -/
theorem candidate_gives_result_fact {invoke : InvokeMeaning} {σs : NRow} {shared : Sharing}
    {shape : ResultShape} {function : (NTy.function σs shared shape.row).carrier}
    {entries : HList σs} {state : Memory unit} {run : HList σs × shape.carrier × Memory unit}
    (candidate : SettledRun invoke σs shared shape function entries state run)
    (default : shape.carrier) :
    shape.Holds (ResultOf invoke σs shared shape function entries state default) :=
  holds_resultOf ⟨runEncoding σs shape run, run, candidate, rfl⟩ default

/-- Where every run from the state returns a result outside its type's
predicate, the labels read no run: `result_of` is the default it was given
and `state_of` the state it was read from.  A run alone does not make the
labels' conclusions; its result's fact does. -/
theorem labels_read_no_invalid_result {invoke : InvokeMeaning} {σs : NRow} {shared : Sharing}
    {shape : ResultShape} {function : (NTy.function σs shared shape.row).carrier}
    {entries : HList σs} {state : Memory unit}
    (invalid : ∀ (args : HList σs) (result : shape.carrier) (post : Memory unit),
      EnsuresOf invoke σs shared shape function args result state post → ¬shape.Holds result)
    (default : shape.carrier) :
    ResultOf invoke σs shared shape function entries state default = default ∧
      StateOf invoke σs shared shape function entries state = state := by
  have none : ¬∃ encoded, SettledEncoding invoke σs shared shape function entries state encoded := by
    rintro ⟨_, run, candidate, -⟩
    exact invalid run.1 run.2.1 run.2.2 candidate.2.2.2.2 candidate.2.1
  unfold ResultOf StateOf
  rw [dif_neg none, dif_neg none]
  exact ⟨rfl, rfl⟩

/-- A run on valid arguments to a valid, settled result is the run the
labels read: with the result's fact, the conclusions hold. -/
theorem labels_read_valid_run {executable : Validation.ExecutableUnit unit}
    (agrees : RunsAgree executable) {σs : NRow} (flat : σs.lentFlat = true)
    {shape : ResultShape} (shapeFlat : shape.row.lentFlat = true) {shared : Sharing}
    {function : (NTy.function σs shared shape.row).carrier} {args : HList σs}
    {state post : Memory unit} {result : shape.carrier} (valid : HList.Holds σs args)
    (ok : EnsuresOf (closureMeaning executable) σs shared shape function args result state post)
    (settled : shape.Settled result) (validResult : shape.Holds result)
    (default : shape.carrier) :
    ResultOf (closureMeaning executable) σs shared shape function (HList.settle σs args) state
        default = result ∧
      StateOf (closureMeaning executable) σs shared shape function (HList.settle σs args) state =
        post :=
  ⟨resultOf_eq_of_ok agrees flat shapeFlat valid rfl ok settled validResult default,
    stateOf_eq_of_ok agrees flat shapeFlat valid rfl ok settled validResult⟩

/-- `requires_of` of a clause is read at a valid row of its entry values,
and is owed at the valid rows alone. -/
theorem requires_at_valid_arguments {requires : RequiresMeaning unit} {σs : NRow}
    {shared : Sharing} {results : NRow} {function : (NTy.function σs shared results).carrier}
    {args : HList σs} {state : Memory unit} (valid : HList.Holds σs args)
    (stated : RequiresOfEntries requires σs shared results function (HList.settle σs args)
      state) :
    RequiresOf requires σs shared results function args state :=
  requiresOf_of_entries valid stated

theorem requires_owed_at_valid_rows {requires : RequiresMeaning unit} {σs : NRow}
    {shared : Sharing} {results : NRow} {function : (NTy.function σs shared results).carrier}
    {entries : HList σs} {state : Memory unit}
    (owed : ∀ args : HList σs, HList.Holds σs args → HList.settle σs args = entries →
      RequiresOf requires σs shared results function args state) :
    RequiresOfEntries requires σs shared results function entries state :=
  owed

end Invocations

/-! ## Captures -/

section Captures
variable {unit : Validation.ValidatedUnit} [FunctionCarriers unit] [Skolems unit]
  {full captured supplied : NRow} (weave : Weave full captured supplied)
  {shape : ResultShape} (run : HList full → Comp unit shape.carrier)
  (captures : HList captured) (entries : HList supplied) (state : Memory unit)

/-- A woven row of valid captures and valid arguments is valid, and a valid
woven row gives both back. -/
theorem woven_valid_iff (args : HList supplied) :
    HList.Holds full (weave.compose captures args) ↔
      HList.Holds captured captures ∧ HList.Holds supplied args :=
  weave.holds_compose captures args

/-- From the supplied row to the target's: the captures' validity makes the
woven row a valid one. -/
theorem supplied_to_woven (free : captured.refFree = true)
    (valid : HList.Holds captured captures)
    (stated : AbortsOfMeaning supplied shape (fun args => run (weave.compose captures args))
      entries state) :
    AbortsOfMeaning full shape run (weave.compose captures entries) state :=
  abortsOfMeaning_weave weave free run captures valid entries state stated

/-- From the target's row to the supplied one, of any captures. -/
theorem woven_to_supplied (free : captured.refFree = true)
    (stated : AbortsOfMeaning full shape run (weave.compose captures entries) state) :
    AbortsOfMeaning supplied shape (fun args => run (weave.compose captures args)) entries
      state :=
  abortsOfMeaning_unweave weave free run captures entries state stated

/-- Captures outside their types' predicates leave the behavior at the
woven entry values without a witness. -/
theorem invalid_captures_refute_woven (free : captured.refFree = true)
    (invalid : ¬HList.Holds captured captures) :
    ¬AbortsOfMeaning full shape run (weave.compose captures entries) state :=
  fun stated => invalid
    (holds_captures_of_abortsOfMeaning weave free run captures entries state stated)

/-- So the equation of a closure's behavior at its target does not hold of
such captures where the supplied row states the behavior: the supplied side
holds and the woven side does not. -/
theorem woven_equation_needs_valid_captures (free : captured.refFree = true)
    (invalid : ¬HList.Holds captured captures)
    (stated : AbortsOfMeaning supplied shape (fun args => run (weave.compose captures args))
      entries state) :
    AbortsOfMeaning supplied shape (fun args => run (weave.compose captures args)) entries state ≠
      AbortsOfMeaning full shape run (weave.compose captures entries) state :=
  fun same => invalid_captures_refute_woven weave run captures entries state free invalid
    (same ▸ stated)

/-- The same two directions of `ensures_of`, whose finals are the supplied
arguments'. -/
theorem ensures_supplied_to_woven (free : captured.refFree = true)
    (valid : HList.Holds captured captures) {finals : HList (NRow.finals supplied)}
    {finals' : HList (NRow.finals full)} (same : HEq finals' finals) (result : shape.carrier)
    (pre post : Memory unit)
    (stated : EnsuresOfMeaning supplied shape (fun args => run (weave.compose captures args))
      entries finals result pre post) :
    EnsuresOfMeaning full shape run (weave.compose captures entries) finals' result pre post :=
  ensuresOfMeaning_weave weave free run captures valid entries same result pre post stated

theorem ensures_woven_to_supplied (free : captured.refFree = true)
    {finals : HList (NRow.finals supplied)} {finals' : HList (NRow.finals full)}
    (same : HEq finals' finals) (result : shape.carrier) (pre post : Memory unit)
    (stated : EnsuresOfMeaning full shape run (weave.compose captures entries) finals' result
      pre post) :
    EnsuresOfMeaning supplied shape (fun args => run (weave.compose captures args)) entries finals
      result pre post :=
  ensuresOfMeaning_unweave weave free run captures entries same result pre post stated

end Captures

/-! ## Frames -/

section Frames
variable {unit : Validation.ValidatedUnit} [FunctionCarriers unit]
  {executable : Validation.ExecutableUnit unit} [Skolems unit]
  {parameters : NRow} {shared : Sharing} {shape : ResultShape}
  {function : (NTy.function parameters shared shape.row).carrier}

/-- A frame fact states nothing of a run whose result lies outside its
type's predicate: where no result holds, every function value keeps even the
frame no memory satisfies. -/
theorem frame_silent_on_invalid_results
    (invalid : ∀ result : shape.carrier, ¬shape.Holds result) :
    FramedAt executable parameters shared shape (fun _ _ _ => False) function :=
  fun _ _ _ result _ _ _ validResult _ _ => (invalid result validResult).elim

/-- The same of a frame over an invocation meaning. -/
theorem frameOf_silent_on_invalid_results (invoke : InvokeMeaning)
    (invalid : ∀ result : shape.carrier, ¬shape.Holds result) :
    FramedAtOf invoke parameters shared shape (fun _ _ _ => False) function :=
  fun _ result _ _ _ validResult _ _ => (invalid result validResult).elim

/-- At a run on valid arguments of a value whose results are valid, the
frame is the run's. -/
theorem frame_at_valid_run {frame : HList parameters → Memory unit → Memory unit → Prop}
    (framed : FramedAt executable parameters shared shape frame function)
    (validResults : ValidResultsAt executable parameters shared shape function)
    (free : parameters.refFree = true) (shapeFree : shape.row.refFree = true)
    {args : HList parameters} {result : shape.carrier} {pre post : Memory unit}
    (valid : HList.Holds parameters args) (typed : MemoryTyped unit pre)
    (ensures : EnsuresOf (closureMeaning executable) parameters shared shape function args result
      pre post) :
    frame args pre post :=
  framed.at_run validResults free shapeFree valid typed ensures

end Frames

/-! ## Axioms

Every statement above and every law it reads rests on the three standard
axioms and on nothing else. -/

open Lean Elab Command in
/-- Fails where a declaration depends on an axiom other than the standard
three. -/
elab "#standard_axioms " names:ident+ : command => do
  for name in names do
    let declaration ← liftCoreM <| realizeGlobalConstNoOverloadWithInfo name
    let axioms ← collectAxioms declaration
    for used in axioms do
      unless [``propext, ``Classical.choice, ``Quot.sound].contains used do
        throwErrorAt name "{declaration} depends on the axiom {used}"

#standard_axioms
  invalid_prophecy_refutes_atEntryFinals
  valid_prophecy_gives_atEntryFinals
  atEntryFinals_gives_prophecy_fact
  invalid_value_refutes_atEntries
  run_excludes_abort
  invalid_current_refutes_ensuresOfSettled
  candidate_gives_result_fact
  supplied_to_woven
  woven_to_supplied
  invalid_captures_refute_woven
  ensures_supplied_to_woven
  ensures_woven_to_supplied
  labels_read_no_invalid_result
  labels_read_valid_run
  frame_silent_on_invalid_results
  frameOf_silent_on_invalid_results
  frame_at_valid_run
  VerexIR.Proofs.FramedAt.at_run
  VerexIR.Proofs.FramedAt.mono
  VerexIR.Proofs.FramedAtOf.mono
  VerexIR.Proofs.Denote.Weave.holds_compose
  VerexIR.Proofs.Denote.Weave.holds_split
  VerexIR.Proofs.abortsOfMeaning_woven
  VerexIR.Proofs.ensuresOfMeaning_woven
  VerexIR.Proofs.ensuresOfSettled_closureOf_woven
  VerexIR.Proofs.holds_resultOf
  VerexIR.Proofs.settledEncoding_instantiate
  VerexIR.Proofs.resultOf_eq_of_ok
  VerexIR.Proofs.stateOf_eq_of_ok
  VerexIR.Proofs.wp_closureMeaning_unseen
  VerexIR.Proofs.wp_named_typed
  VerexIR.Proofs.wp_invoke_unseen
  VerexIR.Proofs.returns_of_terminating
  VerexIR.Proofs.FramedAt.ofVerified
  VerexIR.Proofs.FramedAt.ofBody
  VerexIR.Proofs.FramedAt.of_instantiate
  VerexIR.Proofs.FramedAtOf.ofRouted

end VerexIR.Tests.BehaviorDomains

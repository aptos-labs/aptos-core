-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Meaning
import LeanerIR.Proofs.Denote.Closures

/-!
# Behavioral predicates

The behavior of a function value as a specification states it
(`designs/higher-order-functions.md`, "Behavioral predicates and state
labels"): `aborts_of`, `ensures_of`, and `result_of` are defined from the
invocation's big-step meaning on runtime values, from the starts a call
runs from (`Denote.closureMeaning`), exactly and without axioms. Of a
closure whose target, weave, and captures a proof sees, they are runs of
the target's prophetic meaning, which its theorem speaks about.
-/

namespace LeanerIR.Proofs

open LeanerIR.Validation (ExecutableUnit ValidatedUnit)

/-- A literal callable's encoding with its specification type retained as
an elaboration hint. This asserts no typing property: invocation rules
still establish the closure's typing independently. -/
abbrev Denote.ClosureValue.encodeFor (_type : Denote.NTy) (closure : Denote.ClosureValue) : RuntimeValue :=
  closure.encode

/-- An invocation of a function value on supplied arguments, as the big-step
semantics runs it: the closure's target under the instantiation the closure
fixed, on the captures and the arguments composed by its mask. A value that
is not a closure, and arguments the mask does not compose with, have no
execution. -/
def invocationSpec {unit : ValidatedUnit} (executable : ExecutableUnit unit)
    (callable : RuntimeValue)
    (arguments : List RuntimeValue) : Spec RuntimeState Failure (Array RuntimeValue) :=
  match callable with
  | .closure function mask typeInstantiation captures =>
      match ClosureMask.compose mask captures.toList arguments with
      | some composed => functionSpecAt executable function typeInstantiation composed.toArray
      | none => Spec.bottom
  | _ => Spec.bottom

/-- A start an invocation runs from, as a call's does: globals encoding
the memory, with loan bookkeeping that lends nothing (`Denote.Admissible`). -/
def StartsAt {unit : ValidatedUnit} (start : RuntimeState) (memory : Denote.Memory unit) : Prop :=
  Denote.Encodes unit memory start.globals ∧ Denote.Admissible start []

/-- `aborts_of<f>(x)`: the invocation aborts from a start at `memory`. -/
def AbortsOf {unit : ValidatedUnit} (executable : ExecutableUnit unit) (callable : RuntimeValue)
    (arguments : List RuntimeValue)
    (memory : Denote.Memory unit) : Prop :=
  ∃ start failure, StartsAt start memory ∧
    (invocationSpec executable callable arguments).aborts start failure

/-- `ensures_of<f>(x, r)`: the invocation returns `results` from a start at
`pre` and leaves the memory `post`. -/
def EnsuresOf {unit : ValidatedUnit} (executable : ExecutableUnit unit) (callable : RuntimeValue)
    (arguments : List RuntimeValue)
    (results : Array RuntimeValue) (pre post : Denote.Memory unit) : Prop :=
  ∃ start exit, StartsAt start pre ∧ (invocationSpec executable callable arguments).ok start results exit ∧
    Denote.Encodes unit post exit.globals ∧ Denote.AgreeUnnamed unit post pre

open Classical in
/-- `result_of<f>(x)`: results the invocation returns from a start at
`memory`; unspecified when it returns none. -/
noncomputable def ResultOf {unit : ValidatedUnit} (executable : ExecutableUnit unit)
    (callable : RuntimeValue)
    (arguments : List RuntimeValue) (state : Denote.Memory unit) : Array RuntimeValue :=
  if returns : ∃ results post, EnsuresOf executable callable arguments results state post
  then returns.choose else #[]

open Classical in
/-- The post-state of a successful invocation, sharing `ResultOf`'s choice.
On an invocation with no successful execution this defaults to its pre-state;
using the label does not assume that the invocation succeeds. -/
noncomputable def StateOf {unit : ValidatedUnit} (executable : ExecutableUnit unit)
    (callable : RuntimeValue) (arguments : List RuntimeValue)
    (state : Denote.Memory unit) : Denote.Memory unit :=
  if returns : ∃ results post, EnsuresOf executable callable arguments results state post
  then returns.choose_spec.choose else state

/-- Where the invocation returns, `result_of` names results it returns. -/
theorem ensuresOf_resultOf {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    {callable : RuntimeValue}
    {arguments : List RuntimeValue} {results : Array RuntimeValue} {pre post : Denote.Memory unit}
    (ensures : EnsuresOf executable callable arguments results pre post) :
    ∃ post, EnsuresOf executable callable arguments (ResultOf executable callable arguments pre) pre post := by
  have returns : ∃ results post, EnsuresOf executable callable arguments results pre post :=
    ⟨results, post, ensures⟩
  unfold ResultOf
  rw [dif_pos returns]
  exact returns.choose_spec

/-- Declared preconditions by target: a function's at its arguments. A
unit's table names the targets its closures have whose contracts state no
behavioral predicate, and holds `True` for every other function. -/
abbrev RequiresTable (unit : Validation.ValidatedUnit) :=
  FunctionHandle → Array RuntimeValue → Denote.Memory unit → Prop

/-- `requires_of<f>(x)`: the declared precondition of the closure's target,
read from a table, at the captures and the arguments composed by its mask;
`True` where there is none to read. -/
def RequiresOf {unit : Validation.ValidatedUnit} (table : RequiresTable unit)
    (callable : RuntimeValue) (arguments : List RuntimeValue)
    (state : Denote.Memory unit) : Prop :=
  match callable with
  | .closure function mask _ captures =>
      match ClosureMask.compose mask captures.toList arguments with
      | some composed => table function composed.toArray state
      | none => True
  | _ => True

section
open Denote
variable {unit : Validation.ValidatedUnit} [Skolems unit]

/-- `requires_of` of a closure whose target, weave, and captures a proof sees
is the target's declared precondition at the captures and the arguments
woven into its parameter row. -/
theorem requiresOf_closureOf (table : RequiresTable unit) (handle : FunctionHandle)
    {full captured supplied : NRow} (weave : Weave full captured supplied)
    (typeInstantiation : Array (TypeId × TypeId)) (captures : HList captured)
    (arguments : List RuntimeValue) (state : Memory unit)
    (lengths : arguments.length = supplied.length) :
    RequiresOf table (closureOf handle weave.mask typeInstantiation captures).encode arguments
        state =
      table handle (weave.composeList (HList.encode captures) arguments).toArray state := by
  simp only [RequiresOf, ClosureValue.encode, closureOf, List.toList_toArray]
  rw [weave.compose_mask _ _ (HList.encode_length captured captures) lengths]

/-- A woven row lent under no loans resolves every argument: none is a
reference. -/
private theorem argumentsResolve_refFree : (row : NRow) → row.refFree = true →
    (values : HList row) → (returned : Array RuntimeValue) → (exports : List (Nat × RuntimeValue)) →
    argumentsResolve row values [] returned exports
  | .nil, _, _, _, _ => trivial
  | .cons _ rest, free, values, returned, exports => by
      simp only [NRow.refFree, Bool.and_eq_true] at free
      exact (argumentsResolve_cons_refFree free.1 values [] returned exports).mpr
        (argumentsResolve_refFree rest free.2 values.2 returned exports)

/-- The woven row of reference-free captures and arguments lends as the
woven encodings. -/
theorem lendArguments_weave {full captured supplied : NRow}
    (weave : Weave full captured supplied) (capturedFree : captured.refFree = true)
    (suppliedFree : supplied.refFree = true) (captures : HList captured) (args : HList supplied) :
    lendArguments full (weave.compose captures args) [] =
      some (weave.composeList (HList.encode captures) (HList.encode args)).toArray := by
  unfold lendArguments
  rw [weave.lend_compose capturedFree, NRow.lend_refFree supplied suppliedFree]
  rfl

/-- The invocation of a closure whose target, weave, and captures a proof
sees, on reference-free arguments, runs the target on the woven row. -/
theorem invocationSpec_closureOf {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    [Skolems unit] {handle : FunctionHandle}
    {full captured supplied : NRow} (weave : Weave full captured supplied)
    (typeInstantiation : Array (TypeId × TypeId)) (captures : HList captured)
    (args : HList supplied) :
    invocationSpec executable (closureOf handle weave.mask typeInstantiation captures).encode
        (HList.encode args) =
      functionSpecAt executable handle typeInstantiation
        (weave.composeList (HList.encode captures) (HList.encode args)).toArray := by
  simp only [invocationSpec, ClosureValue.encode, closureOf, List.toList_toArray]
  rw [weave.compose_mask _ _ (HList.encode_length captured captures)
    (HList.encode_length supplied args)]

/-- `ensures_of` of a closure whose target, weave, and captures a proof sees,
at reference-free arguments and result, is a run of the target's prophetic
meaning at a frame coherent with the closure's instantiation. -/
theorem ensuresOf_closureOf {unit : ValidatedUnit} {executable : ExecutableUnit unit} [Skolems unit]
    {handle : FunctionHandle}
    {full captured supplied : NRow} (weave : Weave full captured supplied)
    (capturedFree : captured.refFree = true) (suppliedFree : supplied.refFree = true)
    {τ : NTy} (resultFree : τ.refFree = true) (typeInstantiation : Array (TypeId × TypeId))
    (coherent : Coherent unit handle typeInstantiation)
    (captures : HList captured) (args : HList supplied) (result : τ.carrier)
    {pre post : Memory unit}
    (ensures : EnsuresOf executable (closureOf handle weave.mask typeInstantiation captures).encode
      (HList.encode args) #[τ.encode result] pre post) :
    (propheticMeaning executable typeInstantiation handle full (.one τ)
      (weave.compose captures args)).ok pre result post := by
  obtain ⟨start, exit, ⟨globals, admissible⟩, runs, encoded, agree⟩ := ensures
  rw [invocationSpec_closureOf] at runs
  have lent : ResultShape.lend (.one τ) false result [] = some #[τ.encode result] := by
    simp only [ResultShape.lend, NTy.lend_refFree τ resultFree]
  have prophesied : ResultShape.lend (.one τ) true result [] = some #[τ.encode result] := by
    simp only [ResultShape.lend, NTy.lend_refFree τ resultFree]
  refine ⟨coherent, start, [], _, _, exit, [], _, globals, admissible,
    lendArguments_weave weave capturedFree suppliedFree captures args, runs, lent, prophesied,
    ?_, encoded, agree⟩
  exact (weave.argumentsResolve_compose capturedFree captures args [] _ _).mpr
    (argumentsResolve_refFree supplied suppliedFree args _ _)

/-- `aborts_of` of a closure whose target, weave, and captures a proof sees,
at reference-free arguments, is an abort of the target's prophetic meaning at
a frame coherent with the closure's instantiation. -/
theorem abortsOf_closureOf {unit : ValidatedUnit} {executable : ExecutableUnit unit} [Skolems unit]
    {handle : FunctionHandle}
    {full captured supplied : NRow} (weave : Weave full captured supplied)
    (capturedFree : captured.refFree = true) (suppliedFree : supplied.refFree = true)
    (shape : ResultShape) (typeInstantiation : Array (TypeId × TypeId))
    (coherent : Coherent unit handle typeInstantiation)
    (captures : HList captured) (args : HList supplied) {state : Memory unit}
    (aborts : AbortsOf executable (closureOf handle weave.mask typeInstantiation captures).encode
      (HList.encode args) state) :
    ∃ failure, (propheticMeaning executable typeInstantiation handle full shape
      (weave.compose captures args)).aborts state failure := by
  obtain ⟨start, failure, ⟨globals, admissible⟩, runs⟩ := aborts
  rw [invocationSpec_closureOf] at runs
  exact ⟨failure, coherent, start, [], _, globals, admissible,
    lendArguments_weave weave capturedFree suppliedFree captures args, runs⟩

/-- `ensures_of` of a closure whose target is verified: where what the
target's theorem assumes and its precondition hold, what its contract
ensures of the run, and that no condition under which it must abort held. -/
theorem ensuresOf_closureOf_verified {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    [Skolems unit] {handle : FunctionHandle}
    {full captured supplied : NRow} (weave : Weave full captured supplied)
    (capturedFree : captured.refFree = true) (suppliedFree : supplied.refFree = true)
    {τ : NTy} (resultFree : τ.refFree = true) (typeInstantiation : Array (TypeId × TypeId))
    (coherent : Coherent unit handle typeInstantiation)
    (captures : HList captured) (args : HList supplied) (result : τ.carrier)
    {pre post : Memory unit} {contract : Contract (Memory unit) Failure (HList full) τ.carrier}
    (verified : Satisfies (propheticMeaning executable typeInstantiation handle full (.one τ)) contract)
    (ensures : EnsuresOf executable (closureOf handle weave.mask typeInstantiation captures).encode
      (HList.encode args) #[τ.encode result] pre post)
    (assumed : contract.assumes (weave.compose captures args) pre)
    (permitted : contract.requires (weave.compose captures args) pre) :
    (¬contract.mayAbort (weave.compose captures args) pre →
        contract.ensures (weave.compose captures args) pre result post) ∧
      contract.frame (weave.compose captures args) pre post ∧
      ¬contract.mustAbort (weave.compose captures args) pre :=
  (verified _ pre assumed permitted).1 result post
    (ensuresOf_closureOf weave capturedFree suppliedFree resultFree typeInstantiation coherent
      captures args result ensures)

/-- `aborts_of` of a closure whose target is verified: where what the
target's theorem assumes and its precondition hold, a failure its contract
permits. -/
theorem abortsOf_closureOf_verified {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    [Skolems unit] {handle : FunctionHandle}
    {full captured supplied : NRow} (weave : Weave full captured supplied)
    (capturedFree : captured.refFree = true) (suppliedFree : supplied.refFree = true)
    {shape : ResultShape} (typeInstantiation : Array (TypeId × TypeId))
    (coherent : Coherent unit handle typeInstantiation)
    (captures : HList captured) (args : HList supplied) {state : Memory unit}
    {contract : Contract (Memory unit) Failure (HList full) shape.carrier}
    (verified : Satisfies (propheticMeaning executable typeInstantiation handle full shape) contract)
    (aborts : AbortsOf executable (closureOf handle weave.mask typeInstantiation captures).encode
      (HList.encode args) state)
    (assumed : contract.assumes (weave.compose captures args) state)
    (permitted : contract.requires (weave.compose captures args) state) :
    ∃ failure, contract.aborts (weave.compose captures args) state failure := by
  obtain ⟨failure, runs⟩ := abortsOf_closureOf weave capturedFree suppliedFree shape
    typeInstantiation coherent captures args aborts
  exact ⟨failure, (verified _ state assumed permitted).2.1 failure runs⟩

/-- The arguments of a row without references lend as their encodings, and
only under no loans. -/
private theorem lendArguments_refFree {row : NRow} (free : row.refFree = true)
    {args : HList row} {loans : List Nat} {arguments : Array RuntimeValue}
    (lent : lendArguments row args loans = some arguments) :
    loans = [] ∧ arguments = (HList.encode args).toArray := by
  unfold lendArguments at lent
  rw [NRow.lend_refFree row free false args loans] at lent
  cases loans with
  | nil => exact ⟨rfl, (Option.some.inj lent).symm⟩
  | cons _ _ => exact absurd lent (by simp)

/-- A run of a target that takes and returns no reference is a run of the
function value naming it: `ensures_of` of that value at the call's
arguments. -/
theorem ensuresOf_of_run {unit : ValidatedUnit} {executable : ExecutableUnit unit} [Skolems unit]
    {handle : FunctionHandle}
    {typeInstantiation : Array (TypeId × TypeId)} {full : NRow} (fullFree : full.refFree = true)
    {shape : ResultShape} (shapeFree : shape.row.refFree = true) {args : HList full}
    {result : shape.carrier} {pre final : Memory unit}
    (runs : (propheticMeaning executable typeInstantiation handle full shape args).ok pre result final) :
    EnsuresOf executable (closureOf handle (Weave.supplying full).mask typeInstantiation
      (σs := .nil) ()).encode (HList.encode args) ((resultCodec shape).encode result) pre final := by
  obtain ⟨-, start, loans, arguments, results, exit, returnedLoans, _, globals, admissible,
    lent, ran, resultsLent, -, -, encoded, agree⟩ := runs
  obtain ⟨rfl, rfl⟩ := lendArguments_refFree fullFree lent
  have returned : results = (resultCodec shape).encode result := by
    cases shape with
    | none =>
        cases returnedLoans with
        | nil => exact (Option.some.inj resultsLent).symm
        | cons _ _ => exact absurd resultsLent (by simp [ResultShape.lend])
    | one τ =>
        have resultFree : τ.refFree = true := by
          simpa only [ResultShape.row, NRow.refFree, Bool.and_true] using shapeFree
        simp only [ResultShape.lend, NTy.lend_refFree τ resultFree] at resultsLent
        cases returnedLoans with
        | nil => exact (Option.some.inj resultsLent).symm
        | cons _ _ => exact absurd resultsLent (by simp)
  subst returned
  refine ⟨start, exit, ⟨globals, admissible⟩, ?_, encoded, agree⟩
  rw [invocationSpec_closureOf (Weave.supplying full) typeInstantiation () args,
    HList.encode_nil, Weave.composeList_supplying]
  exact ran

/-- An abort of a target that takes no reference is an abort of the function
value naming it: `aborts_of` of that value at the call's arguments. -/
theorem abortsOf_of_abort {unit : ValidatedUnit} {executable : ExecutableUnit unit} [Skolems unit]
    {handle : FunctionHandle}
    {typeInstantiation : Array (TypeId × TypeId)} {full : NRow} (fullFree : full.refFree = true)
    {shape : ResultShape} {args : HList full} {pre : Memory unit} {failure : Failure}
    (aborts : (propheticMeaning executable typeInstantiation handle full shape args).aborts pre failure) :
    AbortsOf executable (closureOf handle (Weave.supplying full).mask typeInstantiation
      (σs := .nil) ()).encode (HList.encode args) pre := by
  obtain ⟨-, start, loans, arguments, globals, admissible, lent, ran⟩ := aborts
  obtain ⟨rfl, rfl⟩ := lendArguments_refFree fullFree lent
  refine ⟨start, failure, ⟨globals, admissible⟩, ?_⟩
  rw [invocationSpec_closureOf (Weave.supplying full) typeInstantiation () args,
    HList.encode_nil, Weave.composeList_supplying]
  exact ran

/-- Where a target that takes and returns no reference is called, the run is
one of the function value naming the target: the continuations may assume
its `ensures_of` where the call returns and its `aborts_of` where it aborts. -/
theorem wp_named {unit : ValidatedUnit} {executable : ExecutableUnit unit} [Skolems unit]
    {handle : FunctionHandle}
    {typeInstantiation : Array (TypeId × TypeId)} {full : NRow} {shape : ResultShape}
    (fullFree : full.refFree = true) (shapeFree : shape.row.refFree = true) {args : HList full}
    {ensures : shape.carrier → Memory unit → Prop} {aborts : Failure → Prop}
    {initial : Memory unit}
    (named : wp (propheticMeaning executable typeInstantiation handle full shape args)
      (fun result final =>
        EnsuresOf executable (ClosureValue.encode (closureOf handle
          (Weave.supplying full).mask typeInstantiation (σs := .nil) ()))
          (HList.encode args) ((resultCodec shape).encode result) initial final →
        ensures result final)
      (fun error =>
        AbortsOf executable (ClosureValue.encode (closureOf handle
          (Weave.supplying full).mask typeInstantiation (σs := .nil) ()))
          (HList.encode args) initial → aborts error)
      initial) :
    wp (propheticMeaning executable typeInstantiation handle full shape args) ensures aborts initial :=
  ⟨fun result final execution =>
      named.1 result final execution (ensuresOf_of_run fullFree shapeFree execution),
    fun error execution => named.2.1 error execution (abortsOf_of_abort fullFree execution),
    named.2.2⟩

end

end LeanerIR.Proofs

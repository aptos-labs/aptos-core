-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Meaning
import VerexIR.Proofs.Denote.Closures

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

namespace VerexIR.Proofs

open VerexIR.Validation (ExecutableUnit ValidatedUnit)

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

/-- A start an invocation runs from, as a call's does: globals and Table
storage encoding the memory, with loan bookkeeping that lends nothing (`Denote.Admissible`). -/
def StartsAt {unit : ValidatedUnit} (start : RuntimeState) (memory : Denote.Memory unit) : Prop :=
  Denote.StorageEncodes unit memory start ∧ Denote.Admissible start []

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
    Denote.StorageEncodesReturned unit post exit results ∧ Denote.AgreeUnnamed unit post pre

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

/-! ## Invocations lending mutable references

A function value whose parameters include mutable references is invoked as a
call invokes its target: each `&mut` argument is lent under a loan of its
own and passes its entry value; its final value is the one the run exports
for that loan, the holes of the references the run returns filled
(`Denote.argumentsResolve`). `mutable` marks the supplied arguments that are
mutable references; the predicates read their entry values among the
arguments and their final values in order, as the Move Prover lays out
`ensures_of<f>(inputs, results, finals)`. `mutableResults` marks the results
that are mutable references, which the predicates read as their referents'
values at return (`readResults`). -/

/-- The supplied arguments with each mutable one lent under the next loan,
as a borrow of its entry value; and the loans left. -/
def lendMutable : List Bool → List RuntimeValue → List Nat → Option (List RuntimeValue × List Nat)
  | [], [], loans => some ([], loans)
  | true :: mutable, entry :: arguments, loan :: loans =>
      (lendMutable mutable arguments loans).map fun lent => (.borrow loan entry :: lent.1, lent.2)
  | false :: mutable, argument :: arguments, loans =>
      (lendMutable mutable arguments loans).map fun lent => (argument :: lent.1, lent.2)
  | _, _, _ => none

/-- The results a behavioral predicate reads of returned values: a mutable
reference, where `mutable` marks one, as its referent's value at return
(`RuntimeValue.observe`), as the Move Prover reads a reference result; any
other result as returned. -/
def readResults : List Bool → List RuntimeValue → List RuntimeValue
  | true :: mutable, value :: values => value.observe :: readResults mutable values
  | false :: mutable, value :: values => value :: readResults mutable values
  | _, values => values

/-- Results none of which is a mutable reference are read as returned. -/
theorem readResults_of_all_false : {mutable : List Bool} → (∀ marked ∈ mutable, marked = false) →
    (values : List RuntimeValue) → readResults mutable values = values
  | [], _, values => by cases values <;> rfl
  | marked :: mutable, unmarked, values => by
      have plain : marked = false := unmarked marked (by simp)
      subst plain
      cases values with
      | nil => rfl
      | cons value values =>
          simp only [readResults, List.cons.injEq, true_and]
          exact readResults_of_all_false
            (fun marked member => unmarked marked (by simp [member])) values

/-- Lent loans resolve at final values, in order, as lent arguments do
(`Denote.argumentsResolve`): the run exported each loan with a value that,
the returned references' holes filled, is its final value, or returned the
loan carrying it. -/
def ResolvesAt (returned : Array RuntimeValue) (exports : List (Nat × RuntimeValue)) :
    List Nat → List RuntimeValue → Prop
  | _, [] => True
  | [], _ :: _ => False
  | loan :: loans, final :: finals =>
      (match Denote.exportedRaw? loan exports with
        | some exported => SemanticOperations.resolveReturnedBorrows returned exported = final
        | none => ∃ entry ∈ returned.toList,
            (loan, final) ∈ (SemanticOperations.outermostBorrows entry).toList) ∧
      ResolvesAt returned exports loans finals

/-- `aborts_of<f>(x)` of a function value taking mutable references: the
invocation lending them aborts from a start at `memory`. -/
def AbortsOfMut {unit : ValidatedUnit} (executable : ExecutableUnit unit) (callable : RuntimeValue)
    (mutable : List Bool) (arguments : List RuntimeValue) (memory : Denote.Memory unit) : Prop :=
  ∃ start loans lent failure,
    Denote.StorageEncodes unit memory start ∧ Denote.Admissible start loans ∧
    lendMutable mutable arguments loans = some (lent, []) ∧
    (invocationSpec executable callable lent).aborts start failure

/-- `ensures_of<f>(x, r, y)` of a function value taking or returning mutable
references: the invocation lending them returns results read as `results`
from a start at `pre`, leaves the memory `post`, and the references at
`finals`. -/
def EnsuresOfMut {unit : ValidatedUnit} (executable : ExecutableUnit unit) (callable : RuntimeValue)
    (mutable mutableResults : List Bool) (arguments : List RuntimeValue)
    (results : Array RuntimeValue) (finals : List RuntimeValue) (pre post : Denote.Memory unit) :
    Prop :=
  ∃ start loans lent returned exit,
    Denote.StorageEncodes unit pre start ∧ Denote.Admissible start loans ∧
    lendMutable mutable arguments loans = some (lent, []) ∧
    (invocationSpec executable callable lent).ok start returned exit ∧
    results = (readResults mutableResults returned.toList).toArray ∧
    ResolvesAt returned (Denote.exportsAfter start.pending exit.pending) loans finals ∧
    Denote.StorageEncodesReturned unit post exit returned ∧ Denote.AgreeUnnamed unit post pre

open Classical in
/-- `result_of<f>(x)` of a function value taking or returning mutable
references: results the invocation returns from a start at `memory`, read
as `EnsuresOfMut` reads them; unspecified when it returns none. -/
noncomputable def ResultOfMut {unit : ValidatedUnit} (executable : ExecutableUnit unit)
    (callable : RuntimeValue) (mutable mutableResults : List Bool) (arguments : List RuntimeValue)
    (state : Denote.Memory unit) : Array RuntimeValue :=
  if returns : ∃ results finals post,
      EnsuresOfMut executable callable mutable mutableResults arguments results finals state post
  then returns.choose else #[]

/-- Where the invocation returns, `result_of` names results it returns. -/
theorem ensuresOfMut_resultOfMut {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    {callable : RuntimeValue} {mutable mutableResults : List Bool} {arguments : List RuntimeValue}
    {results : Array RuntimeValue} {finals : List RuntimeValue} {pre post : Denote.Memory unit}
    (ensures : EnsuresOfMut executable callable mutable mutableResults arguments results finals
      pre post) :
    ∃ finals post, EnsuresOfMut executable callable mutable mutableResults arguments
      (ResultOfMut executable callable mutable mutableResults arguments pre) finals pre post := by
  have returns : ∃ results finals post,
      EnsuresOfMut executable callable mutable mutableResults arguments results finals pre post :=
    ⟨results, finals, post, ensures⟩
  unfold ResultOfMut
  rw [dif_pos returns]
  exact returns.choose_spec

section
open Denote

/-- The positions of a parameter row that are mutable references. -/
def Denote.NRow.mutable : NRow → List Bool
  | .nil => []
  | .cons (.ref _) rest => true :: rest.mutable
  | .cons _ rest => false :: rest.mutable

/-- A row without references marks none of its positions mutable. -/
theorem Denote.NRow.mutable_refFree : (row : NRow) → row.refFree = true →
    ∀ marked ∈ row.mutable, marked = false
  | .nil, _, _, member => nomatch member
  | .cons τ rest, free, marked, member => by
      simp only [NRow.refFree, Bool.and_eq_true] at free
      cases τ <;> simp only [NTy.refFree, Bool.false_eq_true, false_and] at free <;>
        simp only [NRow.mutable, List.mem_cons] at member <;>
        rcases member with rfl | member <;>
        first | rfl | exact NRow.mutable_refFree rest free.2 marked member

/-- A parameter row each element of which is a mutable reference or lends as
its encoding, as a function's parameters are, none holding map contents. -/
def Denote.NRow.lentFlat : NRow → Bool
  | .nil => true
  | .cons (.ref σ) rest => σ.tableFree && rest.lentFlat
  | .cons τ rest => τ.lendsEncoded && rest.lentFlat


variable [Carriers]

/-- The values a row of arguments passes, as encodings: a mutable
reference's entry value, any other argument's value. A flat row holds no
table, so these are the physical values a run lends (`NRow.lend_lentFlat`). -/
def Denote.HList.entries : {row : NRow} → HList row → List RuntimeValue
  | .nil, _ => []
  | .cons (.ref σ) _, values => σ.encode values.1.1 :: HList.entries values.2
  | .cons τ _, values => τ.encode values.1 :: HList.entries values.2

/-- The final values of a row's mutable references, in order, as encodings. -/
def Denote.HList.finals : {row : NRow} → HList row → List RuntimeValue
  | .nil, _ => []
  | .cons (.ref σ) _, values => σ.encode values.1.2 :: HList.finals values.2
  | .cons _ _, values => HList.finals values.2

/-- The values of a flat row hold no table. -/
theorem Denote.HList.tables_eq_nil_of_lentFlat (prophecies : Bool) : (row : NRow) →
    row.lentFlat = true → (values : HList row) → HList.tables prophecies row values = []
  | .nil, _, _ => by simp [HList.tables]
  | .cons τ rest, flat, values => by
      cases τ with
      | ref σ =>
          simp only [NRow.lentFlat, Bool.and_eq_true] at flat
          simp only [HList.tables, NTy.tables, NTy.tables_eq_nil prophecies σ flat.1,
            HList.tables_eq_nil_of_lentFlat prophecies rest flat.2 values.2, List.append_nil]
      | _ =>
          simp only [NRow.lentFlat, Bool.and_eq_true] at flat
          simp only [HList.tables, NTy.tables_eq_nil prophecies _ (NTy.tableFree_of_lendsEncoded flat.1),
            HList.tables_eq_nil_of_lentFlat prophecies rest flat.2 values.2, List.append_nil]

/-- A result of a flat shape holds no table. -/
theorem Denote.ResultShape.tables_eq_nil_of_lentFlat (prophecies : Bool) : (shape : ResultShape) →
    shape.row.lentFlat = true → (result : shape.carrier) → shape.tables prophecies result = []
  | .none, _, _ => rfl
  | .one τ, flat, result => by
      have := HList.tables_eq_nil_of_lentFlat prophecies (.cons τ .nil) flat (result, ())
      simpa [HList.tables, ResultShape.tables] using this

end

section
open Denote
variable {unit : Validation.ValidatedUnit} [Skolems unit]

/-- A flat row lends as its entries, each mutable one under the next loan. -/
theorem Denote.NRow.lend_lentFlat : (row : NRow) → row.lentFlat = true → (values : HList row) →
    (loans : List Nat) →
    NRow.lend row false values loans = lendMutable row.mutable (HList.entries values) loans
  | .nil, _, _, _ => rfl
  | .cons τ rest, flat, values, loans => by
      cases τ with
      | ref σ =>
          simp only [NRow.lentFlat, Bool.and_eq_true] at flat
          cases loans with
          | nil => rfl
          | cons loan loans =>
              simp only [NRow.lend, NTy.lend, Option.bind_some, NRow.mutable, HList.entries,
                lendMutable, NRow.lend_lentFlat rest flat.2 values.2 loans, ite_false,
                Bool.false_eq_true, NTy.erase_eq_encode σ flat.1]
      | _ =>
          simp only [NRow.lentFlat, Bool.and_eq_true] at flat
          simp only [NRow.lend, NTy.lend_lendsEncoded _ flat.1, Option.bind_some, NRow.mutable,
            HList.entries, lendMutable, NRow.lend_lentFlat rest flat.2 values.2 loans]

/-- A row's references resolve as its final values do. -/
theorem argumentsResolve_finals : (row : NRow) → row.lentFlat = true → (values : HList row) →
    (loans : List Nat) → (returned : Array RuntimeValue) → (exports : List (Nat × RuntimeValue)) →
    (argumentsResolve row values loans returned exports ↔
      ResolvesAt returned exports loans (HList.finals values))
  | .nil, _, _, _, _, _ => by simp [argumentsResolve, HList.finals, ResolvesAt]
  | .cons τ rest, flat, values, loans, returned, exports => by
      have flatRest : rest.lentFlat = true := by
        cases τ <;> simp only [NRow.lentFlat, Bool.and_eq_true] at flat <;> exact flat.2
      have later := argumentsResolve_finals rest flatRest values.2
      cases τ
      case ref σ =>
        have free : σ.tableFree = true := by
          simp only [NRow.lentFlat, Bool.and_eq_true] at flat
          exact flat.1
        cases loans <;>
          simp only [argumentsResolve, HList.finals, ResolvesAt, later, NTy.erase_eq_encode σ free]
        -- The two sides' matches on the export are separate but equal auxiliaries.
        all_goals exact Iff.rfl
      all_goals
        cases loans <;> simp only [argumentsResolve, HList.finals, ResolvesAt, later]
end

section
open Denote
variable {unit : Validation.ValidatedUnit} [Skolems unit]

/-- The invocation of a closure whose target, weave, and captures a proof
sees, on lent supplied arguments, runs the target on the woven row, which
lends as the woven arguments. -/
theorem invocationSpec_closureOf_lent {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    [Skolems unit] {handle : FunctionHandle}
    {full captured supplied : NRow} (weave : Weave full captured supplied)
    (capturedPlain : captured.lendsEncoded = true) (suppliedFlat : supplied.lentFlat = true)
    (typeInstantiation : Array (TypeId × TypeId)) (captures : HList captured)
    (args : HList supplied) {loans : List Nat} {lent : List RuntimeValue}
    (lends : lendMutable supplied.mutable (HList.entries args) loans = some (lent, [])) :
    lendArguments full (weave.compose captures args) loans =
        some (weave.composeList (HList.encode captures) lent).toArray ∧
      invocationSpec executable (closureOf handle weave.mask typeInstantiation captures).encode lent =
        functionSpecAt executable handle typeInstantiation
          (weave.composeList (HList.encode captures) lent).toArray := by
  rw [← NRow.lend_lentFlat supplied suppliedFlat args loans] at lends
  refine ⟨?_, ?_⟩
  · unfold lendArguments
    rw [weave.lend_compose capturedPlain captures args loans, lends]
    rfl
  · simp only [invocationSpec, ClosureValue.encode, closureOf]
    rw [weave.compose_mask _ _ (HList.encode_length captured captures)
      (NRow.lend_length supplied false args loans _ lends)]

end

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

/-- The woven row of captures and arguments lending as their encodings lends
as the woven encodings. -/
theorem lendArguments_weave {full captured supplied : NRow}
    (weave : Weave full captured supplied) (capturedPlain : captured.lendsEncoded = true)
    (suppliedPlain : supplied.lendsEncoded = true) (captures : HList captured)
    (args : HList supplied) :
    lendArguments full (weave.compose captures args) [] =
      some (weave.composeList (HList.encode captures) (HList.encode args)).toArray := by
  unfold lendArguments
  rw [weave.lend_compose capturedPlain, NRow.lend_lendsEncoded supplied suppliedPlain]
  rfl

/-- A woven row of captures and arguments lending as their encodings holds no
table. -/
theorem Denote.Weave.tables_nil {full captured supplied : NRow} (weave : Weave full captured supplied)
    (capturedPlain : captured.lendsEncoded = true) (suppliedPlain : supplied.lendsEncoded = true)
    (prophecies : Bool) (captures : HList captured) (args : HList supplied) :
    HList.tables prophecies full (weave.compose captures args) = [] := by
  rw [weave.tables_compose (NRow.tableFree_of_lendsEncoded capturedPlain),
    HList.tables_eq_nil prophecies supplied (NRow.tableFree_of_lendsEncoded suppliedPlain)]

/-- A woven row of captures lending as their encodings and flat arguments
holds no table. -/
theorem Denote.Weave.tables_nil_of_lentFlat {full captured supplied : NRow}
    (weave : Weave full captured supplied) (capturedPlain : captured.lendsEncoded = true)
    (suppliedFlat : supplied.lentFlat = true) (prophecies : Bool) (captures : HList captured)
    (args : HList supplied) :
    HList.tables prophecies full (weave.compose captures args) = [] := by
  rw [weave.tables_compose (NRow.tableFree_of_lendsEncoded capturedPlain),
    HList.tables_eq_nil_of_lentFlat prophecies supplied suppliedFlat]

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

/-- The invocation of a literal closure is that of the function value
naming its target, capturing nothing, at the woven arguments. -/
theorem invocationSpec_closureOf_woven {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    [Skolems unit] {handle : FunctionHandle}
    {full captured supplied : NRow} (weave : Weave full captured supplied)
    (typeInstantiation : Array (TypeId × TypeId)) (captures : HList captured)
    (arguments : List RuntimeValue) (lengths : arguments.length = supplied.length) :
    invocationSpec executable (closureOf handle weave.mask typeInstantiation captures).encode
        arguments =
      invocationSpec executable (closureOf handle (Weave.supplying full).mask typeInstantiation
        (σs := .nil) ()).encode (weave.composeList (HList.encode captures) arguments) := by
  have woven := weave.composeList_length (HList.encode captures) arguments
    (HList.encode_length captured captures) lengths
  simp only [invocationSpec, ClosureValue.encode, closureOf]
  rw [weave.compose_mask _ _ (HList.encode_length captured captures) lengths,
    (Weave.supplying full).compose_mask _ _ (HList.encode_length .nil ()) woven,
    HList.encode_nil, Weave.composeList_supplying]

/-- `aborts_of` of a literal closure, as of the function value naming its
target at the woven arguments. -/
theorem abortsOf_closureOf_woven {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    [Skolems unit] {handle : FunctionHandle}
    {full captured supplied : NRow} (weave : Weave full captured supplied)
    (typeInstantiation : Array (TypeId × TypeId)) (captures : HList captured)
    (arguments : List RuntimeValue) (lengths : arguments.length = supplied.length)
    (state : Denote.Memory unit) :
    AbortsOf executable (closureOf handle weave.mask typeInstantiation captures).encode
        arguments state =
      AbortsOf executable (closureOf handle (Weave.supplying full).mask typeInstantiation
        (σs := .nil) ()).encode (weave.composeList (HList.encode captures) arguments) state := by
  simp only [AbortsOf, invocationSpec_closureOf_woven weave typeInstantiation captures arguments
    lengths]

/-- `ensures_of` of a literal closure, as of the function value naming its
target at the woven arguments. -/
theorem ensuresOf_closureOf_woven {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    [Skolems unit] {handle : FunctionHandle}
    {full captured supplied : NRow} (weave : Weave full captured supplied)
    (typeInstantiation : Array (TypeId × TypeId)) (captures : HList captured)
    (arguments : List RuntimeValue) (lengths : arguments.length = supplied.length)
    (results : Array RuntimeValue) (pre post : Denote.Memory unit) :
    EnsuresOf executable (closureOf handle weave.mask typeInstantiation captures).encode
        arguments results pre post =
      EnsuresOf executable (closureOf handle (Weave.supplying full).mask typeInstantiation
        (σs := .nil) ()).encode (weave.composeList (HList.encode captures) arguments) results
        pre post := by
  simp only [EnsuresOf, invocationSpec_closureOf_woven weave typeInstantiation captures arguments
    lengths]

/-- `aborts_of` of a literal closure capturing a value, as of the function
value naming its target at the woven arguments. -/
theorem abortsOf_closureOf_captured {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    [Skolems unit] {handle : FunctionHandle}
    {full : NRow} {τ : NTy} {captured supplied : NRow}
    (weave : Weave full (.cons τ captured) supplied)
    (typeInstantiation : Array (TypeId × TypeId)) (captures : HList (.cons τ captured))
    (arguments : List RuntimeValue) (lengths : arguments.length = supplied.length)
    (state : Denote.Memory unit) :
    AbortsOf executable (closureOf handle weave.mask typeInstantiation captures).encode
        arguments state =
      AbortsOf executable (closureOf handle (Weave.supplying full).mask typeInstantiation
        (σs := .nil) ()).encode (weave.composeList (HList.encode captures) arguments) state :=
  abortsOf_closureOf_woven weave typeInstantiation captures arguments lengths state

/-- `ensures_of` of a literal closure capturing a value, as of the function
value naming its target at the woven arguments. -/
theorem ensuresOf_closureOf_captured {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    [Skolems unit] {handle : FunctionHandle}
    {full : NRow} {τ : NTy} {captured supplied : NRow}
    (weave : Weave full (.cons τ captured) supplied)
    (typeInstantiation : Array (TypeId × TypeId)) (captures : HList (.cons τ captured))
    (arguments : List RuntimeValue) (lengths : arguments.length = supplied.length)
    (results : Array RuntimeValue) (pre post : Denote.Memory unit) :
    EnsuresOf executable (closureOf handle weave.mask typeInstantiation captures).encode
        arguments results pre post =
      EnsuresOf executable (closureOf handle (Weave.supplying full).mask typeInstantiation
        (σs := .nil) ()).encode (weave.composeList (HList.encode captures) arguments) results
        pre post :=
  ensuresOf_closureOf_woven weave typeInstantiation captures arguments lengths results pre post

/-- `ensures_of` of a closure whose target, weave, and captures a proof sees,
at reference-free arguments and result, is a run of the target's prophetic
meaning at a frame coherent with the closure's instantiation. -/
theorem ensuresOf_closureOf {unit : ValidatedUnit} {executable : ExecutableUnit unit} [Skolems unit]
    {handle : FunctionHandle}
    {full captured supplied : NRow} (weave : Weave full captured supplied)
    (capturedPlain : captured.lendsEncoded = true) (suppliedPlain : supplied.lendsEncoded = true)
    {τ : NTy} (resultPlain : τ.lendsEncoded = true) (typeInstantiation : Array (TypeId × TypeId))
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
    simp only [ResultShape.lend, NTy.lend_lendsEncoded τ resultPlain]
  have prophesied : ResultShape.lend (.one τ) true result [] = some #[τ.encode result] := by
    simp only [ResultShape.lend, NTy.lend_lendsEncoded τ resultPlain]
  have none : ∀ prophecies, HList.tables prophecies full (weave.compose captures args) = [] :=
    fun prophecies => by
      rw [weave.tables_compose (NRow.tableFree_of_lendsEncoded capturedPlain),
        HList.tables_eq_nil prophecies supplied (NRow.tableFree_of_lendsEncoded suppliedPlain)]
  refine ⟨coherent, start, [], _, _, exit, [], _, Encloses.of_nil globals (none false), admissible,
    lendArguments_weave weave capturedPlain suppliedPlain captures args, runs, lent, prophesied,
    ?_, ReturnedEncloses.of_nil encoded (by
      rw [none, ResultShape.tables_eq_nil true (.one τ) (by
        simpa [ResultShape.row, NRow.tableFree] using NTy.tableFree_of_lendsEncoded resultPlain)]
      rfl), agree⟩
  exact (weave.argumentsResolve_compose (NRow.refFree_of_lendsEncoded capturedPlain) captures args
      [] _ _).mpr
    (argumentsResolve_refFree supplied (NRow.refFree_of_lendsEncoded suppliedPlain) args _ _)

/-- `aborts_of` of a closure whose target, weave, and captures a proof sees,
at reference-free arguments, is an abort of the target's prophetic meaning at
a frame coherent with the closure's instantiation. -/
theorem abortsOf_closureOf {unit : ValidatedUnit} {executable : ExecutableUnit unit} [Skolems unit]
    {handle : FunctionHandle}
    {full captured supplied : NRow} (weave : Weave full captured supplied)
    (capturedPlain : captured.lendsEncoded = true) (suppliedPlain : supplied.lendsEncoded = true)
    (shape : ResultShape) (typeInstantiation : Array (TypeId × TypeId))
    (coherent : Coherent unit handle typeInstantiation)
    (captures : HList captured) (args : HList supplied) {state : Memory unit}
    (aborts : AbortsOf executable (closureOf handle weave.mask typeInstantiation captures).encode
      (HList.encode args) state) :
    ∃ failure, (propheticMeaning executable typeInstantiation handle full shape
      (weave.compose captures args)).aborts state failure := by
  obtain ⟨start, failure, ⟨globals, admissible⟩, runs⟩ := aborts
  rw [invocationSpec_closureOf] at runs
  refine ⟨failure, coherent, start, [], _, Encloses.of_nil globals ?_, admissible,
    lendArguments_weave weave capturedPlain suppliedPlain captures args, runs⟩
  rw [weave.tables_compose (NRow.tableFree_of_lendsEncoded capturedPlain),
    HList.tables_eq_nil false supplied (NRow.tableFree_of_lendsEncoded suppliedPlain)]

/-- `ensures_of` of a closure whose target is verified: where what the
target's theorem assumes and its precondition hold, what its contract
ensures of the run, and that no condition under which it must abort held. -/
theorem ensuresOf_closureOf_verified {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    [Skolems unit] {handle : FunctionHandle}
    {full captured supplied : NRow} (weave : Weave full captured supplied)
    (capturedPlain : captured.lendsEncoded = true) (suppliedPlain : supplied.lendsEncoded = true)
    {τ : NTy} (resultPlain : τ.lendsEncoded = true) (typeInstantiation : Array (TypeId × TypeId))
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
      contract.frame (weave.compose captures args) pre result post ∧
      ¬contract.mustAbort (weave.compose captures args) pre :=
  (verified _ pre assumed permitted).1 result post
    (ensuresOf_closureOf weave capturedPlain suppliedPlain resultPlain typeInstantiation coherent
      captures args result ensures)

/-- `aborts_of` of a closure whose target is verified: where what the
target's theorem assumes and its precondition hold, a failure its contract
permits. -/
theorem abortsOf_closureOf_verified {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    [Skolems unit] {handle : FunctionHandle}
    {full captured supplied : NRow} (weave : Weave full captured supplied)
    (capturedPlain : captured.lendsEncoded = true) (suppliedPlain : supplied.lendsEncoded = true)
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
  obtain ⟨failure, runs⟩ := abortsOf_closureOf weave capturedPlain suppliedPlain shape
    typeInstantiation coherent captures args aborts
  exact ⟨failure, (verified _ state assumed permitted).2.1 failure runs⟩

/-- The arguments of a row lending as its encodings lend so, and only under
no loans. -/
private theorem lendArguments_lendsEncoded {row : NRow} (plain : row.lendsEncoded = true)
    {args : HList row} {loans : List Nat} {arguments : Array RuntimeValue}
    (lent : lendArguments row args loans = some arguments) :
    loans = [] ∧ arguments = (HList.encode args).toArray := by
  unfold lendArguments at lent
  rw [NRow.lend_lendsEncoded row plain false args loans] at lent
  cases loans with
  | nil => exact ⟨rfl, (Option.some.inj lent).symm⟩
  | cons _ _ => exact absurd lent (by simp)

/-- A run of a target that takes and returns no reference is a run of the
function value naming it: `ensures_of` of that value at the call's
arguments. -/
theorem ensuresOf_of_run {unit : ValidatedUnit} {executable : ExecutableUnit unit} [Skolems unit]
    {handle : FunctionHandle}
    {typeInstantiation : Array (TypeId × TypeId)} {full : NRow} (fullPlain : full.lendsEncoded = true)
    {shape : ResultShape} (shapePlain : shape.row.lendsEncoded = true) {args : HList full}
    {result : shape.carrier} {pre final : Memory unit}
    (runs : (propheticMeaning executable typeInstantiation handle full shape args).ok pre result final) :
    EnsuresOf executable (closureOf handle (Weave.supplying full).mask typeInstantiation
      (σs := .nil) ()).encode (HList.encode args) ((resultCodec shape).encode result) pre final := by
  obtain ⟨-, start, loans, arguments, results, exit, returnedLoans, prophecyRow, globals, admissible,
    lent, ran, resultsLent, prophecyLent, -, encoded, agree⟩ := runs
  obtain ⟨rfl, rfl⟩ := lendArguments_lendsEncoded fullPlain lent
  rw [ResultShape.lend_refFree_view shape (NRow.refFree_of_lendsEncoded shapePlain), resultsLent]
    at prophecyLent
  have sameRow : prophecyRow = results := (Option.some.inj prophecyLent).symm
  subst prophecyRow
  have returned : results = (resultCodec shape).encode result := by
    cases shape with
    | none =>
        cases returnedLoans with
        | nil => exact (Option.some.inj resultsLent).symm
        | cons _ _ => exact absurd resultsLent (by simp [ResultShape.lend])
    | one τ =>
        have resultPlain : τ.lendsEncoded = true := (NRow.lendsEncoded_singleton τ).mp shapePlain
        simp only [ResultShape.lend, NTy.lend_lendsEncoded τ resultPlain] at resultsLent
        cases returnedLoans with
        | nil => exact (Option.some.inj resultsLent).symm
        | cons _ _ => exact absurd resultsLent (by simp)
  subst returned
  refine ⟨start, exit, ⟨globals.stores, admissible⟩, ?_, encoded.stores, agree⟩
  rw [invocationSpec_closureOf (Weave.supplying full) typeInstantiation () args,
    HList.encode_nil, Weave.composeList_supplying]
  exact ran

/-- An abort of a target that takes no reference is an abort of the function
value naming it: `aborts_of` of that value at the call's arguments. -/
theorem abortsOf_of_abort {unit : ValidatedUnit} {executable : ExecutableUnit unit} [Skolems unit]
    {handle : FunctionHandle}
    {typeInstantiation : Array (TypeId × TypeId)} {full : NRow} (fullPlain : full.lendsEncoded = true)
    {shape : ResultShape} {args : HList full} {pre : Memory unit} {failure : Failure}
    (aborts : (propheticMeaning executable typeInstantiation handle full shape args).aborts pre failure) :
    AbortsOf executable (closureOf handle (Weave.supplying full).mask typeInstantiation
      (σs := .nil) ()).encode (HList.encode args) pre := by
  obtain ⟨-, start, loans, arguments, globals, admissible, lent, ran⟩ := aborts
  obtain ⟨rfl, rfl⟩ := lendArguments_lendsEncoded fullPlain lent
  refine ⟨start, failure, ⟨globals.stores, admissible⟩, ?_⟩
  rw [invocationSpec_closureOf (Weave.supplying full) typeInstantiation () args,
    HList.encode_nil, Weave.composeList_supplying]
  exact ran

/-- Where a target that takes and returns no reference is called, the run is
one of the function value naming the target: the continuations may assume
its `ensures_of` where the call returns and its `aborts_of` where it aborts. -/
theorem wp_named {unit : ValidatedUnit} {executable : ExecutableUnit unit} [Skolems unit]
    {handle : FunctionHandle}
    {typeInstantiation : Array (TypeId × TypeId)} {full : NRow} {shape : ResultShape}
    (fullPlain : full.lendsEncoded = true) (shapePlain : shape.row.lendsEncoded = true)
    {args : HList full} {ensures : shape.carrier → Memory unit → Prop} {aborts : Failure → Prop}
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
      named.1 result final execution (ensuresOf_of_run fullPlain shapePlain execution),
    fun error execution => named.2.1 error execution (abortsOf_of_abort fullPlain execution),
    named.2.2⟩

/-- A result's values as a behavioral predicate reads a run's
(`readResults`): a returned mutable reference as its current value, each
physical. -/
def Denote.ResultShape.current : (shape : ResultShape) → shape.carrier → Array RuntimeValue
  | .none, _ => #[]
  | .one (.ref σ), value => #[σ.erase value.1]
  | .one τ, value => #[τ.erase value]

/-- A result whose returned mutable reference has its current value as its
prophecy, as the reference's death leaves it (`Term.resolve`). -/
def Denote.ResultShape.Settled : (shape : ResultShape) → shape.carrier → Prop
  | .one (.ref _), value => value.1 = value.2
  | _, _ => True

-- The solver reads a lending call's named run at concrete rows and results.
attribute [vir_denote_norm] Denote.NRow.mutable Denote.HList.entries Denote.HList.finals
  Denote.ResultShape.current Denote.ResultShape.Settled

/-- A result read as values holds no loan. -/
theorem Denote.ResultShape.current_plain : (shape : ResultShape) → (result : shape.carrier) →
    ∀ value ∈ shape.current result, SemanticOperations.Plain value
  | .none, _, _, member => by simp [ResultShape.current] at member
  | .one τ, result, value, member => by
      cases τ <;> simp only [ResultShape.current, List.mem_toArray, List.mem_singleton] at member <;>
        subst member <;> exact NTy.erase_plain _ _

/-- A settled result lends its prophecies as its current values. -/
theorem Denote.ResultShape.lend_settled : (shape : ResultShape) → shape.row.lentFlat = true →
    (result : shape.carrier) → shape.Settled result → (loans : List Nat) →
    shape.lend true result loans = shape.lend false result loans
  | .none, _, _, _, loans => by cases loans <;> rfl
  | .one τ, flat, result, settled, loans => by
      cases τ with
      | ref σ =>
          simp only [ResultShape.Settled] at settled
          cases loans <;> simp [ResultShape.lend, NTy.lend, settled]
      | _ =>
          simp only [NRow.lentFlat, Bool.and_true] at flat
          simp only [ResultShape.lend, NTy.lend_refFree _ (NTy.refFree_of_lendsEncoded flat)]

/-- A result lent at its current values reads as them. -/
theorem Denote.ResultShape.readResults_lend : (shape : ResultShape) → shape.row.lentFlat = true →
    (result : shape.carrier) → (loans : List Nat) → {results : Array RuntimeValue} →
    shape.lend false result loans = some results →
    readResults shape.row.mutable results.toList = (shape.current result).toList
  | .none, _, _, loans, results, lent => by
      cases loans <;> simp only [ResultShape.lend, Option.some.injEq, reduceCtorEq] at lent
      subst lent
      rfl
  | .one τ, flat, result, loans, results, lent => by
      cases τ with
      | ref σ =>
          match loans, lent with
          | [], lent => simp [ResultShape.lend, NTy.lend] at lent
          | loan :: rest, lent =>
              cases rest <;> simp only [ResultShape.lend, NTy.lend, Option.some.injEq,
                reduceCtorEq] at lent
              subst lent
              simp only [NRow.mutable, List.toList_toArray, readResults, ite_false,
                RuntimeValue.observe, RuntimeValue.observe_of_plain (NTy.erase_plain σ result.1),
                ResultShape.current]
      | _ =>
          simp only [NRow.lentFlat, Bool.and_true] at flat
          simp only [ResultShape.lend, NTy.lend_refFree _ (NTy.refFree_of_lendsEncoded flat)] at lent
          cases loans <;> simp only [Option.some.injEq, reduceCtorEq] at lent
          subst lent
          rfl

/-- The arguments of a flat row lend as its entries, each mutable one under
the next loan. -/
theorem lendArguments_lentFlat {row : NRow} (flat : row.lentFlat = true) {args : HList row}
    {loans : List Nat} {arguments : Array RuntimeValue}
    (lent : lendArguments row args loans = some arguments) :
    ∃ lentRow, lendMutable row.mutable (HList.entries args) loans = some (lentRow, []) ∧
      arguments = lentRow.toArray := by
  unfold lendArguments at lent
  rw [NRow.lend_lentFlat row flat args loans] at lent
  match lends : lendMutable row.mutable (HList.entries args) loans, lent with
  | some (lentRow, []), lent => exact ⟨lentRow, rfl, (Option.some.inj lent).symm⟩
  | some (_, _ :: _), lent => exact absurd lent (by simp)
  | none, lent => exact absurd lent (by simp)

/-- A run of a target taking and returning mutable references, or values
holding none, whose returned references are settled, is a run of the
function value naming it: `ensures_of` of that value at the call's entry
values, its results' current values, and its references' final values. -/
theorem ensuresOfMut_of_run {unit : ValidatedUnit} {executable : ExecutableUnit unit} [Skolems unit]
    {handle : FunctionHandle} {typeInstantiation : Array (TypeId × TypeId)} {full : NRow}
    (fullFlat : full.lentFlat = true) {shape : ResultShape} (shapeFlat : shape.row.lentFlat = true)
    {args : HList full} {result : shape.carrier} {pre final : Memory unit}
    (runs : (propheticMeaning executable typeInstantiation handle full shape args).ok pre result final)
    (settled : shape.Settled result) :
    EnsuresOfMut executable (closureOf handle (Weave.supplying full).mask typeInstantiation
        (σs := .nil) ()).encode full.mutable shape.row.mutable (HList.entries args)
      (shape.current result) (HList.finals args) pre final := by
  obtain ⟨-, start, loans, arguments, results, exit, returnedLoans, prophecyRow, globals, admissible,
    lent, ran, resultsLent, prophecyLent, resolves, encoded, agree⟩ := runs
  rw [ResultShape.lend_settled shape shapeFlat result settled, resultsLent] at prophecyLent
  have sameRow : prophecyRow = results := (Option.some.inj prophecyLent).symm
  subst sameRow
  obtain ⟨lentRow, lends, rfl⟩ := lendArguments_lentFlat fullFlat lent
  obtain ⟨-, invocation⟩ := invocationSpec_closureOf_lent (Weave.supplying full) rfl fullFlat
    typeInstantiation () args (executable := executable) (handle := handle) lends
  refine ⟨start, loans, lentRow, prophecyRow, exit, globals.stores, admissible, lends, ?_, ?_,
    (argumentsResolve_finals full fullFlat args loans _ _).mp resolves, encoded.stores, agree⟩
  · rw [invocation, HList.encode_nil, Weave.composeList_supplying]
    exact ran
  · rw [ResultShape.readResults_lend shape shapeFlat result returnedLoans resultsLent,
      Array.toArray_toList]

/-- An abort of a target taking mutable references, or values holding none,
is an abort of the function value naming it: `aborts_of` of that value at
the call's entry values. -/
theorem abortsOfMut_of_abort {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    [Skolems unit] {handle : FunctionHandle} {typeInstantiation : Array (TypeId × TypeId)}
    {full : NRow} (fullFlat : full.lentFlat = true) {shape : ResultShape} {args : HList full}
    {pre : Memory unit} {failure : Failure}
    (aborts : (propheticMeaning executable typeInstantiation handle full shape args).aborts pre failure) :
    AbortsOfMut executable (closureOf handle (Weave.supplying full).mask typeInstantiation
      (σs := .nil) ()).encode full.mutable (HList.entries args) pre := by
  obtain ⟨-, start, loans, arguments, globals, admissible, lent, ran⟩ := aborts
  obtain ⟨lentRow, lends, rfl⟩ := lendArguments_lentFlat fullFlat lent
  obtain ⟨-, invocation⟩ := invocationSpec_closureOf_lent (Weave.supplying full) rfl fullFlat
    typeInstantiation () args (executable := executable) (handle := handle) lends
  refine ⟨start, loans, lentRow, failure, globals.stores, admissible, lends, ?_⟩
  rw [invocation, HList.encode_nil, Weave.composeList_supplying]
  exact ran

/-- Where a target taking mutable references, or values holding none, and
returning one or a value holding none is called, the run is one of the
function value naming the target: the continuations may assume its
`ensures_of` where the call returns with its returned reference settled, and
its `aborts_of` where it aborts. -/
theorem wp_named_mut {unit : ValidatedUnit} {executable : ExecutableUnit unit} [Skolems unit]
    {handle : FunctionHandle} {typeInstantiation : Array (TypeId × TypeId)} {full : NRow}
    {shape : ResultShape} (fullFlat : full.lentFlat = true) (shapeFlat : shape.row.lentFlat = true)
    {args : HList full} {ensures : shape.carrier → Memory unit → Prop} {aborts : Failure → Prop}
    {initial : Memory unit}
    (named : wp (propheticMeaning executable typeInstantiation handle full shape args)
      (fun result final =>
        (shape.Settled result →
          EnsuresOfMut executable (ClosureValue.encode (closureOf handle
            (Weave.supplying full).mask typeInstantiation (σs := .nil) ()))
            full.mutable shape.row.mutable (HList.entries args) (shape.current result)
            (HList.finals args) initial final) →
        ensures result final)
      (fun error =>
        AbortsOfMut executable (ClosureValue.encode (closureOf handle
          (Weave.supplying full).mask typeInstantiation (σs := .nil) ()))
          full.mutable (HList.entries args) initial → aborts error)
      initial) :
    wp (propheticMeaning executable typeInstantiation handle full shape args) ensures aborts initial :=
  ⟨fun result final execution =>
      named.1 result final execution (ensuresOfMut_of_run fullFlat shapeFlat execution),
    fun error execution => named.2.1 error execution (abortsOfMut_of_abort fullFlat execution),
    named.2.2⟩

end

end VerexIR.Proofs

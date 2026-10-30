-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Behavior
import VerexIR.Proofs.Completeness
import VerexIR.Semantics.LoanIndependence
import VerexIR.Semantics.Preservation

/-!
# Invoking a function value a proof does not see

An invocation of a closure whose target, weave, and captures a proof does
not see denotes `closureMeaning`, undefined where a run of the target
returns results that do not decode at the invocation's type. Preservation
excludes that when the closure, its arguments, and global memory are typed
(`doc/historical/static-typing.md`, "Phase 5"); the invocation then has the weakest
precondition of its runs and aborts, which the behavioral predicates state.
Runs from the starts these predicates range over agree up to the loans they
mint (`runs_mirror`), so a run that returns refutes `aborts_of` and names
`result_of`.
-/

namespace VerexIR.Proofs

open VerexIR.Validation

/-- Global memory typed: every slot at its key's type, holding no loan. -/
def GlobalsTyped (unit : ValidatedUnit) (state : RuntimeState) : Prop :=
  ∀ slot ∈ state.globals.entries, ∃ ns type,
    unit.namespaces[slot.key.namespaceId.index]? = some ns ∧
      Resolves ns.tables #[] slot.key.typeId type ∧ HasType unit (fun _ => none) slot.value type

mutual
/-- A value typed under no loan holds none. -/
theorem HasType.plain {unit : ValidatedUnit} {value : RuntimeValue} {type : SemTy} :
    HasType unit (fun _ => none) value type → SemanticOperations.Plain value
  | .unit => .unit
  | .bool value => .bool value
  | .character value _ => .character value
  | .string value => .string value
  | .bytes value => .bytes value
  | .integer value _ _ _ => .integer value
  | .address value => .address value
  | .signer value => .signer value
  | .tuple values _ elements =>
      .tuple values fun value member => HasTypes.plain elements value (Array.mem_toList_iff.mpr member)
  | .vector values _ _ _ elements =>
      .vector values fun value member =>
        HasTypeEach.plain elements value (Array.mem_toList_iff.mpr member)
  | .nominal source variant fields _ _ _ _ _ _ _ _ _ _ typed =>
      .nominal source variant fields fun value member =>
        HasTypes.plain typed value (Array.mem_toList_iff.mpr member)
  | .closure function mask instantiation captures _ _ _ _ _ _ _ _ _ _ _ _ typed _ _ =>
      .closure function mask instantiation captures fun value member =>
        HasTypes.plain typed value (Array.mem_toList_iff.mpr member)
  | .borrow _ _ _ loan_eq _ => nomatch loan_eq
  | .shared _ _ typed => HasType.plain typed
  | .dead _ => .unit
  | .hole _ _ _ loan_eq _ _ => nomatch loan_eq
  | .unitEmptyTuple => .unit
  | .emptyTupleUnit => .tuple #[] fun _ member => nomatch member
  | .ghost contents _ plain => .ghost contents plain

theorem HasTypes.plain {unit : ValidatedUnit} {values : List RuntimeValue} {types : List SemTy} :
    HasTypes unit (fun _ => none) values types → ∀ value ∈ values, SemanticOperations.Plain value
  | .nil, _, member => nomatch member
  | .cons head _, _, .head _ => HasType.plain head
  | .cons _ tail, value, .tail _ member => HasTypes.plain tail value member

theorem HasTypeEach.plain {unit : ValidatedUnit} {values : List RuntimeValue} {type : SemTy} :
    HasTypeEach unit (fun _ => none) values type →
      ∀ value ∈ values, SemanticOperations.Plain value
  | .nil, _, member => nomatch member
  | .cons head _, _, .head _ => HasType.plain head
  | .cons _ tail, value, .tail _ member => HasTypeEach.plain tail value member
end

/-- Typed global memory holds no loan. -/
theorem GlobalsTyped.plain {unit : ValidatedUnit} {state : RuntimeState}
    (typed : GlobalsTyped unit state) :
    ∀ slot ∈ state.globals.entries, SemanticOperations.Plain slot.value := fun slot member =>
  let ⟨_, _, _, _, slotTyped⟩ := typed slot member
  HasType.plain slotTyped

/-- A memory whose runtime encodings are typed: every slot at its key's
type. -/
def MemoryTyped (unit : ValidatedUnit) (memory : Denote.Memory unit) : Prop :=
  ∀ start : RuntimeState, Denote.Encodes unit memory start.globals →
    ∃ globals : GlobalMap, globals.normalize unit = start.globals ∧
      GlobalsTyped unit { start with globals := globals }

/-- A memory's runtime encoding holds no loan: every slot is the encoding of
a native value. -/
theorem Denote.Encodes.plain {unit : ValidatedUnit} {memory : Denote.Memory unit}
    {globals : GlobalMap} (encodes : Denote.Encodes unit memory globals) :
    ∀ slot ∈ globals.entries, SemanticOperations.Plain slot.value := by
  intro slot member
  have found := encodes.2 slot.key.namespaceId slot.key.typeId slot.key.key
  rw [show (⟨slot.key.namespaceId, slot.key.typeId, slot.key.key⟩ : GlobalKey) = slot.key from rfl,
    encodes.1.lookup_of_mem member] at found
  split at found
  · obtain ⟨value, -, encoded⟩ := Option.map_eq_some_iff.mp found.symm
    rw [← encoded]
    exact Denote.ResourceType.encode_plain _ value
  · cases found

/-- A function value that runs on arguments of `parameters` and returns
results of `results`: its target, frame, and captures as a closure's typing
states them (`HasType.closure`), with the target's result row exact. -/
def ClosureTyped (unit : ValidatedUnit) (function : FunctionHandle) (mask : Nat)
    (typeInstantiation : Array (TypeId × TypeId)) (captures : Array RuntimeValue)
    (parameters results : List SemTy) : Prop :=
  ∃ targetNs declaration arguments allParameters,
    unit.namespaces[function.namespaceId.index]? = some targetNs ∧
    targetNs.functions[function.functionId.index]? = some declaration ∧
    StaticTyping.signatureTypes? targetNs declaration
      (frameEnv declaration.signature.generics arguments) = some (allParameters, results) ∧
    FrameInstantiation targetNs (requiredAt unit function) typeInstantiation
      (frameEnv declaration.signature.generics arguments) ∧
    mask < 2 ^ declaration.signature.parameters.size ∧
    (∃ represented : Array RuntimeValue,
      represented.map (SemanticOperations.normalize unit) = captures ∧
      HasTypes unit (fun _ => none) represented.toList
        (ClosureMask.extract mask true allParameters)) ∧
    parameters = ClosureMask.extract mask false allParameters

/-- A function's results hold no hole outside a borrow. -/
theorem EvalFunction_holeFree {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    {handle : FunctionHandle}
    {typeInstantiation : Array (TypeId × TypeId)} {initial final : RuntimeState}
    {arguments : Array RuntimeValue} {outcome : Outcome}
    (step : BigStep.EvalFunction executable handle typeInstantiation initial arguments final outcome) :
    outcome.holeFree = true := by
  cases step <;> assumption

section
open Denote
variable {unit : Validation.ValidatedUnit} [Skolems unit]

/-- Results of `results` that hold no hole decode at a result shape, read up
to normalization: the values a run returns represent the results its
meaning returns (`functionRuns`). -/
def RowDecodes (unit : ValidatedUnit) [Skolems unit] (shape : ResultShape)
    (results : List SemTy) : Prop :=
  ∀ (loans : LoanTypes) (values : Array RuntimeValue), HasTypes unit loans values.toList results →
    (Outcome.returned values).holeFree = true →
    ∃ result, shape.lend false result [] = some (values.map (SemanticOperations.normalize unit))

/-- A result without references lends its prophecies as its values. -/
theorem ResultShape.lend_refFree : (shape : ResultShape) → shape.row.refFree = true →
    (result : shape.carrier) → shape.lend true result [] = shape.lend false result []
  | .none, _, _ => rfl
  | .one τ, free, result => by
      simp only [NRow.refFree, Bool.and_true] at free
      simp only [ResultShape.lend, NTy.lend_refFree τ free]

/-- Arguments without references resolve nothing. -/
theorem argumentsResolve_refFree : (σs : NRow) → σs.refFree = true → (args : HList σs) →
    (returned : Array RuntimeValue) → (exports : List (Nat × RuntimeValue)) →
    argumentsResolve σs args [] returned exports
  | .nil, _, _, _, _ => trivial
  | .cons _ rest, free, args, returned, exports => by
      simp only [NRow.refFree, Bool.and_eq_true] at free
      exact (argumentsResolve_cons_refFree free.1 args [] returned exports).mpr
        (argumentsResolve_refFree rest free.2 args.2 returned exports)

/-- A result without references lends no loan. -/
theorem ResultShape.lend_refFree_loans : (shape : ResultShape) → shape.row.refFree = true →
    {prophecies : Bool} → {result : shape.carrier} → {loans : List Nat} →
    {values : Array RuntimeValue} → shape.lend prophecies result loans = some values → loans = []
  | .none, _, _, _, [], _, _ => rfl
  | .none, _, _, _, _ :: _, _, lent => by simp [ResultShape.lend] at lent
  | .one τ, free, prophecies, result, loans, _, lent => by
      simp only [NRow.refFree, Bool.and_true] at free
      simp only [ResultShape.lend, NTy.lend_refFree τ free prophecies result loans] at lent
      cases loans with
      | nil => rfl
      | cons _ _ => simp at lent

/-- Arguments without references lend as their encodings, under no loan. -/
theorem lendArguments_refFree {σs : NRow} (free : σs.refFree = true) {args : HList σs}
    {loans : List Nat} {supplied : Array RuntimeValue}
    (lent : lendArguments σs args loans = some supplied) :
    loans = [] ∧ supplied = (HList.encode args).toArray := by
  unfold lendArguments at lent
  rw [NRow.lend_refFree σs free false args loans] at lent
  cases loans with
  | nil => simp only [Option.some.injEq] at lent; exact ⟨rfl, lent.symm⟩
  | cons _ _ => simp at lent

/-- A run of a typed closure on typed arguments from typed global memory has
no undefined outcome: every run of its target returns results that decode
at the invocation's type. -/
theorem closureRun_defined {unit : ValidatedUnit} (executable : ExecutableUnit unit)
    [Skolems unit] (natives : NativesTyped executable)
    (closure : ClosureValue) {σs : NRow} {shape : ResultShape} (args : HList σs)
    (free : σs.refFree = true) (shapeFree : shape.row.refFree = true)
    {parameters results : List SemTy}
    (typed : ClosureTyped unit closure.function closure.mask closure.typeInstantiation
      closure.captures parameters results)
    (decodes : RowDecodes unit shape results) {initial : Memory unit} :
    ¬(closureRun executable closure σs shape args).undefined initial := by
  rintro ⟨start, loans, supplied, arguments, values, exit, -, -, lend_eq, -, run, undecoded⟩
  have lent : lendArguments σs args [] = some (HList.encode args).toArray := by
    unfold lendArguments
    rw [NRow.lend_refFree σs free false args []]
  cases loans with
  | cons _ _ =>
      unfold lendArguments at lend_eq
      rw [NRow.lend_refFree σs free false args _] at lend_eq
      simp at lend_eq
  | nil =>
  rw [lent, Option.some.injEq] at lend_eq
  subst lend_eq
  obtain ⟨start', arguments', exit', outcome, represents, -, outcome_eq, step⟩ := run
  obtain ⟨returned, rfl, values_eq⟩ := Outcome.normalize_eq_returned outcome_eq
  obtain ⟨targetNs, declaration, semantic, allParameters, ns_eq, declaration_eq, signature_eq,
    faithful, -, -, rfl⟩ := typed
  obtain ⟨loans', inert, startTyped, argumentsTyped⟩ := represents.typed
  obtain ⟨loans'', -, -, returns⟩ := preservation natives step loans' inert targetNs declaration
    semantic allParameters results ns_eq declaration_eq faithful signature_eq
    (argumentsTyped targetNs declaration semantic allParameters results ns_eq declaration_eq
      faithful signature_eq) startTyped
  obtain ⟨result, lend_result⟩ :=
    decodes loans'' returned (returns returned rfl) (EvalFunction_holeFree step)
  rw [values_eq] at lend_result
  exact undecoded ⟨result, [], args, lend_result,
    (ResultShape.lend_refFree shape shapeFree result).trans lend_result, lent,
    argumentsResolve_refFree σs free args values _⟩

/-- The weakest precondition of such a run: what its runs and aborts
establish. -/
theorem wp_closureRun_typed {unit : ValidatedUnit} (executable : ExecutableUnit unit)
    [Skolems unit] (natives : NativesTyped executable)
    (closure : ClosureValue) {σs : NRow} {shape : ResultShape} (args : HList σs)
    (free : σs.refFree = true) (shapeFree : shape.row.refFree = true)
    {parameters results : List SemTy}
    (typed : ClosureTyped unit closure.function closure.mask closure.typeInstantiation
      closure.captures parameters results)
    (decodes : RowDecodes unit shape results) {initial : Memory unit}
    (ensures : shape.carrier → Memory unit → Prop) (aborts : Failure → Prop) :
    wp (closureRun executable closure σs shape args) ensures aborts initial ↔
      (∀ result final, (closureRun executable closure σs shape args).ok initial result final →
        ensures result final) ∧
      (∀ error, (closureRun executable closure σs shape args).aborts initial error →
        aborts error) :=
  wp_total_iff (closureRun_defined executable natives closure args free shapeFree typed decodes)

end

/-! ## Lending, as the agreement reads it

The runtime arguments a row of native arguments lends as, and the runtime
results a result shape reads as: what relates the denotation's runs to the
big-step semantics. -/

section Lending
open Denote

/-- The supplied arguments with each mutable one lent under the next loan,
as a borrow of its entry value; and the loans left. -/
def lendMutable : List Bool → List RuntimeValue → List Nat → Option (List RuntimeValue × List Nat)
  | [], [], loans => some ([], loans)
  | true :: mutable, entry :: arguments, loan :: loans =>
      (lendMutable mutable arguments loans).map fun lent => (.borrow loan entry :: lent.1, lent.2)
  | false :: mutable, argument :: arguments, loans =>
      (lendMutable mutable arguments loans).map fun lent => (argument :: lent.1, lent.2)
  | _, _, _ => none

/-- Returned values as their values: a mutable reference, where `mutable`
marks one, as its referent's value at return (`RuntimeValue.observe`); any
other result as returned. -/
def readResults : List Bool → List RuntimeValue → List RuntimeValue
  | true :: mutable, value :: values => value.observe :: readResults mutable values
  | false :: mutable, value :: values => value :: readResults mutable values
  | _, values => values

/-- The positions of a parameter row that are mutable references. -/
def Denote.NRow.mutable : NRow → List Bool
  | .nil => []
  | .cons (.ref _) rest => true :: rest.mutable
  | .cons _ rest => false :: rest.mutable

/-- A parameter row each element of which is a mutable reference or holds
none, as a function's parameters are. -/
def Denote.NRow.lentFlat : NRow → Bool
  | .nil => true
  | .cons (.ref _) rest => rest.lentFlat
  | .cons τ rest => τ.refFree && rest.lentFlat

/-- A row without references is flat. -/
theorem Denote.NRow.lentFlat_of_refFree : (row : NRow) → row.refFree = true → row.lentFlat = true
  | .nil, _ => rfl
  | .cons τ rest, free => by
      simp only [NRow.refFree, Bool.and_eq_true] at free
      have later := Denote.NRow.lentFlat_of_refFree rest free.2
      cases τ <;> simp_all [NRow.lentFlat, NTy.refFree]

/-- A reference-free result is settled. -/
theorem Denote.ResultShape.settled_of_refFree [Carriers] : (shape : ResultShape) →
    shape.row.refFree = true → (result : shape.carrier) → shape.Settled result
  | .none, _, _ => trivial
  | .one τ, free, _ => by
      cases τ <;> simp_all [ResultShape.Settled, NRow.refFree, NTy.refFree]

variable [Carriers]

/-- The values a row of arguments passes: a mutable reference's entry value,
any other argument's encoding. -/
def Denote.HList.entries : {row : NRow} → HList row → List RuntimeValue
  | .nil, _ => []
  | .cons (.ref σ) _, values => σ.encode values.1.1 :: HList.entries values.2
  | .cons τ _, values => τ.encode values.1 :: HList.entries values.2

/-- The entry values of a row of arguments hold no loan. -/
theorem Denote.HList.entries_plain : (row : NRow) → (values : HList row) →
    ∀ entry ∈ HList.entries values, SemanticOperations.Plain entry
  | .nil, _, _, member => nomatch member
  | .cons τ rest, values, entry, member => by
      cases τ <;> simp only [HList.entries, List.mem_cons] at member <;>
        rcases member with rfl | member <;>
        first | exact NTy.encode_plain _ _ | exact HList.entries_plain rest values.2 entry member

/-- Settled arguments pass the arguments' entry values. -/
theorem Denote.HList.entries_settle : (row : NRow) → (values : HList row) →
    HList.entries (HList.settle row values) = HList.entries values
  | .nil, _ => rfl
  | .cons τ rest, values => by
      have later := Denote.HList.entries_settle rest values.2
      cases τ <;> simp only [HList.settle, HList.entries, NTy.settle, HList.cons, later]

/-- Settling is idempotent. -/
theorem Denote.HList.settle_settle : (row : NRow) → (values : HList row) →
    HList.settle row (HList.settle row values) = HList.settle row values
  | .nil, _ => rfl
  | .cons τ rest, values => by
      have later := Denote.HList.settle_settle rest values.2
      cases τ <;> simp only [HList.settle, NTy.settle, HList.cons, later]

/-- A result's values as a run returns them, read as `readResults` reads
them: a returned mutable reference as its current value. -/
def Denote.ResultShape.current : (shape : ResultShape) → shape.carrier → Array RuntimeValue
  | .none, _ => #[]
  | .one (.ref σ), value => #[σ.encode value.1]
  | .one τ, value => #[τ.encode value]

/-- A result read as values holds no loan. -/
theorem Denote.ResultShape.current_plain : (shape : ResultShape) → (result : shape.carrier) →
    ∀ value ∈ shape.current result, SemanticOperations.Plain value
  | .none, _, _, member => by simp [ResultShape.current] at member
  | .one τ, result, value, member => by
      cases τ <;> simp only [ResultShape.current, List.mem_toArray, List.mem_singleton] at member <;>
        subst member <;> exact NTy.encode_plain _ _

/-- Settled results read as the same values are equal. -/
theorem Denote.ResultShape.eq_of_current : (shape : ResultShape) → {result result' : shape.carrier} →
    shape.Settled result → shape.Settled result' → shape.current result = shape.current result' →
    result = result'
  | .none, (), (), _, _, _ => rfl
  | .one τ, result, result', settled, settled', same => by
      have listed := congrArg Array.toList same
      cases τ with
      | ref σ =>
          simp only [ResultShape.current, ResultShape.Settled, List.toList_toArray,
            List.cons.injEq, and_true, NTy.encode, Codec.encode_eq_encode] at listed settled settled'
          exact Prod.ext listed (settled.symm.trans (listed.trans settled'))
      | _ =>
          simp only [ResultShape.current, List.toList_toArray, List.cons.injEq, and_true,
            NTy.encode, Codec.encode_eq_encode] at listed
          first | exact listed | rfl

end Lending

section Lending
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
          simp only [NRow.lentFlat] at flat
          cases loans with
          | nil => rfl
          | cons loan loans =>
              simp only [NRow.lend, NTy.lend, Option.bind_some, NRow.mutable, HList.entries,
                lendMutable, NRow.lend_lentFlat rest flat values.2 loans, ite_false,
                Bool.false_eq_true]
      | _ =>
          simp only [NRow.lentFlat, Bool.and_eq_true] at flat
          simp only [NRow.lend, NTy.lend_refFree _ flat.1, Option.bind_some, NRow.mutable,
            HList.entries, lendMutable, NRow.lend_lentFlat rest flat.2 values.2 loans]

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
          simp only [ResultShape.lend, NTy.lend_refFree _ flat]

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
                RuntimeValue.observe, RuntimeValue.observe_of_plain (NTy.encode_plain σ result.1),
                ResultShape.current, NTy.encode]
      | _ =>
          simp only [NRow.lentFlat, Bool.and_true] at flat
          simp only [ResultShape.lend, NTy.lend_refFree _ flat] at lent
          cases loans <;> simp only [Option.some.injEq, reduceCtorEq] at lent
          subst lent
          rfl

end Lending

section Agreement

open SemanticOperations
open BigStep

/-- Two runs from starts with the same global memory holding no loan and the
same table handles, each lending distinct unregistered loans, on arguments holding only
those loans that agree once both starts' lent loans are placed at the same
positions: the runs agree up to injective renamings of their loans. -/
theorem evalFunction_agree {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    (natives : NativesShift executable)
    {handle : FunctionHandle} {instantiation : Array (TypeId × TypeId)}
    {start start₂ final final₂ : RuntimeState} {arguments arguments₂ : Array RuntimeValue}
    {outcome outcome₂ : Outcome} {loans loans₂ : List Nat}
    (globals_eq : start₂.globals = start.globals)
    (tableHandles_eq : start₂.tableHandles = start.tableHandles)
    (plain : ∀ slot ∈ start.globals.entries, Plain slot.value)
    (admissible : Denote.Admissible start loans) (admissible₂ : Denote.Admissible start₂ loans₂)
    (length_eq : loans.length = loans₂.length)
    (within : ∀ argument ∈ arguments, argument.Within (lentInside loans start.nextLoan))
    (within₂ : ∀ argument ∈ arguments₂, argument.Within (lentInside loans₂ start₂.nextLoan))
    (arguments_eq : ∀ target,
      arguments.map (·.rename (lentRenaming loans start.nextLoan target)) =
        arguments₂.map (·.rename (lentRenaming loans₂ start₂.nextLoan target)))
    (run : EvalFunction executable handle instantiation start arguments final outcome)
    (run₂ : EvalFunction executable handle instantiation start₂ arguments₂ final₂ outcome₂) :
    ∃ ρ ρ₂, Function.Injective ρ ∧ Function.Injective ρ₂ ∧
      outcome.rename ρ = outcome₂.rename ρ₂ ∧
      final.globals.rename ρ = final₂.globals.rename ρ₂ ∧
      final.tableHandles = final₂.tableHandles := by
  obtain ⟨nodup, lent, fresh⟩ := admissible
  obtain ⟨nodup₂, lent₂, fresh₂⟩ := admissible₂
  have registered : ∀ entry ∈ start₂.globalLoans,
      entry.1 < max start.nextLoan start₂.nextLoan := fun entry member =>
    Nat.lt_of_lt_of_le (fresh₂.below entry member) (Nat.le_max_right _ _)
  obtain ⟨common, mirror, globals, tables⟩ := runs_mirror natives run
    (start₂ := { start₂ with nextLoan := max start.nextLoan start₂.nextLoan + loans.length })
    globals_eq tableHandles_eq plain nodup (fun loan member => (lent loan member).1)
    (fun loan member => (lent loan member).2) fresh (Nat.le_max_left _ _) rfl registered within
  obtain ⟨common₂, mirror₂, globals₂, tables₂⟩ := runs_mirror natives run₂
    (start₂ := { start₂ with nextLoan := max start.nextLoan start₂.nextLoan + loans.length })
    rfl rfl (by rw [globals_eq]; exact plain) nodup₂
    (fun loan member => (lent₂ loan member).1) (fun loan member => (lent₂ loan member).2) fresh₂
    (Nat.le_max_right _ _) (by rw [length_eq]) registered within₂
  rw [arguments_eq] at mirror
  obtain ⟨same, sameOutcome⟩ := Completeness.evalFunction_deterministic mirror mirror₂
  subst same
  exact ⟨_, _, lentRenaming_injective (Nat.le_max_left _ _),
    lentRenaming_injective (Nat.le_max_right _ _), sameOutcome, globals.symm.trans globals₂,
    tables.symm.trans tables₂⟩

/-- `evalFunction_agree` from starts lending nothing, on arguments holding no
loan. -/
theorem evalFunction_agree_plain {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    (natives : NativesShift executable)
    {handle : FunctionHandle} {instantiation : Array (TypeId × TypeId)}
    {start start₂ final final₂ : RuntimeState} {arguments : Array RuntimeValue}
    {outcome outcome₂ : Outcome}
    (globals_eq : start₂.globals = start.globals)
    (tableHandles_eq : start₂.tableHandles = start.tableHandles)
    (plain : ∀ slot ∈ start.globals.entries, Plain slot.value)
    (admissible : Denote.Admissible start []) (admissible₂ : Denote.Admissible start₂ [])
    (arguments_plain : ∀ argument ∈ arguments, Plain argument)
    (run : EvalFunction executable handle instantiation start arguments final outcome)
    (run₂ : EvalFunction executable handle instantiation start₂ arguments final₂ outcome₂) :
    ∃ ρ ρ₂, Function.Injective ρ ∧ Function.Injective ρ₂ ∧
      outcome.rename ρ = outcome₂.rename ρ₂ ∧
      final.globals.rename ρ = final₂.globals.rename ρ₂ ∧
      final.tableHandles = final₂.tableHandles :=
  evalFunction_agree natives globals_eq tableHandles_eq plain admissible admissible₂ rfl
    (fun argument member => RuntimeValue.within_of_plain _ (arguments_plain argument member))
    (fun argument member => RuntimeValue.within_of_plain _ (arguments_plain argument member))
    (fun _ => by
      rw [Array.map_rename_of_plain _ arguments_plain, Array.map_rename_of_plain _ arguments_plain])
    run run₂

/-- Runs of the function meaning (`functionRuns`) agree as big-step runs do
(`evalFunction_agree`): from starts with the same global memory holding no
loan and the same table handles, each lending distinct unregistered loans,
on arguments holding only those loans that agree once both starts' lent
loans are placed at the same positions, up to injective renamings of their
loans. While only tables are abstract this follows from the natives'
shifting (`RunsAgree.ofShift`); with abstract owners it states that a run
depends on its maps' abstract values alone, assumed of a unit as its
natives' contracts are (`doc/designs/intrinsic-maps.md`, "Abstract
representation"). -/
structure RunsAgree {unit : ValidatedUnit} (executable : ExecutableUnit unit) : Prop where
  agree : ∀ {handle : FunctionHandle} {instantiation : Array (TypeId × TypeId)}
    {start start₂ final final₂ : RuntimeState} {arguments arguments₂ : Array RuntimeValue}
    {outcome outcome₂ : Outcome} {loans loans₂ : List Nat},
    start₂.globals = start.globals → start₂.tableHandles = start.tableHandles →
    (∀ slot ∈ start.globals.entries, Plain slot.value) →
    Denote.Admissible start loans → Denote.Admissible start₂ loans₂ →
    loans.length = loans₂.length →
    (∀ argument ∈ arguments, argument.Within (lentInside loans start.nextLoan)) →
    (∀ argument ∈ arguments₂, argument.Within (lentInside loans₂ start₂.nextLoan)) →
    (∀ target, arguments.map (·.rename (lentRenaming loans start.nextLoan target)) =
      arguments₂.map (·.rename (lentRenaming loans₂ start₂.nextLoan target))) →
    functionRuns executable handle instantiation start arguments final outcome →
    functionRuns executable handle instantiation start₂ arguments₂ final₂ outcome₂ →
    ∃ ρ ρ₂, Function.Injective ρ ∧ Function.Injective ρ₂ ∧
      outcome.rename ρ = outcome₂.rename ρ₂ ∧
      final.globals.rename ρ = final₂.globals.rename ρ₂ ∧
      final.tableHandles = final₂.tableHandles

/-- `RunsAgree` from starts lending nothing, on arguments holding no loan. -/
theorem RunsAgree.plain {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    (agrees : RunsAgree executable)
    {handle : FunctionHandle} {instantiation : Array (TypeId × TypeId)}
    {start start₂ final final₂ : RuntimeState} {arguments : Array RuntimeValue}
    {outcome outcome₂ : Outcome}
    (globals_eq : start₂.globals = start.globals)
    (tableHandles_eq : start₂.tableHandles = start.tableHandles)
    (plain : ∀ slot ∈ start.globals.entries, Plain slot.value)
    (admissible : Denote.Admissible start []) (admissible₂ : Denote.Admissible start₂ [])
    (arguments_plain : ∀ argument ∈ arguments, Plain argument)
    (runs : functionRuns executable handle instantiation start arguments final outcome)
    (runs₂ : functionRuns executable handle instantiation start₂ arguments final₂ outcome₂) :
    ∃ ρ ρ₂, Function.Injective ρ ∧ Function.Injective ρ₂ ∧
      outcome.rename ρ = outcome₂.rename ρ₂ ∧
      final.globals.rename ρ = final₂.globals.rename ρ₂ ∧
      final.tableHandles = final₂.tableHandles :=
  agrees.agree globals_eq tableHandles_eq plain admissible admissible₂ rfl
    (fun argument member => RuntimeValue.within_of_plain _ (arguments_plain argument member))
    (fun argument member => RuntimeValue.within_of_plain _ (arguments_plain argument member))
    (fun _ => by
      rw [Array.map_rename_of_plain _ arguments_plain, Array.map_rename_of_plain _ arguments_plain])
    runs runs₂

/-- Equal runtime stores determine their named slots; agreement at unnamed
resource types determines the rest of a semantic memory. -/
private theorem memory_eq_of_encodes {unit : ValidatedUnit}
    {left right pre : Denote.Memory unit} {leftState rightState : RuntimeState}
    (leftEncoded : Denote.StorageEncodes unit left leftState)
    (rightEncoded : Denote.StorageEncodes unit right rightState)
    (same : leftState.globals = rightState.globals ∧ leftState.tableHandles = rightState.tableHandles)
    (leftAgree : Denote.AgreeUnnamed unit left pre)
    (rightAgree : Denote.AgreeUnnamed unit right pre) : left = right := by
  classical
  funext resource key
  by_cases globalNamed : ∃ namespaceId typeId,
      Denote.runtimeResourceOf unit namespaceId typeId = some resource
  · obtain ⟨namespaceId, typeId, named⟩ := globalNamed
    have encoded := leftEncoded.globals.2 namespaceId typeId key
    rw [same.1, rightEncoded.globals.2, named] at encoded
    exact Option.map_injective
      (Denote.ResourceType.encode_injective resource) encoded.symm
  by_cases allocation : resource = Denote.TableMemory.allocationResource
  · subst resource
    rw [leftEncoded.allocated, rightEncoded.allocated, same.2]
  have unnamed : Denote.Unnamed unit resource :=
    ⟨fun ns ty named => globalNamed ⟨ns, ty, named⟩, allocation⟩
  exact congrFun ((leftAgree resource unnamed).trans (rightAgree resource unnamed).symm) key

/-! ### Invocations lending mutable references -/

/-- Lending to plain arguments commutes with renaming the loans. -/
theorem lendMutable_rename (ρ : Nat → Nat) : (mutable : List Bool) →
    (arguments : List RuntimeValue) → (loans : List Nat) →
    (∀ argument ∈ arguments, Plain argument) → {lent : List RuntimeValue} → {rest : List Nat} →
    lendMutable mutable arguments loans = some (lent, rest) →
    lendMutable mutable arguments (loans.map ρ) = some (lent.map (·.rename ρ), rest.map ρ)
  | [], [], loans, _, lent, rest, lends => by
      simp only [lendMutable, Option.some.injEq, Prod.mk.injEq] at lends
      obtain ⟨rfl, rfl⟩ := lends
      rfl
  | true :: mutable, entry :: arguments, loan :: loans, plain, lent, rest, lends => by
      simp only [lendMutable, Option.map_eq_some_iff, Prod.mk.injEq, Prod.exists] at lends
      obtain ⟨lent', rest', lends, rfl, rfl⟩ := lends
      have later := lendMutable_rename ρ mutable arguments loans
        (fun argument member => plain argument (by simp [member])) lends
      simp only [List.map_cons, lendMutable, later, Option.map_some, RuntimeValue.rename_borrow,
        RuntimeValue.rename_of_plain ρ (plain entry (by simp))]
  | false :: mutable, argument :: arguments, loans, plain, lent, rest, lends => by
      simp only [lendMutable, Option.map_eq_some_iff, Prod.mk.injEq, Prod.exists] at lends
      obtain ⟨lent', rest', lends, rfl, rfl⟩ := lends
      have later := lendMutable_rename ρ mutable arguments loans
        (fun argument member => plain argument (by simp [member])) lends
      simp only [List.map_cons, lendMutable, later, Option.map_some,
        RuntimeValue.rename_of_plain ρ (plain argument (by simp))]
  | true :: _, _ :: _, [], _, _, _, lends => by simp [lendMutable] at lends
  | [], _ :: _, _, _, _, _, lends => by simp [lendMutable] at lends
  | _ :: _, [], _, _, _, _, lends => by simp [lendMutable] at lends

/-- Values lent to plain arguments hold only the loans lent. -/
theorem lendMutable_within {inside : Nat → Prop} : (mutable : List Bool) →
    (arguments : List RuntimeValue) → (loans : List Nat) →
    (∀ argument ∈ arguments, Plain argument) → (∀ loan ∈ loans, inside loan) →
    {lent : List RuntimeValue} → {rest : List Nat} →
    lendMutable mutable arguments loans = some (lent, rest) → ∀ value ∈ lent, value.Within inside
  | [], [], _, _, _, lent, rest, lends => by
      simp only [lendMutable, Option.some.injEq, Prod.mk.injEq] at lends
      obtain ⟨rfl, rfl⟩ := lends
      simp
  | true :: mutable, entry :: arguments, loan :: loans, plain, lentInside, lent, rest, lends => by
      simp only [lendMutable, Option.map_eq_some_iff, Prod.mk.injEq, Prod.exists] at lends
      obtain ⟨lent', rest', lends, rfl, rfl⟩ := lends
      intro value member
      rcases List.mem_cons.mp member with rfl | member
      · rw [RuntimeValue.within_borrow]
        exact ⟨lentInside loan (by simp), RuntimeValue.within_of_plain _ (plain entry (by simp))⟩
      · exact lendMutable_within mutable arguments loans
          (fun argument member => plain argument (by simp [member]))
          (fun loan member => lentInside loan (by simp [member])) lends value member
  | false :: mutable, argument :: arguments, loans, plain, lentInside, lent, rest, lends => by
      simp only [lendMutable, Option.map_eq_some_iff, Prod.mk.injEq, Prod.exists] at lends
      obtain ⟨lent', rest', lends, rfl, rfl⟩ := lends
      intro value member
      rcases List.mem_cons.mp member with rfl | member
      · exact RuntimeValue.within_of_plain _ (plain value (by simp))
      · exact lendMutable_within mutable arguments loans
          (fun argument member => plain argument (by simp [member])) lentInside lends value member
  | true :: _, _ :: _, [], _, _, _, _, lends => by simp [lendMutable] at lends
  | [], _ :: _, _, _, _, _, _, lends => by simp [lendMutable] at lends
  | _ :: _, [], _, _, _, _, _, lends => by simp [lendMutable] at lends

/-- Lending takes a loan for each mutable argument. -/
theorem lendMutable_length : (mutable : List Bool) → (arguments : List RuntimeValue) →
    (loans : List Nat) → {lent : List RuntimeValue} → {rest : List Nat} →
    lendMutable mutable arguments loans = some (lent, rest) →
    loans.length = (mutable.filter id).length + rest.length
  | [], [], loans, lent, rest, lends => by
      simp only [lendMutable, Option.some.injEq, Prod.mk.injEq] at lends
      obtain ⟨rfl, rfl⟩ := lends
      simp
  | true :: mutable, _ :: arguments, loan :: loans, lent, rest, lends => by
      simp only [lendMutable, Option.map_eq_some_iff, Prod.mk.injEq, Prod.exists] at lends
      obtain ⟨lent', rest', lends, rfl, rfl⟩ := lends
      have later := lendMutable_length mutable arguments loans lends
      simp only [List.length_cons, List.filter_cons_of_pos, id, later]
      omega
  | false :: mutable, _ :: arguments, loans, lent, rest, lends => by
      simp only [lendMutable, Option.map_eq_some_iff, Prod.mk.injEq, Prod.exists] at lends
      obtain ⟨lent', rest', lends, rfl, rfl⟩ := lends
      simpa [List.filter_cons] using lendMutable_length mutable arguments loans lends
  | true :: _, _ :: _, [], _, _, lends => by simp [lendMutable] at lends
  | [], _ :: _, _, _, _, lends => by simp [lendMutable] at lends
  | _ :: _, [], _, _, _, lends => by simp [lendMutable] at lends

@[simp] theorem readResults_nil (values : List RuntimeValue) : readResults [] values = values := by
  cases values <;> rfl

/-- Results a run returns and those of another run renamed alike, where the
first reads as plain values, read alike. -/
theorem readResults_agree {ρ ρ₂ : Nat → Nat} (injective₂ : Function.Injective ρ₂) :
    (mutable : List Bool) → (values values₂ : List RuntimeValue) →
    values.map (·.rename ρ) = values₂.map (·.rename ρ₂) →
    (∀ result ∈ readResults mutable values, Plain result) →
    readResults mutable values₂ = readResults mutable values
  | [], values, values₂, renamed, plain => by
      simp only [readResults_nil] at plain ⊢
      refine (List.map_inj_right fun _ _ => RuntimeValue.rename_injective injective₂).1 ?_
      rw [← renamed]
      exact List.map_congr_left fun value member =>
        (RuntimeValue.rename_of_plain ρ (plain value member)).trans
          (RuntimeValue.rename_of_plain ρ₂ (plain value member)).symm
  | marked :: mutable, [], values₂, renamed, _ => by
      cases values₂ with
      | nil => rfl
      | cons _ _ => simp at renamed
  | marked :: mutable, value :: values, [], renamed, _ => by simp at renamed
  | marked :: mutable, value :: values, value₂ :: values₂, renamed, plain => by
      simp only [List.map_cons, List.cons.injEq] at renamed
      cases marked with
      | true =>
          simp only [readResults, List.mem_cons, forall_eq_or_imp] at plain ⊢
          rw [← RuntimeValue.observe_rename ρ₂ value₂, ← renamed.1, RuntimeValue.observe_rename,
            readResults_agree injective₂ mutable values values₂ renamed.2 plain.2]
      | false =>
          simp only [readResults, List.mem_cons, forall_eq_or_imp] at plain ⊢
          have same : value₂ = value := RuntimeValue.rename_injective injective₂ (by
            rw [← renamed.1, RuntimeValue.rename_of_plain ρ plain.1,
              RuntimeValue.rename_of_plain ρ₂ plain.1])
          rw [same, readResults_agree injective₂ mutable values values₂ renamed.2 plain.2]

/-- Runs of the target of a closure holding no loan, on its captures and
plain arguments with mutable ones lent from starts at the same memory,
agree up to injective renamings of their loans. -/
private theorem lentRuns_agree {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    (agrees : RunsAgree executable) {function : FunctionHandle} {mask : Nat}
    {typeInstantiation : Array (TypeId × TypeId)} {captures : Array RuntimeValue}
    {mutable : List Bool} {arguments : List RuntimeValue} {state : Denote.Memory unit}
    {start start₂ exit exit₂ : RuntimeState} {loans loans₂ : List Nat}
    {lent lent₂ composed composed₂ : List RuntimeValue} {outcome outcome₂ : Outcome}
    (captures_plain : ∀ capture ∈ captures, Plain capture)
    (arguments_plain : ∀ argument ∈ arguments, Plain argument)
    (encodes : Denote.StorageEncodes unit state start)
    (encodes₂ : Denote.StorageEncodes unit state start₂)
    (admissible : Denote.Admissible start loans) (admissible₂ : Denote.Admissible start₂ loans₂)
    (lends : lendMutable mutable arguments loans = some (lent, []))
    (lends₂ : lendMutable mutable arguments loans₂ = some (lent₂, []))
    (composes : ClosureMask.compose mask captures.toList lent = some composed)
    (composes₂ : ClosureMask.compose mask captures.toList lent₂ = some composed₂)
    (run : functionRuns executable function typeInstantiation start composed.toArray exit outcome)
    (run₂ : functionRuns executable function typeInstantiation start₂ composed₂.toArray exit₂
      outcome₂) :
    ∃ ρ ρ₂, Function.Injective ρ ∧ Function.Injective ρ₂ ∧
      outcome.rename ρ = outcome₂.rename ρ₂ ∧
      exit.globals.rename ρ = exit₂.globals.rename ρ₂ ∧
      exit.tableHandles = exit₂.tableHandles := by
  have length_eq : loans.length = loans₂.length := by
    rw [lendMutable_length _ _ _ lends, lendMutable_length _ _ _ lends₂]
  have capturesFixed : ∀ σ : Nat → Nat, captures.toList.map (·.rename σ) = captures.toList :=
    fun σ => by rw [← Array.toList_map, Array.map_rename_of_plain σ captures_plain]
  have within : ∀ {loans : List Nat} {lent composed : List RuntimeValue} (frontier : Nat),
      lendMutable mutable arguments loans = some (lent, []) →
      ClosureMask.compose mask captures.toList lent = some composed →
      ∀ argument ∈ composed.toArray, argument.Within (lentInside loans frontier) := by
    intro loans lent composed frontier lends composes argument member
    rcases ClosureMask.compose_mem composes argument (by simpa using member) with
      captured | lent
    · exact RuntimeValue.within_of_plain _ (captures_plain argument (by simpa using captured))
    · exact lendMutable_within mutable arguments loans arguments_plain
        (fun _ member => .inr member) lends argument lent
  have arguments_eq : ∀ target,
      composed.toArray.map (·.rename (lentRenaming loans start.nextLoan target)) =
        composed₂.toArray.map (·.rename (lentRenaming loans₂ start₂.nextLoan target)) := by
    intro target
    obtain ⟨nodup, lentBelow, -⟩ := admissible
    obtain ⟨nodup₂, lentBelow₂, -⟩ := admissible₂
    have canonical := lendMutable_rename (lentRenaming loans start.nextLoan target) mutable
      arguments loans arguments_plain lends
    have canonical₂ := lendMutable_rename (lentRenaming loans₂ start₂.nextLoan target) mutable
      arguments loans₂ arguments_plain lends₂
    rw [lentRenaming_lent nodup fun loan member => (lentBelow loan member).1] at canonical
    rw [lentRenaming_lent nodup₂ fun loan member => (lentBelow₂ loan member).1, ← length_eq,
      canonical] at canonical₂
    simp only [List.map_nil, Option.some.injEq, Prod.mk.injEq, and_true] at canonical₂
    have mapped := ClosureMask.compose_map (·.rename (lentRenaming loans start.nextLoan target))
      mask captures.toList lent
    have mapped₂ := ClosureMask.compose_map (·.rename (lentRenaming loans₂ start₂.nextLoan target))
      mask captures.toList lent₂
    rw [capturesFixed, composes, canonical₂] at mapped
    rw [capturesFixed, composes₂, mapped] at mapped₂
    simp only [Option.map_some, Option.some.injEq] at mapped₂
    simp only [List.map_toArray, mapped₂]
  exact agrees.agree (encodes₂.unique encodes).1 (encodes₂.unique encodes).2
    encodes.globals.plain admissible admissible₂ length_eq
    (within _ lends composes) (within _ lends₂ composes₂) arguments_eq run run₂

end Agreement

/-! ## Runs from one memory agree

Two runs of an invocation from the same memory, on arguments of the same
entry values, agree up to the loans they mint (`RunsAgree`): a run that
returns refutes `aborts_of`, and the settled results and final memories of
two runs are equal, which `result_of` and `state_of` read. -/

section Determinism
open Denote
open SemanticOperations
variable {unit : ValidatedUnit} [Skolems unit]

/-- Arguments of the same entry values pass the same entry values. -/
private theorem entries_of_settle {σs : NRow} {args args' : HList σs}
    (same : HList.settle σs args = HList.settle σs args') :
    HList.entries args' = HList.entries args := by
  rw [← HList.entries_settle σs args', ← same, HList.entries_settle]

/-- Two runs of an invocation from one memory, on arguments of the same
entry values, returning settled results, return the same result and leave
the same memory. -/
private theorem closureRun_ok_agree {executable : ExecutableUnit unit}
    (agrees : RunsAgree executable) {closure : ClosureValue} {σs : NRow}
    (flat : σs.lentFlat = true) {shape : ResultShape} (shapeFlat : shape.row.lentFlat = true)
    {args args' : HList σs} (same : HList.settle σs args = HList.settle σs args')
    {state post post' : Memory unit} {result result' : shape.carrier}
    (settled : shape.Settled result) (settled' : shape.Settled result')
    (ok : (closureRun executable closure σs shape args).ok state result post)
    (ok' : (closureRun executable closure σs shape args').ok state result' post') :
    result = result' ∧ post = post' := by
  obtain ⟨start, loans, supplied, arguments, results, exit, returnedLoans, prophecyRow, encodes,
    admissible, lends, composes, runs, lendResult, lendProphecy, -, encoded, agree⟩ := ok
  obtain ⟨start', loans', supplied', arguments', results', exit', returnedLoans', prophecyRow',
    encodes', admissible', lends', composes', runs', lendResult', lendProphecy', -, encoded',
    agree'⟩ := ok'
  obtain ⟨lentRow, lendsRow, rfl⟩ := lendArguments_lentFlat flat lends
  obtain ⟨lentRow', lendsRow', rfl⟩ := lendArguments_lentFlat flat lends'
  rw [entries_of_settle same] at lendsRow'
  obtain ⟨ρ, ρ₂, injective, injective₂, renamed, globals, tableHandles⟩ :=
    lentRuns_agree agrees closure.plain (HList.entries_plain σs args) encodes encodes' admissible
      admissible' lendsRow lendsRow' composes composes' runs runs'
  simp only [Outcome.rename_returned, Outcome.returned.injEq] at renamed
  have read := ResultShape.readResults_lend shape shapeFlat result returnedLoans lendResult
  have read' := ResultShape.readResults_lend shape shapeFlat result' returnedLoans' lendResult'
  have readAlike := readResults_agree injective₂ shape.row.mutable results.toList results'.toList
    (by simpa only [Array.toList_map] using congrArg Array.toList renamed)
    (by
      rw [read]
      exact fun value member => ResultShape.current_plain shape result value (by simpa using member))
  rw [read, read'] at readAlike
  have sameResult := ResultShape.eq_of_current shape settled settled'
    (Array.toList_inj.mp readAlike).symm
  rw [ResultShape.lend_settled shape shapeFlat result settled, lendResult] at lendProphecy
  rw [ResultShape.lend_settled shape shapeFlat result' settled', lendResult'] at lendProphecy'
  obtain rfl : prophecyRow = results := (Option.some.inj lendProphecy).symm
  obtain rfl : prophecyRow' = results' := (Option.some.inj lendProphecy').symm
  refine ⟨sameResult, ?_⟩
  suffices same : (exit'.resolveReturned prophecyRow').globals =
        (exit.resolveReturned prophecyRow).globals ∧
      (exit'.resolveReturned prophecyRow').tableHandles =
        (exit.resolveReturned prophecyRow).tableHandles by
    exact (memory_eq_of_encodes encoded' encoded same agree' agree).symm
  have resolved := RuntimeState.resolveReturned_rename injective (state' := { exit with
    globals := exit.globals.rename ρ }) rfl prophecyRow
  have resolved₂ := RuntimeState.resolveReturned_rename injective₂ (state' := { exit' with
    globals := exit'.globals.rename ρ₂ }) rfl prophecyRow'
  rw [GlobalMap.rename_of_plain _ encoded.globals.plain] at resolved
  rw [GlobalMap.rename_of_plain _ encoded'.globals.plain] at resolved₂
  rw [← resolved, ← resolved₂]
  simp only [RuntimeState.resolveReturned, renamed, globals, tableHandles, and_self]

/-- An invocation that returns from `state` does not abort from it, on
arguments of the same entry values. -/
theorem not_abortsOf_of_ok {executable : ExecutableUnit unit} (agrees : RunsAgree executable)
    {σs : NRow} (flat : σs.lentFlat = true) {shape : ResultShape}
    {shared : Sharing} {function : (NTy.function σs shared shape.row).carrier}
    {args args' : HList σs} (same : HList.settle σs args = HList.settle σs args')
    {state post : Memory unit} {result : shape.carrier}
    (ok : EnsuresOf (closureMeaning executable) σs shared shape function args result state post) :
    ¬AbortsOf (closureMeaning executable) σs shared shape function args' state := by
  rintro ⟨error, start', loans', supplied', arguments', encodes', admissible', lends', composes',
    aborted⟩
  obtain ⟨start, loans, supplied, arguments, results, exit, -, -, encodes, admissible, lends,
    composes, runs, -⟩ := ok
  obtain ⟨lentRow, lendsRow, rfl⟩ := lendArguments_lentFlat flat lends
  obtain ⟨lentRow', lendsRow', rfl⟩ := lendArguments_lentFlat flat lends'
  rw [entries_of_settle same] at lendsRow'
  obtain ⟨_, aborted⟩ := aborted
  obtain ⟨_, _, -, -, renamed, -⟩ := lentRuns_agree agrees
    ((Carriers.functions σs shared shape.row).closure function).plain
    (HList.entries_plain σs args) encodes encodes' admissible admissible' lendsRow lendsRow'
    composes composes' runs aborted
  simp [Outcome.rename_returned, Outcome.rename_threw] at renamed

/-- `result_of` is the settled result of every run from `state` on
arguments of the same entry values. -/
theorem resultOf_eq_of_ok {executable : ExecutableUnit unit} (agrees : RunsAgree executable)
    {σs : NRow} (flat : σs.lentFlat = true) {shape : ResultShape}
    {shared : Sharing} {function : (NTy.function σs shared shape.row).carrier}
    (shapeFlat : shape.row.lentFlat = true) {args args' : HList σs}
    (same : HList.settle σs args = HList.settle σs args') {state post : Memory unit}
    {result : shape.carrier}
    (ok : EnsuresOf (closureMeaning executable) σs shared shape function args result state post)
    (settled : shape.Settled result) (default : shape.carrier) :
    ResultOf (closureMeaning executable) σs shared shape function args' state default = result := by
  obtain ⟨chosen, ⟨sameChosen, settledChosen, okChosen⟩, chosenResult, _⟩ :=
    settledEncoding_choose (invoke := closureMeaning executable)
      ⟨runEncoding σs shape (args, result, post), (args, result, post), ⟨same, settled, ok⟩, rfl⟩
      default
  rw [chosenResult]
  exact (closureRun_ok_agree agrees flat shapeFlat (same.trans sameChosen.symm) settled
    settledChosen ok okChosen).1.symm

/-- `state_of` is the final memory of every run from `state` on arguments
of the same entry values that returns a settled result. -/
theorem stateOf_eq_of_ok {executable : ExecutableUnit unit} (agrees : RunsAgree executable)
    {σs : NRow} (flat : σs.lentFlat = true) {shape : ResultShape}
    {shared : Sharing} {function : (NTy.function σs shared shape.row).carrier}
    (shapeFlat : shape.row.lentFlat = true) {args args' : HList σs}
    (same : HList.settle σs args = HList.settle σs args') {state post : Memory unit}
    {result : shape.carrier}
    (ok : EnsuresOf (closureMeaning executable) σs shared shape function args result state post)
    (settled : shape.Settled result) :
    StateOf (closureMeaning executable) σs shared shape function args' state = post := by
  obtain ⟨chosen, ⟨sameChosen, settledChosen, okChosen⟩, _, chosenState⟩ :=
    settledEncoding_choose (invoke := closureMeaning executable)
      ⟨runEncoding σs shape (args, result, post), (args, result, post), ⟨same, settled, ok⟩, rfl⟩
      result
  rw [chosenState]
  exact (closureRun_ok_agree agrees flat shapeFlat (same.trans sameChosen.symm) settled
    settledChosen ok okChosen).2.symm

end Determinism

section Rule

open Denote
open SemanticOperations
variable {unit : Validation.ValidatedUnit} [Skolems unit]

/-- Results lent at a reference-free shape hold no loan. -/
theorem ResultShape.lend_plain : (shape : ResultShape) → shape.row.refFree = true →
    {result : shape.carrier} → {values : Array RuntimeValue} →
    shape.lend false result [] = some values → ∀ value ∈ values, Plain value
  | .none, _, _, _, lent => by
      simp only [ResultShape.lend, Option.some.injEq] at lent
      subst lent
      intro _ member
      simp at member
  | .one τ, free, result, _, lent => by
      simp only [NRow.refFree, Bool.and_true] at free
      simp only [ResultShape.lend, NTy.lend_refFree τ free false result [],
        Option.some.injEq] at lent
      subst lent
      intro value member
      simp only [List.mem_toArray, List.mem_singleton] at member
      subst member
      exact NTy.encode_plain τ result

/-- An invocation of a typed closure a proof cannot see, on typed arguments
from typed global memory, has the weakest precondition its behavioral
predicates state: a run is one `ensures_of` names, `aborts_of` does not
hold, and its result and final memory are `result_of` and `state_of`; an
abort is one `aborts_of` names. -/
theorem wp_closureMeaning_unseen {unit : ValidatedUnit} (executable : ExecutableUnit unit)
    [Skolems unit] (natives : NativesTyped executable)
    (agrees : RunsAgree executable) {σs : NRow} {shared : Sharing} {shape : ResultShape}
    (function : (NTy.function σs shared shape.row).carrier)
    (args : HList σs) (free : σs.refFree = true) (shapeFree : shape.row.refFree = true)
    {parameters results : List SemTy}
    (typed : ClosureTyped unit ((Carriers.functions σs shared shape.row).closure function).function ((Carriers.functions σs shared shape.row).closure function).mask
      ((Carriers.functions σs shared shape.row).closure function).typeInstantiation ((Carriers.functions σs shared shape.row).closure function).captures parameters results)
    (decodes : RowDecodes unit shape results) {initial : Memory unit}
    {ensures : shape.carrier → Memory unit → Prop} {aborts : Failure → Prop}
    (returns : ∀ result final,
      EnsuresOf (closureMeaning executable) σs shared shape function args result initial final →
      ¬AbortsOf (closureMeaning executable) σs shared shape function args initial →
      (∀ default, ResultOf (closureMeaning executable) σs shared shape function args initial
        default = result) →
      StateOf (closureMeaning executable) σs shared shape function args initial = final →
      ensures result final)
    (fails : ∀ error, AbortsOf (closureMeaning executable) σs shared shape function args initial →
      aborts error) :
    wp (closureMeaning executable σs shared shape function args) ensures aborts initial := by
  have flat := NRow.lentFlat_of_refFree σs free
  have shapeFlat := NRow.lentFlat_of_refFree shape.row shapeFree
  refine (wp_closureRun_typed executable natives _ args free shapeFree typed decodes
    ensures aborts).mpr
    ⟨fun result final run => ?_, fun error aborted => fails error ⟨error, aborted⟩⟩
  have settled := ResultShape.settled_of_refFree shape shapeFree result
  exact returns result final run (not_abortsOf_of_ok agrees flat rfl run)
    (resultOf_eq_of_ok agrees flat shapeFlat rfl run settled)
    (stateOf_eq_of_ok agrees flat shapeFlat rfl run settled)

end Rule

section Rows

open Denote
variable {unit : Validation.ValidatedUnit} [Skolems unit]

/-- The semantic type of a scalar native type: what its encodings inhabit,
and a typed value decodes at. -/
def Denote.NTy.scalarType? : NTy → Option SemTy
  | .bool => some .bool
  | .int width signed => some (.integer (.bits width) signed)
  | .num => some (.integer .unbounded true)
  | .address => some .address
  | .signer => some .signer
  | .string => some .string
  | .bytes => some .bytes
  | .unit | .tuple _ | .struct .. | .enum .. | .vector _ | .table _ _ | .ghost _ | .ref _
  | .param _ | .function .. => none

theorem Denote.NTy.encode_hasType {unit : ValidatedUnit} [Skolems unit] {loans : LoanTypes} :
    (τ : NTy) → {type : SemTy} → τ.scalarType? = some type → (value : τ.carrier) →
      HasType unit loans (τ.encode value) type
  | .bool, _, scalar, value => by
      simp only [NTy.scalarType?, Option.some.injEq] at scalar; subst scalar; exact .bool value
  | .int width signed, _, scalar, value => by
      simp only [NTy.scalarType?, Option.some.injEq] at scalar; subst scalar
      refine .integer value.val (.bits width) signed ?_
      have fits := value.fits
      simp only [IntegerValueFits] at fits
      simp [StaticTyping.holdsAt, StaticTyping.targetWidth, fits]
  | .num, _, scalar, value => by
      simp only [NTy.scalarType?, Option.some.injEq] at scalar; subst scalar
      refine .integer value.val .unbounded true ?_
      simp [StaticTyping.holdsAt, StaticTyping.targetWidth, Ty.integerValueFits?]
  | .address, _, scalar, value => by
      simp only [NTy.scalarType?, Option.some.injEq] at scalar; subst scalar; exact .address value
  | .signer, _, scalar, value => by
      simp only [NTy.scalarType?, Option.some.injEq] at scalar; subst scalar; exact .signer value
  | .string, _, scalar, value => by
      simp only [NTy.scalarType?, Option.some.injEq] at scalar; subst scalar; exact .string value
  | .bytes, _, scalar, value => by
      simp only [NTy.scalarType?, Option.some.injEq] at scalar; subst scalar; exact .bytes value
  | .unit, _, scalar, _ | .tuple _, _, scalar, _ | .struct .., _, scalar, _
  | .enum .., _, scalar, _ | .vector _, _, scalar, _ | .table _ _, _, scalar, _
  | .ghost _, _, scalar, _ | .ref _, _, scalar, _ | .param _, _, scalar, _
  | .function .., _, scalar, _ => by
      simp [NTy.scalarType?] at scalar

theorem Denote.NTy.decode_of_hasType {unit : ValidatedUnit} [Skolems unit] {loans : LoanTypes} :
    (τ : NTy) → {type : SemTy} → τ.scalarType? = some type → {value : RuntimeValue} →
      HasType unit loans value type → value.holeFree? = true → ∃ decoded, τ.encode decoded = value
  | .bool, _, scalar, value, typed, _ => by
      simp only [NTy.scalarType?, Option.some.injEq] at scalar; subst scalar
      cases typed with
      | bool value => exact ⟨value, rfl⟩
      | hole => simp [RuntimeValue.holeFree?] at *
  | .int width signed, _, scalar, value, typed, _ => by
      simp only [NTy.scalarType?, Option.some.injEq] at scalar; subst scalar
      cases typed with
      | integer value _ _ fits =>
          refine ⟨⟨value, ?_⟩, rfl⟩
          simp only [StaticTyping.holdsAt, StaticTyping.targetWidth, beq_iff_eq] at fits
          exact fits
      | hole => simp [RuntimeValue.holeFree?] at *
  | .num, _, scalar, value, typed, _ => by
      simp only [NTy.scalarType?, Option.some.injEq] at scalar; subst scalar
      cases typed with
      | integer value _ _ fits => exact ⟨⟨value, rfl⟩, rfl⟩
      | hole => simp [RuntimeValue.holeFree?] at *
  | .address, _, scalar, value, typed, _ => by
      simp only [NTy.scalarType?, Option.some.injEq] at scalar; subst scalar
      cases typed with
      | address value => exact ⟨value, rfl⟩
      | hole => simp [RuntimeValue.holeFree?] at *
  | .signer, _, scalar, value, typed, _ => by
      simp only [NTy.scalarType?, Option.some.injEq] at scalar; subst scalar
      cases typed with
      | signer value => exact ⟨value, rfl⟩
      | hole => simp [RuntimeValue.holeFree?] at *
  | .string, _, scalar, value, typed, _ => by
      simp only [NTy.scalarType?, Option.some.injEq] at scalar; subst scalar
      cases typed with
      | string value => exact ⟨value, rfl⟩
      | hole => simp [RuntimeValue.holeFree?] at *
  | .bytes, _, scalar, value, typed, _ => by
      simp only [NTy.scalarType?, Option.some.injEq] at scalar; subst scalar
      cases typed with
      | bytes value => exact ⟨value, rfl⟩
      | hole => simp [RuntimeValue.holeFree?] at *
  | .unit, _, scalar, _, _, _ | .tuple _, _, scalar, _, _, _ | .struct .., _, scalar, _, _, _
  | .enum .., _, scalar, _, _, _ | .vector _, _, scalar, _, _, _ | .table _ _, _, scalar, _, _, _
  | .ghost _, _, scalar, _, _, _ | .ref _, _, scalar, _, _, _ | .param _, _, scalar, _, _, _
  | .function .., _, scalar, _, _, _ =>
    by
      simp [NTy.scalarType?] at scalar

/-- A scalar encoding normalizes to itself. -/
theorem Denote.NTy.normalize_encode_scalar {unit : ValidatedUnit} [Skolems unit] :
    (τ : NTy) → {type : SemTy} → τ.scalarType? = some type → (value : τ.carrier) →
      SemanticOperations.normalize unit (τ.encode value) = τ.encode value
  | .bool, _, _, _ | .int _ _, _, _, _ | .num, _, _, _ | .address, _, _, _ | .signer, _, _, _
  | .string, _, _, _ | .bytes, _, _, _ => by simp
  | .unit, _, scalar, _ | .tuple _, _, scalar, _ | .struct .., _, scalar, _
  | .enum .., _, scalar, _ | .vector _, _, scalar, _ | .table _ _, _, scalar, _
  | .ghost _, _, scalar, _ | .ref _, _, scalar, _ | .param _, _, scalar, _
  | .function .., _, scalar, _ => by simp [NTy.scalarType?] at scalar

/-- A value typed at a scalar type normalizes to itself: it is a scalar or a
hole. -/
theorem normalize_of_hasType_scalar {unit : ValidatedUnit} {loans : LoanTypes} {τ : NTy}
    {type : SemTy} (scalar : τ.scalarType? = some type) {value : RuntimeValue}
    (typed : HasType unit loans value type) :
    SemanticOperations.normalize unit value = value := by
  cases τ <;> simp only [NTy.scalarType?, Option.some.injEq, reduceCtorEq] at scalar <;>
    subst scalar <;> cases typed <;> simp

/-- A scalar result decodes at its native type. -/
theorem RowDecodes.scalar {unit : ValidatedUnit} [Skolems unit] {τ : NTy} {type : SemTy}
    (scalar : τ.scalarType? = some type) : RowDecodes unit (.one τ) [type] := by
  intro loans values typed holeFree
  obtain ⟨list⟩ := values
  cases typed with
  | cons head tail =>
      cases tail with
      | nil =>
          rename_i value
          simp only [Outcome.holeFree, List.all_toArray, List.all_cons, List.all_nil,
            Bool.and_true] at holeFree
          obtain ⟨decoded, encoded⟩ := NTy.decode_of_hasType τ scalar head holeFree
          have free : τ.refFree = true := by
            cases τ <;> simp_all [NTy.scalarType?, NTy.refFree]
          have fixed : SemanticOperations.normalize unit value = value := by
            rw [← encoded]; exact NTy.normalize_encode_scalar τ scalar decoded
          refine ⟨decoded, ?_⟩
          simp only [ResultShape.lend, NTy.lend_refFree τ free false decoded [], encoded,
            List.map_toArray, List.map_cons, List.map_nil, fixed]

/-- The semantic types of a row of scalar native types. -/
def Denote.NRow.scalarTypes? : NRow → Option (List SemTy)
  | .nil => some []
  | .cons τ rest => do some ((← τ.scalarType?) :: (← NRow.scalarTypes? rest))

/-- A row of scalar native values encodes as values of its semantic types. -/
theorem Denote.NRow.encode_hasTypes {unit : ValidatedUnit} [Skolems unit] {loans : LoanTypes} :
    (row : NRow) → {types : List SemTy} → row.scalarTypes? = some types → (values : HList row) →
      HasTypes unit loans (HList.encode values) types
  | .nil, _, scalar, _ => by
      simp only [NRow.scalarTypes?, Option.some.injEq] at scalar; subst scalar; exact .nil
  | .cons τ rest, _, scalar, values => by
      simp only [NRow.scalarTypes?, Option.bind_eq_bind, Option.bind_eq_some_iff,
        Option.some.injEq] at scalar
      obtain ⟨type, head, types, tail, rfl⟩ := scalar
      exact .cons (NTy.encode_hasType τ head values.1) (NRow.encode_hasTypes rest tail values.2)

/-- The type a native value of a parameter of `type` has: a shared
reference is the value it observes. -/
def observedType : SemTy → SemTy
  | .reference .shared referent => referent
  | .reference .mutable referent => .reference .mutable referent
  | .unit => .unit | .never => .never | .bool => .bool | .character => .character
  | .string => .string | .bytes => .bytes | .address => .address | .signer => .signer
  | .integer width signed => .integer width signed
  | .tuple elements => .tuple elements
  | .vector element length => .vector element length
  | .nominal name arguments => .nominal name arguments
  | .function arguments result => .function arguments result
  | .profile value => .profile value
  | .param index => .param index
  | .ghost slot => .ghost slot

omit [Skolems unit] in
theorem HasTypes.ofObserved {unit : ValidatedUnit} {loans : LoanTypes} :
    {values : List RuntimeValue} → {types : List SemTy} →
      HasTypes unit loans values (types.map observedType) → HasTypes unit loans values types
  | [], [], _ => .nil
  | _ :: _, [], typed => by cases typed
  | [], _ :: _, typed => by cases typed
  | _ :: _, type :: _, typed => by
      cases typed with
      | cons head tail =>
          refine .cons ?_ (HasTypes.ofObserved tail)
          cases type with
          | reference kind referent =>
              cases kind with
              | shared => exact .shared _ _ head
              | mutable => exact head
          | _ => exact head

omit [Skolems unit] in
theorem HasTypes.toObserved {unit : ValidatedUnit} {loans : LoanTypes} :
    {values : List RuntimeValue} → {types : List SemTy} →
      HasTypes unit loans values types → HasTypes unit loans values (types.map observedType)
  | [], [], _ => .nil
  | _ :: _, [], typed => by cases typed
  | [], _ :: _, typed => by cases typed
  | _ :: _, type :: _, typed => by
      cases typed with
      | cons head tail =>
          refine .cons ?_ (HasTypes.toObserved tail)
          cases type with
          | reference kind referent =>
              cases kind with
              | shared => exact head.shared_referent
              | mutable => exact head
          | _ => exact head

/-- Results decode at the types their shared references observe, so at
those references. -/
theorem RowDecodes.ofObserved {unit : ValidatedUnit} [Skolems unit] {shape : ResultShape}
    {types : List SemTy} (decodes : RowDecodes unit shape (types.map observedType)) :
    RowDecodes unit shape types := fun loans values typed holeFree =>
  decodes loans values (HasTypes.toObserved typed) holeFree

/-- Whether every native type of a row is a scalar or a type parameter: the
rows an invocation of a function value a proof cannot see is decided at. -/
def Denote.NRow.invocable : NRow → Bool
  | .nil => true
  | .cons τ rest => (τ.scalarType?.isSome || τ matches .param _) && NRow.invocable rest

/-- Semantic types every value of a row encodes at. -/
def RowTyped (unit : ValidatedUnit) [Skolems unit] (row : NRow) (types : List SemTy) : Prop :=
  ∀ values : HList row, HasTypes unit (fun _ => none) (HList.encode values) types

/-- A row whose types the frame resolves to scalars is typed at their
semantic types. -/
theorem RowTyped.ofResolved {unit : ValidatedUnit} [Skolems unit] {row : NRow} {types : List SemTy}
    (scalar : row.resolved.scalarTypes? = some types) : RowTyped unit row types := fun values => by
  rw [← HList.encode_toRuntime row values]
  exact @NRow.encode_hasTypes unit (Skolems.runtime unit) (fun _ => none) row.resolved types scalar
    (HList.toRuntime row values)

/-- A row the frame resolves to scalars: the rows the rule for an invocation
of a function value a proof cannot see is decided at. -/
def ScalarAt (unit : ValidatedUnit) [Skolems unit] (row : NRow) : Prop :=
  row.resolved.scalarTypes?.isSome = true

/-- A function value typed at a native function type of scalars and type
parameters: semantic rows its target's typing states, at which the
parameter row's values are typed and typed results decode at the result
row. Vacuous at other rows. -/
def ClosureTypedAt (unit : ValidatedUnit) [Skolems unit] (parameters : NRow) (shared : Sharing)
    (results : NRow) (function : (NTy.function parameters shared results).carrier) : Prop :=
  parameters.invocable = true → results.invocable = true →
    let closure := (Carriers.functions parameters shared results).closure function
    ∃ parameterTypes resultTypes targetTypes, targetTypes.map observedType = parameterTypes ∧
      ClosureTyped unit closure.function closure.mask closure.typeInstantiation closure.captures
        targetTypes resultTypes ∧
      RowTyped unit parameters parameterTypes ∧
      ∀ shape : ResultShape, shape.row = results → RowDecodes unit shape resultTypes

omit [Skolems unit] in
/-- A row of scalar types holds no reference. -/
theorem Denote.NRow.refFree_of_scalarTypes : (row : NRow) → {types : List SemTy} →
    row.scalarTypes? = some types → row.refFree = true
  | .nil, _, _ => rfl
  | .cons τ rest, _, scalar => by
      simp only [NRow.scalarTypes?, Option.bind_eq_bind, Option.bind_eq_some_iff] at scalar
      obtain ⟨type, head, types, tail, -⟩ := scalar
      simp only [NRow.refFree, NRow.refFree_of_scalarTypes rest tail, Bool.and_true]
      cases τ <;> simp_all [NTy.scalarType?, NTy.refFree]

/-- Values typed at a row of scalars normalize to themselves. -/
theorem HasTypes.map_normalize_of_scalar {unit : ValidatedUnit} {loans : LoanTypes} :
    (row : NRow) → {types : List SemTy} → row.scalarTypes? = some types →
      {values : List RuntimeValue} → HasTypes unit loans values types →
      values.map (SemanticOperations.normalize unit) = values
  | .nil, _, scalar, _, typed => by
      simp only [NRow.scalarTypes?, Option.some.injEq] at scalar
      subst scalar
      cases typed
      rfl
  | .cons τ rest, _, scalar, _, typed => by
      simp only [NRow.scalarTypes?, Option.bind_eq_bind, Option.bind_eq_some_iff,
        Option.some.injEq] at scalar
      obtain ⟨type, head, types, tail, rfl⟩ := scalar
      cases typed with
      | cons headTyped tailTyped =>
          simp only [List.map_cons, normalize_of_hasType_scalar head headTyped,
            HasTypes.map_normalize_of_scalar rest tail tailTyped]

/-- Results of a scalar row decode at their result shape. -/
theorem RowDecodes.ofScalar {unit : ValidatedUnit} [Skolems unit] :
    (shape : ResultShape) → {types : List SemTy} → shape.row.scalarTypes? = some types →
      RowDecodes unit shape types
  | .none, _, scalar => by
      simp only [NRow.scalarTypes?, Option.some.injEq] at scalar
      subst scalar
      intro loans values typed _
      obtain ⟨list⟩ := values
      cases typed
      exact ⟨(), by simp <;> rfl⟩
  | .one τ, _, scalar => by
      simp only [NRow.scalarTypes?, Option.bind_eq_bind, Option.bind_eq_some_iff,
        Option.some.injEq] at scalar
      obtain ⟨type, head, _, tail, rfl⟩ := scalar
      cases tail
      exact RowDecodes.scalar head

/-- Results of a reference-free shape whose row the frame resolves to
scalars decode at their semantic types. -/
theorem RowDecodes.ofResolved {unit : ValidatedUnit} [Skolems unit] :
    (shape : ResultShape) → {types : List SemTy} → shape.row.refFree = true →
      shape.row.resolved.scalarTypes? = some types → RowDecodes unit shape types
  | .none, _, _, scalar => by
      simp only [ResultShape.row, NRow.resolved, NRow.scalarTypes?, Option.some.injEq] at scalar
      subst scalar
      intro loans values typed _
      obtain ⟨list⟩ := values
      cases typed
      exact ⟨(), by simp <;> rfl⟩
  | .one τ, _, free, scalar => by
      simp only [ResultShape.row, NRow.resolved, NRow.scalarTypes?, Option.bind_eq_bind,
        Option.bind_eq_some_iff, Option.some.injEq] at scalar
      obtain ⟨type, head, _, tail, rfl⟩ := scalar
      cases tail
      have τFree : τ.refFree = true := by simpa [NRow.refFree] using free
      intro loans values typed holeFree
      obtain ⟨list⟩ := values
      cases typed with
      | cons headTyped tailTyped =>
          cases tailTyped with
          | nil =>
              rename_i value
              simp only [Outcome.holeFree, List.all_toArray, List.all_cons, List.all_nil,
                Bool.and_true] at holeFree
              obtain ⟨decoded, encoded⟩ := @NTy.decode_of_hasType unit (Skolems.runtime unit) loans
                (Skolems.resolve τ) type head value headTyped holeFree
              have fixed : SemanticOperations.normalize unit value = value := by
                rw [← encoded]
                exact @NTy.normalize_encode_scalar unit (Skolems.runtime unit) _ _ head decoded
              refine ⟨Skolems.ofRuntime τ decoded, ?_⟩
              simp only [ResultShape.lend, NTy.lend_refFree τ τFree false _ [], List.map_toArray,
                List.map_cons, List.map_nil, fixed]
              rw [← Skolems.encode_toRuntime, Skolems.toRuntime_ofRuntime]
              exact congrArg (fun raw => some #[raw]) encoded

theorem Denote.resultCodec_none_encode (result : ResultShape.none.carrier) :
    (resultCodec .none).encode result = #[] := rfl

theorem Denote.resultCodec_one_encode (τ : NTy) (result : τ.carrier) :
    (resultCodec (.one τ)).encode result = #[τ.encode result] := rfl

/-- A reference-free result lends as its encoding. -/
theorem Denote.ResultShape.lend_encode : (shape : ResultShape) → shape.row.refFree = true →
    (result : shape.carrier) → shape.lend false result [] = some ((resultCodec shape).encode result)
  | .none, _, _ => rfl
  | .one τ, free, result => by
      simp only [NRow.refFree, Bool.and_true] at free
      simp only [ResultShape.lend, NTy.lend_refFree τ free false result [], resultCodec_one_encode]

@[vir_denote_norm] theorem Denote.ResultShape.row_none : ResultShape.none.row = .nil := rfl
@[vir_denote_norm] theorem Denote.ResultShape.row_one (τ : NTy) :
    (ResultShape.one τ).row = .cons τ .nil := rfl

/-- `wp_named` where runs agree: the call's run also rules out an abort of
the function value naming the target at the call's entry values and, with
its returned reference settled, is the result and final memory `result_of`
and `state_of` read of it. -/
theorem wp_named_typed {unit : ValidatedUnit} (executable : ExecutableUnit unit) [Skolems unit]
    (agrees : RunsAgree executable)
    {handle : FunctionHandle} {typeInstantiation : Array (TypeId × TypeId)} {full : NRow}
    {shape : ResultShape} (fullFlat : full.lentFlat = true) (shapeFlat : shape.row.lentFlat = true)
    {shared : Sharing} {named : (NTy.function full shared shape.row).carrier}
    (naming : (Carriers.functions full shared shape.row).closure named =
      closureOf handle (Weave.supplying full).mask typeInstantiation (σs := .nil) ())
    {args : HList full} {ensures : shape.carrier → Memory unit → Prop} {aborts : Failure → Prop}
    {initial : Memory unit}
    (continued : wp (propheticMeaning executable typeInstantiation handle full shape args)
      (fun result final =>
        EnsuresOf (closureMeaning executable) full shared shape named args result initial final →
        ¬AbortsOf (closureMeaning executable) full shared shape named (HList.settle full args)
          initial →
        (shape.Settled result → ∀ default,
          ResultOf (closureMeaning executable) full shared shape named (HList.settle full args)
            initial default = result) →
        (shape.Settled result →
          StateOf (closureMeaning executable) full shared shape named (HList.settle full args)
            initial = final) →
        ensures result final)
      (fun error =>
        AbortsOf (closureMeaning executable) full shared shape named args initial → aborts error)
      initial) :
    wp (propheticMeaning executable typeInstantiation handle full shape args) ensures aborts
      initial :=
  have same := (HList.settle_settle full args).symm
  wp_named naming ⟨fun result final execution ensuresOf => continued.1 result final execution
      ensuresOf
      (not_abortsOf_of_ok agrees fullFlat same ensuresOf)
      (fun settled default => resultOf_eq_of_ok agrees fullFlat shapeFlat same ensuresOf settled
        default)
      (fun settled => stateOf_eq_of_ok agrees fullFlat shapeFlat same ensuresOf settled),
    continued.2.1, continued.2.2⟩

/-- The rule at an invocation of a function value of rows of scalars and type
parameters, typed at them: `wp_closureMeaning_unseen` with the typing and
decoding its typing states. -/
theorem wp_closureMeaning_unseen_at {unit : ValidatedUnit} (executable : ExecutableUnit unit)
    [Skolems unit] (natives : NativesTyped executable)
    (agrees : RunsAgree executable) {σs : NRow} {shared : Sharing} {shape : ResultShape}
    (function : (NTy.function σs shared shape.row).carrier)
    (args : HList σs) (invocable : σs.invocable = true) (shapeInvocable : shape.row.invocable = true)
    (free : σs.refFree = true) (shapeFree : shape.row.refFree = true)
    (typed : ClosureTypedAt unit σs shared shape.row function) {initial : Memory unit}
    {ensures : shape.carrier → Memory unit → Prop} {aborts : Failure → Prop}
    (returns : ∀ result final,
      EnsuresOf (closureMeaning executable) σs shared shape function args result initial final →
      ¬AbortsOf (closureMeaning executable) σs shared shape function args initial →
      (∀ default, ResultOf (closureMeaning executable) σs shared shape function args initial
        default = result) →
      StateOf (closureMeaning executable) σs shared shape function args initial = final →
      ensures result final)
    (fails : ∀ error, AbortsOf (closureMeaning executable) σs shared shape function args initial →
      aborts error) :
    wp (closureMeaning executable σs shared shape function args) ensures aborts initial := by
  obtain ⟨_, resultTypes, targetTypes, -, typed, -, decodes⟩ := typed invocable shapeInvocable
  exact wp_closureMeaning_unseen executable natives agrees function args free shapeFree typed
    (decodes shape rfl) returns fails

/-- Runs of the unit's functions end, at a family: an invocation of a typed
closure on typed arguments from typed global memory returns or aborts. -/
def TerminatingAt {unit : ValidatedUnit} (executable : ExecutableUnit unit) [Skolems unit] : Prop :=
  ∀ (closure : ClosureValue) (σs : NRow) (shape : ResultShape) (args : HList σs)
    (parameters results : List SemTy) (initial : Memory unit),
    σs.refFree = true → shape.row.refFree = true →
    ClosureTyped unit closure.function closure.mask closure.typeInstantiation closure.captures
      parameters results →
    HasTypes unit (fun _ => none) (HList.encode args) parameters →
    RowDecodes unit shape results → MemoryTyped unit initial →
    (∃ error, (closureRun executable closure σs shape args).aborts initial error) ∨
      ∃ result final, (closureRun executable closure σs shape args).ok initial result final

omit [Skolems unit] in
/-- Runs of the unit's functions end, at every family: the Move Prover's
assumption of every function. A theorem that reads `result_of` of a known
function value holds it as a hypothesis, which no proof discharges. -/
def Terminating {unit : ValidatedUnit}
    (executable : ExecutableUnit unit) : Prop := ∀ Θ : Skolems unit, @TerminatingAt _ executable Θ

/-- A typed invocation of rows of scalars and type parameters that does not
abort returns, where runs end: a run `ensures_of` names, whose result and
final memory `result_of` and `state_of` read. -/
theorem returns_of_terminating {unit : ValidatedUnit} (executable : ExecutableUnit unit)
    [Skolems unit] (terminating : Terminating executable)
    (agrees : RunsAgree executable) {σs : NRow} {shared : Sharing} {shape : ResultShape}
    (function : (NTy.function σs shared shape.row).carrier)
    (args : HList σs) (invocable : σs.invocable = true) (shapeInvocable : shape.row.invocable = true)
    (free : σs.refFree = true) (shapeFree : shape.row.refFree = true)
    (typed : ClosureTypedAt unit σs shared shape.row function)
    {initial : Memory unit} (globals : MemoryTyped unit initial)
    (unaborted : ¬AbortsOf (closureMeaning executable) σs shared shape function args initial) :
    ∃ result final,
      EnsuresOf (closureMeaning executable) σs shared shape function args result initial final ∧
      (∀ default, ResultOf (closureMeaning executable) σs shared shape function args initial
        default = result) ∧
      StateOf (closureMeaning executable) σs shared shape function args initial = final := by
  obtain ⟨parameterTypes, resultTypes, targetTypes, observed, typed, rowTyped, decodes⟩ :=
    typed invocable shapeInvocable
  rcases terminating ‹_› _ σs shape args targetTypes resultTypes initial free shapeFree
      typed (HasTypes.ofObserved (observed ▸ rowTyped args)) (decodes shape rfl) globals with
    ⟨error, aborted⟩ | ⟨result, final, run⟩
  · exact absurd ⟨error, aborted⟩ unaborted
  · have flat := NRow.lentFlat_of_refFree σs free
    have shapeFlat := NRow.lentFlat_of_refFree shape.row shapeFree
    have settled := ResultShape.settled_of_refFree shape shapeFree result
    exact ⟨result, final, run, resultOf_eq_of_ok agrees flat shapeFlat rfl run settled,
      stateOf_eq_of_ok agrees flat shapeFlat rfl run settled⟩

/-- The rows of semantic types a closure of a non-generic target is typed
at: its target's parameters split by the mask into the captured and the
supplied ones, and its results. Computed over a unit. -/
def closureTypes? (unit : ValidatedUnit) (function : FunctionHandle) (mask : Nat) :
    Option (List SemTy × List SemTy × List SemTy) := do
  let targetNs ← unit.namespaces[function.namespaceId.index]?
  let declaration ← targetNs.functions[function.functionId.index]?
  if !declaration.signature.generics.isEmpty ||
      !decide (mask < 2 ^ declaration.signature.parameters.size) then none
  let (allParameters, results) ← StaticTyping.signatureTypes? targetNs declaration
    (frameEnv declaration.signature.generics #[])
  some (ClosureMask.extract mask true allParameters, ClosureMask.extract mask false allParameters,
    results)

omit [Skolems unit] in
/-- A closure of a non-generic target, with captures of the captured types
its rows name, is typed at them. -/
theorem ClosureTyped.ofTypes {unit : ValidatedUnit} {function : FunctionHandle} {mask : Nat}
    {captures : Array RuntimeValue} {captured parameters results : List SemTy}
    (types : closureTypes? unit function mask = some (captured, parameters, results))
    (capturesTyped : ∃ represented : Array RuntimeValue,
      represented.map (SemanticOperations.normalize unit) = captures ∧
      HasTypes unit (fun _ => none) represented.toList captured) :
    ClosureTyped unit function mask #[] captures parameters results := by
  simp only [closureTypes?, Option.bind_eq_bind, Option.bind_eq_some_iff] at types
  obtain ⟨targetNs, namespace_eq, declaration, declaration_eq, types⟩ := types
  split at types
  · cases types
  next bounded =>
  simp only [Bool.or_eq_true, Bool.not_eq_true', decide_eq_false_iff_not, not_or,
    Bool.not_eq_false, Decidable.not_not] at bounded
  obtain ⟨generics, bound⟩ := bounded
  rw [Option.bind_eq_some_iff] at types
  obtain ⟨⟨allParameters, targetResults⟩, signature_eq, types⟩ := types
  simp only [Option.some.injEq, Prod.mk.injEq] at types
  obtain ⟨rfl, rfl, rfl⟩ := types
  have empty : frameEnv declaration.signature.generics #[] = #[] := by
    rw [Array.isEmpty_iff] at generics
    simp [frameEnv, StaticTyping.staticEnv, generics]
  refine ⟨targetNs, declaration, #[], allParameters, namespace_eq, declaration_eq, signature_eq,
    .inl ⟨rfl, fun _ _ _ resolves => ?_⟩, bound, capturesTyped, rfl⟩
  rwa [empty] at resolves

/-- A function value whose closure is a literal closure of a non-generic
target, built at a frame `Θc`, at rows whose types the frame resolves to the
scalar rows its target's types are or observe, is typed at them. -/
theorem ClosureTypedAt.ofClosureOf {unit : ValidatedUnit} [Skolems unit] {parameters results : NRow}
    {shared : Sharing} {handle : FunctionHandle} {mask : Nat} (Θc : Skolems unit)
    {capturedRow : NRow} (captures : @HList Θc.toCarriers capturedRow)
    (function : (NTy.function parameters shared results).carrier)
    (closed : (Carriers.functions parameters shared results).closure function =
      @closureOf _ Θc handle mask #[] capturedRow captures)
    {capturedTypes parameterTypes resultTypes : List SemTy}
    (capturedScalar : (@NRow.resolved _ Θc capturedRow).scalarTypes? = some capturedTypes)
    (parametersScalar : parameters.resolved.scalarTypes? = some parameterTypes)
    (resultsScalar : results.resolved.scalarTypes? = some resultTypes)
    (resultsFree : results.refFree = true)
    (types : (closureTypes? unit handle mask).map
      (fun (captured, targetTypes, results) => (captured, targetTypes.map observedType, results)) =
      some (capturedTypes, parameterTypes, resultTypes)) :
    ClosureTypedAt unit parameters shared results function := by
  intro _ _
  simp only [closed]
  obtain ⟨⟨captured, targetTypes, targetResults⟩, declared, mapped⟩ :=
    Option.map_eq_some_iff.mp types
  simp only [Prod.mk.injEq] at mapped
  obtain ⟨rfl, observed, rfl⟩ := mapped
  have typedCaptures := @RowTyped.ofResolved unit Θc capturedRow _ capturedScalar captures
  refine ⟨_, _, targetTypes, observed, ClosureTyped.ofTypes declared
    ⟨(HList.encode captures).toArray, ?_, by simpa using typedCaptures⟩,
    RowTyped.ofResolved parametersScalar, fun shape row => ?_⟩
  · show _ = (HList.encode captures).toArray
    rw [List.map_toArray, HasTypes.map_normalize_of_scalar _ capturedScalar typedCaptures]
  subst row
  exact RowDecodes.ofResolved shape resultsFree resultsScalar

/-- A function value whose runs on arguments of `parameters` from typed
global memory keep a frame: what a function-typed parameter's `modifies_of`
permits an invocation to change, at its arguments. Vacuous at rows with
references. -/
def FramedAt {unit : ValidatedUnit} (executable : ExecutableUnit unit) [Skolems unit]
    (parameters : NRow) (shared : Sharing) (shape : ResultShape)
    (frame : HList parameters → Memory unit → Memory unit → Prop)
    (function : (NTy.function parameters shared shape.row).carrier) : Prop :=
  parameters.refFree = true → shape.row.refFree = true →
    ∀ (args : HList parameters) (result : shape.carrier) (pre post : Memory unit),
      MemoryTyped unit pre →
      EnsuresOf (closureMeaning executable) parameters shared shape function args result pre post →
      frame args pre post

/-- A function value whose runs leave global memory as they found it: the
frame of a function-typed parameter that declares no `modifies_of`. -/
abbrev KeepsMemoryAt {unit : ValidatedUnit} (executable : ExecutableUnit unit) [Skolems unit]
    (parameters : NRow) (shared : Sharing) (shape : ResultShape)
    (function : (NTy.function parameters shared shape.row).carrier) : Prop :=
  FramedAt executable parameters shared shape (fun _ pre post => post = pre) function

/-- A frame holds of a function value wherever a narrower one does. -/
theorem FramedAt.mono {unit : ValidatedUnit} {executable : ExecutableUnit unit} [Skolems unit]
    {parameters : NRow} {shared : Sharing} {shape : ResultShape}
    {narrow wide : HList parameters → Memory unit → Memory unit → Prop}
    {function : (NTy.function parameters shared shape.row).carrier}
    (framed : FramedAt executable parameters shared shape narrow function)
    (within : ∀ args pre post, narrow args pre post → wide args pre post) :
    FramedAt executable parameters shared shape wide function :=
  fun free shapeFree args result pre post typed ensures =>
    within args pre post (framed free shapeFree args result pre post typed ensures)

/-- A frame at the frame a call's type arguments induce holds of a function
value where the frame read in the caller's view does at the caller's: the
value's runs at the induced frame are its runs at the caller's types
(`closureMeaning_instantiate`). -/
theorem FramedAt.of_instantiate {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    [Θ : Skolems unit] (θ : TypeArgs) (free : θ.1.refFree = true) {parameters : NRow}
    {shared : Sharing} {shape : ResultShape}
    {frame : @HList (Skolems.instantiate θ Θ).toCarriers parameters → Memory unit → Memory unit →
      Prop}
    {function : @NTy.carrier (Skolems.instantiate θ Θ).toCarriers (.function parameters shared shape.row)}
    (framed : FramedAt executable (NRow.subst θ.1 parameters) shared (shape.subst θ.1)
      (fun args => frame (HList.toSkolem θ parameters args))
      (shape.functionOfSkolem θ parameters shared function)) :
    @FramedAt unit executable (Skolems.instantiate θ Θ) parameters shared shape frame function := by
  intro parametersFree shapeFree args result pre post typed ensures
  unfold EnsuresOf at ensures
  rw [closureMeaning_instantiate executable θ free parameters shared shape function args] at ensures
  simp only [Spec.bind_ok, Spec.pure_ok] at ensures
  obtain ⟨value, middle, run, rfl, rfl⟩ := ensures
  have substFree : (shape.subst θ.1).row.refFree = true := by
    rw [ResultShape.row_subst]
    exact NRow.refFree_subst θ.1 free shape.row shapeFree
  have := framed (NRow.refFree_subst θ.1 free parameters parametersFree) substFree
    (HList.ofSkolem θ parameters args) value pre _ typed run
  simpa only [HList.toSkolem_ofSkolem] using this

theorem FramedAt.of_refFree_false {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    [Skolems unit] {parameters : NRow} {shared : Sharing} {shape : ResultShape}
    {frame : HList parameters → Memory unit → Memory unit → Prop}
    {function : (NTy.function parameters shared shape.row).carrier}
    (references : parameters.refFree = false) :
    FramedAt executable parameters shared shape frame function :=
  fun free => by simp [references] at free

/-- A literal closure keeps a frame where every run of its target from
typed memory, at any arguments, keeps it: the frame its body establishes,
whatever its precondition. -/
theorem FramedAt.ofBody {unit : ValidatedUnit} {executable : ExecutableUnit unit} [Skolems unit]
    {handle : FunctionHandle}
    {full captured supplied : NRow} (weave : Weave full captured supplied)
    (capturedFree : captured.refFree = true) {shared : Sharing} (shape : ResultShape)
    (typeInstantiation : Array (TypeId × TypeId))
    (coherent : Coherent unit handle typeInstantiation) (captures : HList captured)
    (typed : FunctionTyped supplied shared shape.row
      handle weave.mask typeInstantiation captures)
    {frame : HList supplied → Memory unit → Memory unit → Prop}
    (kept : ∀ (args : HList supplied) (pre : Memory unit), MemoryTyped unit pre →
      wp (propheticMeaning executable typeInstantiation handle full shape (weave.compose captures args))
        (fun _ post => frame args pre post) (fun _ => True) pre) :
    FramedAt executable supplied shared shape frame
      (functionOf supplied shared shape.row handle weave.mask typeInstantiation captures typed) := by
  intro _ _ args result pre post memoryTyped ensures
  have holds := kept args pre memoryTyped
  rw [← closureMeaning_closureOf_coherent executable weave capturedFree coherent captures shared
    shape typed args] at holds
  exact holds.1 result post ensures

/-- A literal closure of a verified target keeps a frame where the target's
theorem applies at every typed memory and its frame lies within it. -/
theorem FramedAt.ofVerified {unit : ValidatedUnit} {executable : ExecutableUnit unit} [Skolems unit]
    {handle : FunctionHandle}
    {full captured supplied : NRow} (weave : Weave full captured supplied)
    (capturedFree : captured.refFree = true) {shared : Sharing} (shape : ResultShape)
    (typeInstantiation : Array (TypeId × TypeId))
    (coherent : Coherent unit handle typeInstantiation) (captures : HList captured)
    (typed : FunctionTyped supplied shared shape.row
      handle weave.mask typeInstantiation captures)
    {contract : Contract (Memory unit) Failure (HList full) shape.carrier}
    (verified : Satisfies (propheticMeaning executable typeInstantiation handle full shape) contract)
    {frame : HList supplied → Memory unit → Memory unit → Prop}
    (held : ∀ (args : HList supplied) (pre : Memory unit), MemoryTyped unit pre →
      contract.assumes (weave.compose captures args) pre ∧
        contract.requires (weave.compose captures args) pre)
    (framed : ∀ (args : HList supplied) (pre post : Memory unit) (result : shape.carrier),
      contract.frame (weave.compose captures args) pre result post → frame args pre post) :
    FramedAt executable supplied shared shape frame
      (functionOf supplied shared shape.row handle weave.mask typeInstantiation captures typed) := by
  intro _ _ args result pre post memoryTyped ensures
  obtain ⟨assumed, permitted⟩ := held args pre memoryTyped
  exact framed args pre post result
    (ensuresOf_closureOf_verified weave capturedFree coherent captures args verified ensures
      assumed permitted).2.1

end Rows

/-! ## Frames and invocations over an invocation meaning

In a unit verified through its model (`doc/designs/ghost-fields.md`), a
function value's frame and the rule at an invocation the proof does not
see are stated over the model's invocation meaning, as the behavioral
predicates are (`EnsuresOfAt`, `AbortsOfAt`). -/

section Model

open Denote
variable {unit : ValidatedUnit} [Skolems unit]

/-- A function value whose invocations over a meaning, from typed memory,
keep a frame: what a function-typed parameter's `modifies_of` permits an
invocation to change, at its arguments. -/
def FramedAtOf (invoke : InvokeMeaning) (parameters : NRow) (shared : Sharing)
    (shape : ResultShape) (frame : HList parameters → Memory unit → Memory unit → Prop)
    (function : (NTy.function parameters shared shape.row).carrier) : Prop :=
  ∀ (args : HList parameters) (result : shape.carrier) (pre post : Memory unit),
    MemoryTyped unit pre → (invoke parameters shared shape function args).ok pre result post →
      frame args pre post

/-- A function value whose invocations over a meaning leave global memory as
they found it. -/
abbrev KeepsMemoryAtOf (invoke : InvokeMeaning) (parameters : NRow) (shared : Sharing)
    (shape : ResultShape) (function : (NTy.function parameters shared shape.row).carrier) : Prop :=
  FramedAtOf invoke parameters shared shape (fun _ pre post => post = pre) function

/-- A frame holds of a function value wherever a narrower one does. -/
theorem FramedAtOf.mono {invoke : InvokeMeaning} {parameters : NRow} {shared : Sharing}
    {shape : ResultShape} {narrow wide : HList parameters → Memory unit → Memory unit → Prop}
    {function : (NTy.function parameters shared shape.row).carrier}
    (framed : FramedAtOf invoke parameters shared shape narrow function)
    (within : ∀ args pre post, narrow args pre post → wide args pre post) :
    FramedAtOf invoke parameters shared shape wide function :=
  fun args result pre post typed run =>
    within args pre post (framed args result pre post typed run)

/-- A literal closure of a routed target keeps a frame where the target's
meaning on the woven arguments keeps it, from every typed memory. -/
theorem FramedAtOf.ofRouted {invoke : InvokeMeaning} {handle : FunctionHandle}
    {π captured supplied : NRow} {ρ : ResultShape} {self : HList π → Comp unit ρ.carrier}
    (weave : Weave π captured supplied) (typeInstantiation : Array (TypeId × TypeId))
    (captures : HList captured) {shared : Sharing} {shape : ResultShape}
    (typed : FunctionTyped supplied shared shape.row
      handle weave.mask typeInstantiation captures)
    (routes : ∀ (args : HList supplied),
      invoke supplied shared shape
          (functionOf supplied shared shape.row handle weave.mask typeInstantiation captures typed)
          args =
        invokeAt handle π ρ self supplied shared shape
          (functionOf supplied shared shape.row handle weave.mask typeInstantiation captures typed)
          args)
    {frame : HList supplied → Memory unit → Memory unit → Prop}
    (kept : ∀ (args : HList supplied) (pre : Memory unit), MemoryTyped unit pre →
      wp (self (weave.compose captures args)) (fun _ post => frame args pre post) (fun _ => True)
        pre) :
    FramedAtOf invoke supplied shared shape frame
      (functionOf supplied shared shape.row handle weave.mask typeInstantiation captures typed) := by
  intro args result pre post memoryTyped run
  rw [routes, invokeAt_closureOf_read] at run
  obtain ⟨value, middle, ran, read⟩ := (Spec.bind_ok _ _).mp run
  split at read
  · obtain ⟨_, same⟩ := (Spec.pure_ok).mp read
    rw [same]
    exact (kept args pre memoryTyped).1 value middle ran
  · exact ((Spec.stuck_ok _ _ _).mp read).elim

/-- An invocation of a function value the proof does not see, over a
meaning: defined by hypothesis, its runs and aborts stated by the
behavioral predicates. -/
theorem wp_invoke_unseen (invoke : InvokeMeaning) {σs : NRow} {shared : Sharing}
    {shape : ResultShape} (function : (NTy.function σs shared shape.row).carrier) (args : HList σs)
    {initial : Memory unit} {ensures : shape.carrier → Memory unit → Prop} {aborts : Failure → Prop}
    (defined : ¬(invoke σs shared shape function args).undefined initial)
    (returns : ∀ result final, EnsuresOf invoke σs shared shape function args result initial final →
      ensures result final)
    (fails : ∀ error, (invoke σs shared shape function args).aborts initial error →
      AbortsOf invoke σs shared shape function args initial → aborts error) :
    wp (invoke σs shared shape function args) ensures aborts initial :=
  ⟨returns, fun error aborted => fails error aborted ⟨error, aborted⟩, defined⟩

omit [Skolems unit] in
/-- What a verified function's contract states of one of its runs, where
what its theorem assumes and its precondition hold. -/
theorem runOfSatisfies {Args Result : Type} {function : Args → Comp unit Result}
    {contract : Contract (Memory unit) Failure Args Result} (verified : Satisfies function contract)
    {args : Args} {initial final : Memory unit} {result : Result}
    (run : (function args).ok initial result final)
    (assumed : contract.assumes args initial) (permitted : contract.requires args initial) :
    (¬contract.mayAbort args initial → contract.ensures args initial result final) ∧
      contract.frame args initial result final ∧ ¬contract.mustAbort args initial :=
  (verified args initial assumed permitted).1 result final run

omit [Skolems unit] in
/-- An abort of a verified function is one its contract permits, where what
its theorem assumes and its precondition hold. -/
theorem abortOfSatisfies {Args Result : Type} {function : Args → Comp unit Result}
    {contract : Contract (Memory unit) Failure Args Result} (verified : Satisfies function contract)
    {args : Args} {initial : Memory unit}
    (aborted : ∃ error, (function args).aborts initial error)
    (assumed : contract.assumes args initial) (permitted : contract.requires args initial) :
    ∃ error, contract.aborts args initial error :=
  aborted.imp fun error h => (verified args initial assumed permitted).2.1 error h

omit [Skolems unit] in
/-- A run of a meaning refining another is a run of the other. -/
theorem okOfRefines {α : Type} {left right : Comp unit α} (refines : Spec.Refines left right)
    {initial final : Memory unit} {result : α} (run : left.ok initial result final) :
    right.ok initial result final :=
  refines.ok initial result final run

omit [Skolems unit] in
/-- An abort of a meaning refining another is an abort of the other. -/
theorem abortsOfRefines {α : Type} {left right : Comp unit α} (refines : Spec.Refines left right)
    {initial : Memory unit} (aborted : ∃ error, left.aborts initial error) :
    ∃ error, right.aborts initial error :=
  aborted.imp fun error h => refines.aborts initial error h

end Model

end VerexIR.Proofs

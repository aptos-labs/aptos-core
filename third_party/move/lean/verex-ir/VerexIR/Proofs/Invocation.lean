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
(`designs/static-typing.md`, "Phase 5"); the invocation then has the weakest
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
  | .nominal source variant fields _ _ _ _ _ _ _ _ typed =>
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
  ∀ start : RuntimeState, Denote.Encodes unit memory start.globals → GlobalsTyped unit start

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
    exact Denote.ResourceType.erase_plain _ value
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
    HasTypes unit (fun _ => none) captures.toList
      (ClosureMask.extract mask true allParameters) ∧
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

/-- Results of `results` that hold no hole decode at a result shape. -/
def RowDecodes (unit : ValidatedUnit) [Skolems unit] (shape : ResultShape)
    (results : List SemTy) : Prop :=
  ∀ (loans : LoanTypes) (values : Array RuntimeValue), HasTypes unit loans values.toList results →
    (Outcome.returned values).holeFree = true → ∃ result, shape.lend false result [] = some values

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

/-- Arguments lending as their encodings lend so, under no loan. -/
theorem lendArguments_lendsEncoded {σs : NRow} (free : σs.lendsEncoded = true) {args : HList σs}
    {loans : List Nat} {supplied : Array RuntimeValue}
    (lent : lendArguments σs args loans = some supplied) :
    loans = [] ∧ supplied = (HList.encode args).toArray := by
  unfold lendArguments at lent
  rw [NRow.lend_lendsEncoded σs free false args loans] at lent
  cases loans with
  | nil => simp only [Option.some.injEq] at lent; exact ⟨rfl, lent.symm⟩
  | cons _ _ => simp at lent

/-- A run of an invocation is one `ensures_of` states: from a start at the
initial state, on the encoded arguments, returning the encoded result. -/
theorem closureMeaning_ok_ensuresOf {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    [Skolems unit] {closure : ClosureValue}
    {σs : NRow} {shape : ResultShape} {args : HList σs} (free : σs.lendsEncoded = true)
    (shapeFree : shape.row.lendsEncoded = true) {initial final : Memory unit}
    {result : shape.carrier}
    (run : (closureMeaning executable closure σs shape args).ok initial result final) :
    ∃ values, shape.lend false result [] = some values ∧
      EnsuresOf executable closure.encode (HList.encode args) values initial final := by
  obtain ⟨start, loans, supplied, arguments, results, exit, returnedLoans, prophecyRow,
    globals_eq, admissible, lend_eq, compose_eq, step, lend_false, lend_true, -, encoded, agree⟩ := run
  obtain ⟨rfl, rfl⟩ := lendArguments_lendsEncoded free lend_eq
  cases ResultShape.lend_refFree_loans shape (NRow.refFree_of_lendsEncoded shapeFree) lend_false
  rw [ResultShape.lend_refFree_view shape (NRow.refFree_of_lendsEncoded shapeFree), lend_false] at lend_true
  have sameRow : prophecyRow = results := (Option.some.inj lend_true).symm
  subst prophecyRow
  refine ⟨results, lend_false, start, exit, ⟨globals_eq.stores, admissible⟩, ?_, encoded.stores,
    agree⟩
  simp only [ClosureValue.encode, invocationSpec]
  simp only at compose_eq
  rw [compose_eq]
  exact step

/-- An abort of an invocation is one `aborts_of` states. -/
theorem closureMeaning_aborts_abortsOf {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    [Skolems unit] {closure : ClosureValue}
    {σs : NRow} {shape : ResultShape} {args : HList σs} (free : σs.lendsEncoded = true)
    {initial : Memory unit} {error : Failure}
    (aborted : (closureMeaning executable closure σs shape args).aborts initial error) :
    AbortsOf executable closure.encode (HList.encode args) initial := by
  obtain ⟨start, loans, supplied, arguments, globals_eq, admissible, lend_eq, compose_eq,
    step⟩ := aborted
  obtain ⟨rfl, rfl⟩ := lendArguments_lendsEncoded free lend_eq
  refine ⟨start, error, ⟨globals_eq.stores, admissible⟩, ?_⟩
  simp only [ClosureValue.encode, invocationSpec]
  simp only at compose_eq
  rw [compose_eq]
  exact step

/-- An invocation of a typed closure on typed arguments from typed global
memory has no undefined outcome: every run of its target returns results
that decode at the invocation's type. -/
theorem closureMeaning_defined {unit : ValidatedUnit} (executable : ExecutableUnit unit)
    [Skolems unit] (natives : NativesTyped executable)
    (closure : ClosureValue) {σs : NRow} {shape : ResultShape} (args : HList σs)
    (free : σs.lendsEncoded = true) (shapeFree : shape.row.lendsEncoded = true)
    {parameters results : List SemTy}
    (typed : ClosureTyped unit closure.function closure.mask closure.typeInstantiation
      closure.captures parameters results)
    (argumentsTyped : HasTypes unit (fun _ => none) (HList.encode args) parameters)
    (decodes : RowDecodes unit shape results)
    {initial : Memory unit} (globals : MemoryTyped unit initial) :
    ¬(closureMeaning executable closure σs shape args).undefined initial := by
  rintro ⟨start, loans, supplied, arguments, values, exit, globals_eq, -, lend_eq, compose_eq,
    run, undecoded⟩
  have lent : lendArguments σs args [] = some (HList.encode args).toArray := by
    unfold lendArguments
    rw [NRow.lend_lendsEncoded σs free false args []]
  cases loans with
  | cons _ _ =>
      unfold lendArguments at lend_eq
      rw [NRow.lend_lendsEncoded σs free false args _] at lend_eq
      simp at lend_eq
  | nil =>
  rw [lent, Option.some.injEq] at lend_eq
  subst lend_eq
  obtain ⟨targetNs, declaration, semantic, allParameters, ns_eq, declaration_eq, signature_eq,
    faithful, mask_bound, captures_typed, rfl⟩ := typed
  have composed := HasTypes.compose
    (by rw [signatureTypes?_length signature_eq]; exact mask_bound) captures_typed
    (by simpa using argumentsTyped) compose_eq
  have startTyped : TypedState unit (fun _ => none) start.pending.size start :=
    { globals := globals start globals_eq.stores.globals
      inert_le := Nat.le_refl _
      pending := fun _ member => by simp at member
      bounded := fun _ _ none_eq => by simp at none_eq }
  obtain ⟨loans', -, -, returned⟩ := preservation natives run (fun _ => none) start.pending.size
    targetNs declaration semantic allParameters results ns_eq declaration_eq faithful signature_eq
    (by simpa using composed) startTyped
  obtain ⟨result, lend_result⟩ :=
    decodes loans' values (returned values rfl) (EvalFunction_holeFree run)
  refine undecoded ⟨result, [], args, lend_result,
    (ResultShape.lend_refFree shape (NRow.refFree_of_lendsEncoded shapeFree) result).trans lend_result,
    lent, ?_, argumentsResolve_refFree σs (NRow.refFree_of_lendsEncoded free) args values _, ?_⟩
  · rw [HList.tables_eq_nil false σs (NRow.tableFree_of_lendsEncoded free)]
    exact TableMemory.TablesIn.nil _
  · rw [HList.tables_eq_nil true σs (NRow.tableFree_of_lendsEncoded free),
      ResultShape.tables_eq_nil true shape (NRow.tableFree_of_lendsEncoded shapeFree)]
    exact TableMemory.TablesIn.nil _

/-- The weakest precondition of such an invocation: what its runs and aborts
establish. -/
theorem wp_closureMeaning_typed {unit : ValidatedUnit} (executable : ExecutableUnit unit)
    [Skolems unit] (natives : NativesTyped executable)
    (closure : ClosureValue) {σs : NRow} {shape : ResultShape} (args : HList σs)
    (free : σs.lendsEncoded = true) (shapeFree : shape.row.lendsEncoded = true)
    {parameters results : List SemTy}
    (typed : ClosureTyped unit closure.function closure.mask closure.typeInstantiation
      closure.captures parameters results)
    (argumentsTyped : HasTypes unit (fun _ => none) (HList.encode args) parameters)
    (decodes : RowDecodes unit shape results)
    {initial : Memory unit} (globals : MemoryTyped unit initial)
    (ensures : shape.carrier → Memory unit → Prop) (aborts : Failure → Prop) :
    wp (closureMeaning executable closure σs shape args) ensures aborts initial ↔
      (∀ result final, (closureMeaning executable closure σs shape args).ok initial result final →
        ensures result final) ∧
      (∀ error, (closureMeaning executable closure σs shape args).aborts initial error →
        aborts error) :=
  wp_total_iff (closureMeaning_defined executable natives closure args free shapeFree typed
    argumentsTyped decodes globals)

end

section Agreement

open SemanticOperations
open BigStep

/-- Two runs from starts with the same global and Table storage holding no
loan, each lending distinct unregistered loans, on arguments holding only
those loans that agree once both starts' lent loans are placed at the same
positions: the runs agree up to injective renamings of their loans. -/
theorem evalFunction_agree {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    (natives : NativesShift executable)
    {handle : FunctionHandle} {instantiation : Array (TypeId × TypeId)}
    {start start₂ final final₂ : RuntimeState} {arguments arguments₂ : Array RuntimeValue}
    {outcome outcome₂ : Outcome} {loans loans₂ : List Nat}
    (globals_eq : start₂.globals = start.globals)
    (tables_eq : start₂.tables = start.tables)
    (plain : ∀ slot ∈ start.globals.entries, Plain slot.value)
    (tables_plain : ∀ slot ∈ start.tables.contents.entries, Plain slot.value)
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
      final.tables.rename ρ = final₂.tables.rename ρ₂ := by
  obtain ⟨nodup, lent, fresh⟩ := admissible
  obtain ⟨nodup₂, lent₂, fresh₂⟩ := admissible₂
  have registered : ∀ entry ∈ start₂.storageLoans,
      entry.1 < max start.nextLoan start₂.nextLoan := fun entry member =>
    Nat.lt_of_lt_of_le (fresh₂.below entry member) (Nat.le_max_right _ _)
  obtain ⟨common, mirror, globals, tables⟩ := runs_mirror natives run
    (start₂ := { start₂ with nextLoan := max start.nextLoan start₂.nextLoan + loans.length })
    globals_eq tables_eq plain tables_plain nodup (fun loan member => (lent loan member).1)
    (fun loan member => (lent loan member).2) fresh (Nat.le_max_left _ _) rfl registered within
  obtain ⟨common₂, mirror₂, globals₂, tables₂⟩ := runs_mirror natives run₂
    (start₂ := { start₂ with nextLoan := max start.nextLoan start₂.nextLoan + loans.length })
    rfl rfl (by rw [globals_eq]; exact plain) (by rw [tables_eq]; exact tables_plain) nodup₂
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
    (tables_eq : start₂.tables = start.tables)
    (plain : ∀ slot ∈ start.globals.entries, Plain slot.value)
    (tables_plain : ∀ slot ∈ start.tables.contents.entries, Plain slot.value)
    (admissible : Denote.Admissible start []) (admissible₂ : Denote.Admissible start₂ [])
    (arguments_plain : ∀ argument ∈ arguments, Plain argument)
    (run : EvalFunction executable handle instantiation start arguments final outcome)
    (run₂ : EvalFunction executable handle instantiation start₂ arguments final₂ outcome₂) :
    ∃ ρ ρ₂, Function.Injective ρ ∧ Function.Injective ρ₂ ∧
      outcome.rename ρ = outcome₂.rename ρ₂ ∧
      final.globals.rename ρ = final₂.globals.rename ρ₂ ∧
      final.tables.rename ρ = final₂.tables.rename ρ₂ :=
  evalFunction_agree natives globals_eq tables_eq plain tables_plain admissible admissible₂ rfl
    (fun argument member => RuntimeValue.within_of_plain _ (arguments_plain argument member))
    (fun argument member => RuntimeValue.within_of_plain _ (arguments_plain argument member))
    (fun _ => by
      rw [Array.map_rename_of_plain _ arguments_plain, Array.map_rename_of_plain _ arguments_plain])
    run run₂

/-- The arguments an invocation of a closure holding no loan runs its
target on, from supplied arguments holding none, hold none. -/
theorem invocationSpec_plain {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    {callable : RuntimeValue}
    {arguments : List RuntimeValue} (callable_plain : Plain callable)
    (arguments_plain : ∀ argument ∈ arguments, Plain argument) :
    invocationSpec executable callable arguments = Spec.bottom ∨
      ∃ function typeInstantiation composed,
        invocationSpec executable callable arguments =
          functionSpecAt executable function typeInstantiation composed ∧
        ∀ argument ∈ composed, Plain argument := by
  cases callable_plain with
  | closure function mask typeInstantiation captures captures_plain =>
      simp only [invocationSpec]
      cases compose_eq : ClosureMask.compose mask captures.toList arguments with
      | none => exact .inl rfl
      | some composed =>
          refine .inr ⟨function, typeInstantiation, composed.toArray, rfl, ?_⟩
          intro argument member
          rcases ClosureMask.compose_mem compose_eq argument (by simpa using member) with
            member | member
          · exact captures_plain argument (by simpa using member)
          · exact arguments_plain argument member
  | _ => exact .inl rfl

/-- An invocation that returns from a start at `state` does not abort from
any: runs from such starts agree. -/
theorem not_abortsOf_of_ok {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    (natives : NativesShift executable)
    {callable : RuntimeValue} {arguments : List RuntimeValue} {state : Denote.Memory unit}
    {start exit : RuntimeState} {results : Array RuntimeValue}
    (callable_plain : Plain callable) (arguments_plain : ∀ argument ∈ arguments, Plain argument)
    (starts : StartsAt start state)
    (ok : (invocationSpec executable callable arguments).ok start results exit) :
    ¬ AbortsOf executable callable arguments state := by
  rintro ⟨start₂, failure, starts₂, aborted⟩
  rcases invocationSpec_plain (executable := executable) callable_plain arguments_plain with
    bottom | ⟨function, typeInstantiation, composed, spec_eq, composed_plain⟩
  · rw [bottom] at ok; exact ok
  rw [spec_eq] at ok aborted
  obtain ⟨_, threw⟩ := aborted
  obtain ⟨_, _, -, -, renamed, -⟩ := evalFunction_agree_plain natives
    (starts₂.1.unique starts.1).1 (starts₂.1.unique starts.1).2
    starts.1.globals.plain starts.1.tables.plain starts.2 starts₂.2 composed_plain ok threw
  simp [Outcome.rename_returned, Outcome.rename_threw] at renamed

/-- `result_of` is the result of every run from a start at `state` that
returns results holding no loan. -/
theorem resultOf_eq_of_ok {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    (natives : NativesShift executable)
    {callable : RuntimeValue} {arguments : List RuntimeValue} {state post : Denote.Memory unit}
    {start exit : RuntimeState} {results : Array RuntimeValue}
    (callable_plain : Plain callable) (arguments_plain : ∀ argument ∈ arguments, Plain argument)
    (starts : StartsAt start state)
    (ok : (invocationSpec executable callable arguments).ok start results exit)
    (encoded : Denote.StorageEncodesReturned unit post exit results)
    (agree : Denote.AgreeUnnamed unit post state)
    (results_plain : ∀ result ∈ results, Plain result) :
    ResultOf executable callable arguments state = results := by
  have returns : ∃ results post, EnsuresOf executable callable arguments results state post :=
    ⟨results, post, start, exit, starts, ok, encoded, agree⟩
  unfold ResultOf
  rw [dif_pos returns]
  obtain ⟨_, start₂, exit₂, starts₂, ok₂, -⟩ := returns.choose_spec
  rcases invocationSpec_plain (executable := executable) callable_plain arguments_plain with
    bottom | ⟨function, typeInstantiation, composed, spec_eq, composed_plain⟩
  · rw [bottom] at ok; exact ok.elim
  rw [spec_eq] at ok ok₂
  obtain ⟨ρ, ρ₂, -, injective₂, renamed, -⟩ := evalFunction_agree_plain natives
    (starts₂.1.unique starts.1).1 (starts₂.1.unique starts.1).2
    starts.1.globals.plain starts.1.tables.plain starts.2 starts₂.2 composed_plain ok ok₂
  simp only [Outcome.rename_returned, Outcome.returned.injEq,
    Array.map_rename_of_plain ρ results_plain] at renamed
  refine (Array.map_inj_right fun _ _ => RuntimeValue.rename_injective injective₂).1 ?_
  rw [← renamed, Array.map_rename_of_plain ρ₂ results_plain]

/-- Equal runtime stores determine their named slots; agreement at unnamed
resource types determines the rest of a semantic memory. -/
private theorem memory_eq_of_encodes {unit : ValidatedUnit}
    {left right pre : Denote.Memory unit} {leftState rightState : RuntimeState}
    (leftEncoded : Denote.StorageEncodes unit left leftState)
    (rightEncoded : Denote.StorageEncodes unit right rightState)
    (same : leftState.globals = rightState.globals ∧ leftState.tables = rightState.tables)
    (leftAgree : Denote.AgreeUnnamed unit left pre)
    (rightAgree : Denote.AgreeUnnamed unit right pre) : left = right := by
  classical
  funext resource key
  by_cases globalNamed : ∃ namespaceId typeId,
      Denote.runtimeResourceOf unit namespaceId typeId = some resource
  · obtain ⟨namespaceId, typeId, named⟩ := globalNamed
    have encoded := leftEncoded.globals.2 namespaceId typeId key
    rw [same.1, rightEncoded.globals.2, named] at encoded
    simp only at encoded
    have ordinary := Denote.runtimeResourceOf_kind named
    have free := Denote.runtimeResourceOf_refFree named
    -- The physical values agree, and the tables both values hold are held at
    -- the same slots of the same storage: the values agree.
    rcases resource with ⟨type, arguments, kind⟩
    cases kind
    · cases foundLeft : left ⟨type, arguments, .value⟩ key with
      | none =>
          cases foundRight : right ⟨type, arguments, .value⟩ key with
          | none => rfl
          | some _ => rw [foundLeft, foundRight] at encoded; simp at encoded
      | some leftValue =>
          cases foundRight : right ⟨type, arguments, .value⟩ key with
          | none => rw [foundLeft, foundRight] at encoded; simp at encoded
          | some rightValue =>
              rw [foundLeft, foundRight] at encoded
              simp only [Option.map_some, Option.some.injEq] at encoded
              congr 1
              refine @Denote.NTy.eq_of_erase_eq (Denote.Carriers.runtime unit)
                (Denote.tableSlot? unit) leftState.tables.contents.lookup false type free leftValue
                rightValue encoded.symm
                (fun table member => leftEncoded.tables.held table
                  ⟨namespaceId, typeId, _, key, leftValue, named, foundLeft, member⟩)
                (fun table member => ?_)
              rw [same.2]
              exact rightEncoded.tables.held table
                ⟨namespaceId, typeId, _, key, rightValue, named, foundRight, member⟩
    · cases ordinary
  by_cases allocation : resource = Denote.TableMemory.allocationResource
  · subst resource
    rw [leftEncoded.tables.allocated, rightEncoded.tables.allocated, same.2]
  have unnamed : Denote.Unnamed unit resource :=
    ⟨fun ns ty named => globalNamed ⟨ns, ty, named⟩, allocation⟩
  exact congrFun ((leftAgree resource unnamed).trans (rightAgree resource unnamed).symm) key

/-- Successful invocations determine their labeled post-state independently
of the program points or loan identifiers of either execution. -/
theorem stateOf_eq_of_ensuresOf {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    (natives : NativesShift executable)
    {callable : RuntimeValue} {arguments : List RuntimeValue}
    {results : Array RuntimeValue} {pre post : Denote.Memory unit}
    (callable_plain : Plain callable) (arguments_plain : ∀ argument ∈ arguments, Plain argument)
    (ensures : EnsuresOf executable callable arguments results pre post) :
    StateOf executable callable arguments pre = post := by
  classical
  have returns : ∃ results post, EnsuresOf executable callable arguments results pre post :=
    ⟨results, post, ensures⟩
  unfold StateOf
  rw [dif_pos returns]
  obtain ⟨start, exit, starts, ok, encoded, agree⟩ := ensures
  obtain ⟨start₂, exit₂, starts₂, ok₂, encoded₂, agree₂⟩ := returns.choose_spec.choose_spec
  suffices same : (exit₂.resolveReturned returns.choose).globals =
        (exit.resolveReturned results).globals ∧
      (exit₂.resolveReturned returns.choose).tables = (exit.resolveReturned results).tables by
    exact memory_eq_of_encodes encoded₂ encoded same agree₂ agree
  rcases invocationSpec_plain (executable := executable) callable_plain arguments_plain with
    bottom | ⟨function, typeInstantiation, composed, spec_eq, composed_plain⟩
  · rw [bottom] at ok; exact ok.elim
  rw [spec_eq] at ok ok₂
  obtain ⟨ρ, ρ₂, injective, injective₂, renamed, globals, tables⟩ := evalFunction_agree_plain
    natives (starts₂.1.unique starts.1).1 (starts₂.1.unique starts.1).2
    starts.1.globals.plain starts.1.tables.plain starts.2 starts₂.2 composed_plain ok ok₂
  simp only [Outcome.rename_returned, Outcome.returned.injEq] at renamed
  have resolved := RuntimeState.resolveReturned_rename injective (state' := { exit with
    globals := exit.globals.rename ρ, tables := exit.tables.rename ρ }) rfl rfl results
  have resolved₂ := RuntimeState.resolveReturned_rename injective₂ (state' := { exit₂ with
    globals := exit₂.globals.rename ρ₂, tables := exit₂.tables.rename ρ₂ }) rfl rfl returns.choose
  rw [GlobalMap.rename_of_plain _ encoded.globals.plain,
    NativeTableStorage.rename_of_plain _ encoded.tables.plain] at resolved
  rw [GlobalMap.rename_of_plain _ encoded₂.globals.plain,
    NativeTableStorage.rename_of_plain _ encoded₂.tables.plain] at resolved₂
  rw [← resolved.1, ← resolved₂.1, ← resolved.2, ← resolved₂.2]
  simp only [RuntimeState.resolveReturned, renamed, globals, tables, and_self]

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

/-- A property of an invocation of a closure that does not hold of no
execution holds of a run of its target on the captures and the arguments
composed by its mask. -/
private theorem invocationSpec_closure {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    {function : FunctionHandle} {mask : Nat} {typeInstantiation : Array (TypeId × TypeId)}
    {captures : Array RuntimeValue} {arguments : List RuntimeValue}
    {property : Spec RuntimeState Failure (Array RuntimeValue) → Prop}
    (holds : property (invocationSpec executable (.closure function mask typeInstantiation captures)
      arguments))
    (bottom : ¬property Spec.bottom) :
    ∃ composed, ClosureMask.compose mask captures.toList arguments = some composed ∧
      property (functionSpecAt executable function typeInstantiation composed.toArray) := by
  simp only [invocationSpec] at holds
  cases composes : ClosureMask.compose mask captures.toList arguments with
  | none => rw [composes] at holds; exact absurd holds bottom
  | some composed => rw [composes] at holds; exact ⟨composed, rfl, holds⟩

/-- Runs of the target of a closure holding no loan, on its captures and
plain arguments with mutable ones lent from starts at the same memory,
agree up to injective renamings of their loans. -/
private theorem lentRuns_agree {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    (natives : NativesShift executable) {function : FunctionHandle} {mask : Nat}
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
    (run : EvalFunction executable function typeInstantiation start composed.toArray exit outcome)
    (run₂ : EvalFunction executable function typeInstantiation start₂ composed₂.toArray exit₂
      outcome₂) :
    ∃ ρ ρ₂, Function.Injective ρ ∧ Function.Injective ρ₂ ∧
      outcome.rename ρ = outcome₂.rename ρ₂ ∧
      exit.globals.rename ρ = exit₂.globals.rename ρ₂ ∧
      exit.tables.rename ρ = exit₂.tables.rename ρ₂ := by
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
  exact evalFunction_agree natives (encodes₂.unique encodes).1 (encodes₂.unique encodes).2
    encodes.globals.plain encodes.tables.plain admissible admissible₂ length_eq
    (within _ lends composes) (within _ lends₂ composes₂) arguments_eq run run₂

/-- An invocation lending mutable references that returns from a start at
`state` does not abort from any: runs from such starts agree. -/
theorem not_abortsOfMut_of_ok {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    (natives : NativesShift executable)
    {callable : RuntimeValue} {mutable : List Bool} {arguments : List RuntimeValue}
    {state : Denote.Memory unit} {start exit : RuntimeState} {loans : List Nat}
    {lent : List RuntimeValue} {returned : Array RuntimeValue}
    (callable_plain : Plain callable) (arguments_plain : ∀ argument ∈ arguments, Plain argument)
    (encodes : Denote.StorageEncodes unit state start) (admissible : Denote.Admissible start loans)
    (lends : lendMutable mutable arguments loans = some (lent, []))
    (ok : (invocationSpec executable callable lent).ok start returned exit) :
    ¬ AbortsOfMut executable callable mutable arguments state := by
  rintro ⟨start₂, loans₂, lent₂, failure, encodes₂, admissible₂, lends₂, aborted⟩
  cases callable_plain with
  | closure function mask typeInstantiation captures captures_plain =>
      obtain ⟨composed, composes, run⟩ :=
        invocationSpec_closure (property := fun spec => spec.ok start returned exit) ok id
      obtain ⟨composed₂, composes₂, _, run₂⟩ :=
        invocationSpec_closure (property := fun spec => spec.aborts start₂ failure) aborted id
      obtain ⟨_, _, -, -, renamed, -⟩ := lentRuns_agree natives captures_plain arguments_plain
        encodes encodes₂ admissible admissible₂ lends lends₂ composes composes₂ run run₂
      simp [Outcome.rename_returned, Outcome.rename_threw] at renamed
  | _ => exact ok

/-- `result_of` of an invocation lending mutable references is the results
every run from a start at `state` returns, read as `EnsuresOfMut` reads
them, where those hold no loan. -/
theorem resultOfMut_eq_of_ensuresOfMut {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    (natives : NativesShift executable)
    {callable : RuntimeValue} {mutable mutableResults : List Bool} {arguments : List RuntimeValue}
    {results : Array RuntimeValue} {finals : List RuntimeValue} {state post : Denote.Memory unit}
    (callable_plain : Plain callable) (arguments_plain : ∀ argument ∈ arguments, Plain argument)
    (ensures : EnsuresOfMut executable callable mutable mutableResults arguments results finals
      state post)
    (results_plain : ∀ result ∈ results, Plain result) :
    ResultOfMut executable callable mutable mutableResults arguments state = results := by
  have returns : ∃ results finals post,
      EnsuresOfMut executable callable mutable mutableResults arguments results finals state post :=
    ⟨results, finals, post, ensures⟩
  unfold ResultOfMut
  rw [dif_pos returns]
  obtain ⟨start, loans, lent, returned, exit, encodes, admissible, lends, ok, read, -⟩ := ensures
  obtain ⟨start₂, loans₂, lent₂, returned₂, exit₂, encodes₂, admissible₂, lends₂, ok₂, read₂, -⟩ :=
    returns.choose_spec.choose_spec.choose_spec
  cases callable_plain with
  | closure function mask typeInstantiation captures captures_plain =>
      obtain ⟨composed, composes, run⟩ :=
        invocationSpec_closure (property := fun spec => spec.ok start returned exit) ok id
      obtain ⟨composed₂, composes₂, run₂⟩ :=
        invocationSpec_closure (property := fun spec => spec.ok start₂ returned₂ exit₂) ok₂ id
      obtain ⟨ρ, ρ₂, -, injective₂, renamed, -⟩ := lentRuns_agree natives captures_plain
        arguments_plain encodes encodes₂ admissible admissible₂ lends lends₂ composes composes₂
        run run₂
      simp only [Outcome.rename_returned, Outcome.returned.injEq] at renamed
      subst read
      rw [read₂]
      congr 1
      exact readResults_agree injective₂ mutableResults returned.toList returned₂.toList
        (by simpa only [Array.toList_map] using congrArg Array.toList renamed)
        (fun result member => results_plain result (by simpa using member))
  | _ => exact ok.elim

end Agreement

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
      exact NTy.erase_plain τ result

/-- An invocation of a typed closure a proof cannot see, on typed arguments
from typed global memory, has the weakest precondition its behavioral
predicates state: a run returns results `ensures_of` names, `aborts_of`
does not hold, and the results are `result_of`; an abort is one
`aborts_of` names. -/
theorem wp_closureMeaning_unseen {unit : ValidatedUnit} (executable : ExecutableUnit unit)
    [Skolems unit] (natives : NativesTyped executable)
    (shifts : NativesShift executable) (closure : ClosureValue) {σs : NRow} {shape : ResultShape}
    (args : HList σs) (free : σs.lendsEncoded = true) (shapeFree : shape.row.lendsEncoded = true)
    {parameters results : List SemTy}
    (typed : ClosureTyped unit closure.function closure.mask closure.typeInstantiation
      closure.captures parameters results)
    (argumentsTyped : HasTypes unit (fun _ => none) (HList.encode args) parameters)
    (decodes : RowDecodes unit shape results)
    {initial : Memory unit} (globals : MemoryTyped unit initial)
    {ensures : shape.carrier → Memory unit → Prop} {aborts : Failure → Prop}
    (returns : ∀ result final values, shape.lend false result [] = some values →
      EnsuresOf executable closure.encode (HList.encode args) values initial final →
      ¬AbortsOf executable closure.encode (HList.encode args) initial →
      ResultOf executable closure.encode (HList.encode args) initial = values →
      StateOf executable closure.encode (HList.encode args) initial = final → ensures result final)
    (fails : ∀ error, AbortsOf executable closure.encode (HList.encode args) initial → aborts error) :
    wp (closureMeaning executable closure σs shape args) ensures aborts initial := by
  have callable_plain : Plain closure.encode := .closure _ _ _ _ closure.plain
  have arguments_plain : ∀ argument ∈ HList.encode args, Plain argument := rowCodec_plain σs args
  refine (wp_closureMeaning_typed executable natives closure args free shapeFree typed argumentsTyped
    decodes globals ensures aborts).mpr
    ⟨fun result final run => ?_, fun error aborted => fails error
      (closureMeaning_aborts_abortsOf free aborted)⟩
  obtain ⟨values, lent, ensuresOf⟩ := closureMeaning_ok_ensuresOf free shapeFree run
  obtain ⟨start, exit, starts, ok, encoded, agree⟩ := id ensuresOf
  exact returns result final values lent ensuresOf
    (not_abortsOf_of_ok shifts callable_plain arguments_plain starts ok)
    (resultOf_eq_of_ok shifts callable_plain arguments_plain starts ok encoded agree
      (ResultShape.lend_plain shape (NRow.refFree_of_lendsEncoded shapeFree) lent))
    (stateOf_eq_of_ensuresOf shifts callable_plain arguments_plain ensuresOf)

end Rule

section Rows

open Denote
variable {unit : Validation.ValidatedUnit} [Skolems unit]

/-- The semantic type of a scalar native type: what its encodings inhabit,
and a typed value decodes at. -/
def Denote.NTy.scalarType? : NTy → Option SemTy
  | .bool => some .bool
  | .int width signed => some (.integer (.bits width) signed)
  | .address => some .address
  | .signer => some .signer
  | .string => some .string
  | .bytes => some .bytes
  | .unit | .tuple _ | .struct .. | .enum .. | .vector _ | .table _ _ | .ref _ | .param _
  | .function .. => none

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
  | .ref _, _, scalar, _ | .param _, _, scalar, _ | .function .., _, scalar, _ => by
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
  | .ref _, _, scalar, _, _, _ | .param _, _, scalar, _, _, _ | .function .., _, scalar, _, _, _ => by
      simp [NTy.scalarType?] at scalar

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
          have free : τ.lendsEncoded = true := by
            cases τ <;> simp_all [NTy.scalarType?, NTy.lendsEncoded, NTy.refFree, NTy.tableFree]
          refine ⟨decoded, ?_⟩
          simp only [ResultShape.lend, NTy.lend_lendsEncoded τ free false decoded [], encoded]

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
def ClosureTypedAt (unit : ValidatedUnit) [Skolems unit] (parameters results : NRow)
    (closure : ClosureValue) : Prop :=
  parameters.invocable = true → results.invocable = true →
    ∃ parameterTypes resultTypes targetTypes, targetTypes.map observedType = parameterTypes ∧
      ClosureTyped unit closure.function closure.mask closure.typeInstantiation closure.captures
        targetTypes resultTypes ∧
      RowTyped unit parameters parameterTypes ∧
      ∀ shape : ResultShape, shape.row = results → RowDecodes unit shape resultTypes

omit [Skolems unit] in
/-- A row of scalar types lends as its encodings. -/
theorem Denote.NRow.lendsEncoded_of_scalarTypes : (row : NRow) → {types : List SemTy} →
    row.scalarTypes? = some types → row.lendsEncoded = true
  | .nil, _, _ => rfl
  | .cons τ rest, _, scalar => by
      simp only [NRow.scalarTypes?, Option.bind_eq_bind, Option.bind_eq_some_iff] at scalar
      obtain ⟨type, head, types, tail, -⟩ := scalar
      rw [NRow.lendsEncoded_cons]
      refine ⟨?_, NRow.lendsEncoded_of_scalarTypes rest tail⟩
      cases τ <;> simp_all [NTy.scalarType?, NTy.lendsEncoded, NTy.refFree, NTy.tableFree]

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
      exact ⟨(), rfl⟩
  | .one τ, _, scalar => by
      simp only [NRow.scalarTypes?, Option.bind_eq_bind, Option.bind_eq_some_iff,
        Option.some.injEq] at scalar
      obtain ⟨type, head, _, tail, rfl⟩ := scalar
      cases tail
      exact RowDecodes.scalar head

/-- Results of a shape lending as its encodings whose row the frame resolves
to scalars decode at their semantic types. -/
theorem RowDecodes.ofResolved {unit : ValidatedUnit} [Skolems unit] :
    (shape : ResultShape) → {types : List SemTy} → shape.row.lendsEncoded = true →
      shape.row.resolved.scalarTypes? = some types → RowDecodes unit shape types
  | .none, _, _, scalar => by
      simp only [ResultShape.row, NRow.resolved, NRow.scalarTypes?, Option.some.injEq] at scalar
      subst scalar
      intro loans values typed _
      obtain ⟨list⟩ := values
      cases typed
      exact ⟨(), rfl⟩
  | .one τ, _, free, scalar => by
      simp only [ResultShape.row, NRow.resolved, NRow.scalarTypes?, Option.bind_eq_bind,
        Option.bind_eq_some_iff, Option.some.injEq] at scalar
      obtain ⟨type, head, _, tail, rfl⟩ := scalar
      cases tail
      have τFree : τ.lendsEncoded = true := (NRow.lendsEncoded_singleton τ).mp free
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
              refine ⟨Skolems.ofRuntime τ decoded, ?_⟩
              simp only [ResultShape.lend, NTy.lend_lendsEncoded τ τFree false _ []]
              rw [← Skolems.encode_toRuntime, Skolems.toRuntime_ofRuntime]
              exact congrArg (fun raw => some #[raw]) encoded

@[vir_denote_norm] theorem Denote.resultCodec_none_encode (result : ResultShape.none.carrier) :
    (resultCodec .none).encode result = #[] := rfl

@[vir_denote_norm] theorem Denote.resultCodec_one_encode (τ : NTy) (result : τ.carrier) :
    (resultCodec (.one τ)).encode result = #[τ.encode result] := rfl

/-- A result lending as its encoding lends so. -/
theorem Denote.ResultShape.lend_encode : (shape : ResultShape) → shape.row.lendsEncoded = true →
    (result : shape.carrier) → shape.lend false result [] = some ((resultCodec shape).encode result)
  | .none, _, _ => rfl
  | .one τ, free, result => by
      have plain := (NRow.lendsEncoded_singleton τ).mp free
      simp only [ResultShape.lend, NTy.lend_lendsEncoded τ plain false result [],
        resultCodec_one_encode]

@[vir_denote_norm] theorem Denote.ResultShape.row_none : ResultShape.none.row = .nil := rfl
@[vir_denote_norm] theorem Denote.ResultShape.row_one (τ : NTy) :
    (ResultShape.one τ).row = .cons τ .nil := rfl

/-- `wp_named` from typed global memory: the call's run also rules out an
abort of the function value naming the target, and is the result
`result_of` reads of it. -/
theorem wp_named_typed {unit : ValidatedUnit} (executable : ExecutableUnit unit) [Skolems unit]
    (shifts : NativesShift executable)
    {handle : FunctionHandle} {typeInstantiation : Array (TypeId × TypeId)} {full : NRow}
    {shape : ResultShape} (fullFree : full.lendsEncoded = true) (shapeFree : shape.row.lendsEncoded = true)
    {args : HList full} {ensures : shape.carrier → Memory unit → Prop} {aborts : Failure → Prop}
    {initial : Memory unit}
    (named : wp (propheticMeaning executable typeInstantiation handle full shape args)
      (fun result final =>
        EnsuresOf executable (ClosureValue.encode (closureOf handle
          (Weave.supplying full).mask typeInstantiation (σs := .nil) ()))
          (HList.encode args) ((resultCodec shape).encode result) initial final →
        ¬AbortsOf executable (ClosureValue.encode (closureOf handle
          (Weave.supplying full).mask typeInstantiation (σs := .nil) ()))
          (HList.encode args) initial →
        ResultOf executable (ClosureValue.encode (closureOf handle
          (Weave.supplying full).mask typeInstantiation (σs := .nil) ()))
          (HList.encode args) initial = (resultCodec shape).encode result →
        StateOf executable (ClosureValue.encode (closureOf handle
          (Weave.supplying full).mask typeInstantiation (σs := .nil) ()))
          (HList.encode args) initial = final →
        ensures result final)
      (fun error =>
        AbortsOf executable (ClosureValue.encode (closureOf handle
          (Weave.supplying full).mask typeInstantiation (σs := .nil) ()))
          (HList.encode args) initial → aborts error)
      initial) :
    wp (propheticMeaning executable typeInstantiation handle full shape args) ensures aborts initial :=
  have callablePlain : SemanticOperations.Plain (ClosureValue.encode
      (closureOf handle (Weave.supplying full).mask typeInstantiation (σs := .nil) ())) :=
    .closure _ _ _ _ (closureOf handle (Weave.supplying full).mask typeInstantiation
      (σs := .nil) ()).plain
  have argumentsPlain : ∀ argument ∈ HList.encode args, SemanticOperations.Plain argument :=
    rowCodec_plain full args
  ⟨fun result final execution => by
      have ensuresOf := ensuresOf_of_run fullFree shapeFree execution
      obtain ⟨start, exit, starts, ok, encoded, agree⟩ := id ensuresOf
      have resultsPlain := ResultShape.lend_plain shape (NRow.refFree_of_lendsEncoded shapeFree)
        (ResultShape.lend_encode shape shapeFree result)
      exact named.1 result final execution ensuresOf
        (not_abortsOf_of_ok shifts callablePlain argumentsPlain starts ok)
        (resultOf_eq_of_ok shifts callablePlain argumentsPlain starts ok encoded agree
          resultsPlain)
        (stateOf_eq_of_ensuresOf shifts callablePlain argumentsPlain ensuresOf),
    fun error execution => named.2.1 error execution (abortsOf_of_abort fullFree execution),
    named.2.2⟩

/-- The entry values of a row of arguments hold no loan. -/
theorem HList.entries_plain : (row : NRow) → (values : HList row) →
    ∀ entry ∈ HList.entries values, SemanticOperations.Plain entry
  | .nil, _, _, member => nomatch member
  | .cons τ rest, values, entry, member => by
      cases τ <;> simp only [HList.entries, List.mem_cons] at member <;>
        rcases member with rfl | member <;>
        first | exact NTy.encode_plain _ _ | exact HList.entries_plain rest values.2 entry member

/-- `wp_named_mut` from typed global memory: the call's run also rules out an
abort of the function value naming the target and, with its returned
reference settled, is the result `result_of` reads of it. -/
theorem wp_named_mut_typed {unit : ValidatedUnit} (executable : ExecutableUnit unit)
    [Skolems unit] (shifts : NativesShift executable)
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
        ¬AbortsOfMut executable (ClosureValue.encode (closureOf handle
          (Weave.supplying full).mask typeInstantiation (σs := .nil) ()))
          full.mutable (HList.entries args) initial →
        (shape.Settled result →
          ResultOfMut executable (ClosureValue.encode (closureOf handle
            (Weave.supplying full).mask typeInstantiation (σs := .nil) ()))
            full.mutable shape.row.mutable (HList.entries args) initial = shape.current result) →
        ensures result final)
      (fun error =>
        AbortsOfMut executable (ClosureValue.encode (closureOf handle
          (Weave.supplying full).mask typeInstantiation (σs := .nil) ()))
          full.mutable (HList.entries args) initial → aborts error)
      initial) :
    wp (propheticMeaning executable typeInstantiation handle full shape args) ensures aborts initial :=
  have callablePlain : SemanticOperations.Plain (ClosureValue.encode
      (closureOf handle (Weave.supplying full).mask typeInstantiation (σs := .nil) ())) :=
    .closure _ _ _ _ (closureOf handle (Weave.supplying full).mask typeInstantiation
      (σs := .nil) ()).plain
  have entriesPlain := HList.entries_plain full args
  ⟨fun result final execution => by
      have ensuresOf := ensuresOfMut_of_run fullFlat shapeFlat execution
      have notAborts : ¬AbortsOfMut executable (ClosureValue.encode (closureOf handle
          (Weave.supplying full).mask typeInstantiation (σs := .nil) ()))
          full.mutable (HList.entries args) initial := by
        obtain ⟨-, start, loans, arguments, results, exit, -, -, globals, admissible, lent, ran, -⟩ :=
          execution
        obtain ⟨lentRow, lends, rfl⟩ := lendArguments_lentFlat fullFlat lent
        obtain ⟨-, invocation⟩ := invocationSpec_closureOf_lent (Weave.supplying full) rfl fullFlat
          typeInstantiation () args (executable := executable) (handle := handle) lends
        refine not_abortsOfMut_of_ok shifts callablePlain entriesPlain globals.stores admissible lends
          (returned := results) (exit := exit) ?_
        rw [invocation, HList.encode_nil, Weave.composeList_supplying]
        exact ran
      exact named.1 result final execution ensuresOf notAborts fun settled =>
        resultOfMut_eq_of_ensuresOfMut shifts callablePlain entriesPlain (ensuresOf settled)
          (ResultShape.current_plain shape result),
    fun error execution => named.2.1 error execution (abortsOfMut_of_abort fullFlat execution),
    named.2.2⟩

/-- The rule at an invocation of a function value of rows of scalars and type
parameters, typed at them: `wp_closureMeaning_unseen` with the typing and
decoding its typing states, stated at the function value's encoding and its
results' as a contract spells them. -/
theorem wp_closureMeaning_unseen_at {unit : ValidatedUnit} (executable : ExecutableUnit unit)
    [Skolems unit] (natives : NativesTyped executable)
    (shifts : NativesShift executable) (closure : ClosureValue) {σs : NRow} {shape : ResultShape}
    (args : HList σs) (invocable : σs.invocable = true) (shapeInvocable : shape.row.invocable = true)
    (free : σs.lendsEncoded = true) (shapeFree : shape.row.lendsEncoded = true)
    (typed : ClosureTypedAt unit σs shape.row closure)
    {initial : Memory unit} (globals : MemoryTyped unit initial)
    {ensures : shape.carrier → Memory unit → Prop} {aborts : Failure → Prop}
    (returns : ∀ result final,
      EnsuresOf executable (ClosureValue.encode closure) (HList.encode args)
        ((resultCodec shape).encode result) initial final →
      ¬AbortsOf executable (ClosureValue.encode closure) (HList.encode args) initial →
      ResultOf executable (ClosureValue.encode closure) (HList.encode args) initial =
        (resultCodec shape).encode result →
      StateOf executable (ClosureValue.encode closure) (HList.encode args) initial = final →
      ensures result final)
    (fails : ∀ error,
      AbortsOf executable (ClosureValue.encode closure) (HList.encode args) initial →
      aborts error) :
    wp (closureMeaning executable closure σs shape args) ensures aborts initial := by
  obtain ⟨parameterTypes, resultTypes, targetTypes, observed, typed, rowTyped, decodes⟩ :=
    typed invocable shapeInvocable
  refine wp_closureMeaning_unseen executable natives shifts closure args free shapeFree typed
    (HasTypes.ofObserved (observed ▸ rowTyped args)) (decodes shape rfl) globals
    (fun result final values lent ensured unaborted resulted state => ?_) fails
  rw [ResultShape.lend_encode shape shapeFree result, Option.some.injEq] at lent
  subst lent
  exact returns result final ensured unaborted resulted state

/-- `wp_closureMeaning_unseen_at` at a native function value, its behavior
stated at the value's encoding, as a contract spells the value. -/
theorem wp_closureMeaning_unseen_native {unit : ValidatedUnit} (executable : ExecutableUnit unit)
    [Skolems unit] (natives : NativesTyped executable)
    (shifts : NativesShift executable) {parameters : NRow} {shared : List Bool} {results : NRow}
    (function : (NTy.function parameters shared results).carrier) {σs : NRow} {shape : ResultShape}
    (args : HList σs) (invocable : σs.invocable = true) (shapeInvocable : shape.row.invocable = true)
    (free : σs.lendsEncoded = true) (shapeFree : shape.row.lendsEncoded = true)
    (typed : ClosureTypedAt unit σs shape.row function.val)
    {initial : Memory unit} (globals : MemoryTyped unit initial)
    {ensures : shape.carrier → Memory unit → Prop} {aborts : Failure → Prop}
    (returns : ∀ result final,
      EnsuresOf executable ((NTy.function parameters shared results).encode function)
        (HList.encode args) ((resultCodec shape).encode result) initial final →
      ¬AbortsOf executable ((NTy.function parameters shared results).encode function)
        (HList.encode args) initial →
      ResultOf executable ((NTy.function parameters shared results).encode function)
        (HList.encode args) initial = (resultCodec shape).encode result →
      StateOf executable ((NTy.function parameters shared results).encode function)
        (HList.encode args) initial = final →
      ensures result final)
    (fails : ∀ error,
      AbortsOf executable ((NTy.function parameters shared results).encode function)
        (HList.encode args) initial →
      aborts error) :
    wp (closureMeaning executable function.val σs shape args) ensures aborts initial :=
  wp_closureMeaning_unseen_at executable natives shifts function.val args invocable shapeInvocable
    free shapeFree typed globals returns fails

/-- Runs of the unit's functions end, at a family: an invocation of a typed
closure on typed arguments from typed global memory returns or aborts. -/
def TerminatingAt {unit : ValidatedUnit} (executable : ExecutableUnit unit) [Skolems unit] : Prop :=
  ∀ (closure : ClosureValue) (σs : NRow) (shape : ResultShape) (args : HList σs)
    (parameters results : List SemTy) (initial : Memory unit),
    σs.lendsEncoded = true → shape.row.lendsEncoded = true →
    ClosureTyped unit closure.function closure.mask closure.typeInstantiation closure.captures
      parameters results →
    HasTypes unit (fun _ => none) (HList.encode args) parameters →
    RowDecodes unit shape results → MemoryTyped unit initial →
    (∃ error, (closureMeaning executable closure σs shape args).aborts initial error) ∨
      ∃ result final, (closureMeaning executable closure σs shape args).ok initial result final

omit [Skolems unit] in
/-- Runs of the unit's functions end, at every family: the Move Prover's
assumption of every function. A theorem that reads `result_of` of a known
function value holds it as a hypothesis, which no proof discharges. -/
def Terminating {unit : ValidatedUnit}
    (executable : ExecutableUnit unit) : Prop := ∀ Θ : Skolems unit, @TerminatingAt _ executable Θ

/-- A typed invocation of rows of scalars and type parameters that does not
abort returns, where runs end: a run `ensures_of` names, whose results
`result_of` reads. -/
theorem returns_of_terminating {unit : ValidatedUnit} (executable : ExecutableUnit unit)
    [Skolems unit] (terminating : Terminating executable)
    (shifts : NativesShift executable) (closure : ClosureValue) {σs : NRow} {shape : ResultShape}
    (args : HList σs) (invocable : σs.invocable = true) (shapeInvocable : shape.row.invocable = true)
    (free : σs.lendsEncoded = true) (shapeFree : shape.row.lendsEncoded = true)
    (typed : ClosureTypedAt unit σs shape.row closure)
    {initial : Memory unit} (globals : MemoryTyped unit initial)
    (unaborted : ¬AbortsOf executable (ClosureValue.encode closure)
      (HList.encode args) initial) :
    ∃ result final,
      EnsuresOf executable (ClosureValue.encode closure) (HList.encode args)
        ((resultCodec shape).encode result) initial final ∧
      ResultOf executable (ClosureValue.encode closure) (HList.encode args) initial =
        (resultCodec shape).encode result ∧
      StateOf executable (ClosureValue.encode closure) (HList.encode args) initial = final := by
  have callable_plain : SemanticOperations.Plain closure.encode := .closure _ _ _ _ closure.plain
  have arguments_plain : ∀ argument ∈ HList.encode args, SemanticOperations.Plain argument :=
    rowCodec_plain σs args
  obtain ⟨parameterTypes, resultTypes, targetTypes, observed, typed, rowTyped, decodes⟩ :=
    typed invocable shapeInvocable
  rcases terminating ‹_› closure σs shape args targetTypes resultTypes initial free shapeFree
      typed (HasTypes.ofObserved (observed ▸ rowTyped args)) (decodes shape rfl) globals with
    ⟨_, aborted⟩ | ⟨result, final, run⟩
  · exact absurd (closureMeaning_aborts_abortsOf free aborted) unaborted
  · obtain ⟨values, lent, ensuresOf⟩ := closureMeaning_ok_ensuresOf free shapeFree run
    obtain ⟨_, _, starts, ok, encoded, agree⟩ := id ensuresOf
    have resulted := resultOf_eq_of_ok shifts callable_plain arguments_plain starts ok encoded
      agree (ResultShape.lend_plain shape (NRow.refFree_of_lendsEncoded shapeFree) lent)
    rw [ResultShape.lend_encode shape shapeFree result, Option.some.injEq] at lent
    subst lent
    exact ⟨result, final, ensuresOf, resulted,
      stateOf_eq_of_ensuresOf shifts callable_plain arguments_plain ensuresOf⟩

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
    (capturesTyped : HasTypes unit (fun _ => none) captures.toList captured) :
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

/-- A literal closure of a non-generic target, built at a frame `Θc`, at rows
whose types the frame resolves to the scalar rows its target's types are or
observe, is typed at them. -/
theorem ClosureTypedAt.ofClosureOf {unit : ValidatedUnit} [Skolems unit] {parameters results : NRow}
    {handle : FunctionHandle} {mask : Nat} (Θc : Skolems unit) {capturedRow : NRow}
    (captures : @HList Θc.toCarriers capturedRow)
    {capturedTypes parameterTypes resultTypes : List SemTy}
    (capturedScalar : (@NRow.resolved _ Θc capturedRow).scalarTypes? = some capturedTypes)
    (parametersScalar : parameters.resolved.scalarTypes? = some parameterTypes)
    (resultsScalar : results.resolved.scalarTypes? = some resultTypes)
    (resultsFree : results.lendsEncoded = true)
    (types : (closureTypes? unit handle mask).map
      (fun (captured, targetTypes, results) => (captured, targetTypes.map observedType, results)) =
      some (capturedTypes, parameterTypes, resultTypes)) :
    ClosureTypedAt unit parameters results (@closureOf _ Θc handle mask #[] capturedRow captures) := by
  intro _ _
  obtain ⟨⟨captured, targetTypes, targetResults⟩, declared, mapped⟩ :=
    Option.map_eq_some_iff.mp types
  simp only [Prod.mk.injEq] at mapped
  obtain ⟨rfl, observed, rfl⟩ := mapped
  refine ⟨_, _, targetTypes, observed, ClosureTyped.ofTypes declared (by
    simpa [Denote.closureOf] using @RowTyped.ofResolved unit Θc capturedRow _ capturedScalar
      captures), RowTyped.ofResolved parametersScalar, fun shape row => ?_⟩
  subst row
  exact RowDecodes.ofResolved shape resultsFree resultsScalar

/-- A function value whose runs on arguments of `parameters` from typed
global memory keep a frame: what a function-typed parameter's `modifies_of`
permits an invocation to change, at its arguments. Vacuous at rows with
references or map contents. -/
def FramedAt {unit : ValidatedUnit} (executable : ExecutableUnit unit) [Skolems unit]
    (parameters : NRow)
    (frame : HList parameters → Memory unit → Memory unit → Prop) (closure : ClosureValue) : Prop :=
  parameters.lendsEncoded = true →
    ∀ (args : HList parameters) (results : Array RuntimeValue) (pre post : Memory unit),
      MemoryTyped unit pre →
        EnsuresOf executable closure.encode (HList.encode args) results pre post → frame args pre post

/-- A function value whose runs leave global memory as they found it: the
frame of a function-typed parameter that declares no `modifies_of`. -/
abbrev KeepsMemoryAt {unit : ValidatedUnit} (executable : ExecutableUnit unit) [Skolems unit]
    (parameters : NRow) (closure : ClosureValue) :
    Prop :=
  FramedAt executable parameters (fun _ pre post => post = pre) closure

/-- The default frame of a Move function-valued field, expressed over its
runtime encoding so data invariants can carry it through structs and enums. -/
def EncodedKeepsMemory {unit : ValidatedUnit} (executable : ExecutableUnit unit)
    [Skolems unit] (parameters : NRow) (value : RuntimeValue) : Prop :=
  ∀ closure : ClosureValue, closure.encode = value → KeepsMemoryAt executable parameters closure

/-- A stored function's declared write frame, transported through its runtime
encoding just as the default read-only frame is. -/
def EncodedFramed {unit : ValidatedUnit} (executable : ExecutableUnit unit)
    [Skolems unit] (parameters : NRow)
    (frame : HList parameters → Memory unit → Memory unit → Prop) (value : RuntimeValue) : Prop :=
  ∀ closure : ClosureValue, closure.encode = value → FramedAt executable parameters frame closure

@[simp] theorem encodedFramed_encode {unit : ValidatedUnit}
    (executable : ExecutableUnit unit) [Skolems unit] (parameters : NRow)
    (frame : HList parameters → Memory unit → Memory unit → Prop) (closure : ClosureValue) :
    EncodedFramed executable parameters frame closure.encode ↔
      FramedAt executable parameters frame closure := by
  constructor
  · intro held
    exact held closure rfl
  · intro held other equal
    cases ClosureValue.encode_injective equal
    exact held

@[simp] theorem encodedFramed_function {unit : ValidatedUnit}
    (executable : ExecutableUnit unit) [Skolems unit] (parameters results : NRow)
    (mutable : List Bool) (frame : HList parameters → Memory unit → Memory unit → Prop)
    (closure : (NTy.function parameters mutable results).carrier) :
    EncodedFramed executable parameters frame
        ((NTy.function parameters mutable results).encode closure) ↔
      FramedAt executable parameters frame closure.val :=
  encodedFramed_encode executable parameters frame closure.val

@[simp] theorem encodedKeepsMemory_encode {unit : ValidatedUnit}
    (executable : ExecutableUnit unit) [Skolems unit] (parameters : NRow)
    (closure : ClosureValue) :
    EncodedKeepsMemory executable parameters closure.encode ↔
      KeepsMemoryAt executable parameters closure := by
  constructor
  · intro held
    exact held closure rfl
  · intro held other equal
    cases ClosureValue.encode_injective equal
    exact held

@[simp] theorem encodedKeepsMemory_function {unit : ValidatedUnit}
    (executable : ExecutableUnit unit) [Skolems unit] (parameters results : NRow)
    (mutable : List Bool) (closure : (NTy.function parameters mutable results).carrier) :
    EncodedKeepsMemory executable parameters ((NTy.function parameters mutable results).encode closure) ↔
      KeepsMemoryAt executable parameters closure.val :=
  encodedKeepsMemory_encode executable parameters closure.val

/-- A frame holds of a function value wherever a narrower one does. -/
theorem FramedAt.mono {unit : ValidatedUnit} {executable : ExecutableUnit unit} [Skolems unit]
    {parameters : NRow}
    {narrow wide : HList parameters → Memory unit → Memory unit → Prop} {closure : ClosureValue}
    (framed : FramedAt executable parameters narrow closure)
    (within : ∀ args pre post, narrow args pre post → wide args pre post) :
    FramedAt executable parameters wide closure :=
  fun free args results pre post typed ensures =>
    within args pre post (framed free args results pre post typed ensures)

theorem FramedAt.of_lendsEncoded_false {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    [Skolems unit] {parameters : NRow}
    {frame : HList parameters → Memory unit → Memory unit → Prop} {closure : ClosureValue}
    (held : parameters.lendsEncoded = false) : FramedAt executable parameters frame closure :=
  fun plain => by simp [held] at plain

/-- A run of a closure whose target, weave, and captures a proof sees, at
reference-free arguments and results: a run of the target's prophetic
meaning, or, where its results do not decode, an undefined outcome of it. -/
theorem ok_or_undefined_of_ensuresOf_closureOf {unit : ValidatedUnit}
    {executable : ExecutableUnit unit} [Skolems unit] {handle : FunctionHandle}
    {full captured supplied : NRow} (weave : Weave full captured supplied)
    (capturedFree : captured.lendsEncoded = true) (suppliedFree : supplied.lendsEncoded = true)
    {shape : ResultShape} (shapeFree : shape.row.lendsEncoded = true)
    (typeInstantiation : Array (TypeId × TypeId))
    (coherent : Coherent unit handle typeInstantiation)
    (captures : HList captured) (args : HList supplied) {results : Array RuntimeValue}
    {pre post : Memory unit}
    (ensures : EnsuresOf executable (closureOf handle weave.mask typeInstantiation captures).encode
      (HList.encode args) results pre post) :
    (∃ result, (propheticMeaning executable typeInstantiation handle full shape
        (weave.compose captures args)).ok pre result post) ∨
      (propheticMeaning executable typeInstantiation handle full shape
        (weave.compose captures args)).undefined pre := by
  obtain ⟨start, exit, ⟨globals, admissible⟩, runs, encoded, agree⟩ := ensures
  rw [invocationSpec_closureOf] at runs
  have lent := lendArguments_weave weave capturedFree suppliedFree captures args
  have none := weave.tables_nil capturedFree suppliedFree
  by_cases decodes : ∃ result, shape.lend false result [] = some results
  · obtain ⟨result, resultLent⟩ := decodes
    refine .inl ⟨result, coherent, start, [], _, results, exit, [], results,
      Encloses.of_nil globals (none false captures args), admissible,
      lent, runs, resultLent, (ResultShape.lend_refFree shape (NRow.refFree_of_lendsEncoded shapeFree) result).trans resultLent,
      ?_, ReturnedEncloses.of_nil encoded (by
        rw [none, ResultShape.tables_eq_nil true shape (NRow.tableFree_of_lendsEncoded shapeFree)]
        rfl), agree⟩
    exact (weave.argumentsResolve_compose (NRow.refFree_of_lendsEncoded capturedFree) captures args [] _ _).mpr
      (argumentsResolve_refFree supplied (NRow.refFree_of_lendsEncoded suppliedFree) args _ _)
  · refine .inr ⟨coherent, start, [], _, results, exit, Encloses.of_nil globals (none false captures args),
      admissible, lent, runs, ?_⟩
    rintro ⟨result, returnedLoans, -, resultLent, -, -, -, -, -⟩
    obtain rfl := ResultShape.lend_refFree_loans shape (NRow.refFree_of_lendsEncoded shapeFree) resultLent
    exact decodes ⟨result, resultLent⟩

/-- `ensures_of` of a closure whose target, weave, and captures a proof sees,
at reference-free arguments and results of any shape, is a run of the
target's prophetic meaning. -/
theorem ensuresOf_closureOf_shape {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    [Skolems unit] {handle : FunctionHandle}
    {full captured supplied : NRow} (weave : Weave full captured supplied)
    (capturedFree : captured.lendsEncoded = true) (suppliedFree : supplied.lendsEncoded = true)
    {shape : ResultShape} (shapeFree : shape.row.lendsEncoded = true)
    (typeInstantiation : Array (TypeId × TypeId))
    (coherent : Coherent unit handle typeInstantiation)
    (captures : HList captured) (args : HList supplied) (result : shape.carrier)
    {pre post : Memory unit}
    (ensures : EnsuresOf executable (closureOf handle weave.mask typeInstantiation captures).encode
      (HList.encode args) ((resultCodec shape).encode result) pre post) :
    (propheticMeaning executable typeInstantiation handle full shape
      (weave.compose captures args)).ok pre result post := by
  obtain ⟨start, exit, ⟨globals, admissible⟩, runs, encoded, agree⟩ := ensures
  rw [invocationSpec_closureOf] at runs
  have lent := ResultShape.lend_encode shape shapeFree result
  have none := weave.tables_nil capturedFree suppliedFree
  refine ⟨coherent, start, [], _, _, exit, [], _, Encloses.of_nil globals (none false captures args),
    admissible, lendArguments_weave weave capturedFree suppliedFree captures args, runs, lent,
    (ResultShape.lend_refFree shape (NRow.refFree_of_lendsEncoded shapeFree) result).trans lent, ?_,
    ReturnedEncloses.of_nil encoded (by
      rw [none, ResultShape.tables_eq_nil true shape (NRow.tableFree_of_lendsEncoded shapeFree)]
      rfl), agree⟩
  exact (weave.argumentsResolve_compose (NRow.refFree_of_lendsEncoded capturedFree) captures args [] _ _).mpr
    (argumentsResolve_refFree supplied (NRow.refFree_of_lendsEncoded suppliedFree) args _ _)

/-- `ensures_of` of a closure whose target is verified, at results of any
shape: where what the target's theorem assumes and its precondition hold,
what its contract ensures of the run, its frame, and that no condition
under which it must abort held. -/
theorem ensuresOf_closureOf_verified_shape {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    [Skolems unit] {handle : FunctionHandle}
    {full captured supplied : NRow} (weave : Weave full captured supplied)
    (capturedFree : captured.lendsEncoded = true) (suppliedFree : supplied.lendsEncoded = true)
    {shape : ResultShape} (shapeFree : shape.row.lendsEncoded = true)
    (typeInstantiation : Array (TypeId × TypeId))
    (coherent : Coherent unit handle typeInstantiation)
    (captures : HList captured) (args : HList supplied) (result : shape.carrier)
    {pre post : Memory unit} {contract : Contract (Memory unit) Failure (HList full) shape.carrier}
    (verified : Satisfies (propheticMeaning executable typeInstantiation handle full shape) contract)
    (ensures : EnsuresOf executable (closureOf handle weave.mask typeInstantiation captures).encode
      (HList.encode args) ((resultCodec shape).encode result) pre post)
    (assumed : contract.assumes (weave.compose captures args) pre)
    (permitted : contract.requires (weave.compose captures args) pre) :
    (¬contract.mayAbort (weave.compose captures args) pre →
        contract.ensures (weave.compose captures args) pre result post) ∧
      contract.frame (weave.compose captures args) pre result post ∧
      ¬contract.mustAbort (weave.compose captures args) pre :=
  (verified _ pre assumed permitted).1 result post
    (ensuresOf_closureOf_shape weave capturedFree suppliedFree shapeFree typeInstantiation
      coherent captures args result ensures)

/-- `ensures_of` of a closure whose target, weave, and captures a proof sees,
at arguments lending mutable references and results of any reference-free
shape, is a run of the target's prophetic meaning at the references' entry
and final values. -/
theorem ensuresOfMut_closureOf {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    [Skolems unit] {handle : FunctionHandle}
    {full captured supplied : NRow} (weave : Weave full captured supplied)
    (capturedFree : captured.lendsEncoded = true) (suppliedFlat : supplied.lentFlat = true)
    {shape : ResultShape} (shapeFree : shape.row.lendsEncoded = true)
    (typeInstantiation : Array (TypeId × TypeId))
    (coherent : Coherent unit handle typeInstantiation)
    (captures : HList captured) (args : HList supplied) (result : shape.carrier)
    {pre post : Memory unit}
    (ensures : EnsuresOfMut executable (closureOf handle weave.mask typeInstantiation captures).encode
      supplied.mutable shape.row.mutable (HList.entries args) ((resultCodec shape).encode result)
      (HList.finals args) pre post) :
    (propheticMeaning executable typeInstantiation handle full shape
      (weave.compose captures args)).ok pre result post := by
  obtain ⟨start, loans, lent, returned, exit, globals, admissible, lends, runs, read, resolves,
    encoded, agree⟩ := ensures
  simp only [readResults_of_all_false (NRow.mutable_refFree _ (NRow.refFree_of_lendsEncoded shapeFree)), Array.toArray_toList]
    at read
  subst read
  obtain ⟨lentArguments, invocation⟩ :=
    invocationSpec_closureOf_lent weave capturedFree suppliedFlat typeInstantiation captures args
      (executable := executable) (handle := handle) lends
  rw [invocation] at runs
  have resultLent := ResultShape.lend_encode shape shapeFree result
  have none := weave.tables_nil_of_lentFlat capturedFree suppliedFlat
  refine ⟨coherent, start, loans, _, _, exit, [], _, Encloses.of_nil globals (none false captures args),
    admissible, lentArguments, runs, resultLent,
    (ResultShape.lend_refFree shape (NRow.refFree_of_lendsEncoded shapeFree) result).trans resultLent, ?_,
    ReturnedEncloses.of_nil encoded (by
      rw [none, ResultShape.tables_eq_nil true shape (NRow.tableFree_of_lendsEncoded shapeFree)]
      rfl), agree⟩
  exact (weave.argumentsResolve_compose (NRow.refFree_of_lendsEncoded capturedFree) captures args loans _ _).mpr
    ((argumentsResolve_finals supplied suppliedFlat args loans _ _).mpr resolves)

/-- `aborts_of` of a closure whose target, weave, and captures a proof sees,
at arguments lending mutable references, is an abort of the target's
prophetic meaning at the references' entry values, whatever their final
values. -/
theorem abortsOfMut_closureOf {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    [Skolems unit] {handle : FunctionHandle}
    {full captured supplied : NRow} (weave : Weave full captured supplied)
    (capturedFree : captured.lendsEncoded = true) (suppliedFlat : supplied.lentFlat = true)
    (shape : ResultShape) (typeInstantiation : Array (TypeId × TypeId))
    (coherent : Coherent unit handle typeInstantiation)
    (captures : HList captured) (args : HList supplied) {state : Memory unit}
    (aborts : AbortsOfMut executable (closureOf handle weave.mask typeInstantiation captures).encode
      supplied.mutable (HList.entries args) state) :
    ∃ failure, (propheticMeaning executable typeInstantiation handle full shape
      (weave.compose captures args)).aborts state failure := by
  obtain ⟨start, loans, lent, failure, globals, admissible, lends, runs⟩ := aborts
  obtain ⟨lentArguments, invocation⟩ :=
    invocationSpec_closureOf_lent weave capturedFree suppliedFlat typeInstantiation captures args
      (executable := executable) (handle := handle) lends
  rw [invocation] at runs
  exact ⟨failure, coherent, start, loans, _,
    Encloses.of_nil globals (weave.tables_nil_of_lentFlat capturedFree suppliedFlat false captures args),
    admissible, lentArguments, runs⟩

/-- `ensures_of` of a closure taking mutable references whose target is
verified: where what the target's theorem assumes and its precondition
hold, what its contract ensures of the run, its frame, and that no
condition under which it must abort held. -/
theorem ensuresOfMut_closureOf_verified {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    [Skolems unit] {handle : FunctionHandle}
    {full captured supplied : NRow} (weave : Weave full captured supplied)
    (capturedFree : captured.lendsEncoded = true) (suppliedFlat : supplied.lentFlat = true)
    {shape : ResultShape} (shapeFree : shape.row.lendsEncoded = true)
    (typeInstantiation : Array (TypeId × TypeId))
    (coherent : Coherent unit handle typeInstantiation)
    (captures : HList captured) (args : HList supplied) (result : shape.carrier)
    {pre post : Memory unit} {contract : Contract (Memory unit) Failure (HList full) shape.carrier}
    (verified : Satisfies (propheticMeaning executable typeInstantiation handle full shape) contract)
    (ensures : EnsuresOfMut executable (closureOf handle weave.mask typeInstantiation captures).encode
      supplied.mutable shape.row.mutable (HList.entries args) ((resultCodec shape).encode result)
      (HList.finals args) pre post)
    (assumed : contract.assumes (weave.compose captures args) pre)
    (permitted : contract.requires (weave.compose captures args) pre) :
    (¬contract.mayAbort (weave.compose captures args) pre →
        contract.ensures (weave.compose captures args) pre result post) ∧
      contract.frame (weave.compose captures args) pre result post ∧
      ¬contract.mustAbort (weave.compose captures args) pre :=
  (verified _ pre assumed permitted).1 result post
    (ensuresOfMut_closureOf weave capturedFree suppliedFlat shapeFree typeInstantiation coherent
      captures args result ensures)

/-- `aborts_of` of a closure taking mutable references whose target is
verified: where what the target's theorem assumes and its precondition
hold, a failure its contract permits. -/
theorem abortsOfMut_closureOf_verified {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    [Skolems unit] {handle : FunctionHandle}
    {full captured supplied : NRow} (weave : Weave full captured supplied)
    (capturedFree : captured.lendsEncoded = true) (suppliedFlat : supplied.lentFlat = true)
    {shape : ResultShape} (typeInstantiation : Array (TypeId × TypeId))
    (coherent : Coherent unit handle typeInstantiation)
    (captures : HList captured) (args : HList supplied) {state : Memory unit}
    {contract : Contract (Memory unit) Failure (HList full) shape.carrier}
    (verified : Satisfies (propheticMeaning executable typeInstantiation handle full shape) contract)
    (aborts : AbortsOfMut executable (closureOf handle weave.mask typeInstantiation captures).encode
      supplied.mutable (HList.entries args) state)
    (assumed : contract.assumes (weave.compose captures args) state)
    (permitted : contract.requires (weave.compose captures args) state) :
    ∃ failure, contract.aborts (weave.compose captures args) state failure := by
  obtain ⟨failure, runs⟩ := abortsOfMut_closureOf weave capturedFree suppliedFlat shape
    typeInstantiation coherent captures args aborts
  exact ⟨failure, (verified _ state assumed permitted).2.1 failure runs⟩

/-- A run of a closure whose target is verified, from where what the
target's theorem assumes and its precondition hold, keeps the target's
frame. -/
theorem frame_of_ensuresOf_closureOf_verified {unit : ValidatedUnit}
    {executable : ExecutableUnit unit} [Skolems unit] {handle : FunctionHandle}
    {full captured supplied : NRow} (weave : Weave full captured supplied)
    (capturedFree : captured.lendsEncoded = true) (suppliedFree : supplied.lendsEncoded = true)
    {shape : ResultShape} (shapeFree : shape.row.lendsEncoded = true)
    (typeInstantiation : Array (TypeId × TypeId))
    (coherent : Coherent unit handle typeInstantiation)
    (captures : HList captured) (args : HList supplied) {results : Array RuntimeValue}
    {pre post : Memory unit} {contract : Contract (Memory unit) Failure (HList full) shape.carrier}
    (verified : Satisfies (propheticMeaning executable typeInstantiation handle full shape) contract)
    (ensures : EnsuresOf executable (closureOf handle weave.mask typeInstantiation captures).encode
      (HList.encode args) results pre post)
    (assumed : contract.assumes (weave.compose captures args) pre)
    (permitted : contract.requires (weave.compose captures args) pre) :
    ∃ result, contract.frame (weave.compose captures args) pre result post := by
  have verdict := verified _ pre assumed permitted
  rcases ok_or_undefined_of_ensuresOf_closureOf weave capturedFree suppliedFree shapeFree
      typeInstantiation coherent captures args ensures with ⟨result, ok⟩ | undefined
  · exact ⟨result, (verdict.1 result post ok).2.1⟩
  · exact absurd undefined verdict.2.2

/-- A literal closure keeps a frame where every run of its target from
typed memory, at any arguments, keeps it: the frame its body establishes,
whatever its precondition. -/
theorem FramedAt.ofBody {unit : ValidatedUnit} {executable : ExecutableUnit unit} [Skolems unit]
    {handle : FunctionHandle}
    {full captured supplied : NRow} (weave : Weave full captured supplied)
    (capturedFree : captured.lendsEncoded = true) (suppliedFree : supplied.lendsEncoded = true)
    {shape : ResultShape} (shapeFree : shape.row.lendsEncoded = true)
    (typeInstantiation : Array (TypeId × TypeId))
    (coherent : Coherent unit handle typeInstantiation) (captures : HList captured)
    {frame : HList supplied → Memory unit → Memory unit → Prop}
    (kept : ∀ (args : HList supplied) (pre : Memory unit), MemoryTyped unit pre →
      wp (propheticMeaning executable typeInstantiation handle full shape (weave.compose captures args))
        (fun _ post => frame args pre post) (fun _ => True) pre) :
    FramedAt executable supplied frame (closureOf handle weave.mask typeInstantiation captures) := by
  intro _ args results pre post typed ensures
  have holds := kept args pre typed
  rcases ok_or_undefined_of_ensuresOf_closureOf weave capturedFree suppliedFree shapeFree
      typeInstantiation coherent captures args ensures with ⟨result, ok⟩ | undefined
  · exact holds.1 result post ok
  · exact absurd undefined holds.2.2

/-- A literal closure of a verified target keeps a frame where the target's
theorem applies at every typed memory and its frame lies within it. -/
theorem FramedAt.ofVerified {unit : ValidatedUnit} {executable : ExecutableUnit unit} [Skolems unit]
    {handle : FunctionHandle}
    {full captured supplied : NRow} (weave : Weave full captured supplied)
    (capturedFree : captured.lendsEncoded = true) (suppliedFree : supplied.lendsEncoded = true)
    {shape : ResultShape} (shapeFree : shape.row.lendsEncoded = true)
    (typeInstantiation : Array (TypeId × TypeId))
    (coherent : Coherent unit handle typeInstantiation) (captures : HList captured)
    {contract : Contract (Memory unit) Failure (HList full) shape.carrier}
    (verified : Satisfies (propheticMeaning executable typeInstantiation handle full shape) contract)
    {frame : HList supplied → Memory unit → Memory unit → Prop}
    (held : ∀ (args : HList supplied) (pre : Memory unit), MemoryTyped unit pre →
      contract.assumes (weave.compose captures args) pre ∧
        contract.requires (weave.compose captures args) pre)
    (framed : ∀ (args : HList supplied) (pre post : Memory unit) (result : shape.carrier),
      contract.frame (weave.compose captures args) pre result post → frame args pre post) :
    FramedAt executable supplied frame (closureOf handle weave.mask typeInstantiation captures) := by
  intro _ args results pre post typed ensures
  obtain ⟨assumed, permitted⟩ := held args pre typed
  obtain ⟨result, frameKept⟩ := frame_of_ensuresOf_closureOf_verified weave capturedFree
    suppliedFree shapeFree typeInstantiation coherent captures args verified ensures assumed permitted
  exact framed args pre post result frameKept

end Rows

end VerexIR.Proofs

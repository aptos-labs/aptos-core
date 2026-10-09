-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Behavior
import LeanerIR.Proofs.Completeness
import LeanerIR.Semantics.LoanIndependence
import LeanerIR.Semantics.Preservation

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

namespace LeanerIR.Proofs

open LeanerIR.Validation

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

/-- A run of an invocation is one `ensures_of` states: from a start at the
initial state, on the encoded arguments, returning the encoded result. -/
theorem closureMeaning_ok_ensuresOf {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    [Skolems unit] {closure : ClosureValue}
    {σs : NRow} {shape : ResultShape} {args : HList σs} (free : σs.refFree = true)
    (shapeFree : shape.row.refFree = true) {initial final : Memory unit}
    {result : shape.carrier}
    (run : (closureMeaning executable closure σs shape args).ok initial result final) :
    ∃ values, shape.lend false result [] = some values ∧
      EnsuresOf executable closure.encode (HList.encode args) values initial final := by
  obtain ⟨start, loans, supplied, arguments, results, exit, returnedLoans, prophecyRow,
    globals_eq, admissible, lend_eq, compose_eq, step, lend_false, lend_true, -, encoded, agree⟩ := run
  obtain ⟨rfl, rfl⟩ := lendArguments_refFree free lend_eq
  cases ResultShape.lend_refFree_loans shape shapeFree lend_false
  rw [ResultShape.lend_refFree_view shape shapeFree, lend_false] at lend_true
  have sameRow : prophecyRow = results := (Option.some.inj lend_true).symm
  subst prophecyRow
  refine ⟨results, lend_false, start, exit, ⟨globals_eq, admissible⟩, ?_, encoded, agree⟩
  simp only [ClosureValue.encode, invocationSpec]
  simp only at compose_eq
  rw [compose_eq]
  exact step

/-- An abort of an invocation is one `aborts_of` states. -/
theorem closureMeaning_aborts_abortsOf {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    [Skolems unit] {closure : ClosureValue}
    {σs : NRow} {shape : ResultShape} {args : HList σs} (free : σs.refFree = true)
    {initial : Memory unit} {error : Failure}
    (aborted : (closureMeaning executable closure σs shape args).aborts initial error) :
    AbortsOf executable closure.encode (HList.encode args) initial := by
  obtain ⟨start, loans, supplied, arguments, globals_eq, admissible, lend_eq, compose_eq,
    step⟩ := aborted
  obtain ⟨rfl, rfl⟩ := lendArguments_refFree free lend_eq
  refine ⟨start, error, ⟨globals_eq, admissible⟩, ?_⟩
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
    (free : σs.refFree = true) (shapeFree : shape.row.refFree = true)
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
    rw [NRow.lend_refFree σs free false args []]
  cases loans with
  | cons _ _ =>
      unfold lendArguments at lend_eq
      rw [NRow.lend_refFree σs free false args _] at lend_eq
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
    { globals := globals start globals_eq.globals
      inert_le := Nat.le_refl _
      pending := fun _ member => by simp at member
      bounded := fun _ _ none_eq => by simp at none_eq }
  obtain ⟨loans', -, -, returned⟩ := preservation natives run (fun _ => none) start.pending.size
    targetNs declaration semantic allParameters results ns_eq declaration_eq faithful signature_eq
    (by simpa using composed) startTyped
  obtain ⟨result, lend_result⟩ :=
    decodes loans' values (returned values rfl) (EvalFunction_holeFree run)
  exact undecoded ⟨result, [], args, lend_result,
    (ResultShape.lend_refFree shape shapeFree result).trans lend_result, lent,
    argumentsResolve_refFree σs free args values _⟩

/-- The weakest precondition of such an invocation: what its runs and aborts
establish. -/
theorem wp_closureMeaning_typed {unit : ValidatedUnit} (executable : ExecutableUnit unit)
    [Skolems unit] (natives : NativesTyped executable)
    (closure : ClosureValue) {σs : NRow} {shape : ResultShape} (args : HList σs)
    (free : σs.refFree = true) (shapeFree : shape.row.refFree = true)
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

/-- Two runs from starts with the same global and Table storage holding no loan and
fresh registries, on arguments holding none: one is the other with its
loans raised. -/
theorem evalFunction_agree {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    (natives : NativesShift executable)
    {handle : FunctionHandle} {instantiation : Array (TypeId × TypeId)}
    {start start₂ final final₂ : RuntimeState} {arguments : Array RuntimeValue}
    {outcome outcome₂ : Outcome}
    (globals_eq : start₂.globals = start.globals)
    (tables_eq : start₂.tables = start.tables)
    (plain : ∀ slot ∈ start.globals.entries, Plain slot.value)
    (tables_plain : ∀ slot ∈ start.tables.contents.entries, Plain slot.value)
    (fresh : FreshStorageLoanIds start) (fresh₂ : FreshStorageLoanIds start₂)
    (arguments_plain : ∀ argument ∈ arguments, Plain argument)
    (run : EvalFunction executable handle instantiation start arguments final outcome)
    (run₂ : EvalFunction executable handle instantiation start₂ arguments final₂ outcome₂) :
    (∃ offset, outcome₂ = outcome.shift offset) ∨ ∃ offset, outcome = outcome₂.shift offset := by
  rcases Nat.le_total start.nextLoan start₂.nextLoan with le | le
  · obtain ⟨_, mirror, -⟩ :=
      runs_mirror natives run globals_eq tables_eq plain tables_plain fresh fresh₂ le arguments_plain
    exact .inl ⟨_, (Completeness.evalFunction_deterministic run₂ mirror).2⟩
  · obtain ⟨_, mirror, -⟩ := runs_mirror natives run₂ globals_eq.symm tables_eq.symm
      (by rw [globals_eq]; exact plain) (by rw [tables_eq]; exact tables_plain)
      fresh₂ fresh le arguments_plain
    exact .inr ⟨_, (Completeness.evalFunction_deterministic run mirror).2⟩

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
  rcases evalFunction_agree natives (starts₂.1.unique starts.1).1 (starts₂.1.unique starts.1).2
      starts.1.globals.plain starts.1.tables.1.plain starts.2.2.2
      starts₂.2.2.2 composed_plain ok threw with ⟨_, shifted⟩ | ⟨_, shifted⟩ <;> cases shifted

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
  rcases evalFunction_agree natives (starts₂.1.unique starts.1).1 (starts₂.1.unique starts.1).2
      starts.1.globals.plain starts.1.tables.1.plain starts.2.2.2
      starts₂.2.2.2 composed_plain ok ok₂ with ⟨offset, shifted⟩ | ⟨offset, shifted⟩
  · simp only [Outcome.shift_returned, Outcome.returned.injEq] at shifted
    rw [shifted, Array.map_shift_of_plain offset results_plain]
  · simp only [Outcome.shift_returned, Outcome.returned.injEq] at shifted
    have same := shifted.symm.trans (Array.map_shift_of_plain offset results_plain).symm
    have unshifted := congrArg (·.map (·.unshift offset)) same
    simpa [Array.map_map, Function.comp_def, RuntimeValue.unshift_shift] using unshifted

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
    exact Option.map_injective
      (Denote.ResourceType.encode_injective resource) encoded.symm
  by_cases tableNamed : ∃ namespaceId typeId,
      Denote.TableMemory.runtimeResourceOf unit namespaceId typeId = some resource
  · obtain ⟨namespaceId, typeId, named⟩ := tableNamed
    have encoded := leftEncoded.tables.1.2 namespaceId typeId key
    rw [same.2, rightEncoded.tables.1.2, named] at encoded
    exact Option.map_injective
      (Denote.ResourceType.encode_injective resource) encoded.symm
  by_cases allocation : resource = Denote.TableMemory.allocationResource
  · subst resource
    rw [leftEncoded.tables.2, rightEncoded.tables.2, same.2]
  have unnamed : Denote.Unnamed unit resource :=
    ⟨fun ns ty named => globalNamed ⟨ns, ty, named⟩,
     fun ns ty named => tableNamed ⟨ns, ty, named⟩, allocation⟩
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
  rcases Nat.le_total start.nextLoan start₂.nextLoan with le | le
  · obtain ⟨_, mirror, shifted, tablesShifted⟩ := runs_mirror natives ok
      (starts₂.1.unique starts.1).1 (starts₂.1.unique starts.1).2
      starts.1.globals.plain starts.1.tables.1.plain
      starts.2.2.2 starts₂.2.2.2 le composed_plain
    have deterministic := Completeness.evalFunction_deterministic ok₂ mirror
    have sameResults : returns.choose = results.map (·.shift (start₂.nextLoan - start.nextLoan)) := by
      simpa only [Outcome.shift_returned, Outcome.returned.injEq] using deterministic.2
    rw [← deterministic.1] at shifted tablesShifted
    have globals : (exit₂.resolveReturned returns.choose).globals =
        (exit.resolveReturned results).globals.shift (start₂.nextLoan - start.nextLoan) := by
      simp only [RuntimeState.resolveReturned, shifted, sameResults, GlobalMap.resolveReturned_shift]
    have tables : (exit₂.resolveReturned returns.choose).tables =
        (exit.resolveReturned results).tables.shift (start₂.nextLoan - start.nextLoan) := by
      simp only [RuntimeState.resolveReturned, tablesShifted, sameResults, NativeTableStorage.shift,
        GlobalMap.resolveReturned_shift]
    exact ⟨globals.trans (GlobalMap.shift_of_plain _ encoded.globals.plain),
      tables.trans (NativeTableStorage.shift_of_plain _ encoded.tables.1.plain)⟩
  · obtain ⟨_, mirror, shifted, tablesShifted⟩ := runs_mirror natives ok₂
      (starts.1.unique starts₂.1).1 (starts.1.unique starts₂.1).2
      starts₂.1.globals.plain starts₂.1.tables.1.plain
      starts₂.2.2.2 starts.2.2.2 le composed_plain
    have deterministic := Completeness.evalFunction_deterministic ok mirror
    have sameResults : results = returns.choose.map (·.shift (start.nextLoan - start₂.nextLoan)) := by
      simpa only [Outcome.shift_returned, Outcome.returned.injEq] using deterministic.2
    rw [← deterministic.1] at shifted tablesShifted
    have globals : (exit.resolveReturned results).globals =
        (exit₂.resolveReturned returns.choose).globals.shift (start.nextLoan - start₂.nextLoan) := by
      simp only [RuntimeState.resolveReturned, shifted, sameResults, GlobalMap.resolveReturned_shift]
    have tables : (exit.resolveReturned results).tables =
        (exit₂.resolveReturned returns.choose).tables.shift (start.nextLoan - start₂.nextLoan) := by
      simp only [RuntimeState.resolveReturned, tablesShifted, sameResults, NativeTableStorage.shift,
        GlobalMap.resolveReturned_shift]
    exact ⟨(globals.trans (GlobalMap.shift_of_plain _ encoded₂.globals.plain)).symm,
      (tables.trans (NativeTableStorage.shift_of_plain _ encoded₂.tables.1.plain)).symm⟩

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
      exact NTy.encode_plain τ result

/-- An invocation of a typed closure a proof cannot see, on typed arguments
from typed global memory, has the weakest precondition its behavioral
predicates state: a run returns results `ensures_of` names, `aborts_of`
does not hold, and the results are `result_of`; an abort is one
`aborts_of` names. -/
theorem wp_closureMeaning_unseen {unit : ValidatedUnit} (executable : ExecutableUnit unit)
    [Skolems unit] (natives : NativesTyped executable)
    (shifts : NativesShift executable) (closure : ClosureValue) {σs : NRow} {shape : ResultShape}
    (args : HList σs) (free : σs.refFree = true) (shapeFree : shape.row.refFree = true)
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
      (ResultShape.lend_plain shape shapeFree lent))
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
  | .unit | .tuple _ | .struct .. | .enum .. | .vector _ | .ref _ | .param _
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
  | .enum .., _, scalar, _ | .vector _, _, scalar, _ | .ref _, _, scalar, _
  | .param _, _, scalar, _ | .function .., _, scalar, _ => by simp [NTy.scalarType?] at scalar

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
  | .enum .., _, scalar, _, _, _ | .vector _, _, scalar, _, _, _ | .ref _, _, scalar, _, _, _
  | .param _, _, scalar, _, _, _ | .function .., _, scalar, _, _, _ => by
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
          have free : τ.refFree = true := by
            cases τ <;> simp_all [NTy.scalarType?, NTy.refFree]
          refine ⟨decoded, ?_⟩
          simp only [ResultShape.lend, NTy.lend_refFree τ free false decoded [], encoded]

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
/-- A row of scalar types holds no reference. -/
theorem Denote.NRow.refFree_of_scalarTypes : (row : NRow) → {types : List SemTy} →
    row.scalarTypes? = some types → row.refFree = true
  | .nil, _, _ => rfl
  | .cons τ rest, _, scalar => by
      simp only [NRow.scalarTypes?, Option.bind_eq_bind, Option.bind_eq_some_iff] at scalar
      obtain ⟨type, head, types, tail, -⟩ := scalar
      simp only [NRow.refFree, NRow.refFree_of_scalarTypes rest tail, Bool.and_true]
      cases τ <;> simp_all [NTy.scalarType?, NTy.refFree]

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
      exact ⟨(), rfl⟩
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
              refine ⟨Skolems.ofRuntime τ decoded, ?_⟩
              simp only [ResultShape.lend, NTy.lend_refFree τ τFree false _ []]
              rw [← Skolems.encode_toRuntime, Skolems.toRuntime_ofRuntime]
              exact congrArg (fun raw => some #[raw]) encoded

@[lir_denote_norm] theorem Denote.resultCodec_none_encode (result : ResultShape.none.carrier) :
    (resultCodec .none).encode result = #[] := rfl

@[lir_denote_norm] theorem Denote.resultCodec_one_encode (τ : NTy) (result : τ.carrier) :
    (resultCodec (.one τ)).encode result = #[τ.encode result] := rfl

/-- A reference-free result lends as its encoding. -/
theorem Denote.ResultShape.lend_encode : (shape : ResultShape) → shape.row.refFree = true →
    (result : shape.carrier) → shape.lend false result [] = some ((resultCodec shape).encode result)
  | .none, _, _ => rfl
  | .one τ, free, result => by
      simp only [NRow.refFree, Bool.and_true] at free
      simp only [ResultShape.lend, NTy.lend_refFree τ free false result [], resultCodec_one_encode]

@[lir_denote_norm] theorem Denote.ResultShape.row_none : ResultShape.none.row = .nil := rfl
@[lir_denote_norm] theorem Denote.ResultShape.row_one (τ : NTy) :
    (ResultShape.one τ).row = .cons τ .nil := rfl

/-- `wp_named` from typed global memory: the call's run also rules out an
abort of the function value naming the target, and is the result
`result_of` reads of it. -/
theorem wp_named_typed {unit : ValidatedUnit} (executable : ExecutableUnit unit) [Skolems unit]
    (shifts : NativesShift executable)
    {handle : FunctionHandle} {typeInstantiation : Array (TypeId × TypeId)} {full : NRow}
    {shape : ResultShape} (fullFree : full.refFree = true) (shapeFree : shape.row.refFree = true)
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
      have resultsPlain := ResultShape.lend_plain shape shapeFree
        (ResultShape.lend_encode shape shapeFree result)
      exact named.1 result final execution ensuresOf
        (not_abortsOf_of_ok shifts callablePlain argumentsPlain starts ok)
        (resultOf_eq_of_ok shifts callablePlain argumentsPlain starts ok encoded agree
          resultsPlain)
        (stateOf_eq_of_ensuresOf shifts callablePlain argumentsPlain ensuresOf),
    fun error execution => named.2.1 error execution (abortsOf_of_abort fullFree execution),
    named.2.2⟩

/-- The rule at an invocation of a function value of rows of scalars and type
parameters, typed at them: `wp_closureMeaning_unseen` with the typing and
decoding its typing states, stated at the function value's encoding and its
results' as a contract spells them. -/
theorem wp_closureMeaning_unseen_at {unit : ValidatedUnit} (executable : ExecutableUnit unit)
    [Skolems unit] (natives : NativesTyped executable)
    (shifts : NativesShift executable) (closure : ClosureValue) {σs : NRow} {shape : ResultShape}
    (args : HList σs) (invocable : σs.invocable = true) (shapeInvocable : shape.row.invocable = true)
    (free : σs.refFree = true) (shapeFree : shape.row.refFree = true)
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
    (free : σs.refFree = true) (shapeFree : shape.row.refFree = true)
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
      agree (ResultShape.lend_plain shape shapeFree lent)
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
    (resultsFree : results.refFree = true)
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
references. -/
def FramedAt {unit : ValidatedUnit} (executable : ExecutableUnit unit) [Skolems unit]
    (parameters : NRow)
    (frame : HList parameters → Memory unit → Memory unit → Prop) (closure : ClosureValue) : Prop :=
  parameters.refFree = true →
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

theorem FramedAt.of_refFree_false {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    [Skolems unit] {parameters : NRow}
    {frame : HList parameters → Memory unit → Memory unit → Prop} {closure : ClosureValue}
    (references : parameters.refFree = false) : FramedAt executable parameters frame closure :=
  fun free => by simp [references] at free

/-- A run of a closure whose target, weave, and captures a proof sees, at
reference-free arguments and results: a run of the target's prophetic
meaning, or, where its results do not decode, an undefined outcome of it. -/
theorem ok_or_undefined_of_ensuresOf_closureOf {unit : ValidatedUnit}
    {executable : ExecutableUnit unit} [Skolems unit] {handle : FunctionHandle}
    {full captured supplied : NRow} (weave : Weave full captured supplied)
    (capturedFree : captured.refFree = true) (suppliedFree : supplied.refFree = true)
    {shape : ResultShape} (shapeFree : shape.row.refFree = true)
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
  by_cases decodes : ∃ result, shape.lend false result [] = some results
  · obtain ⟨result, resultLent⟩ := decodes
    refine .inl ⟨result, coherent, start, [], _, results, exit, [], results, globals, admissible,
      lent, runs, resultLent, (ResultShape.lend_refFree shape shapeFree result).trans resultLent,
      ?_, encoded, agree⟩
    exact (weave.argumentsResolve_compose capturedFree captures args [] _ _).mpr
      (argumentsResolve_refFree supplied suppliedFree args _ _)
  · refine .inr ⟨coherent, start, [], _, results, exit, globals, admissible, lent, runs, ?_⟩
    rintro ⟨result, returnedLoans, -, resultLent, -, -, -⟩
    obtain rfl := ResultShape.lend_refFree_loans shape shapeFree resultLent
    exact decodes ⟨result, resultLent⟩

/-- `ensures_of` of a closure whose target, weave, and captures a proof sees,
at reference-free arguments and results of any shape, is a run of the
target's prophetic meaning. -/
theorem ensuresOf_closureOf_shape {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    [Skolems unit] {handle : FunctionHandle}
    {full captured supplied : NRow} (weave : Weave full captured supplied)
    (capturedFree : captured.refFree = true) (suppliedFree : supplied.refFree = true)
    {shape : ResultShape} (shapeFree : shape.row.refFree = true)
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
  refine ⟨coherent, start, [], _, _, exit, [], _, globals, admissible,
    lendArguments_weave weave capturedFree suppliedFree captures args, runs, lent,
    (ResultShape.lend_refFree shape shapeFree result).trans lent, ?_, encoded, agree⟩
  exact (weave.argumentsResolve_compose capturedFree captures args [] _ _).mpr
    (argumentsResolve_refFree supplied suppliedFree args _ _)

/-- `ensures_of` of a closure whose target is verified, at results of any
shape: where what the target's theorem assumes and its precondition hold,
what its contract ensures of the run, its frame, and that no condition
under which it must abort held. -/
theorem ensuresOf_closureOf_verified_shape {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    [Skolems unit] {handle : FunctionHandle}
    {full captured supplied : NRow} (weave : Weave full captured supplied)
    (capturedFree : captured.refFree = true) (suppliedFree : supplied.refFree = true)
    {shape : ResultShape} (shapeFree : shape.row.refFree = true)
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
    (capturedFree : captured.refFree = true) (suppliedFlat : supplied.lentFlat = true)
    {shape : ResultShape} (shapeFree : shape.row.refFree = true)
    (typeInstantiation : Array (TypeId × TypeId))
    (coherent : Coherent unit handle typeInstantiation)
    (captures : HList captured) (args : HList supplied) (result : shape.carrier)
    {pre post : Memory unit}
    (ensures : EnsuresOfMut executable (closureOf handle weave.mask typeInstantiation captures).encode
      supplied.mutable (HList.entries args) ((resultCodec shape).encode result) (HList.finals args)
      pre post) :
    (propheticMeaning executable typeInstantiation handle full shape
      (weave.compose captures args)).ok pre result post := by
  obtain ⟨start, loans, lent, exit, globals, admissible, lends, runs, resolves, encoded, agree⟩ :=
    ensures
  obtain ⟨lentArguments, invocation⟩ :=
    invocationSpec_closureOf_lent weave capturedFree suppliedFlat typeInstantiation captures args
      (executable := executable) (handle := handle) lends
  rw [invocation] at runs
  have resultLent := ResultShape.lend_encode shape shapeFree result
  refine ⟨coherent, start, loans, _, _, exit, [], _, globals, admissible, lentArguments, runs,
    resultLent, (ResultShape.lend_refFree shape shapeFree result).trans resultLent, ?_, encoded,
    agree⟩
  exact (weave.argumentsResolve_compose capturedFree captures args loans _ _).mpr
    ((argumentsResolve_finals supplied args loans _ _).mpr resolves)

/-- `aborts_of` of a closure whose target, weave, and captures a proof sees,
at arguments lending mutable references, is an abort of the target's
prophetic meaning at the references' entry values, whatever their final
values. -/
theorem abortsOfMut_closureOf {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    [Skolems unit] {handle : FunctionHandle}
    {full captured supplied : NRow} (weave : Weave full captured supplied)
    (capturedFree : captured.refFree = true) (suppliedFlat : supplied.lentFlat = true)
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
  exact ⟨failure, coherent, start, loans, _, globals, admissible, lentArguments, runs⟩

/-- `ensures_of` of a closure taking mutable references whose target is
verified: where what the target's theorem assumes and its precondition
hold, what its contract ensures of the run, its frame, and that no
condition under which it must abort held. -/
theorem ensuresOfMut_closureOf_verified {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    [Skolems unit] {handle : FunctionHandle}
    {full captured supplied : NRow} (weave : Weave full captured supplied)
    (capturedFree : captured.refFree = true) (suppliedFlat : supplied.lentFlat = true)
    {shape : ResultShape} (shapeFree : shape.row.refFree = true)
    (typeInstantiation : Array (TypeId × TypeId))
    (coherent : Coherent unit handle typeInstantiation)
    (captures : HList captured) (args : HList supplied) (result : shape.carrier)
    {pre post : Memory unit} {contract : Contract (Memory unit) Failure (HList full) shape.carrier}
    (verified : Satisfies (propheticMeaning executable typeInstantiation handle full shape) contract)
    (ensures : EnsuresOfMut executable (closureOf handle weave.mask typeInstantiation captures).encode
      supplied.mutable (HList.entries args) ((resultCodec shape).encode result) (HList.finals args)
      pre post)
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
    (capturedFree : captured.refFree = true) (suppliedFlat : supplied.lentFlat = true)
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
    (capturedFree : captured.refFree = true) (suppliedFree : supplied.refFree = true)
    {shape : ResultShape} (shapeFree : shape.row.refFree = true)
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
    (capturedFree : captured.refFree = true) (suppliedFree : supplied.refFree = true)
    {shape : ResultShape} (shapeFree : shape.row.refFree = true)
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
    (capturedFree : captured.refFree = true) (suppliedFree : supplied.refFree = true)
    {shape : ResultShape} (shapeFree : shape.row.refFree = true)
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

end LeanerIR.Proofs

-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Denote.Term

/-!
# Closures of known targets

An invocation denotes its closure's meaning (`closureMeaning`). For a closure
whose target, weave, and captures a proof sees, that meaning is the target's
prophetic meaning on the captures and the supplied arguments woven into its
parameter row (`closureMeaning_closureOf`), so a proof invokes a known
closure through its target's theorem, as it calls the target directly.
Captures hold no reference: lending the woven row lends the supplied
arguments alone.
-/

namespace LeanerIR.Proofs.Denote

section Frames
variable {unit : Validation.ValidatedUnit} [Θ : Skolems unit]

/-- The target's native argument row: the captures and the supplied
arguments, woven. -/
def Weave.compose : {full captured supplied : NRow} → Weave full captured supplied →
    HList captured → HList supplied → HList full
  | _, _, _, .nil, _, _ => ()
  | _, _, _, .captured rest, captures, args => (captures.1, rest.compose captures.2 args)
  | _, _, _, .supplied rest, captures, args => (args.1, rest.compose captures args.2)

/-- The weave supplying every parameter: a function value that captures
nothing, as naming a function makes one. -/
def Weave.supplying : (row : NRow) → Weave row .nil row
  | .nil => .nil
  | .cons _ rest => .supplied (Weave.supplying rest)

@[simp] theorem Weave.supplying_nil : Weave.supplying .nil = .nil := rfl
@[simp] theorem Weave.supplying_cons (τ : NTy) (rest : NRow) :
    Weave.supplying (.cons τ rest) = .supplied (Weave.supplying rest) := rfl

/-- The captures and the supplied arguments of a target's native argument row. -/
def Weave.split : {full captured supplied : NRow} → Weave full captured supplied →
    HList full → HList captured × HList supplied
  | _, _, _, .nil, _ => ((), ())
  | _, _, _, .captured rest, values =>
      let (captures, args) := rest.split values.2
      ((values.1, captures), args)
  | _, _, _, .supplied rest, values =>
      let (captures, args) := rest.split values.2
      (captures, (values.1, args))

omit [Skolems unit] in
/-- The target's runtime argument row, woven from runtime captures and
arguments. -/
def Weave.composeList : {full captured supplied : NRow} → Weave full captured supplied →
    List RuntimeValue → List RuntimeValue → List RuntimeValue
  | _, _, _, .nil, _, args => args
  | _, _, _, .captured rest, capture :: captures, args =>
      capture :: rest.composeList captures args
  | _, _, _, .captured rest, [], args => rest.composeList [] args
  | _, _, _, .supplied rest, captures, arg :: args => arg :: rest.composeList captures args
  | _, _, _, .supplied rest, captures, [] => rest.composeList captures []

omit Θ in
/-- Supplying every parameter composes the arguments alone. -/
theorem Weave.composeList_supplying : (row : NRow) → (arguments : List RuntimeValue) →
    (Weave.supplying row).composeList [] arguments = arguments
  | .nil, _ => rfl
  | .cons _ rest, [] => by
      simp only [Weave.supplying, Weave.composeList, Weave.composeList_supplying rest []]
  | .cons _ rest, _ :: arguments => by
      simp only [Weave.supplying, Weave.composeList, Weave.composeList_supplying rest arguments]

theorem Weave.compose_split : {full captured supplied : NRow} →
    (weave : Weave full captured supplied) → (values : HList full) →
    weave.compose (weave.split values).1 (weave.split values).2 = values
  | _, _, _, .nil, () => rfl
  | _, _, _, .captured rest, (value, values) => by
      simp only [Weave.split, Weave.compose, rest.compose_split values]
  | _, _, _, .supplied rest, (value, values) => by
      simp only [Weave.split, Weave.compose, rest.compose_split values]

/-! ## Lending -/

mutual
/-- A value without references lends as its encoding, taking no loan. -/
theorem NTy.lend_refFree : (τ : NTy) → τ.refFree = true → (prophecies : Bool) →
    (value : τ.carrier) → (loans : List Nat) → τ.lend prophecies value loans = some (τ.encode value, loans)
  | .ref _, free, _, _, _ => by simp [NTy.refFree] at free
  | .tuple elements, free, prophecies, values, loans => by
      simp only [NTy.refFree] at free
      simp only [NTy.lend, NRow.lend_refFree elements free prophecies values loans, Option.map_some,
        NTy.encode_tuple, HList.encode]
  | .unit, _, _, _, _ | .bool, _, _, _, _ | .int _ _, _, _, _, _ | .address, _, _, _, _
  | .signer, _, _, _, _ | .string, _, _, _, _ | .bytes, _, _, _, _ | .struct _ _ _, _, _, _, _
  | .enum _ _ _ _ _, _, _, _, _ | .vector _, _, _, _, _ | .param _, _, _, _, _
  | .function _ _ _, _, _, _, _ => rfl

/-- A row without references lends as its encoding, taking no loan. -/
theorem NRow.lend_refFree : (row : NRow) → row.refFree = true → (prophecies : Bool) →
    (values : HList row) → (loans : List Nat) →
    NRow.lend row prophecies values loans = some (HList.encode values, loans)
  | .nil, _, _, _, _ => rfl
  | .cons τ rest, free, prophecies, values, loans => by
      simp only [NRow.refFree, Bool.and_eq_true] at free
      simp only [NRow.lend, NTy.lend_refFree τ free.1 prophecies values.1 loans, Option.bind_some,
        NRow.lend_refFree rest free.2 prophecies values.2 loans, Option.map_some,
        HList.encode_cons]
end

/-- Lending a woven row lends the supplied arguments and places the captures'
encodings among them. -/
theorem Weave.lend_compose : {full captured supplied : NRow} →
    (weave : Weave full captured supplied) → captured.refFree = true →
    (captures : HList captured) → (args : HList supplied) → (loans : List Nat) →
    NRow.lend full false (weave.compose captures args) loans =
      (NRow.lend supplied false args loans).map fun lent =>
        (weave.composeList (HList.encode captures) lent.1, lent.2)
  | _, _, _, .nil, _, _, _, _ => rfl
  | _, _, _, .captured rest, free, captures, args, loans => by
      simp only [NRow.refFree, Bool.and_eq_true] at free
      simp only [Weave.compose, NRow.lend, NTy.lend_refFree _ free.1 false captures.1 loans,
        Option.bind_some, rest.lend_compose free.2 captures.2 args loans, Option.map_map,
        HList.encode_cons, Weave.composeList]
      rfl
  | _, _, _, .supplied rest, free, captures, args, loans => by
      simp only [Weave.compose, NRow.lend]
      cases (NTy.lend _ false args.1 loans) with
      | none => rfl
      | some head =>
          simp only [Option.bind_some, rest.lend_compose free captures args.2 head.2,
            Option.map_map]
          rfl

theorem NRow.lend_length : (row : NRow) → (prophecies : Bool) → (values : HList row) →
    (loans : List Nat) → (lent : List RuntimeValue × List Nat) →
    NRow.lend row prophecies values loans = some lent → lent.1.length = row.length
  | .nil, _, _, _, lent, h => by
      simp only [NRow.lend, Option.some.injEq] at h
      subst h; rfl
  | .cons τ rest, prophecies, values, loans, lent, h => by
      simp only [NRow.lend] at h
      cases head : τ.lend prophecies values.1 loans with
      | none => simp [head] at h
      | some head' =>
          simp only [head, Option.bind_some] at h
          cases tail : NRow.lend rest prophecies values.2 head'.2 with
          | none => simp [tail] at h
          | some tail' =>
              simp only [tail, Option.map_some, Option.some.injEq] at h
              subst h
              simp [NRow.length, NRow.lend_length rest prophecies values.2 head'.2 tail' tail]

theorem HList.encode_length : (row : NRow) → (values : HList row) →
    (HList.encode values).length = row.length
  | .nil, _ => rfl
  | .cons _ rest, values => by
      simp [HList.encode_cons, NRow.length, HList.encode_length rest values.2]

/-! ## The mask -/

omit [Skolems unit] in
/-- A mask's captured positions hold captures: a weave without a captured
position has the mask zero. -/
theorem Weave.mask_eq_zero : {full captured supplied : NRow} →
    (weave : Weave full captured supplied) → weave.mask = 0 → captured = .nil
  | _, _, _, .nil, _ => rfl
  | _, _, _, .captured _, h => by simp [Weave.mask] at h
  | _, _, _, .supplied rest, h => by
      simp only [Weave.mask] at h
      exact rest.mask_eq_zero (by omega)

omit [Skolems unit] in
theorem Weave.composeList_of_mask_zero : {full captured supplied : NRow} →
    (weave : Weave full captured supplied) → weave.mask = 0 → (args : List RuntimeValue) →
    args.length = supplied.length → weave.composeList [] args = args
  | _, _, _, .nil, _, _, _ => rfl
  | _, _, _, .captured _, h, _, _ => by simp [Weave.mask] at h
  | _, _, _, .supplied rest, h, arg :: args, lengths => by
      simp only [Weave.mask] at h
      simp only [NRow.length, List.length_cons, Nat.add_right_cancel_iff] at lengths
      simp only [Weave.composeList, rest.composeList_of_mask_zero (by omega) args lengths]
  | _, _, _, .supplied _, _, [], lengths => by simp [NRow.length] at lengths

omit [Skolems unit] in
/-- The runtime's composition by the weave's mask is the weave's. -/
theorem Weave.compose_go : {full captured supplied : NRow} →
    (weave : Weave full captured supplied) → (captures args : List RuntimeValue) →
    captures.length = captured.length → args.length = supplied.length → (fuel : Nat) →
    captures.length + args.length ≤ fuel →
    ClosureMask.compose.go fuel weave.mask captures args =
      some (weave.composeList captures args)
  | _, _, _, .nil, [], args, _, _, fuel, _ => by
      simp [Weave.mask, ClosureMask.compose.go, Weave.composeList]
  | _, _, _, .nil, _ :: _, _, lengths, _, _, _ => by simp [NRow.length] at lengths
  | _, _, _, .captured rest, capture :: captures, args, lengths, argLengths, fuel + 1, bound => by
      simp only [NRow.length, List.length_cons, Nat.add_right_cancel_iff] at lengths
      simp only [List.length_cons] at bound
      have step : 2 * rest.mask + 1 = (2 * rest.mask) + 1 := rfl
      rw [Weave.mask]
      simp only [ClosureMask.compose.go, show (2 * rest.mask + 1) % 2 = 1 by omega,
        show (2 * rest.mask + 1) / 2 = rest.mask by omega, beq_self_eq_true, ite_true,
        rest.compose_go captures args lengths argLengths fuel (by omega), Option.map_eq_map,
        Option.map_some, Weave.composeList]
  | _, _, _, .captured _, [], _, lengths, _, _, _ => by simp [NRow.length] at lengths
  | _, _, _, .captured _, _ :: _, _, _, _, 0, bound => by simp at bound
  | _, _, _, .supplied rest, captures, arg :: args, lengths, argLengths, fuel + 1, bound => by
      simp only [NRow.length, List.length_cons, Nat.add_right_cancel_iff] at argLengths
      simp only [List.length_cons] at bound
      by_cases zero : rest.mask = 0
      · have none := rest.mask_eq_zero zero
        subst none
        cases captures with
        | cons _ _ => simp [NRow.length] at lengths
        | nil =>
            rw [Weave.mask, zero]
            simp only [ClosureMask.compose.go, Weave.composeList,
              rest.composeList_of_mask_zero zero args argLengths]
      · rw [Weave.mask, show 2 * rest.mask = (2 * rest.mask - 1) + 1 by omega]
        simp only [ClosureMask.compose.go, show (2 * rest.mask - 1 + 1) % 2 = 0 by omega,
          show (2 * rest.mask - 1 + 1) / 2 = rest.mask by omega, Nat.zero_ne_one, beq_iff_eq,
          ite_false, rest.compose_go captures args lengths argLengths fuel (by omega),
          Option.map_eq_map, Option.map_some, Weave.composeList]
  | _, _, _, .supplied _, _, [], _, argLengths, _, _ => by simp [NRow.length] at argLengths
  | _, _, _, .supplied _, _, _ :: _, _, _, 0, bound => by simp at bound

omit [Skolems unit] in
theorem Weave.compose_mask {full captured supplied : NRow} (weave : Weave full captured supplied)
    (captures args : List RuntimeValue) (lengths : captures.length = captured.length)
    (argLengths : args.length = supplied.length) :
    ClosureMask.compose weave.mask captures args = some (weave.composeList captures args) :=
  weave.compose_go captures args lengths argLengths _ (Nat.le_refl _)

omit [Skolems unit] in
/-- Weaving is injective in the captures and the supplied arguments of each
length. -/
theorem Weave.composeList_injective : {full captured supplied : NRow} →
    (weave : Weave full captured supplied) → (captures captures' args args' : List RuntimeValue) →
    captures.length = captured.length → captures'.length = captured.length →
    args.length = supplied.length → args'.length = supplied.length →
    weave.composeList captures args = weave.composeList captures' args' →
    captures = captures' ∧ args = args'
  | _, _, _, .nil, [], [], args, args', _, _, _, _, h => ⟨rfl, h⟩
  | _, _, _, .nil, _ :: _, _, _, _, lengths, _, _, _, _ => by simp [NRow.length] at lengths
  | _, _, _, .nil, [], _ :: _, _, _, _, lengths, _, _, _ => by simp [NRow.length] at lengths
  | _, _, _, .captured rest, c :: cs, c' :: cs', args, args', lengths, lengths', argLengths,
      argLengths', h => by
      simp only [NRow.length, List.length_cons, Nat.add_right_cancel_iff] at lengths lengths'
      simp only [Weave.composeList, List.cons.injEq] at h
      obtain ⟨rfl, rest'⟩ := h
      obtain ⟨rfl, rfl⟩ := rest.composeList_injective cs cs' args args' lengths lengths' argLengths
        argLengths' rest'
      exact ⟨rfl, rfl⟩
  | _, _, _, .captured _, [], _, _, _, lengths, _, _, _, _ => by simp [NRow.length] at lengths
  | _, _, _, .captured _, _ :: _, [], _, _, _, lengths', _, _, _ => by
      simp [NRow.length] at lengths'
  | _, _, _, .supplied rest, captures, captures', a :: as, a' :: as', lengths, lengths',
      argLengths, argLengths', h => by
      simp only [NRow.length, List.length_cons, Nat.add_right_cancel_iff] at argLengths argLengths'
      simp only [Weave.composeList, List.cons.injEq] at h
      obtain ⟨rfl, rest'⟩ := h
      obtain ⟨rfl, rfl⟩ := rest.composeList_injective captures captures' as as' lengths lengths'
        argLengths argLengths' rest'
      exact ⟨rfl, rfl⟩
  | _, _, _, .supplied _, _, _, [], _, _, _, argLengths, _, _ => by
      simp [NRow.length] at argLengths
  | _, _, _, .supplied _, _, _, _ :: _, [], _, _, _, argLengths', _ => by
      simp [NRow.length] at argLengths'

/-! ## Resolution -/

/-- A reference-free argument resolves nothing. -/
theorem argumentsResolve_cons_refFree {τ : NTy} {rest : NRow} (free : τ.refFree = true)
    (values : HList (.cons τ rest)) (loans : List Nat) (returned : Array RuntimeValue)
    (exports : List (Nat × RuntimeValue)) :
    argumentsResolve (.cons τ rest) values loans returned exports ↔
      argumentsResolve rest values.2 loans returned exports := by
  cases τ <;> simp_all [NTy.refFree, argumentsResolve]

/-- The woven arguments resolve as the supplied ones. -/
theorem Weave.argumentsResolve_compose : {full captured supplied : NRow} →
    (weave : Weave full captured supplied) → captured.refFree = true →
    (captures : HList captured) → (args : HList supplied) → (loans : List Nat) →
    (returned : Array RuntimeValue) → (exports : List (Nat × RuntimeValue)) →
    (argumentsResolve full (weave.compose captures args) loans returned exports ↔
      argumentsResolve supplied args loans returned exports)
  | _, _, _, .nil, _, _, _, _, _, _ => Iff.rfl
  | _, _, _, .captured rest, free, captures, args, loans, returned, exports => by
      simp only [NRow.refFree, Bool.and_eq_true] at free
      rw [Weave.compose, argumentsResolve_cons_refFree free.1]
      exact rest.argumentsResolve_compose free.2 captures.2 args loans returned exports
  | _, _, _, @Weave.supplied τ _ _ _ rest, free, captures, args, loans, returned, exports => by
      rw [Weave.compose]
      cases τ <;> cases loans <;>
        simp only [argumentsResolve,
          rest.argumentsResolve_compose free captures args.2 _ returned exports]

/-! ## The meaning of a known closure -/

/-- A closure's captures, as the invocation composes them, are the encoded
native captures. -/
theorem closureOf_captures (handle : FunctionHandle) (mask : Nat)
    (typeInstantiation : Array (TypeId × TypeId)) {σs : NRow} (captures : HList σs) :
    (closureOf handle mask typeInstantiation captures).captures.toList = HList.encode captures := by
  simp [closureOf]

/-- Lending a woven row is lending the supplied arguments and composing the
captures' encodings among them by the weave's mask. -/
theorem Weave.lendArguments_compose {full captured supplied : NRow}
    (weave : Weave full captured supplied) (free : captured.refFree = true)
    (captures : HList captured) (args : HList supplied) (loans : List Nat)
    (arguments : Array RuntimeValue) :
    lendArguments full (weave.compose captures args) loans = some arguments ↔
      ∃ lent, lendArguments supplied args loans = some lent ∧
        ClosureMask.compose weave.mask (HList.encode captures) lent.toList =
          some arguments.toList := by
  unfold lendArguments
  rw [weave.lend_compose free captures args loans]
  constructor
  · intro h
    cases lent : NRow.lend supplied false args loans with
    | none => simp [lent] at h
    | some pair =>
        obtain ⟨row, rest⟩ := pair
        simp only [lent, Option.map_some] at h
        cases rest with
        | cons _ _ => simp at h
        | nil =>
            simp only [Option.some.injEq] at h
            refine ⟨row.toArray, rfl, ?_⟩
            rw [← h]
            exact weave.compose_mask _ _ (HList.encode_length captured captures)
              (NRow.lend_length supplied false args loans _ lent)
  · rintro ⟨lent, hl, hc⟩
    cases pair : NRow.lend supplied false args loans with
    | none => simp [pair] at hl
    | some pair' =>
        obtain ⟨row, rest⟩ := pair'
        cases rest with
        | cons _ _ => simp [pair] at hl
        | nil =>
            simp only [pair, Option.some.injEq] at hl
            subst hl
            change ClosureMask.compose weave.mask (HList.encode captures) row = _ at hc
            rw [weave.compose_mask _ _ (HList.encode_length captured captures)
                (NRow.lend_length supplied false args loans _ pair),
              Option.some.injEq] at hc
            simp only [Option.map_some, hc, Array.toArray_toList]

theorem HList.encode_injective {row : NRow} {left right : HList row}
    (same : HList.encode left = HList.encode right) : left = right :=
  (rowCodec row).encode_injective same

/-- The lent supplied arguments of a composition decide its captures and
supplied arguments. -/
theorem Weave.lendArguments_compose_injective {full captured supplied : NRow}
    (weave : Weave full captured supplied)
    {captures captures' : HList captured} {args args' : HList supplied} {loans : List Nat}
    {lent lent' : Array RuntimeValue} {arguments : Array RuntimeValue}
    (lends : lendArguments supplied args loans = some lent)
    (lends' : lendArguments supplied args' loans = some lent')
    (composes : ClosureMask.compose weave.mask (HList.encode captures) lent.toList =
      some arguments.toList)
    (composes' : ClosureMask.compose weave.mask (HList.encode captures') lent'.toList =
      some arguments.toList) :
    captures = captures' ∧ lent = lent' := by
  have length (values : HList supplied) (row : Array RuntimeValue)
      (lends : lendArguments supplied values loans = some row) :
      row.toList.length = supplied.length := by
    unfold lendArguments at lends
    cases pair : NRow.lend supplied false values loans with
    | none => simp [pair] at lends
    | some pair' =>
        obtain ⟨list, rest⟩ := pair'
        cases rest with
        | cons _ _ => simp [pair] at lends
        | nil =>
            simp only [pair, Option.some.injEq] at lends
            subst lends
            exact NRow.lend_length supplied false values loans _ pair
  rw [weave.compose_mask _ _ (HList.encode_length captured captures) (length args lent lends)]
    at composes
  rw [weave.compose_mask _ _ (HList.encode_length captured captures') (length args' lent' lends')]
    at composes'
  obtain ⟨sameCaptures, sameLent⟩ := weave.composeList_injective _ _ _ _
    (HList.encode_length captured captures) (HList.encode_length captured captures')
    (length args lent lends) (length args' lent' lends')
    (Option.some.inj (composes.trans composes'.symm))
  exact ⟨HList.encode_injective sameCaptures, Array.toList_inj.mp sameLent⟩

/-- Invoking a closure whose target, weave, and captures are known is running
the target on the captures and the supplied arguments woven into its
parameter row. -/
theorem closureMeaning_closureOf {unit : LeanerIR.Validation.ValidatedUnit}
    (executable : LeanerIR.Validation.ExecutableUnit unit) [Θ : Skolems unit]
    (handle : FunctionHandle) {full captured supplied : NRow}
    (weave : Weave full captured supplied) (free : captured.refFree = true)
    (typeInstantiation : Array (TypeId × TypeId)) (captures : HList captured)
    (shape : ResultShape) (args : HList supplied) :
    closureMeaning executable (closureOf handle weave.mask typeInstantiation captures) supplied shape args =
      propheticRun executable typeInstantiation handle full shape (weave.compose captures args) := by
  have lend := weave.lendArguments_compose free
  have resolve := weave.argumentsResolve_compose free
  simp only [closureMeaning, propheticRun, closureOf_captures]
  have mask : (closureOf handle weave.mask typeInstantiation captures).mask = weave.mask := rfl
  have function : (closureOf handle weave.mask typeInstantiation captures).function = handle := rfl
  have instantiation :
      (closureOf handle weave.mask typeInstantiation captures).typeInstantiation =
        typeInstantiation := rfl
  simp only [mask, function, instantiation]
  congr 1
  · funext initial result final
    apply propext
    constructor
    · rintro ⟨start, loans, lent, arguments, results, exit, returnedLoans, prophecyRow,
        globals, admissible, lends, composes, runs, lendsResult, lendsProphecy, resolves, final,
        agree⟩
      exact ⟨start, loans, arguments.toArray, results, exit, returnedLoans, prophecyRow,
        globals, admissible, (lend captures args loans _).mpr ⟨lent, lends, composes⟩, runs,
        lendsResult, lendsProphecy, (resolve captures args loans _ _).mpr resolves, final, agree⟩
    · rintro ⟨start, loans, arguments, results, exit, returnedLoans, prophecyRow, globals,
        admissible, lends, runs, lendsResult, lendsProphecy, resolves, final, agree⟩
      obtain ⟨lent, lends', composes⟩ := (lend captures args loans arguments).mp lends
      exact ⟨start, loans, lent, arguments.toList, results, exit, returnedLoans,
        prophecyRow, globals, admissible, lends', composes, by simpa using runs, lendsResult,
        lendsProphecy, (resolve captures args loans _ _).mp resolves, final, agree⟩
  · funext initial error
    apply propext
    constructor
    · rintro ⟨start, loans, lent, arguments, globals, admissible, lends, composes, aborts⟩
      exact ⟨start, loans, arguments.toArray, globals, admissible,
        (lend captures args loans _).mpr ⟨lent, lends, composes⟩, aborts⟩
    · rintro ⟨start, loans, arguments, globals, admissible, lends, aborts⟩
      obtain ⟨lent, lends', composes⟩ := (lend captures args loans arguments).mp lends
      exact ⟨start, loans, lent, arguments.toList, globals, admissible, lends', composes,
        by simpa using aborts⟩
  · funext initial
    apply propext
    constructor
    · rintro ⟨start, loans, lent, arguments, results, exit, globals, admissible, lends,
        composes, runs, unresolved⟩
      refine ⟨start, loans, arguments.toArray, results, exit, globals, admissible,
        (lend captures args loans _).mpr ⟨lent, lends, composes⟩, runs, ?_⟩
      rintro ⟨result, returnedLoans, values, lendsResult, lendsProphecy, lendsValues, resolves⟩
      rw [← weave.compose_split values] at lendsValues resolves
      obtain ⟨lent', lends', composes'⟩ := (lend _ _ loans _).mp lendsValues
      obtain ⟨rfl, rfl⟩ := weave.lendArguments_compose_injective (arguments := arguments.toArray)
        lends lends' composes composes'
      exact unresolved ⟨result, returnedLoans, (weave.split values).2, lendsResult, lendsProphecy,
        lends', (resolve _ _ loans _ _).mp resolves⟩
    · rintro ⟨start, loans, arguments, results, exit, globals, admissible, lends, runs,
        unresolved⟩
      obtain ⟨lent, lends', composes⟩ := (lend captures args loans arguments).mp lends
      refine ⟨start, loans, lent, arguments.toList, results, exit, globals, admissible,
        lends', composes, by simpa using runs, ?_⟩
      rintro ⟨result, returnedLoans, values, lendsResult, lendsProphecy, lendsValues, resolves⟩
      exact unresolved ⟨result, returnedLoans, weave.compose captures values, lendsResult,
        lendsProphecy, (lend captures values loans arguments).mpr ⟨lent, lendsValues, composes⟩,
        (resolve captures values loans _ _).mpr resolves⟩

/-! ## Generic targets

A generic target's theorem lives at the family its type arguments induce
(`Skolems.instantiate`). A value at the caller's types is carried there by
`toSkolem` and back by `ofSkolem`; neither changes its runtime encoding, and
type arguments hold no reference, so the target's meaning at the caller's
types is its meaning at that family, read back. -/

omit [Skolems unit] in
/-- A weave with its rows' parameters replaced by type arguments. -/
def Weave.subst (θ : NRow) : {full captured supplied : NRow} → Weave full captured supplied →
    Weave (NRow.subst θ full) (NRow.subst θ captured) (NRow.subst θ supplied)
  | _, _, _, .nil => .nil
  | _, _, _, .captured rest => .captured (rest.subst θ)
  | _, _, _, .supplied rest => .supplied (rest.subst θ)

omit [Skolems unit] in
theorem Weave.mask_subst (θ : NRow) : {full captured supplied : NRow} →
    (weave : Weave full captured supplied) → (weave.subst θ).mask = weave.mask
  | _, _, _, .nil => rfl
  | _, _, _, .captured rest => by simp only [Weave.subst, Weave.mask, rest.mask_subst θ]
  | _, _, _, .supplied rest => by simp only [Weave.subst, Weave.mask, rest.mask_subst θ]

section Transport

omit [Skolems unit] in
theorem NRow.getD_refFree : (row : NRow) → row.refFree = true → (index : Nat) →
    (default : NTy) → default.refFree = true → (row.getD index default).refFree = true
  | .nil, _, _, _, free => free
  | .cons _ _, free, 0, _, _ => by
      simp only [NRow.refFree, Bool.and_eq_true] at free
      exact free.1
  | .cons _ rest, free, index + 1, default, defaultFree => by
      simp only [NRow.refFree, Bool.and_eq_true] at free
      exact NRow.getD_refFree rest free.2 index default defaultFree

mutual
/-- A value lends at the target's family as at the caller's types. -/
theorem NTy.lend_toSkolem (θ : TypeArgs) (free : θ.1.refFree = true) : (τ : NTy) →
    (prophecies : Bool) → (value : (τ.subst θ.1).carrier) → (loans : List Nat) →
    @NTy.lend _ (Skolems.instantiate θ Θ) τ prophecies (NTy.toSkolem θ τ value) loans =
      NTy.lend (τ.subst θ.1) prophecies value loans
  | .ref referent, prophecies, value, loan :: loans => by
      cases prophecies <;>
        simp only [NTy.lend, NTy.toSkolem, NTy.encode_toSkolem, Bool.false_eq_true,
          ite_false, ite_true]
  | .ref _, _, _, [] => rfl
  | .tuple elements, prophecies, values, loans => by
      simp only [NTy.lend, NTy.toSkolem, NRow.lend_toSkolem θ free elements]
  | .param index, prophecies, value, loans => by
      rw [NTy.lend_refFree ((NTy.param index).subst θ.1)
        (NRow.getD_refFree θ.1 free index _ rfl) prophecies value loans]
      simp only [NTy.lend, NTy.encode_toSkolem]
  | .unit, _, _, _ | .bool, _, _, _ | .int _ _, _, _, _ | .address, _, _, _
  | .signer, _, _, _ | .string, _, _, _ | .bytes, _, _, _ | .struct _ _ _, _, _, _
  | .enum _ _ _ _ _, _, _, _ | .vector _, _, _, _ | .function _ _ _, _, _, _ => by
      simp only [NTy.lend, NTy.subst, NTy.encode_toSkolem]

/-- A row lends at the target's family as at the caller's types. -/
theorem NRow.lend_toSkolem (θ : TypeArgs) (free : θ.1.refFree = true) : (row : NRow) →
    (prophecies : Bool) → (values : HList (NRow.subst θ.1 row)) → (loans : List Nat) →
    @NRow.lend _ (Skolems.instantiate θ Θ) row prophecies (HList.toSkolem θ row values) loans =
      NRow.lend (NRow.subst θ.1 row) prophecies values loans
  | .nil, _, _, _ => rfl
  | .cons τ rest, prophecies, values, loans => by
      simp only [NRow.lend, HList.toSkolem, NTy.lend_toSkolem θ free τ,
        NRow.lend_toSkolem θ free rest]
end

/-- Arguments resolve at the target's family as at the caller's types. -/
theorem argumentsResolve_toSkolem (θ : TypeArgs) (free : θ.1.refFree = true) : (σs : NRow) →
    (values : HList (NRow.subst θ.1 σs)) → (loans : List Nat) → (returned : Array RuntimeValue) →
    (exports : List (Nat × RuntimeValue)) →
    (@argumentsResolve _ (Skolems.instantiate θ Θ) σs (HList.toSkolem θ σs values) loans returned
        exports ↔
      argumentsResolve (NRow.subst θ.1 σs) values loans returned exports)
  | .nil, _, _, _, _ => Iff.rfl
  | .cons τ rest, values, loans, returned, exports => by
      have later := argumentsResolve_toSkolem θ free rest
      cases τ with
      | ref referent =>
          cases loans with
          | nil => simp only [argumentsResolve]
          | cons loan loans =>
              simp only [argumentsResolve, NTy.subst, HList.toSkolem, NTy.toSkolem,
                NTy.encode_toSkolem, later]
      | param index =>
          refine Iff.trans ?_ (argumentsResolve_cons_refFree (τ := (NTy.param index).subst θ.1)
            (rest := NRow.subst θ.1 rest) (NRow.getD_refFree θ.1 free index _ rfl) values loans
            returned exports).symm
          simp only [argumentsResolve, HList.toSkolem]
          exact later values.2 loans returned exports
      | _ => simp only [argumentsResolve, NTy.subst, HList.toSkolem, later]

/-- A result in the caller's view lends as it does at the target's family. -/
theorem ResultShape.lend_ofSkolem (θ : TypeArgs) (free : θ.1.refFree = true) :
    (shape : ResultShape) → (prophecies : Bool) →
    (result : @ResultShape.carrier (Skolems.instantiate θ Θ).toCarriers shape) → (loans : List Nat) →
    (shape.subst θ.1).lend prophecies (ResultShape.ofSkolem θ shape result) loans =
      @ResultShape.lend _ (Skolems.instantiate θ Θ) shape prophecies result loans
  | .none, _, _, loans => by cases loans <;> rfl
  | .one τ, prophecies, result, loans => by
      conv => rhs; rw [← NTy.toSkolem_ofSkolem θ τ result]
      simp only [ResultShape.lend, ResultShape.ofSkolem, NTy.lend_toSkolem θ free τ]

/-- A result in the target's view. -/
def ResultShape.toSkolem (θ : TypeArgs) : (shape : ResultShape) →
    (shape.subst θ.1).carrier → @ResultShape.carrier (Skolems.instantiate θ Θ).toCarriers shape
  | .none, value => value
  | .one τ, value => NTy.toSkolem θ τ value

theorem ResultShape.ofSkolem_toSkolem (θ : TypeArgs) : (shape : ResultShape) →
    (value : (shape.subst θ.1).carrier) →
    ResultShape.ofSkolem θ shape (ResultShape.toSkolem θ shape value) = value
  | .none, _ => rfl
  | .one τ, value => NTy.ofSkolem_toSkolem θ τ value

theorem NTy.toSkolem_ofSkolem_shape (θ : TypeArgs) : (shape : ResultShape) →
    (value : @ResultShape.carrier (Skolems.instantiate θ Θ).toCarriers shape) →
    ResultShape.toSkolem θ shape (ResultShape.ofSkolem θ shape value) = value
  | .none, _ => rfl
  | .one τ, value => NTy.toSkolem_ofSkolem θ τ value

theorem lendArguments_toSkolem (θ : TypeArgs) (free : θ.1.refFree = true) (σs : NRow)
    (args : HList (NRow.subst θ.1 σs)) (loans : List Nat) :
    @lendArguments _ (Skolems.instantiate θ Θ) σs (HList.toSkolem θ σs args) loans =
      lendArguments (NRow.subst θ.1 σs) args loans := by
  unfold lendArguments
  rw [NRow.lend_toSkolem θ free]

/-- A callee's run at the caller's types is its run at the family its type
arguments induce, read back in the caller's view. -/
theorem propheticRun_subst {unit : LeanerIR.Validation.ValidatedUnit}
    (executable : LeanerIR.Validation.ExecutableUnit unit) [Θ : Skolems unit]
    (typeInstantiation : Array (TypeId × TypeId)) (handle : FunctionHandle) (θ : TypeArgs)
    (free : θ.1.refFree = true) (σs : NRow) (shape : ResultShape)
    (args : HList (NRow.subst θ.1 σs)) :
    propheticRun executable typeInstantiation handle (NRow.subst θ.1 σs) (shape.subst θ.1) args =
      Spec.bind (@propheticRun _ executable (Skolems.instantiate θ Θ) typeInstantiation handle σs shape
        (HList.toSkolem θ σs args)) fun result => Spec.pure (ResultShape.ofSkolem θ shape result) := by
  have lend (values : HList (NRow.subst θ.1 σs)) (loans : List Nat) :=
    lendArguments_toSkolem θ free σs values loans
  have resolve (values : HList (NRow.subst θ.1 σs)) := argumentsResolve_toSkolem θ free σs values
  have result (prophecies : Bool) (value : (shape.subst θ.1).carrier) (loans : List Nat) :
      (shape.subst θ.1).lend prophecies value loans =
        @ResultShape.lend _ (Skolems.instantiate θ Θ) shape prophecies
          (ResultShape.toSkolem θ shape value) loans := by
    conv => lhs; rw [← ResultShape.ofSkolem_toSkolem θ shape value]
    exact ResultShape.lend_ofSkolem θ free shape prophecies _ loans
  simp only [propheticRun, Spec.bind, Spec.pure]
  congr 1
  · funext initial value final
    apply propext
    constructor
    · rintro ⟨start, loans, arguments, results, exit, returnedLoans, prophecyRow, globals,
        admissible, lends, runs, lendsResult, lendsProphecy, resolves, encoded, agree⟩
      refine ⟨ResultShape.toSkolem θ shape value, final,
        ⟨start, loans, arguments, results, exit, returnedLoans, prophecyRow, globals, admissible,
          by rw [lend]; exact lends, runs, by rw [← result]; exact lendsResult,
          by rw [← result]; exact lendsProphecy, (resolve args loans _ _).mpr resolves, encoded,
          agree⟩,
        (ResultShape.ofSkolem_toSkolem θ shape value).symm, rfl⟩
    · rintro ⟨value', middle, ⟨start, loans, arguments, results, exit, returnedLoans, prophecyRow,
        globals, admissible, lends, runs, lendsResult, lendsProphecy, resolves, encoded, agree⟩,
        rfl, rfl⟩
      refine ⟨start, loans, arguments, results, exit, returnedLoans, prophecyRow, globals,
        admissible, by rw [← lend]; exact lends, runs, ?_, ?_, (resolve args loans _ _).mp resolves,
        encoded, agree⟩
      · rw [result, NTy.toSkolem_ofSkolem_shape θ shape value']; exact lendsResult
      · rw [result, NTy.toSkolem_ofSkolem_shape θ shape value']; exact lendsProphecy
  · funext initial error
    apply propext
    try simp only [and_false, exists_false, or_false]
    constructor
    · rintro ⟨start, loans, arguments, globals, admissible, lends, aborts⟩
      exact ⟨start, loans, arguments, globals, admissible, by rw [lend]; exact lends, aborts⟩
    · rintro ⟨start, loans, arguments, globals, admissible, lends, aborts⟩
      exact ⟨start, loans, arguments, globals, admissible, by rw [← lend]; exact lends, aborts⟩
  · funext initial
    apply propext
    try simp only [and_false, exists_false, or_false]
    constructor
    · rintro ⟨start, loans, arguments, results, exit, globals, admissible, lends, runs, unresolved⟩
      refine ⟨start, loans, arguments, results, exit, globals, admissible,
        by rw [lend]; exact lends, runs, ?_⟩
      rintro ⟨value', returnedLoans, values, lendsResult, lendsProphecy, lendsValues, resolves⟩
      apply unresolved
      refine ⟨ResultShape.ofSkolem θ shape value', returnedLoans, HList.ofSkolem θ σs values,
        ?_, ?_, ?_, ?_⟩
      · rw [result, NTy.toSkolem_ofSkolem_shape]; exact lendsResult
      · rw [result, NTy.toSkolem_ofSkolem_shape]; exact lendsProphecy
      · rw [← lend, HList.toSkolem_ofSkolem]; exact lendsValues
      · rw [← resolve, HList.toSkolem_ofSkolem]; exact resolves
    · rintro ⟨start, loans, arguments, results, exit, globals, admissible, lends, runs, unresolved⟩
      refine ⟨start, loans, arguments, results, exit, globals, admissible,
        by rw [← lend]; exact lends, runs, ?_⟩
      rintro ⟨value, returnedLoans, values, lendsResult, lendsProphecy, lendsValues, resolves⟩
      apply unresolved
      exact ⟨ResultShape.toSkolem θ shape value, returnedLoans, HList.toSkolem θ σs values,
        by rw [← result]; exact lendsResult, by rw [← result]; exact lendsProphecy,
        by rw [lend]; exact lendsValues, (resolve values loans _ _).mpr resolves⟩

/-- An invocation of a function value at the family a call's type arguments
induce is its invocation at the caller's types, read in the callee's view. -/
theorem closureMeaning_instantiate {unit : LeanerIR.Validation.ValidatedUnit}
    (executable : LeanerIR.Validation.ExecutableUnit unit) [Θ : Skolems unit]
    (closure : ClosureValue) (θ : TypeArgs) (free : θ.1.refFree = true) (σs : NRow)
    (shape : ResultShape) (args : @HList (Skolems.instantiate θ Θ).toCarriers σs) :
    @closureMeaning _ executable (Skolems.instantiate θ Θ) closure σs shape args =
      Spec.bind (closureMeaning executable closure (NRow.subst θ.1 σs) (shape.subst θ.1)
          (HList.ofSkolem θ σs args))
        fun result => Spec.pure (ResultShape.toSkolem θ shape result) := by
  have lend (loans : List Nat) :
      lendArguments (NRow.subst θ.1 σs) (HList.ofSkolem θ σs args) loans =
        @lendArguments _ (Skolems.instantiate θ Θ) σs args loans := by
    rw [← lendArguments_toSkolem θ free σs, HList.toSkolem_ofSkolem]
  have lendAt (values : HList (NRow.subst θ.1 σs)) (loans : List Nat) :=
    lendArguments_toSkolem θ free σs values loans
  have resolve (values : HList (NRow.subst θ.1 σs)) := argumentsResolve_toSkolem θ free σs values
  have result (prophecies : Bool)
      (value : @ResultShape.carrier (Skolems.instantiate θ Θ).toCarriers shape) (loans : List Nat) :
      @ResultShape.lend _ (Skolems.instantiate θ Θ) shape prophecies value loans =
        (shape.subst θ.1).lend prophecies (ResultShape.ofSkolem θ shape value) loans :=
    (ResultShape.lend_ofSkolem θ free shape prophecies value loans).symm
  simp only [closureMeaning, Spec.bind, Spec.pure]
  congr 1
  · funext initial value final
    apply propext
    constructor
    · rintro ⟨start, loans, supplied, arguments, results, exit, returnedLoans, prophecyRow, globals,
        admissible, lends, composes, runs, lendsResult, lendsProphecy, resolves, encoded, agree⟩
      refine ⟨ResultShape.ofSkolem θ shape value, final,
        ⟨start, loans, supplied, arguments, results, exit, returnedLoans, prophecyRow, globals,
          admissible, by rw [lend]; exact lends, composes, runs, by rw [← result]; exact lendsResult,
          by rw [← result]; exact lendsProphecy, ?_, encoded, agree⟩,
        (NTy.toSkolem_ofSkolem_shape θ shape value).symm, rfl⟩
      rw [← resolve, HList.toSkolem_ofSkolem]
      exact resolves
    · rintro ⟨value', middle, ⟨start, loans, supplied, arguments, results, exit, returnedLoans,
        prophecyRow, globals, admissible, lends, composes, runs, lendsResult, lendsProphecy,
        resolves, encoded, agree⟩, rfl, rfl⟩
      refine ⟨start, loans, supplied, arguments, results, exit, returnedLoans, prophecyRow, globals,
        admissible, by rw [← lend]; exact lends, composes, runs, ?_, ?_, ?_, encoded, agree⟩
      · rw [result, ResultShape.ofSkolem_toSkolem]; exact lendsResult
      · rw [result, ResultShape.ofSkolem_toSkolem]; exact lendsProphecy
      · rw [← HList.toSkolem_ofSkolem θ σs args, resolve]; exact resolves
  · funext initial error
    apply propext
    try simp only [and_false, exists_false, or_false]
    constructor
    · rintro ⟨start, loans, supplied, arguments, globals, admissible, lends, composes, aborts⟩
      exact ⟨start, loans, supplied, arguments, globals, admissible, by rw [lend]; exact lends,
        composes, aborts⟩
    · rintro ⟨start, loans, supplied, arguments, globals, admissible, lends, composes, aborts⟩
      exact ⟨start, loans, supplied, arguments, globals, admissible, by rw [← lend]; exact lends,
        composes, aborts⟩
  · funext initial
    apply propext
    try simp only [and_false, exists_false, or_false]
    constructor
    · rintro ⟨start, loans, supplied, arguments, results, exit, globals, admissible, lends, composes,
        runs, unresolved⟩
      refine ⟨start, loans, supplied, arguments, results, exit, globals, admissible,
        by rw [lend]; exact lends, composes, runs, ?_⟩
      rintro ⟨value', returnedLoans, values, lendsResult, lendsProphecy, lendsValues, resolves⟩
      apply unresolved
      exact ⟨ResultShape.toSkolem θ shape value', returnedLoans, HList.toSkolem θ σs values,
        by rw [result, ResultShape.ofSkolem_toSkolem]; exact lendsResult,
        by rw [result, ResultShape.ofSkolem_toSkolem]; exact lendsProphecy,
        by rw [lendAt]; exact lendsValues, (resolve values loans _ _).mpr resolves⟩
    · rintro ⟨start, loans, supplied, arguments, results, exit, globals, admissible, lends, composes,
        runs, unresolved⟩
      refine ⟨start, loans, supplied, arguments, results, exit, globals, admissible,
        by rw [← lend]; exact lends, composes, runs, ?_⟩
      rintro ⟨value, returnedLoans, values, lendsResult, lendsProphecy, lendsValues, resolves⟩
      apply unresolved
      refine ⟨ResultShape.ofSkolem θ shape value, returnedLoans, HList.ofSkolem θ σs values,
        ?_, ?_, ?_, ?_⟩
      · rw [← result]; exact lendsResult
      · rw [← result]; exact lendsProphecy
      · rw [← lendAt, HList.toSkolem_ofSkolem]; exact lendsValues
      · rw [← resolve, HList.toSkolem_ofSkolem]; exact resolves

/-- A contract at the frame type arguments induce, read in the caller's
view. -/
def _root_.LeanerIR.Proofs.Contract.ofSkolem (θ : TypeArgs) {σs : NRow} {shape : ResultShape}
    (contract : Contract (Memory unit) Failure (@HList (Skolems.instantiate θ Θ).toCarriers σs)
      (@ResultShape.carrier (Skolems.instantiate θ Θ).toCarriers shape)) :
    Contract (Memory unit) Failure (HList (NRow.subst θ.1 σs)) (shape.subst θ.1).carrier where
  requires := fun args => contract.requires (HList.toSkolem θ σs args)
  assumes := fun args => contract.assumes (HList.toSkolem θ σs args)
  ensures := fun args initial result final =>
    contract.ensures (HList.toSkolem θ σs args) initial (ResultShape.toSkolem θ shape result)
      final
  aborts := fun args => contract.aborts (HList.toSkolem θ σs args)
  mayAbort := fun args => contract.mayAbort (HList.toSkolem θ σs args)
  mustAbort := fun args => contract.mustAbort (HList.toSkolem θ σs args)
  frame := fun args => contract.frame (HList.toSkolem θ σs args)

/-- A contract the run satisfies at the frame type arguments induce holds,
read in the caller's view, of the run at the caller's types. -/
theorem satisfies_run_ofSkolem (executable : LeanerIR.Validation.ExecutableUnit unit)
    (θ : TypeArgs) (free : θ.1.refFree = true) (typeInstantiation : Array (TypeId × TypeId))
    (handle : FunctionHandle) (σs : NRow) (shape : ResultShape)
    (contract : Contract (Memory unit) Failure (@HList (Skolems.instantiate θ Θ).toCarriers σs)
      (@ResultShape.carrier (Skolems.instantiate θ Θ).toCarriers shape))
    (verified : Satisfies
      (@propheticRun _ executable (Skolems.instantiate θ Θ) typeInstantiation handle σs shape)
      contract) :
    Satisfies
      (propheticRun executable typeInstantiation handle (NRow.subst θ.1 σs) (shape.subst θ.1))
      (contract.ofSkolem θ) := by
  intro args initial assumed required
  obtain ⟨normal, failing, defined⟩ :=
    verified (HList.toSkolem θ σs args) initial assumed required
  rw [propheticRun_subst executable typeInstantiation handle θ free σs shape args]
  refine ⟨fun result final ok => ?_, fun error aborted => ?_, fun undefined => ?_⟩
  · simp only [Spec.bind_ok, Spec.pure_ok] at ok
    obtain ⟨value, middle, run, rfl, rfl⟩ := ok
    simpa only [Contract.ofSkolem, NTy.toSkolem_ofSkolem_shape] using normal value _ run
  · simp only [Spec.bind_aborts, Spec.pure_aborts, and_false, exists_false, or_false] at aborted
    exact failing error aborted
  · rw [Spec.bind_undefined] at undefined
    simp only [Spec.pure_undefined, and_false, exists_false, or_false] at undefined
    exact defined undefined

end Transport


omit [Skolems unit] in
mutual
/-- A type without references keeps none under reference-free type arguments. -/
theorem NTy.refFree_subst (θ : NRow) (free : θ.refFree = true) : (τ : NTy) →
    τ.refFree = true → (τ.subst θ).refFree = true
  | .param index, _ => NRow.getD_refFree θ free index _ rfl
  | .ref _, typeFree => by simp [NTy.refFree] at typeFree
  | .tuple elements, typeFree => by
      simp only [NTy.refFree] at typeFree
      simp only [NTy.refFree, NRow.refFree_subst θ free elements typeFree]
  | .struct _ _ fields, typeFree => by
      simp only [NTy.refFree] at typeFree
      simp only [NTy.refFree, NRow.refFree_subst θ free fields typeFree]
  | .enum _ _ _ rows _, typeFree => by
      simp only [NTy.refFree] at typeFree
      simp only [NTy.refFree, NRows.refFree_subst θ free rows typeFree]
  | .vector element, typeFree => by
      simp only [NTy.refFree] at typeFree
      simp only [NTy.refFree, NTy.refFree_subst θ free element typeFree]
  | .unit, _ | .bool, _ | .int _ _, _ | .address, _ | .signer, _ | .string, _ | .bytes, _
  | .function _ _ _, _ => rfl

theorem NRow.refFree_subst (θ : NRow) (free : θ.refFree = true) : (row : NRow) →
    row.refFree = true → (NRow.subst θ row).refFree = true
  | .nil, _ => rfl
  | .cons τ rest, rowFree => by
      simp only [NRow.refFree, Bool.and_eq_true] at rowFree
      simp only [NRow.refFree, NTy.refFree_subst θ free τ rowFree.1,
        NRow.refFree_subst θ free rest rowFree.2, Bool.and_self]

theorem NRows.refFree_subst (θ : NRow) (free : θ.refFree = true) : (rows : NRows) →
    rows.refFree = true → (rows.subst θ).refFree = true
  | .nil, _ => rfl
  | .cons fields rest, rowsFree => by
      simp only [NRows.refFree, Bool.and_eq_true] at rowsFree
      simp only [NRows.refFree, NRow.refFree_subst θ free fields rowsFree.1,
        NRows.refFree_subst θ free rest rowsFree.2, Bool.and_self]
end

/-- Invoking a known closure with type arguments is calling its target at the
family they induce, read back in the caller's view, where that frame is
coherent with the closure's instantiation: the form of a direct call with
type arguments. -/
theorem closureMeaning_closureOfGeneric {unit : LeanerIR.Validation.ValidatedUnit}
    (executable : LeanerIR.Validation.ExecutableUnit unit) [Θ : Skolems unit]
    (θ : TypeArgs) (free : θ.1.refFree = true) {full captured supplied : NRow}
    (weave : Weave full captured supplied) (capturedFree : captured.refFree = true)
    (shape : ResultShape) (handle : FunctionHandle) (outer : Array (TypeId × TypeId))
    (typeArgs : Array TypeUse)
    (coherent : @Coherent unit (Skolems.instantiate θ Θ) handle
      (frameInstantiation unit handle outer typeArgs))
    (captures : HList (NRow.subst θ.1 captured)) (args : HList (NRow.subst θ.1 supplied)) :
    closureMeaning executable (closureOfGeneric unit outer typeArgs θ weave shape handle captures)
      (NRow.subst θ.1 supplied) (shape.subst θ.1) args =
      Spec.bind (closedGeneric executable outer handle typeArgs θ full shape
          (HList.toSkolem θ full ((weave.subst θ.1).compose captures args)))
        fun result => Spec.pure (ResultShape.ofSkolem θ shape result) := by
  rw [closureOfGeneric, ← weave.mask_subst θ.1,
    closureMeaning_closureOf executable handle (weave.subst θ.1)
      (NRow.refFree_subst θ.1 free captured capturedFree) _ captures,
    propheticRun_subst executable _ handle θ free full shape]
  simp only [closedGeneric]
  rw [@propheticMeaning_of_coherent _ _ (Skolems.instantiate θ Θ) _ _ coherent]

end Frames

/-- Invoking a known closure without type arguments at the runtime frame,
which runs in the empty instantiation, is calling its target: the form of a
direct call, whose callee theorem a proof applies. -/
theorem closureMeaning_closureOf_nil {unit : LeanerIR.Validation.ValidatedUnit}
    (executable : LeanerIR.Validation.ExecutableUnit unit)
    (handle : FunctionHandle) {full captured supplied : NRow}
    (weave : Weave full captured supplied) (free : captured.refFree = true)
    (captures : @HList (Carriers.runtime unit) captured) (shape : ResultShape)
    (args : @HList (Carriers.runtime unit) supplied) :
    letI : Skolems unit := Skolems.runtime unit
    closureMeaning executable (closureOf handle weave.mask #[] captures) supplied shape args =
      propheticMeaning executable #[] handle full shape (weave.compose captures args) := by
  letI : Skolems unit := Skolems.runtime unit
  rw [propheticMeaning_of_coherent (coherent_runtime unit handle)]
  exact closureMeaning_closureOf executable handle weave free #[] captures shape args

-- An invocation of a known closure is its target's call: the normalizer
-- weaves the arguments and decides that the captures hold no reference.
attribute [lir_denote] closureMeaning_closureOf_nil Weave.compose Weave.subst NTy.refFree
  NRow.refFree NRows.refFree

-- An invocation inside a callee inlined at type arguments is the
-- invocation at the caller's types.
attribute [lir_denote] closureMeaning_instantiate ResultShape.toSkolem

open Lean Meta in
/-- A proof of `Coherent unit handle typeInstantiation` by evaluation, of a
unit, family, and instantiation without free variables (`Coherent.ofCheck`). -/
def coherentByEvaluation (condition : Lean.Expr) : MetaM (Option Lean.Expr) := do
  let_expr Coherent unitExpr family handle typeInstantiation := condition | return none
  if condition.hasFVar || condition.hasMVar then return none
  let check := mkApp4 (mkConst ``coherentCheck) unitExpr family handle typeInstantiation
  let decided ← mkDecideProof (← mkEq check (mkConst ``Bool.true))
  return some (mkAppN (mkConst ``Coherent.ofCheck)
    #[unitExpr, family, handle, typeInstantiation, decided])

open Lean Meta in
/-- A proof of a frame's coherence: a hypothesis the closure's creation took,
or evaluation. -/
def coherentProof? (condition : Lean.Expr) : MetaM (Option Lean.Expr) := do
  let_expr Coherent unitExpr family handle typeInstantiation := condition | return none
  let frame := mkAppN (mkConst ``ClosureFrame) #[unitExpr, family, handle, typeInstantiation]
  let local? ← (← getLCtx).findDeclRevM? fun declaration => do
    if declaration.isImplementationDetail then return none
    if ← isDefEq declaration.type condition then return some declaration.toExpr
    if ← isDefEq declaration.type frame then
      return some (← mkAppM ``And.left #[declaration.toExpr])
    return none
  match local? with
  | some proof => return some proof
  | none => coherentByEvaluation condition

open Lean Meta in
/-- A proof of a closure's target frame: a hypothesis its creation took, or
evaluation of its coherence and faithfulness. -/
def closureFrameProof? (condition : Lean.Expr) : MetaM (Option Lean.Expr) := do
  let_expr ClosureFrame unitExpr family handle typeInstantiation := condition | return none
  let local? ← (← getLCtx).findDeclRevM? fun declaration => do
    if declaration.isImplementationDetail then return none
    if ← isDefEq declaration.type condition then return some declaration.toExpr
    return none
  if let some proof := local? then return some proof
  let some coherent ← coherentByEvaluation
      (mkAppN (mkConst ``Coherent) #[unitExpr, family, handle, typeInstantiation])
    | return none
  let check := mkApp3 (mkConst ``closureFaithful) unitExpr handle typeInstantiation
  let faithful ← mkDecideProof (← mkEq check (mkConst ``Bool.true))
  return some (← mkAppM ``And.intro #[coherent, faithful])

open Lean Meta Simp in
/-- Create a closure with type arguments where its target's frame is
coherent and faithful (`Spec.given_of`). -/
simproc [lir_denote] givenClosureFrame (Spec.given (@ClosureFrame _ ?family _ _) _) := fun e => do
  let_expr Spec.given _ _ _ condition continuation := e | return .continue
  let some holds ← closureFrameProof? (← instantiateMVars condition) | return .continue
  let proof ← mkAppM ``Spec.given_of #[holds, continuation]
  return .visit { expr := continuation.beta #[holds], proof? := some proof }

open Lean Meta Simp in
/-- Rewrite the invocation of a known closure with type arguments by
`closureMeaning_closureOfGeneric`. An invocation spells the rows the target's
weave and type arguments substitute, which the equation's index cannot match,
so the equation is instantiated by unification and its reference-freedom
conditions are decided. -/
simproc [lir_denote] invokeGenericClosure
    (@closureMeaning _ _ ?frame (@closureOfGeneric _ ?family _ _ _ _ _ _ _ _ _ _) _ _ _) := fun e => do
  let equation ← mkConstWithFreshMVarLevels ``closureMeaning_closureOfGeneric
  let (arguments, _, statement) ← forallMetaTelescope (← inferType equation)
  let some (_, lhs, rhs) := statement.eq? | return .continue
  unless ← isDefEq lhs e do return .continue
  for argument in arguments do
    if (← instantiateMVars argument).isMVar then
      let condition ← instantiateMVars (← inferType argument)
      if condition.isAppOfArity ``Coherent 4 then
        -- The target's frame is coherent as the closure's creation took it.
        let some proof ← coherentProof? condition | return .continue
        unless ← isDefEq argument proof do return .continue
        continue
      unless condition.isAppOfArity ``Eq 3 && !condition.hasMVar do return .continue
      unless ← isDefEq argument (← mkDecideProof condition) do return .continue
  let proof ← instantiateMVars (mkAppN equation arguments)
  if proof.hasMVar then return .continue
  return .visit { expr := ← instantiateMVars rhs, proof? := some proof }

end LeanerIR.Proofs.Denote

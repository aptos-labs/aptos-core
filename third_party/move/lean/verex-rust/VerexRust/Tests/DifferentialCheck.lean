-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexRust.Driver
import VerexRust.Tests.DenoteEval
import VerexSL.Print
import VerexSL.SourceVerify

/-!
# The differential check of the denotation against the interpreter

`compileFunction_agrees` is an axiom, so no proof catches a wrong denotation
case. This check exports each fixture through rustc, runs the interpreter on the
exported unit, and compares its outcome with the term verification compiles from
the rendering lowered back, on boundary values per integer type and on all rows
of 8-bit integers for the operations and casts: a return means that result, a failure that failure
and payload, an unchecked operation off its domain `Spec.stuck`. A `&mut` parameter's prophecy is
the value the run writes back; another prophecy leaves no execution, unless the reference
is returned, and the prophecies of local borrows are searched. Any other interpreter error
derives nothing and agrees with nothing, and a function no input runs to a return or an abort
disagrees, so a construct without a runtime derivation cannot pass.

The term's outcome is computed by `Term.tree`, not read from a copy of the
denotation: `checked_tree_means` proves it means `Function.denoteWith`, with each
callee's denotation inlined, so a denotation case changed without its tree does
not build.
-/

namespace VerexIR.Rust.Tests.Differential

open VerexIR
open VerexIR.SemanticOperations
open VerexIR.Proofs
open VerexIR.Proofs.Denote
open VerexIR.Validation

/-! ## The runtime family, computably -/

section Family
variable (unit : ValidatedUnit) [FunctionCarriers unit]

private def plainOfRuntime (raw : RuntimeValue) : PlainValue :=
  if plain : Plain raw then ⟨raw, plain⟩ else ⟨.unit, .unit⟩

private def plainReads : TypedReads PlainValue Subtype.val where
  typed := PlainValue
  read := fun value => value
  decEq := inferInstance
  inhabited := ⟨⟨.unit, .unit⟩⟩
  write := Subtype.val
  back := plainOfRuntime
  back_write := fun value => by simp [plainOfRuntime, value.property]
  back_unit := by rw [plainOfRuntime, dif_pos .unit]; rfl
  write_read := fun _ => rfl
  unread := fun value => value
  unread_read := fun _ => rfl
  order := SpecMap.KeyOrder.byRuntime Subtype.val fun _ _ same => Subtype.ext same

/-- `Carriers.runtime` with its decisions computed. -/
@[reducible] def runtimeCarriers : Carriers where
  carrier := fun _ => PlainValue
  codec := fun _ =>
    ⟨Subtype.val, fun raw => if plain : Plain raw then some ⟨raw, plain⟩ else none,
      fun value => by simp [value.property]⟩
  decEq := fun _ => inferInstance
  inhabited := fun _ => ⟨⟨.unit, .unit⟩⟩
  tight := fun _ raw value decoded => by
    by_cases plain : Plain raw <;> simp [plain] at decoded
    subst decoded
    rfl
  plain := fun _ value => value.property
  functionCarrier := FunctionCarriers.carrier unit
  functions := FunctionCarriers.functions
  nominalCarrier := RuntimeNominal unit
  nominals := RuntimeNominal.nominals unit
  nominalInhabitant := fun source arguments _ =>
    ⟨witnessOrUnit unit source arguments, NTy.admits_witnessOrUnit unit source arguments⟩
  nominalInhabitant_unit := @Carriers.nominalInhabitant_unit (Carriers.runtime unit)
  reads := fun _ => plainReads
  orders := fun _ => PlainValue.orders
  erasures := fun _ => PlainValue.erasures

theorem runtimeCarriers_eq : runtimeCarriers unit = Carriers.runtime unit := by
  have reads : plainReads = PlainValue.reads := by
    unfold plainReads PlainValue.reads
    congr
    funext left right
    exact Subsingleton.elim _ _
  unfold runtimeCarriers Carriers.runtime
  congr
  · funext _ left right
    exact Subsingleton.elim _ _
  · funext _
    exact reads

private theorem carrier_eq {F G : Carriers} (same : F = G) (τ : NTy) :
    @NTy.carrier F τ = @NTy.carrier G τ := by
  subst same
  rfl

private theorem encode_cast {F G : Carriers} (same : F = G) (τ : NTy) (value : @NTy.carrier F τ) :
    @NTy.encode G τ (cast (carrier_eq same τ) value) = @NTy.encode F τ value := by
  subst same
  rfl

/-- `Skolems.runtime` with its decisions computed: its family is the same one
(`runtimeCarriers_eq`). -/
@[reducible] def runtimeSkolems : Skolems unit where
  toCarriers := runtimeCarriers unit
  resolve := fun τ => τ
  resolve_eq := fun τ => (NTy.substWith_param τ).symm
  toRuntime := fun τ value => cast (carrier_eq (runtimeCarriers_eq unit) τ) value
  ofRuntime := fun τ value => cast (carrier_eq (runtimeCarriers_eq unit).symm τ) value
  ofRuntime_toRuntime := fun _ _ => by simp
  toRuntime_ofRuntime := fun _ _ => by simp
  encode_toRuntime := fun τ value => encode_cast (runtimeCarriers_eq unit) τ value
  type_inhabitable := fun _ => rfl

end Family

/-! ## Inputs -/

section Inputs
variable [Carriers]

/-- The values an integer's boundary is at, most telling first: limits, 0, 1,
-1, and the shift distances at and around the width. -/
def intCandidates (width : Nat) (signed : Bool) : List Int :=
  let max : Int := if signed then 2 ^ (width - 1) - 1 else 2 ^ width - 1
  let min : Int := if signed then -(2 ^ (width - 1)) else 0
  [0, 1, max, min, -1, max - 1, min + 1, 2, (width : Int) - 1, (width : Int), (width : Int) + 1,
    7, 8, 31, 32, 63, 64, 127, 128, 255, max / 2, min / 2, -2, -7, 3,
    0xD7FF, 0xD800, 0xDFFF, 0xE000, 0x10FFFF, 0x110000]

def intValues (width : Nat) (signed : Bool) : List (SpecInt (.bits width) signed) :=
  (intCandidates width signed).eraseDups.filterMap fun value =>
    if fits : IntegerValueFits (.bits width) signed value then some ⟨value, fits⟩ else none

def charValues : List SpecChar :=
  [0, 1, 0x41, 0x7F, 0x80, 0xFF, 0xD7FF, 0xE000, 0xFFFF, 0x10000, 0x10FFFF].filterMap fun value =>
    if valid : isUnicodeScalar value = true then some ⟨value, valid⟩ else none

/-- At most `cap` values, spread evenly over the list. -/
def thin {α : Type} (cap : Nat) (values : List α) : List α :=
  if cap = 0 ∨ values.length ≤ cap then values
  else
    let stride := (values.length + cap - 1) / cap
    (values.zipIdx.filter fun entry => entry.2 % stride == 0).map (·.1)

mutual
/-- Sample values of a type, aggregates thinned; none for a type this check
cannot produce. -/
def NTy.samples : Nat → (τ : NTy) → List τ.carrier
  | 0, _ => []
  | _ + 1, .unit => [()]
  | _ + 1, .bool => [false, true]
  | _ + 1, .int width signed => intValues width signed
  | _ + 1, .char => charValues
  | depth + 1, .tuple elements => NRow.samples depth elements
  | depth + 1, .struct _ _ fields => NRow.samples depth fields
  | depth + 1, .enum _ _ names rows _ => variantSamples depth names rows
  | depth + 1, .vector element =>
      let values := (NTy.samples depth element).take 4
      let arrays : List (Array element.carrier) :=
        [#[]] ++ values.map (fun value => #[value]) ++ [values.toArray]
      arrays.filterMap SpecVector.ofArray?
  | depth + 1, .array element length =>
      let values := (NTy.samples depth element).take 4
      let arrays : List (Array element.carrier) :=
        values.map (Array.replicate length) ++
          if values.isEmpty then [] else
            [(List.range length).filterMap (fun index => values[index % values.length]?) |>.toArray]
      arrays.filterMap fun array =>
        match SpecVector.ofArray? array with
        | some vector =>
            if sized : vector.values.size = length then some ⟨vector, sized⟩ else none
        | none => none
  | depth + 1, .ref referent => (NTy.samples depth referent).map fun value => (value, value)
  | _ + 1, .num => []
  | _ + 1, .address => []
  | _ + 1, .signer => []
  | _ + 1, .string => []
  | _ + 1, .bytes => []
  | _ + 1, .table _ _ => []
  | _ + 1, .ghost _ => []
  | _ + 1, .param _ => []
  | _ + 1, .function _ _ _ => []
  | _ + 1, .nominal _ _ _ => []

/-- Rows of sample values, at most a few per component. -/
def NRow.samples : Nat → (row : NRow) → List (HList row)
  | 0, _ => []
  | _ + 1, .nil => [HList.nil]
  | depth + 1, .cons τ rest =>
      let heads := thin 5 (NTy.samples depth τ)
      let tails := NRow.samples depth rest
      heads.flatMap fun head => tails.map fun tail => HList.cons head tail

def variantSamples : Nat → (names : List String) → (rows : NRows) → List (variantCarrier names rows)
  | 0, _, _ => []
  | _ + 1, _, .nil => []
  | _ + 1, [], .cons _ _ => []
  | depth + 1, _ :: names, .cons fields rest =>
      (thin 8 (NRow.samples depth fields)).map Sum.inl ++
        (variantSamples depth names rest).map Sum.inr
end

/-- How many inputs a function gets. -/
inductive Coverage where
  /-- All rows of at most two 8-bit integers, boundary values otherwise. -/
  | exhaustive
  /-- Every combination of the boundary values. -/
  | boundary
  /-- The most telling boundary values of each integer. -/
  | smoke
  deriving BEq, Inhabited

/-- Argument rows of a parameter row: every combination of the values of `row`
at the given per-parameter cap. -/
def argumentRows (coverage : Coverage) : (row : NRow) → Nat → List (HList row)
  | .nil, _ => [HList.nil]
  | .cons τ rest, cap =>
      let values := NTy.samples 16 τ
      let heads := thin cap (if coverage == .smoke then values.take 14 else values)
      let tails := argumentRows coverage rest cap
      heads.flatMap fun head => tails.map fun tail => HList.cons head tail

/-- Every value of an 8-bit integer. -/
def allBytes (signed : Bool) : List (SpecInt (.bits 8) signed) :=
  (List.range 256).filterMap fun (index : Nat) =>
    let value : Int := if signed then (index : Int) - 128 else (index : Int)
    if fits : IntegerValueFits (.bits 8) signed value then some ⟨value, fits⟩ else none

/-- All rows of at most two 8-bit integers, or none for another row. -/
def exhaustiveRows : (row : NRow) → Option (List (HList row))
  | .cons (.int 8 signed) .nil => some ((allBytes signed).map fun a => HList.cons a HList.nil)
  | .cons (.int 8 signed) (.cons (.int 8 signed') .nil) =>
      some ((allBytes signed).flatMap fun a => (allBytes signed').map fun b =>
        HList.cons a (HList.cons b HList.nil))
  | _ => none

end Inputs

/-! ## Comparison with the interpreter -/

section Compare
variable (unit : ValidatedUnit) [FunctionCarriers unit] [Skolems unit]

variable {unit}

mutual
/-- A value read back from its lent form: each reference it holds keeps its
prophecy, which the run does not state, and takes its current value from the
borrow. -/
def NTy.readLent : (τ : NTy) → RuntimeValue → τ.carrier → Option τ.carrier
  | .ref σ, .borrow _ current, value =>
      (NTy.readLent σ current value.1).map fun read => (read, value.2)
  | .tuple elements, .tuple fields, values => NRow.readLent elements fields.toList values
  | .struct source arguments fields, .nominal _ _ raws, values =>
      NRow.readLent fields raws.toList values
  | .enum source arguments names rows distinct, raw@(.nominal _ variant raws), value =>
      (variantReadLent names rows variant raws.toList value).orElse fun _ =>
        (NTy.enum source arguments names rows distinct).codec.decode? raw
  | τ, raw, _ => τ.codec.decode? raw

def NRow.readLent : (row : NRow) → List RuntimeValue → HList row → Option (HList row)
  | .nil, [], values => some values
  | .cons τ rest, raw :: raws, values => do
      let head ← NTy.readLent τ raw values.1
      let tail ← NRow.readLent rest raws values.2
      some (HList.cons head tail)
  | _, _, _ => none

/-- An enum's value read back in the variant it is lent at, if the run left it
there: what a callee left in another variant holds no reference that was lent. -/
def variantReadLent : (names : List String) → (rows : NRows) → Option String → List RuntimeValue →
    variantCarrier names rows → Option (variantCarrier names rows)
  | _, .nil, _, _, value => nomatch value
  | [], .cons _ _, _, _, value => nomatch value
  | name :: _, .cons fields _, variant, raws, .inl values =>
      if variant == some name then (NRow.readLent fields raws values).map .inl else none
  | _ :: names, .cons _ rest, variant, raws, .inr later =>
      (variantReadLent names rest variant raws later).map .inr
end

mutual
/-- A value with each reference it holds at the prophecy the run's exports state,
and the loans it leaves. -/
def NTy.withProphecies : (τ : NTy) → τ.carrier → List Nat → Array RuntimeValue →
    List (Nat × RuntimeValue) → Option (τ.carrier × List Nat)
  | .ref σ, value, loan :: loans, results, exports => do
      let raw ←
        match exportedRaw? loan exports with
        | some exported => some (resolveReturnedBorrows results exported)
        | none => (results.toList.flatMap fun entry => (outermostBorrows entry).toList).lookup loan
      let prophecy ← NTy.readLent σ raw value.1
      let (_, rest) ← σ.lend false value.1 loans
      some ((value.1, prophecy), rest)
  | .ref _, _, [], _, _ => none
  | .tuple elements, values, loans, results, exports =>
      NRow.withProphecies elements values loans results exports
  | .struct _ _ fields, values, loans, results, exports =>
      NRow.withProphecies fields values loans results exports
  | .enum _ _ names rows _, value, loans, results, exports =>
      variantWithProphecies names rows value loans results exports
  | τ, value, loans, _, _ => (τ.lend false value loans).map fun lent => (value, lent.2)

def NRow.withProphecies : (row : NRow) → HList row → List Nat → Array RuntimeValue →
    List (Nat × RuntimeValue) → Option (HList row × List Nat)
  | .nil, values, loans, _, _ => some (values, loans)
  | .cons τ rest, values, loans, results, exports => do
      let (head, loans) ← NTy.withProphecies τ values.1 loans results exports
      let (tail, loans) ← NRow.withProphecies rest values.2 loans results exports
      some (HList.cons head tail, loans)

/-- An enum's value with the references of its active variant at their prophecies. -/
def variantWithProphecies : (names : List String) → (rows : NRows) →
    variantCarrier names rows → List Nat → Array RuntimeValue → List (Nat × RuntimeValue) →
    Option (variantCarrier names rows × List Nat)
  | _, .nil, value, _, _, _ => nomatch value
  | [], .cons _ _, value, _, _, _ => nomatch value
  | _ :: _, .cons fields _, .inl values, loans, results, exports =>
      (NRow.withProphecies fields values loans results exports).map fun found => (.inl found.1, found.2)
  | _ :: names, .cons _ rest, .inr later, loans, results, exports =>
      (variantWithProphecies names rest later loans results exports).map fun found =>
        (.inr found.1, found.2)
end

/-- The argument row with each reference's prophecy from the run's exports. -/
def withProphecies (row : NRow) (values : HList row) (loans : List Nat)
    (results : Array RuntimeValue) (exports : List (Nat × RuntimeValue)) : Option (HList row) :=
  (NRow.withProphecies row values loans results exports).map (·.1)

mutual
/-- A value with one reference's prophecy changed, with that reference's loan. -/
def NTy.perturbed : (τ : NTy) → τ.carrier → List Nat → List (τ.carrier × Nat)
  | .ref σ, value, loans =>
      let loan := loans.headD 0
      let others := ((thin 8 (NTy.samples 16 σ)).filter fun other =>
        match NTy.decEq σ other value.2 with
        | isTrue _ => false
        | isFalse _ => true).take 2
      others.map fun other => ((value.1, other), loan)
  | .tuple elements, values, loans => NRow.perturbed elements values loans
  | .struct _ _ fields, values, loans => NRow.perturbed fields values loans
  | .enum _ _ names rows _, value, loans => variantPerturbed names rows value loans
  | _, _, _ => []

def NRow.perturbed : (row : NRow) → HList row → List Nat → List (HList row × Nat)
  | .nil, _, _ => []
  | .cons τ rest, values, loans =>
      let left := match τ.lend false values.1 loans with
        | some (_, left) => left
        | none => loans
      (NTy.perturbed τ values.1 loans).map (fun (head, at') => (HList.cons head values.2, at')) ++
        (NRow.perturbed rest values.2 left).map fun (tail, at') => (HList.cons values.1 tail, at')

/-- An enum's value with one reference of its active variant at another prophecy. -/
def variantPerturbed : (names : List String) → (rows : NRows) → variantCarrier names rows →
    List Nat → List (variantCarrier names rows × Nat)
  | _, .nil, value, _ => nomatch value
  | [], .cons _ _, value, _ => nomatch value
  | _ :: _, .cons fields _, .inl values, loans =>
      (NRow.perturbed fields values loans).map fun (found, at') => (.inl found, at')
  | _ :: names, .cons _ rest, .inr later, loans =>
      (variantPerturbed names rest later loans).map fun (found, at') => (.inr found, at')
end

/-- The argument row with one reference's prophecy changed, with that
reference's loan. -/
def perturbed (row : NRow) (values : HList row) (loans : List Nat) : List (HList row × Nat) :=
  NRow.perturbed row values loans

/-- A value with its nominal handles translated. -/
partial def translated (handle : StructHandle → StructHandle) : RuntimeValue → RuntimeValue
  | .nominal source variant fields =>
      .nominal (handle source) variant (fields.map (translated handle))
  | .vector elements => .vector (elements.map (translated handle))
  | .tuple elements => .tuple (elements.map (translated handle))
  | .ghost value => .ghost (translated handle value)
  | .borrow loan current => .borrow loan (translated handle current)
  | value => value

/-- The verdict on one input. -/
inductive Verdict where
  | agree (kind : String)
  | disagree (message : String)
  | inconclusive (reason : String)
  /-- The interpreter cannot derive a result, with its stable code, where the
  term has no outcome: nothing ran, so nothing agrees. -/
  | interpreterError (code : String)

/-- A value's text on one line. -/
def oneLine (text : String) : String :=
  " ".intercalate ((text.splitOn "\n").map (·.trimAscii.toString))

def runtimeOutcomeText : Except LocatedInterpreterError (RuntimeState × LocatedOutcome) → String
  | .ok (_, outcome) => oneLine (reprStr outcome.value)
  | .error error => oneLine (reprStr error.value)

def outcomeText {α : Type} (show_ : α → String) : TreeOutcome α → String
  | .ok value => s!"ok {show_ value}"
  | .abort failure => s!"abort {oneLine (reprStr failure.encode)}"
  | .stuck => "stuck"
  | .bottom => "bottom"
  | .unknown reason => s!"unknown ({reason})"
  | .vacuous σ current prophecy =>
      s!"no execution: current {oneLine (reprStr (σ.encode current))}, \
        prophecy {oneLine (reprStr (σ.encode prophecy))}"

/-- The native arguments as text. -/
def argumentsText (row : NRow) (values : HList row) : String :=
  oneLine (reprStr (HList.encode values))

/-- The loans a result row returns, in order. -/
def returnedLoans (results : Array RuntimeValue) : List Nat :=
  results.toList.flatMap fun entry => (outermostBorrows entry).toList.map (·.1)

/-- The loans whose write-back is what a returned reference holds: a reference
reborrowed and returned, whose prophecy is the caller's as well. -/
def loansThroughResults (results : Array RuntimeValue) (exports : List (Nat × RuntimeValue)) :
    List Nat :=
  exports.filterMap fun (loan, exported) =>
    if resolveReturnedBorrows results exported != exported then some loan else none

/-- The rendering declares a function returning `()` without results. -/
def unitless (shape : ResultShape) (results : Array RuntimeValue) : Array RuntimeValue :=
  if shape matches .none && results == #[.unit] then #[] else results

/-- Whether a native result is the runtime's results, its prophecy that of the
references they return. -/
def sameResult (shape : ResultShape) (value : shape.carrier) (results : Array RuntimeValue) : Bool :=
  let loans := returnedLoans results
  let results := unitless shape results
  shape.lend false value loans == some results && shape.lend true value loans == some results

/-- `toOriginal` and `toUnit` translate struct handles between the unit the
interpreter runs and the unit compiled, which may order its structs differently. -/
def checkInput {original unit : ValidatedUnit} [FunctionCarriers unit] [Skolems unit]
    (executable : ExecutableUnit original) (compiled : CompiledUnit unit) (handle : FunctionHandle)
    (toOriginal toUnit : StructHandle → StructHandle)
    (f : Function unit) (args : HList f.params) : Verdict :=
  let loopFuel := 300
  let tree (values : HList f.params) : Tree f.result.carrier :=
    Function.tree (callTree compiled loopFuel 16) loopFuel f values
  let budget := 2048
  let used := (NRow.lend f.params false args (List.range 16)).map (16 - ·.2.length) |>.getD 0
  let resultText (value : f.result.carrier) : String :=
    match f.result.lend false value (List.range used) with
    | some row => oneLine (reprStr row)
    | none => "()"
  let inputs := argumentsText f.params args
  let loans := List.range used
  match lendArguments f.params args loans with
  | none => .inconclusive "the arguments do not lend"
  | some arguments =>
    let start : RuntimeState := { nextLoan := used }
    let interpreted := Interpreter.run executable 4096 handle
      (arguments.map (translated toOriginal)) start
    let report (term : String) :=
      Verdict.disagree s!"inputs {inputs}: interpreter {runtimeOutcomeText interpreted}, denotation {term}"
    match interpreted with
    | .error error =>
        match error.value with
        | .outOfFuel => .inconclusive "interpreter fuel"
        -- An unchecked operation off its domain: no outcome on both sides.
        | .undefinedOperation _ =>
          match (tree args).solve budget with
          | .stuck => .agree "stuck"
          | .unknown reason => .inconclusive reason
          | outcome => report (outcomeText resultText outcome)
        -- A live settle reaching a place no longer there is the error the
        -- denotation's model of deaths rules out, whatever the term does.
        | failure@(.invalidPlace ..) => report s!"interpreter {failure.code}: {outcomeText resultText ((tree args).solve budget)}"
        | failure =>
          match (tree args).solve budget with
          | .stuck | .bottom => .interpreterError failure.code
          | .unknown reason => .inconclusive reason
          | outcome => report (outcomeText resultText outcome)
    | .ok (_, { value := .threw kind payload, .. }) =>
        match (tree args).solve budget with
        | .abort failure =>
            if failure.encode == (kind, payload) then .agree "abort"
            else report (outcomeText resultText (.abort failure))
        | .unknown reason => .inconclusive reason
        | outcome => report (outcomeText resultText outcome)
    | .ok (exit, { value := .returned results, .. }) =>
        let results := results.map (translated toUnit)
        let exports := (exportsAfter start.pending exit.pending).map fun (loan, value) =>
          (loan, translated toUnit value)
        match withProphecies f.params args loans results exports with
        | none => report "(the written-back values do not decode)"
        | some resolved =>
          let distinct := (returnedLoans results).isEmpty
          match (tree resolved).solve budget distinct with
          | .ok value =>
              if sameResult f.result value results then
                -- Another prophecy of a returned reference is the caller's to
                -- choose; of any other reference it leaves no execution.
                let returned := returnedLoans results
                let through := loansThroughResults results exports
                let wrong := (perturbed f.params resolved loans).find? fun (other, loan) =>
                  match (tree other).solve budget distinct with
                  | .ok value =>
                      !(returned.contains loan || through.contains loan) ||
                        f.result.lend false value returned != some (unitless f.result results)
                  | .vacuous .. => returned.contains loan
                  | .unknown _ => false
                  | _ => true
                match wrong with
                | some (other, _) =>
                    report s!"at the prophecies {argumentsText f.params other}: \
                      {outcomeText resultText ((tree other).solve budget distinct)}"
                | none => .agree "return"
              else report (outcomeText resultText (.ok value))
          | .unknown reason => .inconclusive reason
          | outcome => report (outcomeText resultText outcome)

end Compare

/-! ## The fixtures -/

/-- What a fixture function's inputs came to. -/
structure Tally where
  inputs : Nat := 0
  returns : Nat := 0
  aborts : Nat := 0
  noOutcome : Nat := 0
  undecided : Nat := 0
  interpreterErrors : Nat := 0
  inconclusive : Array String := #[]
  disagreements : Array String := #[]
  /-- The frames its calls at type arguments create, and whether each is coherent. -/
  frames : Array (String × Bool) := #[]
  deriving Inhabited

def Tally.add (tally : Tally) : Verdict → Tally
  | .agree "return" => { tally with inputs := tally.inputs + 1, returns := tally.returns + 1 }
  | .agree "abort" => { tally with inputs := tally.inputs + 1, aborts := tally.aborts + 1 }
  | .agree _ => { tally with inputs := tally.inputs + 1, noOutcome := tally.noOutcome + 1 }
  | .inconclusive reason =>
      { tally with inputs := tally.inputs + 1, undecided := tally.undecided + 1,
                   inconclusive := if tally.inconclusive.contains reason then tally.inconclusive
                     else tally.inconclusive.push reason }
  | .interpreterError code =>
      let reason := s!"interpreter error {code}"
      { tally with inputs := tally.inputs + 1, interpreterErrors := tally.interpreterErrors + 1,
                   inconclusive := if tally.inconclusive.contains reason then tally.inconclusive
                     else tally.inconclusive.push reason }
  | .disagree message =>
      { tally with inputs := tally.inputs + 1, disagreements := tally.disagreements.push message }

/-- A fixture's report. -/
structure FixtureReport where
  name : String
  lines : Array String := #[]
  /-- The function of each disagreement, with the inputs and both outcomes. -/
  disagreements : Array (String × String) := #[]
  inputs : Nat := 0
  returns : Nat := 0
  aborts : Nat := 0
  noOutcome : Nat := 0
  inconclusive : Nat := 0
  interpreterErrors : Nat := 0
  /-- The functions no input decided. -/
  unchecked : Array String := #[]
  /-- Why the rendering lowered back does not prepare for execution, which
  verification requires. -/
  refused : Array String := #[]
  /-- What keeps the check from the fixture's exported unit or its rendering. -/
  unrun : Array String := #[]
  deriving Inhabited

/-- A report with a function's tally added. A function no input ran to a return
or an abort disagrees: nothing was compared. -/
def FixtureReport.add (report : FixtureReport) (name label : String) (tally : Tally) : FixtureReport :=
  let line := s!"{tally.inputs} inputs, {tally.returns} return, {tally.aborts} abort, \
    {tally.noOutcome} none, {tally.interpreterErrors} interpreter errors, \
    inconclusive {tally.inconclusive}"
  let neverRan := if tally.returns + tally.aborts == 0
    then #[(label, s!"no input returns or aborts: {line}")] else #[]
  { report with
    inputs := report.inputs + tally.inputs
    returns := report.returns + tally.returns
    aborts := report.aborts + tally.aborts
    noOutcome := report.noOutcome + tally.noOutcome
    inconclusive := report.inconclusive + tally.undecided
    interpreterErrors := report.interpreterErrors + tally.interpreterErrors
    lines := (report.lines.push s!"{name}::{label}: {line}") ++ tally.frames.map fun (frame, coherent) =>
      s!"  {frame}: coherent {coherent}"
    disagreements := report.disagreements ++ tally.disagreements.map (fun line => (label, line)) ++
      neverRan
    unchecked := if tally.undecided + tally.interpreterErrors == tally.inputs
      then report.unchecked.push label else report.unchecked }

/-- The compiled functions of a unit. -/
def compileAll (unit : ValidatedUnit) : CompiledUnit unit :=
  (List.range unit.namespaces.size).toArray.map fun namespaceIndex =>
    let count := (unit.namespaces[namespaceIndex]?).map (·.functions.size) |>.getD 0
    (List.range count).toArray.map fun index =>
      compileFunction unit { namespaceId := ⟨namespaceIndex⟩, functionId := ⟨index⟩ }

/-- Check one function on its inputs. -/
def checkFunction {original unit : ValidatedUnit} [FunctionCarriers unit] [Skolems unit]
    (executable : ExecutableUnit original) (compiled : CompiledUnit unit) (handle : FunctionHandle)
    (toOriginal toUnit : StructHandle → StructHandle)
    (f : Function unit) (coverage : Coverage) : Tally :=
  let arity := f.params.toList.length
  let cap := match arity with
    | 0 | 1 | 2 => 0
    | 3 => 8
    | 4 => 4
    | _ => 2
  let rows :=
    match (if coverage == .exhaustive then exhaustiveRows f.params else none) with
    | some rows => rows
    | none => argumentRows coverage f.params cap
  rows.foldl (fun tally row => tally.add (checkInput executable compiled handle toOriginal toUnit f row)) {}

/-- `filterMap` over at most four threads, in order. -/
def inParallel {α β : Type} (items : Array α) (work : α → Option β) : Array β :=
  let chunk := (items.size + 3) / 4
  let tasks := (List.range 4).map fun index =>
    Task.spawn fun _ => (items.extract (index * chunk) ((index + 1) * chunk)).filterMap work
  tasks.foldl (fun found task => found ++ task.get) #[]

/-- The frames a function's calls at type arguments create, as `closedGeneric` and
`closedInstance` read them, each with whether it is `Coherent`, which no call proves and the
agreement assumes. A call is followed into its callee: an instance compiled at the type
arguments in the ambient family, a generic compiled at none in the family they induce. -/
def callFrames {unit : ValidatedUnit} [FunctionCarriers unit] :
    Nat → Skolems unit → Array (TypeId × TypeId) → Function unit → Array (String × Bool)
  | 0, _, _, _ => #[]
  | depth + 1, family, outer, f =>
      let calls := f.body.foldCalls (fun handle typeArgs θ found => match θ with
          | some θ => found.push (handle, typeArgs, θ)
          | none => found)
        (fun _ _ found => found) (#[] : Array (FunctionHandle × Array GenericArgument × TypeArgs))
      calls.flatMap fun (handle, typeArgs, θ) =>
        let frame := frameInstantiation unit handle outer typeArgs
        let induced := Skolems.instantiate θ family
        let atInstance := θ.atInstance unit handle
        let here := (s!"{if atInstance then "instance" else "generic"} frame of \
          {VerexSL.Contract.functionKey unit handle} at \
          {VerexSL.Contract.typeArgumentsSpelling θ.1}", @coherentCheck unit _ induced handle frame)
        let inner := if atInstance then
            match compileFunctionAt unit handle (some θ) with
            | .ok callee => callFrames depth family frame callee
            | .error _ => #[]
          else
            match compileFunction unit handle with
            | .ok callee => callFrames depth induced frame callee
            | .error _ => #[]
        #[here] ++ inner

/-- Check the functions of a unit whose compiled denotation is what `functions`
names, each against the interpreter's run of the exported function. -/
def checkFunctions {original : ValidatedUnit} (executable : ExecutableUnit original)
    (unit : ValidatedUnit) (toOriginal toUnit : StructHandle → StructHandle) (coverage : Coverage)
    (functions : Array (String × Nat × Nat)) : Array (String × Tally) :=
  let _ : FunctionCarriers unit := FunctionCarriers.runtime unit
  let _ : Skolems unit := runtimeSkolems unit
  let compiled := compileAll unit
  inParallel functions fun (label, exported, lowered) =>
    match calleeAt compiled { namespaceId := ⟨0⟩, functionId := ⟨lowered⟩ } with
    | .ok f =>
        let tally := checkFunction executable compiled { namespaceId := ⟨0⟩, functionId := ⟨exported⟩ }
          toOriginal toUnit f coverage
        let frames := callFrames 8 (runtimeSkolems unit) #[] f
        some (label, { tally with
          frames
          disagreements := tally.disagreements ++ (frames.filter (!·.2)).map fun (frame, _) =>
            s!"incoherent {frame}" })
    | .error _ => none

/-- The handle in `target` of the struct a handle in `source` names. -/
def structHandle (source target : ValidatedUnit) (handle : StructHandle) : StructHandle :=
  let qualified (unit : ValidatedUnit) (id : NameId) : Option (List String × String) :=
    unit.tables.names[id.index]?.map fun entry => (entry.scope.toList, entry.name)
  let namespaceOf (unit : ValidatedUnit) : Option ValidatedNamespace :=
    unit.namespaces[handle.namespaceId.index]?
  let named : Option (List String × String) :=
    (namespaceOf source).bind fun ns =>
      (ns.structs[handle.structId]?).bind fun declaration => qualified source declaration.name
  let found : Option Nat :=
    (namespaceOf target).bind fun ns =>
      ns.structs.findIdx? fun declaration =>
        named.isSome && qualified target declaration.name == named
  match found with
  | some index => { handle with structId := index }
  | none => handle

/-- Check a fixture: the interpreter runs the exported unit. The denotation is
that of its rendering lowered back, as verification compiles it, or, for a
function whose rendering does not lower or compile, of the exported unit. -/
def checkFixture (environment : Lean.Environment) (name : String)
    (result : VerexIR.Rust.Driver.ImportResult) (coverage : Coverage) : IO FixtureReport := do
  let original := result.unit
  let executable ← match prepareExecution #[VerexIR.Rust.semantics] original with
    | .ok executable => pure executable
    | .error diagnostics =>
        let line := s!"{name}: not executable: {diagnostics.map (·.code)}"
        return { name, lines := #[line], unrun := #[line] }
  let some ns := original.namespaces[0]? | throw <| IO.userError s!"{name} has no namespace"
  let qualified (tables : Tables) (id : NameId) : Option (List String × String) :=
    tables.names[id.index]?.map fun entry => (entry.scope.toList, entry.name)
  -- A generic function has no input rows; a call runs its body at the instantiation.
  let keyed := (ns.functions.zipIdx.filter fun (declaration, _) =>
      !declaration.signature.generics.any (·.kind == .typeArg)).filterMap fun (declaration, index) =>
    (qualified original.tables declaration.name).map fun key =>
      (String.intercalate "::" (key.1 ++ [key.2]), key, index)
  let lowered := (VerexSL.Print.reimportUnit environment original).toOption
  let refused : Array String := match lowered with
    | some unit => match prepareExecution #[VerexIR.Rust.semantics] unit with
        | .ok _ => #[]
        | .error diagnostics => diagnostics.map (·.message)
    | none => #[]
  let loweredCompiled : Array (Except String Unit) := match lowered with
    | some unit => match unit.namespaces[0]? with
        | some ns => (List.range ns.functions.size).toArray.map fun index =>
            (compileFunction unit { namespaceId := ⟨0⟩, functionId := ⟨index⟩ }).map fun _ => ()
        | none => #[]
    | none => #[]
  let loweredIndex (key : List String × String) : Option Nat := do
    let unit ← lowered
    let ns ← unit.namespaces[0]?
    let index ← ns.functions.findIdx? fun other => qualified unit.tables other.name == some key
    match (loweredCompiled[index]? : Option (Except String Unit)) with
    | some (.ok ()) => some index
    | _ => none
  let viaLowered := keyed.filterMap fun (label, key, index) =>
    (loweredIndex key).map fun at' => (label, index, at')
  let viaExported := keyed.filterMap fun (label, key, index) =>
    if (loweredIndex key).isSome then none else some (label, index, index)
  let tallies := (match lowered with
      | some unit => checkFunctions executable unit (structHandle unit original)
          (structHandle original unit) coverage viaLowered
      | none => #[]) ++
    (checkFunctions executable original id id coverage viaExported).map fun (label, tally) =>
      (label ++ " (exported)", tally)
  let mut report : FixtureReport := { name }
  for (label, tally) in tallies do
    report := report.add name label tally
  for (label, _, _) in keyed do
    unless tallies.any (fun (other, _) => other == label || other == label ++ " (exported)") do
      report := { report with
        lines := report.lines.push s!"{name}::{label}: no denotation"
        disagreements := report.disagreements.push (label, "no denotation")
        unchecked := report.unchecked.push label }
  for message in refused do
    report := { report with
      lines := report.lines.push s!"{name}: the rendering does not execute: {message}"
      refused := report.refused.push message }
  if let .error error := VerexSL.Print.reimportUnit environment original then
    let line := s!"{name}: the rendering does not lower: {error.message}"
    report := { report with lines := report.lines.push line, unrun := report.unrun.push line }
  pure report

/-- Check a unit authored in VerexSL, which nothing exports: the interpreter runs
the unit whose denotation is compiled. -/
def checkAuthored (name : String) (unit : ValidatedUnit) (coverage : Coverage) : IO FixtureReport := do
  let executable ← match prepareExecution #[VerexIR.Rust.semantics] unit with
    | .ok executable => pure executable
    | .error diagnostics => throw <| IO.userError s!"{name}: {repr diagnostics}"
  let some ns := unit.namespaces[0]? | throw <| IO.userError s!"{name} has no namespace"
  let functions := ns.functions.zipIdx.map fun (declaration, index) =>
    let label := (unit.tables.names[declaration.name.index]?).map (·.name) |>.getD "?"
    (label, index, index)
  let mut report : FixtureReport := { name }
  for (label, tally) in checkFunctions executable unit id id coverage functions do
    report := report.add name label tally
  pure report

/-- The fixtures of a directory, each exported through rustc. -/
def checkDirectory (environment : Lean.Environment) (directory : System.FilePath)
    (coverage : String → Coverage) : IO (Array FixtureReport) := do
  let entries ← directory.readDir
  let sources := entries.filter (·.path.extension == some "rs") |>.map (·.path)
  let sources := sources.qsort (·.toString < ·.toString)
  let mut reports := #[]
  for source in sources do
    let name := source.fileStem.getD source.toString
    let result ← VerexIR.Rust.Driver.importRustFile {
      source, output := ".lake/verex-rust/differential" / s!"{name}.raw.json" }
    reports := reports.push (← checkFixture environment name result (coverage name))
  pure reports

end VerexIR.Rust.Tests.Differential

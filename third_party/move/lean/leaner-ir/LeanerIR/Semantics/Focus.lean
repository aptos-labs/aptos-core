-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Semantics.Operations

/-!
# Focused nominal paths

A storage bracket borrows a resource and reborrows one field of it along a
nominal path.  The path is a list of steps, each naming the struct it
descends into and the focused field's siblings; the resource with a value
at the focus is `focusValue`.  Every place operation the bracket performs
— reading the focus, holing it, writing it back, finding the hole and
filling it, retiring the borrow that rested on it — is stated here once
over the path, so the bracket laws are path-generic: a flat resource, a
nested one, and a struct with siblings around the focus are one shape.

Siblings are `Plain`, loan-free, as every typed erasure is.  The walkers
that look for a loan's hole or its borrow pass over a plain value without
finding anything, which is what lets a walk through a focused resource be
read off its focus alone.
-/

namespace LeanerIR.SemanticOperations

/-! ## Loan-free values -/

/-- A loan-free value: no borrow and no hole anywhere inside. -/
inductive Plain : RuntimeValue → Prop
  | unit : Plain .unit
  | bool (value : Bool) : Plain (.bool value)
  | character (value : Nat) : Plain (.character value)
  | integer (value : Int) : Plain (.integer value)
  | address (value : String) : Plain (.address value)
  | signer (value : String) : Plain (.signer value)
  | string (value : String) : Plain (.string value)
  | bytes (value : Array UInt8) : Plain (.bytes value)
  | vector (elements : Array RuntimeValue)
      (plain : ∀ element ∈ elements, Plain element) : Plain (.vector elements)
  | tuple (elements : Array RuntimeValue)
      (plain : ∀ element ∈ elements, Plain element) : Plain (.tuple elements)
  | nominal (source : StructHandle) (variant : Option String)
      (fields : Array RuntimeValue) (plain : ∀ field ∈ fields, Plain field) :
      Plain (.nominal source variant fields)
  | closure (function : FunctionHandle) (captures : Array RuntimeValue)
      (plain : ∀ capture ∈ captures, Plain capture) :
      Plain (.closure function captures)

/-- A nominal value over a literal field row is plain when its fields are:
the form a generated erasure has. -/
theorem Plain.ofFields (source : StructHandle) (variant : Option String)
    (fields : List RuntimeValue) (plain : ∀ field ∈ fields, Plain field) :
    Plain (.nominal source variant fields.toArray) :=
  .nominal source variant fields.toArray (by simpa using plain)

/-- A matcher that answers only at a borrow or at a hole. -/
def LoanMatcher {α : Type} (f : RuntimeValue → Option α) : Prop :=
  ∀ value, (∀ loan current, value ≠ .borrow loan current) →
    (∀ loan, value ≠ .loanHole loan) → f value = none

theorem LoanMatcher.holeFill? (loan : Nat) (replacement : RuntimeValue) :
    LoanMatcher (holeFill? loan replacement) := by
  intro value notBorrow notHole
  cases value <;> first | rfl | exact absurd rfl (notHole _)

theorem LoanMatcher.holeMark? (loan : Nat) : LoanMatcher (holeMark? loan) := by
  intro value notBorrow notHole
  cases value <;> first | rfl | exact absurd rfl (notHole _)

theorem LoanMatcher.anyHole? : LoanMatcher anyHole? := by
  intro value notBorrow notHole
  cases value <;> first | rfl | exact absurd rfl (notHole _)

theorem LoanMatcher.borrowCurrent? (loan : Nat) :
    LoanMatcher (borrowCurrent? loan) := by
  intro value notBorrow notHole
  cases value <;> first | rfl | exact absurd rfl (notBorrow _ _)

theorem LoanMatcher.borrowRewrite? (loan : Nat) (replacement : RuntimeValue) :
    LoanMatcher (borrowRewrite? loan replacement) := by
  intro value notBorrow notHole
  cases value <;> first | rfl | exact absurd rfl (notBorrow _ _)

theorem LoanMatcher.borrowClear? (loan : Nat) :
    LoanMatcher (borrowClear? loan) := by
  intro value notBorrow notHole
  cases value <;> first | rfl | exact absurd rfl (notBorrow _ _)

/-- A loan matcher answers nothing at a plain node. -/
theorem LoanMatcher.plain {α : Type} {f : RuntimeValue → Option α}
    (matcher : LoanMatcher f) {value : RuntimeValue} (plain : Plain value) :
    f value = none := by
  cases plain <;> exact matcher _ (by intro _ _ h; cases h) (by intro _ h; cases h)

/-- A loan matcher answers nothing at a nominal node, plain or not. -/
theorem LoanMatcher.nominal {α : Type} {f : RuntimeValue → Option α}
    (matcher : LoanMatcher f) (source : StructHandle) (variant : Option String)
    (fields : Array RuntimeValue) : f (.nominal source variant fields) = none :=
  matcher _ (by intro _ _ h; cases h) (by intro _ h; cases h)

theorem findFirstList_eq_none {α : Type} {f : RuntimeValue → Option α}
    {elements : List RuntimeValue}
    (absent : ∀ element ∈ elements, findFirst f element = none) :
    findFirstList f elements = none := by
  induction elements with
  | nil => rw [findFirstList.eq_def]
  | cons element rest ih =>
      rw [findFirstList.eq_def]
      dsimp only
      rw [absent element (by simp)]
      exact ih fun x mem => absent x (by simp [mem])

/-- A loan matcher finds nothing in a plain value. -/
theorem findFirst_eq_none_of_plain {α : Type} {f : RuntimeValue → Option α}
    (matcher : LoanMatcher f) {value : RuntimeValue} (plain : Plain value) :
    findFirst f value = none := by
  induction plain with
  | unit | bool | character | integer | address | signer | string | bytes =>
      rw [findFirst.eq_def, matcher.plain (by constructor)]
  | vector elements plain ih =>
      rw [findFirst.eq_def, matcher.plain (Plain.vector elements plain)]
      dsimp only
      exact findFirstList_eq_none fun x mem => ih x (by simpa using mem)
  | tuple elements plain ih =>
      rw [findFirst.eq_def, matcher.plain (Plain.tuple elements plain)]
      dsimp only
      exact findFirstList_eq_none fun x mem => ih x (by simpa using mem)
  | nominal source variant fields plain ih =>
      rw [findFirst.eq_def, matcher.nominal]
      dsimp only
      exact findFirstList_eq_none fun x mem => ih x (by simpa using mem)
  | closure function captures plain ih =>
      rw [findFirst.eq_def, matcher.plain (Plain.closure function captures plain)]
      dsimp only
      exact findFirstList_eq_none fun x mem => ih x (by simpa using mem)

theorem rewriteFirstList_eq_none {f : RuntimeValue → Option RuntimeValue}
    {elements : List RuntimeValue}
    (absent : ∀ element ∈ elements, rewriteFirst f element = none) :
    rewriteFirstList f elements = none := by
  induction elements with
  | nil => rw [rewriteFirstList.eq_def]
  | cons element rest ih =>
      rw [rewriteFirstList.eq_def]
      dsimp only
      rw [absent element (by simp), ih fun x mem => absent x (by simp [mem])]
      rfl

/-- A loan matcher rewrites nothing in a plain value. -/
theorem rewriteFirst_eq_none_of_plain {f : RuntimeValue → Option RuntimeValue}
    (matcher : LoanMatcher f) {value : RuntimeValue} (plain : Plain value) :
    rewriteFirst f value = none := by
  induction plain with
  | unit | bool | character | integer | address | signer | string | bytes =>
      rw [rewriteFirst.eq_def, matcher.plain (by constructor)]
  | vector elements plain ih =>
      rw [rewriteFirst.eq_def, matcher.plain (Plain.vector elements plain)]
      dsimp only
      rw [rewriteFirstList_eq_none fun x mem => ih x (by simpa using mem)]
      rfl
  | tuple elements plain ih =>
      rw [rewriteFirst.eq_def, matcher.plain (Plain.tuple elements plain)]
      dsimp only
      rw [rewriteFirstList_eq_none fun x mem => ih x (by simpa using mem)]
      rfl
  | nominal source variant fields plain ih =>
      rw [rewriteFirst.eq_def, matcher.nominal]
      dsimp only
      rw [rewriteFirstList_eq_none fun x mem => ih x (by simpa using mem)]
      rfl
  | closure function captures plain ih =>
      rw [rewriteFirst.eq_def, matcher.plain (Plain.closure function captures plain)]
      dsimp only
      rw [rewriteFirstList_eq_none fun x mem => ih x (by simpa using mem)]
      rfl

/-- Searching a plain prefix finds nothing: the search continues past it. -/
theorem findFirstList_append_of_none {α : Type} {f : RuntimeValue → Option α}
    {prefix_ : List RuntimeValue}
    (absent : ∀ element ∈ prefix_, findFirst f element = none)
    (rest : List RuntimeValue) :
    findFirstList f (prefix_ ++ rest) = findFirstList f rest := by
  induction prefix_ with
  | nil => rfl
  | cons element more ih =>
      rw [List.cons_append, findFirstList.eq_def]
      dsimp only
      rw [absent element (by simp)]
      exact ih fun x mem => absent x (by simp [mem])

/-- Rewriting past a plain prefix keeps the prefix. -/
theorem rewriteFirstList_append_of_none {f : RuntimeValue → Option RuntimeValue}
    {prefix_ : List RuntimeValue}
    (absent : ∀ element ∈ prefix_, rewriteFirst f element = none)
    (rest : List RuntimeValue) :
    rewriteFirstList f (prefix_ ++ rest) =
      (rewriteFirstList f rest).map (prefix_ ++ ·) := by
  induction prefix_ with
  | nil => simp
  | cons element more ih =>
      rw [List.cons_append, rewriteFirstList.eq_def]
      dsimp only
      rw [absent element (by simp), ih fun x mem => absent x (by simp [mem])]
      cases rewriteFirstList f rest <;> simp

/-! ## One node at a time

A proof over a literal frame walks each local with these instead of
unfolding the walkers: an unfolding of `findFirst` would also open the
walk through a symbolic focused resource, where the path lemmas below are
the only way through. -/

section NodeAlgebra
variable {α : Type} {f : RuntimeValue → Option α} {g : RuntimeValue → Option RuntimeValue}

theorem findFirst_of_some {value : RuntimeValue} {found : α}
    (matched : f value = some found) : findFirst f value = some found := by
  rw [findFirst.eq_def, matched]

theorem findFirst_borrow (loan : Nat) (current : RuntimeValue) :
    findFirst f (.borrow loan current) =
      match f (.borrow loan current) with
      | some found => some found
      | none => findFirst f current := by
  rw [findFirst.eq_def]; rfl

theorem findFirst_borrow_of_none {loan : Nat} {current : RuntimeValue}
    (passed : f (.borrow loan current) = none) :
    findFirst f (.borrow loan current) = findFirst f current := by
  rw [findFirst_borrow, passed]

theorem findFirst_nominal (source : StructHandle) (variant : Option String)
    (fields : Array RuntimeValue) :
    findFirst f (.nominal source variant fields) =
      match f (.nominal source variant fields) with
      | some found => some found
      | none => findFirstList f fields.toList := by
  rw [findFirst.eq_def]; rfl

theorem findFirst_unit : findFirst f .unit = f .unit := by
  rw [findFirst.eq_def]; cases f .unit <;> rfl
theorem findFirst_bool (value : Bool) : findFirst f (.bool value) = f (.bool value) := by
  rw [findFirst.eq_def]; cases f (.bool value) <;> rfl
theorem findFirst_character (value : Nat) :
    findFirst f (.character value) = f (.character value) := by
  rw [findFirst.eq_def]; cases f (.character value) <;> rfl
theorem findFirst_integer (value : Int) :
    findFirst f (.integer value) = f (.integer value) := by
  rw [findFirst.eq_def]; cases f (.integer value) <;> rfl
theorem findFirst_address (value : String) :
    findFirst f (.address value) = f (.address value) := by
  rw [findFirst.eq_def]; cases f (.address value) <;> rfl
theorem findFirst_signer (value : String) :
    findFirst f (.signer value) = f (.signer value) := by
  rw [findFirst.eq_def]; cases f (.signer value) <;> rfl
theorem findFirst_string (value : String) :
    findFirst f (.string value) = f (.string value) := by
  rw [findFirst.eq_def]; cases f (.string value) <;> rfl
theorem findFirst_bytes (value : Array UInt8) :
    findFirst f (.bytes value) = f (.bytes value) := by
  rw [findFirst.eq_def]; cases f (.bytes value) <;> rfl
theorem findFirst_loanHole (loan : Nat) :
    findFirst f (.loanHole loan) = f (.loanHole loan) := by
  rw [findFirst.eq_def]; cases f (.loanHole loan) <;> rfl

theorem findFirstList_nil : findFirstList f [] = none := by rw [findFirstList.eq_def]
theorem findFirstList_cons (element : RuntimeValue) (rest : List RuntimeValue) :
    findFirstList f (element :: rest) =
      match findFirst f element with
      | some found => some found
      | none => findFirstList f rest := by
  rw [findFirstList.eq_def]; rfl

theorem rewriteFirst_of_some {value rewritten : RuntimeValue}
    (matched : g value = some rewritten) : rewriteFirst g value = some rewritten := by
  rw [rewriteFirst.eq_def, matched]

theorem rewriteFirst_borrow (loan : Nat) (current : RuntimeValue) :
    rewriteFirst g (.borrow loan current) =
      match g (.borrow loan current) with
      | some rewritten => some rewritten
      | none => (rewriteFirst g current).map (.borrow loan) := by
  rw [rewriteFirst.eq_def]; rfl

theorem rewriteFirst_nominal (source : StructHandle) (variant : Option String)
    (fields : Array RuntimeValue) :
    rewriteFirst g (.nominal source variant fields) =
      match g (.nominal source variant fields) with
      | some rewritten => some rewritten
      | none => (rewriteFirstList g fields.toList).map fun rewritten =>
          .nominal source variant rewritten.toArray := by
  rw [rewriteFirst.eq_def]; rfl

theorem rewriteFirst_unit : rewriteFirst g .unit = g .unit := by
  rw [rewriteFirst.eq_def]; cases g .unit <;> rfl
theorem rewriteFirst_bool (value : Bool) :
    rewriteFirst g (.bool value) = g (.bool value) := by
  rw [rewriteFirst.eq_def]; cases g (.bool value) <;> rfl
theorem rewriteFirst_character (value : Nat) :
    rewriteFirst g (.character value) = g (.character value) := by
  rw [rewriteFirst.eq_def]; cases g (.character value) <;> rfl
theorem rewriteFirst_integer (value : Int) :
    rewriteFirst g (.integer value) = g (.integer value) := by
  rw [rewriteFirst.eq_def]; cases g (.integer value) <;> rfl
theorem rewriteFirst_address (value : String) :
    rewriteFirst g (.address value) = g (.address value) := by
  rw [rewriteFirst.eq_def]; cases g (.address value) <;> rfl
theorem rewriteFirst_signer (value : String) :
    rewriteFirst g (.signer value) = g (.signer value) := by
  rw [rewriteFirst.eq_def]; cases g (.signer value) <;> rfl
theorem rewriteFirst_string (value : String) :
    rewriteFirst g (.string value) = g (.string value) := by
  rw [rewriteFirst.eq_def]; cases g (.string value) <;> rfl
theorem rewriteFirst_bytes (value : Array UInt8) :
    rewriteFirst g (.bytes value) = g (.bytes value) := by
  rw [rewriteFirst.eq_def]; cases g (.bytes value) <;> rfl
theorem rewriteFirst_loanHole (loan : Nat) :
    rewriteFirst g (.loanHole loan) = g (.loanHole loan) := by
  rw [rewriteFirst.eq_def]; cases g (.loanHole loan) <;> rfl

theorem rewriteFirstList_nil : rewriteFirstList g [] = none := by
  rw [rewriteFirstList.eq_def]
theorem rewriteFirstList_cons (element : RuntimeValue) (rest : List RuntimeValue) :
    rewriteFirstList g (element :: rest) =
      match rewriteFirst g element with
      | some rewritten => some (rewritten :: rest)
      | none => (rewriteFirstList g rest).map (element :: ·) := by
  rw [rewriteFirstList.eq_def]; rfl

end NodeAlgebra

/-! ## Focus steps -/

/-- One step of a focused path: the struct descended into, and the focused
field's siblings on either side. -/
structure FocusStep where
  source : StructHandle
  before : Array RuntimeValue
  after : Array RuntimeValue

namespace FocusStep

/-- The focused field's position. -/
def index (step : FocusStep) : Nat := step.before.size

/-- The nominal field step a reborrow along this step carries. -/
def fieldStep (step : FocusStep) : NominalFieldStep := ⟨step.source, none, step.index⟩

/-- The struct with `focus` at the focused field. -/
def fill (step : FocusStep) (focus : RuntimeValue) : RuntimeValue :=
  .nominal step.source none (step.before.push focus ++ step.after)

/-- The siblings are loan-free. -/
def Plain (step : FocusStep) : Prop :=
  (∀ sibling ∈ step.before, SemanticOperations.Plain sibling) ∧
    (∀ sibling ∈ step.after, SemanticOperations.Plain sibling)

@[simp] theorem getElem?_fill (step : FocusStep) (focus : RuntimeValue) :
    (step.before.push focus ++ step.after)[step.index]? = some focus := by
  simp [index, Array.getElem?_append]

@[simp] theorem setIfInBounds_fill (step : FocusStep) (focus replacement : RuntimeValue) :
    (step.before.push focus ++ step.after).setIfInBounds step.index replacement =
      step.before.push replacement ++ step.after := by
  apply Array.ext'
  simp [index]

theorem set!_fill (step : FocusStep) (focus replacement : RuntimeValue) :
    (step.before.push focus ++ step.after).set! step.index replacement =
      step.before.push replacement ++ step.after := by
  simp [Array.set!]

theorem toList_fill (step : FocusStep) (focus : RuntimeValue) :
    (step.before.push focus ++ step.after).toList =
      step.before.toList ++ focus :: step.after.toList := by
  simp

end FocusStep

/-- The resource with `leaf` at the focus of `steps`. -/
def focusValue : List FocusStep → RuntimeValue → RuntimeValue
  | [], leaf => leaf
  | step :: rest, leaf => step.fill (focusValue rest leaf)

/-- The nominal field steps a reborrow along the path carries. -/
def focusFields (steps : List FocusStep) : List NominalFieldStep :=
  steps.map FocusStep.fieldStep

/-- The projections the path resolves to. -/
def focusProjections (steps : List FocusStep) : Array RuntimeProjection :=
  (steps.map fun step => RuntimeProjection.field step.index).toArray

/-- Every step's siblings are loan-free. -/
def PlainSteps (steps : List FocusStep) : Prop := ∀ step ∈ steps, step.Plain

theorem PlainSteps.head {step : FocusStep} {rest : List FocusStep}
    (plain : PlainSteps (step :: rest)) : step.Plain := plain step (by simp)

theorem PlainSteps.tail {step : FocusStep} {rest : List FocusStep}
    (plain : PlainSteps (step :: rest)) : PlainSteps rest :=
  fun s mem => plain s (by simp [mem])

@[simp] theorem focusValue_nil (leaf : RuntimeValue) : focusValue [] leaf = leaf := rfl

@[simp] theorem focusValue_cons (step : FocusStep) (rest : List FocusStep)
    (leaf : RuntimeValue) :
    focusValue (step :: rest) leaf = step.fill (focusValue rest leaf) := rfl

@[simp] theorem focusFields_nil : focusFields [] = [] := rfl

@[simp] theorem focusFields_cons (step : FocusStep) (rest : List FocusStep) :
    focusFields (step :: rest) = step.fieldStep :: focusFields rest := rfl

@[simp] theorem focusProjections_nil : focusProjections [] = #[] := rfl

@[simp] theorem focusProjections_cons (step : FocusStep) (rest : List FocusStep) :
    focusProjections (step :: rest) =
      #[RuntimeProjection.field step.index] ++ focusProjections rest := by
  simp [focusProjections]

/-! ## Reading and writing along the path -/

theorem readProjections?_focusValue (steps : List FocusStep) (leaf : RuntimeValue) :
    readProjections? (focusValue steps leaf) (focusProjections steps).toList =
      some leaf := by
  induction steps with
  | nil => rfl
  | cons step rest ih =>
      simp [FocusStep.fill, readProjections?, ih]

theorem writeProjections?_focusValue (steps : List FocusStep)
    (leaf replacement : RuntimeValue) :
    writeProjections? (focusValue steps leaf) (focusProjections steps).toList
        replacement =
      some (focusValue steps replacement) := by
  induction steps with
  | nil => rfl
  | cons step rest ih =>
      simp [FocusStep.fill, writeProjections?, ih]

theorem readProjections?_append (value : RuntimeValue)
    (first second : List RuntimeProjection) :
    readProjections? value (first ++ second) =
      (readProjections? value first).bind (readProjections? · second) := by
  induction first generalizing value with
  | nil => simp [readProjections?]
  | cons projection rest ih =>
      cases projection <;> cases value <;> simp [readProjections?, ih, Option.bind_assoc]
      split <;> (try split) <;> simp

/-- Resolving the path's field steps from a place that reads the focused
resource appends the path's projections. -/
theorem resolveNominalFieldSteps?_focus (frame : RuntimeFrame)
    (state : RuntimeState) (steps : List FocusStep) (leaf : RuntimeValue)
    (place : RuntimePlace)
    (read : readRuntimePlace? frame state place = some (focusValue steps leaf)) :
    resolveNominalFieldSteps? frame state (focusFields steps) place =
      some { place with projections := place.projections ++ focusProjections steps } := by
  induction steps generalizing place with
  | nil => simp [resolveNominalFieldSteps?]
  | cons step rest ih =>
      rw [focusFields_cons, resolveNominalFieldSteps?, read]
      simp only [bind, Option.bind, focusValue_cons, FocusStep.fill, FocusStep.fieldStep,
        bne_self_eq_false, Bool.false_or, Bool.false_eq_true, ↓reduceIte]
      rw [ih]
      · simp only [focusProjections_cons, Array.push_eq_append, Array.append_assoc]
      · simp only [readRuntimePlace?] at read ⊢
        cases lookup : readRoot? frame state place.root with
        | none => simp [lookup] at read
        | some root =>
            simp only [lookup, bind, Option.bind] at read ⊢
            rw [Array.toList_push, readProjections?_append, read]
            simp [readProjections?, FocusStep.fill]

/-! ## Walking through the path -/

/-- A loan matcher's search through a focused resource is its search of the
focus. -/
theorem findFirst_focusValue {α : Type} {f : RuntimeValue → Option α}
    (matcher : LoanMatcher f) {steps : List FocusStep} (plain : PlainSteps steps)
    (leaf : RuntimeValue) :
    findFirst f (focusValue steps leaf) = findFirst f leaf := by
  induction steps with
  | nil => rfl
  | cons step rest ih =>
      obtain ⟨before, after⟩ := plain.head
      rw [focusValue_cons, FocusStep.fill, findFirst.eq_def, matcher.nominal]
      dsimp only
      rw [FocusStep.toList_fill,
        findFirstList_append_of_none fun x mem =>
          findFirst_eq_none_of_plain matcher (before x (by simpa using mem)),
        findFirstList.eq_def]
      dsimp only
      rw [ih plain.tail,
        findFirstList_eq_none fun x mem =>
          findFirst_eq_none_of_plain matcher (after x (by simpa using mem))]
      cases findFirst f leaf <;> rfl

/-- A loan matcher's rewrite through a focused resource is its rewrite of
the focus, put back at the focus. -/
theorem rewriteFirst_focusValue {f : RuntimeValue → Option RuntimeValue}
    (matcher : LoanMatcher f) {steps : List FocusStep} (plain : PlainSteps steps)
    (leaf : RuntimeValue) :
    rewriteFirst f (focusValue steps leaf) =
      (rewriteFirst f leaf).map (focusValue steps) := by
  induction steps with
  | nil => cases h : rewriteFirst f leaf <;> simp [h]
  | cons step rest ih =>
      obtain ⟨before, after⟩ := plain.head
      rw [focusValue_cons, FocusStep.fill, rewriteFirst.eq_def, matcher.nominal]
      dsimp only
      rw [FocusStep.toList_fill,
        rewriteFirstList_append_of_none fun x mem =>
          rewriteFirst_eq_none_of_plain matcher (before x (by simpa using mem)),
        rewriteFirstList.eq_def]
      dsimp only
      rw [ih plain.tail,
        rewriteFirstList_eq_none fun x mem =>
          rewriteFirst_eq_none_of_plain matcher (after x (by simpa using mem))]
      cases rewriteFirst f leaf with
      | none => rfl
      | some rewritten => simp [FocusStep.fill]

theorem holeWithin_focusValue (loan : Nat) {steps : List FocusStep}
    (plain : PlainSteps steps) (leaf : RuntimeValue) :
    holeWithin loan (focusValue steps leaf) = holeWithin loan leaf := by
  simp [holeWithin, findFirst_focusValue (LoanMatcher.holeMark? loan) plain]

theorem fillHole?_focusValue (loan : Nat) (replacement : RuntimeValue)
    {steps : List FocusStep} (plain : PlainSteps steps) (leaf : RuntimeValue) :
    fillHole? loan replacement (focusValue steps leaf) =
      (fillHole? loan replacement leaf).map (focusValue steps) := by
  simp [fillHole?, rewriteFirst_focusValue (LoanMatcher.holeFill? loan replacement) plain]

theorem transferredLoan?_focusValue {steps : List FocusStep}
    (plain : PlainSteps steps) (leaf : RuntimeValue) :
    transferredLoan? (focusValue steps leaf) = transferredLoan? leaf := by
  simp [transferredLoan?, findFirst_focusValue LoanMatcher.anyHole? plain]

theorem findFirst_borrowCurrent?_focusValue (loan : Nat) {steps : List FocusStep}
    (plain : PlainSteps steps) (leaf : RuntimeValue) :
    findFirst (borrowCurrent? loan) (focusValue steps leaf) =
      findFirst (borrowCurrent? loan) leaf :=
  findFirst_focusValue (LoanMatcher.borrowCurrent? loan) plain leaf

theorem rewriteFirst_borrowRewrite?_focusValue (loan : Nat)
    (replacement : RuntimeValue) {steps : List FocusStep} (plain : PlainSteps steps)
    (leaf : RuntimeValue) :
    rewriteFirst (borrowRewrite? loan replacement) (focusValue steps leaf) =
      (rewriteFirst (borrowRewrite? loan replacement) leaf).map (focusValue steps) :=
  rewriteFirst_focusValue (LoanMatcher.borrowRewrite? loan replacement) plain leaf

theorem rewriteFirst_borrowClear?_focusValue (loan : Nat) {steps : List FocusStep}
    (plain : PlainSteps steps) (leaf : RuntimeValue) :
    rewriteFirst (borrowClear? loan) (focusValue steps leaf) =
      (rewriteFirst (borrowClear? loan) leaf).map (focusValue steps) :=
  rewriteFirst_focusValue (LoanMatcher.borrowClear? loan) plain leaf

theorem findFirst_holeMark?_focusValue (loan : Nat) {steps : List FocusStep}
    (plain : PlainSteps steps) (leaf : RuntimeValue) :
    findFirst (holeMark? loan) (focusValue steps leaf) =
      findFirst (holeMark? loan) leaf :=
  findFirst_focusValue (LoanMatcher.holeMark? loan) plain leaf

theorem findFirst_anyHole?_focusValue {steps : List FocusStep}
    (plain : PlainSteps steps) (leaf : RuntimeValue) :
    findFirst anyHole? (focusValue steps leaf) = findFirst anyHole? leaf :=
  findFirst_focusValue LoanMatcher.anyHole? plain leaf

theorem rewriteFirst_holeFill?_focusValue (loan : Nat) (replacement : RuntimeValue)
    {steps : List FocusStep} (plain : PlainSteps steps) (leaf : RuntimeValue) :
    rewriteFirst (holeFill? loan replacement) (focusValue steps leaf) =
      (rewriteFirst (holeFill? loan replacement) leaf).map (focusValue steps) :=
  rewriteFirst_focusValue (LoanMatcher.holeFill? loan replacement) plain leaf

/-- Every walk a bracket performs, through a symbolic focused resource:
the rewrite set a bracket proof adds to the node algebra. -/
theorem focus_walks {steps : List FocusStep} (plain : PlainSteps steps) :
    (∀ loan leaf, findFirst (holeMark? loan) (focusValue steps leaf) =
        findFirst (holeMark? loan) leaf) ∧
      (∀ leaf, findFirst anyHole? (focusValue steps leaf) = findFirst anyHole? leaf) ∧
      (∀ loan leaf, findFirst (borrowCurrent? loan) (focusValue steps leaf) =
        findFirst (borrowCurrent? loan) leaf) ∧
      (∀ loan replacement leaf,
        rewriteFirst (holeFill? loan replacement) (focusValue steps leaf) =
          (rewriteFirst (holeFill? loan replacement) leaf).map (focusValue steps)) ∧
      (∀ loan replacement leaf,
        rewriteFirst (borrowRewrite? loan replacement) (focusValue steps leaf) =
          (rewriteFirst (borrowRewrite? loan replacement) leaf).map (focusValue steps)) ∧
      (∀ loan leaf, rewriteFirst (borrowClear? loan) (focusValue steps leaf) =
        (rewriteFirst (borrowClear? loan) leaf).map (focusValue steps)) :=
  ⟨fun loan leaf => findFirst_holeMark?_focusValue loan plain leaf,
    fun leaf => findFirst_anyHole?_focusValue plain leaf,
    fun loan leaf => findFirst_borrowCurrent?_focusValue loan plain leaf,
    fun loan replacement leaf => rewriteFirst_holeFill?_focusValue loan replacement plain leaf,
    fun loan replacement leaf =>
      rewriteFirst_borrowRewrite?_focusValue loan replacement plain leaf,
    fun loan leaf => rewriteFirst_borrowClear?_focusValue loan plain leaf⟩

/-- The callee's prophecy resolution fills the hole at the focus from the
returned borrow's current. -/
theorem resolveReturnedBorrows_returnedBorrow_focus (loan : Nat) (value : Int)
    (steps : List FocusStep) (plainSteps : PlainSteps steps) :
    resolveReturnedBorrows #[.borrow loan (.integer value)]
        (focusValue steps (.loanHole loan)) =
      focusValue steps (.integer value) := by
  obtain ⟨-, -, -, holeFill, -, -⟩ := focus_walks plainSteps
  simp [resolveReturnedBorrows, outermostBorrows, collectPruned, borrowEntry?,
    fillHole?, holeFill, rewriteFirst_loanHole, holeFill?]

end LeanerIR.SemanticOperations

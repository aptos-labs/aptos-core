-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Denote.Types

/-!
# Typed terms and their denotation

A `Term ρ Γ τ` is the shallow, intrinsically typed form of a validated LIR
body over locals `Γ` in a function returning `ρ`: what the compiler
(`Denote.Compile`) produces and what verification reasons about.  Its
denotation is a `Spec` over a `Flow`: the native value of `τ` with the
locals after evaluation, or the control a body transfers — a return, a
break, a continue — with the locals at that point.  A let-bound local is a
Lean binder in every goal and a slot write is a literal update of a
literal row.  Failures are the `Spec`'s own.

The term language carries exactly the constructs whose agreement with the
big-step semantics is stated in `Denote.Agreement`; a construct outside it
is one the compiler rejects, never one it approximates.
-/

namespace LeanerIR.Proofs.Denote

variable [Skolems]

/-- The outcome of a body: a value, or a control transfer.  `break_` and
`continue_` count the loops they still have to leave, as the runtime's
`Control` does. -/
inductive Flow (ρ : ResultShape) (Γ : NRow) (α : Type) : Type where
  | value (value : α) (env : HEnv Γ)
  | return_ (result : ρ.carrier) (env : HEnv Γ)
  | break_ (nest : Nat) (env : HEnv Γ)
  | continue_ (nest : Nat) (env : HEnv Γ)

/-- Sequence a value-producing computation into a continuation; control
transfers pass through. -/
def Flow.bind {ρ : ResultShape} {Γ : NRow} {α β : Type}
    (action : Comp (Flow ρ Γ α)) (next : α → HEnv Γ → Comp (Flow ρ Γ β)) :
    Comp (Flow ρ Γ β) :=
  Spec.bind action fun (flow : Flow ρ Γ α) =>
    match flow with
    | .value v env => next v env
    | .return_ result env => Spec.pure (.return_ result env)
    | .break_ nest env => Spec.pure (.break_ nest env)
    | .continue_ nest env => Spec.pure (.continue_ nest env)

/-- A typed choice of variant: a position in the names and rows of an enum. -/
inductive Which : List String → NRows → NRow → Type where
  | here {name : String} {names : List String} {fields : NRow} {rest : NRows} :
      Which (name :: names) (.cons fields rest) fields
  | there {name : String} {names : List String} {fields σs : NRow} {rest : NRows}
      (later : Which names rest σs) : Which (name :: names) (.cons fields rest) σs

/-- The declaration index of a variant choice. -/
def Which.index : {names : List String} → {rows : NRows} → {σs : NRow} → Which names rows σs → Nat
  | _, _, _, .here => 0
  | _, _, _, .there later => later.index + 1

/-- The name of a variant choice. -/
def Which.name : {names : List String} → {rows : NRows} → {σs : NRow} → Which names rows σs → String
  | name :: _, _, _, .here => name
  | _ :: _, _, _, .there later => later.name

/-- Build a variant value from its fields. -/
def Which.inject : {names : List String} → {rows : NRows} → {σs : NRow} →
    Which names rows σs → HList σs → variantCarrier names rows
  | _, _, _, .here, fields => .inl fields
  | _, _, _, .there later, fields => .inr (later.inject fields)

/-- The fields of a variant value, when it is the chosen variant. -/
def Which.project? : {names : List String} → {rows : NRows} → {σs : NRow} →
    Which names rows σs → variantCarrier names rows → Option (HList σs)
  | _, _, _, .here, .inl fields => some fields
  | _, _, _, .here, .inr _ => none
  | _, _, _, .there _, .inl _ => none
  | _, _, _, .there later, .inr value => later.project? value

/-- The name of the variant a value holds. -/
def variantName : (names : List String) → (rows : NRows) → variantCarrier names rows → String
  | _, .nil, value => nomatch value
  | [], .cons _ _, value => nomatch value
  | name :: _, .cons _ _, .inl _ => name
  | _ :: names, .cons _ rest, .inr value => variantName names rest value

@[simp] theorem Which.inject_here {name : String} {names : List String} {fields : NRow}
    {rest : NRows} (value : HList fields) :
    (Which.here : Which (name :: names) (.cons fields rest) fields).inject value = .inl value := rfl
@[simp] theorem Which.inject_there {name : String} {names : List String} {fields σs : NRow}
    {rest : NRows} (later : Which names rest σs) (value : HList σs) :
    (Which.there (name := name) (fields := fields) later).inject value = .inr (later.inject value) := rfl
@[simp] theorem Which.project?_here_inl {name : String} {names : List String} {fields : NRow}
    {rest : NRows} (value : HList fields) :
    (Which.here : Which (name :: names) (.cons fields rest) fields).project? (.inl value) =
      some value := rfl
@[simp] theorem Which.project?_here_inr {name : String} {names : List String} {fields : NRow}
    {rest : NRows} (value : variantCarrier names rest) :
    (Which.here : Which (name :: names) (.cons fields rest) fields).project? (.inr value) =
      none := rfl
@[simp] theorem Which.project?_there_inl {name : String} {names : List String}
    {fields σs : NRow} {rest : NRows} (later : Which names rest σs) (value : HList fields) :
    (Which.there (name := name) later).project? (.inl value) = none := rfl
@[simp] theorem Which.project?_there_inr {name : String} {names : List String}
    {fields σs : NRow} {rest : NRows} (later : Which names rest σs)
    (value : variantCarrier names rest) :
    (Which.there (name := name) (fields := fields) later).project? (.inr value) =
      later.project? value := rfl
@[simp] theorem variantName_inl (name : String) (names : List String) (fields : NRow)
    (rest : NRows) (value : HList fields) :
    variantName (name :: names) (.cons fields rest) (.inl value) = name := rfl
@[simp] theorem variantName_inr (name : String) (names : List String) (fields : NRow)
    (rest : NRows) (value : variantCarrier names rest) :
    variantName (name :: names) (.cons fields rest) (.inr value) = variantName names rest value :=
  rfl

/-- An encoded variant value is a nominal value of its declaration, tagged
with the variant it holds. -/
theorem variantEncode_nominal (source : StructHandle) : (names : List String) → (rows : NRows) →
    (codecs : RowCodecs rows) → (value : variantCarrier names rows) →
    ∃ fields, variantEncode source names rows codecs value =
      .nominal source (some (variantName names rows value)) fields
  | _, .nil, _, value => nomatch value
  | [], .cons _ _, _, value => nomatch value
  | _ :: _, .cons _ _, _, .inl _ => ⟨_, rfl⟩
  | _ :: names, .cons _ rest, (_, codecs), .inr later =>
      variantEncode_nominal source names rest codecs later

theorem NTy.encode_enum_nominal (source : StructHandle) (names : List String) (rows : NRows)
    (distinct : names.Nodup) (value : variantCarrier names rows) :
    ∃ fields, NTy.encode (.enum source names rows distinct) value =
      .nominal source (some (variantName names rows value)) fields :=
  variantEncode_nominal source names rows _ value

/-- An encoded enum value equated to a nominal value holds that variant. -/
theorem NTy.variantName_of_encode_enum {source owner : StructHandle} {names : List String}
    {rows : NRows} {distinct : names.Nodup} {value : variantCarrier names rows} {name : String}
    {fields : Array RuntimeValue}
    (encoded : NTy.encode (.enum source names rows distinct) value = .nominal owner (some name) fields) :
    variantName names rows value = name := by
  obtain ⟨_, nominal⟩ := NTy.encode_enum_nominal source names rows distinct value
  rw [nominal] at encoded
  injection encoded with _ variant
  exact Option.some.inj variant

/-- The payload choices of a variant-field selection: for each variant that
has the field, its position in that variant's row. -/
inductive Choices (names : List String) (rows : NRows) (τ : NTy) : Type where
  | nil : Choices names rows τ
  | cons {σs : NRow} (choice : Which names rows σs) (x : Var σs τ) (rest : Choices names rows τ) :
      Choices names rows τ

/-- The chosen field of a variant value, when its variant has one. -/
def Choices.select? {names : List String} {rows : NRows} {τ : NTy} :
    Choices names rows τ → variantCarrier names rows → Option τ.carrier
  | .nil, _ => none
  | .cons choice x rest, value =>
      match choice.project? value with
      | some fields => some (x.select fields)
      | none => rest.select? value


@[simp] theorem Choices.select?_nil {names : List String} {rows : NRows} {τ : NTy}
    (value : variantCarrier names rows) : (Choices.nil (τ := τ)).select? value = none := rfl
@[simp] theorem Choices.select?_cons {names : List String} {rows : NRows} {τ : NTy} {σs : NRow}
    (choice : Which names rows σs) (x : Var σs τ) (rest : Choices names rows τ)
    (value : variantCarrier names rows) :
    (Choices.cons choice x rest).select? value =
      match choice.project? value with
      | some fields => some (x.select fields)
      | none => rest.select? value := rfl

/-- The index of an element place, in the forms validation admits: a
literal, the integer a local slot holds, or an offset from the end. -/
inductive PlaceIndex where
  | literal (index : Nat)
  | slot (index : Nat)
  | fromEnd (offset : Nat)
  deriving Repr, DecidableEq

/-- The integer a slot of the locals holds, at any width. -/
def HEnv.slotInt? : (Γ : NRow) → HEnv Γ → Nat → Option Int
  | .nil, _, _ => none
  | .cons (.int _ _) _, env, 0 => env.1.map (·.val)
  | .cons _ _, _, 0 => none
  | .cons _ rest, env, index + 1 => HEnv.slotInt? rest env.2 index

/-- The element position an index names in a vector of a size; none when
the local is not an integer or the position is negative. -/
def PlaceIndex.resolve? {Γ : NRow} (env : HEnv Γ) (size : Nat) : PlaceIndex → Option Nat
  | .literal index => some index
  | .slot position => (HEnv.slotInt? Γ env position).bind fun value =>
      if value < 0 then none else some value.toNat
  | .fromEnd offset => if offset ≤ size then some (size - offset) else none

/-- A typed projection path into a value: through a mutable reference's
current value, into a struct's field, and to a vector's element.  It is
what a place names once its base local is known. -/
inductive Proj : NTy → NTy → Type where
  | nil {τ : NTy} : Proj τ τ
  | deref {τ σ : NTy} (rest : Proj τ σ) : Proj (.ref τ) σ
  | field {source : StructHandle} {fields : NRow} {σ τ : NTy} (x : Var fields σ)
      (rest : Proj σ τ) : Proj (.struct source fields) τ
  | index {τ σ : NTy} (position : PlaceIndex) (rest : Proj τ σ) : Proj (.vector τ) σ
  /-- The field the variants of an enum share at their chosen positions. -/
  | variant {source : StructHandle} {names : List String} {rows : NRows}
      {distinct : names.Nodup} {σ τ : NTy} (choices : Choices names rows σ) (rest : Proj σ τ) :
      Proj (.enum source names rows distinct) τ

/-- Replace the component of a row at a position. -/
def Var.update : {Γ : NRow} → {τ : NTy} → Var Γ τ → τ.carrier → HList Γ → HList Γ
  | _, _, .here, value, values => (value, values.2)
  | _, _, .there rest, value, values => (values.1, rest.update value values.2)

@[simp] theorem Var.update_here {Γ : NRow} {τ : NTy} (value : τ.carrier)
    (values : HList (.cons τ Γ)) :
    (Var.here : Var (.cons τ Γ) τ).update value values = (value, values.2) := rfl
@[simp] theorem Var.update_there {Γ : NRow} {σ τ : NTy} (rest : Var Γ τ) (value : τ.carrier)
    (values : HList (.cons σ Γ)) :
    (Var.there rest).update value values = (values.1, rest.update value values.2) := rfl

/-- The variant value with its chosen field replaced, when its variant has
one. -/
def Choices.update? {names : List String} {rows : NRows} {τ : NTy} :
    Choices names rows τ → τ.carrier → variantCarrier names rows →
    Option (variantCarrier names rows)
  | .nil, _, _ => none
  | .cons choice x rest, replacement, value =>
      match choice.project? value with
      | some fields => some (choice.inject (x.update replacement fields))
      | none => rest.update? replacement value

@[simp] theorem Choices.update?_nil {names : List String} {rows : NRows} {τ : NTy}
    (replacement : τ.carrier) (value : variantCarrier names rows) :
    (Choices.nil (τ := τ)).update? replacement value = none := rfl
@[simp] theorem Choices.update?_cons {names : List String} {rows : NRows} {τ : NTy} {σs : NRow}
    (choice : Which names rows σs) (x : Var σs τ) (rest : Choices names rows τ)
    (replacement : τ.carrier) (value : variantCarrier names rows) :
    (Choices.cons choice x rest).update? replacement value =
      match choice.project? value with
      | some fields => some (choice.inject (x.update replacement fields))
      | none => rest.update? replacement value := rfl

/-- A variant value's fields are the chosen variant's exactly when the value
is that variant built from them. -/
theorem Which.project?_eq_some_iff : {names : List String} → {rows : NRows} → {σs : NRow} →
    (choice : Which names rows σs) → (value : variantCarrier names rows) → (fields : HList σs) →
    choice.project? value = some fields ↔ value = choice.inject fields
  | _, _, _, .here, .inl value, fields => by
      simp only [Which.project?, Which.inject, Option.some.injEq]
      exact ⟨fun h => h ▸ rfl, fun h => Sum.inl.inj h⟩
  | _, _, _, .here, .inr value, fields => by
      simp [Which.project?, Which.inject]
  | _, _, _, .there later, .inl value, fields => by
      simp [Which.project?, Which.inject]
  | _, _, _, .there later, .inr value, fields => by
      simp only [Which.project?, Which.inject]
      exact (Which.project?_eq_some_iff later value fields).trans
        ⟨fun h => h ▸ rfl, fun h => Sum.inr.inj h⟩

/-- A selection with one choice is that variant's field, when it is the
chosen variant. -/
theorem Choices.select?_single {names : List String} {rows : NRows} {τ : NTy} {σs : NRow}
    (choice : Which names rows σs) (x : Var σs τ) (value : variantCarrier names rows) :
    (Choices.cons choice x .nil).select? value = (choice.project? value).map x.select := by
  simp only [Choices.select?]
  cases choice.project? value <;> rfl

/-- An update with one choice rebuilds that variant, when it is the chosen
variant. -/
theorem Choices.update?_single {names : List String} {rows : NRows} {τ : NTy} {σs : NRow}
    (choice : Which names rows σs) (x : Var σs τ) (replacement : τ.carrier)
    (value : variantCarrier names rows) :
    (Choices.cons choice x .nil).update? replacement value =
      (choice.project? value).map fun fields => choice.inject (x.update replacement fields) := by
  simp only [Choices.update?]
  cases choice.project? value <;> rfl

/-- The component of a value a path reaches; none when an element
position is outside its vector, which the runtime leaves unresolved. -/
def Proj.get? {Γ : NRow} (env : HEnv Γ) : {τ σ : NTy} → Proj τ σ → τ.carrier → Option σ.carrier
  | _, _, .nil, value => some value
  | _, _, .deref rest, value => rest.get? env value.1
  | _, _, .field x rest, value => rest.get? env (x.select value)
  | _, _, .index position rest, value =>
      (position.resolve? env value.values.size).bind fun index =>
        value.values[index]?.bind fun element => rest.get? env element
  | _, _, .variant choices rest, value =>
      (choices.select? value).bind fun field => rest.get? env field

/-- The value with the component a path reaches replaced. -/
def Proj.set? {Γ : NRow} (env : HEnv Γ) : {τ σ : NTy} → Proj τ σ → σ.carrier → τ.carrier →
    Option τ.carrier
  | _, _, .nil, replacement, _ => some replacement
  | _, _, .deref rest, replacement, value =>
      (rest.set? env replacement value.1).map fun current => (current, value.2)
  | _, _, .field x rest, replacement, value =>
      (rest.set? env replacement (x.select value)).map fun component => x.update component value
  | _, _, .index position rest, replacement, value =>
      (position.resolve? env value.values.size).bind fun index =>
        value.values[index]?.bind fun element =>
          (rest.set? env replacement element).map fun updated => value.set index updated
  | _, _, .variant choices rest, replacement, value =>
      (choices.select? value).bind fun field =>
        (rest.set? env replacement field).bind fun updated => choices.update? updated value

@[simp] theorem Proj.get?_nil {Γ : NRow} (env : HEnv Γ) {τ : NTy} (value : τ.carrier) :
    (Proj.nil : Proj τ τ).get? env value = some value := rfl
@[simp] theorem Proj.get?_deref {Γ : NRow} (env : HEnv Γ) {τ σ : NTy} (rest : Proj τ σ)
    (value : (NTy.ref τ).carrier) : (Proj.deref rest).get? env value = rest.get? env value.1 := rfl
@[simp] theorem Proj.get?_field {Γ : NRow} (env : HEnv Γ) {source : StructHandle} {fields : NRow}
    {σ τ : NTy} (x : Var fields σ) (rest : Proj σ τ) (value : (NTy.struct source fields).carrier) :
    (Proj.field x rest).get? env value = rest.get? env (x.select value) := rfl
@[simp] theorem Proj.get?_index {Γ : NRow} (env : HEnv Γ) {τ σ : NTy} (position : PlaceIndex)
    (rest : Proj τ σ) (value : (NTy.vector τ).carrier) :
    (Proj.index position rest).get? env value =
      (position.resolve? env value.values.size).bind fun index =>
        value.values[index]?.bind fun element => rest.get? env element := rfl
@[simp] theorem Proj.get?_variant {Γ : NRow} (env : HEnv Γ) {source : StructHandle}
    {names : List String} {rows : NRows} {distinct : names.Nodup} {σ τ : NTy}
    (choices : Choices names rows σ) (rest : Proj σ τ)
    (value : (NTy.enum source names rows distinct).carrier) :
    (Proj.variant choices rest).get? env value =
      (choices.select? value).bind fun field => rest.get? env field := rfl
@[simp] theorem Proj.set?_nil {Γ : NRow} (env : HEnv Γ) {τ : NTy} (replacement value : τ.carrier) :
    (Proj.nil : Proj τ τ).set? env replacement value = some replacement := rfl
@[simp] theorem Proj.set?_deref {Γ : NRow} (env : HEnv Γ) {τ σ : NTy} (rest : Proj τ σ)
    (replacement : σ.carrier) (value : (NTy.ref τ).carrier) :
    (Proj.deref rest).set? env replacement value =
      (rest.set? env replacement value.1).map fun current => (current, value.2) := rfl
@[simp] theorem Proj.set?_field {Γ : NRow} (env : HEnv Γ) {source : StructHandle} {fields : NRow}
    {σ τ : NTy} (x : Var fields σ) (rest : Proj σ τ) (replacement : τ.carrier)
    (value : (NTy.struct source fields).carrier) :
    (Proj.field x rest).set? env replacement value =
      (rest.set? env replacement (x.select value)).map fun component => x.update component value := rfl
@[simp] theorem Proj.set?_index {Γ : NRow} (env : HEnv Γ) {τ σ : NTy} (position : PlaceIndex)
    (rest : Proj τ σ) (replacement : σ.carrier) (value : (NTy.vector τ).carrier) :
    (Proj.index position rest).set? env replacement value =
      (position.resolve? env value.values.size).bind fun index =>
        value.values[index]?.bind fun element =>
          (rest.set? env replacement element).map fun updated => value.set index updated := rfl
@[simp] theorem Proj.set?_variant {Γ : NRow} (env : HEnv Γ) {source : StructHandle}
    {names : List String} {rows : NRows} {distinct : names.Nodup} {σ τ : NTy}
    (choices : Choices names rows σ) (rest : Proj σ τ) (replacement : τ.carrier)
    (value : (NTy.enum source names rows distinct).carrier) :
    (Proj.variant choices rest).set? env replacement value =
      (choices.select? value).bind fun field =>
        (rest.set? env replacement field).bind fun updated => choices.update? updated value := rfl
@[simp] theorem PlaceIndex.resolve?_literal {Γ : NRow} (env : HEnv Γ) (size index : Nat) :
    (PlaceIndex.literal index).resolve? env size = some index := rfl
@[simp] theorem PlaceIndex.resolve?_slot {Γ : NRow} (env : HEnv Γ) (size slot : Nat) :
    (PlaceIndex.slot slot).resolve? env size =
      (HEnv.slotInt? Γ env slot).bind fun value => if value < 0 then none else some value.toNat := rfl
@[simp] theorem PlaceIndex.resolve?_fromEnd {Γ : NRow} (env : HEnv Γ) (size offset : Nat) :
    (PlaceIndex.fromEnd offset).resolve? env size =
      if offset ≤ size then some (size - offset) else none := rfl
@[simp] theorem HEnv.slotInt?_int_zero {width : Nat} {signed : Bool} {rest : NRow}
    (env : HEnv (.cons (.int width signed) rest)) :
    HEnv.slotInt? (.cons (.int width signed) rest) env 0 = env.1.map (·.val) := rfl
@[simp] theorem HEnv.slotInt?_succ {τ : NTy} {rest : NRow} (env : HEnv (.cons τ rest))
    (index : Nat) : HEnv.slotInt? (.cons τ rest) env (index + 1) =
      HEnv.slotInt? rest env.2 index := by cases τ <;> rfl

/-- A row of one type repeated: the elements of a vector literal. -/
abbrev NRow.replicate : Nat → NTy → NRow
  | 0, _ => .nil
  | count + 1, τ => .cons τ (NRow.replicate count τ)

/-- The elements of a uniform row, in order. -/
def HList.toListOf {τ : NTy} : (count : Nat) → HList (NRow.replicate count τ) → List τ.carrier
  | 0, _ => []
  | count + 1, values => values.1 :: HList.toListOf count values.2

@[simp] theorem HList.toListOf_zero {τ : NTy} (values : HList (NRow.replicate 0 τ)) :
    HList.toListOf 0 values = [] := rfl
@[simp] theorem HList.toListOf_succ {τ : NTy} (count : Nat)
    (values : HList (NRow.replicate (count + 1) τ)) :
    HList.toListOf (count + 1) values = values.1 :: HList.toListOf count values.2 := rfl

/-- The targets of a destructuring bind: one optional slot per component. -/
inductive Vars (Γ : NRow) : NRow → Type where
  | nil : Vars Γ .nil
  | cons {σ : NTy} {σs : NRow} (target : Option (Var Γ σ)) (rest : Vars Γ σs) : Vars Γ (.cons σ σs)

/-- Bind the components of a row to their slots. -/
def Vars.set {Γ : NRow} : {σs : NRow} → Vars Γ σs → HList σs → HEnv Γ → HEnv Γ
  | _, .nil, _, env => env
  | _, .cons none rest, values, env => rest.set values.2 env
  | _, .cons (some x) rest, values, env => rest.set values.2 (x.set values.1 env)

@[simp] theorem Vars.set_nil {Γ : NRow} (values : HList .nil) (env : HEnv Γ) :
    Vars.nil.set values env = env := rfl
@[simp] theorem Vars.set_cons_none {Γ : NRow} {σ : NTy} {σs : NRow} (rest : Vars Γ σs)
    (values : HList (.cons σ σs)) (env : HEnv Γ) :
    (Vars.cons none rest).set values env = rest.set values.2 env := rfl
@[simp] theorem Vars.set_cons_some {Γ : NRow} {σ : NTy} {σs : NRow} (x : Var Γ σ)
    (rest : Vars Γ σs) (values : HList (.cons σ σs)) (env : HEnv Γ) :
    (Vars.cons (some x) rest).set values env = rest.set values.2 (x.set values.1 env) := rfl

/-- The body value a declared result denotes. -/
def ResultShape.toBody : (shape : ResultShape) → shape.carrier → shape.bodyType.carrier
  | .none, _ => ()
  | .one _, value => value

mutual
/-- Shallow typed terms over a local row, in a function returning `ρ`. -/
inductive Term (ρ : ResultShape) : NRow → NTy → Type where
  /-- A literal. -/
  | lit {Γ : NRow} {τ : NTy} (value : τ.groundCarrier) : Term ρ Γ τ
  /-- A read of a local slot. -/
  | var {Γ : NRow} {τ : NTy} (x : Var Γ τ) : Term ρ Γ τ
  /-- Checked integer arithmetic. -/
  | checked {Γ : NRow} {width : Nat} {signed : Bool} (op : CheckedOp)
      (failure : ThrowKind) (left right : Term ρ Γ (.int width signed)) :
      Term ρ Γ (.int width signed)
  /-- Modular integer arithmetic, wrapped into the width. -/
  | modular {Γ : NRow} {width : Nat} {signed : Bool} (op : ModularOp)
      (left right : Term ρ Γ (.int width signed)) : Term ρ Γ (.int width signed)
  /-- An ordered comparison of integers. -/
  | compare {Γ : NRow} {width : Nat} {signed : Bool} (op : CompareOp)
      (left right : Term ρ Γ (.int width signed)) : Term ρ Γ .bool
  /-- Structural equality, or its negation. -/
  | equal {Γ : NRow} {τ : NTy} (negated : Bool) (left right : Term ρ Γ τ) : Term ρ Γ .bool
  | not {Γ : NRow} (operand : Term ρ Γ .bool) : Term ρ Γ .bool
  /-- Boolean conjunction (`true`) or disjunction (`false`) of evaluated operands. -/
  | logical {Γ : NRow} (conjunction : Bool) (left right : Term ρ Γ .bool) : Term ρ Γ .bool
  /-- Bitwise operation on unsigned integers. -/
  | bitwise {Γ : NRow} {width : Nat} (op : BitOp)
      (left right : Term ρ Γ (.int width false)) : Term ρ Γ (.int width false)
  /-- Checked shift of an unsigned integer, left (`true`) or right. -/
  | shift {Γ : NRow} {width distanceWidth : Nat} (left : Bool) (failure : ThrowKind)
      (value : Term ρ Γ (.int width false)) (distance : Term ρ Γ (.int distanceWidth false)) :
      Term ρ Γ (.int width false)
  /-- Checked conversion between integer types. -/
  | cast {Γ : NRow} {width : Nat} {signed : Bool} {width' : Nat} {signed' : Bool}
      (failure : ThrowKind) (value : Term ρ Γ (.int width signed)) : Term ρ Γ (.int width' signed')
  | ite {Γ : NRow} {τ : NTy} (condition : Term ρ Γ .bool)
      (thenBranch elseBranch : Term ρ Γ τ) : Term ρ Γ τ
  /-- Bind a slot, then continue.  The slot stays written afterwards, as
  the runtime frame's does. -/
  | let_ {Γ : NRow} {σ τ : NTy} (x : Var Γ σ) (value : Term ρ Γ σ) (body : Term ρ Γ τ) :
      Term ρ Γ τ
  /-- Evaluate for effect, discard the value, then continue. -/
  | drop {Γ : NRow} {σ τ : NTy} (value : Term ρ Γ σ) (body : Term ρ Γ τ) : Term ρ Γ τ
  /-- Write a slot. -/
  | assign {Γ : NRow} {σ : NTy} (x : Var Γ σ) (value : Term ρ Γ σ) : Term ρ Γ .unit
  /-- A named constant: its closed initializer. -/
  | const {Γ : NRow} {τ : NTy} (value : Term ρ .nil τ) : Term ρ Γ τ
  /-- Throw with no argument.  A throw has every type. -/
  | throw0 {Γ : NRow} {τ : NTy} (kind : ThrowKind) : Term ρ Γ τ
  /-- Throw with one evaluated argument, such as an abort code. -/
  | throw1 {Γ : NRow} {σ τ : NTy} (kind : ThrowKind) (code : Term ρ Γ σ) : Term ρ Γ τ
  /-- Return the function's result.  A return has every type. -/
  | return_ {Γ : NRow} {τ : NTy} (value : Term ρ Γ ρ.bodyType) : Term ρ Γ τ
  /-- Leave `nest + 1` enclosing loops. -/
  | break_ {Γ : NRow} {τ : NTy} (nest : Nat) : Term ρ Γ τ
  /-- Restart the `nest + 1`-th enclosing loop. -/
  | continue_ {Γ : NRow} {τ : NTy} (nest : Nat) : Term ρ Γ τ
  /-- Repeat a body until it breaks; the loop's value is unit.  `site` is
  the loop's expression index, which names its invariant. -/
  | loop {Γ : NRow} (site : Nat) (body : Term ρ Γ .unit) : Term ρ Γ .unit
  /-- A direct call of a monomorphic function on evaluated arguments; its
  value is the callee's declared result. -/
  | call {Γ : NRow} {σs : NRow} (handle : FunctionHandle) (shape : ResultShape)
      (arguments : Args ρ Γ σs) : Term ρ Γ shape.bodyType
  /-- A call with type arguments: the callee's own signature `σs` and
  `shape`, the arguments `θ` in the caller's types, and the LIR type
  arguments that instantiate the callee's frame. -/
  | callGeneric {Γ : NRow} {σs : NRow} (handle : FunctionHandle) (typeArgs : Array TypeUse)
      (θ : TypeArgs) (shape : ResultShape) (arguments : Args ρ Γ (NRow.subst θ.1 σs)) :
      Term ρ Γ (shape.subst θ.1).bodyType
  /-- A tuple of evaluated elements. -/
  | tuple {Γ σs : NRow} (elements : Args ρ Γ σs) : Term ρ Γ (.tuple σs)
  /-- A struct value from its evaluated fields. -/
  | pack {Γ σs : NRow} (source : StructHandle) (fields : Args ρ Γ σs) :
      Term ρ Γ (.struct source σs)
  /-- An enum value of the chosen variant from its evaluated fields. -/
  | variant {Γ σs : NRow} {names : List String} {rows : NRows} (source : StructHandle)
      (distinct : names.Nodup) (choice : Which names rows σs) (fields : Args ρ Γ σs) :
      Term ρ Γ (.enum source names rows distinct)
  /-- A field of a struct value. -/
  | field {Γ σs : NRow} {τ : NTy} {source : StructHandle} (x : Var σs τ)
      (value : Term ρ Γ (.struct source σs)) : Term ρ Γ τ
  /-- Whether an enum value holds one of the named variants. -/
  | isVariant {Γ : NRow} {names : List String} {rows : NRows} {source : StructHandle}
      {distinct : names.Nodup} (tests : List String)
      (value : Term ρ Γ (.enum source names rows distinct)) : Term ρ Γ .bool
  /-- A field of an enum value under the chosen variant; undefined under
  any other, as the runtime's is. -/
  | payload {Γ : NRow} {τ : NTy} {names : List String} {rows : NRows} {source : StructHandle}
      {distinct : names.Nodup} (choices : Choices names rows τ)
      (value : Term ρ Γ (.enum source names rows distinct)) : Term ρ Γ τ
  /-- Bind the elements of a tuple to slots, then continue. -/
  | letRow {Γ σs : NRow} {τ : NTy} (targets : Vars Γ σs) (value : Term ρ Γ (.tuple σs))
      (body : Term ρ Γ τ) : Term ρ Γ τ
  /-- Bind the fields of a struct to slots, then continue. -/
  | letFields {Γ σs : NRow} {τ : NTy} {source : StructHandle} (targets : Vars Γ σs)
      (value : Term ρ Γ (.struct source σs)) (body : Term ρ Γ τ) : Term ρ Γ τ
  /-- The current value of a mutable reference. -/
  | deref {Γ : NRow} {τ : NTy} (value : Term ρ Γ (.ref τ)) : Term ρ Γ τ
  /-- Move a local's value out: read it and empty the slot.  A mutable
  reference used as a value moves, so a function's exit and a loan's death
  resolve only the references a slot still holds. -/
  | take {Γ : NRow} {τ : NTy} (x : Var Γ τ) : Term ρ Γ τ
  /-- The death of the reference a local holds: execution continues only
  where its current value is its prophecy.  An empty slot resolves
  nothing; the reference moved on and resolves where it dies. -/
  | resolve {Γ : NRow} {τ : NTy} (x : Var Γ (.ref τ)) : Term ρ Γ .unit
  /-- Replace the current value of the mutable reference a local holds. -/
  | mutate {Γ : NRow} {τ : NTy} (x : Var Γ (.ref τ)) (value : Term ρ Γ τ) : Term ρ Γ .unit
  /-- Read the component of a local a path reaches. -/
  | readPlace {Γ : NRow} {τ σ : NTy} (x : Var Γ τ) (path : Proj τ σ) : Term ρ Γ σ
  /-- Write the component of a local a path reaches. -/
  | writePlace {Γ : NRow} {τ σ : NTy} (x : Var Γ τ) (path : Proj τ σ) (value : Term ρ Γ σ) :
      Term ρ Γ .unit
  /-- Borrow the component of a local a path reaches: a prophecy is
  chosen, the reference is the component with that prophecy, and the lender
  holds the prophecy at the path from then on. -/
  | borrowPlace {Γ : NRow} {τ σ : NTy} (x : Var Γ τ) (path : Proj τ σ) : Term ρ Γ (.ref σ)
  /-- A vector of listed elements. -/
  | vectorLit {Γ : NRow} {τ : NTy} (count : Nat) (elements : Args ρ Γ (NRow.replicate count τ)) :
      Term ρ Γ (.vector τ)
  /-- The length of a vector, which its bound fits in `u64`. -/
  | length {Γ : NRow} {τ : NTy} (vector : Term ρ Γ (.vector τ)) : Term ρ Γ (.int 64 false)
  /-- The address a signer holds. -/
  | signerAddress {Γ : NRow} (signer : Term ρ Γ .signer) : Term ρ Γ .address
  /-- An element by position; aborts outside the vector. -/
  | index {Γ : NRow} {τ : NTy} {width : Nat} {signed : Bool} (vector : Term ρ Γ (.vector τ))
      (position : Term ρ Γ (.int width signed)) : Term ρ Γ τ
  /-- The bounds check of an element place. -/
  | checkIndex {Γ : NRow} {τ : NTy} {width : Nat} {signed : Bool} (failure : ThrowKind)
      (vector : Term ρ Γ (.vector τ)) (position : Term ρ Γ (.int width signed)) : Term ρ Γ .unit
  /-- A vector with one more element at its back, as a value. -/
  | push {Γ : NRow} {τ : NTy} (vector : Term ρ Γ (.vector τ)) (element : Term ρ Γ τ) :
      Term ρ Γ (.vector τ)
  /-- A vector with an element inserted at a position; aborts past its end. -/
  | insert {Γ : NRow} {τ : NTy} {width : Nat} {signed : Bool} (vector : Term ρ Γ (.vector τ))
      (position : Term ρ Γ (.int width signed)) (element : Term ρ Γ τ) : Term ρ Γ (.vector τ)
  /-- The element at a position with the vector without it; aborts outside
  the vector. -/
  | remove {Γ : NRow} {τ : NTy} {width : Nat} {signed : Bool} (vector : Term ρ Γ (.vector τ))
      (position : Term ρ Γ (.int width signed)) :
      Term ρ Γ (.tuple (.cons τ (.cons (.vector τ) .nil)))
  /-- A vector with two elements exchanged; aborts outside the vector. -/
  | swap {Γ : NRow} {τ : NTy} {width : Nat} {signed : Bool} (vector : Term ρ Γ (.vector τ))
      (left right : Term ρ Γ (.int width signed)) : Term ρ Γ (.vector τ)
  /-- The concatenation of two vectors. -/
  | concat {Γ : NRow} {τ : NTy} (left right : Term ρ Γ (.vector τ)) : Term ρ Γ (.vector τ)
  /-- The half-open slice of a vector; aborts on an invalid range. -/
  | slice {Γ : NRow} {τ : NTy} {width : Nat} {signed : Bool} (vector : Term ρ Γ (.vector τ))
      (start stop : Term ρ Γ (.int width signed)) : Term ρ Γ (.vector τ)
  /-- A vector with a half-open range reversed; aborts on an invalid range. -/
  | reverseSlice {Γ : NRow} {τ : NTy} {width : Nat} {signed : Bool} (vector : Term ρ Γ (.vector τ))
      (start stop : Term ρ Γ (.int width signed)) : Term ρ Γ (.vector τ)
  /-- Consume an empty vector; aborts on a nonempty one. -/
  | destroyEmpty {Γ : NRow} {τ : NTy} (vector : Term ρ Γ (.vector τ)) : Term ρ Γ .unit
  /-- Whether some element equals the needle. -/
  | contains {Γ : NRow} {τ : NTy} (vector : Term ρ Γ (.vector τ)) (needle : Term ρ Γ τ) :
      Term ρ Γ .bool
  /-- Whether some element equals the needle, and the first such position. -/
  | indexOf {Γ : NRow} {τ : NTy} (vector : Term ρ Γ (.vector τ)) (needle : Term ρ Γ τ) :
      Term ρ Γ (.tuple (.cons .bool (.cons (.int 64 false) .nil)))
  /-- The structural order of two values (`-1`, `0`, `1`), enum variants
  by the unit's declaration order. -/
  | order {Γ : NRow} {τ : NTy} (orders : Array (Array (Array String)))
      (left right : Term ρ Γ τ) : Term ρ Γ (.int 8 true)
  /-- Evaluate a value, then an effect, and keep the value. -/
  | seqAfter {Γ : NRow} {τ : NTy} (value : Term ρ Γ τ) (effect : Term ρ Γ .unit) : Term ρ Γ τ
  /-- The resource of a family at a key; aborts when none is published. -/
  | globalRead {Γ : NRow} {κ τ : NTy} (family : Family) (key : Term ρ Γ κ) : Term ρ Γ τ
  /-- Whether a resource of a family is published at a key. -/
  | globalContains {Γ : NRow} {κ : NTy} (family : Family) (key : Term ρ Γ κ) : Term ρ Γ .bool
  /-- A mutable borrow of a published resource: a prophecy is chosen, the
  reference is the resource with that prophecy, and the store holds the
  prophecy at the key from then on. -/
  | globalBorrow {Γ : NRow} {κ τ : NTy} (family : Family) (key : Term ρ Γ κ) : Term ρ Γ (.ref τ)
  /-- Publish a resource at a key; aborts when one is already there. -/
  | globalPublish {Γ : NRow} {κ τ : NTy} (family : Family) (key : Term ρ Γ κ)
      (value : Term ρ Γ τ) : Term ρ Γ .unit
  /-- Take a resource from a key; aborts when none is published. -/
  | globalTake {Γ : NRow} {κ τ : NTy} (family : Family) (key : Term ρ Γ κ) : Term ρ Γ τ

/-- An argument row, evaluated left to right.  A reborrowed argument is a
borrow like any other: the callee receives the reference and resolves its
prophecy, which the lender already holds. -/
inductive Args (ρ : ResultShape) : NRow → NRow → Type where
  | nil {Γ : NRow} : Args ρ Γ .nil
  | cons {Γ : NRow} {σ : NTy} {σs : NRow} (head : Term ρ Γ σ) (tail : Args ρ Γ σs) :
      Args ρ Γ (.cons σ σs)
end

/-- The mutable-reference parameters of a function, in parameter order: the
slots whose references it resolves at every exit. -/
inductive Mutables (Γ : NRow) : Type where
  | nil : Mutables Γ
  | cons {τ : NTy} (x : Var Γ (.ref τ)) (rest : Mutables Γ) : Mutables Γ

/-- Resolve every mutable-reference parameter the function still holds,
then continue.  A parameter moved into the result resolves at the caller. -/
def Mutables.resolve {Γ : NRow} {α : Type} : Mutables Γ → HEnv Γ → Comp α → Comp α
  | .nil, _, next => next
  | .cons x rest, env, next =>
      match x.get env with
      | some reference => Spec.bind (Spec.assume (reference.1 = reference.2)) fun _ =>
          rest.resolve env next
      | none => rest.resolve env next

@[simp] theorem Mutables.resolve_nil {Γ : NRow} {α : Type} (env : HEnv Γ) (next : Comp α) :
    (Mutables.nil : Mutables Γ).resolve env next = next := rfl
@[simp] theorem Mutables.resolve_cons {Γ : NRow} {α : Type} {τ : NTy} (x : Var Γ (.ref τ))
    (rest : Mutables Γ) (env : HEnv Γ) (next : Comp α) :
    (Mutables.cons x rest).resolve env next =
      match x.get env with
      | some reference => Spec.bind (Spec.assume (reference.1 = reference.2)) fun _ =>
          rest.resolve env next
      | none => rest.resolve env next := rfl

/-! ## The prophetic meaning of a callee

The big-step semantics passes a mutable reference as a loan and returns
the lender's final value as an export, which may hold the hole of a loan
the result carries.  The prophetic meaning reads the same run with
references as `(current, prophecy)`: the arguments are lent under loans,
and each argument's prophecy is its export with every returned reference's
hole filled by that reference's prophecy. -/

mutual
/-- The runtime value of a native value whose outermost mutable references
are lent under loans taken in order.  A reference passes its current value,
or with `prophecies` its prophecy, under its loan; a tuple lends its
components; any other value is its encoding.  Returns the unused loans. -/
def NTy.lend : (τ : NTy) → Bool → τ.carrier → List Nat → Option (RuntimeValue × List Nat)
  | .ref σ, prophecies, value, loan :: loans =>
      some (.borrow loan (σ.encode (if prophecies then value.2 else value.1)), loans)
  | .ref _, _, _, [] => none
  | .tuple elements, prophecies, values, loans =>
      (NRow.lend elements prophecies values loans).map fun lent => (.tuple lent.1.toArray, lent.2)
  | .unit, _, value, loans => some (NTy.encode .unit value, loans)
  | .bool, _, value, loans => some (NTy.encode .bool value, loans)
  | .int width signed, _, value, loans => some (NTy.encode (.int width signed) value, loans)
  | .address, _, value, loans => some (NTy.encode .address value, loans)
  | .signer, _, value, loans => some (NTy.encode .signer value, loans)
  | .string, _, value, loans => some (NTy.encode .string value, loans)
  | .bytes, _, value, loans => some (NTy.encode .bytes value, loans)
  | .struct source fields, _, value, loans => some (NTy.encode (.struct source fields) value, loans)
  | .enum source names rows distinct, _, value, loans =>
      some (NTy.encode (.enum source names rows distinct) value, loans)
  | .vector element, _, value, loans => some (NTy.encode (.vector element) value, loans)
  | .param index, _, value, loans => some (NTy.encode (.param index) value, loans)

/-- The runtime values of a row lent under loans taken in order. -/
def NRow.lend : (row : NRow) → Bool → HList row → List Nat →
    Option (List RuntimeValue × List Nat)
  | .nil, _, _, loans => some ([], loans)
  | .cons τ rest, prophecies, values, loans =>
      (τ.lend prophecies values.1 loans).bind fun head =>
        (NRow.lend rest prophecies values.2 head.2).map fun tail => (head.1 :: tail.1, tail.2)
end

/-- The runtime argument row of native arguments lent under exactly the
given loans: each argument reference passes its current value. -/
def lendArguments (σs : NRow) (args : HList σs) (loans : List Nat) : Option (Array RuntimeValue) :=
  match NRow.lend σs false args loans with
  | some (row, []) => some row.toArray
  | _ => none

/-- The runtime result row of a native result lent under exactly the given
loans, with each returned reference's current value or, with
`prophecies`, its prophecy. -/
def ResultShape.lend : (shape : ResultShape) → Bool → shape.carrier → List Nat →
    Option (Array RuntimeValue)
  | .none, _, _, [] => some #[]
  | .none, _, _, _ :: _ => Option.none
  | .one τ, prophecies, value, loans =>
      match τ.lend prophecies value loans with
      | some (raw, []) => some #[raw]
      | _ => Option.none

/-- Every argument reference resolves at its prophecy: either the callee
exported its loan with a value that, with the returned references' holes
filled from `returned`, is the prophecy's encoding, or the callee returned
the loan itself, carrying the prophecy. -/
def argumentsResolve : (σs : NRow) → HList σs → List Nat → Array RuntimeValue →
    List (Nat × RuntimeValue) → Prop
  | .nil, _, _, _, _ => True
  | .cons (.ref σ) rest, values, loan :: loans, returned, exports =>
      (match exportedRaw? loan exports with
        | some exported =>
            LeanerIR.SemanticOperations.resolveReturnedBorrows returned exported =
              σ.encode values.1.2
        | none => ∃ entry ∈ returned.toList,
            (loan, σ.encode values.1.2) ∈
              (LeanerIR.SemanticOperations.outermostBorrows entry).toList) ∧
      argumentsResolve rest values.2 loans returned exports
  | .cons (.ref _) _, _, [], _, _ => False
  | .cons .unit rest, values, loans, returned, exports =>
      argumentsResolve rest values.2 loans returned exports
  | .cons .bool rest, values, loans, returned, exports =>
      argumentsResolve rest values.2 loans returned exports
  | .cons (.int _ _) rest, values, loans, returned, exports =>
      argumentsResolve rest values.2 loans returned exports
  | .cons .address rest, values, loans, returned, exports =>
      argumentsResolve rest values.2 loans returned exports
  | .cons .signer rest, values, loans, returned, exports =>
      argumentsResolve rest values.2 loans returned exports
  | .cons .string rest, values, loans, returned, exports =>
      argumentsResolve rest values.2 loans returned exports
  | .cons .bytes rest, values, loans, returned, exports =>
      argumentsResolve rest values.2 loans returned exports
  | .cons (.tuple _) rest, values, loans, returned, exports =>
      argumentsResolve rest values.2 loans returned exports
  | .cons (.struct _ _) rest, values, loans, returned, exports =>
      argumentsResolve rest values.2 loans returned exports
  | .cons (.enum _ _ _ _) rest, values, loans, returned, exports =>
      argumentsResolve rest values.2 loans returned exports
  | .cons (.vector _) rest, values, loans, returned, exports =>
      argumentsResolve rest values.2 loans returned exports
  | .cons (.param _) rest, values, loans, returned, exports =>
      argumentsResolve rest values.2 loans returned exports

/-- A start state for a call lending references under loans: distinct
loans below the allocation frontier, none registered to a global, and fresh
identifiers beyond the frontier. -/
def Admissible (state : RuntimeState) (loans : List Nat) : Prop :=
  loans.Nodup ∧
    (∀ loan ∈ loans, loan < state.nextLoan ∧
      LeanerIR.SemanticOperations.globalLoanKeyIn? state.globalLoans loan = none) ∧
    LeanerIR.SemanticOperations.FreshGlobalLoanIds state

/-- A state with the loan bookkeeping of another: the prophetic meaning
never touches loans, so a call leaves them as they were. -/
def _root_.LeanerIR.RuntimeState.withLoansOf (state other : RuntimeState) : RuntimeState :=
  { state with globalLoans := other.globalLoans, nextLoan := other.nextLoan, pending := other.pending }

/-- What a call denotes, for every callee handle and signature. -/
abbrev CalleeMeaning : Type :=
  FunctionHandle → (σs : NRow) → (shape : ResultShape) → HList σs → Comp shape.carrier

/-- What a call with type arguments denotes, at the callee's own signature
under the family its type arguments induce. -/
abbrev GenericMeaning [Θ : Skolems] : Type :=
  FunctionHandle → Array TypeUse → (θ : TypeArgs) → (σs : NRow) → (shape : ResultShape) →
    @HList (Skolems.instantiate θ Θ) σs → Comp (@ResultShape.carrier (Skolems.instantiate θ Θ) shape)

/-- The prophetic meaning of a callee: its big-step meaning from any
admissible start with the caller's globals, with references as
`(current, prophecy)`.  A call denotes exactly this; a caller reasons about
it through the callee's published contract, or, for a callee returning a
reference, through its denotation. -/
def propheticMeaning (unit : LeanerIR.Validation.ExecutableUnit)
    (typeInstantiation : Array (TypeId × TypeId)) (handle : FunctionHandle)
    (σs : NRow) (shape : ResultShape) (args : HList σs) : Comp shape.carrier where
  ok := fun initial result final =>
    ∃ start loans arguments results exit returnedLoans prophecyRow,
      start.globals = initial.globals ∧ Admissible start loans ∧
      lendArguments σs args loans = some arguments ∧
      (functionSpecAt unit handle typeInstantiation arguments).ok start results exit ∧
      shape.lend false result returnedLoans = some results ∧
      shape.lend true result returnedLoans = some prophecyRow ∧
      argumentsResolve σs args loans prophecyRow (exportsAfter start.pending exit.pending) ∧
      final = exit.withLoansOf initial
  aborts := fun initial error =>
    ∃ start loans arguments,
      start.globals = initial.globals ∧ Admissible start loans ∧
      lendArguments σs args loans = some arguments ∧
      (functionSpecAt unit handle typeInstantiation arguments).aborts start error
  undefined := fun initial =>
    ∃ start loans arguments results exit,
      start.globals = initial.globals ∧ Admissible start loans ∧
      lendArguments σs args loans = some arguments ∧
      (functionSpecAt unit handle typeInstantiation arguments).ok start results exit ∧
      ¬∃ result, ∃ returnedLoans, ∃ resolved : HList σs,
        shape.lend false result returnedLoans = some results ∧
        shape.lend true result returnedLoans = some results ∧
        lendArguments σs resolved loans = some arguments ∧
        argumentsResolve σs resolved loans results (exportsAfter start.pending exit.pending)

omit [Skolems] in
/-- The type instantiation of a callee's frame, computed from the caller's
and the call's type arguments as the runtime does. -/
def frameInstantiation (unit : LeanerIR.Validation.ValidatedUnit) (handle : FunctionHandle)
    (outer : Array (TypeId × TypeId)) (typeArgs : Array TypeUse) : Array (TypeId × TypeId) :=
  LeanerIR.SemanticOperations.callTypeInstantiation unit handle outer (typeArgs.map .typeArg)

/-- The closed meaning of calls with type arguments: the callee's
prophetic meaning under its frame's instantiation, computed from the
caller's as the runtime does. -/
def closedGeneric (unit : LeanerIR.Validation.ExecutableUnit)
    (typeInstantiation : Array (TypeId × TypeId)) : GenericMeaning :=
  fun handle typeArgs θ σs shape args =>
    @propheticMeaning (Skolems.instantiate θ ‹_›) unit
      (frameInstantiation unit.unit handle typeInstantiation typeArgs) handle σs shape args

/-- What calls denote: a call without type arguments, and a call with them
at the family they induce. -/
structure Meanings [Skolems] where
  call : CalleeMeaning
  generic : GenericMeaning
  /-- The frame's type instantiation, which keys its generic families. -/
  typeInstantiation : Array (TypeId × TypeId)

/-- Every call its callee's prophetic meaning, a call with type arguments
under the instantiation its frame computes from `typeInstantiation`. -/
@[reducible] def closedMeanings (unit : LeanerIR.Validation.ExecutableUnit)
    (typeInstantiation : Array (TypeId × TypeId)) : Meanings :=
  ⟨propheticMeaning unit #[], closedGeneric unit typeInstantiation, typeInstantiation⟩

/-- The outcome of a closed computation, back in the enclosing locals. -/
def Flow.rebase {ρ : ResultShape} {Γ : NRow} {α : Type} (env : HEnv Γ) :
    Flow ρ .nil α → Flow ρ Γ α
  | .value v _ => .value v env
  | .return_ result _ => .return_ result env
  | .break_ nest _ => .break_ nest env
  | .continue_ nest _ => .continue_ nest env

/-- The declared result a body value denotes. -/
def ResultShape.ofBody : (shape : ResultShape) → shape.bodyType.carrier → shape.carrier
  | .none, _ => ()
  | .one _, value => value

/-- What one iteration's outcome means for the loop: continue looping,
finish, or transfer control past the loop. -/
def Flow.iterate {ρ : ResultShape} {Γ : NRow}
    (recurse : HEnv Γ → Comp (Flow ρ Γ Unit)) : Flow ρ Γ Unit → Comp (Flow ρ Γ Unit)
  | .value _ env => recurse env
  | .continue_ 0 env => recurse env
  | .continue_ (nest + 1) env => Spec.pure (.continue_ nest env)
  | .break_ 0 env => Spec.pure (.value () env)
  | .break_ (nest + 1) env => Spec.pure (.break_ nest env)
  | .return_ result env => Spec.pure (.return_ result env)

/-- The least fixed point of a loop body, marked with the loop's site so
that verification can name its invariant.  Semantically `Spec.fix`. -/
def loopAt {ρ : ResultShape} {Γ : NRow} (_site : Nat)
    (iteration : (HEnv Γ → Comp (Flow ρ Γ Unit)) → HEnv Γ → Comp (Flow ρ Γ Unit))
    (entry : HEnv Γ) : Comp (Flow ρ Γ Unit) :=
  Spec.fix iteration entry

mutual
/-- The denotation of a term: its outcome with the locals after it. -/
def Term.denote (meanings : Meanings) {ρ : ResultShape} : {Γ : NRow} → {τ : NTy} →
    Term ρ Γ τ → HEnv Γ → Comp (Flow ρ Γ τ.carrier)
  | _, _, .lit value, env => Spec.pure (.value (NTy.ofGround _ value) env)
  | _, _, .var x, env =>
      match x.get env with
      | some value => Spec.pure (.value value env)
      | none => Spec.bottom
  | _, _, .checked op failure left right, env =>
      Flow.bind (left.denote meanings env) fun l env =>
        Flow.bind (right.denote meanings env) fun r env =>
          Spec.bind (op.run failure l r) fun value => Spec.pure (.value value env)
  | _, _, .modular op left right, env =>
      Flow.bind (left.denote meanings env) fun l env =>
        Flow.bind (right.denote meanings env) fun r env => Spec.pure (.value (op.run l r) env)
  | _, _, .compare op left right, env =>
      Flow.bind (left.denote meanings env) fun l env =>
        Flow.bind (right.denote meanings env) fun r env =>
          Spec.pure (.value (op.decide l.val r.val) env)
  | _, _, .equal negated left right, env =>
      Flow.bind (left.denote meanings env) fun l env =>
        Flow.bind (right.denote meanings env) fun r env =>
          Spec.pure (.value (if negated then !NTy.eqb _ l r else NTy.eqb _ l r) env)
  | _, _, .not operand, env =>
      Flow.bind (operand.denote meanings env) fun b env => Spec.pure (.value (!(b : Bool)) env)
  | _, _, .logical conjunction left right, env =>
      Flow.bind (left.denote meanings env) fun l env =>
        Flow.bind (right.denote meanings env) fun r env =>
          Spec.pure (.value (if conjunction then (l : Bool) && r else (l : Bool) || r) env)
  | _, _, .bitwise op left right, env =>
      Flow.bind (left.denote meanings env) fun l env =>
        Flow.bind (right.denote meanings env) fun r env => Spec.pure (.value (op.run l r) env)
  | _, _, .shift left failure value distance, env =>
      Flow.bind (value.denote meanings env) fun v env =>
        Flow.bind (distance.denote meanings env) fun d env =>
          Spec.bind (if left then checkedShiftLeft failure v d
            else checkedShiftRight failure v d) fun result => Spec.pure (.value result env)
  | _, .int width' signed', .cast failure value, env =>
      Flow.bind (value.denote meanings env) fun v env =>
        Spec.bind (checkedInt failure width' signed' v.val) fun result =>
          Spec.pure (.value result env)
  | _, _, .ite condition thenBranch elseBranch, env =>
      Flow.bind (condition.denote meanings env) fun b env =>
        if (b : Bool) then thenBranch.denote meanings env else elseBranch.denote meanings env
  | _, _, .let_ x value body, env =>
      Flow.bind (value.denote meanings env) fun v env => body.denote meanings (x.set v env)
  | _, _, .drop value body, env =>
      Flow.bind (value.denote meanings env) fun _ env => body.denote meanings env
  | _, _, .assign x value, env =>
      Flow.bind (value.denote meanings env) fun v env => Spec.pure (.value () (x.set v env))
  | _, _, .const value, env =>
      Spec.bind (value.denote meanings ()) fun flow => Spec.pure (flow.rebase env)
  | _, _, .throw0 kind, _ => Spec.abort (kind, #[])
  | _, _, .throw1 kind code, env =>
      Flow.bind (code.denote meanings env) fun value _ =>
        Spec.abort (kind, #[NTy.encode _ value])
  | _, _, .return_ value, env =>
      Flow.bind (value.denote meanings env) fun v env => Spec.pure (.return_ (ρ.ofBody v) env)
  | _, _, .break_ nest, env => Spec.pure (.break_ nest env)
  | _, _, .continue_ nest, env => Spec.pure (.continue_ nest env)
  | _, _, .loop site body, env =>
      loopAt site (fun recurse env => Spec.bind (body.denote meanings env) (Flow.iterate recurse)) env
  | _, _, .call handle shape arguments, env =>
      Flow.bind (arguments.denote meanings env) fun values env =>
        Spec.bind (meanings.call handle _ shape values) fun result =>
          Spec.pure (.value (shape.toBody result) env)
  | _, _, .callGeneric handle typeArgs θ shape arguments, env =>
      Flow.bind (arguments.denote meanings env) fun values env =>
        Spec.bind (meanings.generic handle typeArgs θ _ shape (HList.toSkolem θ _ values)) fun result =>
          Spec.pure (.value ((shape.subst θ.1).toBody (ResultShape.ofSkolem θ shape result)) env)
  | _, _, .tuple elements, env =>
      Flow.bind (elements.denote meanings env) fun values env => Spec.pure (.value values env)
  | _, _, .pack _ fields, env =>
      Flow.bind (fields.denote meanings env) fun values env => Spec.pure (.value values env)
  | _, _, .variant _ _ choice fields, env =>
      Flow.bind (fields.denote meanings env) fun values env =>
        Spec.pure (.value (choice.inject values) env)
  | _, _, .field x value, env =>
      Flow.bind (value.denote meanings env) fun v env => Spec.pure (.value (x.select v) env)
  | _, _, .isVariant tests value, env =>
      Flow.bind (value.denote meanings env) fun v env =>
        Spec.pure (.value (namedIn (variantName _ _ v) tests) env)
  | _, _, .payload choices value, env =>
      Flow.bind (value.denote meanings env) fun v env =>
        match choices.select? v with
        | some field => Spec.pure (.value field env)
        | none => Spec.bottom
  | _, _, .letRow targets value body, env =>
      Flow.bind (value.denote meanings env) fun values env => body.denote meanings (targets.set values env)
  | _, _, .letFields targets value body, env =>
      Flow.bind (value.denote meanings env) fun values env => body.denote meanings (targets.set values env)
  | _, _, .deref value, env =>
      Flow.bind (value.denote meanings env) fun v env => Spec.pure (.value v.1 env)
  | _, _, .take x, env =>
      match x.get env with
      | some value => Spec.pure (.value value (x.clear env))
      | none => Spec.bottom
  | _, _, .resolve x, env =>
      match x.get env with
      | some reference =>
          Spec.bind (Spec.assume (reference.1 = reference.2)) fun _ => Spec.pure (.value () env)
      | none => Spec.pure (.value () env)
  | _, _, .mutate x value, env =>
      Flow.bind (value.denote meanings env) fun v env =>
        match x.get env with
        | some current => Spec.pure (.value () (x.set (v, current.2) env))
        | none => Spec.bottom
  | _, _, .readPlace x path, env =>
      match x.get env with
      | some value =>
          match path.get? env value with
          | some component => Spec.pure (.value component env)
          | none => Spec.bottom
      | none => Spec.bottom
  | _, _, .writePlace x path value, env =>
      Flow.bind (value.denote meanings env) fun v env =>
        match x.get env with
        | some current =>
            match path.set? env v current with
            | some updated => Spec.pure (.value () (x.set updated env))
            | none => Spec.bottom
        | none => Spec.bottom
  | _, _, .borrowPlace x path, env =>
      match x.get env with
      | some value =>
          match path.get? env value with
          | some component =>
              Spec.bind Spec.choose fun prophecy =>
                match path.set? env prophecy value with
                | some lent => Spec.pure (.value (component, prophecy) (x.set lent env))
                | none => Spec.bottom
          | none => Spec.bottom
      | none => Spec.bottom
  | _, _, .vectorLit count elements, env =>
      Flow.bind (elements.denote meanings env) fun values env =>
        match SpecVector.ofArray? (HList.toListOf count values).toArray with
        | some vector => Spec.pure (.value vector env)
        | none => Spec.bottom
  | _, _, .length vector, env =>
      Flow.bind (vector.denote meanings env) fun values env =>
        Spec.pure (.value (vectorLength values) env)
  | _, _, .signerAddress signer, env =>
      Flow.bind (signer.denote meanings env) fun value env => Spec.pure (.value value env)
  | _, _, .index vector position, env =>
      Flow.bind (vector.denote meanings env) fun values env =>
        Flow.bind (position.denote meanings env) fun i env =>
          if i.val < 0 then Spec.abort (.abort, #[.integer i.val])
          else match values.values[i.val.toNat]? with
            | some element => Spec.pure (.value element env)
            | none => Spec.abort (.abort, #[.integer i.val])
  | _, _, .checkIndex failure vector position, env =>
      Flow.bind (vector.denote meanings env) fun values env =>
        Flow.bind (position.denote meanings env) fun i env =>
          if 0 ≤ i.val ∧ i.val < values.values.size then Spec.pure (.value () env)
          else Spec.abort (failure, #[.integer 1])
  | _, _, .push vector element, env =>
      Flow.bind (vector.denote meanings env) fun values env =>
        Flow.bind (element.denote meanings env) fun e env =>
          match SpecVector.ofArray? (values.values.push e) with
          | some vector => Spec.pure (.value vector env)
          | none => Spec.bottom
  | _, _, .insert vector position element, env =>
      Flow.bind (vector.denote meanings env) fun values env =>
        Flow.bind (position.denote meanings env) fun i env =>
          Flow.bind (element.denote meanings env) fun e env =>
            if i.val < 0 ∨ values.values.size < i.val.toNat then Spec.abort (.abort, #[.integer i.val])
            else match SpecVector.ofArray? (values.values.insertIdxIfInBounds i.val.toNat e) with
              | some vector => Spec.pure (.value vector env)
              | none => Spec.bottom
  | _, _, .remove vector position, env =>
      Flow.bind (vector.denote meanings env) fun values env =>
        Flow.bind (position.denote meanings env) fun i env =>
          if i.val < 0 then Spec.abort (.abort, #[.integer i.val])
          else match values.values[i.val.toNat]? with
            | some element =>
                match SpecVector.ofArray? (values.values.eraseIdxIfInBounds i.val.toNat) with
                | some remaining =>
                    Spec.pure (.value ((element, (remaining, ())) :
                      HList (.cons _ (.cons (.vector _) .nil))) env)
                | none => Spec.bottom
            | none => Spec.abort (.abort, #[.integer i.val])
  | _, _, .swap vector left right, env =>
      Flow.bind (vector.denote meanings env) fun values env =>
        Flow.bind (left.denote meanings env) fun i env =>
          Flow.bind (right.denote meanings env) fun j env =>
            if i.val < 0 ∨ j.val < 0 ∨ values.values.size ≤ i.val.toNat ∨
                values.values.size ≤ j.val.toNat then
              Spec.abort (.abort, #[.integer i.val, .integer j.val])
            else match values.values[i.val.toNat]? with
              | none => Spec.abort (.abort, #[.integer i.val, .integer j.val])
              | some a =>
                match values.values[j.val.toNat]? with
                | none => Spec.abort (.abort, #[.integer i.val, .integer j.val])
                | some b =>
                  match SpecVector.ofArray?
                      ((values.values.set! i.val.toNat b).set! j.val.toNat a) with
                  | some vector => Spec.pure (.value vector env)
                  | none => Spec.bottom
  | _, _, .concat left right, env =>
      Flow.bind (left.denote meanings env) fun l env =>
        Flow.bind (right.denote meanings env) fun r env =>
          match SpecVector.ofArray? (l.values ++ r.values) with
          | some vector => Spec.pure (.value vector env)
          | none => Spec.bottom
  | _, _, .slice vector start stop, env =>
      Flow.bind (vector.denote meanings env) fun values env =>
        Flow.bind (start.denote meanings env) fun a env =>
          Flow.bind (stop.denote meanings env) fun b env =>
            if a.val < 0 ∨ b.val < a.val ∨ values.values.size < b.val.toNat then
              Spec.abort (.abort, #[.integer a.val, .integer b.val])
            else match SpecVector.ofArray? (values.values.extract a.val.toNat b.val.toNat) with
              | some vector => Spec.pure (.value vector env)
              | none => Spec.bottom
  | _, _, .reverseSlice vector start stop, env =>
      Flow.bind (vector.denote meanings env) fun values env =>
        Flow.bind (start.denote meanings env) fun a env =>
          Flow.bind (stop.denote meanings env) fun b env =>
            if a.val < 0 ∨ b.val < a.val ∨ values.values.size < b.val.toNat then
              Spec.abort (.abort, #[.integer a.val, .integer b.val])
            else match SpecVector.ofArray? (reverseRange ((b.val.toNat - a.val.toNat) / 2)
                a.val.toNat (b.val.toNat - 1) values.values) with
              | some vector => Spec.pure (.value vector env)
              | none => Spec.bottom
  | _, _, .destroyEmpty vector, env =>
      Flow.bind (vector.denote meanings env) fun values env =>
        if values.values.isEmpty then Spec.pure (.value () env) else Spec.abort (.abort, #[])
  | _, _, @Term.contains _ _ τ vector needle, env =>
      Flow.bind (vector.denote meanings env) fun values env =>
        Flow.bind (needle.denote meanings env) fun n env =>
          Spec.pure (.value
            (findIndex? (NTy.eqb τ) values.values n values.values.size 0).isSome env)
  | _, _, @Term.indexOf _ _ τ vector needle, env =>
      Flow.bind (vector.denote meanings env) fun values env =>
        Flow.bind (needle.denote meanings env) fun n env =>
          Spec.pure (.value
            (((findIndex? (NTy.eqb τ) values.values n values.values.size 0).isSome,
              (foundIndexOf (NTy.eqb τ) values n, ())) :
              HList (.cons .bool (.cons (.int 64 false) .nil))) env)
  | _, _, @Term.order _ _ τ orders left right, env =>
      Flow.bind (left.denote meanings env) fun a env =>
        Flow.bind (right.denote meanings env) fun b env =>
          Spec.pure (.value (compareResult orders (NTy.encode τ a) (NTy.encode τ b)) env)
  | _, _, .seqAfter value effect, env =>
      Flow.bind (value.denote meanings env) fun v env =>
        Flow.bind (effect.denote meanings env) fun _ env => Spec.pure (.value v env)
  | _, τ, @Term.globalRead _ _ κ _ family key, env =>
      Flow.bind (key.denote meanings env) fun k env =>
        Spec.bind Spec.get fun state =>
          match state.globals.lookup ((family.instantiate meanings.typeInstantiation).key (κ.encode k)) with
          | none => Spec.abort (.abort, #[])
          | some raw => Spec.bind (decodeOr τ.codec raw) fun value => Spec.pure (.value value env)
  | _, _, @Term.globalContains _ _ κ family key, env =>
      Flow.bind (key.denote meanings env) fun k env =>
        Spec.bind Spec.get fun state =>
          Spec.pure (.value (state.globals.lookup ((family.instantiate meanings.typeInstantiation).key (κ.encode k))).isSome env)
  | _, .ref τ, @Term.globalBorrow _ _ κ _ family key, env =>
      Flow.bind (key.denote meanings env) fun k env =>
        Spec.bind Spec.get fun state =>
          match state.globals.lookup ((family.instantiate meanings.typeInstantiation).key (κ.encode k)) with
          | none => Spec.abort (.abort, #[])
          | some raw =>
              Spec.bind (decodeOr τ.codec raw) fun value =>
                Spec.bind Spec.choose fun prophecy =>
                  Spec.bind (Spec.modify fun state =>
                    { state with
                      globals := state.globals.insert ((family.instantiate meanings.typeInstantiation).key (κ.encode k)) (τ.encode prophecy) })
                    fun _ => Spec.pure (.value (value, prophecy) env)
  | _, _, @Term.globalPublish _ _ κ τ family key value, env =>
      Flow.bind (key.denote meanings env) fun k env =>
        Flow.bind (value.denote meanings env) fun v env =>
          Spec.bind Spec.get fun state =>
            match state.globals.lookup ((family.instantiate meanings.typeInstantiation).key (κ.encode k)) with
            | some _ => Spec.abort (.abort, #[])
            | none =>
                Spec.bind (Spec.modify fun state =>
                  { state with globals := state.globals.insert ((family.instantiate meanings.typeInstantiation).key (κ.encode k)) (τ.encode v) })
                  fun _ => Spec.pure (.value () env)
  | _, τ, @Term.globalTake _ _ κ _ family key, env =>
      Flow.bind (key.denote meanings env) fun k env =>
        Spec.bind Spec.get fun state =>
          match state.globals.lookup ((family.instantiate meanings.typeInstantiation).key (κ.encode k)) with
          | none => Spec.abort (.abort, #[])
          | some raw =>
              Spec.bind (decodeOr τ.codec raw) fun value =>
                Spec.bind (Spec.modify fun state =>
                  { state with globals := state.globals.erase ((family.instantiate meanings.typeInstantiation).key (κ.encode k)) })
                  fun _ => Spec.pure (.value value env)

/-- The denotation of an argument row: its values with the locals after it. -/
def Args.denote (meanings : Meanings) {ρ : ResultShape} : {Γ σs : NRow} → Args ρ Γ σs →
    HEnv Γ → Comp (Flow ρ Γ (HList σs))
  | _, _, .nil, env => Spec.pure (.value () env)
  | _, _, .cons head tail, env =>
      Flow.bind (head.denote meanings env) fun value env =>
        Flow.bind (tail.denote meanings env) fun values env =>
          Spec.pure (.value (value, values) env)
end

/-- A compiled function: its parameter and remaining local types, its
declared result shape, and its body over the complete local row. -/
structure Function where
  params : NRow
  locals : NRow
  result : ResultShape
  body : Term result (params ++ locals) result.bodyType
  /-- The mutable-reference parameters, resolved at exit. -/
  mutables : Mutables (params ++ locals)

/-- The function result a body outcome denotes, after the mutable-reference
parameters it still holds are resolved.  Loop control that escapes the body
has no meaning, as the runtime's `finishControl?` has none. -/
def ResultShape.finish {Γ : NRow} (mutables : Mutables Γ) (shape : ResultShape) :
    Flow shape Γ shape.bodyType.carrier → Comp shape.carrier
  | .value value env => mutables.resolve env (Spec.pure (shape.ofBody value))
  | .return_ result env => mutables.resolve env (Spec.pure result)
  | .break_ _ _ => Spec.bottom
  | .continue_ _ _ => Spec.bottom

/-- The denotation of a function on native arguments, with calls meaning
what `meaning` says. -/
def Function.denoteWith (meanings : Meanings) (f : Function) (args : HList f.params) :
    Comp f.result.carrier :=
  Spec.bind (f.body.denote meanings (initialEnv f.params f.locals args))
    (f.result.finish f.mutables)

/-- The denotation of a function on native arguments, every call meaning
its callee's prophetic meaning.  Stated directly: every transport unifies
against it. -/
def Function.denote (unit : LeanerIR.Validation.ExecutableUnit)
    (typeInstantiation : Array (TypeId × TypeId)) (f : Function) (args : HList f.params) :
    Comp f.result.carrier :=
  Spec.bind (f.body.denote (closedMeanings unit typeInstantiation)
      (initialEnv f.params f.locals args))
    (f.result.finish f.mutables)

/-- The meaning of calls routing one function to `self`: its handle at its
own signature means `self`, every other call `rest`. -/
def routeMeaning (handle : FunctionHandle) (π : NRow) (ρ : ResultShape)
    (self : HList π → Comp ρ.carrier) (rest : CalleeMeaning) : CalleeMeaning :=
  fun callee σs shape args =>
    if routed : callee = handle ∧ σs = π ∧ shape = ρ then
      routed.2.2 ▸ self (routed.2.1 ▸ args)
    else rest callee σs shape args

@[simp] theorem routeMeaning_self (handle : FunctionHandle) (π : NRow) (ρ : ResultShape)
    (self : HList π → Comp ρ.carrier) (rest : CalleeMeaning) (args : HList π) :
    routeMeaning handle π ρ self rest handle π ρ args = self args := by
  simp [routeMeaning]

theorem routeMeaning_other {handle callee : FunctionHandle} {π σs : NRow} {ρ shape : ResultShape}
    (self : HList π → Comp ρ.carrier) (rest : CalleeMeaning) (args : HList σs)
    (other : callee ≠ handle) :
    routeMeaning handle π ρ self rest callee σs shape args = rest callee σs shape args := by
  simp [routeMeaning, other]

/-- The meaning of calls in the body of a generic function calling itself:
its own handle at its own signature means `self`, every other call its
callee's prophetic meaning. -/
def recursiveMeaning (unit : LeanerIR.Validation.ExecutableUnit) (handle : FunctionHandle)
    (π : NRow) (ρ : ResultShape) (self : HList π → Comp ρ.carrier) : CalleeMeaning :=
  routeMeaning handle π ρ self (propheticMeaning unit #[])

@[simp] theorem recursiveMeaning_self (unit : LeanerIR.Validation.ExecutableUnit)
    (handle : FunctionHandle) (π : NRow) (ρ : ResultShape) (self : HList π → Comp ρ.carrier)
    (args : HList π) :
    recursiveMeaning unit handle π ρ self handle π ρ args = self args :=
  routeMeaning_self handle π ρ self _ args

theorem recursiveMeaning_other (unit : LeanerIR.Validation.ExecutableUnit)
    {handle callee : FunctionHandle} {π σs : NRow} {ρ shape : ResultShape}
    (self : HList π → Comp ρ.carrier) (args : HList σs) (other : callee ≠ handle) :
    recursiveMeaning unit handle π ρ self callee σs shape args =
      propheticMeaning unit #[] callee σs shape args :=
  routeMeaning_other self _ args other

/-- A position among the members of a cycle of calls, each a handle with
its compiled function. -/
inductive CycleIndex : List (FunctionHandle × Function) → Type where
  | here {member : FunctionHandle × Function} {others : List (FunctionHandle × Function)} :
      CycleIndex (member :: others)
  | there {member : FunctionHandle × Function} {others : List (FunctionHandle × Function)} :
      CycleIndex others → CycleIndex (member :: others)

/-- The member a position names. -/
def CycleIndex.member : {members : List (FunctionHandle × Function)} → CycleIndex members →
    FunctionHandle × Function
  | member :: _, .here => member
  | _ :: _, .there later => later.member

/-- The meanings standing for a cycle's members, by position. -/
abbrev CycleSelves (members : List (FunctionHandle × Function)) : Type :=
  (index : CycleIndex members) → HList index.member.2.params → Comp index.member.2.result.carrier

/-- Route each of a cycle's members to its meaning, every other call to `rest`. -/
def cycleRoutes (rest : CalleeMeaning) :
    (members : List (FunctionHandle × Function)) → CycleSelves members → CalleeMeaning
  | [], _ => rest
  | member :: others, self =>
      routeMeaning member.1 member.2.params member.2.result (self .here)
        (cycleRoutes rest others fun index => self (.there index))

/-- The meaning of calls in the bodies of a cycle's members: each member's
handle at its own signature means its `self`, every other call its callee's
prophetic meaning. -/
def cycleMeaning (unit : LeanerIR.Validation.ExecutableUnit)
    (members : List (FunctionHandle × Function)) (self : CycleSelves members) : CalleeMeaning :=
  cycleRoutes (propheticMeaning unit #[]) members self

/-- The recursion hypothesis of a generic function: its meaning at every
skolem family and type instantiation, so that a call to itself with type
arguments is the hypothesis at the family and frame instantiation the call
induces. -/
abbrev SelfFamily (π : NRow) (ρ : ResultShape) : Type 1 :=
  (Θ : Skolems) → Array (TypeId × TypeId) → @HList Θ π → Comp (@ResultShape.carrier Θ ρ)

/-- The meaning of calls with type arguments in the body of a generic
function calling itself: its own handle at its own signature means `self`
at the induced family and the callee's frame instantiation, every other
call its closed meaning. -/
def recursiveGeneric (unit : LeanerIR.Validation.ExecutableUnit) (handle : FunctionHandle)
    (π : NRow) (ρ : ResultShape) (self : SelfFamily π ρ)
    (typeInstantiation : Array (TypeId × TypeId)) : GenericMeaning :=
  fun callee typeArgs θ σs shape args =>
    if routed : callee = handle ∧ σs = π ∧ shape = ρ then
      routed.2.2 ▸ self (Skolems.instantiate θ ‹Skolems›)
        (frameInstantiation unit.unit callee typeInstantiation typeArgs) (routed.2.1 ▸ args)
    else closedGeneric unit typeInstantiation callee typeArgs θ σs shape args

@[simp] theorem recursiveGeneric_self (unit : LeanerIR.Validation.ExecutableUnit)
    (handle : FunctionHandle) (π : NRow) (ρ : ResultShape) (self : SelfFamily π ρ)
    (typeInstantiation : Array (TypeId × TypeId)) (typeArgs : Array TypeUse) (θ : TypeArgs)
    (args : @HList (Skolems.instantiate θ ‹Skolems›) π) :
    recursiveGeneric unit handle π ρ self typeInstantiation handle typeArgs θ π ρ args =
      self (Skolems.instantiate θ ‹Skolems›)
        (frameInstantiation unit.unit handle typeInstantiation typeArgs) args := by
  simp [recursiveGeneric]

theorem recursiveGeneric_other (unit : LeanerIR.Validation.ExecutableUnit)
    {handle callee : FunctionHandle} {π σs : NRow} {ρ shape : ResultShape}
    (self : SelfFamily π ρ) (typeInstantiation : Array (TypeId × TypeId))
    (typeArgs : Array TypeUse) (θ : TypeArgs) (args : @HList (Skolems.instantiate θ ‹Skolems›) σs)
    (other : callee ≠ handle) :
    recursiveGeneric unit handle π ρ self typeInstantiation callee typeArgs θ σs shape args =
      closedGeneric unit typeInstantiation callee typeArgs θ σs shape args := by
  simp [recursiveGeneric, other]

omit [Skolems] in
/-- The weakest precondition through a verified callee: its precondition,
its postcondition and frame under the continuation, and its failures under
the permitted failures. -/
theorem wp_call {Args Result : Type} {function : Args → Comp Result}
    {contract : Contract RuntimeState Failure Args Result} {args : Args}
    {ensures : Result → RuntimeState → Prop} {aborts : Failure → Prop} {initial : RuntimeState}
    (verified : Satisfies function contract)
    (permitted : contract.requires args initial)
    (post : ∀ result final,
      (¬contract.mayAbort args initial → contract.ensures args initial result final) →
      contract.frame args initial final → ¬contract.mustAbort args initial →
      ensures result final)
    (failing : ∀ error, contract.aborts args initial error → aborts error) :
    wp (function args) ensures aborts initial :=
  ⟨fun result final execution =>
      let established := (verified args initial permitted).1 result final execution
      post result final established.1 established.2.1 established.2.2,
    fun error execution => failing error ((verified args initial permitted).2.1 error execution),
    (verified args initial permitted).2.2⟩

/-! ## Weakest preconditions of the flow combinators

A bind on a literal outcome reduces directly; only a symbolic outcome goes
through the weakest-precondition rule. -/

@[simp] theorem Flow.bind_pure_value {ρ : ResultShape} {Γ : NRow} {α β : Type}
    (value : α) (env : HEnv Γ) (next : α → HEnv Γ → Comp (Flow ρ Γ β)) :
    Flow.bind (Spec.pure (.value value env)) next = next value env := by
  simp [Flow.bind]
@[simp] theorem Flow.bind_pure_return {ρ : ResultShape} {Γ : NRow} {α β : Type}
    (result : ρ.carrier) (env : HEnv Γ) (next : α → HEnv Γ → Comp (Flow ρ Γ β)) :
    Flow.bind (Spec.pure (.return_ result env)) next = Spec.pure (.return_ result env) := by
  simp [Flow.bind]
@[simp] theorem Flow.bind_pure_break {ρ : ResultShape} {Γ : NRow} {α β : Type}
    (nest : Nat) (env : HEnv Γ) (next : α → HEnv Γ → Comp (Flow ρ Γ β)) :
    Flow.bind (Spec.pure (.break_ nest env)) next = Spec.pure (.break_ nest env) := by
  simp [Flow.bind]
@[simp] theorem Flow.bind_pure_continue {ρ : ResultShape} {Γ : NRow} {α β : Type}
    (nest : Nat) (env : HEnv Γ) (next : α → HEnv Γ → Comp (Flow ρ Γ β)) :
    Flow.bind (Spec.pure (.continue_ nest env)) next = Spec.pure (.continue_ nest env) := by
  simp [Flow.bind]
@[simp] theorem Flow.bind_abort {ρ : ResultShape} {Γ : NRow} {α β : Type}
    (error : Failure) (next : α → HEnv Γ → Comp (Flow ρ Γ β)) :
    Flow.bind (Spec.abort error) next = Spec.abort error := by
  simp [Flow.bind]


theorem wp_flowBind {ρ : ResultShape} {Γ : NRow} {α β : Type}
    (action : Comp (Flow ρ Γ α)) (next : α → HEnv Γ → Comp (Flow ρ Γ β))
    (ensures : Flow ρ Γ β → RuntimeState → Prop) (aborts : Failure → Prop)
    (state : RuntimeState) :
    wp (Flow.bind action next) ensures aborts state ↔
      wp action (fun flow state =>
        match flow with
        | .value value env => wp (next value env) ensures aborts state
        | .return_ result env => ensures (.return_ result env) state
        | .break_ nest env => ensures (.break_ nest env) state
        | .continue_ nest env => ensures (.continue_ nest env) state) aborts state := by
  rw [Flow.bind, wp_bind]
  constructor
  · intro h
    refine ⟨fun flow final execution => ?_, h.2.1, h.2.2⟩
    have := h.1 flow final execution
    cases flow <;> simpa [wp_pure] using this
  · intro h
    refine ⟨fun flow final execution => ?_, h.2.1, h.2.2⟩
    have := h.1 flow final execution
    cases flow <;> simpa [wp_pure] using this

/-- Loop verification from an invariant over the locals and the state: it
holds at entry, and one iteration under it is correct whenever the next
iteration is assumed correct under it.  Partial correctness. -/
theorem wp_loopAt {ρ : ResultShape} {Γ : NRow} (site : Nat)
    (iteration : (HEnv Γ → Comp (Flow ρ Γ Unit)) → HEnv Γ → Comp (Flow ρ Γ Unit))
    (entry : HEnv Γ) (invariant : HEnv Γ → RuntimeState → Prop)
    (ensures : Flow ρ Γ Unit → RuntimeState → Prop) (aborts : Failure → Prop)
    (initial : RuntimeState)
    (entryHolds : invariant entry initial)
    (step : ∀ recursive : HEnv Γ → Comp (Flow ρ Γ Unit),
      (∀ env state, invariant env state → wp (recursive env) ensures aborts state) →
      ∀ env state, invariant env state → wp (iteration recursive env) ensures aborts state) :
    wp (loopAt site iteration entry) ensures aborts initial :=
  wp_withInvariant_fix (invariant := invariant) entryHolds step

@[simp] theorem finish_value {Γ : NRow} (mutables : Mutables Γ) (shape : ResultShape)
    (value : shape.bodyType.carrier) (env : HEnv Γ) :
    shape.finish mutables (.value value env) = mutables.resolve env (Spec.pure (shape.ofBody value)) :=
  rfl
@[simp] theorem finish_return {Γ : NRow} (mutables : Mutables Γ) (shape : ResultShape)
    (result : shape.carrier) (env : HEnv Γ) :
    shape.finish mutables (.return_ result env) = mutables.resolve env (Spec.pure result) := rfl
@[simp] theorem finish_break {Γ : NRow} (mutables : Mutables Γ) (shape : ResultShape) (nest : Nat)
    (env : HEnv Γ) : shape.finish mutables (.break_ nest env) = Spec.bottom := rfl
@[simp] theorem finish_continue {Γ : NRow} (mutables : Mutables Γ) (shape : ResultShape) (nest : Nat)
    (env : HEnv Γ) : shape.finish mutables (.continue_ nest env) = Spec.bottom := rfl

-- A one-choice selection or update reads as a map over the projection, whose
-- equations the normalizer inverts into the variant value itself.
attribute [lir_denote high] Choices.select?_single Choices.update?_single
attribute [lir_denote] Which.project?_eq_some_iff

attribute [lir_denote] Term.denote Args.denote Function.denote ResultShape.ofBody NTy.ofGround
  closedGeneric
  HList.ofGround
  ResultShape.toBody
  ResultShape.finish Flow.iterate Flow.rebase Flow.bind_pure_value Flow.bind_pure_return
  Flow.bind_pure_break Flow.bind_pure_continue Flow.bind_abort wp_flowBind finish_value finish_return finish_break
  finish_continue Mutables.resolve_nil Mutables.resolve_cons Option.bind_some Option.bind_none
  Proj.get?_nil Proj.get?_deref Proj.get?_field Proj.get?_index Proj.get?_variant
  Proj.set?_nil Proj.set?_deref Proj.set?_field Proj.set?_index Proj.set?_variant
  Choices.update?_nil Choices.update?_cons PlaceIndex.resolve?_literal
  PlaceIndex.resolve?_slot PlaceIndex.resolve?_fromEnd HEnv.slotInt?_int_zero HEnv.slotInt?_succ
  HList.toListOf_zero HList.toListOf_succ Var.update_here Var.update_there
  Var.clear_here Var.clear_there
  NTy.decode?_encode NTy.encode_ref Which.inject_here Which.inject_there Which.project?_here_inl
  Which.project?_here_inr Which.project?_there_inl Which.project?_there_inr variantName_inl
  variantName_inr Choices.select?_nil Choices.select?_cons Vars.set_nil Vars.set_cons_none
  Vars.set_cons_some Var.select_here Var.select_there NTy.eqb_tuple NTy.eqb_struct NTy.eqb_enum
  rowEqb_nil rowEqb_cons variantEqb_inl_inl variantEqb_inr_inr variantEqb_inl_inr
  variantEqb_inr_inl namedIn_nil namedIn_cons NTy.encode_tuple NTy.encode_struct
  NTy.encode_enum_inl NTy.encode_enum_inr HList.encode_nil HList.encode_cons
  String.reduceBEq String.reduceEq String.reduceBNe String.reduceNe Bool.true_or Bool.false_or
  Bool.or_true Bool.or_false Bool.true_and Bool.false_and Bool.and_true Bool.and_false

/-- A readable rendering of a literal. -/
def describeValue : {τ : NTy} → τ.groundCarrier → String
  | .int _ _, value => toString value.val
  | .bool, value => toString (value : Bool)
  | .unit, _ => "()"
  | .address, value => s!"@{(value : String)}"
  | .signer, value => s!"signer {(value : String)}"
  | .string, value => s!"{repr (value : String)}"
  | .bytes, value => s!"{repr (value : Array UInt8)}"
  | .tuple _, _ => "(..)"
  | .struct _ _, _ => "{..}"
  | .enum _ _ _ _, _ => "variant{..}"
  | .vector _, value => s!"[{value.values.size} elements]"
  | .ref _, _ => "&mut(..)"
  | .param index, _ => s!"T{index}"

/-- A readable rendering of destructuring targets. -/
def Vars.describe {Γ : NRow} : {σs : NRow} → Vars Γ σs → String
  | _, .nil => ""
  | _, .cons target .nil => target.elim "_" fun x => s!"local{x.index}"
  | _, .cons target rest => s!"{target.elim "_" fun x => s!"local{x.index}"}, {rest.describe}"

mutual
/-- A readable rendering of a term, for diagnostics. -/
partial def Term.describe {ρ : ResultShape} : {Γ : NRow} → {τ : NTy} → Term ρ Γ τ → String
  | _, _, .lit value => describeValue value
  | _, _, .var x => s!"local{x.index}"
  | _, _, .checked op failure left right =>
      s!"({left.describe} {repr op} {right.describe} ! {repr failure})"
  | _, _, .modular op left right => s!"({left.describe} {repr op} {right.describe} wrapping)"
  | _, _, .compare op left right => s!"({left.describe} {repr op} {right.describe})"
  | _, _, .equal negated left right =>
      s!"({left.describe} {if negated then "!=" else "=="} {right.describe})"
  | _, _, .not operand => s!"!{operand.describe}"
  | _, _, .logical conjunction left right =>
      s!"({left.describe} {if conjunction then "&&" else "||"} {right.describe})"
  | _, _, .bitwise op left right => s!"({left.describe} {repr op} {right.describe})"
  | _, _, .shift left failure value distance =>
      s!"({value.describe} {if left then "<<" else ">>"} {distance.describe} ! {repr failure})"
  | _, _, @Term.cast _ _ _ _ width signed failure value =>
      s!"({value.describe} as int {width} {signed} ! {repr failure})"
  | _, _, .ite condition thenBranch elseBranch =>
      s!"(if {condition.describe} then {thenBranch.describe} else {elseBranch.describe})"
  | _, _, .let_ x value body => s!"(let local{x.index} := {value.describe}; {body.describe})"
  | _, _, .drop value body => s!"({value.describe}; {body.describe})"
  | _, _, .assign x value => s!"(local{x.index} := {value.describe})"
  | _, _, .const value => s!"const({value.describe})"
  | _, _, .throw0 kind => s!"throw {repr kind}"
  | _, _, .throw1 kind code => s!"throw {repr kind}({code.describe})"
  | _, _, .return_ value => s!"return {value.describe}"
  | _, _, .break_ nest => s!"break {nest}"
  | _, _, .continue_ nest => s!"continue {nest}"
  | _, _, .loop site body => s!"loop@{site} {body.describe}"
  | _, _, .call handle _ arguments =>
      s!"call {handle.namespaceId.index}:{handle.functionId.index}({arguments.describe})"
  | _, _, .callGeneric handle typeArgs _ _ arguments =>
      s!"call {handle.namespaceId.index}:{handle.functionId.index}<{typeArgs.size} types>\
        ({arguments.describe})"
  | _, _, .tuple elements => s!"({elements.describe})"
  | _, _, .pack source fields => s!"{repr source} \{{fields.describe}}"
  | _, _, .variant source _ choice fields => s!"{repr source}::{choice.name} \{{fields.describe}}"
  | _, _, .field x value => s!"{value.describe}.{x.index}"
  | _, _, .isVariant tests value => s!"({value.describe} is {tests})"
  | _, _, .payload _ value => s!"{value.describe}.payload"
  | _, _, .letRow targets value body =>
      s!"(let ({targets.describe}) := {value.describe}; {body.describe})"
  | _, _, .letFields targets value body =>
      s!"(let \{{targets.describe}} := {value.describe}; {body.describe})"
  | _, _, .deref value => s!"*{value.describe}"
  | _, _, .mutate x value => s!"(*local{x.index} := {value.describe})"
  | _, _, .readPlace x _ => s!"local{x.index}.path"
  | _, _, .writePlace x _ value => s!"(local{x.index}.path := {value.describe})"
  | _, _, .borrowPlace x _ => s!"&mut local{x.index}.path"
  | _, _, .take x => s!"move local{x.index}"
  | _, _, .resolve x => s!"resolve local{x.index}"
  | _, _, .vectorLit _ elements => s!"vector[{elements.describe}]"
  | _, _, .length vector => s!"{vector.describe}.length"
  | _, _, .signerAddress signer => s!"{signer.describe}.address"
  | _, _, .index vector position => s!"{vector.describe}[{position.describe}]"
  | _, _, .checkIndex _ vector position => s!"check({vector.describe}[{position.describe}])"
  | _, _, .push vector element => s!"push({vector.describe}, {element.describe})"
  | _, _, .insert vector position element =>
      s!"insert({vector.describe}, {position.describe}, {element.describe})"
  | _, _, .remove vector position => s!"remove({vector.describe}, {position.describe})"
  | _, _, .swap vector left right =>
      s!"swap({vector.describe}, {left.describe}, {right.describe})"
  | _, _, .concat left right => s!"concat({left.describe}, {right.describe})"
  | _, _, .slice vector start stop =>
      s!"slice({vector.describe}, {start.describe}, {stop.describe})"
  | _, _, .reverseSlice vector start stop =>
      s!"reverseSlice({vector.describe}, {start.describe}, {stop.describe})"
  | _, _, .destroyEmpty vector => s!"destroyEmpty({vector.describe})"
  | _, _, .contains vector needle => s!"contains({vector.describe}, {needle.describe})"
  | _, _, .indexOf vector needle => s!"indexOf({vector.describe}, {needle.describe})"
  | _, _, .order _ left right => s!"order({left.describe}, {right.describe})"
  | _, _, .seqAfter value effect => s!"({value.describe} then {effect.describe})"
  | _, _, .globalRead family key => s!"global[{repr family}]({key.describe})"
  | _, _, .globalContains family key => s!"exists[{repr family}]({key.describe})"
  | _, _, .globalBorrow family key => s!"&mut global[{repr family}]({key.describe})"
  | _, _, .globalPublish family key value =>
      s!"publish[{repr family}]({key.describe}, {value.describe})"
  | _, _, .globalTake family key => s!"take[{repr family}]({key.describe})"

partial def Args.describe {ρ : ResultShape} : {Γ σs : NRow} → Args ρ Γ σs → String
  | _, _, .nil => ""
  | _, _, .cons head .nil => head.describe
  | _, _, .cons head tail => s!"{head.describe}, {tail.describe}"
end

def Function.describe (f : Function) : String :=
  s!"params={repr f.params} locals={repr f.locals} result={repr f.result} body={f.body.describe}"

end LeanerIR.Proofs.Denote

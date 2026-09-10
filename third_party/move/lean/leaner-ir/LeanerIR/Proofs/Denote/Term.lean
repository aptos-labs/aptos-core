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

/-- A typed projection path into a value: through a mutable reference's
current value and into a struct's field.  It is what a place names once
its base local is known. -/
inductive Proj : NTy → NTy → Type where
  | nil {τ : NTy} : Proj τ τ
  | deref {τ σ : NTy} (rest : Proj τ σ) : Proj (.ref τ) σ
  | field {source : StructHandle} {fields : NRow} {σ τ : NTy} (x : Var fields σ)
      (rest : Proj σ τ) : Proj (.struct source fields) τ

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

/-- The component a path reaches. -/
def Proj.get : {τ σ : NTy} → Proj τ σ → τ.carrier → σ.carrier
  | _, _, .nil, value => value
  | _, _, .deref rest, value => rest.get value.2
  | _, _, .field x rest, value => rest.get (x.select value)

/-- The value with the component a path reaches replaced. -/
def Proj.set : {τ σ : NTy} → Proj τ σ → σ.carrier → τ.carrier → τ.carrier
  | _, _, .nil, replacement, _ => replacement
  | _, _, .deref rest, replacement, value => (value.1, rest.set replacement value.2)
  | _, _, .field x rest, replacement, value =>
      x.update (rest.set replacement (x.select value)) value

@[simp] theorem Proj.get_nil {τ : NTy} (value : τ.carrier) : (Proj.nil : Proj τ τ).get value = value :=
  rfl
@[simp] theorem Proj.get_deref {τ σ : NTy} (rest : Proj τ σ) (value : (NTy.ref τ).carrier) :
    (Proj.deref rest).get value = rest.get value.2 := rfl
@[simp] theorem Proj.get_field {source : StructHandle} {fields : NRow} {σ τ : NTy}
    (x : Var fields σ) (rest : Proj σ τ) (value : (NTy.struct source fields).carrier) :
    (Proj.field x rest).get value = rest.get (x.select value) := rfl
@[simp] theorem Proj.set_nil {τ : NTy} (replacement value : τ.carrier) :
    (Proj.nil : Proj τ τ).set replacement value = replacement := rfl
@[simp] theorem Proj.set_deref {τ σ : NTy} (rest : Proj τ σ) (replacement : σ.carrier)
    (value : (NTy.ref τ).carrier) :
    (Proj.deref rest).set replacement value = (value.1, rest.set replacement value.2) := rfl
@[simp] theorem Proj.set_field {source : StructHandle} {fields : NRow} {σ τ : NTy}
    (x : Var fields σ) (rest : Proj σ τ) (replacement : τ.carrier)
    (value : (NTy.struct source fields).carrier) :
    (Proj.field x rest).set replacement value =
      x.update (rest.set replacement (x.select value)) value := rfl

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
  | lit {Γ : NRow} {τ : NTy} (value : τ.carrier) : Term ρ Γ τ
  /-- A read of a local slot. -/
  | var {Γ : NRow} {τ : NTy} (x : Var Γ τ) : Term ρ Γ τ
  /-- Checked integer arithmetic. -/
  | checked {Γ : NRow} {width : Nat} {signed : Bool} (op : CheckedOp)
      (failure : ThrowKind) (left right : Term ρ Γ (.int width signed)) :
      Term ρ Γ (.int width signed)
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
  /-- Replace the current value of the mutable reference a local holds. -/
  | mutate {Γ : NRow} {τ : NTy} (x : Var Γ (.ref τ)) (value : Term ρ Γ τ) : Term ρ Γ .unit
  /-- Read the component of a local a path reaches. -/
  | readPlace {Γ : NRow} {τ σ : NTy} (x : Var Γ τ) (path : Proj τ σ) : Term ρ Γ σ
  /-- Write the component of a local a path reaches. -/
  | writePlace {Γ : NRow} {τ σ : NTy} (x : Var Γ τ) (path : Proj τ σ) (value : Term ρ Γ σ) :
      Term ρ Γ .unit
  /-- Borrow the component of a local a path reaches: a fresh loan owning
  its value.  The lender keeps its value in place of the runtime's hole;
  the loan's death writes the borrow's current back through the path. -/
  | borrowPlace {Γ : NRow} {τ σ : NTy} (x : Var Γ τ) (path : Proj τ σ) : Term ρ Γ (.ref σ)
  /-- The death of a loan bound to a local: its current value returns to
  the lender's component. -/
  | writeBack {Γ : NRow} {τ σ : NTy} (x : Var Γ τ) (path : Proj τ σ) (loan : Var Γ (.ref σ)) :
      Term ρ Γ .unit
  /-- Evaluate a value, then an effect, and keep the value. -/
  | seqAfter {Γ : NRow} {τ : NTy} (value : Term ρ Γ τ) (effect : Term ρ Γ .unit) : Term ρ Γ τ
  /-- The resource of a family at a key; aborts when none is published. -/
  | globalRead {Γ : NRow} {κ τ : NTy} (family : Family) (key : Term ρ Γ κ) : Term ρ Γ τ
  /-- Whether a resource of a family is published at a key. -/
  | globalContains {Γ : NRow} {κ : NTy} (family : Family) (key : Term ρ Γ κ) : Term ρ Γ .bool
  /-- A mutable borrow of a published resource: a fresh loan owning its
  value, the runtime's hole in the slot, and the loan registered at its key. -/
  | globalBorrow {Γ : NRow} {κ τ : NTy} (family : Family) (key : Term ρ Γ κ) : Term ρ Γ (.ref τ)
  /-- Publish a resource at a key; aborts when one is already there. -/
  | globalPublish {Γ : NRow} {κ τ : NTy} (family : Family) (key : Term ρ Γ κ)
      (value : Term ρ Γ τ) : Term ρ Γ .unit
  /-- Take a resource from a key; aborts when none is published. -/
  | globalTake {Γ : NRow} {κ τ : NTy} (family : Family) (key : Term ρ Γ κ) : Term ρ Γ τ
  /-- The death of a global loan bound to a local: its current value is
  written back at the registered key, and the registration retired. -/
  | publishBack {Γ : NRow} {τ : NTy} (loan : Var Γ (.ref τ)) : Term ρ Γ .unit

/-- An argument row, evaluated left to right.  A reborrowed argument lends
the component of a local to the callee under a fresh loan; the callee's
exported write-back settles it after the call. -/
inductive Args (ρ : ResultShape) : NRow → NRow → Type where
  | nil {Γ : NRow} : Args ρ Γ .nil
  | cons {Γ : NRow} {σ : NTy} {σs : NRow} (head : Term ρ Γ σ) (tail : Args ρ Γ σs) :
      Args ρ Γ (.cons σ σs)
  | reborrow {Γ : NRow} {τ σ : NTy} {σs : NRow} (x : Var Γ τ) (path : Proj τ σ)
      (tail : Args ρ Γ σs) : Args ρ Γ (.cons (.ref σ) σs)
end

/-- Settle the reborrowed arguments of a call from the callee's exports:
each lender receives the native value the export encodes. -/
def Args.settle {ρ : ResultShape} {Γ : NRow} : {σs : NRow} → Args ρ Γ σs → HList σs →
    List (Nat × RuntimeValue) → HEnv Γ → Comp (HEnv Γ)
  | _, .nil, _, _, env => Spec.pure env
  | _, .cons _ tail, values, exports, env => tail.settle values.2 exports env
  | _, @Args.reborrow _ _ _ σ _ x path tail, values, exports, env =>
      Spec.bind (tail.settle values.2 exports env) fun env =>
        match x.get env, exportedRaw? values.1.1 exports with
        | some current, some raw =>
            Spec.bind (decodeOr σ.codec raw) fun replacement =>
              Spec.pure (x.set (path.set replacement current) env)
        | _, _ => Spec.bottom

@[simp] theorem Args.settle_nil {ρ : ResultShape} {Γ : NRow} (values : HList .nil)
    (exports : List (Nat × RuntimeValue)) (env : HEnv Γ) :
    (Args.nil (ρ := ρ)).settle values exports env = Spec.pure env := rfl
@[simp] theorem Args.settle_cons {ρ : ResultShape} {Γ : NRow} {σ : NTy} {σs : NRow}
    (head : Term ρ Γ σ) (tail : Args ρ Γ σs) (values : HList (.cons σ σs))
    (exports : List (Nat × RuntimeValue)) (env : HEnv Γ) :
    (Args.cons head tail).settle values exports env = tail.settle values.2 exports env := rfl
@[simp] theorem Args.settle_reborrow {ρ : ResultShape} {Γ : NRow} {τ σ : NTy} {σs : NRow}
    (x : Var Γ τ) (path : Proj τ σ) (tail : Args ρ Γ σs) (values : HList (.cons (.ref σ) σs))
    (exports : List (Nat × RuntimeValue)) (env : HEnv Γ) :
    (Args.reborrow x path tail).settle values exports env =
      Spec.bind (tail.settle values.2 exports env) fun env =>
        match x.get env, exportedRaw? values.1.1 exports with
        | some current, some raw =>
            Spec.bind (decodeOr σ.codec raw) fun replacement =>
              Spec.pure (x.set (path.set replacement current) env)
        | _, _ => Spec.bottom := rfl

/-- The mutable-reference parameters of a function, as the slots whose
loans it exports at exit, in parameter order. -/
inductive Exports (Γ : NRow) : Type where
  | nil : Exports Γ
  | cons {τ : NTy} (x : Var Γ (.ref τ)) (rest : Exports Γ) : Exports Γ

/-- The write-backs a returning function exports: each exported slot's
loan with its current value. -/
def paramExports {Γ : NRow} : Exports Γ → HEnv Γ → Option (List (Nat × RuntimeValue))
  | .nil, _ => some []
  | @Exports.cons _ τ x rest, env =>
      match x.get env, paramExports rest env with
      | some value, some tail => some ((value.1, τ.encode value.2) :: tail)
      | _, _ => none

@[simp] theorem paramExports_nil {Γ : NRow} (env : HEnv Γ) : paramExports Exports.nil env = some [] :=
  rfl
@[simp] theorem paramExports_cons {Γ : NRow} {τ : NTy} (x : Var Γ (.ref τ)) (rest : Exports Γ)
    (env : HEnv Γ) :
    paramExports (Exports.cons x rest) env =
      match x.get env, paramExports rest env with
      | some value, some tail => some ((value.1, τ.encode value.2) :: tail)
      | _, _ => none := rfl

/-- Append exports to the pending set, one push each, as the runtime does. -/
def pushExports (pending : Array (Nat × RuntimeValue)) : List (Nat × RuntimeValue) →
    Array (Nat × RuntimeValue)
  | [] => pending
  | entry :: rest => pushExports (pending.push entry) rest

@[simp] theorem pushExports_nil (pending : Array (Nat × RuntimeValue)) :
    pushExports pending [] = pending := rfl
@[simp] theorem pushExports_cons (pending : Array (Nat × RuntimeValue))
    (entry : Nat × RuntimeValue) (rest : List (Nat × RuntimeValue)) :
    pushExports pending (entry :: rest) = pushExports (pending.push entry) rest := rfl

/-- Export the write-backs of the parameters, then continue. -/
def exportThen {Γ : NRow} (exports : Exports Γ) (env : HEnv Γ) (next : Comp α) : Comp α :=
  match paramExports exports env with
  | some [] => next
  | some exports =>
      Spec.bind (Spec.modify fun state => { state with pending := pushExports state.pending exports })
        fun _ => next
  | none => Spec.bottom

/-- The typed big-step meaning of a callee at the compiler's codecs.  A
call denotes exactly this; a caller reasons about it through the callee's
published contract. -/
def calleeMeaning (unit : LeanerIR.Validation.ExecutableUnit) (handle : FunctionHandle) (σs : NRow)
    (shape : ResultShape) (values : HList σs) : Comp shape.carrier :=
  typedFunction (hlistCodec σs) (resultCodec shape) (functionSpec unit handle) values

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
def Term.denote (unit : LeanerIR.Validation.ExecutableUnit) {ρ : ResultShape} : {Γ : NRow} → {τ : NTy} →
    Term ρ Γ τ → HEnv Γ → Comp (Flow ρ Γ τ.carrier)
  | _, _, .lit value, env => Spec.pure (.value value env)
  | _, _, .var x, env =>
      match x.get env with
      | some value => Spec.pure (.value value env)
      | none => Spec.bottom
  | _, _, .checked op failure left right, env =>
      Flow.bind (left.denote unit env) fun l env =>
        Flow.bind (right.denote unit env) fun r env =>
          Spec.bind (op.run failure l r) fun value => Spec.pure (.value value env)
  | _, _, .compare op left right, env =>
      Flow.bind (left.denote unit env) fun l env =>
        Flow.bind (right.denote unit env) fun r env =>
          Spec.pure (.value (op.decide l.val r.val) env)
  | _, _, .equal negated left right, env =>
      Flow.bind (left.denote unit env) fun l env =>
        Flow.bind (right.denote unit env) fun r env =>
          Spec.pure (.value (if negated then !NTy.eqb _ l r else NTy.eqb _ l r) env)
  | _, _, .not operand, env =>
      Flow.bind (operand.denote unit env) fun b env => Spec.pure (.value (!(b : Bool)) env)
  | _, _, .logical conjunction left right, env =>
      Flow.bind (left.denote unit env) fun l env =>
        Flow.bind (right.denote unit env) fun r env =>
          Spec.pure (.value (if conjunction then (l : Bool) && r else (l : Bool) || r) env)
  | _, _, .bitwise op left right, env =>
      Flow.bind (left.denote unit env) fun l env =>
        Flow.bind (right.denote unit env) fun r env => Spec.pure (.value (op.run l r) env)
  | _, _, .shift left failure value distance, env =>
      Flow.bind (value.denote unit env) fun v env =>
        Flow.bind (distance.denote unit env) fun d env =>
          Spec.bind (if left then checkedShiftLeft failure v d
            else checkedShiftRight failure v d) fun result => Spec.pure (.value result env)
  | _, .int width' signed', .cast failure value, env =>
      Flow.bind (value.denote unit env) fun v env =>
        Spec.bind (checkedInt failure width' signed' v.val) fun result =>
          Spec.pure (.value result env)
  | _, _, .ite condition thenBranch elseBranch, env =>
      Flow.bind (condition.denote unit env) fun b env =>
        if (b : Bool) then thenBranch.denote unit env else elseBranch.denote unit env
  | _, _, .let_ x value body, env =>
      Flow.bind (value.denote unit env) fun v env => body.denote unit (x.set v env)
  | _, _, .drop value body, env =>
      Flow.bind (value.denote unit env) fun _ env => body.denote unit env
  | _, _, .assign x value, env =>
      Flow.bind (value.denote unit env) fun v env => Spec.pure (.value () (x.set v env))
  | _, _, .const value, env =>
      Spec.bind (value.denote unit ()) fun flow => Spec.pure (flow.rebase env)
  | _, _, .throw0 kind, _ => Spec.abort (kind, #[])
  | _, _, .throw1 kind code, env =>
      Flow.bind (code.denote unit env) fun value _ =>
        Spec.abort (kind, #[NTy.encode _ value])
  | _, _, .return_ value, env =>
      Flow.bind (value.denote unit env) fun v env => Spec.pure (.return_ (ρ.ofBody v) env)
  | _, _, .break_ nest, env => Spec.pure (.break_ nest env)
  | _, _, .continue_ nest, env => Spec.pure (.continue_ nest env)
  | _, _, .loop site body, env =>
      loopAt site (fun recurse env => Spec.bind (body.denote unit env) (Flow.iterate recurse)) env
  | _, _, .call handle shape arguments, env =>
      Flow.bind (arguments.denote unit env) fun values env =>
        Spec.bind Spec.get fun entry =>
          Spec.bind (calleeMeaning unit handle _ shape values) fun result =>
            Spec.bind Spec.get fun exit =>
              Spec.bind (arguments.settle values (exportsAfter entry.pending exit.pending) env)
                fun env =>
                  Spec.bind (Spec.modify fun state => { state with pending := entry.pending })
                    fun _ => Spec.pure (.value (shape.toBody result) env)
  | _, _, .tuple elements, env =>
      Flow.bind (elements.denote unit env) fun values env => Spec.pure (.value values env)
  | _, _, .pack _ fields, env =>
      Flow.bind (fields.denote unit env) fun values env => Spec.pure (.value values env)
  | _, _, .variant _ _ choice fields, env =>
      Flow.bind (fields.denote unit env) fun values env =>
        Spec.pure (.value (choice.inject values) env)
  | _, _, .field x value, env =>
      Flow.bind (value.denote unit env) fun v env => Spec.pure (.value (x.select v) env)
  | _, _, .isVariant tests value, env =>
      Flow.bind (value.denote unit env) fun v env =>
        Spec.pure (.value (namedIn (variantName _ _ v) tests) env)
  | _, _, .payload choices value, env =>
      Flow.bind (value.denote unit env) fun v env =>
        match choices.select? v with
        | some field => Spec.pure (.value field env)
        | none => Spec.bottom
  | _, _, .letRow targets value body, env =>
      Flow.bind (value.denote unit env) fun values env => body.denote unit (targets.set values env)
  | _, _, .letFields targets value body, env =>
      Flow.bind (value.denote unit env) fun values env => body.denote unit (targets.set values env)
  | _, _, .deref value, env =>
      Flow.bind (value.denote unit env) fun v env => Spec.pure (.value v.2 env)
  | _, _, .mutate x value, env =>
      Flow.bind (value.denote unit env) fun v env =>
        match x.get env with
        | some current => Spec.pure (.value () (x.set (current.1, v) env))
        | none => Spec.bottom
  | _, _, .readPlace x path, env =>
      match x.get env with
      | some value => Spec.pure (.value (path.get value) env)
      | none => Spec.bottom
  | _, _, .writePlace x path value, env =>
      Flow.bind (value.denote unit env) fun v env =>
        match x.get env with
        | some current => Spec.pure (.value () (x.set (path.set v current) env))
        | none => Spec.bottom
  | _, _, .borrowPlace x path, env =>
      match x.get env with
      | some value =>
          Spec.bind mintLoan fun loan => Spec.pure (.value (loan, path.get value) env)
      | none => Spec.bottom
  | _, _, .writeBack x path loan, env =>
      match x.get env, loan.get env with
      | some current, some borrow => Spec.pure (.value () (x.set (path.set borrow.2 current) env))
      | _, _ => Spec.bottom
  | _, _, .seqAfter value effect, env =>
      Flow.bind (value.denote unit env) fun v env =>
        Flow.bind (effect.denote unit env) fun _ env => Spec.pure (.value v env)
  | _, τ, @Term.globalRead _ _ κ _ family key, env =>
      Flow.bind (key.denote unit env) fun k env =>
        Spec.bind Spec.get fun state =>
          match state.globals.lookup (family.key (κ.encode k)) with
          | none => Spec.abort (.abort, #[])
          | some raw => Spec.bind (decodeOr τ.codec raw) fun value => Spec.pure (.value value env)
  | _, _, @Term.globalContains _ _ κ family key, env =>
      Flow.bind (key.denote unit env) fun k env =>
        Spec.bind Spec.get fun state =>
          Spec.pure (.value (state.globals.lookup (family.key (κ.encode k))).isSome env)
  | _, .ref τ, @Term.globalBorrow _ _ κ _ family key, env =>
      Flow.bind (key.denote unit env) fun k env =>
        Spec.bind Spec.get fun state =>
          match state.globals.lookup (family.key (κ.encode k)) with
          | none => Spec.abort (.abort, #[])
          | some raw =>
              Spec.bind (decodeOr τ.codec raw) fun value =>
                Spec.bind mintLoan fun loan =>
                  Spec.bind (Spec.modify fun state =>
                    { state with
                      globals := state.globals.insert (family.key (κ.encode k)) (.loanHole loan)
                      globalLoans := (loan, family.key (κ.encode k)) :: state.globalLoans })
                    fun _ => Spec.pure (.value (loan, value) env)
  | _, _, @Term.globalPublish _ _ κ τ family key value, env =>
      Flow.bind (key.denote unit env) fun k env =>
        Flow.bind (value.denote unit env) fun v env =>
          Spec.bind Spec.get fun state =>
            match state.globals.lookup (family.key (κ.encode k)) with
            | some _ => Spec.abort (.abort, #[])
            | none =>
                Spec.bind (Spec.modify fun state =>
                  { state with globals := state.globals.insert (family.key (κ.encode k)) (τ.encode v) })
                  fun _ => Spec.pure (.value () env)
  | _, τ, @Term.globalTake _ _ κ _ family key, env =>
      Flow.bind (key.denote unit env) fun k env =>
        Spec.bind Spec.get fun state =>
          match state.globals.lookup (family.key (κ.encode k)) with
          | none => Spec.abort (.abort, #[])
          | some raw =>
              Spec.bind (decodeOr τ.codec raw) fun value =>
                Spec.bind (Spec.modify fun state =>
                  { state with globals := state.globals.erase (family.key (κ.encode k)) })
                  fun _ => Spec.pure (.value value env)
  | _, _, @Term.publishBack _ _ τ loan, env =>
      match loan.get env with
      | some borrow =>
          Spec.bind Spec.get fun state =>
            match LeanerIR.SemanticOperations.globalLoanKeyIn? state.globalLoans borrow.1 with
            | none => Spec.bottom
            | some key =>
                Spec.bind (Spec.modify fun state =>
                  { state with
                    globals := state.globals.insert key (τ.encode borrow.2)
                    globalLoans := LeanerIR.SemanticOperations.removeGlobalLoan state.globalLoans borrow.1 })
                  fun _ => Spec.pure (.value () env)
      | none => Spec.bottom

/-- The denotation of an argument row: its values with the locals after it. -/
def Args.denote (unit : LeanerIR.Validation.ExecutableUnit) {ρ : ResultShape} : {Γ σs : NRow} → Args ρ Γ σs →
    HEnv Γ → Comp (Flow ρ Γ (HList σs))
  | _, _, .nil, env => Spec.pure (.value () env)
  | _, _, .cons head tail, env =>
      Flow.bind (head.denote unit env) fun value env =>
        Flow.bind (tail.denote unit env) fun values env =>
          Spec.pure (.value (value, values) env)
  | _, _, .reborrow x path tail, env =>
      match x.get env with
      | some value =>
          Spec.bind mintLoan fun loan =>
            Flow.bind (tail.denote unit env) fun values env =>
              Spec.pure (.value ((loan, path.get value), values) env)
      | none => Spec.bottom
end

/-- A compiled function: its parameter and remaining local types, its
declared result shape, and its body over the complete local row. -/
structure Function where
  params : NRow
  locals : NRow
  result : ResultShape
  body : Term result (params ++ locals) result.bodyType
  /-- The mutable-reference parameters, exported at exit. -/
  exports : Exports (params ++ locals)

/-- A function without reference parameters exports nothing. -/
@[simp] theorem exportThen_nil {Γ : NRow} (env : HEnv Γ) (next : Comp α) :
    exportThen (Γ := Γ) Exports.nil env next = next := rfl

/-- The function result a body outcome denotes, after the parameters'
write-backs are exported.  Loop control that escapes the body has no
meaning, as the runtime's `finishControl?` has none. -/
def ResultShape.finish {Γ : NRow} (exports : Exports Γ) (shape : ResultShape) :
    Flow shape Γ shape.bodyType.carrier → Comp shape.carrier
  | .value value env => exportThen exports env (Spec.pure (shape.ofBody value))
  | .return_ result env => exportThen exports env (Spec.pure result)
  | .break_ _ _ => Spec.bottom
  | .continue_ _ _ => Spec.bottom

/-- The denotation of a function on native arguments. -/
def Function.denote (unit : LeanerIR.Validation.ExecutableUnit) (f : Function) (args : HList f.params) :
    Comp f.result.carrier :=
  Spec.bind (f.body.denote unit (initialEnv f.params f.locals args))
    (f.result.finish f.exports)

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

/-- Loop verification from an invariant over the locals: it holds at
entry, and one iteration under it is correct whenever the next iteration
is assumed correct under it.  Partial correctness; the loop leaves the
store as it found it. -/
theorem wp_loopAt {ρ : ResultShape} {Γ : NRow} (site : Nat)
    (iteration : (HEnv Γ → Comp (Flow ρ Γ Unit)) → HEnv Γ → Comp (Flow ρ Γ Unit))
    (entry : HEnv Γ) (invariant : HEnv Γ → Prop)
    (ensures : Flow ρ Γ Unit → RuntimeState → Prop) (aborts : Failure → Prop)
    (initial : RuntimeState)
    (entryHolds : invariant entry)
    (step : ∀ recursive : HEnv Γ → Comp (Flow ρ Γ Unit),
      (∀ env, invariant env → wp (recursive env) ensures aborts initial) →
      ∀ env, invariant env → wp (iteration recursive env) ensures aborts initial) :
    wp (loopAt site iteration entry) ensures aborts initial :=
  wp_withInvariant_fix_frame (invariant := fun env _ => invariant env) entryHolds
    fun recursive hypothesis env holds => step recursive hypothesis env holds

@[simp] theorem finish_value {Γ : NRow} (exports : Exports Γ) (shape : ResultShape)
    (value : shape.bodyType.carrier) (env : HEnv Γ) :
    shape.finish exports (.value value env) = exportThen exports env (Spec.pure (shape.ofBody value)) :=
  rfl
@[simp] theorem finish_return {Γ : NRow} (exports : Exports Γ) (shape : ResultShape)
    (result : shape.carrier) (env : HEnv Γ) :
    shape.finish exports (.return_ result env) = exportThen exports env (Spec.pure result) := rfl
@[simp] theorem finish_break {Γ : NRow} (exports : Exports Γ) (shape : ResultShape) (nest : Nat)
    (env : HEnv Γ) : shape.finish exports (.break_ nest env) = Spec.bottom := rfl
@[simp] theorem finish_continue {Γ : NRow} (exports : Exports Γ) (shape : ResultShape) (nest : Nat)
    (env : HEnv Γ) : shape.finish exports (.continue_ nest env) = Spec.bottom := rfl

attribute [lir_denote] Term.denote Args.denote Function.denote ResultShape.ofBody
  ResultShape.toBody
  ResultShape.finish Flow.iterate Flow.rebase Flow.bind_pure_value Flow.bind_pure_return
  Flow.bind_pure_break Flow.bind_pure_continue Flow.bind_abort wp_flowBind finish_value finish_return finish_break
  finish_continue exportThen_nil exportThen paramExports_nil paramExports_cons pushExports_nil
  pushExports_cons Option.bind_some Option.bind_none Proj.get_nil Proj.get_deref Proj.get_field
  Proj.set_nil Proj.set_deref Proj.set_field Var.update_here Var.update_there Args.settle_nil
  Args.settle_cons Args.settle_reborrow exportsAfter_self exportsAfter_push exportsAfter_push_push
  NTy.decode?_encode NTy.encode_ref Which.inject_here Which.inject_there Which.project?_here_inl
  Which.project?_here_inr Which.project?_there_inl Which.project?_there_inr variantName_inl
  variantName_inr Choices.select?_nil Choices.select?_cons Vars.set_nil Vars.set_cons_none
  Vars.set_cons_some Var.select_here Var.select_there NTy.eqb_tuple NTy.eqb_struct NTy.eqb_enum
  rowEqb_nil rowEqb_cons variantEqb_inl_inl variantEqb_inr_inr variantEqb_inl_inr
  variantEqb_inr_inl namedIn_nil namedIn_cons NTy.encode_tuple NTy.encode_struct
  NTy.encode_enum_inl NTy.encode_enum_inr HList.encode_nil HList.encode_cons
  String.reduceBEq String.reduceEq String.reduceBNe String.reduceNe Bool.true_or Bool.false_or
  Bool.or_true Bool.or_false Bool.true_and Bool.false_and Bool.and_true Bool.and_false

/-- A readable rendering of a native value. -/
def describeValue : {τ : NTy} → τ.carrier → String
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
  | .ref _, value => s!"&mut#{value.1}"

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
  | _, _, .writeBack x _ loan => s!"(local{x.index}.path <- local{loan.index})"
  | _, _, .seqAfter value effect => s!"({value.describe} then {effect.describe})"
  | _, _, .globalRead family key => s!"global[{repr family}]({key.describe})"
  | _, _, .globalContains family key => s!"exists[{repr family}]({key.describe})"
  | _, _, .globalBorrow family key => s!"&mut global[{repr family}]({key.describe})"
  | _, _, .globalPublish family key value =>
      s!"publish[{repr family}]({key.describe}, {value.describe})"
  | _, _, .globalTake family key => s!"take[{repr family}]({key.describe})"
  | _, _, .publishBack loan => s!"(global <- local{loan.index})"

partial def Args.describe {ρ : ResultShape} : {Γ σs : NRow} → Args ρ Γ σs → String
  | _, _, .nil => ""
  | _, _, .cons head .nil => head.describe
  | _, _, .cons head tail => s!"{head.describe}, {tail.describe}"
  | _, _, .reborrow x _ .nil => s!"&mut local{x.index}.path"
  | _, _, .reborrow x _ tail => s!"&mut local{x.index}.path, {tail.describe}"
end

def Function.describe (f : Function) : String :=
  s!"params={repr f.params} locals={repr f.locals} result={repr f.result} body={f.body.describe}"

end LeanerIR.Proofs.Denote

-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.ReturnedStorage
import VerexIR.Proofs.Denote.Interpretation
import VerexIR.Proofs.Denote.LoanCorrespondence
import VerexIR.Semantics.Frames
import VerexIR.Semantics.ScopedMethods

/-!
# Typed terms and their denotation

A `Term ρ Γ τ` is the shallow, intrinsically typed form of a validated VIR
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

namespace VerexIR.Proofs.Denote

attribute [vir_denote_norm] SpecSliceRef.currentVector_size SpecSliceRef.finalVector_size
  SpecSliceRef.settled_currentVector SpecSliceRef.settled_finalVector

section LocalOperations
variable [Carriers] {State : Type}

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
    (action : Spec State Failure (Flow ρ Γ α)) (next : α → HEnv Γ → Spec State Failure (Flow ρ Γ β)) :
    Spec State Failure (Flow ρ Γ β) :=
  Spec.bind action fun (flow : Flow ρ Γ α) =>
    match flow with
    | .value v env => next v env
    | .return_ result env => Spec.pure (.return_ result env)
    | .break_ nest env => Spec.pure (.break_ nest env)
    | .continue_ nest env => Spec.pure (.continue_ nest env)

/-- The component of a plain row at a position: a struct's field, or a
variant's, which a certified value holds plain (`NTy.carrier`). -/
def Var.selectPlain : {Γ : NRow} → {τ : NTy} → Var Γ τ → HListPlain Γ → τ.plain
  | _, _, .here, values => values.1
  | _, _, .there rest, values => rest.selectPlain values.2

/-- Selecting a component of a forgotten row is forgetting the selected
component. -/
theorem Var.selectPlain_forget : {Γ : NRow} → {τ : NTy} → (x : Var Γ τ) →
    (values : HList Γ) → x.selectPlain (HList.forget Γ values) = NTy.forget τ (x.select values)
  | _, _, .here, _ => rfl
  | _, _, .there rest, values => rest.selectPlain_forget values.2

@[simp] theorem Var.selectPlain_here {Γ : NRow} {τ : NTy} (values : HListPlain (.cons τ Γ)) :
    (Var.here : Var (.cons τ Γ) τ).selectPlain values = values.1 := rfl
@[simp] theorem Var.selectPlain_there {Γ : NRow} {σ τ : NTy} (rest : Var Γ τ)
    (values : HListPlain (.cons σ Γ)) :
    (Var.there rest).selectPlain values = rest.selectPlain values.2 := rfl

/-- A component of a row whose data invariants hold at every depth has its
own: the certificate of an owner is the certificate of everything in it
(`doc/designs/data-invariants.md`). -/
theorem Var.selectPlain_deepHolds : {Γ : NRow} → {τ : NTy} → (x : Var Γ τ) →
    (values : HListPlain Γ) → HListPlain.deepHolds Γ values →
    NTy.deepHolds τ (x.selectPlain values)
  | _, _, .here, _, held => held.1
  | _, _, .there rest, values, held => rest.selectPlain_deepHolds values.2 held.2

/-! ## Certifying a plain value

A struct's or a variant's carrier is its plain row certified by the data
invariants at every depth, and the current of a mutable reference is plain
(`NTy.carrier`, `doc/designs/data-invariants.md`).  One operation
carries a plain value to its certified form, and one step of the denotation
owes it: a pack, a write into a certified owner, the end of a mutable loan,
and a value copied out of an open current.
-/

/-- Certify a plain value: its data invariants at every depth are owed here,
and what follows sees the certified value (`Spec.certified`,
`wp_certified`).  Only a value made, a value written into a certified owner,
the current at a loan's death and a value copied out of an open current go
through this; a value read out of a certified owner is certified by the
owner's certificate and owes nothing (`NTy.ofDeepHolds`). -/
noncomputable def certifying (τ : NTy) (value : τ.plain) : Spec State Failure τ.carrier :=
  Interpretation.certifying ⟨inferInstance, State⟩ τ value

@[simp] theorem wp_certifying {τ : NTy} (value : τ.plain) (ensures : τ.carrier → State → Prop)
    (aborts : Failure → Prop) (initial : State) :
    wp (certifying (State := State) τ value) ensures aborts initial ↔
      NTy.deepHolds τ value ∧ ∀ holds, ensures (NTy.ofDeepHolds τ value holds) initial := by
  simp only [certifying, Interpretation.certifying, wp_certified]

/-- What a certifying step certifies, which the report names: a value made,
a value written into a certified owner or the current at a loan's death, or
a value copied out of an open current
(`doc/designs/data-invariants.md`). -/
inductive CertifyKind where
  | construction
  | mutationEnd
  /-- Where a mutable parameter's loan ends: the function's exit
  (`Mutables.resolve`), which the report names apart from a mutation end
  inside the body. -/
  | exit
  | copyOut
  /-- Where a mutable reference is handed over to a call: the callee
  assumes the data invariants of its current at entry
  (`Term.assumeCurrent`), so the caller owes them here
  (`doc/designs/data-invariants.md`, "A call boundary"). -/
  | handOver
  deriving DecidableEq, Repr, Inhabited

/-- `certifying` at the site the obligation is reported at, with what the
site does: the solver cuts here (`wp_certifyingAt`) and names the
obligation after the site and the kind, as it named the one a
`constructedAt` marker carried. -/
noncomputable def certifyingAt (_site : Nat) (_kind : CertifyKind) (τ : NTy)
    (value : τ.plain) : Spec State Failure τ.carrier :=
  certifying τ value

@[simp] theorem wp_certifyingAt {τ : NTy} (site : Nat) (kind : CertifyKind) (value : τ.plain)
    (ensures : τ.carrier → State → Prop) (aborts : Failure → Prop)
    (initial : State) :
    wp (certifyingAt (State := State) site kind τ value) ensures aborts initial ↔
      NTy.deepHolds τ value ∧ ∀ holds, ensures (NTy.ofDeepHolds τ value holds) initial :=
  wp_certifying value ensures aborts initial

/-- The cut the solver takes at a certifying step, in the order the step
reads the step's arguments: the obligation, and the continuation under the
certificate (`dataInvariantStep`). -/
theorem wp_certifyingAt_intro (site : Nat) (kind : CertifyKind) (τ : NTy) (value : τ.plain)
    (ensures : τ.carrier → State → Prop) (aborts : Failure → Prop)
    (initial : State) (holds : NTy.deepHolds τ value)
    (continues : ∀ held : NTy.deepHolds τ value,
      ensures (NTy.ofDeepHolds τ value held) initial) :
    wp (certifyingAt (State := State) site kind τ value) ensures aborts initial :=
  (wp_certifyingAt site kind value ensures aborts initial).mpr ⟨holds, continues⟩

/-- The certifying step of a struct: the obligation is the declaration's
data invariants at every depth and the value is the certified row, the form
the solver walks at a pack and at a write into a certified owner, where the
report names the clauses (`doc/designs/data-invariants.md`). -/
@[simp] theorem certifying_struct (source : StructHandle) (arguments fields : NRow)
    (row : HListPlain fields) :
    certifying (State := State) (.struct source arguments fields) row =
      Spec.certified (Invariant := NTy.deepHolds (.struct source arguments fields) row)
        fun holds => ⟨row, holds⟩ := rfl

/-- The certifying step of a variant, with its data invariants as the
obligation. -/
@[simp] theorem certifying_enum (source : StructHandle) (arguments : NRow)
    (names : List String) (rows : NRows) (distinct : names.Nodup)
    (value : variantPlain names rows) :
    certifying (State := State) (.enum source arguments names rows distinct) value =
      Spec.certified
        (Invariant := NTy.deepHolds (.enum source arguments names rows distinct) value)
        fun holds => ⟨value, holds⟩ := rfl

/-- A certifying step whose invariant holds of every value of the type and
whose certificate the certified value does not read: it owes nothing, so it
is the pure step of that value.  Stated for the types that carry no
certificate below, as `NTy.ofDeepHolds` is stated per constructor. -/
theorem certifying_eq_pure {τ : NTy} {value : τ.plain} {certified : τ.carrier}
    (holds : NTy.deepHolds τ value)
    (built : ∀ held, NTy.ofDeepHolds τ value held = certified) :
    certifying (State := State) τ value = Spec.pure certified := by
  simp only [certifying, Interpretation.certifying, Spec.certified, Spec.pure, Spec.mk.injEq]
  refine ⟨?_, trivial, ?_⟩
  · funext initial result final
    exact propext ⟨fun ⟨held, result_eq, final_eq⟩ => ⟨result_eq.trans (built held), final_eq⟩,
      fun ⟨result_eq, final_eq⟩ => ⟨holds, result_eq.trans (built holds).symm, final_eq⟩⟩
  · funext _
    exact propext ⟨fun fails => fails holds, False.elim⟩

/-! A certifying step at a type that carries no certificate — a scalar, a
function value — is the pure step of the value it got: `NTy.deepHolds` is
`True` there and `NTy.ofDeepHolds` is the value.  The denotation emits the
step uniformly, at every level of a value made or of a loan's death
(`Lender.resolve`, `Mutables.resolve`), and these equations take it out of
the walked form, so a pack or a mutation end of data that carries no
certificate costs nothing and the walk's size follows what the program uses
(`doc/designs/data-invariants.md`). -/

@[simp] theorem certifyingAt_unit (site : Nat) (kind : CertifyKind)
    (value : (NTy.unit).plain) :
    certifyingAt (State := State) site kind .unit value = Spec.pure value :=
  certifying_eq_pure trivial fun _ => rfl

@[simp] theorem certifyingAt_bool (site : Nat) (kind : CertifyKind)
    (value : (NTy.bool).plain) :
    certifyingAt (State := State) site kind .bool value = Spec.pure value :=
  certifying_eq_pure trivial fun _ => rfl

@[simp] theorem certifyingAt_int (site : Nat) (kind : CertifyKind) (width : Nat) (signed : Bool)
    (value : (NTy.int width signed).plain) :
    certifyingAt (State := State) site kind (.int width signed) value = Spec.pure value :=
  certifying_eq_pure trivial fun _ => rfl

@[simp] theorem certifyingAt_num (site : Nat) (kind : CertifyKind) (value : (NTy.num).plain) :
    certifyingAt (State := State) site kind .num value = Spec.pure value :=
  certifying_eq_pure trivial fun _ => rfl

@[simp] theorem certifyingAt_address (site : Nat) (kind : CertifyKind)
    (value : (NTy.address).plain) :
    certifyingAt (State := State) site kind .address value = Spec.pure value :=
  certifying_eq_pure trivial fun _ => rfl

@[simp] theorem certifyingAt_signer (site : Nat) (kind : CertifyKind)
    (value : (NTy.signer).plain) :
    certifyingAt (State := State) site kind .signer value = Spec.pure value :=
  certifying_eq_pure trivial fun _ => rfl

@[simp] theorem certifyingAt_string (site : Nat) (kind : CertifyKind)
    (value : (NTy.string).plain) :
    certifyingAt (State := State) site kind .string value = Spec.pure value :=
  certifying_eq_pure trivial fun _ => rfl

@[simp] theorem certifyingAt_bytes (site : Nat) (kind : CertifyKind)
    (value : (NTy.bytes).plain) :
    certifyingAt (State := State) site kind .bytes value = Spec.pure value :=
  certifying_eq_pure trivial fun _ => rfl

@[simp] theorem certifyingAt_function (site : Nat) (kind : CertifyKind)
    (parameters : NRow) (shared : Sharing) (results : NRow)
    (value : (NTy.function parameters shared results).plain) :
    certifyingAt (State := State) site kind (.function parameters shared results) value =
      Spec.pure value :=
  certifying_eq_pure trivial fun _ => rfl

/-- A write into a mutable reference's plain current owes nothing: the
prophecy it keeps is certified already. -/
@[simp] theorem NTy.deepHolds_ref_forget (referent : NTy) (current : referent.plain)
    (prophecy : referent.carrier) :
    NTy.deepHolds (.ref referent) (current, referent.forget prophecy) :=
  referent.deepHolds_forget prophecy

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

omit [Carriers] in
@[simp, vir_denote_norm] theorem Which.name_here {name : String} {names : List String} {fields : NRow}
    {rest : NRows} : (Which.here : Which (name :: names) (.cons fields rest) fields).name = name := rfl
omit [Carriers] in
@[simp, vir_denote_norm] theorem Which.name_there {name : String} {names : List String}
    {fields σs : NRow} {rest : NRows} (later : Which names rest σs) :
    (Which.there later : Which (name :: names) (.cons fields rest) σs).name = later.name := rfl

/-- Build a variant value from its fields: its row plain, as a certified
enum value holds it (`NTy.carrier`). -/
def Which.inject : {names : List String} → {rows : NRows} → {σs : NRow} →
    Which names rows σs → HListPlain σs → variantPlain names rows
  | _, _, _, .here, fields => .inl fields
  | _, _, _, .there later, fields => .inr (later.inject fields)

/-- The fields of a variant value, when it is the chosen variant. -/
def Which.project? : {names : List String} → {rows : NRows} → {σs : NRow} →
    Which names rows σs → variantPlain names rows → Option (HListPlain σs)
  | _, _, _, .here, .inl fields => some fields
  | _, _, _, .here, .inr _ => none
  | _, _, _, .there _, .inl _ => none
  | _, _, _, .there later, .inr value => later.project? value

@[simp] theorem Which.inject_here {name : String} {names : List String} {fields : NRow}
    {rest : NRows} (value : HListPlain fields) :
    (Which.here : Which (name :: names) (.cons fields rest) fields).inject value = .inl value := rfl
@[simp] theorem Which.inject_there {name : String} {names : List String} {fields σs : NRow}
    {rest : NRows} (later : Which names rest σs) (value : HListPlain σs) :
    (Which.there (name := name) (fields := fields) later).inject value = .inr (later.inject value) := rfl
@[simp] theorem Which.project?_here_inl {name : String} {names : List String} {fields : NRow}
    {rest : NRows} (value : HListPlain fields) :
    (Which.here : Which (name :: names) (.cons fields rest) fields).project? (.inl value) =
      some value := rfl
@[simp] theorem Which.project?_here_inr {name : String} {names : List String} {fields : NRow}
    {rest : NRows} (value : variantPlain names rest) :
    (Which.here : Which (name :: names) (.cons fields rest) fields).project? (.inr value) =
      none := rfl
@[simp] theorem Which.project?_there_inl {name : String} {names : List String}
    {fields σs : NRow} {rest : NRows} (later : Which names rest σs) (value : HListPlain fields) :
    (Which.there (name := name) later).project? (.inl value) = none := rfl
@[simp] theorem Which.project?_there_inr {name : String} {names : List String}
    {fields σs : NRow} {rest : NRows} (later : Which names rest σs)
    (value : variantPlain names rest) :
    (Which.there (name := name) (fields := fields) later).project? (.inr value) =
      later.project? value := rfl
@[simp] theorem variantPlainName_inl (name : String) (names : List String) (fields : NRow)
    (rest : NRows) (value : HListPlain fields) :
    variantPlainName (name :: names) (.cons fields rest) (.inl value) = name := rfl
@[simp] theorem variantPlainName_inr (name : String) (names : List String) (fields : NRow)
    (rest : NRows) (value : variantPlain names rest) :
    variantPlainName (name :: names) (.cons fields rest) (.inr value) =
      variantPlainName names rest value := rfl

/-- A one-variant enum has its declared tag without inspecting its payload. -/
@[simp, vir_denote_norm] theorem variantPlainName_singleton (name : String) (fields : NRow)
    (value : variantPlain [name] (.cons fields .nil)) :
    variantPlainName [name] (.cons fields .nil) value = name := by
  cases value with
  | inl _ => rfl
  | inr empty => cases empty

theorem NTy.encode_enum_nominal (source : StructHandle) (arguments : NRow) (names : List String)
    (rows : NRows) (distinct : names.Nodup) (value : (NTy.enum source arguments names rows distinct).carrier) :
    ∃ fields, NTy.encode (.enum source arguments names rows distinct) value =
      .nominal source (some (variantPlainName names rows value.val)) fields :=
  variantEncodePlain_nominal source names rows _ value.val

/-- The encoded fields of the variant a value holds. -/
def variantPayload : (names : List String) → (rows : NRows) → variantPlain names rows →
    Array RuntimeValue
  | _, .nil, value => nomatch value
  | [], .cons _ _, value => nomatch value
  | _ :: _, .cons _ _, .inl values => (HListPlain.encode values).toArray
  | _ :: names, .cons _ rest, .inr value => variantPayload names rest value

@[simp] theorem variantPayload_inl (name : String) (names : List String) (fields : NRow)
    (rest : NRows) (values : HListPlain fields) :
    variantPayload (name :: names) (.cons fields rest) (.inl values) =
      (HListPlain.encode values).toArray := rfl
@[simp] theorem variantPayload_inr (name : String) (names : List String) (fields : NRow)
    (rest : NRows) (value : variantPlain names rest) :
    variantPayload (name :: names) (.cons fields rest) (.inr value) =
      variantPayload names rest value := rfl

theorem variantEncode_payload (source : StructHandle) : (names : List String) → (rows : NRows) →
    (value : variantPlain names rows) →
    variantEncodePlain source names rows (rowCodecsPlain rows) value =
      .nominal source (some (variantPlainName names rows value)) (variantPayload names rows value)
  | _, .nil, value => nomatch value
  | [], .cons _ _, value => nomatch value
  | _ :: _, .cons _ _, .inl _ => rfl
  | _ :: names, .cons _ rest, .inr later => variantEncode_payload source names rest later

/-- An encoded enum value is a nominal value of its declaration, tagged with
the variant it holds and carrying that variant's encoded fields. -/
theorem NTy.encode_enum_payload (source : StructHandle) (arguments : NRow) (names : List String)
    (rows : NRows) (distinct : names.Nodup)
    (value : (NTy.enum source arguments names rows distinct).carrier) :
    NTy.encode (.enum source arguments names rows distinct) value =
      .nominal source (some (variantPlainName names rows value.val))
        (variantPayload names rows value.val) :=
  variantEncode_payload source names rows value.val

/-- An encoded enum value equated to a nominal value holds that variant. -/
theorem NTy.variantName_of_encode_enum {source owner : StructHandle} {arguments : NRow}
    {names : List String} {rows : NRows} {distinct : names.Nodup}
    {value : (NTy.enum source arguments names rows distinct).carrier} {name : String}
    {fields : Array RuntimeValue}
    (encoded : NTy.encode (.enum source arguments names rows distinct) value =
      .nominal owner (some name) fields) :
    variantPlainName names rows value.val = name := by
  obtain ⟨_, nominal⟩ := NTy.encode_enum_nominal source arguments names rows distinct value
  rw [nominal] at encoded
  injection encoded with _ variant
  exact Option.some.inj variant

/-- The variant a runtime value names; empty off a variant. -/
def variantOf : RuntimeValue → String
  | .nominal _ (some name) _ => name
  | _ => ""

omit [Carriers] in
theorem variantOf_nominal (source : StructHandle) (name : String)
    (fields : Array RuntimeValue) : variantOf (.nominal source (some name) fields) = name := rfl

omit [Carriers] in
theorem variantOf_ite (condition : Prop) [Decidable condition]
    (left right : RuntimeValue) :
    variantOf (if condition then left else right) =
      if condition then variantOf left else variantOf right := by
  split <;> rfl

/-- An encoded enum value equated to a runtime value holds the variant that
value names. -/
theorem NTy.variantName_of_encode_eq {source : StructHandle} {arguments : NRow}
    {names : List String} {rows : NRows} {distinct : names.Nodup}
    {value : (NTy.enum source arguments names rows distinct).carrier} {encoded : RuntimeValue}
    (equation : NTy.encode (.enum source arguments names rows distinct) value = encoded) :
    variantPlainName names rows value.val = variantOf encoded := by
  obtain ⟨_, nominal⟩ := NTy.encode_enum_nominal source arguments names rows distinct value
  rw [← equation, nominal]
  rfl

/-- The payload choices of a variant-field selection: for each variant that
has the field, its position in that variant's row. -/
inductive Choices (names : List String) (rows : NRows) (τ : NTy) : Type where
  | nil : Choices names rows τ
  | cons {σs : NRow} (choice : Which names rows σs) (x : Var σs τ) (rest : Choices names rows τ) :
      Choices names rows τ

/-- The chosen field of a variant value, when its variant has one: a plain
value, as a certified enum value holds its row (`NTy.carrier`). -/
def Choices.select? {names : List String} {rows : NRows} {τ : NTy} :
    Choices names rows τ → variantPlain names rows → Option τ.plain
  | .nil, _ => none
  | .cons choice x rest, value =>
      match choice.project? value with
      | some fields => some (x.selectPlain fields)
      | none => rest.select? value


@[simp] theorem Choices.select?_nil {names : List String} {rows : NRows} {τ : NTy}
    (value : variantPlain names rows) : (Choices.nil (τ := τ)).select? value = none := rfl
@[simp] theorem Choices.select?_cons {names : List String} {rows : NRows} {τ : NTy} {σs : NRow}
    (choice : Which names rows σs) (x : Var σs τ) (rest : Choices names rows τ)
    (value : variantPlain names rows) :
    (Choices.cons choice x rest).select? value =
      match choice.project? value with
      | some fields => some (x.selectPlain fields)
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
  /-- An element of a slice's current contents; no whole-vector write path exists. -/
  | sliceIndex {τ σ : NTy} (position : PlaceIndex) (rest : Proj τ σ) :
      Proj (.sliceRef τ) σ
  | sliceElement {τ σ : NTy} (position : PlaceIndex) (rest : Proj τ σ) :
      Proj (.slice τ) σ
  | field {source : StructHandle} {arguments fields : NRow} {σ τ : NTy} (x : Var fields σ)
      (rest : Proj σ τ) : Proj (.struct source arguments fields) τ
  | index {τ σ : NTy} (position : PlaceIndex) (rest : Proj τ σ) : Proj (.vector τ) σ
  /-- An array's element. -/
  | arrayIndex {τ σ : NTy} {length : Nat} (position : PlaceIndex) (rest : Proj τ σ) :
      Proj (.array τ length) σ
  /-- A tuple's element at its literal position. -/
  | element {elements : NRow} {σ τ : NTy} (x : Var elements σ) (rest : Proj σ τ) :
      Proj (.tuple elements) τ
  /-- The field the variants of an enum share at their chosen positions. -/
  | variant {source : StructHandle} {arguments : NRow} {names : List String} {rows : NRows}
      {distinct : names.Nodup} {σ τ : NTy} (choices : Choices names rows σ) (rest : Proj σ τ) :
      Proj (.enum source arguments names rows distinct) τ
  /-- A ghost slot's contents (`doc/designs/ghost-fields.md`). -/
  | ghost {τ σ : NTy} (rest : Proj τ σ) : Proj (.ghost τ) σ

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

/-- Replace the component of a plain row at a position: a struct's field, or
a variant's, which a certified value holds plain (`NTy.carrier`). -/
def Var.updatePlain : {Γ : NRow} → {τ : NTy} → Var Γ τ → τ.plain → HListPlain Γ → HListPlain Γ
  | _, _, .here, value, values => (value, values.2)
  | _, _, .there rest, value, values => (values.1, rest.updatePlain value values.2)

@[simp] theorem Var.updatePlain_here {Γ : NRow} {τ : NTy} (value : τ.plain)
    (values : HListPlain (.cons τ Γ)) :
    (Var.here : Var (.cons τ Γ) τ).updatePlain value values = (value, values.2) := rfl
@[simp] theorem Var.updatePlain_there {Γ : NRow} {σ τ : NTy} (rest : Var Γ τ) (value : τ.plain)
    (values : HListPlain (.cons σ Γ)) :
    (Var.there rest).updatePlain value values =
      (values.1, rest.updatePlain value values.2) := rfl

/-- The variant value with its chosen field replaced, when its variant has
one. -/
def Choices.update? {names : List String} {rows : NRows} {τ : NTy} :
    Choices names rows τ → τ.plain → variantPlain names rows →
    Option (variantPlain names rows)
  | .nil, _, _ => none
  | .cons choice x rest, replacement, value =>
      match choice.project? value with
      | some fields => some (choice.inject (x.updatePlain replacement fields))
      | none => rest.update? replacement value

@[simp] theorem Choices.update?_nil {names : List String} {rows : NRows} {τ : NTy}
    (replacement : τ.plain) (value : variantPlain names rows) :
    (Choices.nil (τ := τ)).update? replacement value = none := rfl
@[simp] theorem Choices.update?_cons {names : List String} {rows : NRows} {τ : NTy} {σs : NRow}
    (choice : Which names rows σs) (x : Var σs τ) (rest : Choices names rows τ)
    (replacement : τ.plain) (value : variantPlain names rows) :
    (Choices.cons choice x rest).update? replacement value =
      match choice.project? value with
      | some fields => some (choice.inject (x.updatePlain replacement fields))
      | none => rest.update? replacement value := rfl

/-- A variant value's fields are the chosen variant's exactly when the value
is that variant built from them. -/
theorem Which.project?_eq_some_iff : {names : List String} → {rows : NRows} → {σs : NRow} →
    (choice : Which names rows σs) → (value : variantPlain names rows) →
    (fields : HListPlain σs) →
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

/-- The variant of a value built by a choice is the choice's. -/
@[simp, vir_denote_norm] theorem variantPlainName_inject : {names : List String} → {rows : NRows} →
    {σs : NRow} → (choice : Which names rows σs) → (fields : HListPlain σs) →
    variantPlainName names rows (choice.inject fields) = choice.name
  | _, _, _, .here, _ => rfl
  | _, _, _, .there later, fields => variantPlainName_inject later fields

attribute [vir_denote_norm] variantPlainName_inl variantPlainName_inr

omit [Carriers] in
theorem Which.name_mem : {names : List String} → {rows : NRows} → {σs : NRow} →
    (choice : Which names rows σs) → choice.name ∈ names
  | _, _, _, .here => List.mem_cons_self
  | _, _, _, .there later => List.mem_cons_of_mem _ (Which.name_mem later)

/-- A choice projects nothing of a value exactly when the value is not of
the chosen variant, as a specification tests it, where the variant names
are distinct. -/
theorem Which.project?_eq_none_iff : {names : List String} → {rows : NRows} → {σs : NRow} →
    (choice : Which names rows σs) → (value : variantPlain names rows) → names.Nodup →
    (choice.project? value = none ↔ (variantPlainName names rows value == choice.name) = false)
  | _, _, _, .here, .inl _, _ => by simp [Which.project?, variantPlainName_inl]
  | _, _, _, .here, .inr value, distinct => by
      simp only [Which.project?, Which.name, variantPlainName_inr, true_iff]
      exact beq_eq_false_iff_ne.mpr fun h =>
        (List.nodup_cons.mp distinct).1 (h ▸ variantPlainName_mem _ _ value)
  | _, _, _, .there later, .inl _, distinct => by
      simp only [Which.project?, Which.name, variantPlainName_inl, true_iff]
      exact beq_eq_false_iff_ne.mpr fun h => (List.nodup_cons.mp distinct).1 (h ▸ Which.name_mem later)
  | _, _, _, .there later, .inr value, distinct => by
      simp only [Which.project?, Which.name, variantPlainName_inr]
      exact Which.project?_eq_none_iff later value (List.nodup_cons.mp distinct).2

/-- A selection with one choice is that variant's field, when it is the
chosen variant. -/
theorem Choices.select?_single {names : List String} {rows : NRows} {τ : NTy} {σs : NRow}
    (choice : Which names rows σs) (x : Var σs τ) (value : variantPlain names rows) :
    (Choices.cons choice x .nil).select? value = (choice.project? value).map x.selectPlain := by
  simp only [Choices.select?]
  cases choice.project? value <;> rfl

/-- An update with one choice rebuilds that variant, when it is the chosen
variant. -/
theorem Choices.update?_single {names : List String} {rows : NRows} {τ : NTy} {σs : NRow}
    (choice : Which names rows σs) (x : Var σs τ) (replacement : τ.plain)
    (value : variantPlain names rows) :
    (Choices.cons choice x .nil).update? replacement value =
      (choice.project? value).map fun fields =>
        choice.inject (x.updatePlain replacement fields) := by
  simp only [Choices.update?]
  cases choice.project? value <;> rfl

/-- The component of a value a path reaches; none when an element
position is outside its vector, which the runtime leaves unresolved. -/
def Proj.get? {Γ : NRow} (env : HEnv Γ) : {τ σ : NTy} → Proj τ σ → τ.plain → Option σ.plain
  | _, _, .nil, value => some value
  | _, _, .deref rest, value => rest.get? env value.1
  | _, _, .sliceIndex position rest, value =>
      (position.resolve? env value.extent).bind fun index =>
        value.currentVector.values[index]?.bind fun element => rest.get? env element
  | _, _, .field x rest, value => rest.get? env (x.selectPlain value)
  | _, _, .element x rest, value => rest.get? env (x.selectPlain value)
  | _, _, .arrayIndex position rest, value =>
      (position.resolve? env value.vector.values.size).bind fun index =>
        value.vector.values[index]?.bind fun element => rest.get? env element
  | _, _, .index position rest, value | _, _, .sliceElement position rest, value =>
      (position.resolve? env value.values.size).bind fun index =>
        value.values[index]?.bind fun element => rest.get? env element
  | _, _, .variant choices rest, value =>
      (choices.select? value).bind fun field => rest.get? env field
  | _, _, .ghost rest, value => rest.get? env value

/-- The value with the component a path reaches replaced. -/
def Proj.set? {Γ : NRow} (env : HEnv Γ) : {τ σ : NTy} → Proj τ σ → σ.plain → τ.plain →
    Option τ.plain
  | _, _, .nil, replacement, _ => some replacement
  | _, _, .deref rest, replacement, value =>
      (rest.set? env replacement value.1).map fun current => (current, value.2)
  | _, _, .sliceIndex position rest, replacement, value =>
      (position.resolve? env value.extent).bind fun index =>
        value.currentVector.values[index]?.bind fun element =>
          (rest.set? env replacement element).map fun updated =>
            value.withCurrent ⟨value.current.vector.set index updated,
              value.current.size_set index updated⟩
  | _, _, .field x rest, replacement, value =>
      (rest.set? env replacement (x.selectPlain value)).map fun component =>
        x.updatePlain component value
  | _, _, .element x rest, replacement, value =>
      (rest.set? env replacement (x.selectPlain value)).map fun component =>
        x.updatePlain component value
  | _, _, .arrayIndex position rest, replacement, value =>
      (position.resolve? env value.vector.values.size).bind fun index =>
        value.vector.values[index]?.bind fun element =>
          (rest.set? env replacement element).map fun updated =>
            ⟨value.vector.set index updated, value.size_set index updated⟩
  | _, _, .index position rest, replacement, value | _, _, .sliceElement position rest, replacement, value =>
      (position.resolve? env value.values.size).bind fun index =>
        value.values[index]?.bind fun element =>
          (rest.set? env replacement element).map fun updated => value.set index updated
  | _, _, .variant choices rest, replacement, value =>
      (choices.select? value).bind fun field =>
        (rest.set? env replacement field).bind fun updated => choices.update? updated value
  | _, _, .ghost rest, replacement, value => rest.set? env replacement value

@[simp] theorem Proj.get?_nil {Γ : NRow} (env : HEnv Γ) {τ : NTy} (value : τ.plain) :
    (Proj.nil : Proj τ τ).get? env value = some value := rfl
@[simp] theorem Proj.get?_deref {Γ : NRow} (env : HEnv Γ) {τ σ : NTy} (rest : Proj τ σ)
    (value : (NTy.ref τ).plain) : (Proj.deref rest).get? env value = rest.get? env value.1 := rfl
@[simp] theorem Proj.get?_field {Γ : NRow} (env : HEnv Γ) {source : StructHandle} {fields : NRow}
    {σ τ : NTy} (x : Var fields σ) (rest : Proj σ τ)
    (value : (NTy.struct source arguments fields).plain) :
    (Proj.field x rest).get? env value = rest.get? env (x.selectPlain value) := rfl
@[simp] theorem Proj.get?_arrayIndex {Γ : NRow} (env : HEnv Γ) {τ σ : NTy} {length : Nat}
    (position : PlaceIndex) (rest : Proj τ σ) (value : (NTy.array τ length).plain) :
    (Proj.arrayIndex position rest).get? env value =
      (position.resolve? env value.vector.values.size).bind fun index =>
        value.vector.values[index]?.bind fun element => rest.get? env element := rfl
@[simp] theorem Proj.set?_arrayIndex {Γ : NRow} (env : HEnv Γ) {τ σ : NTy} {length : Nat}
    (position : PlaceIndex) (rest : Proj τ σ) (replacement : σ.plain)
    (value : (NTy.array τ length).plain) :
    (Proj.arrayIndex position rest).set? env replacement value =
      (position.resolve? env value.vector.values.size).bind fun index =>
        value.vector.values[index]?.bind fun element =>
          (rest.set? env replacement element).map fun updated =>
            ⟨value.vector.set index updated, value.size_set index updated⟩ := rfl
@[simp] theorem Proj.get?_element {Γ : NRow} (env : HEnv Γ) {elements : NRow}
    {σ τ : NTy} (x : Var elements σ) (rest : Proj σ τ) (value : (NTy.tuple elements).plain) :
    (Proj.element x rest).get? env value = rest.get? env (x.selectPlain value) := rfl
@[simp] theorem Proj.get?_index {Γ : NRow} (env : HEnv Γ) {τ σ : NTy} (position : PlaceIndex)
    (rest : Proj τ σ) (value : (NTy.vector τ).plain) :
    (Proj.index position rest).get? env value =
      (position.resolve? env value.values.size).bind fun index =>
        value.values[index]?.bind fun element => rest.get? env element := rfl
@[simp] theorem Proj.get?_variant {Γ : NRow} (env : HEnv Γ) {source : StructHandle}
    {names : List String} {rows : NRows} {distinct : names.Nodup} {σ τ : NTy}
    (choices : Choices names rows σ) (rest : Proj σ τ)
    (value : (NTy.enum source arguments names rows distinct).plain) :
    (Proj.variant choices rest).get? env value =
      (choices.select? value).bind fun field => rest.get? env field := rfl
@[simp] theorem Proj.get?_ghost {Γ : NRow} (env : HEnv Γ) {τ σ : NTy} (rest : Proj τ σ)
    (value : (NTy.ghost τ).plain) : (Proj.ghost rest).get? env value = rest.get? env value := rfl
@[simp] theorem Proj.set?_ghost {Γ : NRow} (env : HEnv Γ) {τ σ : NTy} (rest : Proj τ σ)
    (replacement : σ.plain) (value : (NTy.ghost τ).plain) :
    (Proj.ghost rest).set? env replacement value = rest.set? env replacement value := rfl
@[simp] theorem Proj.set?_nil {Γ : NRow} (env : HEnv Γ) {τ : NTy} (replacement value : τ.plain) :
    (Proj.nil : Proj τ τ).set? env replacement value = some replacement := rfl
@[simp] theorem Proj.set?_deref {Γ : NRow} (env : HEnv Γ) {τ σ : NTy} (rest : Proj τ σ)
    (replacement : σ.plain) (value : (NTy.ref τ).plain) :
    (Proj.deref rest).set? env replacement value =
      (rest.set? env replacement value.1).map fun current => (current, value.2) := rfl
@[simp] theorem Proj.set?_field {Γ : NRow} (env : HEnv Γ) {source : StructHandle} {fields : NRow}
    {σ τ : NTy} (x : Var fields σ) (rest : Proj σ τ) (replacement : τ.plain)
    (value : (NTy.struct source arguments fields).plain) :
    (Proj.field x rest).set? env replacement value =
      (rest.set? env replacement (x.selectPlain value)).map fun component =>
        x.updatePlain component value := rfl
@[simp] theorem Proj.set?_element {Γ : NRow} (env : HEnv Γ) {elements : NRow}
    {σ τ : NTy} (x : Var elements σ) (rest : Proj σ τ) (replacement : τ.plain)
    (value : (NTy.tuple elements).plain) :
    (Proj.element x rest).set? env replacement value =
      (rest.set? env replacement (x.selectPlain value)).map fun component =>
        x.updatePlain component value := rfl
@[simp] theorem Proj.set?_index {Γ : NRow} (env : HEnv Γ) {τ σ : NTy} (position : PlaceIndex)
    (rest : Proj τ σ) (replacement : σ.plain) (value : (NTy.vector τ).plain) :
    (Proj.index position rest).set? env replacement value =
      (position.resolve? env value.values.size).bind fun index =>
        value.values[index]?.bind fun element =>
          (rest.set? env replacement element).map fun updated => value.set index updated := rfl
@[simp] theorem Proj.set?_variant {Γ : NRow} (env : HEnv Γ) {source : StructHandle}
    {names : List String} {rows : NRows} {distinct : names.Nodup} {σ τ : NTy}
    (choices : Choices names rows σ) (rest : Proj σ τ) (replacement : τ.plain)
    (value : (NTy.enum source arguments names rows distinct).plain) :
    (Proj.variant choices rest).set? env replacement value =
      (choices.select? value).bind fun field =>
        (rest.set? env replacement field).bind fun updated => choices.update? updated value := rfl
/-- Whether a path reads through a mutable reference's current, which an
owner's certificate does not cover: the loan may be open there, so a value
read out of it is certified at the copy
(`doc/designs/data-invariants.md`). -/
def Proj.hasDeref : {τ σ : NTy} → Proj τ σ → Bool
  | _, _, .nil => false
  | _, _, .deref _ => true
  -- A step into a mutable slice reference's current reads out of a current the loan may
  -- have left open, as a dereference does.
  | _, _, .sliceIndex _ _ => true
  | _, _, .field _ rest => rest.hasDeref
  | _, _, .element _ rest => rest.hasDeref
  | _, _, .index _ rest => rest.hasDeref
  | _, _, .arrayIndex _ rest => rest.hasDeref
  | _, _, .sliceElement _ rest => rest.hasDeref
  | _, _, .variant _ rest => rest.hasDeref
  | _, _, .ghost rest => rest.hasDeref

omit [Carriers] in
@[simp] theorem Proj.hasDeref_nil {τ : NTy} : (Proj.nil : Proj τ τ).hasDeref = false := rfl
omit [Carriers] in
@[simp] theorem Proj.hasDeref_deref {τ σ : NTy} (rest : Proj τ σ) :
    (Proj.deref rest).hasDeref = true := rfl

/-- The value a local holds after a write along a path, as a step, with the
place's value replaced; none where an element position is outside its
vector or a variant lacks the field, which the write's own mismatch
reports.  A write into the local's own value ends a mutation of it and owes
its data invariants at every depth at `site`, the write-back into a root
the Move Prover checks; a write through a mutable reference's current owes
nothing, the current being plain while the loan lives and the prophecy the
owner keeps being certified already, and the current is certified where the
loan dies; a write of the whole local takes the written value's own
certificate (`doc/designs/data-invariants.md`). -/
noncomputable def Proj.setOwner? {Γ : NRow} (env : HEnv Γ) (site : Nat) :
    {τ σ : NTy} → Proj τ σ → σ.carrier → τ.carrier → Option (Spec State Failure τ.carrier)
  | _, _, .nil, replacement, _ => some (Spec.pure replacement)
  | _, _, .deref rest, replacement, value =>
      (rest.set? env (NTy.forget _ replacement) value.1).map fun current =>
        Spec.pure (current, value.2)
  | _, _, .field x rest, replacement, value =>
      ((Proj.field x rest).set? env (NTy.forget _ replacement) (NTy.forget _ value)).map
        (certifyingAt site .mutationEnd _)
  | _, _, .index position rest, replacement, value =>
      ((Proj.index position rest).set? env (NTy.forget _ replacement) (NTy.forget _ value)).map
        (certifyingAt site .mutationEnd _)
  | _, _, .element x rest, replacement, value =>
      ((Proj.element x rest).set? env (NTy.forget _ replacement) (NTy.forget _ value)).map
        (certifyingAt site .mutationEnd _)
  | _, _, .arrayIndex position rest, replacement, value =>
      ((Proj.arrayIndex position rest).set? env (NTy.forget _ replacement)
        (NTy.forget _ value)).map (certifyingAt site .mutationEnd _)
  | _, _, .sliceElement position rest, replacement, value =>
      ((Proj.sliceElement position rest).set? env (NTy.forget _ replacement)
        (NTy.forget _ value)).map (certifyingAt site .mutationEnd _)
  -- A slice reference's carrier keeps both readings certified
  -- (`NTy.carrier`, `.sliceRef`): a write through it stores an element
  -- certified by its type, so no cut belongs to the write, and
  -- `Proj.hasDeref` is true at `.sliceIndex`, so an owner's write never
  -- reaches this arm; the cut below is the unreachable case's harmless
  -- spelling, not an obligation the design states.
  | _, _, .sliceIndex position rest, replacement, value =>
      ((Proj.sliceIndex position rest).set? env (NTy.forget _ replacement)
        (NTy.forget _ value)).map (certifyingAt site .mutationEnd _)
  | _, _, .variant choices rest, replacement, value =>
      ((Proj.variant choices rest).set? env (NTy.forget _ replacement) (NTy.forget _ value)).map
        (certifyingAt site .mutationEnd _)
  | _, _, .ghost rest, replacement, value =>
      ((Proj.ghost rest).set? env (NTy.forget _ replacement) (NTy.forget _ value)).map
        (certifyingAt site .mutationEnd _)

@[simp] theorem Proj.setOwner?_nil {Γ : NRow} (env : HEnv Γ) (site : Nat) {τ : NTy}
    (replacement value : τ.carrier) :
    (Proj.nil : Proj τ τ).setOwner? (State := State) env site replacement value =
      some (Spec.pure replacement) := rfl
@[simp] theorem Proj.setOwner?_deref {Γ : NRow} (env : HEnv Γ) (site : Nat) {τ σ : NTy}
    (rest : Proj τ σ) (replacement : σ.carrier) (value : (NTy.ref τ).carrier) :
    (Proj.deref rest).setOwner? (State := State) env site replacement value =
      (rest.set? env (NTy.forget σ replacement) value.1).map fun current =>
        Spec.pure (current, value.2) := rfl
@[simp] theorem Proj.setOwner?_field {Γ : NRow} (env : HEnv Γ) (site : Nat)
    {source : StructHandle} {fields : NRow} {σ τ : NTy} (x : Var fields σ) (rest : Proj σ τ)
    (replacement : τ.carrier) (value : (NTy.struct source arguments fields).carrier) :
    (Proj.field x rest).setOwner? (State := State) env site replacement value =
      ((Proj.field x rest).set? env (NTy.forget τ replacement)
          (NTy.forget (.struct source arguments fields) value)).map
        (certifyingAt site .mutationEnd (.struct source arguments fields)) := rfl
@[simp] theorem Proj.setOwner?_index {Γ : NRow} (env : HEnv Γ) (site : Nat) {τ σ : NTy}
    (position : PlaceIndex) (rest : Proj τ σ) (replacement : σ.carrier)
    (value : (NTy.vector τ).carrier) :
    (Proj.index position rest).setOwner? (State := State) env site replacement value =
      ((Proj.index position rest).set? env (NTy.forget σ replacement)
          (NTy.forget (.vector τ) value)).map
        (certifyingAt site .mutationEnd (.vector τ)) := rfl
@[simp] theorem Proj.setOwner?_element {Γ : NRow} (env : HEnv Γ) (site : Nat)
    {elements : NRow} {σ τ : NTy} (x : Var elements σ) (rest : Proj σ τ)
    (replacement : τ.carrier) (value : (NTy.tuple elements).carrier) :
    (Proj.element x rest).setOwner? (State := State) env site replacement value =
      ((Proj.element x rest).set? env (NTy.forget τ replacement)
          (NTy.forget (.tuple elements) value)).map
        (certifyingAt site .mutationEnd (.tuple elements)) := rfl
@[simp] theorem Proj.setOwner?_arrayIndex {Γ : NRow} (env : HEnv Γ) (site : Nat) {τ σ : NTy}
    {length : Nat} (position : PlaceIndex) (rest : Proj τ σ) (replacement : σ.carrier)
    (value : (NTy.array τ length).carrier) :
    (Proj.arrayIndex position rest).setOwner? (State := State) env site replacement value =
      ((Proj.arrayIndex position rest).set? env (NTy.forget σ replacement)
          (NTy.forget (.array τ length) value)).map
        (certifyingAt site .mutationEnd (.array τ length)) := rfl
@[simp] theorem Proj.setOwner?_sliceElement {Γ : NRow} (env : HEnv Γ) (site : Nat) {τ σ : NTy}
    (position : PlaceIndex) (rest : Proj τ σ) (replacement : σ.carrier)
    (value : (NTy.slice τ).carrier) :
    (Proj.sliceElement position rest).setOwner? (State := State) env site replacement value =
      ((Proj.sliceElement position rest).set? env (NTy.forget σ replacement)
          (NTy.forget (.slice τ) value)).map
        (certifyingAt site .mutationEnd (.slice τ)) := rfl
@[simp] theorem Proj.setOwner?_sliceIndex {Γ : NRow} (env : HEnv Γ) (site : Nat) {τ σ : NTy}
    (position : PlaceIndex) (rest : Proj τ σ) (replacement : σ.carrier)
    (value : (NTy.sliceRef τ).carrier) :
    (Proj.sliceIndex position rest).setOwner? (State := State) env site replacement value =
      ((Proj.sliceIndex position rest).set? env (NTy.forget σ replacement)
          (NTy.forget (.sliceRef τ) value)).map
        (certifyingAt site .mutationEnd (.sliceRef τ)) := rfl
@[simp] theorem Proj.setOwner?_variant {Γ : NRow} (env : HEnv Γ) (site : Nat)
    {source : StructHandle} {names : List String} {rows : NRows} {distinct : names.Nodup}
    {σ τ : NTy} (choices : Choices names rows σ) (rest : Proj σ τ) (replacement : τ.carrier)
    (value : (NTy.enum source arguments names rows distinct).carrier) :
    (Proj.variant choices rest).setOwner? (State := State) env site replacement value =
      ((Proj.variant choices rest).set? env (NTy.forget τ replacement)
          (NTy.forget (.enum source arguments names rows distinct) value)).map
        (certifyingAt site .mutationEnd (.enum source arguments names rows distinct)) := rfl
@[simp] theorem Proj.setOwner?_ghost {Γ : NRow} (env : HEnv Γ) (site : Nat) {τ σ : NTy}
    (rest : Proj τ σ) (replacement : σ.carrier) (value : (NTy.ghost τ).carrier) :
    (Proj.ghost rest).setOwner? (State := State) env site replacement value =
      ((Proj.ghost rest).set? env (NTy.forget σ replacement)
          (NTy.forget (.ghost τ) value)).map
        (certifyingAt site .mutationEnd (.ghost τ)) := rfl

/-- A choice projects a row whose data invariants hold out of a value whose
own hold. -/
theorem Which.project?_deepHolds : {names : List String} → {rows : NRows} → {σs : NRow} →
    (choice : Which names rows σs) → (value : variantPlain names rows) →
    variantPlain.deepHolds names rows value → (fields : HListPlain σs) →
    choice.project? value = some fields → HListPlain.deepHolds σs fields
  | _, _, _, .here, .inl row, held, fields, projected => by
      cases Option.some.inj projected
      exact held
  | _, _, _, .here, .inr _, _, _, projected => by simp [Which.project?] at projected
  | _, _, _, .there _, .inl _, _, _, projected => by simp [Which.project?] at projected
  | _, _, _, .there later, .inr value, held, fields, projected =>
      later.project?_deepHolds value held fields projected

/-- Read a value out of an option under the equation that it is there: the
match lives here, over this function's own argument, so that no term of a
caller is abstracted by a motive.  A certificate taken from a value that a
caller's own `match … with` would generalize does not follow that
generalization, and the kernel rejects the application
(`Which.projectCertified?`, `doc/designs/data-invariants.md`). -/
def withSome {α β : Type} : (o : Option α) → ((a : α) → o = some a → β) → β → β
  | some a, found, _ => found a rfl
  | none, _, missing => missing

omit [Carriers] in
@[simp, vir_denote_norm] theorem withSome_some {α β : Type} (a : α)
    (found : (x : α) → (some a : Option α) = some x → β) (missing : β) :
    withSome (some a) found missing = found a rfl := rfl

omit [Carriers] in
@[simp, vir_denote_norm] theorem withSome_none {α β : Type}
    (found : (x : α) → (none : Option α) = some x → β) (missing : β) :
    withSome none found missing = missing := rfl

omit [Carriers] in
/-- `withSome` at an option a context equation says is there: what a step
that has already split the option applies, since a normal form cannot read
a hypothesis (`Choices.selectCertified?`). -/
theorem withSome_of_some {α β : Type} {o : Option α} {a : α} (h : o = some a)
    (found : (x : α) → o = some x → β) (missing : β) :
    withSome o found missing = found a h := by
  subst h; rfl

omit [Carriers] in
/-- `withSome` at an option a context equation says is empty. -/
theorem withSome_of_none {α β : Type} {o : Option α} (h : o = none)
    (found : (x : α) → o = some x → β) (missing : β) :
    withSome o found missing = missing := by
  subst h; rfl

omit [Carriers] in
/-- The cases of a reading of an option: the branch the reading takes,
under the equation of the pattern the option took.  A step that splits a
match on such a reading takes its cases here, so that the case states the
option's own pattern — which the normal forms decide and a context fixing
the value refutes — and not the reading it has just consumed, which the
case would otherwise carry and the leaf would have to take again
(`withSome_of_some`, `withSome_of_none`). -/
theorem withSome_cases {α β : Type} {motive : β → Prop} : {o : Option α} →
    {found : (a : α) → o = some a → β} → {missing : β} →
    (∀ (a : α) (equation : o = some a), motive (found a equation)) →
    (o = none → motive missing) → motive (withSome o found missing)
  | some a, _, _, isSome, _ => isSome a rfl
  | none, _, _, _, isNone => isNone rfl

omit [Carriers] in
/-- A function of what `withSome` reads applies in both of its branches. -/
theorem map_withSome {α β γ : Type} (f : β → γ) : (o : Option α) →
    (found : (a : α) → o = some a → β) → (missing : β) →
    f (withSome o found missing) = withSome o (fun a equation => f (found a equation)) (f missing)
  | some _, _, _ => rfl
  | none, _, _ => rfl

omit [Carriers] in
/-- `withSome` whose branches do not read the equation is the match on the
option: what a statement about the plain reading compares against. -/
theorem withSome_of_const {α β : Type} (o : Option α) (found : α → β) (missing : β) :
    withSome o (fun a _ => found a) missing =
      (match o with | some a => found a | none => missing) := by
  cases o <;> rfl

/-- The data invariants of the row of a certified struct value: the second
conjunct of its certificate, read through the statement the type makes of
it (`NTy.deepHolds_struct`).  Taken this way, and never as `.2` of the
certificate itself, so that no term carries the binder of the match in
`NTy.deepHolds`: a `.2` there is `And.right` at the match's body, whose
type arguments mention that binder once an equation unfolds the definition,
and the kernel then rejects the application
(`doc/designs/data-invariants.md`). -/
theorem NTy.fieldsHoldOf {source : StructHandle} {arguments fields : NRow}
    (row : HListPlain fields)
    (held : NTy.deepHolds (.struct source arguments fields) row) :
    HListPlain.deepHolds fields row :=
  ((NTy.deepHolds_struct source arguments fields row).mp held).2

theorem NTy.fieldsHold {source : StructHandle} {arguments fields : NRow}
    (value : (NTy.struct source arguments fields).carrier) :
    HListPlain.deepHolds fields value.val :=
  NTy.fieldsHoldOf value.val value.property

/-- Read a struct field with the certificate owned by the whole value. The
result type depends only on the field type, so normalizing the owner does not
transport a separately exposed dependent certificate. -/
def Var.selectCertified {source : StructHandle} {arguments fields : NRow} {field : NTy}
    (position : Var fields field) (value : (NTy.struct source arguments fields).carrier) :
    field.carrier :=
  NTy.ofDeepHolds field (position.selectPlain value.val)
    (position.selectPlain_deepHolds value.val (NTy.fieldsHold value))

/-- The uniform field operation specializes through a generic family before
certificate-free observations collapse to the plain projection. -/
theorem Var.selectCertified_of_certificateFree {source : StructHandle}
    {arguments fields : NRow} {field : NTy} (position : Var fields field)
    (value : (NTy.struct source arguments fields).carrier) (free : field.certificateFree = true) :
    position.selectCertified value =
      cast (NTy.carrier_eq_plain_of_certificateFree field free).symm
        (position.selectPlain value.val) :=
  (NTy.cast_eq_ofDeepHolds_of_certificateFree field free _).symm

/-- Forgetting a certified field is the plain field selected from the owner's row. -/
@[vir_denote_norm] theorem Var.forget_selectCertified {source : StructHandle}
    {arguments fields : NRow} {field : NTy} (position : Var fields field)
    (value : (NTy.struct source arguments fields).carrier) :
    NTy.forget field (position.selectCertified value) = position.selectPlain value.val :=
  NTy.forget_ofDeepHolds _ _ _

/-- The data invariants of the value of a certified variant: the second
conjunct of its certificate, read through the statement the type makes of
it (`NTy.deepHolds_enum`, `NTy.fieldsHold`). -/
theorem NTy.variantHoldsOf {source : StructHandle} {arguments : NRow} {names : List String}
    {rows : NRows} {distinct : names.Nodup} (value : variantPlain names rows)
    (held : NTy.deepHolds (.enum source arguments names rows distinct) value) :
    variantPlain.deepHolds names rows value :=
  ((NTy.deepHolds_enum source arguments names rows distinct value).mp held).2

theorem NTy.variantHolds {source : StructHandle} {arguments : NRow} {names : List String}
    {rows : NRows} {distinct : names.Nodup}
    (value : (NTy.enum source arguments names rows distinct).carrier) :
    variantPlain.deepHolds names rows value.val :=
  NTy.variantHoldsOf value.val value.property

/-- Forgetting the certificates of a selected component is the plain
selection of the forgotten row. -/
theorem Var.forget_select : {Γ : NRow} → {τ : NTy} → (x : Var Γ τ) → (values : HList Γ) →
    NTy.forget τ (x.select values) = x.selectPlain (HList.forget Γ values)
  | _, _, .here, _ => rfl
  | _, _, .there rest, values => rest.forget_select values.2

/-- The row a choice projects out of a certified enum value, certified by
the value's own certificate: the plain projection (`Which.project?`) with
the data invariants of the row read off the value's
(`Which.project?_deepHolds`).

Over the certified value alone, so that no goal carries the value's plain
side beside its certificate.  A caller's `match` over the plain side
generalizes it and leaves the certificate, whose statement still names the
plain side, behind, and the kernel rejects the application; a projection
taking only the certified value cannot be parted that way.  Its readings
are lemmas (`Which.forget_projectCertified?`), as a certified selection's
are (`Choices.selectCertified?`,
`doc/designs/data-invariants.md`). -/
def Which.projectCertified? {source : StructHandle} {arguments : NRow} {names : List String}
    {rows : NRows} {distinct : names.Nodup} {σs : NRow} (choice : Which names rows σs)
    (value : (NTy.enum source arguments names rows distinct).carrier) : Option (HList σs) :=
  withSome (choice.project? value.val)
    (fun fields projected =>
      some (HListPlain.ofDeepHolds σs fields
        (choice.project?_deepHolds value.val (NTy.variantHoldsOf value.val value.property)
          fields projected)))
    none

/-- A certified projection forgets to the plain one. -/
@[simp, vir_denote_norm] theorem Which.forget_projectCertified? {source : StructHandle}
    {arguments : NRow} {names : List String} {rows : NRows} {distinct : names.Nodup} {σs : NRow}
    (choice : Which names rows σs)
    (value : (NTy.enum source arguments names rows distinct).carrier) :
    (choice.projectCertified? value).map (HList.forget σs) = choice.project? value.val := by
  rw [Which.projectCertified?, map_withSome (Option.map (HList.forget σs))]
  simp only [Option.map_some, HListPlain.forget_ofDeepHolds]
  rw [withSome_of_const]
  cases choice.project? value.val <;> rfl

/-- A certified projection reads a row exactly where the plain one does. -/
@[simp, vir_denote_norm] theorem Which.isSome_projectCertified? {source : StructHandle}
    {arguments : NRow} {names : List String} {rows : NRows} {distinct : names.Nodup} {σs : NRow}
    (choice : Which names rows σs)
    (value : (NTy.enum source arguments names rows distinct).carrier) :
    (choice.projectCertified? value).isSome = (choice.project? value.val).isSome := by
  rw [← choice.forget_projectCertified? value, Option.isSome_map]

/-- A certified projection takes a row exactly when the value is the chosen
variant's, injected from the row the projection returns: the plain reading
relates the two options (`Which.forget_projectCertified?`) and a certified
row is determined by its forgetting (`HList.forget_injective`).  A leaf
reads a projected row this way, as it reads a plain one
(`Which.project?_eq_some_iff`). -/
theorem Which.projectCertified?_eq_some_iff {source : StructHandle} {arguments : NRow}
    {names : List String} {rows : NRows} {distinct : names.Nodup} {σs : NRow}
    (choice : Which names rows σs)
    (value : (NTy.enum source arguments names rows distinct).carrier) (fields : HList σs) :
    choice.projectCertified? value = some fields ↔
      value.val = choice.inject (HList.forget σs fields) := by
  rw [← choice.project?_eq_some_iff value.val (HList.forget σs fields),
    ← choice.forget_projectCertified? value]
  cases found : choice.projectCertified? value with
  | none => simp [found]
  | some row =>
      simp only [found, Option.map_some, Option.some.injEq]
      exact ⟨fun equal => equal ▸ rfl, HList.forget_injective σs⟩

/-- A certified projection takes nothing exactly where the plain one takes
nothing of the value's plain side, the two options being related by their
forgetting (`Which.isSome_projectCertified?`).  A leaf refutes a variant
this way, as it refutes it of a plain value
(`Which.project?_eq_none_iff`). -/
theorem Which.projectCertified?_eq_none_iff {source : StructHandle} {arguments : NRow}
    {names : List String} {rows : NRows} {distinct : names.Nodup} {σs : NRow}
    (choice : Which names rows σs)
    (value : (NTy.enum source arguments names rows distinct).carrier) :
    choice.projectCertified? value = none ↔ choice.project? value.val = none := by
  rw [← Option.not_isSome_iff_eq_none, ← Option.not_isSome_iff_eq_none,
    choice.isSome_projectCertified? value]

/-- The shared field a selection reads out of a value whose data invariants
hold has its own. -/
theorem Choices.select?_deepHolds {names : List String} {rows : NRows} {τ : NTy} :
    (choices : Choices names rows τ) → (value : variantPlain names rows) →
    variantPlain.deepHolds names rows value → (field : τ.plain) →
    choices.select? value = some field → NTy.deepHolds τ field
  | .nil, _, _, _, selected => by simp [Choices.select?] at selected
  | .cons choice x rest, value, held, field, selected => by
      simp only [Choices.select?] at selected
      split at selected
      · rename_i fields projected
        cases Option.some.inj selected
        exact x.selectPlain_deepHolds fields
          (choice.project?_deepHolds value held fields projected)
      · exact rest.select?_deepHolds value held field selected

/-- The chosen field of a certified enum value: the field of the certified
row the choice projects (`Which.projectCertified?`), whose certificate the
row's own is, so that the selection names no plain value and no certificate
beside one.  A clause reads a nominal variant field of a
value of the type this way; the current of a live mutable reference holds no
certificate of its own and is read plain
(`doc/designs/data-invariants.md`).  By recursion over the choices,
as the plain selection is, so that its computation at a value of a known
variant holds by reduction. -/
def Choices.selectCertified? {source : StructHandle} {arguments : NRow} {names : List String}
    {rows : NRows} {distinct : names.Nodup} {τ : NTy} :
    Choices names rows τ → (value : (NTy.enum source arguments names rows distinct).carrier) →
    Option τ.carrier
  | .nil, _ => none
  -- The projection is read through `withSome`, whose own match is over a
  -- variable, so that the equations of the selection hold by reduction; the
  -- branch reads a certified row and needs no equation of its own.
  | .cons choice x rest, value =>
      withSome (choice.projectCertified? value)
        (fun fields _ => some (x.select fields))
        (rest.selectCertified? value)

/-- A selection with no choice reads nothing of a certified value, as it
reads nothing of the plain one. -/
@[simp, vir_denote_norm] theorem Choices.selectCertified?_nil {source : StructHandle}
    {arguments : NRow} {names : List String} {rows : NRows} {distinct : names.Nodup} {τ : NTy}
    (value : (NTy.enum source arguments names rows distinct).carrier) :
    (Choices.nil (τ := τ)).selectCertified? value = none := rfl

/-- The first choice that projects the certified value reads its field off
the row it projects; the rest read it where that choice does not take the
variant. -/
@[simp, vir_denote_norm] theorem Choices.selectCertified?_cons {source : StructHandle}
    {arguments : NRow} {names : List String} {rows : NRows} {distinct : names.Nodup} {τ : NTy}
    {σs : NRow} (choice : Which names rows σs) (x : Var σs τ) (rest : Choices names rows τ)
    (value : (NTy.enum source arguments names rows distinct).carrier) :
    (Choices.cons choice x rest).selectCertified? value =
      withSome (choice.projectCertified? value)
        (fun fields _ => some (x.select fields))
        (rest.selectCertified? value) := rfl

/-- A certified selection forgets to the plain one. -/
@[simp, vir_denote_norm] theorem Choices.forget_selectCertified?
    {source : StructHandle} {arguments : NRow} {names : List String} {rows : NRows}
    {distinct : names.Nodup} {τ : NTy} (choices : Choices names rows τ)
    (value : (NTy.enum source arguments names rows distinct).carrier) :
    (choices.selectCertified? value).map (NTy.forget τ) = choices.select? value.val := by
  induction choices with
  | nil => rfl
  | cons choice x rest held =>
      rw [Choices.selectCertified?_cons, Choices.select?_cons,
        map_withSome (Option.map (NTy.forget τ))]
      -- Neither branch reads the projection's equation, so both readings are
      -- plain matches (`withSome_of_const`), over options the certified
      -- projection relates (`Which.forget_projectCertified?`).
      rw [withSome_of_const, ← choice.forget_projectCertified? value]
      cases choice.projectCertified? value with
      | some row => simp only [Option.map_some, Var.forget_select]
      | none => exact held

/-- A certified selection reads a field exactly where the plain one does. -/
@[simp, vir_denote_norm] theorem Choices.isSome_selectCertified?
    {source : StructHandle} {arguments : NRow} {names : List String} {rows : NRows}
    {distinct : names.Nodup} {τ : NTy} (choices : Choices names rows τ)
    (value : (NTy.enum source arguments names rows distinct).carrier) :
    (choices.selectCertified? value).isSome = (choices.select? value.val).isSome := by
  rw [← choices.forget_selectCertified? value, Option.isSome_map]

/-- The plain field a certified selection reads. -/
theorem Choices.select?_of_selectCertified?
    {source : StructHandle} {arguments : NRow} {names : List String} {rows : NRows}
    {distinct : names.Nodup} {τ : NTy} (choices : Choices names rows τ)
    (value : (NTy.enum source arguments names rows distinct).carrier) (field : τ.carrier)
    (selected : choices.selectCertified? value = some field) :
    choices.select? value.val = some (NTy.forget τ field) := by
  rw [← choices.forget_selectCertified? value, selected, Option.map_some]

/-! ### A scalar field's certified selection

A selection with one choice of a field whose type carries no certificate is
the plain selection of the value's plain side: the certified selection
forgets to the plain one (`Choices.forget_selectCertified?`,
`Choices.select?_single`) and at such a type the forgetting of a value is
the value (`NTy.forget_bool` and its siblings), so the two readings are one
reading.  Stated at each of those types, as the carrier's own collapse is
(`certifyingAt_bool`): the equation is the type's, and no certificate-free
predicate can state it of a type parameter, which a generic contract leaves
in a clause's field type until the caller's type arguments are spelled
there (`Solve/Normal.lean`, `choicesSelectCertifiedFamily`).

Read before the certified selection's own recursion
(`Choices.selectCertified?_cons`), as the plain reading is read before
`Choices.select?_cons`: the leaf then reads the value's plain side, which a
path's variant destructuring determines, and carries no certificate a
`match` over that side would leave behind. -/

theorem Choices.selectCertified?_single_unit {source : StructHandle}
    {arguments : NRow} {names : List String} {rows : NRows} {distinct : names.Nodup}
    {σs : NRow} (choice : Which names rows σs) (x : Var σs .unit)
    (value : (NTy.enum source arguments names rows distinct).carrier) :
    (Choices.cons choice x Choices.nil).selectCertified? value
      = (choice.project? value.val).map x.selectPlain := by
  rw [← Choices.select?_single choice x value.val,
    ← Choices.forget_selectCertified? (Choices.cons choice x Choices.nil) value]
  cases (Choices.cons choice x Choices.nil).selectCertified? value <;> rfl

theorem Choices.selectCertified?_single_bool {source : StructHandle}
    {arguments : NRow} {names : List String} {rows : NRows} {distinct : names.Nodup}
    {σs : NRow} (choice : Which names rows σs) (x : Var σs .bool)
    (value : (NTy.enum source arguments names rows distinct).carrier) :
    (Choices.cons choice x Choices.nil).selectCertified? value
      = (choice.project? value.val).map x.selectPlain := by
  rw [← Choices.select?_single choice x value.val,
    ← Choices.forget_selectCertified? (Choices.cons choice x Choices.nil) value]
  cases (Choices.cons choice x Choices.nil).selectCertified? value <;> rfl

theorem Choices.selectCertified?_single_int {source : StructHandle}
    {arguments : NRow} {names : List String} {rows : NRows} {distinct : names.Nodup}
    {σs : NRow} {width : Nat} {signed : Bool} (choice : Which names rows σs)
    (x : Var σs (.int width signed))
    (value : (NTy.enum source arguments names rows distinct).carrier) :
    (Choices.cons choice x Choices.nil).selectCertified? value
      = (choice.project? value.val).map x.selectPlain := by
  rw [← Choices.select?_single choice x value.val,
    ← Choices.forget_selectCertified? (Choices.cons choice x Choices.nil) value]
  cases (Choices.cons choice x Choices.nil).selectCertified? value <;> rfl

theorem Choices.selectCertified?_single_num {source : StructHandle}
    {arguments : NRow} {names : List String} {rows : NRows} {distinct : names.Nodup}
    {σs : NRow} (choice : Which names rows σs) (x : Var σs .num)
    (value : (NTy.enum source arguments names rows distinct).carrier) :
    (Choices.cons choice x Choices.nil).selectCertified? value
      = (choice.project? value.val).map x.selectPlain := by
  rw [← Choices.select?_single choice x value.val,
    ← Choices.forget_selectCertified? (Choices.cons choice x Choices.nil) value]
  cases (Choices.cons choice x Choices.nil).selectCertified? value <;> rfl

theorem Choices.selectCertified?_single_address {source : StructHandle}
    {arguments : NRow} {names : List String} {rows : NRows} {distinct : names.Nodup}
    {σs : NRow} (choice : Which names rows σs) (x : Var σs .address)
    (value : (NTy.enum source arguments names rows distinct).carrier) :
    (Choices.cons choice x Choices.nil).selectCertified? value
      = (choice.project? value.val).map x.selectPlain := by
  rw [← Choices.select?_single choice x value.val,
    ← Choices.forget_selectCertified? (Choices.cons choice x Choices.nil) value]
  cases (Choices.cons choice x Choices.nil).selectCertified? value <;> rfl

theorem Choices.selectCertified?_single_signer {source : StructHandle}
    {arguments : NRow} {names : List String} {rows : NRows} {distinct : names.Nodup}
    {σs : NRow} (choice : Which names rows σs) (x : Var σs .signer)
    (value : (NTy.enum source arguments names rows distinct).carrier) :
    (Choices.cons choice x Choices.nil).selectCertified? value
      = (choice.project? value.val).map x.selectPlain := by
  rw [← Choices.select?_single choice x value.val,
    ← Choices.forget_selectCertified? (Choices.cons choice x Choices.nil) value]
  cases (Choices.cons choice x Choices.nil).selectCertified? value <;> rfl

theorem Choices.selectCertified?_single_string {source : StructHandle}
    {arguments : NRow} {names : List String} {rows : NRows} {distinct : names.Nodup}
    {σs : NRow} (choice : Which names rows σs) (x : Var σs .string)
    (value : (NTy.enum source arguments names rows distinct).carrier) :
    (Choices.cons choice x Choices.nil).selectCertified? value
      = (choice.project? value.val).map x.selectPlain := by
  rw [← Choices.select?_single choice x value.val,
    ← Choices.forget_selectCertified? (Choices.cons choice x Choices.nil) value]
  cases (Choices.cons choice x Choices.nil).selectCertified? value <;> rfl

theorem Choices.selectCertified?_single_bytes {source : StructHandle}
    {arguments : NRow} {names : List String} {rows : NRows} {distinct : names.Nodup}
    {σs : NRow} (choice : Which names rows σs) (x : Var σs .bytes)
    (value : (NTy.enum source arguments names rows distinct).carrier) :
    (Choices.cons choice x Choices.nil).selectCertified? value
      = (choice.project? value.val).map x.selectPlain := by
  rw [← Choices.select?_single choice x value.val,
    ← Choices.forget_selectCertified? (Choices.cons choice x Choices.nil) value]
  cases (Choices.cons choice x Choices.nil).selectCertified? value <;> rfl

theorem Choices.selectCertified?_single_function {source : StructHandle}
    {arguments : NRow} {names : List String} {rows : NRows} {distinct : names.Nodup}
    {σs : NRow} {parameters : NRow} {shared : Sharing} {results : NRow}
    (choice : Which names rows σs) (x : Var σs (.function parameters shared results))
    (value : (NTy.enum source arguments names rows distinct).carrier) :
    (Choices.cons choice x Choices.nil).selectCertified? value
      = (choice.project? value.val).map x.selectPlain := by
  rw [← Choices.select?_single choice x value.val,
    ← Choices.forget_selectCertified? (Choices.cons choice x Choices.nil) value]
  cases (Choices.cons choice x Choices.nil).selectCertified? value <;> rfl

@[simp] theorem Proj.get?_sliceIndex {Γ : NRow} (env : HEnv Γ) {τ σ : NTy}
    (position : PlaceIndex) (rest : Proj τ σ) (value : SpecSliceRef τ.plain) :
    (Proj.sliceIndex position rest).get? env value =
      (position.resolve? env value.extent).bind fun index =>
        value.currentVector.values[index]?.bind fun element => rest.get? env element := rfl

@[simp] theorem Proj.set?_sliceIndex {Γ : NRow} (env : HEnv Γ) {τ σ : NTy}
    (position : PlaceIndex) (rest : Proj τ σ) (replacement : σ.plain)
    (value : SpecSliceRef τ.plain) :
    (Proj.sliceIndex position rest).set? env replacement value =
      (position.resolve? env value.extent).bind fun index =>
        value.currentVector.values[index]?.bind fun element =>
          (rest.set? env replacement element).map fun updated =>
            value.withCurrent ⟨value.current.vector.set index updated,
              value.current.size_set index updated⟩ := rfl

@[simp] theorem Proj.get?_sliceElement {Γ : NRow} (env : HEnv Γ) {τ σ : NTy}
    (position : PlaceIndex) (rest : Proj τ σ) (value : (NTy.slice τ).plain) :
    (Proj.sliceElement position rest).get? env value =
      (position.resolve? env value.values.size).bind fun index =>
        value.values[index]?.bind fun element => rest.get? env element := rfl

@[simp] theorem Proj.set?_sliceElement {Γ : NRow} (env : HEnv Γ) {τ σ : NTy}
    (position : PlaceIndex) (rest : Proj τ σ) (replacement : σ.plain)
    (value : (NTy.slice τ).plain) :
    (Proj.sliceElement position rest).set? env replacement value =
      (position.resolve? env value.values.size).bind fun index =>
        value.values[index]?.bind fun element =>
          (rest.set? env replacement element).map fun updated => value.set index updated := rfl

/-- The component a path reaches has the data invariants of the value it
reaches it in, where the path stays inside it: the certificate of an owner
is the certificate of everything in it.  A path through a dereference reads
out of a current the loan may have left open, which this does not cover. -/
theorem Proj.get?_deepHolds {Γ : NRow} (env : HEnv Γ) :
    {τ σ : NTy} → (path : Proj τ σ) → path.hasDeref = false → (value : τ.plain) →
    NTy.deepHolds τ value → (component : σ.plain) →
    path.get? env value = some component → NTy.deepHolds σ component
  | _, _, .nil, _, value, held, component, read => by
      cases Option.some.inj read
      exact held
  | _, _, .deref _, open_, _, _, _, _ => by simp at open_
  | _, _, .sliceIndex _ _, open_, _, _, _, _ => by simp [Proj.hasDeref] at open_
  | _, _, .field x rest, inside, value, held, component, read => by
      exact rest.get?_deepHolds env inside (x.selectPlain value)
        (x.selectPlain_deepHolds value held.2) component read
  -- A tuple carries no own clause, so its row's `deepHolds` is the whole
  -- hypothesis, where a struct's is its second component.
  | _, _, .element x rest, inside, value, held, component, read => by
      exact rest.get?_deepHolds env inside (x.selectPlain value)
        (x.selectPlain_deepHolds value held) component read
  | _, _, .arrayIndex position rest, inside, value, held, component, read => by
      simp only [Proj.get?_arrayIndex, Option.bind_eq_some_iff] at read
      obtain ⟨index, _, element, found, read⟩ := read
      exact rest.get?_deepHolds env inside element
        (held element (Array.mem_of_getElem? found)) component read
  | _, _, .sliceElement position rest, inside, value, held, component, read => by
      simp only [Proj.get?_sliceElement, Option.bind_eq_some_iff] at read
      obtain ⟨index, _, element, found, read⟩ := read
      exact rest.get?_deepHolds env inside element
        (held element (Array.mem_of_getElem? found)) component read
  | _, _, .index position rest, inside, value, held, component, read => by
      simp only [Proj.get?_index, Option.bind_eq_some_iff] at read
      obtain ⟨index, _, element, found, read⟩ := read
      exact rest.get?_deepHolds env inside element
        (held element (Array.mem_of_getElem? found)) component read
  | _, _, .variant choices rest, inside, value, held, component, read => by
      simp only [Proj.get?_variant, Option.bind_eq_some_iff] at read
      obtain ⟨field, selected, read⟩ := read
      exact rest.get?_deepHolds env inside field
        (choices.select?_deepHolds value held.2 field selected) component read
  | _, _, .ghost rest, inside, value, held, component, read =>
      rest.get?_deepHolds env inside value held component read

/-- The certified component a path reaches in a certified value: each step
reads its component certified by the part of the owner's certificate that is
the component's (`Var.selectPlain_deepHolds`, `Choices.selectCertified?`),
an element of a vector and the contents of a ghost slot being certified
already.  A path through a dereference reads out of a current the loan may
have left open, which carries no certificate, and reaches nothing here; the
denotation splits on `Proj.hasDeref` and certifies such a read at the copy.
By recursion over the path, as the plain read (`Proj.get?`) is, so that its
computation at a concrete path holds by reduction: a plain read with the
certificate taken from a dependent match on its equation would carry that
equation into every reading of the case, and a normalization that rewrites
the read leaves the matcher's motive behind
(`doc/designs/data-invariants.md`). -/
def Proj.getCertified? {Γ : NRow} (env : HEnv Γ) :
    {τ σ : NTy} → Proj τ σ → τ.carrier → Option σ.carrier
  | _, _, .nil, value => some value
  | _, _, .deref _, _ => none
  -- A step into a mutable slice reference's current reaches nothing certified, as a
  -- dereference does.
  | _, _, .sliceIndex _ _, _ => none
  | _, _, .field x rest, value =>
      rest.getCertified? env (x.selectCertified value)
  -- A tuple's carrier is the row of certified components: the component is certified already.
  | _, _, .element x rest, value => rest.getCertified? env (x.select value)
  | _, _, .index position rest, value =>
      (position.resolve? env value.values.size).bind fun index =>
        value.values[index]?.bind fun element => rest.getCertified? env element
  | _, _, .arrayIndex position rest, value =>
      (position.resolve? env value.vector.values.size).bind fun index =>
        value.vector.values[index]?.bind fun element => rest.getCertified? env element
  | _, _, .sliceElement position rest, value =>
      (position.resolve? env value.values.size).bind fun index =>
        value.values[index]?.bind fun element => rest.getCertified? env element
  | _, _, .variant choices rest, value =>
      (choices.selectCertified? value).bind fun field => rest.getCertified? env field
  | _, _, .ghost rest, value => rest.getCertified? env value

/-- An empty path reaches the value itself, certified as it is held. -/
@[simp, vir_denote_norm] theorem Proj.getCertified?_nil {Γ : NRow} (env : HEnv Γ) {τ : NTy}
    (value : τ.carrier) :
    (Proj.nil : Proj τ τ).getCertified? env value = some value := rfl

/-- A path through a dereference reaches no certified component: the current
it reads out of is plain while the loan lives. -/
@[simp, vir_denote_norm] theorem Proj.getCertified?_deref {Γ : NRow} (env : HEnv Γ) {τ σ : NTy}
    (rest : Proj τ σ) (value : (NTy.ref τ).carrier) :
    (Proj.deref rest).getCertified? env value = none := rfl

/-- A field step reads the field certified by the field's part of the
owner's certificate, and the rest of the path reads on in it. -/
@[simp, vir_denote_norm] theorem Proj.getCertified?_field {Γ : NRow} (env : HEnv Γ)
    {source : StructHandle} {fields : NRow} {σ τ : NTy} (x : Var fields σ) (rest : Proj σ τ)
    (value : (NTy.struct source arguments fields).carrier) :
    (Proj.field x rest).getCertified? env value =
      rest.getCertified? env (x.selectCertified value) := rfl

/-- An element step reads the element the position resolves to, certified as
the vector holds it. -/
@[simp, vir_denote_norm] theorem Proj.getCertified?_index {Γ : NRow} (env : HEnv Γ) {τ σ : NTy}
    (position : PlaceIndex) (rest : Proj τ σ) (value : (NTy.vector τ).carrier) :
    (Proj.index position rest).getCertified? env value =
      (position.resolve? env value.values.size).bind fun index =>
        value.values[index]?.bind fun element => rest.getCertified? env element := rfl

/-- A variant step reads the chosen field certified by the value's
certificate (`Choices.selectCertified?`). -/
@[simp, vir_denote_norm] theorem Proj.getCertified?_variant {Γ : NRow} (env : HEnv Γ)
    {source : StructHandle} {names : List String} {rows : NRows} {distinct : names.Nodup}
    {σ τ : NTy} (choices : Choices names rows σ) (rest : Proj σ τ)
    (value : (NTy.enum source arguments names rows distinct).carrier) :
    (Proj.variant choices rest).getCertified? env value =
      (choices.selectCertified? value).bind fun field => rest.getCertified? env field := rfl

/-- A tuple's component step reads the component the carrier holds, certified already. -/
@[simp, vir_denote_norm] theorem Proj.getCertified?_element {Γ : NRow} (env : HEnv Γ)
    {elements : NRow} {σ τ : NTy} (x : Var elements σ) (rest : Proj σ τ)
    (value : (NTy.tuple elements).carrier) :
    (Proj.element x rest).getCertified? env value = rest.getCertified? env (x.select value) := rfl

/-- An array's element step reads the element the position resolves to, certified as the
array holds it. -/
@[simp, vir_denote_norm] theorem Proj.getCertified?_arrayIndex {Γ : NRow} (env : HEnv Γ)
    {τ σ : NTy} {length : Nat} (position : PlaceIndex) (rest : Proj τ σ)
    (value : (NTy.array τ length).carrier) :
    (Proj.arrayIndex position rest).getCertified? env value =
      (position.resolve? env value.vector.values.size).bind fun index =>
        value.vector.values[index]?.bind fun element => rest.getCertified? env element := rfl

/-- A slice's element step reads the element the position resolves to, certified as the
slice holds it. -/
@[simp, vir_denote_norm] theorem Proj.getCertified?_sliceElement {Γ : NRow} (env : HEnv Γ)
    {τ σ : NTy} (position : PlaceIndex) (rest : Proj τ σ) (value : (NTy.slice τ).carrier) :
    (Proj.sliceElement position rest).getCertified? env value =
      (position.resolve? env value.values.size).bind fun index =>
        value.values[index]?.bind fun element => rest.getCertified? env element := rfl

/-- A step into a mutable slice reference's current reaches no certified component. -/
@[simp, vir_denote_norm] theorem Proj.getCertified?_sliceIndex {Γ : NRow} (env : HEnv Γ)
    {τ σ : NTy} (position : PlaceIndex) (rest : Proj τ σ) (value : (NTy.sliceRef τ).carrier) :
    (Proj.sliceIndex position rest).getCertified? env value = none := rfl

/-- A ghost step reads the slot's contents, certified as the slot holds
them. -/
@[simp, vir_denote_norm] theorem Proj.getCertified?_ghost {Γ : NRow} (env : HEnv Γ) {τ σ : NTy}
    (rest : Proj τ σ) (value : (NTy.ghost τ).carrier) :
    (Proj.ghost rest).getCertified? env value = rest.getCertified? env value := rfl

/-- A certified read forgets to the plain one, where the path stays inside
the value: the two reads take the same component, the certified one carrying
the certificates the plain one drops.  This is what a specification's plain
reasoning about a read place reaches. -/
@[simp, vir_denote_norm] theorem Proj.forget_getCertified? {Γ : NRow} (env : HEnv Γ) :
    {τ σ : NTy} → (path : Proj τ σ) → path.hasDeref = false → (value : τ.carrier) →
    (path.getCertified? env value).map (NTy.forget σ) = path.get? env (NTy.forget τ value)
  | _, _, .nil, _, _ => rfl
  | _, _, .deref _, open_, _ => by simp at open_
  | _, _, .sliceIndex _ _, open_, _ => by simp [Proj.hasDeref] at open_
  | _, _, .field x rest, inside, value => by
      rw [Proj.getCertified?_field, Proj.get?_field,
        rest.forget_getCertified? env inside, Var.forget_selectCertified, NTy.forget_struct]
  -- A tuple's forgotten component is the forgotten row's component
  -- (`Var.selectPlain_forget`).
  | _, _, .element x rest, inside, value => by
      rw [Proj.getCertified?_element, Proj.get?_element,
        rest.forget_getCertified? env inside, NTy.forget_tuple, Var.selectPlain_forget]
  | _, _, .arrayIndex position rest, inside, value => by
      have values : (NTy.forget (.array _ _) value).vector.values =
          value.vector.values.map (NTy.forget _) := rfl
      rw [Proj.getCertified?_arrayIndex, Proj.get?_arrayIndex, values]
      simp only [Array.size_map, Array.getElem?_map]
      rcases position.resolve? env value.vector.values.size with _ | index
      · simp only [Option.bind_none, Option.map_none]
      rcases found : value.vector.values[index]? with _ | element
      · simp only [found, Option.bind_some, Option.bind_none, Option.map_none]
      · simp only [found, Option.bind_some, Option.map_some]
        exact rest.forget_getCertified? env inside element
  | _, _, .sliceElement position rest, inside, value => by
      have values : (NTy.forget (.slice _) value).values = value.values.map (NTy.forget _) := rfl
      rw [Proj.getCertified?_sliceElement, Proj.get?_sliceElement, values]
      simp only [Array.size_map, Array.getElem?_map]
      rcases position.resolve? env value.values.size with _ | index
      · simp only [Option.bind_none, Option.map_none]
      rcases found : value.values[index]? with _ | element
      · simp only [found, Option.bind_some, Option.bind_none, Option.map_none]
      · simp only [found, Option.bind_some, Option.map_some]
        exact rest.forget_getCertified? env inside element
  | _, _, .index position rest, inside, value => by
      have values : (NTy.forget (.vector _) value).values = value.values.map (NTy.forget _) := rfl
      rw [Proj.getCertified?_index, Proj.get?_index, values]
      simp only [Array.size_map, Array.getElem?_map]
      rcases position.resolve? env value.values.size with _ | index
      · simp only [Option.bind_none, Option.map_none]
      rcases found : value.values[index]? with _ | element
      · simp only [found, Option.bind_some, Option.bind_none, Option.map_none]
      · simp only [found, Option.bind_some, Option.map_some]
        exact rest.forget_getCertified? env inside element
  | _, _, .variant choices rest, inside, value => by
      rw [Proj.getCertified?_variant, Proj.get?_variant, NTy.forget_enum,
        ← choices.forget_selectCertified? value]
      cases selected : choices.selectCertified? value with
      | none => rfl
      | some field => exact rest.forget_getCertified? env inside field
  | _, _, .ghost rest, inside, value => by
      rw [Proj.getCertified?_ghost, Proj.get?_ghost, NTy.forget_ghost,
        rest.forget_getCertified? env inside]

/-- A certified read reaches a component exactly where the plain one does,
so the mismatch a read reports is the same at both layers. -/
@[simp, vir_denote_norm] theorem Proj.isSome_getCertified? {Γ : NRow} (env : HEnv Γ) {τ σ : NTy}
    (path : Proj τ σ) (inside : path.hasDeref = false) (value : τ.carrier) :
    (path.getCertified? env value).isSome = (path.get? env (NTy.forget τ value)).isSome := by
  rw [← path.forget_getCertified? env inside value, Option.isSome_map]

/-- The plain component a certified read reaches. -/
theorem Proj.get?_of_getCertified? {Γ : NRow} (env : HEnv Γ) {τ σ : NTy} (path : Proj τ σ)
    (inside : path.hasDeref = false) (value : τ.carrier) (component : σ.carrier)
    (read : path.getCertified? env value = some component) :
    path.get? env (NTy.forget τ value) = some (NTy.forget σ component) := by
  rw [← path.forget_getCertified? env inside value, read, Option.map_some]

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

/-- The ghost slots a pack initializes: the site of each initializer and
the slot of the field row it writes, in declaration order
(`doc/designs/ghost-fields.md`).  The initializers run over the certified
field row, so an initializer reads the pack's fields as the values they are,
and the data invariants of the value are owed once, of the row they leave
(`doc/designs/data-invariants.md`). -/
inductive GhostInits (σs : NRow) : Type where
  | nil : GhostInits σs
  | cons {σ : NTy} (site : Nat) (slot : Var σs (.ghost σ)) (rest : GhostInits σs) :
      GhostInits σs

/-- What a dying loan writes back to.  A borrow of a local place names the
local it was borrowed from and the path it was borrowed at: the owner
rebuilt with the reference's final current is certified where the loan dies.
A borrow of global memory, and a mutable parameter at an exit, name no
local: the slot holds the referent, so the current itself is certified
there.  A reborrow is a place through a dereference, whose owner is another
reference's plain current: rebuilding it certifies nothing, and the outer
loan's death owes the rest (`doc/designs/data-invariants.md`). -/
inductive Lender (Γ : NRow) (σ : NTy) : Type where
  | slot : Lender Γ σ
  | place {τ : NTy} (owner : Var Γ τ) (path : Proj τ σ) : Lender Γ σ
  /-- One of several owners a dying loan may have been borrowed from, where
  the holder may hold another loan's reference instead: a reference joined
  from borrows of different owners carries no record of the owner it came
  from.  The owner rebuilt with this current is certified, as `place`
  certifies it, and no prophecy equation is stated, because this holder
  need not hold this loan; the equation is stated once beside these, by a
  `slot` resolve of the same holder (`deathEffect`). -/
  | candidate {τ : NTy} (owner : Var Γ τ) (path : Proj τ σ) : Lender Γ σ
  /-- The obligation of a dying loan that no local lends, where the holder
  may hold another loan's reference instead: the referent's current, as
  `slot` certifies it, and no prophecy equation. -/
  | referent : Lender Γ σ
  /-- The prophecy equation of a holder that may hold either of several
  loans' references, stated once beside their obligations
  (`candidate`, `referent`): a borrow constrains the owner it sets to agree
  with the reference's prophecy at the path (`Term.borrowPlace`), so this one
  equation is the relation every one of those lenders states. -/
  | equation : Lender Γ σ

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

mutual
/-- A pattern over values of one type, binding slots of a row: the typed
form of a validated VIR pattern. -/
inductive Pat (Γ : NRow) : NTy → Type where
  | wildcard {τ : NTy} : Pat Γ τ
  | var {τ : NTy} (x : Var Γ τ) : Pat Γ τ
  /-- A value structurally equal to a closed one. -/
  | literal {τ : NTy} (value : τ.groundCarrier) (literal : τ.literal = true) : Pat Γ τ
  /-- An integer within optional bounds, the upper one inclusive or not. -/
  | range {width : Nat} {signed : Bool} (lower upper : Option Int) (inclusive : Bool) :
      Pat Γ (.int width signed)
  | tuple {σs : NRow} (elements : Pats Γ σs) : Pat Γ (.tuple σs)
  | struct {source : StructHandle} {arguments σs : NRow} (fields : Pats Γ σs) :
      Pat Γ (.struct source arguments σs)
  | variant {source : StructHandle} {arguments : NRow} {names : List String} {rows : NRows}
      {distinct : names.Nodup} {σs : NRow} (choice : Which names rows σs) (fields : Pats Γ σs) :
      Pat Γ (.enum source arguments names rows distinct)

/-- Patterns over the components of a row. -/
inductive Pats (Γ : NRow) : NRow → Type where
  | nil : Pats Γ .nil
  | cons {σ : NTy} {σs : NRow} (head : Pat Γ σ) (tail : Pats Γ σs) : Pats Γ (.cons σ σs)
end

/-- Whether an integer lies within a range pattern's bounds. -/
def inRange (lower upper : Option Int) (inclusive : Bool) (value : Int) : Bool :=
  lower.all (· ≤ value) && upper.all fun upper => if inclusive then value ≤ upper else value < upper

mutual
/-- Match a value against a pattern, binding its slots; `none` where it does
not match, with no slot bound, as the runtime's `bindPattern` fails
atomically. -/
def Pat.bindValue {Γ : NRow} : {τ : NTy} → Pat Γ τ → τ.carrier → HEnv Γ → Option (HEnv Γ)
  | _, .wildcard, _, env => some env
  | _, .var x, value, env => some (x.set value env)
  | τ, .literal expected literal, value, env =>
      if NTy.eqb τ (NTy.ofGround τ literal expected) value then some env else none
  | _, .range lower upper inclusive, value, env =>
      if inRange lower upper inclusive value.val then some env else none
  | _, .tuple elements, values, env => elements.bindValues values env
  | _, .struct fields, values, env => fields.bindPlainValues values.val (NTy.fieldsHold values) env
  | _, .variant choice fields, value, env =>
      -- Through the certified projection, so that this reading of the value
      -- never puts its plain side beside its certificate in a goal
      -- (`Which.projectCertified?`).
      withSome (choice.projectCertified? value)
        (fun values _ =>
          fields.bindPlainValues (HList.forget _ values) (HList.deepHolds_forget _ values) env)
        none

/-- Match a row of values component by component. -/
def Pats.bindValues {Γ : NRow} : {σs : NRow} → Pats Γ σs → HList σs → HEnv Γ → Option (HEnv Γ)
  | _, .nil, _, env => some env
  | _, .cons head tail, values, env =>
      (head.bindValue values.1 env).bind fun env => tail.bindValues values.2 env

/-- Match a plain value against a pattern: a component of a certified owner,
whose certificate a bound slot takes (`NTy.ofDeepHolds`).  A component of a
certified value has one, so a pattern binds it without owing anything. -/
def Pat.bindPlain {Γ : NRow} : {τ : NTy} → Pat Γ τ → (value : τ.plain) →
    NTy.deepHolds τ value → HEnv Γ → Option (HEnv Γ)
  | _, .wildcard, _, _, env => some env
  | τ, .var x, value, holds, env => some (x.set (NTy.ofDeepHolds τ value holds) env)
  | τ, .literal expected literal, value, _, env =>
      if NTy.plainEqb τ (NTy.forget τ (NTy.ofGround τ literal expected)) value then some env
      else none
  | _, .range lower upper inclusive, value, _, env =>
      if inRange lower upper inclusive value.val then some env else none
  | _, .tuple elements, values, holds, env => elements.bindPlainValues values holds env
  | _, .struct fields, values, holds, env => fields.bindPlainValues values holds.2 env
  | _, .variant choice fields, value, holds, env =>
      withSome (choice.project? value)
        (fun values projected =>
          fields.bindPlainValues values
            (choice.project?_deepHolds value (NTy.variantHoldsOf value holds) values projected) env)
        none

/-- Match a plain row of values component by component. -/
def Pats.bindPlainValues {Γ : NRow} : {σs : NRow} → Pats Γ σs → (values : HListPlain σs) →
    HListPlain.deepHolds σs values → HEnv Γ → Option (HEnv Γ)
  | _, .nil, _, _, env => some env
  | _, .cons head tail, values, holds, env =>
      (head.bindPlain values.1 holds.1 env).bind fun env =>
        tail.bindPlainValues values.2 holds.2 env
end

@[simp] theorem Pat.bindValue_wildcard {Γ : NRow} {τ : NTy} (value : τ.carrier) (env : HEnv Γ) :
    (Pat.wildcard : Pat Γ τ).bindValue value env = some env := by
  rw [Pat.bindValue]
@[simp] theorem Pat.bindValue_var {Γ : NRow} {τ : NTy} (x : Var Γ τ) (value : τ.carrier)
    (env : HEnv Γ) : (Pat.var x).bindValue value env = some (x.set value env) := by
  rw [Pat.bindValue]
@[simp] theorem Pat.bindValue_literal {Γ : NRow} {τ : NTy} (expected : τ.groundCarrier)
    (literal : τ.literal = true) (value : τ.carrier) (env : HEnv Γ) :
    (Pat.literal expected literal : Pat Γ τ).bindValue value env =
      if NTy.eqb τ (NTy.ofGround τ literal expected) value then some env else none := by
  rw [Pat.bindValue]
@[simp] theorem Pat.bindValue_range {Γ : NRow} {width : Nat} {signed : Bool}
    (lower upper : Option Int) (inclusive : Bool) (value : (NTy.int width signed).carrier)
    (env : HEnv Γ) :
    (Pat.range lower upper inclusive : Pat Γ (.int width signed)).bindValue value env =
      if inRange lower upper inclusive value.val then some env else none := by
  rw [Pat.bindValue]
@[simp] theorem Pat.bindValue_tuple {Γ σs : NRow} (elements : Pats Γ σs) (values : HList σs)
    (env : HEnv Γ) : (Pat.tuple elements).bindValue values env = elements.bindValues values env := by
  rw [Pat.bindValue]
@[simp] theorem Pat.bindValue_struct {Γ σs : NRow} {source : StructHandle} {arguments : NRow}
    (fields : Pats Γ σs)
    (values : (NTy.struct source arguments σs).carrier) (env : HEnv Γ) :
    (Pat.struct (source := source) (arguments := arguments) fields).bindValue values env =
      fields.bindPlainValues values.val (NTy.fieldsHold values) env := by
  rw [Pat.bindValue]
@[simp] theorem Pat.bindValue_variant {Γ σs : NRow} {source : StructHandle} {arguments : NRow}
    {names : List String} {rows : NRows} {distinct : names.Nodup} (choice : Which names rows σs)
    (fields : Pats Γ σs)
    (value : (NTy.enum source arguments names rows distinct).carrier) (env : HEnv Γ) :
    (Pat.variant (source := source) (arguments := arguments) (distinct := distinct) choice
      fields).bindValue value env =
      withSome (choice.projectCertified? value)
        (fun values _ =>
          fields.bindPlainValues (HList.forget σs values)
            (HList.deepHolds_forget σs values) env)
        none := by
  rw [Pat.bindValue]
@[simp] theorem Pats.bindValues_nil {Γ : NRow} (values : HList .nil) (env : HEnv Γ) :
    (Pats.nil : Pats Γ .nil).bindValues values env = some env := by
  rw [Pats.bindValues]
@[simp] theorem Pats.bindValues_cons {Γ : NRow} {σ : NTy} {σs : NRow} (head : Pat Γ σ)
    (tail : Pats Γ σs) (values : HList (.cons σ σs)) (env : HEnv Γ) :
    (Pats.cons head tail).bindValues values env =
      (head.bindValue values.1 env).bind fun env => tail.bindValues values.2 env := by
  rw [Pats.bindValues]

@[simp, vir_denote] theorem Pat.bindPlain_wildcard {Γ : NRow} {τ : NTy}
    (value : τ.plain) (holds) (env : HEnv Γ) :
    (Pat.wildcard : Pat Γ τ).bindPlain value holds env = some env := by
  rw [Pat.bindPlain]
@[simp, vir_denote] theorem Pat.bindPlain_var {Γ : NRow} {τ : NTy} (x : Var Γ τ)
    (value : τ.plain) (holds) (env : HEnv Γ) :
    (Pat.var x).bindPlain value holds env = some (x.set (NTy.ofDeepHolds τ value holds) env) := by
  rw [Pat.bindPlain]
@[simp, vir_denote] theorem Pat.bindPlain_literal {Γ : NRow} {τ : NTy}
    (expected : τ.groundCarrier) (literal : τ.literal = true)
    (value : τ.plain) (holds) (env : HEnv Γ) :
    (Pat.literal expected literal : Pat Γ τ).bindPlain value holds env =
      if NTy.plainEqb τ (NTy.forget τ (NTy.ofGround τ literal expected)) value then some env
      else none := by
  rw [Pat.bindPlain]
@[simp, vir_denote] theorem Pat.bindPlain_range {Γ : NRow} {width : Nat} {signed : Bool}
    (lower upper : Option Int) (inclusive : Bool) (value : (NTy.int width signed).plain)
    (holds) (env : HEnv Γ) :
    (Pat.range lower upper inclusive : Pat Γ (.int width signed)).bindPlain value holds env =
      if inRange lower upper inclusive value.val then some env else none := by
  rw [Pat.bindPlain]
@[simp, vir_denote] theorem Pat.bindPlain_tuple {Γ σs : NRow} (elements : Pats Γ σs)
    (values : HListPlain σs) (holds) (env : HEnv Γ) :
    (Pat.tuple elements).bindPlain values holds env = elements.bindPlainValues values holds env := by
  rw [Pat.bindPlain]
@[simp, vir_denote] theorem Pat.bindPlain_struct {Γ σs : NRow} {source : StructHandle}
    {arguments : NRow} (fields : Pats Γ σs)
    (values : (NTy.struct source arguments σs).plain) (holds) (env : HEnv Γ) :
    (Pat.struct (source := source) (arguments := arguments) fields).bindPlain values holds env =
      fields.bindPlainValues values holds.2 env := by
  rw [Pat.bindPlain]
@[simp, vir_denote] theorem Pat.bindPlain_variant {Γ σs : NRow} {source : StructHandle}
    {arguments : NRow} {names : List String} {rows : NRows} {distinct : names.Nodup}
    (choice : Which names rows σs) (fields : Pats Γ σs)
    (value : (NTy.enum source arguments names rows distinct).plain) (holds) (env : HEnv Γ) :
    (Pat.variant (source := source) (arguments := arguments) (distinct := distinct) choice fields).bindPlain value holds env =
      withSome (choice.project? value)
        (fun values projected =>
          fields.bindPlainValues values
            (choice.project?_deepHolds value (NTy.variantHoldsOf value holds) values projected) env)
        none := by
  rw [Pat.bindPlain]
@[simp, vir_denote] theorem Pats.bindPlainValues_nil {Γ : NRow} (values : HListPlain .nil)
    (holds) (env : HEnv Γ) :
    (Pats.nil : Pats Γ .nil).bindPlainValues values holds env = some env := by
  rw [Pats.bindPlainValues]
@[simp, vir_denote] theorem Pats.bindPlainValues_cons {Γ : NRow} {σ : NTy} {σs : NRow}
    (head : Pat Γ σ) (tail : Pats Γ σs) (values : HListPlain (.cons σ σs)) (holds)
    (env : HEnv Γ) :
    (Pats.cons head tail).bindPlainValues values holds env =
      (head.bindPlain values.1 holds.1 env).bind fun env =>
        tail.bindPlainValues values.2 holds.2 env := by
  rw [Pats.bindPlainValues]


/-- The body value a declared result denotes. -/
def ResultShape.toBody : (shape : ResultShape) → shape.carrier → shape.bodyType.carrier
  | .none, _ => ()
  | .one _, value => value

/-- How a closure's mask weaves the parameters it captures and those an
invocation supplies into its target's parameter row, position by position. -/
inductive Weave : NRow → NRow → NRow → Type where
  | nil : Weave .nil .nil .nil
  | captured {τ : NTy} {full captured supplied : NRow} (rest : Weave full captured supplied) :
      Weave (.cons τ full) (.cons τ captured) supplied
  | supplied {τ : NTy} {full captured supplied : NRow} (rest : Weave full captured supplied) :
      Weave (.cons τ full) captured (.cons τ supplied)

omit [Carriers] in
/-- The mask of a weave: bit `i` set when parameter `i` is captured. -/
def Weave.mask : {full captured supplied : NRow} → Weave full captured supplied → Nat
  | _, _, _, .nil => 0
  | _, _, _, .captured rest => 2 * rest.mask + 1
  | _, _, _, .supplied rest => 2 * rest.mask

omit [Carriers] in
/-- The weave of a mask over a parameter row. -/
def Weave.ofMask : Nat → (full : List NTy) →
    Σ captured supplied : NRow, Weave (NRow.ofList full) captured supplied
  | _, [] => ⟨.nil, .nil, .nil⟩
  | mask, τ :: rest =>
      let ⟨taken, given, weave⟩ := Weave.ofMask (mask / 2) rest
      if mask % 2 == 1 then ⟨.cons τ taken, given, .captured weave⟩
      else ⟨taken, .cons τ given, .supplied weave⟩

omit [Carriers] in
/-- Whether a type parameter is a closure target's own and has a node, as its
frames' coherence reads it. -/
def closureParameter (unit : Validation.ValidatedUnit) (handle : FunctionHandle)
    (index : Nat) : Bool :=
  decide (index < typeArity unit handle) && (paramNodeIn? unit handle.namespaceId index).isSome

omit [Carriers] in
/-- Whether every type parameter a closure target's own signature mentions
is its own and has a node. -/
def closureSignatureBelow (unit : Validation.ValidatedUnit) (handle : FunctionHandle) : Bool :=
  match closureSignature? unit handle with
  | some (parameters, results) =>
      parameters.all (·.paramsAll (closureParameter unit handle)) &&
        results.all (·.paramsAll (closureParameter unit handle))
  | none => true

mutual
/-- Shallow typed terms over a local row, in a function returning `ρ`. -/
inductive Term (unit : Validation.ValidatedUnit) (ρ : ResultShape) : NRow → NTy → Type where
  /-- A literal. -/
  | lit {Γ : NRow} {τ : NTy} (value : τ.groundCarrier) (literal : τ.literal = true) :
      Term unit ρ Γ τ
  /-- A read of a local slot. -/
  | var {Γ : NRow} {τ : NTy} (x : Var Γ τ) : Term unit ρ Γ τ
  /-- Checked integer arithmetic. -/
  | checked {Γ : NRow} {width : Nat} {signed : Bool} (site : Nat) (op : CheckedOp)
      (failure : ThrowKind) (left right : Term unit ρ Γ (.int width signed)) :
      Term unit ρ Γ (.int width signed)
  /-- Modular integer arithmetic, wrapped into the width. -/
  | modular {Γ : NRow} {width : Nat} {signed : Bool} (op : ModularOp)
      (left right : Term unit ρ Γ (.int width signed)) : Term unit ρ Γ (.int width signed)
  /-- An ordered comparison of integers. -/
  | compare {Γ : NRow} {width : Nat} {signed : Bool} (op : CompareOp)
      (left right : Term unit ρ Γ (.int width signed)) : Term unit ρ Γ .bool
  /-- Structural equality, or its negation. -/
  | equal {Γ : NRow} {τ : NTy} (negated : Bool) (left right : Term unit ρ Γ τ) : Term unit ρ Γ .bool
  /-- Equality of the runtime encodings with ghost contents dropped, or its
  negation: in a unit declaring ghosts, at a type mentioning a type
  parameter, whose values' own equality ignores their ghosts only once
  instantiated (`doc/designs/ghost-fields.md`). -/
  | equalErased {Γ : NRow} {τ : NTy} (negated : Bool) (left right : Term unit ρ Γ τ) :
      Term unit ρ Γ .bool
  | not {Γ : NRow} (operand : Term unit ρ Γ .bool) : Term unit ρ Γ .bool
  /-- Boolean conjunction (`true`) or disjunction (`false`) of evaluated operands. -/
  | logical {Γ : NRow} (conjunction : Bool) (left right : Term unit ρ Γ .bool) : Term unit ρ Γ .bool
  /-- Bitwise operation on unsigned integers. -/
  | bitwise {Γ : NRow} {width : Nat} (op : BitOp)
      (left right : Term unit ρ Γ (.int width false)) : Term unit ρ Γ (.int width false)
  /-- Checked shift of an unsigned integer, left (`true`) or right. -/
  | shift {Γ : NRow} {width distanceWidth : Nat} (site : Nat) (left : Bool) (failure : ThrowKind)
      (value : Term unit ρ Γ (.int width false))
      (distance : Term unit ρ Γ (.int distanceWidth false)) :
      Term unit ρ Γ (.int width false)
  /-- A checked shift of any other operands, the distance's range first. -/
  | shiftSigned {Γ : NRow} {width distanceWidth : Nat} {signed distanceSigned : Bool} (site : Nat)
      (left : Bool) (failure : ThrowKind) (value : Term unit ρ Γ (.int width signed))
      (distance : Term unit ρ Γ (.int distanceWidth distanceSigned)) :
      Term unit ρ Γ (.int width signed)
  /-- A shift with no outcome where its distance is out of range. -/
  | shiftUnchecked {Γ : NRow} {width distanceWidth : Nat} {signed distanceSigned : Bool}
      (site : Nat) (left : Bool) (value : Term unit ρ Γ (.int width signed))
      (distance : Term unit ρ Γ (.int distanceWidth distanceSigned)) :
      Term unit ρ Γ (.int width signed)
  /-- A quotient (`modulo` false) or remainder wrapped into the width, with no
  outcome at a zero divisor. -/
  | divideUnchecked {Γ : NRow} {width : Nat} {signed : Bool} (site : Nat) (modulo : Bool)
      (left right : Term unit ρ Γ (.int width signed)) : Term unit ρ Γ (.int width signed)
  /-- A modular operation's value with whether its exact result left the width. -/
  | overflowing {Γ : NRow} {width : Nat} {signed : Bool} (op : ModularOp)
      (left right : Term unit ρ Γ (.int width signed)) :
      Term unit ρ Γ (.tuple (.cons (.int width signed) (.cons .bool .nil)))
  /-- An integer wrapped into another width: the `as` cast. -/
  | wrap {Γ : NRow} {width : Nat} {signed : Bool} {width' : Nat} {signed' : Bool}
      (nonzero : width' ≠ 0) (value : Term unit ρ Γ (.int width signed)) :
      Term unit ρ Γ (.int width' signed')
  /-- A character's code point. -/
  | charCode {Γ : NRow} (value : Term unit ρ Γ .char) : Term unit ρ Γ (.int 32 false)
  /-- The character of a code point, with no outcome where it is no Unicode
  scalar value. -/
  | charOf {Γ : NRow} {width : Nat} {signed : Bool} (site : Nat)
      (value : Term unit ρ Γ (.int width signed)) : Term unit ρ Γ .char
  /-- The character of a code point, throwing where it is no Unicode scalar
  value. -/
  | charOfChecked {Γ : NRow} {width : Nat} {signed : Bool} (site : Nat) (failure : ThrowKind)
      (value : Term unit ρ Γ (.int width signed)) : Term unit ρ Γ .char
  /-- A string's length in bytes, wrapped into the width. -/
  | stringLength {Γ : NRow} (value : Term unit ρ Γ .string) : Term unit ρ Γ (.int 64 false)
  /-- What follows a value of type `!`: no run gets here. -/
  | unreachable {Γ : NRow} {τ : NTy} : Term unit ρ Γ τ
  /-- Checked conversion between integer types. -/
  | cast {Γ : NRow} {width : Nat} {signed : Bool} {width' : Nat} {signed' : Bool}
      (site : Nat) (failure : ThrowKind)
      (value : Term unit ρ Γ (.int width signed)) : Term unit ρ Γ (.int width' signed')
  /-- A conditional, with its site where it is one of the source's decisions
  (not an `&&`, an `||` or an `assert!`). -/
  | ite {Γ : NRow} {τ : NTy} (site : Option Nat) (condition : Term unit ρ Γ .bool)
      (thenBranch elseBranch : Term unit ρ Γ τ) : Term unit ρ Γ τ
  /-- Bind a slot, then continue.  The slot stays written afterwards, as
  the runtime frame's does. -/
  | let_ {Γ : NRow} {σ τ : NTy} (x : Var Γ σ) (value : Term unit ρ Γ σ) (body : Term unit ρ Γ τ) :
      Term unit ρ Γ τ
  /-- Evaluate for effect, discard the value, then continue. -/
  | drop {Γ : NRow} {σ τ : NTy} (value : Term unit ρ Γ σ) (body : Term unit ρ Γ τ) : Term unit ρ Γ τ
  /-- Write a slot. -/
  | assign {Γ : NRow} {σ : NTy} (x : Var Γ σ) (value : Term unit ρ Γ σ) : Term unit ρ Γ .unit
  /-- A named constant: its closed initializer. -/
  | const {Γ : NRow} {τ : NTy} (value : Term unit ρ .nil τ) : Term unit ρ Γ τ
  /-- Throw with no argument, at its site.  A throw has every type. -/
  | throw0 {Γ : NRow} {τ : NTy} (site : Nat) (kind : ThrowKind) : Term unit ρ Γ τ
  /-- Throw with one evaluated integer argument, such as an abort code, at
  its site. -/
  | throw1 {Γ : NRow} {τ : NTy} {width : Nat} {signed : Bool} (site : Nat) (kind : ThrowKind)
      (code : Term unit ρ Γ (.int width signed)) : Term unit ρ Γ τ
  /-- Return the function's result.  A return has every type. -/
  | return_ {Γ : NRow} {τ : NTy} (value : Term unit ρ Γ ρ.bodyType) : Term unit ρ Γ τ
  /-- Leave `nest + 1` enclosing loops. -/
  | break_ {Γ : NRow} {τ : NTy} (nest : Nat) : Term unit ρ Γ τ
  /-- Restart the `nest + 1`-th enclosing loop. -/
  | continue_ {Γ : NRow} {τ : NTy} (nest : Nat) : Term unit ρ Γ τ
  /-- Repeat a body until it breaks; the loop's value is unit.  `site` is
  the loop's expression index, which names its invariant. -/
  | loop {Γ : NRow} (site : Nat) (body : Term unit ρ Γ .unit) : Term unit ρ Γ .unit
  /-- A direct call of a monomorphic function on evaluated arguments; its
  value is the callee's declared result. `site` is the call's expression
  index, which names the invariants owed after it. -/
  | call {Γ : NRow} {σs : NRow} (site : Nat) (handle : FunctionHandle) (shape : ResultShape)
      (arguments : Args unit ρ Γ σs) : Term unit ρ Γ shape.bodyType
  /-- A call with type arguments: the callee's own signature `σs` and
  `shape`, the arguments `θ` in the caller's types, the VIR generic
  arguments that instantiate the callee's frame, and the signature `θ`
  instantiates, evaluated by the compiler. -/
  | callGeneric {Γ : NRow} {σs σs' : NRow} (site : Nat) (handle : FunctionHandle)
      (typeArgs : Array GenericArgument) (θ : TypeArgs) (shape shape' : ResultShape)
      (resolved : (NRow.subst θ.1 σs == σs' && shape.subst θ.1 == shape') = true)
      (arguments : Args unit ρ Γ σs') :
      Term unit ρ Γ shape'.bodyType
  /-- A call at a type argument holding a mutable reference: the callee's rows
  substituted at `θ`, so its references are lent where its type says, and the VIR
  generic arguments that instantiate its frame. -/
  | callInstance {Γ : NRow} {σs : NRow} (site : Nat) (handle : FunctionHandle)
      (typeArgs : Array GenericArgument) (θ : TypeArgs) (shape : ResultShape)
      (arguments : Args unit ρ Γ σs) : Term unit ρ Γ shape.bodyType
  /-- A closure of a function without type arguments over evaluated
  captures: the target and how its parameters are captured or supplied. -/
  | closure {Γ full captured supplied : NRow} (handle : FunctionHandle)
      (weave : Weave full captured supplied) (results : NRow) (shared : Sharing)
      (rows : closureRows? unit handle weave.mask = some (captured, supplied, results))
      (sharing : closureShared unit handle weave.mask = shared)
      (closed : (captured.paramFree && supplied.paramFree && results.paramFree) = true)
      (faithful : closureFaithful unit handle #[] = true)
      (captures : Args unit ρ Γ captured) :
      Term unit ρ Γ (.function supplied shared results)
  /-- A closure with type arguments: the weave of the target's own
  parameters, its own result shape, the arguments `θ` in the caller's types,
  the VIR generic arguments that instantiate its frame, and the rows `θ`
  instantiates, evaluated by the compiler. -/
  | closureGeneric {Γ full captured supplied captured' supplied' : NRow} (handle : FunctionHandle)
      (weave : Weave full captured supplied) (typeArgs : Array GenericArgument) (θ : TypeArgs)
      (shape shape' : ResultShape) (shared : Sharing)
      (rows : closureRows? unit handle weave.mask = some (captured, supplied, shape.row))
      (sharing : closureShared unit handle weave.mask = shared)
      (below : closureSignatureBelow unit handle = true)
      (resolved : (NRow.subst θ.1 captured == captured' && NRow.subst θ.1 supplied == supplied' &&
        shape.subst θ.1 == shape') = true)
      (captures : Args unit ρ Γ captured') :
      Term unit ρ Γ (.function supplied' shared shape'.row)
  /-- An invocation of a function value on evaluated arguments, after the
  value itself; its value is the function's result. -/
  | invoke {Γ σs : NRow} {shared : Sharing} (site : Nat) (shape : ResultShape)
      (function : Term unit ρ Γ (.function σs shared shape.row))
      (arguments : Args unit ρ Γ σs) : Term unit ρ Γ shape.bodyType
  /-- A tuple of evaluated elements. -/
  | tuple {Γ σs : NRow} (elements : Args unit ρ Γ σs) : Term unit ρ Γ (.tuple σs)
  /-- A struct value from its evaluated fields. -/
  | pack {Γ σs : NRow} (site : Nat) (source : StructHandle) (arguments : NRow)
      (fields : Args unit ρ Γ σs)
      (ghosts : GhostInits σs) : Term unit ρ Γ (.struct source arguments σs)
  /-- An enum value of the chosen variant from its evaluated fields. -/
  | variant {Γ σs : NRow} {names : List String} {rows : NRows} (site : Nat)
      (source : StructHandle)
      (arguments : NRow) (distinct : names.Nodup) (choice : Which names rows σs)
      (fields : Args unit ρ Γ σs)
      (ghosts : GhostInits σs) : Term unit ρ Γ (.enum source arguments names rows distinct)
  /-- A field of a struct value. -/
  | field {Γ σs : NRow} {τ : NTy} {source : StructHandle} {arguments : NRow} (x : Var σs τ)
      (value : Term unit ρ Γ (.struct source arguments σs)) : Term unit ρ Γ τ
  /-- Whether an enum value holds one of the named variants. -/
  | isVariant {Γ : NRow} {names : List String} {rows : NRows} {source : StructHandle}
      {arguments : NRow} {distinct : names.Nodup} (tests : List String)
      (value : Term unit ρ Γ (.enum source arguments names rows distinct)) : Term unit ρ Γ .bool
  /-- A field of an enum value under the chosen variant; under any other,
  the profile's mismatch throw, as the runtime's, and without one no
  outcome. -/
  | payload {Γ : NRow} {τ : NTy} {names : List String} {rows : NRows} {source : StructHandle}
      {arguments : NRow} {distinct : names.Nodup} (mismatch : Option Failure)
      (choices : Choices names rows τ)
      (value : Term unit ρ Γ (.enum source arguments names rows distinct)) : Term unit ρ Γ τ
  /-- Bind the elements of a tuple to slots, then continue. -/
  | letRow {Γ σs : NRow} {τ : NTy} (targets : Vars Γ σs) (value : Term unit ρ Γ (.tuple σs))
      (body : Term unit ρ Γ τ) : Term unit ρ Γ τ
  /-- Bind the fields of a struct to slots, then continue. -/
  | letFields {Γ σs : NRow} {τ : NTy} {source : StructHandle} {arguments : NRow}
      (targets : Vars Γ σs) (value : Term unit ρ Γ (.struct source arguments σs))
      (body : Term unit ρ Γ τ) : Term unit ρ Γ τ
  /-- The first arm whose pattern matches the value and whose guard holds. -/
  | caseOf {Γ : NRow} {σ τ : NTy} (scrutinee : Term unit ρ Γ σ) (arms : Arms unit ρ Γ σ τ) : Term unit ρ Γ τ
  /-- The current value of a mutable reference. -/
  | deref {Γ : NRow} {τ : NTy} (site : Nat) (value : Term unit ρ Γ (.ref τ)) : Term unit ρ Γ τ
  /-- The current value of a mutable reference whose type carries no
  certificate: the certified and the plain carrier of such a type are the
  same (`NTy.carrier_eq_plain_of_certificateFree`), so the copy is the
  value cast along that equation, owes nothing and names no site.  The
  compiler emits it where `certificateFree` decides of the type, and
  `deref` otherwise (`doc/designs/data-invariants.md`). -/
  | copy {Γ : NRow} {τ : NTy} (value : Term unit ρ Γ (.ref τ))
      (free : τ.certificateFree = true) : Term unit ρ Γ τ
  /-- Move a local's value out: read it and empty the slot.  A mutable
  reference used as a value moves, so a function's exit and a loan's death
  resolve only the references a slot still holds. -/
  | take {Γ : NRow} {τ : NTy} (x : Var Γ τ) : Term unit ρ Γ τ
  /-- A mutable reference moved out of a place an overwrite follows, the loan's only
  holder from then on: the run clears the component, and the place keeps `(π, π)`, whose
  value nothing reads and whose resolution assumes nothing. -/
  | takeAt {Γ : NRow} {τ σ : NTy} (site : Nat) (mismatch : Option Failure) (x : Var Γ τ)
      (path : Proj τ (.ref σ)) : Term unit ρ Γ (.ref σ)
  /-- The death of the reference a local holds: execution continues only
  where its current value is its prophecy.  An empty slot resolves
  nothing; the reference moved on and resolves where it dies. -/
  | resolve {Γ : NRow} {σ : NTy} (site : Nat) (x : Var Γ (.ref σ)) (lender : Lender Γ σ) :
      Term unit ρ Γ .unit
  /-- The death of the reference a local holds at a path; nothing where the
  slot is empty or the path reaches no component.  The lender is the loan's,
  read from its fact as a direct holder's is (`lenderOf`). -/
  | resolveAt {Γ : NRow} {τ σ : NTy} (site : Nat) (x : Var Γ τ) (path : Proj τ (.ref σ))
      (lender : Lender Γ σ) : Term unit ρ Γ .unit
  /-- The data invariants of the current of the mutable reference a local
  holds, assumed: implicit in the parameter's type, as an integer's bound
  is, and owed by the caller where it handed the reference over
  (`Term.handedOver`).  The function's entry states it of each of its
  mutable-reference parameters, without which nothing of the referent is
  known and a callee that writes through it could not meet its exit
  obligation (`doc/designs/data-invariants.md`, "A call
  boundary").  An empty slot assumes nothing. -/
  | assumeCurrent {Γ : NRow} {σ : NTy} (x : Var Γ (.ref σ)) : Term unit ρ Γ .unit
  /-- A mutable reference handed over to a call: the reference itself, with
  the data invariants of its current owed at `site`, the argument's own,
  since the callee assumes them at its entry (`Term.assumeCurrent`). -/
  | handedOver {Γ : NRow} {σ : NTy} (site : Nat) (reference : Term unit ρ Γ (.ref σ)) :
      Term unit ρ Γ (.ref σ)
  /-- A mutable reference the function returns: the referent's current is
  certified where the function leaves it, the site a `&mut` parameter's own
  invariant is owed at (`Mutables.resolve`).  The slot the reference left is
  empty, so no resolution at the exit reaches it, and the caller reads the
  reference it receives as one whose invariants hold, exactly as a callee
  reads a `&mut` parameter (`Mutables.assumedAtEntry`).  The value is the
  reference itself.  The compiler states this where a function's result is a
  mutable reference over a referent that carries certificates, so no case on
  the result shape is left in the denotation. -/
  | returned {Γ : NRow} {σ : NTy} (site : Nat) (reference : Term unit ρ Γ (.ref σ)) :
      Term unit ρ Γ (.ref σ)
  /-- Replace the current value of the mutable reference a local holds. -/
  | mutate {Γ : NRow} {τ : NTy} (x : Var Γ (.ref τ)) (value : Term unit ρ Γ τ) : Term unit ρ Γ .unit
  /-- What a shared view of a value holds, each mutable reference as its
  current value.  A reference's current is plain (`NTy.carrier`), so the
  view is a copy out of plain currents, certified where it is taken
  (`certifyingAt`, `doc/plan/data-and-global-invariants.md`: a value copied
  out of a plain current is certified at the copy). -/
  | observe {Γ : NRow} {τ : NTy} (site : Nat) (value : Term unit ρ Γ τ) : Term unit ρ Γ τ.erase
  /-- Read the component of a local a path reaches. -/
  | readPlace {Γ : NRow} {τ σ : NTy} (site : Nat) (mismatch : Option Failure) (x : Var Γ τ)
      (path : Proj τ σ) : Term unit ρ Γ σ
  /-- Read, through a dereference, the component of a local a path reaches
  whose type carries no certificate: the component is certified by its type
  alone (`NTy.carrier_eq_plain_of_certificateFree`), so the copy out of a
  current the loan may have left open owes nothing and names no site.  The
  compiler emits it in place of `readPlace` where the path dereferences and
  `certificateFree` decides of the component's type
  (`doc/designs/data-invariants.md`). -/
  | copyPlace {Γ : NRow} {τ σ : NTy} (mismatch : Option Failure) (x : Var Γ τ)
      (path : Proj τ σ) (free : σ.certificateFree = true) : Term unit ρ Γ σ
  /-- Write the component of a local a path reaches. -/
  | writePlace {Γ : NRow} {τ σ : NTy} (site : Nat) (mismatch : Option Failure) (x : Var Γ τ)
      (path : Proj τ σ)
      (value : Term unit ρ Γ σ) : Term unit ρ Γ .unit
  /-- Borrow the component of a local a path reaches: a prophecy is
  chosen, the reference is the component with that prophecy, and the lender
  holds the prophecy at the path from then on. -/
  | borrowPlace {Γ : NRow} {τ σ : NTy} (mismatch : Option Failure) (x : Var Γ τ) (path : Proj τ σ) :
      Term unit ρ Γ (.ref σ)
  /-- Observe a slice without exposing a writable growable vector. -/
  | sliceCurrent {Γ : NRow} {τ : NTy} (value : Term unit ρ Γ (.sliceRef τ)) :
      Term unit ρ Γ (.slice τ)
  /-- Read slice contents as a logical sequence snapshot. -/
  | sliceVector {Γ : NRow} {τ : NTy} (value : Term unit ρ Γ (.slice τ)) :
      Term unit ρ Γ (.vector τ)
  /-- Form a shared slice view from checked backing storage. -/
  | vectorSlice {Γ : NRow} {τ : NTy} (value : Term unit ρ Γ (.vector τ)) :
      Term unit ρ Γ (.slice τ)
  | borrowArraySlice {Γ : NRow} {τ σ : NTy} {length : Nat}
      (mismatch : Option Failure) (x : Var Γ τ) (path : Proj τ (.array σ length)) :
      Term unit ρ Γ (.sliceRef σ)
  | borrowVectorSlice {Γ : NRow} {τ σ : NTy}
      (mismatch : Option Failure) (x : Var Γ τ) (path : Proj τ (.vector σ)) :
      Term unit ρ Γ (.sliceRef σ)
  | reborrowSlice {Γ : NRow} {τ σ : NTy}
      (mismatch : Option Failure) (x : Var Γ τ) (path : Proj τ (.sliceRef σ)) :
      Term unit ρ Γ (.sliceRef σ)
  | takeSliceAt {Γ : NRow} {τ σ : NTy}
      (mismatch : Option Failure) (x : Var Γ τ) (path : Proj τ (.sliceRef σ)) :
      Term unit ρ Γ (.sliceRef σ)
  | resolveSliceAt {Γ : NRow} {τ σ : NTy}
      (x : Var Γ τ) (path : Proj τ (.sliceRef σ)) : Term unit ρ Γ .unit
  /-- A vector of listed elements. -/
  | vectorLit {Γ : NRow} {τ : NTy} (count : Nat) (elements : Args unit ρ Γ (NRow.replicate count τ)) :
      Term unit ρ Γ (.vector τ)
  /-- An array's vector: its length a fact from here on. -/
  | arrayVector {Γ : NRow} {τ : NTy} {length : Nat} (array : Term unit ρ Γ (.array τ length)) :
      Term unit ρ Γ (.vector τ)
  /-- A vector of an array's length as the array, with no outcome at another
  length. -/
  | arrayOf {Γ : NRow} {τ : NTy} {length : Nat} (vector : Term unit ρ Γ (.vector τ)) :
      Term unit ρ Γ (.array τ length)
  /-- A vector holding one value `count` times. -/
  | repeatVector {Γ : NRow} {τ : NTy} (count : Nat) (value : Term unit ρ Γ τ) :
      Term unit ρ Γ (.vector τ)
  /-- The length of a vector, which its bound fits in `u64`. -/
  | length {Γ : NRow} {τ : NTy} (vector : Term unit ρ Γ (.vector τ)) : Term unit ρ Γ (.int 64 false)
  /-- The address a signer holds. -/
  | signerAddress {Γ : NRow} (signer : Term unit ρ Γ .signer) : Term unit ρ Γ .address
  /-- An element by position; aborts outside the vector. -/
  | index {Γ : NRow} {τ : NTy} {width : Nat} {signed : Bool} (site : Nat)
      (vector : Term unit ρ Γ (.vector τ))
      (position : Term unit ρ Γ (.int width signed)) : Term unit ρ Γ τ
  /-- The bounds check of an element place. -/
  | checkIndex {Γ : NRow} {τ : NTy} {width : Nat} {signed : Bool} (site : Nat) (failure : ThrowKind)
      (vector : Term unit ρ Γ (.vector τ))
      (position : Term unit ρ Γ (.int width signed)) : Term unit ρ Γ .unit
  /-- A vector with one more element at its back, as a value. -/
  | push {Γ : NRow} {τ : NTy} (vector : Term unit ρ Γ (.vector τ)) (element : Term unit ρ Γ τ) :
      Term unit ρ Γ (.vector τ)
  /-- A vector with an element inserted at a position; aborts past its end. -/
  | insert {Γ : NRow} {τ : NTy} {width : Nat} {signed : Bool} (site : Nat)
      (vector : Term unit ρ Γ (.vector τ)) (position : Term unit ρ Γ (.int width signed))
      (element : Term unit ρ Γ τ) : Term unit ρ Γ (.vector τ)
  /-- The element at a position with the vector without it; aborts outside
  the vector. -/
  | remove {Γ : NRow} {τ : NTy} {width : Nat} {signed : Bool} (site : Nat)
      (vector : Term unit ρ Γ (.vector τ))
      (position : Term unit ρ Γ (.int width signed)) :
      Term unit ρ Γ (.tuple (.cons τ (.cons (.vector τ) .nil)))
  /-- A vector with two elements exchanged; aborts outside the vector. -/
  | swap {Γ : NRow} {τ : NTy} {width : Nat} {signed : Bool} (site : Nat)
      (vector : Term unit ρ Γ (.vector τ))
      (left right : Term unit ρ Γ (.int width signed)) : Term unit ρ Γ (.vector τ)
  /-- The concatenation of two vectors. -/
  | concat {Γ : NRow} {τ : NTy} (left right : Term unit ρ Γ (.vector τ)) : Term unit ρ Γ (.vector τ)
  /-- The half-open slice of a vector; aborts on an invalid range. -/
  | slice {Γ : NRow} {τ : NTy} {width : Nat} {signed : Bool} (site : Nat)
      (vector : Term unit ρ Γ (.vector τ))
      (start stop : Term unit ρ Γ (.int width signed)) : Term unit ρ Γ (.vector τ)
  /-- A vector with a half-open range reversed; aborts on an invalid range. -/
  | reverseSlice {Γ : NRow} {τ : NTy} {width : Nat} {signed : Bool} (site : Nat)
      (vector : Term unit ρ Γ (.vector τ))
      (start stop : Term unit ρ Γ (.int width signed)) : Term unit ρ Γ (.vector τ)
  /-- Consume an empty vector; aborts on a nonempty one. -/
  | destroyEmpty {Γ : NRow} {τ : NTy} (site : Nat) (vector : Term unit ρ Γ (.vector τ)) :
      Term unit ρ Γ .unit
  /-- Whether some element equals the needle. -/
  | contains {Γ : NRow} {τ : NTy} (vector : Term unit ρ Γ (.vector τ)) (needle : Term unit ρ Γ τ) :
      Term unit ρ Γ .bool
  /-- Whether some element equals the needle, and the first such position. -/
  | indexOf {Γ : NRow} {τ : NTy} (vector : Term unit ρ Γ (.vector τ)) (needle : Term unit ρ Γ τ) :
      Term unit ρ Γ (.tuple (.cons .bool (.cons (.int 64 false) .nil)))
  /-- The structural order of two values (`-1`, `0`, `1`), enum variants
  by the unit's declaration order. -/
  | order {Γ : NRow} {τ : NTy} (orders : Validation.ValueOrders)
      (left right : Term unit ρ Γ τ) : Term unit ρ Γ (.int 8 true)
  /-- Evaluate a value, then an effect, and keep the value. -/
  | seqAfter {Γ : NRow} {τ : NTy} (value : Term unit ρ Γ τ) (effect : Term unit ρ Γ .unit) : Term unit ρ Γ τ
  /-- An in-body assertion, at its site: no effect, so that verification can
  name and check the asserted condition there. -/
  | assertion {Γ : NRow} (site : Nat) : Term unit ρ Γ .unit
  /-- A state anchor, at its site: no effect, so that verification can
  record the locals and state an assertion reads there. -/
  | anchor {Γ : NRow} (site : Nat) : Term unit ρ Γ .unit
  /-- An in-body assumption, at its site: a run continues where the
  condition the meanings read for the site holds (`Meanings.assumption`). -/
  | assume {Γ : NRow} (site : Nat) : Term unit ρ Γ .unit
  /-- Any value of its type: a ghost field fresh at a pack, whose runtime
  value is one of them (`doc/designs/ghost-fields.md`). -/
  | arbitrary {Γ : NRow} {τ : NTy} : Term unit ρ Γ τ
  /-- The value a ghost write takes at its site, an `update`'s: the
  meanings' for the site (`Meanings.ghost`), or any value of its type. -/
  | ghostValue {Γ : NRow} {τ : NTy} (site : Nat) : Term unit ρ Γ τ
  /-- A value held in a ghost slot. -/
  | ghostSlot {Γ : NRow} {τ : NTy} (value : Term unit ρ Γ τ) : Term unit ρ Γ (.ghost τ)
  /-- A ghost slot's contents. -/
  | ghostRead {Γ : NRow} {τ : NTy} (slot : Term unit ρ Γ (.ghost τ)) : Term unit ρ Γ τ
  /-- The end of a mutation of a local whose type carries a data invariant,
  at its site: no effect, so that verification can owe the invariant there. -/
  | mutationEnd {Γ : NRow} (site : Nat) : Term unit ρ Γ .unit
  /-- A value constructed at a site whose type carries a data invariant: the
  value itself, so that verification can owe the invariant of it there. -/
  | constructed {Γ : NRow} {τ : NTy} (site : Nat) (value : Term unit ρ Γ τ) : Term unit ρ Γ τ
  /-- The resource of type `τ`, with its declaration's type arguments, at a
  key; aborts when none is published. -/
  | globalRead {Γ : NRow} {κ τ : NTy} (site : Nat) (arguments : NRow) (key : Term unit ρ Γ κ) :
      Term unit ρ Γ τ
  /-- Whether a resource of type `τ` is published at a key. -/
  | globalContains {Γ : NRow} {κ : NTy} (τ : NTy) (arguments : NRow) (key : Term unit ρ Γ κ) :
      Term unit ρ Γ .bool
  /-- A mutable borrow of a published resource: a prophecy is chosen, the
  reference is the resource with that prophecy, and memory holds the
  prophecy at the key from then on. The memory before the write is
  anchored at `site`, once the key is evaluated. -/
  | globalBorrow {Γ : NRow} {κ τ : NTy} (site : Nat) (arguments : NRow) (key : Term unit ρ Γ κ) :
      Term unit ρ Γ (.ref τ)
  /-- Publish a resource at a key; aborts when one is already there. The
  memory before the write is anchored at `site`. -/
  | globalPublish {Γ : NRow} {κ τ : NTy} (site : Nat) (arguments : NRow) (key : Term unit ρ Γ κ)
      (value : Term unit ρ Γ τ) : Term unit ρ Γ .unit
  /-- Take a resource from a key; aborts when none is published. The memory
  before the write is anchored at `site`. -/
  | globalTake {Γ : NRow} {κ τ : NTy} (site : Nat) (arguments : NRow) (key : Term unit ρ Γ κ) :
      Term unit ρ Γ τ
  /-- A write of global memory has ended at its site: no effect, so that
  verification can owe there the invariants of the memory written. -/
  | memoryWritten {Γ : NRow} (site : Nat) : Term unit ρ Γ .unit
  /-- A spec variable's write: the ghost resource at a key replaced, which
  only the model's memory holds (`doc/designs/ghost-fields.md`). The memory
  before the write is anchored at `site`. -/
  | ghostStore {Γ : NRow} {κ τ : NTy} (site : Nat) (arguments : NRow) (key : Term unit ρ Γ κ)
      (value : Term unit ρ Γ τ) : Term unit ρ Γ .unit

/-- An argument row, evaluated left to right.  A reborrowed argument is a
borrow like any other: the callee receives the reference and resolves its
prophecy, which the lender already holds. -/
inductive Args (unit : Validation.ValidatedUnit) (ρ : ResultShape) : NRow → NRow → Type where
  | nil {Γ : NRow} : Args unit ρ Γ .nil
  | cons {Γ : NRow} {σ : NTy} {σs : NRow} (head : Term unit ρ Γ σ) (tail : Args unit ρ Γ σs) :
      Args unit ρ Γ (.cons σ σs)

/-- The arms of a match over values of `σ`, in order, ending in the throw
a value no arm takes makes, if its profile has one (`patternMismatchThrow?`). -/
inductive Arms (unit : Validation.ValidatedUnit) (ρ : ResultShape) : NRow → NTy → NTy → Type where
  | nil {Γ : NRow} {σ τ : NTy} (mismatch : Option Failure) : Arms unit ρ Γ σ τ
  /-- An arm, with the site of the source's `match` it belongs to and the
  variant it takes (`_` for another pattern), none for a destructuring. -/
  | cons {Γ : NRow} {σ τ : NTy} (arm : Option (Nat × String)) (pattern : Pat Γ σ)
      (body : Term unit ρ Γ τ) (rest : Arms unit ρ Γ σ τ) : Arms unit ρ Γ σ τ
  /-- An arm taken only where its guard, evaluated after the binding, holds. -/
  | guarded {Γ : NRow} {σ τ : NTy} (arm : Option (Nat × String)) (pattern : Pat Γ σ)
      (guard : Term unit ρ Γ .bool)
      (body : Term unit ρ Γ τ) (rest : Arms unit ρ Γ σ τ) : Arms unit ρ Γ σ τ
end

/-- A field projection retaining the source type named by quotation. Its
layout certificate converts only the indexed term; the interpreter can keep
that source type when it evaluates the value being projected. -/
def Term.fieldAt {unit : Validation.ValidatedUnit} {ρ : ResultShape} {Γ : NRow}
    (owner : NTy) {source : StructHandle} {arguments fields : NRow} {field : NTy}
    (layout : owner = .struct source arguments fields) (position : Var fields field)
    (value : Term unit ρ Γ owner) : Term unit ρ Γ field := by
  cases layout
  exact .field position value

/-- A struct construction retaining the nominal result type named by quotation. -/
def Term.packAt {unit : Validation.ValidatedUnit} {ρ : ResultShape} {Γ : NRow}
    (owner : NTy) {source : StructHandle} {arguments fields : NRow}
    (layout : owner = .struct source arguments fields) (site : Nat)
    (values : Args unit ρ Γ fields) (ghosts : GhostInits fields) : Term unit ρ Γ owner := by
  cases layout
  exact .pack site source arguments values ghosts

/-- Destructure at the source owner's type before observing its structural row. -/
def Term.letFieldsAt {unit : Validation.ValidatedUnit} {ρ : ResultShape} {Γ : NRow}
    (owner : NTy) {source : StructHandle} {arguments fields : NRow} {result : NTy}
    (layout : owner = .struct source arguments fields) (targets : Vars Γ fields)
    (value : Term unit ρ Γ owner) (body : Term unit ρ Γ result) : Term unit ρ Γ result := by
  cases layout
  exact .letFields targets value body

/-- Enum operations keep the quoted owner just as struct operations do. -/
def Term.variantAt {unit : Validation.ValidatedUnit} {ρ : ResultShape} {Γ : NRow}
    (owner : NTy) {source : StructHandle} {arguments fields : NRow}
    {names : List String} {rows : NRows} {distinct : names.Nodup}
    (layout : owner = .enum source arguments names rows distinct) (site : Nat)
    (choice : Which names rows fields) (values : Args unit ρ Γ fields)
    (ghosts : GhostInits fields) : Term unit ρ Γ owner := by
  cases layout
  exact .variant site source arguments distinct choice values ghosts

def Term.isVariantAt {unit : Validation.ValidatedUnit} {ρ : ResultShape} {Γ : NRow}
    (owner : NTy) {source : StructHandle} {arguments : NRow}
    {names : List String} {rows : NRows} {distinct : names.Nodup}
    (layout : owner = .enum source arguments names rows distinct) (tests : List String)
    (value : Term unit ρ Γ owner) : Term unit ρ Γ .bool := by
  cases layout
  exact .isVariant tests value

def Term.payloadAt {unit : Validation.ValidatedUnit} {ρ : ResultShape} {Γ : NRow}
    (owner : NTy) {source : StructHandle} {arguments : NRow}
    {names : List String} {rows : NRows} {distinct : names.Nodup} {result : NTy}
    (layout : owner = .enum source arguments names rows distinct) (mismatch : Option Failure)
    (choices : Choices names rows result) (value : Term unit ρ Γ owner) : Term unit ρ Γ result := by
  cases layout
  exact .payload mismatch choices value

/-- The mutable-reference parameters of a function, in parameter order: the
slots whose references it resolves at every exit. -/
inductive Mutables (Γ : NRow) : Type where
  | nil : Mutables Γ
  | cons {τ : NTy} (x : Var Γ (.ref τ)) (rest : Mutables Γ) : Mutables Γ
  /-- A reference an aggregate slot holds at a path. -/
  | at {τ σ : NTy} (x : Var Γ τ) (path : Proj τ (.ref σ)) (rest : Mutables Γ) : Mutables Γ
  | sliceAt {τ σ : NTy} (x : Var Γ τ) (path : Proj τ (.sliceRef σ))
      (rest : Mutables Γ) : Mutables Γ

/-- The death of a loan.  Where a local lends the referent, the owner
rebuilt with the reference's final current is certified first — its data
invariants at every depth, which is the bottom-up `PackRefDeep` by the
structure of `NTy.deepHolds` — and only then are the prophecies' equations
assumed, so a run that breaks an invariant has no outcome before an equation
could prune it.  Where no local lends it, the current itself is certified
(`doc/designs/data-invariants.md`). -/
noncomputable def Lender.resolve {ρ : ResultShape} {Γ : NRow} {σ : NTy} (site : Nat)
    (env : HEnv Γ) (reference : (NTy.ref σ).carrier) : Lender Γ σ →
    Spec State Failure (Flow ρ Γ Unit)
  | .slot =>
      Spec.bind (certifyingAt site .mutationEnd σ reference.1) fun _ =>
        Spec.bind (Spec.assume (reference.1 = NTy.forget σ reference.2)) fun _ =>
          Spec.pure (.value () env)
  | .place (τ := τ) owner path =>
      match owner.get env with
      | some held =>
          match path.set? env reference.1 (NTy.forget τ held) with
          | some rebuilt =>
              -- A write-back into a reference's current owes nothing: the
              -- prophecy the owner keeps is certified already and the
              -- current is certified where the owner's own loan dies, as a
              -- write through a reference owes nothing (`Proj.setOwner?`).
              -- An owner that is a root owes its data invariants at every
              -- depth.
              if path.hasDeref then
                Spec.bind (Spec.assume (rebuilt = NTy.forget τ held)) fun _ =>
                  Spec.bind (Spec.assume (reference.1 = NTy.forget σ reference.2)) fun _ =>
                    Spec.pure (.value () env)
              else
                Spec.bind (certifyingAt site .mutationEnd τ rebuilt) fun _ =>
                  Spec.bind (Spec.assume (rebuilt = NTy.forget τ held)) fun _ =>
                    Spec.bind (Spec.assume (reference.1 = NTy.forget σ reference.2)) fun _ =>
                      Spec.pure (.value () env)
          | none => Spec.bottom
      | none =>
          Spec.bind (certifyingAt site .mutationEnd σ reference.1) fun _ =>
            Spec.bind (Spec.assume (reference.1 = NTy.forget σ reference.2)) fun _ =>
              Spec.pure (.value () env)
  | .candidate (τ := τ) owner path =>
      -- No equation: this holder need not hold this loan, so nothing here
      -- may constrain the owner.  Only the obligation is stated, which a
      -- holder that does hold the loan owes and a holder that does not owes
      -- of a value that cannot arise, never fewer than `place` owes.  A
      -- path the owner cannot take is no candidate and owes nothing;
      -- `Spec.bottom` would discharge every obligation instead.
      match owner.get env with
      | some held =>
          match path.set? env reference.1 (NTy.forget τ held) with
          | some rebuilt =>
              if path.hasDeref then Spec.pure (.value () env)
              else
                Spec.bind (certifyingAt site .mutationEnd τ rebuilt) fun _ =>
                  Spec.pure (.value () env)
          | none => Spec.pure (.value () env)
      | none => Spec.pure (.value () env)
  | .referent =>
      Spec.bind (certifyingAt site .mutationEnd σ reference.1) fun _ =>
        Spec.pure (.value () env)
  | .equation =>
      Spec.bind (Spec.assume (reference.1 = NTy.forget σ reference.2)) fun _ =>
        Spec.pure (.value () env)

@[simp] theorem Lender.resolve_slot {ρ : ResultShape} {Γ : NRow} {σ : NTy} (site : Nat)
    (env : HEnv Γ) (reference : (NTy.ref σ).carrier) :
    (Lender.slot : Lender Γ σ).resolve (State := State) (ρ := ρ) site env reference =
      Spec.bind (certifyingAt site .mutationEnd σ reference.1) fun _ =>
        Spec.bind (Spec.assume (reference.1 = NTy.forget σ reference.2)) fun _ =>
          Spec.pure (.value () env) := rfl

@[simp] theorem Lender.resolve_place {ρ : ResultShape} {Γ : NRow} {σ τ : NTy} (site : Nat)
    (env : HEnv Γ) (reference : (NTy.ref σ).carrier) (owner : Var Γ τ) (path : Proj τ σ) :
    (Lender.place owner path).resolve (State := State) (ρ := ρ) site env reference =
      match owner.get env with
      | some held =>
          match path.set? env reference.1 (NTy.forget τ held) with
          | some rebuilt =>
              if path.hasDeref then
                Spec.bind (Spec.assume (rebuilt = NTy.forget τ held)) fun _ =>
                  Spec.bind (Spec.assume (reference.1 = NTy.forget σ reference.2)) fun _ =>
                    Spec.pure (.value () env)
              else
                Spec.bind (certifyingAt site .mutationEnd τ rebuilt) fun _ =>
                  Spec.bind (Spec.assume (rebuilt = NTy.forget τ held)) fun _ =>
                    Spec.bind (Spec.assume (reference.1 = NTy.forget σ reference.2)) fun _ =>
                      Spec.pure (.value () env)
          | none => Spec.bottom
      | none =>
          Spec.bind (certifyingAt site .mutationEnd σ reference.1) fun _ =>
            Spec.bind (Spec.assume (reference.1 = NTy.forget σ reference.2)) fun _ =>
              Spec.pure (.value () env) := rfl

@[simp] theorem Lender.resolve_candidate {ρ : ResultShape} {Γ : NRow} {σ τ : NTy} (site : Nat)
    (env : HEnv Γ) (reference : (NTy.ref σ).carrier) (owner : Var Γ τ) (path : Proj τ σ) :
    (Lender.candidate owner path).resolve (State := State) (ρ := ρ) site env reference =
      match owner.get env with
      | some held =>
          match path.set? env reference.1 (NTy.forget τ held) with
          | some rebuilt =>
              if path.hasDeref then Spec.pure (.value () env)
              else
                Spec.bind (certifyingAt site .mutationEnd τ rebuilt) fun _ =>
                  Spec.pure (.value () env)
          | none => Spec.pure (.value () env)
      | none => Spec.pure (.value () env) := rfl

@[simp] theorem Lender.resolve_referent {ρ : ResultShape} {Γ : NRow} {σ : NTy} (site : Nat)
    (env : HEnv Γ) (reference : (NTy.ref σ).carrier) :
    (Lender.referent : Lender Γ σ).resolve (State := State) (ρ := ρ) site env reference =
      Spec.bind (certifyingAt site .mutationEnd σ reference.1) fun _ =>
        Spec.pure (.value () env) := rfl

@[simp] theorem Lender.resolve_equation {ρ : ResultShape} {Γ : NRow} {σ : NTy} (site : Nat)
    (env : HEnv Γ) (reference : (NTy.ref σ).carrier) :
    (Lender.equation : Lender Γ σ).resolve (State := State) (ρ := ρ) site env reference =
      Spec.bind (Spec.assume (reference.1 = NTy.forget σ reference.2)) fun _ =>
        Spec.pure (.value () env) := rfl

/-- Resolve every mutable reference the function still holds where it
leaves, a parameter's in its slot or in a local it was moved into, then
continue.  A reference moved into the result resolves at the caller. -/
-- Certificate-free referents need no certification step. The expansion
-- theorem below proves this omission equal to the asserting interpretation.
noncomputable def Mutables.resolve {Γ : NRow} {α : Type} (site : Nat) :
    Mutables Γ → HEnv Γ → Spec State Failure α → Spec State Failure α
  | .nil, _, next => next
  | .cons (τ := τ) x rest, env, next =>
      match x.get env with
      | some reference =>
          let continues :=
            Spec.bind (Spec.assume (reference.1 = NTy.forget τ reference.2)) fun _ =>
              rest.resolve site env next
          if τ.certificateFree then continues
          else Spec.bind (certifyingAt site .exit τ reference.1) fun _ => continues
      | none => rest.resolve site env next
  -- A reference held at a path owes its current's data invariants at the
  -- exit as one held in its own slot does (`certifyingAt`); the settle
  -- equation then compares the plain readings.
  | .at (τ := τ) x path rest, env, next =>
      match (x.get env).bind fun held => path.get? env (NTy.forget τ held) with
      | some reference =>
          Spec.bind (certifyingAt site .exit _ reference.1) fun _ =>
            Spec.bind (Spec.assume (reference.1 = reference.2)) fun _ =>
              rest.resolve site env next
      | none => rest.resolve site env next
  | .sliceAt (τ := τ) x path rest, env, next =>
      match (x.get env).bind fun held => path.get? env (NTy.forget τ held) with
      | some reference => Spec.bind (Spec.assume (reference.current = reference.final)) fun _ =>
          rest.resolve site env next
      | none => rest.resolve site env next

@[simp] theorem Mutables.resolve_nil {Γ : NRow} {α : Type} (site : Nat) (env : HEnv Γ)
    (next : Spec State Failure α) :
    (Mutables.nil : Mutables Γ).resolve site env next = next := rfl
@[simp] theorem Mutables.resolve_cons {Γ : NRow} {α : Type} {τ : NTy} (site : Nat)
    (x : Var Γ (.ref τ)) (rest : Mutables Γ) (env : HEnv Γ) (next : Spec State Failure α) :
    (Mutables.cons x rest).resolve site env next =
      match x.get env with
      | some reference =>
          Spec.bind (certifyingAt site .exit τ reference.1) fun _ =>
            Spec.bind (Spec.assume (reference.1 = NTy.forget τ reference.2)) fun _ =>
              rest.resolve site env next
      | none => rest.resolve site env next := by
  simp only [Mutables.resolve]
  cases h : x.get env with
  | none => rfl
  | some reference =>
      by_cases free : τ.certificateFree = true
      · simp only [free, ↓reduceIte]
        have cert : certifyingAt (State := State) site .exit τ reference.1 =
            Spec.pure (NTy.ofDeepHolds τ reference.1
              (NTy.deepHolds_of_certificateFree τ free reference.1)) :=
          certifying_eq_pure (NTy.deepHolds_of_certificateFree τ free reference.1) (fun _ => rfl)
        rw [cert, Spec.pure_bind]
      · simp only [free, Bool.false_eq_true, ↓reduceIte]

@[simp] theorem Mutables.resolve_at {Γ : NRow} {α : Type} {τ σ : NTy} (site : Nat) (x : Var Γ τ)
    (path : Proj τ (.ref σ)) (rest : Mutables Γ) (env : HEnv Γ) (next : Spec State Failure α) :
    (Mutables.at x path rest).resolve site env next =
      match (x.get env).bind fun held => path.get? env (NTy.forget τ held) with
      | some reference =>
          Spec.bind (certifyingAt site .exit σ reference.1) fun _ =>
            Spec.bind (Spec.assume (reference.1 = reference.2)) fun _ =>
              rest.resolve site env next
      | none => rest.resolve site env next := rfl

@[simp] theorem Mutables.resolve_sliceAt {Γ : NRow} {α : Type} {τ σ : NTy} (site : Nat)
    (x : Var Γ τ) (path : Proj τ (.sliceRef σ)) (rest : Mutables Γ)
    (env : HEnv Γ) (next : Spec State Failure α) :
    (Mutables.sliceAt x path rest).resolve site env next =
      match (x.get env).bind fun held => path.get? env (NTy.forget τ held) with
      | some reference => Spec.bind (Spec.assume (reference.current = reference.final)) fun _ =>
          rest.resolve site env next
      | none => rest.resolve site env next := rfl

/-- Resolving scalar references introduces only their prophecy equations;
there is no certificate to establish. The fast path is definitionally pure. -/
@[defeq, vir_denote 2000] theorem Mutables.resolve_cons_int {Γ : NRow} {α : Type}
    (width : Nat) (signed : Bool) (site : Nat) (x : Var Γ (.ref (.int width signed)))
    (rest : Mutables Γ) (env : HEnv Γ) (next : Spec State Failure α) :
    (Mutables.cons x rest).resolve site env next =
      match x.get env with
      | some reference =>
          Spec.bind (Spec.assume (reference.1 = reference.2)) fun _ => rest.resolve site env next
      | none => rest.resolve site env next := by
  rw [Mutables.resolve]
  cases h : x.get env <;> rfl

end LocalOperations

section Runtime
variable {unit : Validation.ValidatedUnit} [FunctionCarriers unit] [Skolems unit]

/-- A callee's value in the caller's view holds the variant it holds at the
callee's family. -/
theorem variantPlainName_ofSkolem {unit : Validation.ValidatedUnit}
    [FunctionCarriers unit] [Θ : Skolems unit]
    (θ : TypeArgs) : (names : List String) →
    (rows : NRows) → (value : @variantPlain (Carriers.instantiate θ Θ.toCarriers) names rows) →
    variantPlainName names (rows.subst θ.1) (variantPlain.ofSkolem θ names rows value) =
      @variantPlainName (Carriers.instantiate θ Θ.toCarriers) names rows value
  | _, .nil, value => nomatch value
  | [], .cons _ _, value => nomatch value
  | _ :: _, .cons _ _, .inl _ => rfl
  | _ :: names, .cons _ rest, .inr value => variantPlainName_ofSkolem θ names rest value


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
or with `prophecies` its prophecy, under its loan; a tuple, a struct and an
enum's active variant lend their components; any other value is its
encoding.  Returns the unused loans. -/
noncomputable def NTy.lend : (τ : NTy) → Bool → τ.carrier → List Nat → Option (RuntimeValue × List Nat)
  | .ref σ, prophecies, value, loan :: loans =>
      -- The references inside a referent stay lent under the following
      -- loans, at the plain layer both readings live at: the prophecy's
      -- certificates say nothing about its encoding (`plainEncode_forget`).
      (σ.plainLend prophecies (if prophecies then σ.forget value.2 else value.1) loans).map
        fun lent => (.borrow loan lent.1, lent.2)
  | .ref _, _, _, [] => none
  | .sliceRef σ, prophecies, value, loan :: loans =>
      some (.borrow loan (NTy.encode (.vector σ)
        (if prophecies then value.finalVector else value.currentVector)), loans)
  | .sliceRef _, _, _, [] => none
  | .tuple elements, prophecies, values, loans =>
      (NRow.lend elements prophecies values loans).map fun lent => (.tuple lent.1.toArray, lent.2)
  | .struct source _ fields, prophecies, values, loans =>
      (HListPlain.lend fields prophecies values.val loans).map fun lent =>
        (.nominal source none lent.1.toArray, lent.2)
  | .unit, _, value, loans => some (NTy.encode .unit value, loans)
  | .bool, _, value, loans => some (NTy.encode .bool value, loans)
  | .int width signed, _, value, loans => some (NTy.encode (.int width signed) value, loans)
  | .num, _, value, loans => some (NTy.encode .num value, loans)
  | .address, _, value, loans => some (NTy.encode .address value, loans)
  | .signer, _, value, loans => some (NTy.encode .signer value, loans)
  | .string, _, value, loans => some (NTy.encode .string value, loans)
  | .bytes, _, value, loans => some (NTy.encode .bytes value, loans)
  | .char, _, value, loans => some (NTy.encode .char value, loans)
  | .enum source _ names rows _, prophecies, value, loans =>
      lendVariantPlain source names rows prophecies value.val loans
  | .vector element, _, value, loans => some (NTy.encode (.vector element) value, loans)
  | .slice element, _, value, loans => some (NTy.encode (.slice element) value, loans)
  | .array element length, _, value, loans => some (NTy.encode (.array element length) value, loans)
  | .table key value, _, entries, loans => some (NTy.encode (.table key value) entries, loans)
  | .ghost slot, _, value, loans => some (NTy.encode (.ghost slot) value, loans)
  | .param index, _, value, loans => some (NTy.encode (.param index) value, loans)
  | .function parameters shared results, _, value, loans =>
      some (NTy.encode (.function parameters shared results) value, loans)
  | .nominal source arguments productive, _, value, loans =>
      some (NTy.encode (.nominal source arguments productive) value, loans)
  | .observed referent, _, value, loans => some (NTy.encode (.observed referent) value, loans)

/-- The runtime values of a row lent under loans taken in order. -/
noncomputable def NRow.lend : (row : NRow) → Bool → HList row → List Nat →
    Option (List RuntimeValue × List Nat)
  | .nil, _, _, loans => some ([], loans)
  | .cons τ rest, prophecies, values, loans =>
      (τ.lend prophecies values.1 loans).bind fun head =>
        (NRow.lend rest prophecies values.2 head.2).map fun tail => (head.1 :: tail.1, tail.2)

/-- A plain value lent: the layer a reference's readings live at
(`NTy.carrier`), where the references inside a current or a prophecy keep
their own loans. -/
noncomputable def NTy.plainLend : (τ : NTy) → Bool → τ.plain → List Nat →
    Option (RuntimeValue × List Nat)
  | .ref σ, prophecies, value, loan :: loans =>
      (σ.plainLend prophecies (if prophecies then value.2 else value.1) loans).map fun lent =>
        (.borrow loan lent.1, lent.2)
  | .ref _, _, _, [] => none
  | .sliceRef σ, prophecies, value, loan :: loans =>
      some (.borrow loan (NTy.plainEncode (.vector σ)
        (if prophecies then value.finalVector else value.currentVector)), loans)
  | .sliceRef _, _, _, [] => none
  | .tuple elements, prophecies, values, loans =>
      (HListPlain.lend elements prophecies values loans).map fun lent =>
        (.tuple lent.1.toArray, lent.2)
  | .struct source _ fields, prophecies, values, loans =>
      (HListPlain.lend fields prophecies values loans).map fun lent =>
        (.nominal source none lent.1.toArray, lent.2)
  | .enum source _ names rows _, prophecies, value, loans =>
      lendVariantPlain source names rows prophecies value loans
  | .unit, _, value, loans => some (NTy.plainEncode .unit value, loans)
  | .bool, _, value, loans => some (NTy.plainEncode .bool value, loans)
  | .int width signed, _, value, loans => some (NTy.plainEncode (.int width signed) value, loans)
  | .num, _, value, loans => some (NTy.plainEncode .num value, loans)
  | .address, _, value, loans => some (NTy.plainEncode .address value, loans)
  | .signer, _, value, loans => some (NTy.plainEncode .signer value, loans)
  | .string, _, value, loans => some (NTy.plainEncode .string value, loans)
  | .bytes, _, value, loans => some (NTy.plainEncode .bytes value, loans)
  | .char, _, value, loans => some (NTy.plainEncode .char value, loans)
  | .vector element, _, value, loans => some (NTy.plainEncode (.vector element) value, loans)
  | .slice element, _, value, loans => some (NTy.plainEncode (.slice element) value, loans)
  | .array element length, _, value, loans =>
      some (NTy.plainEncode (.array element length) value, loans)
  | .table key value, _, entries, loans => some (NTy.plainEncode (.table key value) entries, loans)
  | .ghost slot, _, value, loans => some (NTy.plainEncode (.ghost slot) value, loans)
  | .param index, _, value, loans => some (NTy.plainEncode (.param index) value, loans)
  | .function parameters shared results, _, value, loans =>
      some (NTy.plainEncode (.function parameters shared results) value, loans)
  | .nominal source arguments productive, _, value, loans =>
      some (NTy.plainEncode (.nominal source arguments productive) value, loans)
  | .observed referent, _, value, loans => some (NTy.plainEncode (.observed referent) value, loans)

/-- The runtime value of an enum's plain value: its active variant's fields lent. -/
noncomputable def lendVariantPlain (source : StructHandle) : (names : List String) →
    (rows : NRows) → Bool → variantPlain names rows → List Nat →
    Option (RuntimeValue × List Nat)
  | _, .nil, _, value, _ => nomatch value
  | [], .cons _ _, _, value, _ => nomatch value
  | name :: _, .cons fields _, prophecies, .inl values, loans =>
      (HListPlain.lend fields prophecies values loans).map fun lent =>
        (.nominal source (some name) lent.1.toArray, lent.2)
  | _ :: names, .cons _ rest, prophecies, .inr later, loans =>
      lendVariantPlain source names rest prophecies later loans

/-- The runtime values of a plain row lent under loans taken in order. -/
noncomputable def HListPlain.lend : (row : NRow) → Bool → HListPlain row → List Nat →
    Option (List RuntimeValue × List Nat)
  | .nil, _, _, loans => some ([], loans)
  | .cons τ rest, prophecies, values, loans =>
      (τ.plainLend prophecies values.1 loans).bind fun head =>
        (HListPlain.lend rest prophecies values.2 head.2).map fun tail =>
          (head.1 :: tail.1, tail.2)
end

attribute [vir_denote_norm] NTy.plainViewed HListPlain.plainViewed variantPlainViewed
attribute [vir_denote_norm] NTy.viewed HList.viewed variantViewed
attribute [vir_denote_norm] NTy.plainObserve HListPlain.plainObserve variantPlainObserve
attribute [vir_denote_norm] NTy.plainUnerase HListPlain.plainUnerase variantPlainUnerase

/-- The runtime argument row of native arguments lent under exactly the
given loans: each argument reference passes its current value. -/
noncomputable def lendArguments (σs : NRow) (args : HList σs) (loans : List Nat) : Option (Array RuntimeValue) :=
  match NRow.lend σs false args loans with
  | some (row, []) => some row.toArray
  | _ => none

/-- The runtime result row of a native result lent under exactly the given
loans, with each returned reference's current value or, with
`prophecies`, its prophecy. -/
noncomputable def ResultShape.lend : (shape : ResultShape) → Bool → shape.carrier → List Nat →
    Option (Array RuntimeValue)
  | .none, _, _, [] => some #[]
  | .none, _, _, _ :: _ => Option.none
  | .one τ, prophecies, value, loans =>
      match τ.lend prophecies value loans with
      | some (raw, []) => some #[raw]
      | _ => Option.none

mutual
/-- Every reference a value holds resolves at its prophecy: either the callee
exported its loan with a value that, with the returned references' holes
filled from `returned`, is the prophecy lent, or the callee returned the loan
itself, carrying the prophecy.  What a prophecy holds in turn stays lent under
its own loans, which travel with it. -/
def NTy.resolves : (τ : NTy) → τ.carrier → List Nat → Array RuntimeValue →
    List (Nat × RuntimeValue) → Prop
  | .ref σ, value, loan :: loans, returned, exports =>
      ∃ lent, σ.plainLend false (σ.forget value.2) loans = some lent ∧
        match exportedRaw? loan exports with
        | some exported =>
            VerexIR.SemanticOperations.resolveReturnedBorrows returned exported = lent.1
        | none => ∃ entry ∈ returned.toList,
            (loan, lent.1) ∈ (VerexIR.SemanticOperations.outermostBorrows entry).toList
  | .ref _, _, [], _, _ => False
  | .sliceRef σ, value, loan :: _, returned, exports =>
      let final := NTy.encode (.vector σ) value.finalVector
      match exportedRaw? loan exports with
      | some exported => VerexIR.SemanticOperations.resolveReturnedBorrows returned exported = final
      | none => ∃ entry ∈ returned.toList,
          (loan, final) ∈ (VerexIR.SemanticOperations.outermostBorrows entry).toList
  | .sliceRef _, _, [], _, _ => False
  | .tuple elements, values, loans, returned, exports =>
      argumentsResolve elements values loans returned exports
  | .struct _ _ fields, values, loans, returned, exports =>
      plainArgumentsResolve fields values.val loans returned exports
  | .enum _ _ names rows _, value, loans, returned, exports =>
      variantPlainResolves names rows value.val loans returned exports
  | .unit, _, _, _, _ | .bool, _, _, _, _ | .int _ _, _, _, _, _ | .num, _, _, _, _
  | .address, _, _, _, _ | .signer, _, _, _, _ | .string, _, _, _, _ | .bytes, _, _, _, _
  | .char, _, _, _, _ | .vector _, _, _, _, _ | .slice _, _, _, _, _
  | .array _ _, _, _, _, _ | .table _ _, _, _, _, _ | .ghost _, _, _, _, _
  | .param _, _, _, _, _ | .function _ _ _, _, _, _, _ | .nominal _ _ _, _, _, _, _
  | .observed _, _, _, _, _ => True

/-- Every reference an enum's active variant holds resolves at its prophecy. -/
def variantResolves : (names : List String) → (rows : NRows) → variantCarrier names rows →
    List Nat → Array RuntimeValue → List (Nat × RuntimeValue) → Prop
  | _, .nil, value, _, _, _ => nomatch value
  | [], .cons _ _, value, _, _, _ => nomatch value
  | _ :: _, .cons fields _, .inl values, loans, returned, exports =>
      argumentsResolve fields values loans returned exports
  | _ :: names, .cons _ rest, .inr later, loans, returned, exports =>
      variantResolves names rest later loans returned exports

/-- Every reference a row of values holds resolves at its prophecy, under the
loans the row was lent, taken in order. -/
def argumentsResolve : (σs : NRow) → HList σs → List Nat → Array RuntimeValue →
    List (Nat × RuntimeValue) → Prop
  | .nil, _, _, _, _ => True
  | .cons τ rest, values, loans, returned, exports =>
      match τ.lend false values.1 loans with
      | some lent =>
          τ.resolves values.1 loans returned exports ∧
            argumentsResolve rest values.2 lent.2 returned exports
      | none => False

/-- Every reference a plain value holds resolves at its prophecy: the layer
a reference's readings live at (`NTy.plainLend`). -/
def NTy.plainResolves : (τ : NTy) → τ.plain → List Nat → Array RuntimeValue →
    List (Nat × RuntimeValue) → Prop
  | .ref σ, value, loan :: loans, returned, exports =>
      ∃ lent, σ.plainLend false value.2 loans = some lent ∧
        match exportedRaw? loan exports with
        | some exported =>
            VerexIR.SemanticOperations.resolveReturnedBorrows returned exported = lent.1
        | none => ∃ entry ∈ returned.toList,
            (loan, lent.1) ∈ (VerexIR.SemanticOperations.outermostBorrows entry).toList
  | .ref _, _, [], _, _ => False
  | .sliceRef σ, value, loan :: _, returned, exports =>
      let final := NTy.plainEncode (.vector σ) value.finalVector
      match exportedRaw? loan exports with
      | some exported => VerexIR.SemanticOperations.resolveReturnedBorrows returned exported = final
      | none => ∃ entry ∈ returned.toList,
          (loan, final) ∈ (VerexIR.SemanticOperations.outermostBorrows entry).toList
  | .sliceRef _, _, [], _, _ => False
  | .tuple elements, values, loans, returned, exports =>
      plainArgumentsResolve elements values loans returned exports
  | .struct _ _ fields, values, loans, returned, exports =>
      plainArgumentsResolve fields values loans returned exports
  | .enum _ _ names rows _, value, loans, returned, exports =>
      variantPlainResolves names rows value loans returned exports
  | .unit, _, _, _, _ | .bool, _, _, _, _ | .int _ _, _, _, _, _ | .num, _, _, _, _
  | .address, _, _, _, _ | .signer, _, _, _, _ | .string, _, _, _, _ | .bytes, _, _, _, _
  | .char, _, _, _, _ | .vector _, _, _, _, _ | .slice _, _, _, _, _
  | .array _ _, _, _, _, _ | .table _ _, _, _, _, _ | .ghost _, _, _, _, _
  | .param _, _, _, _, _ | .function _ _ _, _, _, _, _ | .nominal _ _ _, _, _, _, _
  | .observed _, _, _, _, _ => True

/-- Every reference an enum's plain active variant holds resolves at its
prophecy. -/
def variantPlainResolves : (names : List String) → (rows : NRows) → variantPlain names rows →
    List Nat → Array RuntimeValue → List (Nat × RuntimeValue) → Prop
  | _, .nil, value, _, _, _ => nomatch value
  | [], .cons _ _, value, _, _, _ => nomatch value
  | _ :: _, .cons fields _, .inl values, loans, returned, exports =>
      plainArgumentsResolve fields values loans returned exports
  | _ :: names, .cons _ rest, .inr later, loans, returned, exports =>
      variantPlainResolves names rest later loans returned exports

/-- Every reference a plain row of values holds resolves at its prophecy,
under the loans the row was lent, taken in order. -/
def plainArgumentsResolve : (σs : NRow) → HListPlain σs → List Nat → Array RuntimeValue →
    List (Nat × RuntimeValue) → Prop
  | .nil, _, _, _, _ => True
  | .cons τ rest, values, loans, returned, exports =>
      match τ.plainLend false values.1 loans with
      | some lent =>
          τ.plainResolves values.1 loans returned exports ∧
            plainArgumentsResolve rest values.2 lent.2 returned exports
      | none => False
end

/-- A start state for a call lending references under loans: distinct
loans below the allocation frontier, none registered to a global, and fresh
identifiers beyond the frontier. -/
def Admissible (state : VerexIR.RuntimeState) (loans : List Nat) : Prop :=
  loans.Nodup ∧
    (∀ loan ∈ loans, loan < state.nextLoan ∧
      VerexIR.SemanticOperations.globalLoanKeyIn? state.globalLoans loan = none) ∧
    VerexIR.SemanticOperations.FreshGlobalLoanIds state

/-- What a call denotes, for every callee handle and signature. -/
abbrev CalleeMeaning : Type :=
  FunctionHandle → (σs : NRow) → (shape : ResultShape) → HList σs → Comp unit shape.carrier

/-- What a call with type arguments denotes, at the callee's own signature
under the family its type arguments induce. -/
abbrev GenericMeaning [Θ : Skolems unit] : Type :=
  FunctionHandle → Array GenericArgument → (θ : TypeArgs) → (σs : NRow) → (shape : ResultShape) →
    @HList (Skolems.instantiate θ Θ).toCarriers σs → Comp unit (@ResultShape.carrier (Skolems.instantiate θ Θ).toCarriers shape)

/-- The big-step run of a callee: from any admissible start whose globals and
Table storage encode the caller's memory, with references as `(current, prophecy)`,
ending in the memory its exit stores encode.  What invoking a function
value denotes. -/
def propheticRun {unit : VerexIR.Validation.ValidatedUnit} [FunctionCarriers unit]
    (executable : VerexIR.Validation.ExecutableUnit unit) [Skolems unit]
    (typeInstantiation : Array (TypeId × TypeId)) (handle : FunctionHandle)
    (σs : NRow) (shape : ResultShape) (args : HList σs) : Comp unit shape.carrier where
  ok := fun initial result final =>
    ∃ start loans arguments results exit correspondence,
      StorageEncodes unit initial start ∧ Admissible start loans ∧
      lendArguments σs args loans = some arguments ∧
      (functionSpecAt executable handle typeInstantiation arguments).ok start results exit ∧
      BoundaryCorrespondence σs args shape result arguments results loans
        (exportsAfter start.pending exit.pending) correspondence ∧
      StorageEncodesReturned unit final exit (correspondence.returnedRow results) ∧ AgreeUnnamed unit final initial
  aborts := fun initial error =>
    ∃ start loans arguments,
      StorageEncodes unit initial start ∧ Admissible start loans ∧
      lendArguments σs args loans = some arguments ∧
      (functionSpecAt executable handle typeInstantiation arguments).aborts start error.encode
  undefined := fun initial =>
    ∃ start loans arguments results exit,
      StorageEncodes unit initial start ∧ Admissible start loans ∧
      lendArguments σs args loans = some arguments ∧
      (functionSpecAt executable handle typeInstantiation arguments).ok start results exit ∧
      ¬∃ result, ∃ resolved : HList σs, ∃ correspondence,
        lendArguments σs resolved loans = some arguments ∧
        BoundaryCorrespondence σs resolved shape result arguments results loans
          (exportsAfter start.pending exit.pending) correspondence ∧
        correspondence.ReturnsStorage results exit

/-- The prophetic meaning of a callee at a frame: its run, where the frame
agrees with the runtime type instantiation on the types the callee reads
(`Coherent`), and no outcome where they disagree.  A call denotes
exactly this; a caller reasons about it through the callee's published
contract, or, for a callee returning a reference, through its
denotation. -/
def propheticMeaning {unit : VerexIR.Validation.ValidatedUnit} [FunctionCarriers unit]
    (executable : VerexIR.Validation.ExecutableUnit unit) [Skolems unit]
    (typeInstantiation : Array (TypeId × TypeId)) (handle : FunctionHandle)
    (σs : NRow) (shape : ResultShape) (args : HList σs) : Comp unit shape.carrier where
  ok := fun initial result final =>
    Coherent unit handle typeInstantiation ∧
      (propheticRun executable typeInstantiation handle σs shape args).ok initial result final
  aborts := fun initial error =>
    Coherent unit handle typeInstantiation ∧
      (propheticRun executable typeInstantiation handle σs shape args).aborts initial error
  undefined := fun initial =>
    Coherent unit handle typeInstantiation ∧
      (propheticRun executable typeInstantiation handle σs shape args).undefined initial

/-- At a coherent frame the prophetic meaning is the run. -/
theorem propheticMeaning_of_coherent {unit : VerexIR.Validation.ValidatedUnit} [FunctionCarriers unit]
    {executable : VerexIR.Validation.ExecutableUnit unit} [Skolems unit]
    {typeInstantiation : Array (TypeId × TypeId)} {handle : FunctionHandle}
    (coherent : Coherent unit handle typeInstantiation) (σs : NRow) (shape : ResultShape)
    (args : HList σs) :
    propheticMeaning executable typeInstantiation handle σs shape args =
      propheticRun executable typeInstantiation handle σs shape args := by
  simp only [propheticMeaning, coherent, true_and]

/-- The prophetic meaning of a callee compiled at a substitution. Without one it is
`propheticMeaning`. At `θ` the rows are already substituted and the run lends at them,
in the ambient family, while the frame's agreement on the callee's own types is at the
family `θ` induces, where its type parameters are the arguments. -/
def propheticMeaningAt {unit : VerexIR.Validation.ValidatedUnit} [FunctionCarriers unit]
    (executable : VerexIR.Validation.ExecutableUnit unit) [Θ : Skolems unit]
    (substitution : Option TypeArgs) (typeInstantiation : Array (TypeId × TypeId))
    (handle : FunctionHandle) (σs : NRow) (shape : ResultShape) (args : HList σs) :
    Comp unit shape.carrier :=
  match substitution with
  | none => propheticMeaning executable typeInstantiation handle σs shape args
  | some θ =>
      { ok := fun initial result final =>
          @Coherent unit _ (Skolems.instantiate θ Θ) handle typeInstantiation ∧
            (propheticRun executable typeInstantiation handle σs shape args).ok initial result final
        aborts := fun initial error =>
          @Coherent unit _ (Skolems.instantiate θ Θ) handle typeInstantiation ∧
            (propheticRun executable typeInstantiation handle σs shape args).aborts initial error
        undefined := fun initial =>
          @Coherent unit _ (Skolems.instantiate θ Θ) handle typeInstantiation ∧
            (propheticRun executable typeInstantiation handle σs shape args).undefined initial }

/-- The function value of a closure: its target, mask, and type
instantiation, with the captures' encodings. -/
noncomputable def closureOf (handle : FunctionHandle) (mask : Nat) (typeInstantiation : Array (TypeId × TypeId))
    {σs : NRow} (captures : HList σs) : ClosureValue :=
  ⟨handle, mask, typeInstantiation, (HList.encode captures).toArray,
    fun capture member => rowCodec_plain _ captures capture (by simpa [HList.encode] using member)⟩

/-- A closure site is typed at a function type at the frame: its target's
rows at its instantiation are the frame's resolution of the captured row
and the function type's rows, as are its sharing marks, and the
instantiation is faithful. What a closure node carries, read at the frame;
construction takes it, so a constructed value is typed by construction. -/
structure SiteTyped (parameters : NRow) (shared : Sharing) (results : NRow)
    (handle : FunctionHandle) (mask : Nat) (typeInstantiation : Array (TypeId × TypeId))
    (captured : NRow) : Prop where
  rows : closureRowsIn? unit handle mask typeInstantiation =
    some (captured.resolved, parameters.resolved, results.resolved)
  sharing : closureShared unit handle mask = shared
  faithful : closureFaithful unit handle typeInstantiation = true

/-- A frame's values of a row encode as runtime values of the row's
resolution, which admit it. -/
theorem HList.admits_resolved (row : NRow) (values : HList row) :
    NRow.admits unit row.resolved (HList.encode values) = true := by
  have encoded := HList.encode_toRuntime row values
  simp only [HList.encode] at encoded ⊢
  rw [← encoded]
  exact NRow.admits_encode unit row.resolved (HList.toRuntime row values)

/-- A typed site's closure is one of the frame's function values. -/
theorem SiteTyped.decodes {parameters : NRow} {shared : Sharing} {results : NRow}
    {handle : FunctionHandle} {mask : Nat} {typeInstantiation : Array (TypeId × TypeId)}
    {σs : NRow} (typed : SiteTyped parameters shared results handle mask typeInstantiation σs)
    (captures : HList σs) :
    ((Carriers.functions parameters shared results).ofClosure?
      (closureOf handle mask typeInstantiation captures)).isSome = true := by
  apply Skolems.ofClosure?_of_runtime
  rw [← NRow.resolved_eq_substWith, ← NRow.resolved_eq_substWith]
  show NTy.admits unit _ (closureOf handle mask typeInstantiation captures).encode = true
  simp only [ClosureValue.encode, closureOf]
  unfold NTy.admits
  rw [typed.rows]
  simp only [typed.faithful, typed.sharing, beq_self_eq_true, Bool.true_and]
  exact HList.admits_resolved σs captures

/-- A closure of a target without type parameters, at its own rows
(`closureRows?`), closed and faithful at the empty instantiation, is typed
at every frame: what a closure node carries. -/
theorem SiteTyped.ofClosure {handle : FunctionHandle} {mask : Nat}
    {captured supplied results : NRow} {shared : Sharing}
    (rows : closureRows? unit handle mask = some (captured, supplied, results))
    (sharing : closureShared unit handle mask = shared)
    (closed : (captured.paramFree && supplied.paramFree && results.paramFree) = true)
    (faithful : closureFaithful unit handle #[] = true) :
    SiteTyped supplied shared results handle mask #[] captured := by
  simp only [Bool.and_eq_true] at closed
  obtain ⟨⟨capturedFree, suppliedFree⟩, resultsFree⟩ := closed
  refine ⟨?_, sharing, faithful⟩
  simp only [closureRowsIn?, rows, Option.map_some, NRow.substWith_paramFree _ _ capturedFree,
    NRow.substWith_paramFree _ _ suppliedFree, NRow.substWith_paramFree _ _ resultsFree,
    NRow.resolved_paramFree _ capturedFree, NRow.resolved_paramFree _ suppliedFree,
    NRow.resolved_paramFree _ resultsFree]

omit [Skolems unit] in
/-- Whether a closure's rows hold no type parameter. -/
def closureClosed (captured supplied results : NRow) : Bool :=
  captured.paramFree && supplied.paramFree && results.paramFree

omit [Skolems unit] in
/-- Whether a closure of a target without type parameters is a function
value at rows, decided at the unit: the target's own rows, its sharing, the
rows closed, and the empty instantiation faithful (`SiteTyped.ofCheck`). -/
def closureTypingCheck (unit : Validation.ValidatedUnit) (handle : FunctionHandle) (mask : Nat)
    (captured supplied results : NRow) (shared : Sharing) : Bool :=
  closureRows? unit handle mask == some (captured, supplied, results) &&
    closureShared unit handle mask == shared && closureClosed captured supplied results &&
    closureFaithful unit handle #[]

/-- A closure the unit's check types is typed at every frame: the typing of
a literal function value a clause writes, its check evaluated once where the
clause is read at the verified unit. -/
theorem SiteTyped.ofCheck {handle : FunctionHandle} {mask : Nat}
    {captured supplied results : NRow} {shared : Sharing}
    (check : closureTypingCheck unit handle mask captured supplied results shared = true) :
    SiteTyped supplied shared results handle mask #[] captured := by
  simp only [closureTypingCheck, Bool.and_eq_true, beq_iff_eq] at check
  obtain ⟨⟨⟨rows, sharing⟩, closed⟩, faithful⟩ := check
  exact SiteTyped.ofClosure rows sharing closed faithful

omit [Skolems unit] in
private theorem all_paramsAll {allowed : Nat → Bool} :
    (types : List NTy) → types.all (·.paramsAll allowed) = true →
      (NRow.ofList types).paramsAll allowed = true
  | [], _ => rfl
  | τ :: types, mentioned => by
      simp only [List.all_cons, Bool.and_eq_true] at mentioned
      simp only [NRow.ofList, NRow.paramsAll, mentioned.1, all_paramsAll types mentioned.2,
        Bool.and_self]

omit [Skolems unit] in
private theorem extract_all {α : Type} {keep : α → Bool} :
    (mask : Nat) → (captured : Bool) → (items : List α) → items.all keep = true →
      (ClosureMask.extract mask captured items).all keep = true
  | _, _, [], _ => rfl
  | mask, captured, item :: items, kept => by
      simp only [List.all_cons, Bool.and_eq_true] at kept
      simp only [ClosureMask.extract]
      split
      · simp only [List.all_cons, kept.1, extract_all (mask / 2) captured items kept.2,
          Bool.and_self]
      · exact extract_all (mask / 2) captured items kept.2

/-- At a frame coherent with a runtime type instantiation, a closure's rows
there are the frame's resolution of its target's own rows: the
instantiation gives each parameter the frame's type for it. -/
theorem closureRowsIn?_coherent {handle : FunctionHandle} {mask : Nat}
    {typeInstantiation : Array (TypeId × TypeId)} {captured supplied results : NRow}
    (rows : closureRows? unit handle mask = some (captured, supplied, results))
    (below : closureSignatureBelow unit handle = true)
    (coherent : Coherent unit handle typeInstantiation) :
    closureRowsIn? unit handle mask typeInstantiation =
      some (captured.resolved, supplied.resolved, results.resolved) := by
  have agree : ∀ index, closureParameter unit handle index = true →
      closureArgument unit handle typeInstantiation index = Skolems.type ‹_› index := by
    intro index parameter
    simp only [closureParameter, Bool.and_eq_true, decide_eq_true_eq, Option.isSome_iff_exists]
      at parameter
    obtain ⟨bounded, node, found⟩ := parameter
    simp only [closureArgument, found, coherent.params index bounded node found, Option.getD_some]
  obtain ⟨parameters, returned, signature, _, _, rfl, rfl, rfl⟩ := closureRows?_eq_some.mp rows
  simp only [closureSignatureBelow, signature, Bool.and_eq_true] at below
  have mentioned := fun captured => all_paramsAll _ (extract_all mask captured _ below.1)
  simp only [closureRowsIn?, rows, Option.map_some, NRow.resolved_eq_substWith,
    NRow.substWith_congr agree _ (mentioned true), NRow.substWith_congr agree _ (mentioned false),
    NRow.substWith_congr agree _ (all_paramsAll _ below.2)]

/-- The function value a closure creates at the frame: the value of the
function type whose runtime closure has the target, mask, instantiation,
and captures, typed by the closure's site (`SiteTyped`). Construction is
an operation of the frame: irreducible, a leaf reads it by its closure
(`Skolems.closure_construct`) and, at the base frame, by the generated
family's constructors (`SiteFamily.construct_eq`). -/
noncomputable def Skolems.construct (parameters : NRow) (shared : Sharing) (results : NRow)
    (handle : FunctionHandle) (mask : Nat) (typeInstantiation : Array (TypeId × TypeId))
    {σs : NRow} (captures : HList σs)
    (typed : SiteTyped parameters shared results handle mask typeInstantiation σs) :
    (NTy.function parameters shared results).carrier :=
  ((Carriers.functions parameters shared results).ofClosure?
    (closureOf handle mask typeInstantiation captures)).get (typed.decodes captures)

/-- A created function value's runtime closure is the closure created. -/
theorem Skolems.closure_construct (parameters : NRow) (shared : Sharing) (results : NRow)
    (handle : FunctionHandle) (mask : Nat) (typeInstantiation : Array (TypeId × TypeId))
    {σs : NRow} (captures : HList σs)
    (typed : SiteTyped parameters shared results handle mask typeInstantiation σs) :
    (Carriers.functions parameters shared results).closure
        (Skolems.construct parameters shared results handle mask typeInstantiation captures typed) =
      closureOf handle mask typeInstantiation captures :=
  (Carriers.functions parameters shared results).closure_of_ofClosure? _ _
    (Option.some_get (typed.decodes captures)).symm

attribute [irreducible] Skolems.construct

/-- The prophetic run of a runtime closure: its target's big-step meaning
under the closure's type instantiation, with the captures composed into the
lent arguments by the closure's mask, as an invocation runs it. -/
def closureRun {unit : VerexIR.Validation.ValidatedUnit} [FunctionCarriers unit]
    (executable : VerexIR.Validation.ExecutableUnit unit) [Skolems unit] (closure : ClosureValue)
    (σs : NRow) (shape : ResultShape) (args : HList σs) : Comp unit shape.carrier where
  ok := fun initial result final =>
    ∃ start loans supplied arguments results exit correspondence,
      StorageEncodes unit initial start ∧ Admissible start loans ∧
      lendArguments σs args loans = some supplied ∧
      ClosureMask.compose closure.mask closure.captures.toList supplied.toList =
        some arguments ∧
      (functionSpecAt executable closure.function closure.typeInstantiation arguments.toArray).ok
        start results exit ∧
      BoundaryCorrespondence σs args shape result supplied results loans
        (exportsAfter start.pending exit.pending) correspondence ∧
      StorageEncodesReturned unit final exit (correspondence.returnedRow results) ∧
      AgreeUnnamed unit final initial
  aborts := fun initial error =>
    ∃ start loans supplied arguments,
      StorageEncodes unit initial start ∧ Admissible start loans ∧
      lendArguments σs args loans = some supplied ∧
      ClosureMask.compose closure.mask closure.captures.toList supplied.toList =
        some arguments ∧
      (functionSpecAt executable closure.function closure.typeInstantiation arguments.toArray).aborts
        start error.encode
  undefined := fun initial =>
    ∃ start loans supplied arguments results exit,
      StorageEncodes unit initial start ∧ Admissible start loans ∧
      lendArguments σs args loans = some supplied ∧
      ClosureMask.compose closure.mask closure.captures.toList supplied.toList =
        some arguments ∧
      (functionSpecAt executable closure.function closure.typeInstantiation arguments.toArray).ok
        start results exit ∧
      ¬∃ result, ∃ resolved : HList σs, ∃ correspondence,
        lendArguments σs resolved loans = some supplied ∧
        BoundaryCorrespondence σs resolved shape result supplied results loans
          (exportsAfter start.pending exit.pending) correspondence ∧
        correspondence.ReturnsStorage results exit

omit [Skolems unit] in
/-- The type instantiation of a callee's frame, computed from the caller's
and the call's generic arguments as the runtime does. -/
def frameInstantiation (unit : VerexIR.Validation.ValidatedUnit) (handle : FunctionHandle)
    (outer : Array (TypeId × TypeId)) (typeArgs : Array GenericArgument) :
    Array (TypeId × TypeId) :=
  VerexIR.SemanticOperations.callTypeInstantiation unit handle outer typeArgs

omit [Skolems unit] in
/-- Calls whose generic arguments name the same types and lifetimes, wherever
they occur, share a frame. -/
theorem frameInstantiation_congr {unit : VerexIR.Validation.ValidatedUnit}
    {handle : FunctionHandle} {outer : Array (TypeId × TypeId)}
    {left right : Array GenericArgument}
    (same : left.toList.map GenericArgument.eraseLoc =
      right.toList.map GenericArgument.eraseLoc) :
    frameInstantiation unit handle outer left = frameInstantiation unit handle outer right := by
  have same : left.map GenericArgument.eraseLoc = right.map GenericArgument.eraseLoc := by
    apply Array.ext'
    simpa only [Array.toList_map] using same
  unfold frameInstantiation
  rw [← VerexIR.SemanticOperations.callTypeInstantiation_eraseLoc, same,
    VerexIR.SemanticOperations.callTypeInstantiation_eraseLoc]

/-- Whether the arguments are one per binder, each of its binder's kind. -/
def argumentsOfKinds : List BinderKind → List GenericArgument → Bool
  | [], [] => true
  | .typeArg :: binders, .typeArg _ :: arguments
  | .const :: binders, .const _ :: arguments
  | .lifetime :: binders, .lifetime _ :: arguments
  | .evidence :: binders, .evidence _ :: arguments => argumentsOfKinds binders arguments
  | _, _ => false

/-- Arguments for type binders alone are type arguments. -/
theorem argumentsOfKinds_types : (binders : List BinderKind) → (arguments : List GenericArgument) →
    argumentsOfKinds binders arguments = true → binders.all (· == .typeArg) = true →
    ∃ uses : List TypeUse, arguments = uses.map .typeArg ∧ uses.length = binders.length
  | [], [], _, _ => ⟨[], rfl, rfl⟩
  | .typeArg :: binders, .typeArg use :: arguments, kinds, types => by
      simp only [List.all_cons, Bool.and_eq_true] at types
      obtain ⟨uses, same, sizes⟩ := argumentsOfKinds_types binders arguments kinds types.2
      exact ⟨use :: uses, by simp [same], by simp [sizes]⟩
  | .const :: _, _, _, types | .lifetime :: _, _, _, types | .evidence :: _, _, _, types => by
      simp only [List.all_cons, Bool.and_eq_true] at types
      exact absurd types.1 (by decide)
  | [], _ :: _, kinds, _ | _ :: _, [], kinds, _ => by
      simp [argumentsOfKinds] at kinds
  | .typeArg :: _, .const _ :: _, kinds, _ | .typeArg :: _, .lifetime _ :: _, kinds, _
  | .typeArg :: _, .evidence _ :: _, kinds, _ => by
      simp [argumentsOfKinds] at kinds

omit [Skolems unit] in
/-- The instantiations a generic function runs in: the empty one, where it
runs on its own, or a call's frame, from the caller's frame and one generic
argument per binder. -/
def FrameOf (unit : VerexIR.Validation.ValidatedUnit) (handle : FunctionHandle)
    (binders : List BinderKind) (typeInstantiation : Array (TypeId × TypeId)) : Prop :=
  typeInstantiation = #[] ∨ ∃ outer typeArgs, argumentsOfKinds binders typeArgs.toList = true ∧
    typeInstantiation = frameInstantiation unit handle outer typeArgs

omit [Skolems unit] in
theorem FrameOf.empty {unit : VerexIR.Validation.ValidatedUnit} {handle : FunctionHandle}
    {binders : List BinderKind} : FrameOf unit handle binders #[] := .inl rfl

omit [Skolems unit] in
theorem FrameOf.frame {unit : VerexIR.Validation.ValidatedUnit} {handle : FunctionHandle}
    {binders : List BinderKind} {outer : Array (TypeId × TypeId)}
    {typeArgs : Array GenericArgument}
    (kinds : argumentsOfKinds binders typeArgs.toList = true) :
    FrameOf unit handle binders (frameInstantiation unit handle outer typeArgs) :=
  .inr ⟨outer, typeArgs, kinds, rfl⟩

omit [Skolems unit] in
theorem FrameOf.rewrite {unit : VerexIR.Validation.ValidatedUnit} {handle : FunctionHandle}
    {binders : List BinderKind} {left right : Array (TypeId × TypeId)}
    (frame : FrameOf unit handle binders left)
    (same : left = right) : FrameOf unit handle binders right :=
  same ▸ frame

omit [Skolems unit] in
/-- A call passing its caller's own type parameters, in order, from the
caller's namespace, runs in the caller's frame; at the empty frame as the
runtime computes it for the call (`empty`). -/
theorem frameInstantiation_own {unit : VerexIR.Validation.ValidatedUnit}
    {caller callee : FunctionHandle} {binders : List BinderKind}
    {own : Array GenericArgument} {typeInstantiation : Array (TypeId × TypeId)}
    (same : callee.namespaceId = caller.namespaceId) (sizes : own.size = binders.length)
    (types : binders.all (· == .typeArg) = true)
    (parameters : VerexIR.SemanticOperations.ownParametersIn unit caller own = true)
    (empty : frameInstantiation unit callee #[] own = #[])
    (frame : FrameOf unit caller binders typeInstantiation) :
    frameInstantiation unit callee typeInstantiation own = typeInstantiation := by
  rcases frame with rfl | ⟨outer, typeArgs, kinds, rfl⟩
  · exact empty
  · unfold VerexIR.SemanticOperations.ownParametersIn at parameters
    split at parameters
    · rename_i ns namespaceOf
      obtain ⟨uses, ownUses, typed⟩ :=
        VerexIR.SemanticOperations.ownParameters_spec parameters
      obtain ⟨arguments, argumentsUses, count⟩ := argumentsOfKinds_types _ _ kinds types
      have typeArgsUses : typeArgs = arguments.toArray.map .typeArg := by
        apply Array.ext'
        simpa using argumentsUses
      subst ownUses
      subst typeArgsUses
      exact VerexIR.SemanticOperations.callTypeInstantiation_own_parameters unit ns caller callee
        outer _ uses namespaceOf same (by simp_all) typed
    · cases parameters

/-- The closed meaning of calls with type arguments: the callee's
prophetic meaning under its frame's instantiation, computed from the
caller's as the runtime does. -/
noncomputable def closedGeneric {unit : VerexIR.Validation.ValidatedUnit} [FunctionCarriers unit]
    (executable : VerexIR.Validation.ExecutableUnit unit) [Θ : Skolems unit]
    (typeInstantiation : Array (TypeId × TypeId)) : GenericMeaning :=
  fun handle typeArgs θ σs shape args =>
    @propheticMeaning _ _ executable (Skolems.instantiate θ ‹_›)
      (frameInstantiation unit handle typeInstantiation typeArgs) handle σs shape args

/-- What a call at a type argument holding a mutable reference denotes, over the
callee's substituted rows. -/
abbrev InstanceMeaning [Skolems unit] : Type :=
  FunctionHandle → Array GenericArgument → (θ : TypeArgs) → (σs : NRow) → (shape : ResultShape) →
    HList σs → Comp unit shape.carrier

/-- The closed meaning of such calls: the callee's prophetic meaning at `θ`, under
its frame's instantiation, computed from the caller's as the runtime does. -/
def closedInstance {unit : VerexIR.Validation.ValidatedUnit} [FunctionCarriers unit]
    (executable : VerexIR.Validation.ExecutableUnit unit) [Skolems unit]
    (typeInstantiation : Array (TypeId × TypeId)) : InstanceMeaning :=
  fun handle typeArgs θ σs shape args =>
    propheticMeaningAt executable (some θ)
      (frameInstantiation unit handle typeInstantiation typeArgs) handle σs shape args

/-- What invoking a function value denotes: of a value of a function type,
on arguments of its parameter row, a result of its shape. -/
abbrev InvokeMeaning : Type :=
  (σs : NRow) → (shared : Sharing) → (shape : ResultShape) →
    (NTy.function σs shared shape.row).carrier → HList σs → Comp unit shape.carrier

/-- The prophetic meaning of a function value: the run of its runtime
closure (`closureRun`). -/
def closureMeaning {unit : VerexIR.Validation.ValidatedUnit} [FunctionCarriers unit]
    (executable : VerexIR.Validation.ExecutableUnit unit) [Skolems unit] : InvokeMeaning :=
  fun σs shared shape function args =>
    closureRun executable ((Carriers.functions σs shared shape.row).closure function) σs shape args

/-- What calls denote: a call without type arguments, and a call with them
at the family they induce. -/
structure Meanings {unit : VerexIR.Validation.ValidatedUnit} [FunctionCarriers unit]
    (executable : VerexIR.Validation.ExecutableUnit unit) [Θ : Skolems unit] where
  call : CalleeMeaning
  generic : GenericMeaning
  /-- What invoking a function value means: its target's big-step run, or,
  in a unit verified through its model, the target's meaning
  (`doc/designs/ghost-fields.md`). -/
  invoke : InvokeMeaning := closureMeaning executable
  /-- The frame's type instantiation, which keys its generic families. -/
  typeInstantiation : Array (TypeId × TypeId)
  /-- The condition of the in-body assumption at a site, over the locals of
  its function and the state: none where a run assumes nothing there. -/
  assumption : Nat → Option ((Γ : NRow) × (HEnv Γ → Memory unit → Prop))
  /-- The value an `update` at a site writes, over the locals of its
  function and the state: none where it writes any value of its type
  (`doc/designs/ghost-fields.md`). -/
  ghost : Nat → Option ((Γ : NRow) × (τ : NTy) × (HEnv Γ → Memory unit → Option τ.carrier)) :=
    fun _ => none
  /-- The value an initializer gives a ghost slot of a value packed at a
  site, over the pack's certified field row and the state: it runs before
  the row is forgotten, so it reads the pack's fields as the values they are
  (`GhostInits.initialize`). -/
  ghostInit : Nat →
      Option ((fields : NRow) × (σ : NTy) × (HList fields → Memory unit → Option σ.carrier)) :=
    fun _ => none
  /-- What a call at a type argument holding a mutable reference means. -/
  atInstance : InstanceMeaning

/-- Every call its callee's prophetic meaning, a call with type arguments
under the instantiation its frame computes from `typeInstantiation`. -/
@[reducible] noncomputable def closedMeanings {unit : VerexIR.Validation.ValidatedUnit}
    [FunctionCarriers unit]
    (executable : VerexIR.Validation.ExecutableUnit unit) [Θ : Skolems unit]
    (typeInstantiation : Array (TypeId × TypeId)) : Meanings executable :=
  ⟨propheticMeaning executable #[], closedGeneric executable typeInstantiation,
    closureMeaning executable, typeInstantiation, fun _ => none, fun _ => none, fun _ => none,
    closedInstance executable typeInstantiation⟩

/-- The external operations a typed term observes, at its own carrier family
and state. Generic calls already use the caller's substituted rows: no
runtime-frame transport is part of this interface. Its implementations and
their relations are explicit inputs to generic verification. -/
structure InterpreterOps (unit : Validation.ValidatedUnit) (carriers : Carriers)
    (State : Type) where
  call : (handle : FunctionHandle) → (parameters : NRow) → (shape : ResultShape) →
    @HList carriers parameters → Spec State Failure (@ResultShape.carrier carriers shape)
  generic : (handle : FunctionHandle) → Array GenericArgument → (arguments : TypeArgs) →
    (parameters : NRow) → (shape : ResultShape) →
    @HList carriers (NRow.subst arguments.1 parameters) →
    Spec State Failure (@ResultShape.carrier carriers (shape.subst arguments.1))
  invoke : (parameters : NRow) → (shared : Sharing) → (shape : ResultShape) →
    @NTy.carrier carriers (.function parameters shared shape.row) →
    @HList carriers parameters → Spec State Failure (@ResultShape.carrier carriers shape)
  closure : {full captured supplied : NRow} → (handle : FunctionHandle) →
    (weave : Weave full captured supplied) → (results : NRow) → (shared : Sharing) →
    closureRows? unit handle weave.mask = some (captured, supplied, results) →
    closureShared unit handle weave.mask = shared →
    (captured.paramFree && supplied.paramFree && results.paramFree) = true →
    closureFaithful unit handle #[] = true → @HList carriers captured →
    Spec State Failure (@NTy.carrier carriers (.function supplied shared results))
  closureGeneric : {full captured supplied : NRow} → (handle : FunctionHandle) →
    (weave : Weave full captured supplied) → Array GenericArgument → (arguments : TypeArgs) →
    (shape : ResultShape) → (shared : Sharing) →
    closureRows? unit handle weave.mask = some (captured, supplied, shape.row) →
    closureShared unit handle weave.mask = shared →
    closureSignatureBelow unit handle = true →
    @HList carriers (NRow.subst arguments.1 captured) →
    Spec State Failure (@NTy.carrier carriers
      (.function (NRow.subst arguments.1 supplied) shared (shape.subst arguments.1).row))
  read : (type : NTy) → (arguments : NRow) → StorageKey → State →
    Option (@NTy.carrier carriers type)
  write : (type : NTy) → (arguments : NRow) → StorageKey →
    Option (@NTy.carrier carriers type) → State → State
  assumption : Nat → Option ((locals : NRow) × (@HEnv carriers locals → State → Prop))
  ghost : Nat → Option ((locals : NRow) × (type : NTy) ×
    (@HEnv carriers locals → State → Option (@NTy.carrier carriers type)))
  ghostInit : Nat → Option ((fields : NRow) × (type : NTy) ×
    (@HList carriers fields → State → Option (@NTy.carrier carriers type)))
  /-- A call at a type argument holding a mutable reference: at the caller's
  rows, which the callee instantiates itself. -/
  atInstance : (handle : FunctionHandle) → Array GenericArgument → (arguments : TypeArgs) →
    (parameters : NRow) → (shape : ResultShape) →
    @HList carriers parameters → Spec State Failure (@ResultShape.carrier carriers shape)


section LocalComputations
variable [C : Carriers] {State : Type}

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
    (recurse : HEnv Γ → Spec State Failure (Flow ρ Γ Unit)) : Flow ρ Γ Unit → Spec State Failure (Flow ρ Γ Unit)
  | .value _ env => recurse env
  | .continue_ 0 env => recurse env
  | .continue_ (nest + 1) env => Spec.pure (.continue_ nest env)
  | .break_ 0 env => Spec.pure (.value () env)
  | .break_ (nest + 1) env => Spec.pure (.break_ nest env)
  | .return_ result env => Spec.pure (.return_ result env)

/-- The least fixed point of a loop body, marked with the loop's site so
that verification can name its invariant.  Semantically `Spec.fix`. -/
def loopAt {ρ : ResultShape} {Γ : NRow} (_site : Nat)
    (iteration : (HEnv Γ → Spec State Failure (Flow ρ Γ Unit)) → HEnv Γ → Spec State Failure (Flow ρ Γ Unit))
    (entry : HEnv Γ) : Spec State Failure (Flow ρ Γ Unit) :=
  Spec.fix iteration entry

/-- An in-body assertion, marked with its site so that verification can name
the asserted condition.  Semantically no step. -/
def assertAt {ρ : ResultShape} {Γ : NRow} (_site : Nat) (entry : HEnv Γ) :
    Spec State Failure (Flow ρ Γ Unit) :=
  Spec.pure (.value () entry)

/-- A state anchor, marked with its site so that verification can record
the locals and state there.  Semantically no step. -/
def anchorAt {ρ : ResultShape} {Γ : NRow} (_site : Nat) (entry : HEnv Γ) :
    Spec State Failure (Flow ρ Γ Unit) :=
  Spec.pure (.value () entry)

/-- The condition an assumption entry states of locals of a row: its own
where the rows agree, none otherwise. -/
def assumptionOf {Γ : NRow} (entry : Option ((Γ' : NRow) × (HEnv Γ' → State → Prop)))
    (env : HEnv Γ) : State → Prop :=
  match entry with
  | some ⟨Γ', holds⟩ => if same : Γ = Γ' then holds (same ▸ env) else fun _ => True
  | none => fun _ => True

@[simp] theorem assumptionOf_none {Γ : NRow} (env : HEnv Γ) :
    assumptionOf (State := State) none env = fun _ => True := rfl

@[simp] theorem assumptionOf_same {Γ : NRow} (holds : HEnv Γ → State → Prop) (env : HEnv Γ) :
    assumptionOf (some ⟨Γ, holds⟩) env = holds env := by
  simp [assumptionOf]

omit [Carriers] in
/-- The prophecy a mutable borrow of the local `x` at `path` chooses: any
value, chosen through a mark of the borrow (`Prophecy`). Generic in the
value's type, as `Spec.choose` is, so that its rule needs no carriers. -/
def prophecyOf {α : Type} {Γ : NRow} {τ σ : NTy} (_x : Var Γ τ) (_path : Proj τ σ) :
    Spec (State) Failure α :=
  Spec.choose

omit [Carriers] in
/-- `value` is the prophecy of a mutable borrow of the local `x` at `path`:
no constraint, a mark the walk reads where it introduces the value, to name
it as the value written back (`x'`). -/
@[irreducible] def Prophecy {α : Type} {Γ : NRow} {τ σ : NTy} (_x : Var Γ τ) (_path : Proj τ σ)
    (_value : α) : Prop :=
  True

omit [Carriers] in
theorem wp_prophecyOf {α : Type} {Γ : NRow} {τ σ : NTy} (x : Var Γ τ) (path : Proj τ σ)
    (ensures : α → State → Prop) (aborts : Failure → Prop) (state : State) :
    wp (prophecyOf (State := State) x path) ensures aborts state ↔
      ∀ value, Prophecy x path value → ensures value state := by
  unfold prophecyOf Prophecy
  simp [wp_choose]

/-- Any value of a type. -/
def arbitraryIn {ρ : ResultShape} {Γ : NRow} {τ : NTy} (entry : HEnv Γ) :
    Spec State Failure (Flow ρ Γ τ.carrier) :=
  Spec.bind Spec.choose fun value => Spec.pure (.value value entry)

/-- The value the meanings give a ghost write at a site, read in the
locals and state there: none where the entry is for other locals or
another type. -/
def ghostValueOf {Γ : NRow} {τ : NTy}
    (entry : Option ((Γ' : NRow) × (τ' : NTy) × (HEnv Γ' → State → Option τ'.carrier)))
    (env : HEnv Γ) (state : State) : Option τ.carrier :=
  match entry with
  | some ⟨Γ', τ', value⟩ =>
      if same : Γ = Γ' ∧ τ = τ' then
        (value (same.1 ▸ env) state).map (cast (congrArg NTy.carrier same.2.symm))
      else none
  | none => none

@[simp] theorem ghostValueOf_none {Γ : NRow} {τ : NTy} (env : HEnv Γ) (state : State) :
    ghostValueOf (τ := τ) none env state = none := rfl

@[simp] theorem ghostValueOf_same {Γ : NRow} {τ : NTy}
    (value : HEnv Γ → State → Option τ.carrier) (env : HEnv Γ) (state : State) :
    ghostValueOf (some ⟨Γ, τ, value⟩) env state = value env state := by
  simp only [ghostValueOf, and_self, dite_true, cast_eq]
  cases value env state <;> rfl

/-- A ghost write's value, marked with its site: the value the meanings
give it there. That they give one — the value is in the field type's
domain — is owed, as the Move Prover asserts it. -/
def ghostAt {ρ : ResultShape} {Γ : NRow} {τ : NTy} (_site : Nat)
    (chosen : State → Option τ.carrier) (entry : HEnv Γ) :
    Spec State Failure (Flow ρ Γ τ.carrier) :=
  Spec.bind (Spec.certifyState fun state => (chosen state).isSome) fun _ =>
  Spec.bind Spec.get fun state => Spec.bind Spec.choose fun value =>
    Spec.bind (Spec.assume (chosen state = some value)) fun _ =>
      Spec.pure (.value value entry)

/-- The initializer the meanings give a pack's site, at the packed value's
type and its slot's: none where the entry is for other types. -/
def ghostInitOf {σs : NRow} {σ : NTy}
    (entry : Option ((fields : NRow) × (σ' : NTy) ×
      (HList fields → State → Option σ'.carrier))) :
    Option (HList σs → State → Option σ.carrier) :=
  match entry with
  | some ⟨fields, σ', initializer⟩ =>
      if same : σs = fields ∧ σ = σ' then
        some fun values state =>
          (initializer (cast (congrArg HList same.1) values) state).map
            (cast (congrArg NTy.carrier same.2.symm))
      else none
  | none => none

@[simp] theorem ghostInitOf_none {σs : NRow} {σ : NTy} :
    ghostInitOf (State := State) (σs := σs) (σ := σ) none = none := rfl

@[simp] theorem ghostInitOf_same {σs : NRow} {σ : NTy}
    (initializer : HList σs → State → Option σ.carrier) :
    ghostInitOf (some ⟨σs, σ, initializer⟩) = some initializer := by
  simp only [ghostInitOf, and_self, dite_true, cast_eq, Option.some.injEq]
  funext values state
  cases initializer values state <;> rfl

/-- A certified field row with the ghost slot an initializer names set from
the row and the state, marked with its site; unchanged without one, the slot
holding any value already.  This runs before the row is forgotten, so the
data invariants a pack owes are those of the row its initializers leave
(`GhostInits.initialize`). -/
noncomputable def ghostInitAt {σs : NRow} {σ : NTy} (_site : Nat)
    (initializer : Option (HList σs → State → Option σ.carrier))
    (slot : Var σs (.ghost σ)) (values : HList σs) : Spec State Failure (HList σs) :=
  Spec.bind Spec.get fun state =>
    Spec.pure (match initializer.bind fun given => given values state with
      | some value => slot.update value values
      | none => values)

/-- The certified field row of a pack with every initialized ghost slot set,
in declaration order: the row the pack forgets and certifies. -/
noncomputable def GhostInits.initialize {σs : NRow}
    {unit : Validation.ValidatedUnit} (meanings : InterpreterOps unit C State) :
    GhostInits σs → HList σs → Spec State Failure (HList σs)
  | .nil, values => Spec.pure values
  | .cons site slot rest, values =>
      Spec.bind (ghostInitAt site (ghostInitOf (meanings.ghostInit site)) slot values)
        fun next => rest.initialize meanings next

@[simp] theorem GhostInits.initialize_nil {σs : NRow}
    {unit : Validation.ValidatedUnit} (meanings : InterpreterOps unit C State)
    (values : HList σs) :
    (GhostInits.nil (σs := σs)).initialize meanings values = Spec.pure values := rfl

@[simp] theorem GhostInits.initialize_cons {σs : NRow} {σ : NTy}
    {unit : Validation.ValidatedUnit} (meanings : InterpreterOps unit C State)
    (site : Nat) (slot : Var σs (.ghost σ)) (rest : GhostInits σs) (values : HList σs) :
    (GhostInits.cons site slot rest).initialize meanings values =
      Spec.bind (ghostInitAt site (ghostInitOf (meanings.ghostInit site)) slot values)
        fun next => rest.initialize meanings next := rfl

/-- A computation of a value read as one of a ghost slot holding it: the
slot's carrier is its type's. -/
@[reducible] def ghostSlotted {ρ : ResultShape} {Γ : NRow} {τ : NTy}
    (value : Spec State Failure (Flow ρ Γ τ.carrier)) : Spec State Failure (Flow ρ Γ (NTy.ghost τ).carrier) :=
  value

/-- A ghost slot's computation read as its contents'. -/
@[reducible] def ghostContents {ρ : ResultShape} {Γ : NRow} {τ : NTy}
    (slot : Spec State Failure (Flow ρ Γ (NTy.ghost τ).carrier)) : Spec State Failure (Flow ρ Γ τ.carrier) :=
  slot

/-- An in-body assumption, marked with its site: the runs on which the
condition holds there. -/
def assumeAt {ρ : ResultShape} {Γ : NRow} (_site : Nat) (holds : State → Prop)
    (entry : HEnv Γ) : Spec State Failure (Flow ρ Γ Unit) where
  ok := fun initial result final => holds initial ∧ result = .value () entry ∧ final = initial
  aborts := fun _ _ => False

/-- The end of a mutation of a local, marked with its site so that
verification can name the invariant owed there. Semantically no step. -/
def mutationEndAt {ρ : ResultShape} {Γ : NRow} (_site : Nat) (entry : HEnv Γ) :
    Spec State Failure (Flow ρ Γ Unit) :=
  Spec.pure (.value () entry)

/-- The end of a write of global memory, marked with its site so that
verification can name the invariants owed there. Semantically no step. -/
def memoryWrittenAt {ρ : ResultShape} {Γ : NRow} (_site : Nat) (entry : HEnv Γ) :
    Spec State Failure (Flow ρ Γ Unit) :=
  Spec.pure (.value () entry)

/-- An abort, marked with its site so that verification can name the paths
aborting there. Semantically the abort itself. -/
def abortAt {α : Type} (_site : Nat) (failure : Failure) : Spec State Failure α :=
  Spec.abort failure

/-- A branch of a conditional, marked with the conditional's site and the
branch taken, so that verification can name the paths through it.
Semantically the branch itself. -/
def branchAt {α : Type} (_site : Nat) (_taken : Bool) (meaning : Spec State Failure α) : Spec State Failure α :=
  meaning

/-- A branch, marked where its conditional is one of the source's
decisions. -/
def branchIn {α : Type} : Option Nat → Bool → Spec State Failure α → Spec State Failure α
  | some site, taken, meaning => branchAt site taken meaning
  | none, _, meaning => meaning

/-- What a path through a branch owes, marked with the conditional's site and
the branch taken, so that verification records the path's decision.
Reducible: the normal form and the deciders see what it owes. -/
@[reducible] def Taken (_site : Nat) (_taken : Bool) (holds : Prop) : Prop :=
  holds

/-- An arm of a `match`, marked with the match's site and the variant the arm
takes, so that verification can name the paths through it. Semantically
the arm itself. -/
def armAt {α : Type} (_site : Nat) (_variant : String) (meaning : Spec State Failure α) : Spec State Failure α :=
  meaning

/-- An arm, marked where it belongs to a source `match`. -/
def armIn {α : Type} : Option (Nat × String) → Spec State Failure α → Spec State Failure α
  | some (site, variant), meaning => armAt site variant meaning
  | none, meaning => meaning

/-- What a path through an arm owes, marked with the match's site and the
variant taken, so that verification records the path's decision.
Reducible: the normal form and the deciders see what it owes. -/
@[reducible] def Matched (_site : Nat) (_variant : String) (holds : Prop) : Prop :=
  holds

/-- An operation that may abort, marked with its site so that verification
can name the paths aborting in it. Semantically the operation itself. -/
def opAt {α : Type} (_site : Nat) (meaning : Spec State Failure α) : Spec State Failure α :=
  meaning

/-- The abort condition a failure at a site owes, marked with the site so
that verification records where the path aborts (`doc/designs/proof-language.md`,
"Obligation addressing and provenance"). Reducible: the normal form and
the deciders see the condition. -/
@[reducible] def AbortsAt (_site : Nat) (holds : Prop) : Prop :=
  holds

/-- An invocation whose function value's constructor is known, marked with
the invocation's site and the constructor's target (or `foreign`), so that
verification records the arm the invocation takes
(`doc/designs/proof-language.md`, "Obligation addressing and provenance").
Semantically the invocation itself. -/
def dispatchAt {α : Type} (_site : Nat) (_target : String) (meaning : Spec State Failure α) :
    Spec State Failure α :=
  meaning

/-- The weakest precondition of an invocation taking a known arm, marked
with the invocation's site and the arm's target. Reducible: the normal form
and the deciders see the condition. -/
@[reducible] def Dispatched (_site : Nat) (_target : String) (holds : Prop) : Prop :=
  holds

/-- An operation a proof places facts after (`after op k`), as `opAt`; the
paths continuing after it carry the site (`AfterAt`). Semantically the
operation itself. -/
def opAfterAt {α : Type} (_site : Nat) (meaning : Spec State Failure α) : Spec State Failure α :=
  meaning

/-- A source `let` of the local `x` to `value` around its body `k`
(`doc/designs/proof-language.md`, "Expressions"): semantically `k`, a binding
site where a step proof's walk names the value by the local's source name.
Generic in the value's type, as `prophecyOf` is. -/
def letAt {α β : Type} {Γ : NRow} {τ : NTy} (_x : Var Γ τ) (_value : β) (k : Spec State Failure α) :
    Spec State Failure α :=
  k

/-- A row's `let` (`let (a, b) := …`) of its targets to `values` around `k`,
as `letAt` for each target. -/
def letRowAt {α : Type} {Γ σs : NRow} (_targets : Vars Γ σs) (_values : HList σs)
    (k : Spec State Failure α) : Spec State Failure α :=
  k

/-- What a path owes after a source `let` of the local `x` to `value`:
`holds`, a binding site a step proof's walk meets (`letAt`). Not reducible:
the normal form rewrites in it but does not distribute across it. -/
def LetIn {β : Type} {Γ : NRow} {τ : NTy} (_x : Var Γ τ) (_value : β) (holds : Prop) : Prop :=
  holds

/-- What a path owes after a row's `let` (`letRowAt`), as `LetIn`. -/
def LetRowIn {Γ σs : NRow} (_targets : Vars Γ σs) (_values : HList σs) (holds : Prop) : Prop :=
  holds

/-- What a path owes after an operation, marked with the operation's site so
that verification meets the point on every path continuing there, however
the normal form distributes the path. Not reducible: the normal form
rewrites inside it but does not distribute across it. -/
def AfterAt (_site : Nat) (holds : Prop) : Prop :=
  holds

/-- A call's meaning, marked with the call's site so that verification can
name the invariants owed after it. Semantically the meaning itself. -/
def callAt {α : Type} (_site : Nat) (meaning : Spec State Failure α) : Spec State Failure α :=
  meaning

/-- A value constructed at a site, marked so that verification can name the
invariant owed of it. Semantically no step. -/
def constructedAt {ρ : ResultShape} {Γ : NRow} {α : Type} (_site : Nat) (value : α)
    (entry : HEnv Γ) : Spec State Failure (Flow ρ Γ α) :=
  Spec.pure (.value value entry)

/-- The locals and memory an anchor at a site records. -/
def AnchorOf (_site : Nat) (Γ : NRow) : Type := HEnv Γ × (State)

/-- A declared result is the body value it denotes (`ResultShape.toBody`):
the two carriers are one type. A generic call's result takes this single
transport from the callee's view, so that it is the callee's result itself
once the transport resolves, not a body projection of it. -/
theorem ResultShape.carrier_eq_bodyType : (shape : ResultShape) →
    shape.carrier = shape.bodyType.carrier
  | .none => rfl
  | .one _ => rfl

omit [Carriers] in
/-- The rows and shape of a generic call, from its compile-time evidence. -/
theorem resolvedCall {θ σs σs' : NRow} {shape shape' : ResultShape}
    (resolved : (NRow.subst θ σs == σs' && shape.subst θ == shape') = true) :
    NRow.subst θ σs = σs' ∧ shape.subst θ = shape' := by
  simpa only [Bool.and_eq_true, beq_iff_eq] using resolved

omit [Carriers] in
/-- The captured rows and the function type of a generic closure, from its
compile-time evidence. -/
theorem resolvedClosure {θ captured supplied captured' supplied' : NRow}
    {shape shape' : ResultShape} {shared : Sharing}
    (resolved : (NRow.subst θ captured == captured' && NRow.subst θ supplied == supplied' &&
      shape.subst θ == shape') = true) :
    NRow.subst θ captured = captured' ∧
      NTy.function (NRow.subst θ supplied) shared (shape.subst θ).row =
        NTy.function supplied' shared shape'.row := by
  simp only [Bool.and_eq_true, beq_iff_eq] at resolved
  obtain ⟨⟨captured, supplied⟩, shape⟩ := resolved
  subst captured supplied shape
  exact ⟨rfl, rfl⟩


end LocalComputations

/-- The function value of a closure with type arguments: `closureOf` in the
instantiation its frame computes, carrying the target's own weave and result
shape and the type arguments, which name the meaning of its invocation at the
family they induce. -/
noncomputable def closureOfGeneric (unit : VerexIR.Validation.ValidatedUnit) [FunctionCarriers unit]
    [Skolems unit]
    (outer : Array (TypeId × TypeId)) (typeArgs : Array GenericArgument) (θ : TypeArgs)
    {full captured supplied : NRow} (weave : Weave full captured supplied) (_shape : ResultShape)
    (handle : FunctionHandle) (captures : HList (NRow.subst θ.1 captured)) : ClosureValue :=
  closureOf handle weave.mask (frameInstantiation unit handle outer typeArgs) captures

/-- The function value of a closure with type arguments: `Skolems.construct` at the
closure of `closureOfGeneric`, carrying the target's own weave and result
shape and the type arguments, which name the meaning of its invocation. -/
noncomputable def functionOfGeneric (unit : VerexIR.Validation.ValidatedUnit) [FunctionCarriers unit]
    [Θ : Skolems unit]
    (outer : Array (TypeId × TypeId)) (typeArgs : Array GenericArgument) (θ : TypeArgs)
    {full captured supplied : NRow} (weave : Weave full captured supplied) (shape : ResultShape)
    (shared : Sharing) (handle : FunctionHandle) (captures : HList (NRow.subst θ.1 captured))
    (typed : SiteTyped (NRow.subst θ.1 supplied) shared (shape.subst θ.1).row
      handle weave.mask (frameInstantiation unit handle outer typeArgs) (NRow.subst θ.1 captured)) :
    (NTy.function (NRow.subst θ.1 supplied) shared (shape.subst θ.1).row).carrier :=
  Skolems.construct (NRow.subst θ.1 supplied) shared (shape.subst θ.1).row handle weave.mask
    (frameInstantiation unit handle outer typeArgs) captures typed

/-- A created generic function value's runtime closure is the closure
created. -/
theorem functionOfGeneric_closure
    (outer : Array (TypeId × TypeId)) (typeArgs : Array GenericArgument)
    (θ : TypeArgs) {full captured supplied : NRow} (weave : Weave full captured supplied)
    (shape : ResultShape) (shared : Sharing) (handle : FunctionHandle)
    (captures : HList (NRow.subst θ.1 captured))
    (typed : SiteTyped (NRow.subst θ.1 supplied) shared (shape.subst θ.1).row
      handle weave.mask (frameInstantiation unit handle outer typeArgs) (NRow.subst θ.1 captured)) :
    (Carriers.functions (NRow.subst θ.1 supplied) shared (shape.subst θ.1).row).closure
        (functionOfGeneric unit outer typeArgs θ weave shape shared handle captures typed) =
      closureOfGeneric unit outer typeArgs θ weave shape handle captures :=
  Skolems.closure_construct _ _ _ _ _ _ captures typed

/-- A closure's target frame as its creation takes it: coherent with the
closure's instantiation, which is faithful to the target's frame. -/
def ClosureFrame (unit : VerexIR.Validation.ValidatedUnit) [FunctionCarriers unit] [Skolems unit]
    (handle : FunctionHandle) (typeInstantiation : Array (TypeId × TypeId)) : Prop :=
  Coherent unit handle typeInstantiation ∧ closureFaithful unit handle typeInstantiation = true

/-- A closure of a target with type parameters, at a frame coherent with the
instantiation its caller's frame computes, is typed at the caller's rows: the
frame's resolution of its target's own rows (`closureRowsIn?_coherent`). -/
theorem closureOfGeneric_typed
    (outer : Array (TypeId × TypeId)) (typeArgs : Array GenericArgument)
    (θ : TypeArgs) {full captured supplied : NRow} (weave : Weave full captured supplied)
    (shape : ResultShape) {handle : FunctionHandle} {shared : Sharing}
    (rows : closureRows? unit handle weave.mask = some (captured, supplied, shape.row))
    (sharing : closureShared unit handle weave.mask = shared)
    (below : closureSignatureBelow unit handle = true)
    (frame : @ClosureFrame unit _ (Skolems.instantiate θ ‹_›) handle
      (frameInstantiation unit handle outer typeArgs)) :
    SiteTyped (NRow.subst θ.1 supplied) shared (shape.subst θ.1).row
      handle weave.mask (frameInstantiation unit handle outer typeArgs) (NRow.subst θ.1 captured) := by
  have atFrame := @closureRowsIn?_coherent unit _ (Skolems.instantiate θ ‹_›) handle weave.mask
    (frameInstantiation unit handle outer typeArgs) captured supplied shape.row rows below frame.1
  simp only [NRow.resolved_instantiate] at atFrame
  refine ⟨?_, sharing, frame.2⟩
  rw [atFrame, ResultShape.row_subst]

/-- Concrete runtime operations implementing the shared interpreter interface.
The generic boundary transports once, here, into and out of the callee frame. -/
@[reducible] noncomputable def Meanings.interpreterOps
    {executable : Validation.ExecutableUnit unit} [Θ : Skolems unit]
    (meanings : Meanings executable) : InterpreterOps unit Θ.toCarriers (Memory unit) where
  call := meanings.call
  generic := fun handle typeArgs θ parameters shape values =>
    Spec.bind (meanings.generic handle typeArgs θ parameters shape (HList.toSkolem θ parameters values))
      fun result => Spec.pure (ResultShape.ofSkolem θ shape result)
  invoke := meanings.invoke
  closure := fun handle weave results shared rows sharing closed faithful values =>
    Spec.pure (Skolems.construct _ shared results handle weave.mask #[] values
      (SiteTyped.ofClosure rows sharing closed faithful))
  closureGeneric := fun handle weave typeArgs θ shape shared rows sharing below values =>
    Spec.given (@ClosureFrame unit _ (Skolems.instantiate θ Θ) handle
        (frameInstantiation unit handle meanings.typeInstantiation typeArgs)) fun frame =>
      Spec.pure (functionOfGeneric unit meanings.typeInstantiation typeArgs θ weave shape shared
        handle values (closureOfGeneric_typed meanings.typeInstantiation typeArgs θ weave shape
          rows sharing below frame))
  read := fun τ arguments key memory =>
    (memory (Skolems.resource τ arguments) key).map (Skolems.ofRuntime τ)
  write := fun τ arguments key value memory =>
    memory.set (Skolems.resource τ arguments) key (value.map (Skolems.toRuntime τ))
  assumption := meanings.assumption
  ghost := meanings.ghost
  ghostInit := meanings.ghostInit
  atInstance := meanings.atInstance

/-- The outcome of a place or selection that resolves nothing: the profile's
throw, or else undefined, which a proof must rule out by the check before it. -/
def mismatchOutcome {State α : Type} : Option Failure → Spec State Failure α
  | some failure => Spec.abort failure
  | none => Spec.stuck

@[simp] theorem mismatchOutcome_some {State α : Type} (failure : Failure) :
    (mismatchOutcome (some failure) : Spec State Failure α) = Spec.abort failure := rfl
@[simp] theorem mismatchOutcome_none {State α : Type} :
    (mismatchOutcome none : Spec State Failure α) = Spec.stuck := rfl

/-- Fixed-extent slice operation, factored out of the recursive term evaluator. -/
def borrowArraySliceAt [Carriers] {State : Type} {ρ : ResultShape} {Γ : NRow} {τ σ : NTy} {length : Nat}
    (mismatch : Option Failure) (x : Var Γ τ) (path : Proj τ (.array σ length)) (env : HEnv Γ) :
    Spec State Failure (Flow ρ Γ (NTy.sliceRef σ).carrier) :=
  match x.get env with
  | some value => match path.get? env (NTy.forget τ value) with
    | some current =>
        Spec.bind (prophecyOf (α := (NTy.sliceRef σ).carrier × τ.carrier) x path) fun chosen =>
          Spec.bind (Spec.assume (∃ final : SpecArray σ.plain length,
              NTy.forget (.sliceRef σ) chosen.1 = SpecSliceRef.ofArrays current final ∧
                path.set? env final (NTy.forget τ value) = some (NTy.forget τ chosen.2))) fun _ =>
            Spec.pure (.value chosen.1 (x.set chosen.2 env))
    | none => mismatchOutcome mismatch
  | none => Spec.bottom

/-- Fixed-extent slice operation, factored out of the recursive term evaluator. -/
def borrowVectorSliceAt [Carriers] {State : Type} {ρ : ResultShape} {Γ : NRow} {τ σ : NTy}
    (mismatch : Option Failure) (x : Var Γ τ) (path : Proj τ (.vector σ)) (env : HEnv Γ) :
    Spec State Failure (Flow ρ Γ (NTy.sliceRef σ).carrier) :=
  match x.get env with
  | some value => match path.get? env (NTy.forget τ value) with
    | some current =>
        Spec.bind (prophecyOf (α := (NTy.sliceRef σ).carrier × τ.carrier) x path) fun chosen =>
          Spec.bind (Spec.assume (∃ final : SpecArray σ.plain current.values.size,
              NTy.forget (.sliceRef σ) chosen.1 = SpecSliceRef.ofArrays ⟨current, rfl⟩ final ∧
                path.set? env final.vector (NTy.forget τ value) =
                  some (NTy.forget τ chosen.2))) fun _ =>
            Spec.pure (.value chosen.1 (x.set chosen.2 env))
    | none => mismatchOutcome mismatch
  | none => Spec.bottom

/-- Fixed-extent slice operation, factored out of the recursive term evaluator. -/
def reborrowSliceAt [Carriers] {State : Type} {ρ : ResultShape} {Γ : NRow} {τ σ : NTy}
    (mismatch : Option Failure) (x : Var Γ τ) (path : Proj τ (.sliceRef σ)) (env : HEnv Γ) :
    Spec State Failure (Flow ρ Γ (NTy.sliceRef σ).carrier) :=
  match x.get env with
  | some value => match path.get? env (NTy.forget τ value) with
    | some current =>
        Spec.bind (prophecyOf (α := (NTy.sliceRef σ).carrier × τ.carrier) x path) fun chosen =>
          Spec.bind (Spec.assume (∃ final : SpecArray σ.plain current.extent,
              NTy.forget (.sliceRef σ) chosen.1 =
                  SpecSliceRef.ofArrays current.current final ∧
                path.set? env (current.withCurrent final) (NTy.forget τ value) =
                  some (NTy.forget τ chosen.2))) fun _ =>
            Spec.pure (.value chosen.1 (x.set chosen.2 env))
    | none => mismatchOutcome mismatch
  | none => Spec.bottom

/-- Fixed-extent slice operation, factored out of the recursive term evaluator. -/
def takeSliceAtValue [Carriers] {State : Type} {ρ : ResultShape} {Γ : NRow} {τ σ : NTy}
    (mismatch : Option Failure) (x : Var Γ τ) (path : Proj τ (.sliceRef σ)) (env : HEnv Γ) :
    Spec State Failure (Flow ρ Γ (NTy.sliceRef σ).carrier) :=
  match x.get env with
  | some value => match path.getCertified? env value with
    | some reference =>
        let settled := NTy.forget (.sliceRef σ) reference
        Spec.bind (prophecyOf (α := τ.carrier) x path) fun cleared =>
          Spec.bind (Spec.assume (path.set? env
              (SpecSliceRef.ofArrays settled.final settled.final) (NTy.forget τ value) =
                some (NTy.forget τ cleared))) fun _ =>
            Spec.pure (.value reference (x.set cleared env))
    | none => mismatchOutcome mismatch
  | none => Spec.bottom

/-- Fixed-extent slice operation, factored out of the recursive term evaluator. -/
def resolveSliceAtValue [Carriers] {State : Type} {ρ : ResultShape} {Γ : NRow} {τ σ : NTy}
    (x : Var Γ τ) (path : Proj τ (.sliceRef σ)) (env : HEnv Γ) :
    Spec State Failure (Flow ρ Γ Unit) :=
  match (x.get env).bind fun value => path.get? env (NTy.forget τ value) with
  | some reference => Spec.bind (Spec.assume (reference.current = reference.final)) fun _ =>
      Spec.pure (.value () env)
  | none => Spec.pure (.value () env)

mutual
/-- The denotation of a term: its outcome with the locals after it. -/
noncomputable def Term.denoteIn {unit : Validation.ValidatedUnit} [C : Carriers] {State : Type}
    (meanings : InterpreterOps unit C State) {ρ : ResultShape} : {Γ : NRow} → {τ : NTy} →
    Term unit ρ Γ τ → HEnv Γ → Spec State Failure (Flow ρ Γ τ.carrier)
  | _, _, .lit value literal, env => Spec.pure (.value (NTy.ofGround _ literal value) env)
  | _, _, .var x, env =>
      match x.get env with
      | some value => Spec.pure (.value value env)
      | none => Spec.bottom
  | _, _, .checked site op failure left right, env =>
      Flow.bind (left.denoteIn meanings env) fun l env =>
        Flow.bind (right.denoteIn meanings env) fun r env =>
          Spec.bind (opAt site (op.run failure l r)) fun value => Spec.pure (.value value env)
  | _, _, .modular op left right, env =>
      Flow.bind (left.denoteIn meanings env) fun l env =>
        Flow.bind (right.denoteIn meanings env) fun r env => Spec.pure (.value (op.run l r) env)
  | _, _, .compare op left right, env =>
      Flow.bind (left.denoteIn meanings env) fun l env =>
        Flow.bind (right.denoteIn meanings env) fun r env =>
          Spec.pure (.value (op.decide l.val r.val) env)
  | _, _, .equal negated left right, env =>
      Flow.bind (left.denoteIn meanings env) fun l env =>
        Flow.bind (right.denoteIn meanings env) fun r env =>
          Spec.pure (.value (if negated then !NTy.eqb _ l r else NTy.eqb _ l r) env)
  | _, _, @Term.equalErased _ _ _ τ negated left right, env =>
      Flow.bind (left.denoteIn meanings env) fun l env =>
        Flow.bind (right.denoteIn meanings env) fun r env =>
          let same := NTy.equal false τ l r
          Spec.pure (.value (if negated then !same else same) env)
  | _, _, .not operand, env =>
      Flow.bind (operand.denoteIn meanings env) fun b env => Spec.pure (.value (!(b : Bool)) env)
  | _, _, .logical conjunction left right, env =>
      Flow.bind (left.denoteIn meanings env) fun l env =>
        Flow.bind (right.denoteIn meanings env) fun r env =>
          Spec.pure (.value (if conjunction then (l : Bool) && r else (l : Bool) || r) env)
  | _, _, .bitwise op left right, env =>
      Flow.bind (left.denoteIn meanings env) fun l env =>
        Flow.bind (right.denoteIn meanings env) fun r env => Spec.pure (.value (op.run l r) env)
  | _, _, .shift site left failure value distance, env =>
      Flow.bind (value.denoteIn meanings env) fun v env =>
        Flow.bind (distance.denoteIn meanings env) fun d env =>
          Spec.bind (opAt site (if left then checkedShiftLeft failure v d
            else checkedShiftRight failure v d)) fun result => Spec.pure (.value result env)
  | _, .int width signed, .shiftSigned site left failure value distance, env =>
      Flow.bind (value.denoteIn meanings env) fun v env =>
        Flow.bind (distance.denoteIn meanings env) fun d env =>
          opAt site (if 0 ≤ d.val ∧ d.val < (width : Int) then
            Spec.pure (.value ⟨wrapInt width signed (if left then Int.shiftLeft v.val d.val.toNat
              else Int.shiftRight v.val d.val.toNat), wrapInt_fits _ _ _ v.width_nonzero⟩ env)
          else abortAt site (Failure.ofInts failure #[d.val]))
  | _, .int width signed, .shiftUnchecked site left value distance, env =>
      Flow.bind (value.denoteIn meanings env) fun v env =>
        Flow.bind (distance.denoteIn meanings env) fun d env =>
          opAt site (if 0 ≤ d.val ∧ d.val < (width : Int) then
            Spec.pure (.value ⟨wrapInt width signed (if left then Int.shiftLeft v.val d.val.toNat
              else Int.shiftRight v.val d.val.toNat), wrapInt_fits _ _ _ v.width_nonzero⟩ env)
          else Spec.stuck)
  | _, .int width signed, .divideUnchecked site modulo left right, env =>
      Flow.bind (left.denoteIn meanings env) fun l env =>
        Flow.bind (right.denoteIn meanings env) fun r env =>
          opAt site (if r.val = 0 then Spec.stuck
          else Spec.pure (.value ⟨wrapInt width signed (if modulo then l.val.tmod r.val else l.val.tdiv r.val),
            wrapInt_fits _ _ _ l.width_nonzero⟩ env))
  | _, _, .overflowing op left right, env =>
      Flow.bind (left.denoteIn meanings env) fun l env =>
        Flow.bind (right.denoteIn meanings env) fun r env =>
          Spec.pure (.value ((op.run l r, (op.overflows l r, ())) :
            HList (.cons (.int _ _) (.cons .bool .nil))) env)
  | _, .int width' signed', .wrap nonzero value, env =>
      Flow.bind (value.denoteIn meanings env) fun v env =>
        Spec.pure (.value ⟨wrapInt width' signed' v.val, wrapInt_fits _ _ _ nonzero⟩ env)
  | _, _, .charCode value, env =>
      Flow.bind (value.denoteIn meanings env) fun v env => Spec.pure (.value (SpecChar.code v) env)
  | _, _, .charOf site value, env =>
      Flow.bind (value.denoteIn meanings env) fun v env =>
        opAt site (if valid : 0 ≤ v.val ∧ isUnicodeScalar v.val.toNat = true then
          Spec.pure (.value ⟨v.val.toNat, valid.2⟩ env)
        else Spec.stuck)
  | _, _, .charOfChecked site failure value, env =>
      Flow.bind (value.denoteIn meanings env) fun v env =>
        opAt site (if valid : 0 ≤ v.val ∧ isUnicodeScalar v.val.toNat = true then
          Spec.pure (.value ⟨v.val.toNat, valid.2⟩ env)
        else abortAt site (Failure.ofInts failure #[v.val]))
  | _, _, .stringLength value, env =>
      Flow.bind (value.denoteIn meanings env) fun v env =>
        Spec.pure (.value ⟨wrapInt 64 false (v : String).utf8ByteSize,
          wrapInt_fits _ _ _ (by decide)⟩ env)
  | _, _, .unreachable, _ => Spec.bottom
  | _, .int width' signed', .cast site failure value, env =>
      Flow.bind (value.denoteIn meanings env) fun v env =>
        Spec.bind (opAt site (checkedInt failure width' signed' v.val)) fun result =>
          Spec.pure (.value result env)
  | _, _, .ite site condition thenBranch elseBranch, env =>
      Flow.bind (condition.denoteIn meanings env) fun b env =>
        if (b : Bool) then branchIn site true (thenBranch.denoteIn meanings env)
        else branchIn site false (elseBranch.denoteIn meanings env)
  | _, _, .let_ x value body, env =>
      Flow.bind (value.denoteIn meanings env) fun v env => letAt x v (body.denoteIn meanings (x.set v env))
  | _, _, .drop value body, env =>
      Flow.bind (value.denoteIn meanings env) fun _ env => body.denoteIn meanings env
  | _, _, .assign x value, env =>
      Flow.bind (value.denoteIn meanings env) fun v env => Spec.pure (.value () (x.set v env))
  | _, _, .const value, env =>
      Spec.bind (value.denoteIn meanings ()) fun flow => Spec.pure (flow.rebase env)
  | _, _, .throw0 site kind, _ => abortAt site (Failure.ofInts kind #[])
  | _, _, .throw1 site kind code, env =>
      Flow.bind (code.denoteIn meanings env) fun value _ =>
        abortAt site (Failure.ofInts kind #[value.val])
  | _, _, .return_ value, env =>
      Flow.bind (value.denoteIn meanings env) fun v env => Spec.pure (.return_ (ρ.ofBody v) env)
  | _, _, .break_ nest, env => Spec.pure (.break_ nest env)
  | _, _, .continue_ nest, env => Spec.pure (.continue_ nest env)
  | _, _, .loop site body, env =>
      loopAt site (fun recurse env => Spec.bind (body.denoteIn meanings env) (Flow.iterate recurse)) env
  | _, _, .call site handle shape arguments, env =>
      Flow.bind (arguments.denoteIn meanings env) fun values env =>
        Spec.bind (callAt site (meanings.call handle _ shape values)) fun result =>
          Spec.pure (.value (shape.toBody result) env)
  | _, _, .callGeneric site handle typeArgs θ shape shape' resolved arguments, env =>
      Flow.bind (arguments.denoteIn meanings env) fun values env =>
        Spec.bind (callAt site (meanings.generic handle typeArgs θ _ shape
            (cast (congrArg HList (resolvedCall resolved).1.symm) values)))
          fun result => Spec.pure (.value
            (cast ((congrArg ResultShape.carrier (resolvedCall resolved).2).trans
              shape'.carrier_eq_bodyType) result) env)
  | _, _, .callInstance site handle typeArgs θ shape arguments, env =>
      Flow.bind (arguments.denoteIn meanings env) fun values env =>
        Spec.bind (callAt site (meanings.atInstance handle typeArgs θ _ shape values)) fun result =>
          Spec.pure (.value (shape.toBody result) env)
  | _, _, .closure (supplied := supplied) handle weave results shared rows sharing closed faithful
      captures, env =>
      Flow.bind (captures.denoteIn meanings env) fun values env =>
        Spec.bind (meanings.closure handle weave results shared rows sharing closed faithful values)
          fun value => Spec.pure (.value value env)
  | _, _, .closureGeneric handle weave typeArgs θ shape _ shared rows sharing below resolved
      captures, env =>
      Flow.bind (captures.denoteIn meanings env) fun values env =>
        Spec.bind (meanings.closureGeneric handle weave typeArgs θ shape shared rows sharing below
            (cast (congrArg HList (resolvedClosure (shared := shared) resolved).1.symm) values))
          fun value => Spec.pure (.value
            (cast (congrArg NTy.carrier (resolvedClosure (shared := shared) resolved).2) value) env)
  | _, _, .invoke site shape function arguments, env =>
      Flow.bind (function.denoteIn meanings env) fun closure env =>
        Flow.bind (arguments.denoteIn meanings env) fun values env =>
          Spec.bind (opAt site (meanings.invoke _ _ shape closure values)) fun result =>
            Spec.pure (.value (shape.toBody result) env)
  | _, _, .tuple elements, env =>
      Flow.bind (elements.denoteIn meanings env) fun values env => Spec.pure (.value values env)
  | _, _, @Term.pack _ _ _ σs site source arguments fields ghosts, env =>
      Flow.bind (fields.denoteIn meanings env) fun values env =>
        Spec.bind (ghosts.initialize meanings values) fun row =>
          Spec.bind (certifyingAt site .construction (.struct source arguments σs)
              (HList.forget σs row)) fun packed =>
            Spec.pure (.value packed env)
  | _, _, @Term.variant _ _ _ σs names rows site source arguments distinct choice fields ghosts,
      env =>
      Flow.bind (fields.denoteIn meanings env) fun values env =>
        Spec.bind (ghosts.initialize meanings values) fun row =>
          Spec.bind (certifyingAt site .construction
              (.enum source arguments names rows distinct)
              (choice.inject (HList.forget σs row))) fun packed =>
            Spec.pure (.value packed env)
  | _, _, @Term.field _ _ _ _ _ _ _ x value, env =>
      Flow.bind (value.denoteIn meanings env) fun v env =>
        Spec.pure (.value (x.selectCertified v) env)
  | _, _, .isVariant tests value, env =>
      Flow.bind (value.denoteIn meanings env) fun v env =>
        Spec.pure (.value (namedIn (variantPlainName _ _ v.val) tests) env)
  | _, _, .payload mismatch choices value, env =>
      Flow.bind (value.denoteIn meanings env) fun v env =>
        -- The certified selection, not a plain one with the certificate
        -- taken from the equation here: a dependent match on the plain
        -- selection would carry that equation into every reading of this
        -- case, and a normalization that rewrites the selection leaves the
        -- matcher's motive behind (`Choices.selectCertified?`).
        match choices.selectCertified? v with
        | some field => Spec.pure (.value field env)
        | none => mismatchOutcome mismatch
  | _, _, .letRow targets value body, env =>
      Flow.bind (value.denoteIn meanings env) fun values env =>
        letRowAt targets values (body.denoteIn meanings (targets.set values env))
  | _, _, @Term.letFields _ _ _ σs _ _ _ targets value body, env =>
      Flow.bind (value.denoteIn meanings env) fun values env =>
        body.denoteIn meanings
          (targets.set (HListPlain.ofDeepHolds σs values.val (NTy.fieldsHold values)) env)
  | _, _, .caseOf scrutinee arms, env =>
      Flow.bind (scrutinee.denoteIn meanings env) fun value env => arms.denoteIn meanings value env
  | _, _, @Term.deref _ _ _ τ site value, env =>
      Flow.bind (value.denoteIn meanings env) fun v env =>
        Spec.bind (certifyingAt site .copyOut τ v.1) fun current =>
          Spec.pure (.value current env)
  | _, _, @Term.copy _ _ _ τ value free, env =>
      Flow.bind (value.denoteIn meanings env) fun v env =>
        Spec.pure (.value
          (cast (NTy.carrier_eq_plain_of_certificateFree τ free).symm v.1) env)
  | _, _, @Term.observe _ _ _ τ site value, env =>
      Flow.bind (value.denoteIn meanings env) fun v env =>
        Spec.bind (certifyingAt site .copyOut τ.erase (τ.plainObserve (τ.forget v))) fun observed =>
          Spec.pure (.value observed env)
  | _, _, .take x, env =>
      match x.get env with
      | some value => Spec.pure (.value value (x.clear env))
      | none => Spec.bottom
  | _, _, .resolve site x lender, env =>
      match x.get env with
      | some reference => lender.resolve site env reference
      | none => Spec.pure (.value () env)
  | _, _, @Term.assumeCurrent _ _ _ σ x, env =>
      match x.get env with
      | some reference =>
          Spec.bind (Spec.assume (NTy.deepHolds σ reference.1)) fun _ => Spec.pure (.value () env)
      | none => Spec.pure (.value () env)
  | _, _, @Term.handedOver _ _ _ σ site reference, env =>
      Flow.bind (reference.denoteIn meanings env) fun v env =>
        Spec.bind (certifyingAt site .handOver σ v.1) fun _ => Spec.pure (.value v env)
  | _, _, @Term.returned _ _ _ σ site reference, env =>
      Flow.bind (reference.denoteIn meanings env) fun v env =>
        Spec.bind (certifyingAt site .exit σ v.1) fun _ => Spec.pure (.value v env)
  | _, _, .resolveAt site x path lender, env =>
      match (x.get env).bind (path.getCertified? env) with
      | some reference => lender.resolve site env reference
      | none => Spec.pure (.value () env)
  | _, _, @Term.mutate _ _ _ τ x value, env =>
      Flow.bind (value.denoteIn meanings env) fun v env =>
        match x.get env with
        | some current => Spec.pure (.value () (x.set (NTy.forget τ v, current.2) env))
        | none => Spec.bottom
  | _, _, @Term.readPlace _ _ _ τ σ site mismatch x path, env =>
      match x.get env with
      | some value =>
          if path.hasDeref then
            -- A value copied out of a current the loan may have left open
            -- is certified at the copy
            -- (`doc/designs/data-invariants.md`).
            match path.get? env (NTy.forget τ value) with
            | some component =>
                Spec.bind (certifyingAt site .copyOut σ component) fun copied =>
                  Spec.pure (.value copied env)
            | none => mismatchOutcome mismatch
          else
            -- The certified read, not a plain one with the certificate
            -- taken from the equation here: a dependent match on the plain
            -- read would carry that equation into every reading of this
            -- case, and a normalization that rewrites the read leaves the
            -- matcher's motive behind (`Proj.getCertified?`).
            match path.getCertified? env value with
            | some component => Spec.pure (.value component env)
            | none => mismatchOutcome mismatch
      | none => Spec.bottom
  | _, _, @Term.copyPlace _ _ _ τ σ mismatch x path free, env =>
      match x.get env with
      | some value =>
          -- The plain read of the owner's plain value, certified by the
          -- component's type, which costs one proof term
          -- (`NTy.carrier_eq_plain_of_certificateFree`).
          match path.get? env (NTy.forget τ value) with
          | some component =>
              Spec.pure (.value
                (cast (NTy.carrier_eq_plain_of_certificateFree σ free).symm component) env)
          | none => mismatchOutcome mismatch
      | none => Spec.bottom
  | _, _, @Term.takeAt _ _ _ τ σ site mismatch x path, env =>
      match x.get env with
      | some value =>
          -- Clearing the place writes the prophecy back settled, and the
          -- owner owes its data invariants at the write (`setOwner?`), as at
          -- any write into a root; only the Rust profile moves a reference
          -- out of an aggregate, where every declaration's clause is trivial
          -- today.
          match path.getCertified? env value with
          | some reference =>
              match path.setOwner? env site (NTy.forget σ reference.2, reference.2) value with
              | some written =>
                  Spec.bind written fun owner => Spec.pure (.value reference (x.set owner env))
              | none => mismatchOutcome mismatch
          | none => mismatchOutcome mismatch
      | none => Spec.bottom
  | _, _, @Term.writePlace _ _ _ _ _ site mismatch x path value, env =>
      Flow.bind (value.denoteIn meanings env) fun v env =>
        match x.get env with
        | some current =>
            match path.setOwner? env site v current with
            | some written => Spec.bind written fun owner => Spec.pure (.value () (x.set owner env))
            | none => mismatchOutcome mismatch
        | none => Spec.bottom
  -- A whole-local borrow has one prophecy: its referent is its owner.
  -- The general path case below chooses both and relates their plain values;
  -- `Term.denoteIn_borrowPlace_nil_expanded` proves the two readings equal.
  | _, _, @Term.borrowPlace _ _ _ τ _ _ x .nil, env =>
      match x.get env with
      | some value =>
          Spec.bind (prophecyOf (α := τ.carrier) x (.nil : Proj τ τ)) fun chosen =>
            Spec.pure (.value (NTy.forget τ value, chosen) (x.set chosen env))
      | none => Spec.bottom
  | _, _, @Term.borrowPlace _ _ _ τ σ mismatch x path, env =>
      match x.get env with
      | some value =>
          match path.get? env (NTy.forget τ value) with
          | some component =>
              Spec.bind (prophecyOf (α := σ.carrier × τ.carrier) x path) fun chosen =>
                Spec.bind (Spec.assume (path.set? env (NTy.forget σ chosen.1)
                    (NTy.forget τ value) = some (NTy.forget τ chosen.2))) fun _ =>
                  Spec.pure (.value (component, chosen.1) (x.set chosen.2 env))
          | none => mismatchOutcome mismatch
      | none => Spec.bottom
  | _, _, .sliceVector value, env =>
      Flow.bind (value.denoteIn meanings env) fun value env => Spec.pure (.value value env)
  | _, _, .vectorSlice value, env =>
      Flow.bind (value.denoteIn meanings env) fun value env => Spec.pure (.value value env)
  | _, _, .sliceCurrent value, env =>
      Flow.bind (value.denoteIn meanings env) fun value env =>
        Spec.pure (.value value.currentVector env)
  | _, _, .borrowArraySlice mismatch x path, env => borrowArraySliceAt mismatch x path env
  | _, _, .borrowVectorSlice mismatch x path, env => borrowVectorSliceAt mismatch x path env
  | _, _, .reborrowSlice mismatch x path, env => reborrowSliceAt mismatch x path env
  | _, _, .takeSliceAt mismatch x path, env => takeSliceAtValue mismatch x path env
  | _, _, .resolveSliceAt x path, env => resolveSliceAtValue x path env
  | _, _, .vectorLit count elements, env =>
      Flow.bind (elements.denoteIn meanings env) fun values env =>
        match SpecVector.ofArray? (HList.toListOf count values).toArray with
        | some vector => Spec.pure (.value vector env)
        | none => Spec.bottom
  | _, _, @Term.arrayVector _ _ _ _ length array, env =>
      Flow.bind (array.denoteIn meanings env) fun a env =>
        Spec.bind (Spec.assume (a.vector.values.size = length)) fun _ =>
          Spec.pure (.value a.vector env)
  | _, .array _ length, .arrayOf vector, env =>
      Flow.bind (vector.denoteIn meanings env) fun values env =>
        if sized : values.values.size = length then Spec.pure (.value ⟨values, sized⟩ env)
        else Spec.stuck
  | _, _, .repeatVector count value, env =>
      Flow.bind (value.denoteIn meanings env) fun v env =>
        match SpecVector.ofArray? (Array.replicate count v) with
        | some vector => Spec.pure (.value vector env)
        | none => Spec.stuck
  | _, _, .length vector, env =>
      Flow.bind (vector.denoteIn meanings env) fun values env =>
        Spec.pure (.value (vectorLength values) env)
  | _, _, .signerAddress signer, env =>
      Flow.bind (signer.denoteIn meanings env) fun value env => Spec.pure (.value value env)
  | _, _, .index site vector position, env =>
      Flow.bind (vector.denoteIn meanings env) fun values env =>
        Flow.bind (position.denoteIn meanings env) fun i env =>
          opAt site (if i.val < 0 then abortAt site (Failure.ofInts .abort #[i.val])
          else match values.values[i.val.toNat]? with
            | some element => Spec.pure (.value element env)
            | none => abortAt site (Failure.ofInts .abort #[i.val]))
  | _, _, .checkIndex site failure vector position, env =>
      Flow.bind (vector.denoteIn meanings env) fun values env =>
        Flow.bind (position.denoteIn meanings env) fun i env =>
          opAt site (if 0 ≤ i.val ∧ i.val < values.values.size then Spec.pure (.value () env)
          else abortAt site (Failure.ofInts failure #[1]))
  | _, _, .push vector element, env =>
      Flow.bind (vector.denoteIn meanings env) fun values env =>
        Flow.bind (element.denoteIn meanings env) fun e env =>
          match SpecVector.ofArray? (values.values.push e) with
          | some vector => Spec.pure (.value vector env)
          | none => Spec.bottom
  | _, _, .insert site vector position element, env =>
      Flow.bind (vector.denoteIn meanings env) fun values env =>
        Flow.bind (position.denoteIn meanings env) fun i env =>
          Flow.bind (element.denoteIn meanings env) fun e env =>
            opAt site (if i.val < 0 ∨ values.values.size < i.val.toNat then
              abortAt site (Failure.ofInts .abort #[i.val])
            else match SpecVector.ofArray? (values.values.insertIdxIfInBounds i.val.toNat e) with
              | some vector => Spec.pure (.value vector env)
              | none => Spec.bottom)
  | _, _, .remove site vector position, env =>
      Flow.bind (vector.denoteIn meanings env) fun values env =>
        Flow.bind (position.denoteIn meanings env) fun i env =>
          opAt site (if i.val < 0 then abortAt site (Failure.ofInts .abort #[i.val])
          else match values.values[i.val.toNat]? with
            | some element =>
                match SpecVector.ofArray? (values.values.eraseIdxIfInBounds i.val.toNat) with
                | some remaining =>
                    Spec.pure (.value ((element, (remaining, ())) :
                      HList (.cons _ (.cons (.vector _) .nil))) env)
                | none => Spec.bottom
            | none => abortAt site (Failure.ofInts .abort #[i.val]))
  | _, _, .swap site vector left right, env =>
      Flow.bind (vector.denoteIn meanings env) fun values env =>
        Flow.bind (left.denoteIn meanings env) fun i env =>
          Flow.bind (right.denoteIn meanings env) fun j env =>
            if i.val < 0 ∨ j.val < 0 ∨ values.values.size ≤ i.val.toNat ∨
                values.values.size ≤ j.val.toNat then
              abortAt site (Failure.ofInts .abort #[i.val, j.val])
            else match values.values[i.val.toNat]? with
              | none => abortAt site (Failure.ofInts .abort #[i.val, j.val])
              | some a =>
                match values.values[j.val.toNat]? with
                | none => abortAt site (Failure.ofInts .abort #[i.val, j.val])
                | some b =>
                  match SpecVector.ofArray?
                      ((values.values.set! i.val.toNat b).set! j.val.toNat a) with
                  | some vector => Spec.pure (.value vector env)
                  | none => Spec.bottom
  | _, _, .concat left right, env =>
      Flow.bind (left.denoteIn meanings env) fun l env =>
        Flow.bind (right.denoteIn meanings env) fun r env =>
          match SpecVector.ofArray? (l.values ++ r.values) with
          | some vector => Spec.pure (.value vector env)
          | none => Spec.bottom
  | _, _, .slice site vector start stop, env =>
      Flow.bind (vector.denoteIn meanings env) fun values env =>
        Flow.bind (start.denoteIn meanings env) fun a env =>
          Flow.bind (stop.denoteIn meanings env) fun b env =>
            if a.val < 0 ∨ b.val < a.val ∨ values.values.size < b.val.toNat then
              abortAt site (Failure.ofInts .abort #[a.val, b.val])
            else match SpecVector.ofArray? (values.values.extract a.val.toNat b.val.toNat) with
              | some vector => Spec.pure (.value vector env)
              | none => Spec.bottom
  | _, _, .reverseSlice site vector start stop, env =>
      Flow.bind (vector.denoteIn meanings env) fun values env =>
        Flow.bind (start.denoteIn meanings env) fun a env =>
          Flow.bind (stop.denoteIn meanings env) fun b env =>
            if a.val < 0 ∨ b.val < a.val ∨ values.values.size < b.val.toNat then
              abortAt site (Failure.ofInts .abort #[a.val, b.val])
            else match SpecVector.ofArray? (reverseSlice values.values a.val.toNat b.val.toNat) with
              | some vector => Spec.pure (.value vector env)
              | none => Spec.bottom
  | _, _, .destroyEmpty site vector, env =>
      Flow.bind (vector.denoteIn meanings env) fun values env =>
        if values.values.isEmpty then Spec.pure (.value () env) else abortAt site (Failure.ofInts .abort #[])
  | _, _, @Term.contains _ _ _ τ vector needle, env =>
      Flow.bind (vector.denoteIn meanings env) fun values env =>
        Flow.bind (needle.denoteIn meanings env) fun n env =>
          Spec.pure (.value
            (findIndex? (NTy.eqb τ) values.values n values.values.size 0).isSome env)
  | _, _, @Term.indexOf _ _ _ τ vector needle, env =>
      Flow.bind (vector.denoteIn meanings env) fun values env =>
        Flow.bind (needle.denoteIn meanings env) fun n env =>
          Spec.pure (.value
            (((findIndex? (NTy.eqb τ) values.values n values.values.size 0).isSome,
              (foundIndexOf (NTy.eqb τ) values n, ())) :
              HList (.cons .bool (.cons (.int 64 false) .nil))) env)
  | _, _, @Term.order _ _ _ τ orders left right, env =>
      Flow.bind (left.denoteIn meanings env) fun a env =>
        Flow.bind (right.denoteIn meanings env) fun b env =>
          Spec.pure (.value (NTy.compareResult orders τ a b) env)
  | _, _, .seqAfter value effect, env =>
      Flow.bind (value.denoteIn meanings env) fun v env =>
        Flow.bind (effect.denoteIn meanings env) fun _ env => Spec.pure (.value v env)
  | _, _, .assertion site, env => assertAt site env
  | _, _, .anchor site, env => anchorAt site env
  | _, _, .assume site, env => assumeAt site (assumptionOf (meanings.assumption site) env) env
  | _, _, .arbitrary, env => arbitraryIn env
  | _, _, .ghostValue site, env => ghostAt site (ghostValueOf (meanings.ghost site) env) env
  | _, _, .ghostSlot value, env => ghostSlotted (value.denoteIn meanings env)
  | _, _, .ghostRead slot, env => ghostContents (slot.denoteIn meanings env)
  | _, _, .mutationEnd site, env => mutationEndAt site env
  | _, _, .memoryWritten site, env => memoryWrittenAt site env
  | _, _, .constructed site value, env =>
      Flow.bind (value.denoteIn meanings env) fun v env => constructedAt site v env
  | _, τ, @Term.globalRead _ _ _ κ _ site arguments key, env =>
      Flow.bind (key.denoteIn meanings env) fun k env =>
        Spec.bind Spec.get fun memory =>
          match meanings.read τ arguments (κ.storageKey k) memory with
          | none => abortAt site (Failure.ofInts .abort #[])
          | some value => Spec.pure (.value value env)
  | _, _, @Term.globalContains _ _ _ κ τ arguments key, env =>
      Flow.bind (key.denoteIn meanings env) fun k env =>
        Spec.bind Spec.get fun memory =>
          Spec.pure (.value (meanings.read τ arguments (κ.storageKey k) memory).isSome env)
  | _, .ref τ, @Term.globalBorrow _ _ _ κ _ site arguments key, env =>
      Flow.bind (key.denoteIn meanings env) fun k env =>
      Flow.bind (anchorAt site env) fun _ env =>
        Spec.bind Spec.get fun memory =>
          match meanings.read τ arguments (κ.storageKey k) memory with
          | none => abortAt site (Failure.ofInts .abort #[])
          | some value =>
              Spec.bind Spec.choose fun prophecy =>
                Spec.bind (Spec.set (meanings.write τ arguments (κ.storageKey k) (some prophecy) memory))
                  fun _ => Spec.pure
                    (.value (NTy.forget τ value, prophecy) env)
  | _, _, @Term.globalPublish _ _ _ κ τ site arguments key value, env =>
      Flow.bind (key.denoteIn meanings env) fun k env =>
        Flow.bind (value.denoteIn meanings env) fun v env =>
        Flow.bind (anchorAt site env) fun _ env =>
          Spec.bind Spec.get fun memory =>
            match meanings.read τ arguments (κ.storageKey k) memory with
            | some _ => abortAt site (Failure.ofInts .abort #[])
            | none =>
                Spec.bind (Spec.set (meanings.write τ arguments (κ.storageKey k) (some v) memory))
                  fun _ => Spec.pure (.value () env)
  | _, _, @Term.ghostStore _ _ _ κ τ site arguments key value, env =>
      Flow.bind (key.denoteIn meanings env) fun k env =>
        Flow.bind (value.denoteIn meanings env) fun v env =>
        Flow.bind (anchorAt site env) fun _ env =>
          Spec.bind Spec.get fun memory =>
            Spec.bind (Spec.set (meanings.write τ arguments (κ.storageKey k) (some v) memory))
              fun _ => Spec.pure (.value () env)
  | _, τ, @Term.globalTake _ _ _ κ _ site arguments key, env =>
      Flow.bind (key.denoteIn meanings env) fun k env =>
      Flow.bind (anchorAt site env) fun _ env =>
        Spec.bind Spec.get fun memory =>
          match meanings.read τ arguments (κ.storageKey k) memory with
          | none => abortAt site (Failure.ofInts .abort #[])
          | some value =>
              Spec.bind (Spec.set (meanings.write τ arguments (κ.storageKey k) none memory))
                fun _ => Spec.pure (.value value env)

/-- The denotation of an argument row: its values with the locals after it. -/
noncomputable def Args.denoteIn {unit : Validation.ValidatedUnit} [C : Carriers] {State : Type}
    (meanings : InterpreterOps unit C State) {ρ : ResultShape} : {Γ σs : NRow} → Args unit ρ Γ σs →
    HEnv Γ → Spec State Failure (Flow ρ Γ (HList σs))
  | _, _, .nil, env => Spec.pure (.value () env)
  | _, _, .cons head tail, env =>
      Flow.bind (head.denoteIn meanings env) fun value env =>
        Flow.bind (tail.denoteIn meanings env) fun values env =>
          Spec.pure (.value (value, values) env)

/-- The denotation of match arms at a value: the first arm that matches and
whose guard holds. A guard that fails continues with the locals and memory
before the arm, as the runtime's does. Where no arm is taken, the profile's
mismatch throw is the outcome, and without one there is none. -/
noncomputable def Arms.denoteIn {unit : Validation.ValidatedUnit} [C : Carriers] {State : Type}
    (meanings : InterpreterOps unit C State) {ρ : ResultShape} : {Γ : NRow} → {σ τ : NTy} →
    Arms unit ρ Γ σ τ → σ.carrier → HEnv Γ → Spec State Failure (Flow ρ Γ τ.carrier)
  | _, _, _, .nil (some failure), _, _ => Spec.abort failure
  | _, _, _, .nil none, _, _ => Spec.bottom
  | _, _, _, .cons arm pattern body rest, value, env =>
      match pattern.bindValue value env with
      | some bound => armIn arm (body.denoteIn meanings bound)
      | none => rest.denoteIn meanings value env
  | _, _, _, .guarded arm pattern guard body rest, value, env =>
      match pattern.bindValue value env with
      | some bound =>
          Spec.bind Spec.get fun memory =>
            Flow.bind (guard.denoteIn meanings bound) fun holds guarded =>
              if holds then armIn arm (body.denoteIn meanings guarded)
              else Spec.bind (Spec.set memory) fun _ => rest.denoteIn meanings value env
      | none => rest.denoteIn meanings value env
end

/-- A named projection evaluates its owner at the owner's original native
index. Opening the structural `Term.field` first would use the expanded layout
as that index in the recursive denotation. -/
theorem Term.denoteIn_fieldAt {unit : Validation.ValidatedUnit} [C : Carriers] {State : Type}
    (meanings : InterpreterOps unit C State) {ρ : ResultShape} {Γ : NRow}
    (owner : NTy) {source : StructHandle} {arguments fields : NRow} {field : NTy}
    (layout : owner = .struct source arguments fields) (position : Var fields field)
    (value : Term unit ρ Γ owner) (env : HEnv Γ) :
    (Term.fieldAt owner layout position value).denoteIn meanings env =
      Flow.bind (value.denoteIn meanings env) fun v env =>
        Spec.pure (.value (position.selectCertified (layout ▸ v)) env) := by
  cases layout
  rfl

attribute [vir_denote ↓] Term.denoteIn_fieldAt

/-- A named construction certifies its row at the original nominal type. -/
theorem Term.denoteIn_packAt {unit : Validation.ValidatedUnit} [C : Carriers] {State : Type}
    (meanings : InterpreterOps unit C State) {ρ : ResultShape} {Γ : NRow}
    (owner : NTy) {source : StructHandle} {arguments fields : NRow}
    (layout : owner = .struct source arguments fields) (site : Nat)
    (values : Args unit ρ Γ fields) (ghosts : GhostInits fields) (env : HEnv Γ) :
    (Term.packAt owner layout site values ghosts).denoteIn meanings env =
      Flow.bind (values.denoteIn meanings env) fun values env =>
        Spec.bind (ghosts.initialize meanings values) fun row =>
          Spec.bind (certifyingAt site .construction owner
              (_root_.cast (congrArg NTy.plain layout.symm) (HList.forget fields row))) fun packed =>
            Spec.pure (.value packed env) := by
  cases layout
  rfl

/-- Destructuring evaluates its source at the original nominal type. -/
theorem Term.denoteIn_letFieldsAt {unit : Validation.ValidatedUnit} [C : Carriers] {State : Type}
    (meanings : InterpreterOps unit C State) {ρ : ResultShape} {Γ : NRow}
    (owner : NTy) {source : StructHandle} {arguments fields : NRow} {result : NTy}
    (layout : owner = .struct source arguments fields) (targets : Vars Γ fields)
    (value : Term unit ρ Γ owner) (body : Term unit ρ Γ result) (env : HEnv Γ) :
    (Term.letFieldsAt owner layout targets value body).denoteIn meanings env =
      Flow.bind (value.denoteIn meanings env) fun values env =>
        body.denoteIn meanings
          (targets.set (HListPlain.ofDeepHolds fields (layout ▸ values).val
            (NTy.fieldsHold (layout ▸ values))) env) := by
  cases layout
  rfl

attribute [vir_denote ↓] Term.denoteIn_packAt Term.denoteIn_letFieldsAt

theorem Term.denoteIn_variantAt {unit : Validation.ValidatedUnit} [C : Carriers] {State : Type}
    (meanings : InterpreterOps unit C State) {ρ : ResultShape} {Γ : NRow}
    (owner : NTy) {source : StructHandle} {arguments fields : NRow}
    {names : List String} {rows : NRows} {distinct : names.Nodup}
    (layout : owner = .enum source arguments names rows distinct) (site : Nat)
    (choice : Which names rows fields) (values : Args unit ρ Γ fields)
    (ghosts : GhostInits fields) (env : HEnv Γ) :
    (Term.variantAt owner layout site choice values ghosts).denoteIn meanings env =
      Flow.bind (values.denoteIn meanings env) fun values env =>
        Spec.bind (ghosts.initialize meanings values) fun row =>
          Spec.bind (certifyingAt site .construction owner
              (_root_.cast (congrArg NTy.plain layout.symm)
                (choice.inject (HList.forget fields row)))) fun packed =>
            Spec.pure (.value packed env) := by
  cases layout
  rfl

theorem Term.denoteIn_isVariantAt {unit : Validation.ValidatedUnit} [C : Carriers] {State : Type}
    (meanings : InterpreterOps unit C State) {ρ : ResultShape} {Γ : NRow}
    (owner : NTy) {source : StructHandle} {arguments : NRow}
    {names : List String} {rows : NRows} {distinct : names.Nodup}
    (layout : owner = .enum source arguments names rows distinct) (tests : List String)
    (value : Term unit ρ Γ owner) (env : HEnv Γ) :
    (Term.isVariantAt owner layout tests value).denoteIn meanings env =
      Flow.bind (value.denoteIn meanings env) fun v env =>
        Spec.pure (.value (namedIn (variantPlainName names rows (layout ▸ v).val) tests) env) := by
  cases layout
  rfl

theorem Term.denoteIn_payloadAt {unit : Validation.ValidatedUnit} [C : Carriers] {State : Type}
    (meanings : InterpreterOps unit C State) {ρ : ResultShape} {Γ : NRow}
    (owner : NTy) {source : StructHandle} {arguments : NRow}
    {names : List String} {rows : NRows} {distinct : names.Nodup} {result : NTy}
    (layout : owner = .enum source arguments names rows distinct) (mismatch : Option Failure)
    (choices : Choices names rows result) (value : Term unit ρ Γ owner) (env : HEnv Γ) :
    (Term.payloadAt owner layout mismatch choices value).denoteIn meanings env =
      Flow.bind (value.denoteIn meanings env) fun v env =>
        match choices.selectCertified? (layout ▸ v) with
        | some selected => Spec.pure (.value selected env)
        | none => mismatchOutcome mismatch := by
  cases layout
  rfl

attribute [vir_denote ↓] Term.denoteIn_variantAt Term.denoteIn_isVariantAt Term.denoteIn_payloadAt

/-- Concrete entry point for the shared interpreter. -/
noncomputable def Term.denote {unit : Validation.ValidatedUnit} [FunctionCarriers unit]
    {executable : Validation.ExecutableUnit unit} [Θ : Skolems unit]
    (meanings : Meanings executable) {ρ : ResultShape} {Γ : NRow} {τ : NTy}
    (term : Term unit ρ Γ τ) (env : HEnv Γ) : Comp unit (Flow ρ Γ τ.carrier) :=
  term.denoteIn meanings.interpreterOps env

noncomputable def Args.denote {unit : Validation.ValidatedUnit} [FunctionCarriers unit]
    {executable : Validation.ExecutableUnit unit} [Θ : Skolems unit]
    (meanings : Meanings executable) {ρ : ResultShape} {Γ σs : NRow}
    (args : Args unit ρ Γ σs) (env : HEnv Γ) : Comp unit (Flow ρ Γ (HList σs)) :=
  args.denoteIn meanings.interpreterOps env

noncomputable def Arms.denote {unit : Validation.ValidatedUnit} [FunctionCarriers unit]
    {executable : Validation.ExecutableUnit unit} [Θ : Skolems unit]
    (meanings : Meanings executable) {ρ : ResultShape} {Γ : NRow} {σ τ : NTy}
    (arms : Arms unit ρ Γ σ τ) (value : σ.carrier) (env : HEnv Γ) : Comp unit (Flow ρ Γ τ.carrier) :=
  arms.denoteIn meanings.interpreterOps value env

section BorrowingEquivalences

variable {executable : Validation.ExecutableUnit unit} (meanings : Meanings executable)
  {ρ : ResultShape} {Γ : NRow} {τ : NTy}

/-- An entry assumption for a type with no certificates is a pure step.
The compiler omits it instead of asking normalization to remove it. -/
theorem Term.denote_assumeCurrent_certificateFree (free : τ.certificateFree = true)
    (x : Var Γ (.ref τ)) (env : HEnv Γ) :
    (Term.assumeCurrent (ρ := ρ) x).denote meanings env = Spec.pure (.value () env) := by
  simp only [Term.denote, Term.denoteIn]
  cases h : x.get env with
  | none => rfl
  | some reference =>
      have held := NTy.deepHolds_of_certificateFree τ free reference.1
      have h : (Spec.assume (NTy.deepHolds τ reference.1) : Comp unit Unit) = Spec.pure () := by
        apply Spec.extensionality
        · funext x y z; simp [Spec.assume, Spec.pure, held]
        · funext x y; rfl
        · funext x; rfl
      simp only [h, Spec.pure_bind]

/-- Passing a reference whose referent has no certificates adds no check. -/
theorem Term.denote_handedOver_certificateFree (free : τ.certificateFree = true)
    (site : Nat) (reference : Term unit ρ Γ (.ref τ)) (env : HEnv Γ) :
    (Term.handedOver site reference).denote meanings env = reference.denote meanings env := by
  simp only [Term.denote, Term.denoteIn]
  have cert (value : τ.plain) : certifyingAt (State := Memory unit) site .handOver τ value =
      Spec.pure (NTy.ofDeepHolds τ value (NTy.deepHolds_of_certificateFree τ free value)) :=
    certifying_eq_pure (NTy.deepHolds_of_certificateFree τ free value) (fun _ => rfl)
  simp only [cert, Spec.pure_bind, Flow.bind]
  apply Eq.trans (b := Spec.bind (reference.denote meanings env) Spec.pure)
  · apply congrArg (Spec.bind (reference.denote meanings env))
    funext flow
    cases flow <;> rfl
  · exact Spec.bind_pure _

/-- The single prophecy of a whole-local borrow is equivalent to the general
path rule's two prophecies constrained to have the same plain value. -/
theorem Term.denote_borrowPlace_nil_expanded (mismatch : Option Failure) (x : Var Γ τ) (env : HEnv Γ) :
    (Term.borrowPlace (ρ := ρ) mismatch x .nil).denote meanings env =
      match x.get env with
      | some value =>
          Spec.bind (prophecyOf (α := τ.carrier × τ.carrier) x (.nil : Proj τ τ)) fun chosen =>
            Spec.bind (Spec.assume (NTy.forget τ chosen.1 = NTy.forget τ chosen.2)) fun _ =>
              Spec.pure (.value (NTy.forget τ value, chosen.1) (x.set chosen.2 env))
      | none => Spec.bottom := by
  simp only [Term.denote, Term.denoteIn]
  cases h : x.get env with
  | none => rfl
  | some value =>
      apply Eq.symm
      apply Spec.extensionality
      · funext initial result final
        apply propext
        simp only [Spec.bind, prophecyOf, Spec.choose, Spec.assume, Spec.pure]
        constructor
        · rintro ⟨⟨chosen, owner⟩, middle, first, unused, middle', ⟨same, _, step⟩, output, last⟩
          have eq : chosen = owner := NTy.forget_injective τ same
          subst owner
          exact ⟨chosen, middle, first, output, step ▸ last⟩
        · rintro ⟨chosen, middle, first, output, last⟩
          exact ⟨(chosen, chosen), middle, first, (), middle, ⟨rfl, trivial, rfl⟩, output, last⟩
      · funext initial failure
        simp [Spec.bind, prophecyOf, Spec.choose, Spec.assume, Spec.pure]
      · funext initial
        simp [Spec.bind, prophecyOf, Spec.choose, Spec.assume, Spec.pure]

end BorrowingEquivalences

/-- A compiled function: its parameter and remaining local types, the
complete local row they spell as a literal, its declared result shape, and
its body over that row. -/
structure Function (unit : Validation.ValidatedUnit) where
  params : NRow
  locals : NRow
  frame : NRow
  frame_eq : params ++ locals = frame
  result : ResultShape
  body : Term unit result frame result.bodyType
  /-- The mutable-reference parameters, resolved at exit. -/
  mutables : Mutables frame
  /-- Where a mutable parameter's loan ends: the function's exit, which is
  the site its data invariant is reported at
  (`doc/designs/data-invariants.md`). -/
  exitSite : Nat
  /-- Scoped dictionary parameters, checked before evaluating the body. -/
  scopedMethods : List Int := []

@[simp, vir_denote_norm] theorem scopedValues_nil
    (unit : VerexIR.Validation.ValidatedUnit) (arguments : Array VerexIR.RuntimeValue) :
    VerexIR.scopedValues unit [] arguments = true := rfl

/-- The function result a body outcome denotes, after the mutable-reference
parameters it still holds are resolved.  Loop control that escapes the body
has no meaning, as the runtime's `finishControl?` has none. -/
noncomputable def ResultShape.finish [Carriers] {State : Type} {Γ : NRow} (site : Nat) (mutables : Mutables Γ)
    (shape : ResultShape) :
    Flow shape Γ shape.bodyType.carrier → Spec State Failure shape.carrier
  | .value value env => mutables.resolve site env (Spec.pure (shape.ofBody value))
  | .return_ result env => mutables.resolve site env (Spec.pure result)
  | .break_ _ _ => Spec.bottom
  | .continue_ _ _ => Spec.bottom

/-- A compiled function interpreted in a given carrier family and state.
Its scoped dictionary parameters are checked at entry, as the runtime checks
them, so that every family's entry is the one semantics. -/
noncomputable def Function.denoteIn [C : Carriers] {State : Type}
    (operations : InterpreterOps unit C State) (f : Function unit) (args : HList f.params) :
    Spec State Failure f.result.carrier :=
  let body := Spec.bind (f.body.denoteIn operations (initialEnv f.params f.locals f.frame_eq args))
    (f.result.finish f.exitSite f.mutables)
  match f.scopedMethods with
  | [] => body
  | _ :: _ =>
      if scopedValues unit f.scopedMethods (HList.encode args).toArray then body else Spec.stuck

/-- The denotation of a function on native arguments, with calls meaning
what `meaning` says. -/
noncomputable def Function.denoteWith {unit : VerexIR.Validation.ValidatedUnit} [FunctionCarriers unit]
    {executable : VerexIR.Validation.ExecutableUnit unit} [Θ : Skolems unit]
    (meanings : Meanings executable) (f : Function unit) (args : HList f.params) :
    Comp unit f.result.carrier :=
  let body := Spec.bind (f.body.denote meanings (initialEnv f.params f.locals f.frame_eq args))
    (f.result.finish f.exitSite f.mutables)
  match f.scopedMethods with
  | [] => body
  | _ :: _ =>
      if scopedValues unit f.scopedMethods (HList.encode args).toArray then body else Spec.stuck

/-- The denotation of a function on native arguments, every call meaning
its callee's prophetic meaning.  Stated directly: every transport unifies
against it. -/
noncomputable def Function.denote {unit : VerexIR.Validation.ValidatedUnit} [FunctionCarriers unit]
    (executable : VerexIR.Validation.ExecutableUnit unit) [Θ : Skolems unit]
    (typeInstantiation : Array (TypeId × TypeId)) (f : Function unit) (args : HList f.params) :
    Comp unit f.result.carrier :=
  let body := Spec.bind (f.body.denote (closedMeanings executable typeInstantiation)
    (initialEnv f.params f.locals f.frame_eq args)) (f.result.finish f.exitSite f.mutables)
  match f.scopedMethods with
  | [] => body
  | _ :: _ =>
      if scopedValues unit f.scopedMethods (HList.encode args).toArray then body else Spec.stuck

/-- The meaning of calls routing one function to `self`: its handle at its
own signature means `self`, every other call `rest`. -/
def routeMeaning (handle : FunctionHandle) (π : NRow) (ρ : ResultShape)
    (self : HList π → Comp unit ρ.carrier) (rest : CalleeMeaning) : CalleeMeaning :=
  fun callee σs shape args =>
    if routed : callee = handle ∧ σs = π ∧ shape = ρ then
      routed.2.2 ▸ self (routed.2.1 ▸ args)
    else rest callee σs shape args

@[simp] theorem routeMeaning_self (handle : FunctionHandle) (π : NRow) (ρ : ResultShape)
    (self : HList π → Comp unit ρ.carrier) (rest : CalleeMeaning) (args : HList π) :
    routeMeaning handle π ρ self rest handle π ρ args = self args := by
  simp [routeMeaning]

theorem routeMeaning_other {handle callee : FunctionHandle} {π σs : NRow} {ρ shape : ResultShape}
    (self : HList π → Comp unit ρ.carrier) (rest : CalleeMeaning) (args : HList σs)
    (other : callee ≠ handle) :
    routeMeaning handle π ρ self rest callee σs shape args = rest callee σs shape args := by
  simp [routeMeaning, other]

/-- A position among the members of a cycle of calls, each a handle with
its compiled function. -/
inductive CycleIndex : List (FunctionHandle × Function unit) → Type where
  | here {member : FunctionHandle × Function unit} {others : List (FunctionHandle × Function unit)} :
      CycleIndex (member :: others)
  | there {member : FunctionHandle × Function unit} {others : List (FunctionHandle × Function unit)} :
      CycleIndex others → CycleIndex (member :: others)

/-- The member a position names. -/
def CycleIndex.member : {members : List (FunctionHandle × Function unit)} → CycleIndex members →
    FunctionHandle × Function unit
  | member :: _, .here => member
  | _ :: _, .there later => later.member

/-- A function's meaning at every skolem family and type instantiation, so
that a call to it with type arguments means it at the family and frame
instantiation the call induces. -/
abbrev SelfFamily (unit : Validation.ValidatedUnit) [FunctionCarriers unit]
    (π : NRow) (ρ : ResultShape) : Type 1 :=
  (Θ : Skolems unit) → Array (TypeId × TypeId) → @HList Θ.toCarriers π → Comp unit (@ResultShape.carrier Θ.toCarriers ρ)

omit [Skolems unit] in
/-- A slot of a cycle's fixed point: a member at the runtime family
(`none`), or at a skolem family and a type instantiation. -/
structure CycleSlot (unit : Validation.ValidatedUnit) [FunctionCarriers unit]
    (members : List (FunctionHandle × Function unit)) where
  position : CycleIndex members
  family : Option (Skolems unit × Array (TypeId × TypeId))

omit [Skolems unit] in
/-- The skolem family a slot's family names: the ambient one at the runtime
family. -/
@[reducible] def familySkolems [Θ : Skolems unit] : Option (Skolems unit × Array (TypeId × TypeId)) → Skolems unit
  | none => Θ
  | some (family, _) => family

omit [Skolems unit] in
/-- The type instantiation a slot's family names: the frame's own at the
runtime family. -/
def familyInstantiation (typeInstantiation : Array (TypeId × TypeId)) :
    Option (Skolems unit × Array (TypeId × TypeId)) → Array (TypeId × TypeId)
  | none => typeInstantiation
  | some (_, instantiation) => instantiation

omit [Skolems unit] in
/-- The meanings standing for a cycle's members, one per slot. -/
abbrev CycleFamilySelves [Θ : Skolems unit]
    (members : List (FunctionHandle × Function unit)) : Type 1 :=
  (slot : CycleSlot unit members) → @HList (familySkolems slot.family).toCarriers slot.position.member.2.params →
    Comp unit (@ResultShape.carrier (familySkolems slot.family).toCarriers slot.position.member.2.result)

omit [Skolems unit] in
/-- Route calls with type arguments to one function: its handle at its own
signature means `self` at the family the call induces and the callee's frame
instantiation, every other call `rest`. -/
noncomputable def routeGeneric [Θ : Skolems unit]
    (handle : FunctionHandle) (π : NRow) (ρ : ResultShape)
    (self : SelfFamily unit π ρ)
    (frame : FunctionHandle → Array GenericArgument → Array (TypeId × TypeId))
    (rest : GenericMeaning) : GenericMeaning :=
  fun callee typeArgs θ σs shape args =>
    if routed : callee = handle ∧ σs = π ∧ shape = ρ then
      routed.2.2 ▸ self (Skolems.instantiate θ Θ) (frame callee typeArgs) (routed.2.1 ▸ args)
    else rest callee typeArgs θ σs shape args

omit [Skolems unit] in
@[simp] theorem routeGeneric_self [Θ : Skolems unit]
    (handle : FunctionHandle) (π : NRow)
    (ρ : ResultShape) (self : SelfFamily unit π ρ)
    (frame : FunctionHandle → Array GenericArgument → Array (TypeId × TypeId)) (rest : GenericMeaning)
    (typeArgs : Array GenericArgument) (θ : TypeArgs) (args : @HList (Skolems.instantiate θ Θ).toCarriers π) :
    routeGeneric handle π ρ self frame rest handle typeArgs θ π ρ args =
      self (Skolems.instantiate θ Θ) (frame handle typeArgs) args := by
  simp [routeGeneric]

omit [Skolems unit] in
theorem routeGeneric_other [Θ : Skolems unit]
    {handle callee : FunctionHandle} {π σs : NRow}
    {ρ shape : ResultShape} (self : SelfFamily unit π ρ)
    (frame : FunctionHandle → Array GenericArgument → Array (TypeId × TypeId)) (rest : GenericMeaning)
    (typeArgs : Array GenericArgument) (θ : TypeArgs) (args : @HList (Skolems.instantiate θ Θ).toCarriers σs)
    (other : callee ≠ handle) :
    routeGeneric handle π ρ self frame rest callee typeArgs θ σs shape args =
      rest callee typeArgs θ σs shape args := by
  simp [routeGeneric, other]

omit [Skolems unit] in
/-- The meaning of calls without type arguments in a body at a slot's
family: each member at its own signature means its meaning at that family,
every other call `rest`. -/
def cycleCalls [Θ : Skolems unit] (family : Option (Skolems unit × Array (TypeId × TypeId)))
    (rest : @CalleeMeaning _ _ (familySkolems family)) :
    (members : List (FunctionHandle × Function unit)) → CycleFamilySelves members →
      @CalleeMeaning _ _ (familySkolems family)
  | [], _ => rest
  | member :: others, self =>
      @routeMeaning _ _ (familySkolems family) member.1 member.2.params member.2.result
        (self ⟨.here, family⟩)
        (cycleCalls family rest others fun slot => self ⟨.there slot.position, slot.family⟩)

omit [Skolems unit] in
/-- The meaning of calls with type arguments in a body at a family: each
member at its own signature means its meaning at the family and frame
instantiation the call induces, every other call `rest`. -/
noncomputable def cycleGenerics {unit : VerexIR.Validation.ValidatedUnit} [FunctionCarriers unit]
    (executable : VerexIR.Validation.ExecutableUnit unit) [Θ : Skolems unit]
    (root : Skolems unit) (instantiation : Array (TypeId × TypeId))
    (rest : @GenericMeaning _ _ Θ) :
    (members : List (FunctionHandle × Function unit)) → CycleFamilySelves (Θ := root) members →
      @GenericMeaning _ _ Θ
  | [], _ => rest
  | member :: others, self =>
      routeGeneric (Θ := Θ) member.1 member.2.params member.2.result
        (fun family instantiation => self ⟨.here, some (family, instantiation)⟩)
        (fun callee typeArgs => frameInstantiation unit callee instantiation typeArgs)
        (cycleGenerics executable (Θ := Θ) root instantiation rest others
          fun slot => self ⟨.there slot.position, slot.family⟩)

omit [Skolems unit] in
/-- The meaning of calls with type arguments in a body at a family: at the
runtime family every such call is closed, since a proof covers the runtime
slots only when no member is generic; at a skolem family each member is
routed as `cycleGenerics` does. -/
noncomputable def cycleGenericsAt {unit : VerexIR.Validation.ValidatedUnit} [FunctionCarriers unit]
    (executable : VerexIR.Validation.ExecutableUnit unit) [Θ : Skolems unit]
    (members : List (FunctionHandle × Function unit)) (typeInstantiation : Array (TypeId × TypeId))
    (self : CycleFamilySelves (Θ := Θ) members) :
    (family : Option (Skolems unit × Array (TypeId × TypeId))) →
      @GenericMeaning _ _ (familySkolems family)
  | none => closedGeneric executable typeInstantiation
  | some (family, instantiation) =>
      cycleGenerics executable (Θ := family) Θ instantiation (@closedGeneric _ _ executable family instantiation)
        members self

omit [Skolems unit] in
/-- What calls denote in the body of a cycle's member at a slot: a call to a
member its meaning at the slot the call reaches, every other call its
callee's prophetic meaning. -/
noncomputable def cycleMeanings {unit : VerexIR.Validation.ValidatedUnit} [FunctionCarriers unit]
    (executable : VerexIR.Validation.ExecutableUnit unit) [Θ : Skolems unit]
    (members : List (FunctionHandle × Function unit)) (typeInstantiation : Array (TypeId × TypeId))
    (self : CycleFamilySelves (Θ := Θ) members) (slot : CycleSlot unit members) :
    @Meanings _ _ executable (familySkolems slot.family) :=
  @Meanings.mk _ _ executable (familySkolems slot.family)
    (cycleCalls slot.family (@propheticMeaning _ _ executable (familySkolems slot.family) #[]) members self)
    (cycleGenericsAt executable members typeInstantiation self slot.family)
    (@closureMeaning _ _ executable (familySkolems slot.family))
    (familyInstantiation typeInstantiation slot.family) (fun _ => none) (fun _ => none)
    (fun _ => none)
    (@closedInstance _ _ executable (familySkolems slot.family)
      (familyInstantiation typeInstantiation slot.family))

end Runtime

section GenericRules
variable [Carriers] {State : Type}

omit [Carriers] in
/-- The weakest precondition through a verified callee: what its theorem
assumes and its precondition, its postcondition and frame under the
continuation, and its failures under the permitted failures. -/
theorem wp_call {Args Result : Type} {function : Args → Spec State Failure Result}
    {contract : Contract (State) Failure Args Result} {args : Args}
    {ensures : Result → State → Prop} {aborts : Failure → Prop} {initial : State}
    (verified : Satisfies function contract)
    (assumed : contract.assumes args initial)
    (permitted : contract.requires args initial)
    (post : ∀ result final,
      (¬contract.mayAbort args initial → contract.ensures args initial result final) →
      contract.frame args initial result final → ¬contract.mustAbort args initial →
      ensures result final)
    (failing : ∀ error, contract.aborts args initial error → aborts error) :
    wp (function args) ensures aborts initial :=
  ⟨fun result final execution =>
      let established := (verified args initial assumed permitted).1 result final execution
      post result final established.1 established.2.1 established.2.2,
    fun error execution =>
      failing error ((verified args initial assumed permitted).2.1 error execution),
    (verified args initial assumed permitted).2.2⟩

/-! ## Weakest preconditions of the flow combinators

A bind on a literal outcome reduces directly; only a symbolic outcome goes
through the weakest-precondition rule. -/

/-- A pure short-circuit guard can be kept as one proposition. Splitting its
two false paths first would verify the same continuation twice. The identical
environments and pure branches are essential: an effectful right operand or
a branch that changes locals must retain its short-circuit execution. -/
theorem Flow.pure_shortCircuit_and {ρ : ResultShape} {Γ : NRow}
    (p q : Prop) [Decidable p] [Decidable q] (env : HEnv Γ) :
    (if p then (Spec.pure (.value (decide q) env) : Spec State Failure (Flow ρ Γ Bool))
     else Spec.pure (.value false env)) =
      Spec.pure (.value (decide (p ∧ q)) env) := by
  by_cases hp : p <;> by_cases hq : q <;> simp [hp, hq]

/-- The corresponding sharing lemma for the two true paths of a pure `||`. -/
theorem Flow.pure_shortCircuit_or {ρ : ResultShape} {Γ : NRow}
    (p q : Prop) [Decidable p] [Decidable q] (env : HEnv Γ) :
    (if p then (Spec.pure (.value true env) : Spec State Failure (Flow ρ Γ Bool))
     else Spec.pure (.value (decide q) env)) =
      Spec.pure (.value (decide (p ∨ q)) env) := by
  by_cases hp : p <;> by_cases hq : q <;> simp [hp, hq]

@[simp] theorem Flow.bind_pure_value {ρ : ResultShape} {Γ : NRow} {α β : Type}
    (value : α) (env : HEnv Γ) (next : α → HEnv Γ → Spec State Failure (Flow ρ Γ β)) :
    Flow.bind (Spec.pure (.value value env)) next = next value env := by
  simp [Flow.bind]
@[simp] theorem Flow.bind_pure_return {ρ : ResultShape} {Γ : NRow} {α β : Type}
    (result : ρ.carrier) (env : HEnv Γ) (next : α → HEnv Γ → Spec State Failure (Flow ρ Γ β)) :
    Flow.bind (Spec.pure (.return_ result env)) next = Spec.pure (.return_ result env) := by
  simp [Flow.bind]
@[simp] theorem Flow.bind_pure_break {ρ : ResultShape} {Γ : NRow} {α β : Type}
    (nest : Nat) (env : HEnv Γ) (next : α → HEnv Γ → Spec State Failure (Flow ρ Γ β)) :
    Flow.bind (Spec.pure (.break_ nest env)) next = Spec.pure (.break_ nest env) := by
  simp [Flow.bind]
@[simp] theorem Flow.bind_pure_continue {ρ : ResultShape} {Γ : NRow} {α β : Type}
    (nest : Nat) (env : HEnv Γ) (next : α → HEnv Γ → Spec State Failure (Flow ρ Γ β)) :
    Flow.bind (Spec.pure (.continue_ nest env)) next = Spec.pure (.continue_ nest env) := by
  simp [Flow.bind]
@[simp] theorem Flow.bind_abort {ρ : ResultShape} {Γ : NRow} {α β : Type}
    (error : Failure) (next : α → HEnv Γ → Spec State Failure (Flow ρ Γ β)) :
    Flow.bind (Spec.abort error) next = Spec.abort error := by
  simp [Flow.bind]
@[simp] theorem Flow.bind_abortAt {ρ : ResultShape} {Γ : NRow} {α β : Type} (site : Nat)
    (error : Failure) (next : α → HEnv Γ → Spec State Failure (Flow ρ Γ β)) :
    Flow.bind (abortAt site error) next = abortAt site error := by
  simp [Flow.bind, abortAt]
omit [Carriers] in
/-- Binding after a marked failure stays that failure. -/
@[simp] theorem abortAt_bind {α β : Type} (site : Nat) (error : Failure)
    (next : α → Spec State Failure β) : Spec.bind (abortAt site error) next = abortAt site error := by
  simp [abortAt]

omit [Carriers] in
/-- A path aborting at a site owes the abort condition at its failure, marked
with the site. -/
@[simp] theorem wp_abortAt {α : Type} (site : Nat) (error : Failure)
    (ensures : α → State → Prop) (aborts : Failure → Prop) (initial : State) :
    wp (abortAt site error) ensures aborts initial ↔ AbortsAt site (aborts error) := by
  unfold abortAt AbortsAt
  exact wp_abort _ _ _ _

omit [Carriers] in
/-- An operation at a site owes the abort condition of each of its failures,
marked with the site. -/
@[simp] theorem wp_opAt {α : Type} (site : Nat) (meaning : Spec State Failure α)
    (ensures : α → State → Prop) (aborts : Failure → Prop) (initial : State) :
    wp (opAt site meaning) ensures aborts initial ↔
      wp meaning ensures (fun error => AbortsAt site (aborts error)) initial :=
  Iff.rfl

omit [Carriers] in
/-- An invocation taking a known arm owes what the arm does, marked with the
invocation's site and the arm's target. -/
@[simp] theorem wp_dispatchAt {α : Type} (site : Nat) (target : String) (meaning : Spec State Failure α)
    (ensures : α → State → Prop) (aborts : Failure → Prop) (initial : State) :
    wp (dispatchAt site target meaning) ensures aborts initial ↔
      Dispatched site target (wp meaning ensures aborts initial) :=
  Iff.rfl

omit [Carriers] in
/-- An operation a proof places facts after owes, on the paths continuing
after it, its postcondition marked with the site. -/
@[simp] theorem wp_opAfterAt {α : Type} (site : Nat) (meaning : Spec State Failure α)
    (ensures : α → State → Prop) (aborts : Failure → Prop) (initial : State) :
    wp (opAfterAt site meaning) ensures aborts initial ↔
      wp meaning (fun value state => AfterAt site (ensures value state))
        (fun error => AbortsAt site (aborts error)) initial :=
  Iff.rfl

omit [Carriers] in
/-- A path through a branch owes what the branch does, marked with the
branch. -/
@[simp] theorem wp_branchAt {α : Type} (site : Nat) (taken : Bool) (meaning : Spec State Failure α)
    (ensures : α → State → Prop) (aborts : Failure → Prop) (initial : State) :
    wp (branchAt site taken meaning) ensures aborts initial ↔
      Taken site taken (wp meaning ensures aborts initial) :=
  Iff.rfl

/-- A branch computing a value takes no path of its own: the conditional is
one of values (`&&`, `||`, an `if` of values), as the normal form merges it. -/
@[simp] theorem branchAt_value {ρ : ResultShape} {Γ : NRow} {α : Type} (site : Nat) (taken : Bool)
    (value : α) (env : HEnv Γ) :
    branchAt (State := State) site taken (Spec.pure (Flow.value value env : Flow ρ Γ α)) =
      (Spec.pure (Flow.value value env) : Spec State Failure (Flow ρ Γ α)) := rfl

omit [Carriers] in
/-- A path through an arm owes what the arm does, marked with the arm. -/
@[simp] theorem wp_armAt {α : Type} (site : Nat) (variant : String) (meaning : Spec State Failure α)
    (ensures : α → State → Prop) (aborts : Failure → Prop) (initial : State) :
    wp (armAt site variant meaning) ensures aborts initial ↔
      Matched site variant (wp meaning ensures aborts initial) :=
  Iff.rfl

omit [Carriers] in
@[simp] theorem branchIn_some {α : Type} (site : Nat) (taken : Bool) (meaning : Spec State Failure α) :
    branchIn (some site) taken meaning = branchAt site taken meaning := rfl

omit [Carriers] in
@[simp] theorem branchIn_none {α : Type} (taken : Bool) (meaning : Spec State Failure α) :
    branchIn none taken meaning = meaning := rfl

omit [Carriers] in
@[simp] theorem armIn_some {α : Type} (site : Nat) (variant : String) (meaning : Spec State Failure α) :
    armIn (some (site, variant)) meaning = armAt site variant meaning := rfl

omit [Carriers] in
@[simp] theorem armIn_none {α : Type} (meaning : Spec State Failure α) : armIn none meaning = meaning := rfl

/-- An arm computing a value takes no path of its own, as a branch computing
one does not (`branchAt_value`). -/
@[simp] theorem armAt_value {ρ : ResultShape} {Γ : NRow} {α : Type} (site : Nat) (variant : String)
    (value : α) (env : HEnv Γ) :
    armAt (State := State) site variant (Spec.pure (Flow.value value env : Flow ρ Γ α)) =
      (Spec.pure (Flow.value value env) : Spec State Failure (Flow ρ Γ α)) := rfl

omit [Carriers] in
@[simp] theorem Matched_true (site : Nat) (variant : String) : Matched site variant True = True :=
  rfl

omit [Carriers] in
/-- An abort condition that always holds owes nothing at its site. -/
@[simp] theorem AbortsAt_true (site : Nat) : AbortsAt site True = True := rfl
omit [Carriers] in
@[simp] theorem AfterAt_true (site : Nat) : AfterAt site True = True := rfl

omit [Carriers] in
/-- A branch owing nothing owes nothing on its path. -/
@[simp] theorem Taken_true (site : Nat) (taken : Bool) : Taken site taken True = True := rfl
omit [Carriers] in
@[simp] theorem Dispatched_true (site : Nat) (target : String) :
    Dispatched site target True = True := rfl

omit [Carriers] in
@[simp] theorem opAt_pure {α : Type} (site : Nat) (value : α) :
    opAt (State := State) site (Spec.pure value) = Spec.pure value := rfl

omit [Carriers] in
@[simp] theorem opAt_abort {α : Type} (site : Nat) (error : Failure) :
    opAt (State := State) (α := α) site (Spec.abort error) = abortAt site error := rfl


theorem wp_flowBind {ρ : ResultShape} {Γ : NRow} {α β : Type}
    (action : Spec State Failure (Flow ρ Γ α)) (next : α → HEnv Γ → Spec State Failure (Flow ρ Γ β))
    (ensures : Flow ρ Γ β → State → Prop) (aborts : Failure → Prop)
    (state : State) :
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

/-- Flows associate: an abrupt flow of the inner action passes both binds
unchanged, a value reaches the outer continuation through the inner one. -/
theorem Flow.bind_assoc {ρ : ResultShape} {Γ : NRow} {α β γ : Type}
    (action : Spec State Failure (Flow ρ Γ α)) (middle : α → HEnv Γ → Spec State Failure (Flow ρ Γ β))
    (next : β → HEnv Γ → Spec State Failure (Flow ρ Γ γ)) :
    Flow.bind (Flow.bind action middle) next =
      Flow.bind action fun value env => Flow.bind (middle value env) next := by
  simp only [Flow.bind, Spec.bind_assoc]
  congr 1
  funext flow
  cases flow <;> simp [Spec.pure_bind]

/-- The rule for a flow bound after a flow: the binds associate to the right,
so that the inner action heads the goal and the goal's continuation is not
copied into the inner bind's arms. -/
theorem wp_flowBind_flowBind {ρ : ResultShape} {Γ : NRow} {α β γ : Type}
    (action : Spec State Failure (Flow ρ Γ α)) (middle : α → HEnv Γ → Spec State Failure (Flow ρ Γ β))
    (next : β → HEnv Γ → Spec State Failure (Flow ρ Γ γ))
    (ensures : Flow ρ Γ γ → State → Prop) (aborts : Failure → Prop)
    (state : State) :
    wp (Flow.bind (Flow.bind action middle) next) ensures aborts state ↔
      wp (Flow.bind action fun value env => Flow.bind (middle value env) next) ensures aborts
        state := by
  rw [Flow.bind_assoc]

/-- A flow bound after a bind is bound inside the bind's continuation. -/
theorem Flow.bind_spec_bind {ρ : ResultShape} {Γ : NRow} {α β γ : Type}
    (first : Spec State Failure γ) (middle : γ → Spec State Failure (Flow ρ Γ α))
    (next : α → HEnv Γ → Spec State Failure (Flow ρ Γ β)) :
    Flow.bind (Spec.bind first middle) next = Spec.bind first fun value => Flow.bind (middle value) next := by
  simp only [Flow.bind, Spec.bind_assoc]

/-- The rule for a flow bound after a bind, as a call's denotation is: the
flow's continuation moves into the bind's, where it appears once — taken by
`wp_flowBind` instead, it would be copied into every flow's arm. -/
theorem wp_flowBind_specBind {ρ : ResultShape} {Γ : NRow} {α β γ : Type}
    (first : Spec State Failure γ) (middle : γ → Spec State Failure (Flow ρ Γ α))
    (next : α → HEnv Γ → Spec State Failure (Flow ρ Γ β))
    (ensures : Flow ρ Γ β → State → Prop) (aborts : Failure → Prop)
    (state : State) :
    wp (Flow.bind (Spec.bind first middle) next) ensures aborts state ↔
      wp first (fun value state => wp (Flow.bind (middle value) next) ensures aborts state)
        aborts state := by
  rw [Flow.bind_spec_bind, wp_bind]

/-- Loop verification from an invariant over the locals and the state: it
holds at entry, and one iteration under it is correct whenever the next
iteration is assumed correct under it.  Partial correctness. -/
theorem wp_loopAt {ρ : ResultShape} {Γ : NRow} (site : Nat)
    (iteration : (HEnv Γ → Spec State Failure (Flow ρ Γ Unit)) → HEnv Γ → Spec State Failure (Flow ρ Γ Unit))
    (entry : HEnv Γ) (invariant : HEnv Γ → State → Prop)
    (ensures : Flow ρ Γ Unit → State → Prop) (aborts : Failure → Prop)
    (initial : State)
    (entryHolds : invariant entry initial)
    (step : ∀ recursive : HEnv Γ → Spec State Failure (Flow ρ Γ Unit),
      (∀ env state, invariant env state → wp (recursive env) ensures aborts state) →
      ∀ env state, invariant env state → wp (iteration recursive env) ensures aborts state) :
    wp (loopAt site iteration entry) ensures aborts initial :=
  wp_withInvariant_fix (invariant := invariant) entryHolds step

/-- An in-body assertion as a cut: it holds where it is stated, and what
follows may assume it. -/
theorem wp_assertAt {ρ : ResultShape} {Γ : NRow} (site : Nat) (entry : HEnv Γ)
    (assertion : HEnv Γ → State → Prop)
    (ensures : Flow ρ Γ Unit → State → Prop) (aborts : Failure → Prop)
    (initial : State) (holds : assertion entry initial)
    (continues : assertion entry initial → ensures (.value () entry) initial) :
    wp (assertAt site entry) ensures aborts initial := by
  unfold assertAt
  exact (wp_pure _ _ _ _).mpr (continues holds)

/-- An in-body assertion verification does not check: no step. -/
theorem wp_assertAt_unchecked {ρ : ResultShape} {Γ : NRow} (site : Nat) (entry : HEnv Γ)
    (ensures : Flow ρ Γ Unit → State → Prop) (aborts : Failure → Prop)
    (initial : State) (continues : ensures (.value () entry) initial) :
    wp (assertAt site entry) ensures aborts initial := by
  unfold assertAt
  exact (wp_pure _ _ _ _).mpr continues

/-- An in-body assumption: what follows may assume its condition. -/
theorem wp_assumeAt {ρ : ResultShape} {Γ : NRow} (site : Nat) (holds : State → Prop)
    (entry : HEnv Γ) (ensures : Flow ρ Γ Unit → State → Prop) (aborts : Failure → Prop)
    (initial : State) (continues : holds initial → ensures (.value () entry) initial) :
    wp (assumeAt site holds entry) ensures aborts initial := by
  refine ⟨?_, fun _ failed => failed.elim, fun undefined => undefined⟩
  rintro _ _ ⟨assumed, rfl, rfl⟩
  exact continues assumed

/-- The end of a mutation as a cut: the invariant owed there holds, and
what follows may assume it. -/
theorem wp_mutationEndAt {ρ : ResultShape} {Γ : NRow} (site : Nat) (entry : HEnv Γ)
    (invariant : HEnv Γ → State → Prop)
    (ensures : Flow ρ Γ Unit → State → Prop) (aborts : Failure → Prop)
    (initial : State) (holds : invariant entry initial)
    (continues : invariant entry initial → ensures (.value () entry) initial) :
    wp (mutationEndAt site entry) ensures aborts initial := by
  unfold mutationEndAt
  exact (wp_pure _ _ _ _).mpr (continues holds)

/-- The end of a write of global memory as a cut: the invariants owed there
hold, and what follows may assume them. -/
theorem wp_memoryWrittenAt {ρ : ResultShape} {Γ : NRow} (site : Nat) (entry : HEnv Γ)
    (invariant : HEnv Γ → State → Prop)
    (ensures : Flow ρ Γ Unit → State → Prop) (aborts : Failure → Prop)
    (initial : State) (holds : invariant entry initial)
    (continues : invariant entry initial → ensures (.value () entry) initial) :
    wp (memoryWrittenAt site entry) ensures aborts initial := by
  unfold memoryWrittenAt
  exact (wp_pure _ _ _ _).mpr (continues holds)

omit [Carriers] in
/-- A call owing nothing after it is its meaning. -/
theorem wp_callAt {α : Type} (site : Nat) (meaning : Spec State Failure α) (ensures : α → State → Prop)
    (aborts : Failure → Prop) (initial : State)
    (runs : wp meaning ensures (fun error => AbortsAt site (aborts error)) initial) :
    wp (callAt site meaning) ensures aborts initial :=
  runs

/-- What holds where a call returns that owes invariants there: they hold,
and what follows may assume them. -/
def CallChecked (owed continues : Prop) : Prop :=
  owed ∧ (owed → continues)

omit [Carriers] in
theorem CallChecked.intro {owed continues : Prop} (holds : owed)
    (rest : owed → continues) : CallChecked owed continues :=
  ⟨holds, rest⟩

omit [Carriers] in
/-- A call as a cut where it returns: the invariants owed over the memory
before and after it hold, and what follows may assume them. -/
theorem wp_callAt_checked {α : Type} (site : Nat) (meaning : Spec State Failure α)
    (owed : State → State → Prop) (ensures : α → State → Prop)
    (aborts : Failure → Prop) (initial : State)
    (runs : wp meaning (fun result final => CallChecked (owed initial final)
      (ensures result final)) (fun error => AbortsAt site (aborts error)) initial) :
    wp (callAt site meaning) ensures aborts initial :=
  wp_mono runs (fun _ _ checked => checked.2 checked.1) (fun _ failed => failed)

/-- A construction as a cut: the invariant owed of the value holds, and what
follows may assume it. -/
theorem wp_constructedAt {ρ : ResultShape} {Γ : NRow} {α : Type} (site : Nat) (value : α)
    (entry : HEnv Γ) (invariant : α → State → Prop)
    (ensures : Flow ρ Γ α → State → Prop) (aborts : Failure → Prop)
    (initial : State) (holds : invariant value initial)
    (continues : invariant value initial → ensures (.value value entry) initial) :
    wp (constructedAt site value entry) ensures aborts initial := by
  unfold constructedAt
  exact (wp_pure _ _ _ _).mpr (continues holds)

/-- A ghost write as a cut: its value is defined — in the field type's
domain — and what follows takes the value the meanings give. -/
theorem wp_ghostAt {ρ : ResultShape} {Γ : NRow} {τ : NTy} (site : Nat)
    (chosen : State → Option τ.carrier) (entry : HEnv Γ)
    (ensures : Flow ρ Γ τ.carrier → State → Prop) (aborts : Failure → Prop)
    (initial : State) (defined : (chosen initial).isSome)
    (continues : ∀ value, chosen initial = some value → ensures (.value value entry) initial) :
    wp (ghostAt site chosen entry) ensures aborts initial := by
  unfold ghostAt
  rw [wp_bind, wp_certifyState]
  refine ⟨defined, ?_⟩
  rw [wp_bind_get, wp_bind, wp_choose]
  intro value
  rw [wp_bind, wp_assume]
  intro chosenValue
  exact (wp_pure _ _ _ _).mpr (continues value chosenValue)

/-- A state anchor: no step. -/
theorem wp_anchorAt {ρ : ResultShape} {Γ : NRow} (site : Nat) (entry : HEnv Γ)
    (ensures : Flow ρ Γ Unit → State → Prop) (aborts : Failure → Prop)
    (initial : State) (continues : ensures (.value () entry) initial) :
    wp (anchorAt site entry) ensures aborts initial := by
  unfold anchorAt
  exact (wp_pure _ _ _ _).mpr continues

@[simp] theorem finish_value {Γ : NRow} (site : Nat) (mutables : Mutables Γ)
    (shape : ResultShape) (value : shape.bodyType.carrier) (env : HEnv Γ) :
    shape.finish (State := State) site mutables (.value value env) =
      mutables.resolve site env (Spec.pure (shape.ofBody value)) := rfl
@[simp] theorem finish_return {Γ : NRow} (site : Nat) (mutables : Mutables Γ)
    (shape : ResultShape) (result : shape.carrier) (env : HEnv Γ) :
    shape.finish (State := State) site mutables (.return_ result env) =
      mutables.resolve site env (Spec.pure result) := rfl
@[simp] theorem finish_break {Γ : NRow} (site : Nat) (mutables : Mutables Γ)
    (shape : ResultShape) (nest : Nat) (env : HEnv Γ) :
    shape.finish (State := State) site mutables (.break_ nest env) = Spec.bottom := rfl
@[simp] theorem finish_continue {Γ : NRow} (site : Nat) (mutables : Mutables Γ)
    (shape : ResultShape) (nest : Nat) (env : HEnv Γ) :
    shape.finish (State := State) site mutables (.continue_ nest env) = Spec.bottom := rfl

end GenericRules

variable {unit : Validation.ValidatedUnit} [FunctionCarriers unit] [Skolems unit]

-- A one-choice selection or update reads as a map over the projection, whose
-- equations the normalizer inverts into the variant value itself.
attribute [vir_denote high] Choices.select?_single Choices.update?_single
attribute [vir_denote high] Choices.selectCertified?_single_unit
  Choices.selectCertified?_single_bool Choices.selectCertified?_single_int
  Choices.selectCertified?_single_num Choices.selectCertified?_single_address
  Choices.selectCertified?_single_signer Choices.selectCertified?_single_string
  Choices.selectCertified?_single_bytes Choices.selectCertified?_single_function
attribute [vir_denote] Flow.pure_shortCircuit_and Flow.pure_shortCircuit_or
attribute [vir_denote] Which.project?_eq_some_iff
attribute [vir_denote, vir_denote_norm] Which.projectCertified?_eq_some_iff
attribute [vir_denote, vir_denote_norm] Which.projectCertified?_eq_none_iff

-- The denotation rewrites by its equations; unfolding remains the fallback
-- where an equation's type indices do not match the term's at reducible
-- transparency, as a call's result type does.
vir_denote_equations Term.denoteIn Args.denoteIn Arms.denoteIn
attribute [vir_denote] Term.denote Args.denote Arms.denote Term.denoteIn Args.denoteIn Arms.denoteIn
  Meanings.interpreterOps Function.denote Function.denoteIn ResultShape.ofBody NTy.ofGround
  closedGeneric closedInstance
  HList.ofGround
  ResultShape.toBody
  ResultShape.finish Flow.iterate Flow.rebase Flow.bind_pure_value Flow.bind_pure_return
  Flow.bind_pure_break Flow.bind_pure_continue Flow.bind_abort Flow.bind_abortAt abortAt_bind
  wp_abortAt wp_opAt opAt_pure opAt_abort wp_branchAt branchIn_some branchIn_none
  AbortsAt_true Taken_true
  wp_armAt armIn_some armIn_none Matched_true wp_dispatchAt Dispatched_true
  finish_value finish_return finish_break
  finish_continue Mutables.resolve_nil Mutables.resolve_cons Mutables.resolve_at
  Mutables.resolve_sliceAt Option.bind_some Option.bind_none
  Proj.get?_nil Proj.get?_deref Proj.get?_field Proj.get?_element Proj.get?_index
  Proj.get?_arrayIndex Proj.get?_sliceIndex Proj.get?_sliceElement Proj.get?_variant
  Proj.get?_ghost
  Proj.getCertified?_nil Proj.getCertified?_deref Proj.getCertified?_field
  Proj.getCertified?_element Proj.getCertified?_index Proj.getCertified?_arrayIndex
  Proj.getCertified?_sliceIndex Proj.getCertified?_sliceElement
  Proj.getCertified?_variant Proj.getCertified?_ghost
  Proj.set?_nil Proj.set?_deref Proj.set?_field Proj.set?_element Proj.set?_index
  Proj.set?_arrayIndex Proj.set?_sliceIndex Proj.set?_sliceElement Proj.set?_variant
  Proj.set?_ghost
  Proj.setOwner?_nil Proj.setOwner?_deref Proj.setOwner?_field Proj.setOwner?_index
  Proj.setOwner?_element Proj.setOwner?_arrayIndex Proj.setOwner?_sliceElement
  Proj.setOwner?_sliceIndex Proj.setOwner?_variant Proj.setOwner?_ghost
  arbitraryIn ghostInitAt GhostInits.initialize Lender.resolve
  GhostInits.initialize_nil GhostInits.initialize_cons
  Lender.resolve_slot Lender.resolve_place Lender.resolve_candidate
  Lender.resolve_referent Lender.resolve_equation
  certifyingAt_unit certifyingAt_bool certifyingAt_int certifyingAt_num certifyingAt_address
  certifyingAt_signer certifyingAt_string certifyingAt_bytes certifyingAt_function
  borrowArraySliceAt borrowVectorSliceAt reborrowSliceAt takeSliceAtValue resolveSliceAtValue
  Choices.update?_nil Choices.update?_cons PlaceIndex.resolve?_literal
  PlaceIndex.resolve?_slot PlaceIndex.resolve?_fromEnd HEnv.slotInt?_int_zero HEnv.slotInt?_succ
  HList.toListOf_zero HList.toListOf_succ Var.update_here Var.update_there
  Var.updatePlain_here Var.updatePlain_there
  Var.clear_here Var.clear_there
  Which.inject_here Which.inject_there Which.project?_here_inl
  Which.project?_here_inr Which.project?_there_inl Which.project?_there_inr variantPlainName_inl
  variantPlainName_inr Choices.select?_nil Choices.select?_cons
  Choices.selectCertified?_nil Choices.selectCertified?_cons Vars.set_nil Vars.set_cons_none
  Vars.set_cons_some Pat.bindValue_wildcard Pat.bindValue_var Pat.bindValue_literal
  Pat.bindValue_range Pat.bindValue_tuple Pat.bindValue_struct Pat.bindValue_variant
  Pats.bindValues_nil Pats.bindValues_cons inRange mismatchOutcome_some mismatchOutcome_none
  Var.select_here Var.select_there Var.selectPlain_here Var.selectPlain_there
  NTy.eqb_tuple NTy.eqb_struct NTy.eqb_enum
  rowEqb_nil rowEqb_cons variantEqb_inl_inl variantEqb_inr_inr variantEqb_inl_inr
  variantEqb_inr_inl namedIn_nil namedIn_cons
  String.reduceBEq String.reduceEq String.reduceBNe String.reduceNe Bool.true_or Bool.false_or
  Bool.or_true Bool.or_false Bool.true_and Bool.false_and Bool.and_true Bool.and_false

-- Specifications read variants as the program does
-- (`doc/designs/native-verification.md`), so contracts normalize by the same
-- equations.
attribute [vir_denote_norm] Which.inject_here Which.inject_there Which.project?_here_inl
  Which.project?_here_inr Which.project?_there_inl Which.project?_there_inr variantPlainName_inl
  variantPlainName_inr Choices.select?_nil Choices.select?_cons Var.select_here Var.select_there
  Var.selectPlain_here Var.selectPlain_there namedIn_nil namedIn_cons
attribute [vir_denote_norm high] Choices.select?_single
attribute [vir_denote_norm high] Choices.selectCertified?_single_unit
  Choices.selectCertified?_single_bool Choices.selectCertified?_single_int
  Choices.selectCertified?_single_num Choices.selectCertified?_single_address
  Choices.selectCertified?_single_signer Choices.selectCertified?_single_string
  Choices.selectCertified?_single_bytes Choices.selectCertified?_single_function

/-- A readable rendering of a literal. -/
def describeValue : {τ : NTy} → τ.groundCarrier → String
  | .int _ _, value => toString value.val
  | .num, value => toString value.val
  | .bool, value => toString (value : Bool)
  | .unit, _ => "()"
  | .address, value => s!"@{(value : String)}"
  | .signer, value => s!"signer {(value : String)}"
  | .string, value => s!"{repr (value : String)}"
  | .bytes, value => s!"{repr (value : Array UInt8)}"
  | .char, value => s!"char {(value : SpecChar).val}"
  | .tuple _, _ => "(..)"
  | .struct _ _ _, _ => "{..}"
  | .enum _ _ _ _ _, _ => "variant{..}"
  | .vector _, value => s!"[{value.values.size} elements]"
  | .slice _, value => s!"slice[{value.values.size} elements]"
  | .array _ length, _ => s!"[{length} elements]"
  | .table _ _, value => s!"[{value.entries.length} entries]"
  | .ghost _, _ => "ghost(..)"
  | .ref _, _ => "&mut(..)"
  | .sliceRef _, value => s!"&mut slice[{value.extent}]"
  | .param index, _ => s!"T{index}"
  | .function _ _ _, value => value.elim
  | .nominal _ _ _, _ => "nominal(..)"
  | .observed _, _ => "observed(..)"

/-- A readable rendering of destructuring targets. -/
def Vars.describe {Γ : NRow} : {σs : NRow} → Vars Γ σs → String
  | _, .nil => ""
  | _, .cons target .nil => target.elim "_" fun x => s!"local{x.index}"
  | _, .cons target rest => s!"{target.elim "_" fun x => s!"local{x.index}"}, {rest.describe}"

mutual
/-- A readable rendering of a pattern. -/
partial def Pat.describe {Γ : NRow} : {τ : NTy} → Pat Γ τ → String
  | _, .wildcard => "_"
  | _, .var x => s!"local{x.index}"
  | _, .literal value _ => describeValue value
  | _, .range lower upper inclusive =>
      s!"{lower.elim "" toString}..{if inclusive then "=" else ""}{upper.elim "" toString}"
  | _, .tuple elements => s!"({elements.describe})"
  | _, .struct fields => s!"\{{fields.describe}}"
  | _, .variant choice fields => s!"{choice.name}\{{fields.describe}}"

partial def Pats.describe {Γ : NRow} : {σs : NRow} → Pats Γ σs → String
  | _, .nil => ""
  | _, .cons head .nil => head.describe
  | _, .cons head tail => s!"{head.describe}, {tail.describe}"
end

mutual
/-- A readable rendering of a term, for diagnostics. -/
partial def Term.describe {ρ : ResultShape} : {Γ : NRow} → {τ : NTy} → Term unit ρ Γ τ → String
  | _, _, .lit value _ => describeValue value
  | _, _, .var x => s!"local{x.index}"
  | _, _, .checked _ op failure left right =>
      s!"({left.describe} {repr op} {right.describe} ! {repr failure})"
  | _, _, .modular op left right => s!"({left.describe} {repr op} {right.describe} wrapping)"
  | _, _, .compare op left right => s!"({left.describe} {repr op} {right.describe})"
  | _, _, .equal negated left right =>
      s!"({left.describe} {if negated then "!=" else "=="} {right.describe})"
  | _, _, .equalErased negated left right =>
      s!"({left.describe} {if negated then "!=" else "=="}ᵍ {right.describe})"
  | _, _, .not operand => s!"!{operand.describe}"
  | _, _, .logical conjunction left right =>
      s!"({left.describe} {if conjunction then "&&" else "||"} {right.describe})"
  | _, _, .bitwise op left right => s!"({left.describe} {repr op} {right.describe})"
  | _, _, .shift _ left failure value distance =>
      s!"({value.describe} {if left then "<<" else ">>"} {distance.describe} ! {repr failure})"
  | _, _, .shiftSigned _ left failure value distance =>
      s!"({value.describe} {if left then "<<" else ">>"} {distance.describe} ! {repr failure})"
  | _, _, .shiftUnchecked _ left value distance =>
      s!"({value.describe} {if left then "<<" else ">>"} {distance.describe})"
  | _, _, .divideUnchecked _ modulo left right =>
      s!"({left.describe} {if modulo then "%" else "/"} {right.describe})"
  | _, _, .overflowing op left right =>
      s!"overflowing({left.describe} {repr op} {right.describe})"
  | _, _, @Term.wrap _ _ _ _ _ width signed _ value =>
      s!"({value.describe} as int {width} {signed})"
  | _, _, .charCode value => s!"({value.describe} as code)"
  | _, _, .charOf _ value => s!"({value.describe} as char)"
  | _, _, .charOfChecked _ failure value => s!"({value.describe} as char ! {repr failure})"
  | _, _, .arrayVector array => array.describe
  | _, _, .arrayOf vector => vector.describe
  | _, _, .repeatVector count value => s!"[{value.describe}; {count}]"
  | _, _, .stringLength value => s!"{value.describe}.length"
  | _, _, .unreachable => "unreachable"
  | _, _, @Term.cast _ _ _ _ _ width signed _ failure value =>
      s!"({value.describe} as int {width} {signed} ! {repr failure})"
  | _, _, .ite _ condition thenBranch elseBranch =>
      s!"(if {condition.describe} then {thenBranch.describe} else {elseBranch.describe})"
  | _, _, .let_ x value body => s!"(let local{x.index} := {value.describe}; {body.describe})"
  | _, _, .drop value body => s!"({value.describe}; {body.describe})"
  | _, _, .assign x value => s!"(local{x.index} := {value.describe})"
  | _, _, .const value => s!"const({value.describe})"
  | _, _, .throw0 _ kind => s!"throw {repr kind}"
  | _, _, .throw1 _ kind code => s!"throw {repr kind}({code.describe})"
  | _, _, .return_ value => s!"return {value.describe}"
  | _, _, .break_ nest => s!"break {nest}"
  | _, _, .continue_ nest => s!"continue {nest}"
  | _, _, .loop site body => s!"loop@{site} {body.describe}"
  | _, _, .call _ handle _ arguments =>
      s!"call {handle.namespaceId.index}:{handle.functionId.index}({arguments.describe})"
  | _, _, .closure handle weave _ _ _ _ _ _ captures =>
      s!"closure {handle.namespaceId.index}:{handle.functionId.index} mask {weave.mask}\
        ({captures.describe})"
  | _, _, .closureGeneric handle weave typeArgs _ _ _ _ _ _ _ _ captures =>
      s!"closure {handle.namespaceId.index}:{handle.functionId.index}<{typeArgs.size} types> \
        mask {weave.mask}({captures.describe})"
  | _, _, .invoke _ _ function arguments =>
      s!"invoke {function.describe}({arguments.describe})"
  | _, _, .callGeneric _ handle typeArgs _ _ _ _ arguments =>
      s!"call {handle.namespaceId.index}:{handle.functionId.index}<{typeArgs.size} types>\
        ({arguments.describe})"
  | _, _, .callInstance _ handle typeArgs _ _ arguments =>
      s!"call {handle.namespaceId.index}:{handle.functionId.index}<{typeArgs.size} types, \
        instance>({arguments.describe})"
  | _, _, .tuple elements => s!"({elements.describe})"
  | _, _, .pack _ source _ fields _ => s!"{repr source} \{{fields.describe}}"
  | _, _, .variant _ source _ _ choice fields _ =>
      s!"{repr source}::{choice.name} \{{fields.describe}}"
  | _, _, .field x value => s!"{value.describe}.{x.index}"
  | _, _, .isVariant tests value => s!"({value.describe} is {tests})"
  | _, _, .payload _ _ value => s!"{value.describe}.payload"
  | _, _, .letRow targets value body =>
      s!"(let ({targets.describe}) := {value.describe}; {body.describe})"
  | _, _, .takeAt _ _ x _ => s!"move local{x.index}.path"
  | _, _, .letFields targets value body =>
      s!"(let \{{targets.describe}} := {value.describe}; {body.describe})"
  | _, _, .caseOf scrutinee arms => s!"(match {scrutinee.describe} with {arms.describe})"
  | _, _, .deref _ value => s!"*{value.describe}"
  | _, _, .copy value _ => s!"*{value.describe}"
  | _, _, .mutate x value => s!"(*local{x.index} := {value.describe})"
  | _, _, .observe _ value => s!"observe {value.describe}"
  | _, _, .readPlace _ _ x _ => s!"local{x.index}.path"
  | _, _, .copyPlace _ x _ _ => s!"local{x.index}.path"
  | _, _, .writePlace _ _ x _ value => s!"(local{x.index}.path := {value.describe})"
  | _, _, .borrowPlace _ x _ => s!"&mut local{x.index}.path"
  | _, _, .sliceCurrent value => s!"slice current {value.describe}"
  | _, _, .sliceVector value => s!"slice contents {value.describe}"
  | _, _, .vectorSlice value => s!"shared slice {value.describe}"
  | _, _, .borrowArraySlice _ x _ => s!"&mut array slice local{x.index}.path"
  | _, _, .borrowVectorSlice _ x _ => s!"&mut vector slice local{x.index}.path"
  | _, _, .reborrowSlice _ x _ => s!"&mut slice local{x.index}.path"
  | _, _, .takeSliceAt _ x _ => s!"move slice local{x.index}.path"
  | _, _, .resolveSliceAt x _ => s!"resolve slice local{x.index}.path"
  | _, _, .take x => s!"move local{x.index}"
  | _, _, .resolve _ x _ => s!"resolve local{x.index}"
  | _, _, .resolveAt _ x _ _ => s!"resolve local{x.index}.path"
  | _, _, .assumeCurrent x => s!"assumeCurrent local{x.index}"
  | _, _, .handedOver site reference => s!"handedOver@{site}({reference.describe})"
  | _, _, .returned site reference => s!"returned@{site}({reference.describe})"
  | _, _, .vectorLit _ elements => s!"vector[{elements.describe}]"
  | _, _, .length vector => s!"{vector.describe}.length"
  | _, _, .signerAddress signer => s!"{signer.describe}.address"
  | _, _, .index _ vector position => s!"{vector.describe}[{position.describe}]"
  | _, _, .checkIndex _ _ vector position => s!"check({vector.describe}[{position.describe}])"
  | _, _, .push vector element => s!"push({vector.describe}, {element.describe})"
  | _, _, .insert _ vector position element =>
      s!"insert({vector.describe}, {position.describe}, {element.describe})"
  | _, _, .remove _ vector position => s!"remove({vector.describe}, {position.describe})"
  | _, _, .swap _ vector left right =>
      s!"swap({vector.describe}, {left.describe}, {right.describe})"
  | _, _, .concat left right => s!"concat({left.describe}, {right.describe})"
  | _, _, .slice _ vector start stop =>
      s!"slice({vector.describe}, {start.describe}, {stop.describe})"
  | _, _, .reverseSlice _ vector start stop =>
      s!"reverseSlice({vector.describe}, {start.describe}, {stop.describe})"
  | _, _, .destroyEmpty _ vector => s!"destroyEmpty({vector.describe})"
  | _, _, .contains vector needle => s!"contains({vector.describe}, {needle.describe})"
  | _, _, .indexOf vector needle => s!"indexOf({vector.describe}, {needle.describe})"
  | _, _, .order _ left right => s!"order({left.describe}, {right.describe})"
  | _, _, .seqAfter value effect => s!"({value.describe} then {effect.describe})"
  | _, _, .assertion site => s!"assert@{site}"
  | _, _, .anchor site => s!"anchor@{site}"
  | _, _, .assume site => s!"assume@{site}"
  | _, _, .arbitrary => "arbitrary"
  | _, _, .ghostValue site => s!"ghost@{site}"
  | _, _, .ghostSlot value => s!"ghostSlot({value.describe})"
  | _, _, .ghostRead slot => s!"ghostRead({slot.describe})"
  | _, _, .mutationEnd site => s!"mutationEnd@{site}"
  | _, _, .constructed site value => s!"constructed@{site}({value.describe})"
  | _, τ, .globalRead _ _ key => s!"global[{repr τ}]({key.describe})"
  | _, _, .globalContains τ _ key => s!"exists[{repr τ}]({key.describe})"
  | _, .ref τ, .globalBorrow _ _ key => s!"&mut global[{repr τ}]({key.describe})"
  | _, _, @Term.globalPublish _ _ _ _ τ _ _ key value =>
      s!"publish[{repr τ}]({key.describe}, {value.describe})"
  | _, τ, .globalTake _ _ key => s!"take[{repr τ}]({key.describe})"
  | _, _, .memoryWritten site => s!"memoryWritten@{site}"
  | _, _, @Term.ghostStore _ _ _ _ τ _ _ key value =>
      s!"ghostStore[{repr τ}]({key.describe}, {value.describe})"

partial def Args.describe {ρ : ResultShape} : {Γ σs : NRow} → Args unit ρ Γ σs → String
  | _, _, .nil => ""
  | _, _, .cons head .nil => head.describe
  | _, _, .cons head tail => s!"{head.describe}, {tail.describe}"

partial def Arms.describe {ρ : ResultShape} : {Γ : NRow} → {σ τ : NTy} → Arms unit ρ Γ σ τ → String
  | _, _, _, .nil none => ""
  | _, _, _, .nil (some failure) => s!"| _ => throw {repr failure.1}"
  | _, _, _, .cons _ pattern body rest => s!"| {pattern.describe} => {body.describe} {rest.describe}"
  | _, _, _, .guarded _ pattern guard body rest =>
      s!"| {pattern.describe} if {guard.describe} => {body.describe} {rest.describe}"
end

def Function.describe (f : Function unit) : String :=
  s!"params={repr f.params} locals={repr f.locals} result={repr f.result} body={f.body.describe}"

end VerexIR.Proofs.Denote

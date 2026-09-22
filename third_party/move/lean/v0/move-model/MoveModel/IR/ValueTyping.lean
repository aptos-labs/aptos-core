-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.IR.Value

/-!
# Value Typing: `IsValid` over the Declared Types

This module defines the declared types `Ty` and `StructDecl`, together with
the semantic validity predicate `IsValid`.  It models the multisorted
`$IsValid'T'` discipline of the Boogie prelude.

The operational semantics remains untyped because bytecode verification is
an upstream assumption.  Instead, `IsValid` states when a value conforms to a
declared type.  The translation injects these facts at the same boundaries as
the production prover:

* at function entry, for the arguments (`TypedArgs`) and for global memory
  (`TypedMemory`, both in `Syntax.lean`);
* at loop headers, for the havocked loop targets (havoc in a multisorted
  logic ranges over the sort — a havocked local is *defined* and
  well-formed for its declared type);
* at call sites, for the callee's results and the memory it may modify;
* at quantifiers, whose declared domain type bounds the range
  (`EvalSpec` in `Spec.lean`).

Validity includes the following refinements.  A `u64` is below `U64_SIZE`,
and a signer is represented by an address.  Immutable-reference boundary
values are already dereferenced.  A mutable-reference slot contains a
`Value.mut` with a valid payload.  Struct fields match their declarations,
and vector elements are valid with a length representable as `u64`.

`IsValid` is an inductive least predicate.  Ill-founded recursive struct
declarations therefore denote empty types; Move's type system excludes such
declarations upstream.
-/

namespace MoveModel.IR

/-- Move abilities, used both on declarations and as generic constraints. -/
structure AbilitySet where
  copy : Bool := false
  drop : Bool := false
  store : Bool := false
  key : Bool := false
  deriving BEq, Repr

/-- A declaration-scoped Move type parameter. -/
structure TypeParamDecl where
  name : String
  abilities : AbilitySet := {}
  phantom : Bool := false
  deriving BEq, Repr

/-- Move types of the supported fragment.  Types are *declarations*: the
dynamic semantics is untyped (an ill-typed configuration is stuck), and
typing assumptions enter verification as well-formedness conditions over
the boundary state.  `signer` values are addresses.  Reference-typed
*parameters* are dereferenced at the boundary (`IsValid`); references in
code execute (see `Semantics.lean`) and are verified after reference
elimination (`RefElim.lean`). -/
inductive Ty where
  | bool
  | int (nt : NumType)
  | address
  | signer
  | typeParam (index : Nat)
  | struct (r : ResourceId)
  | structInst (r : ResourceId) (args : List Ty)
  | enum (r : ResourceId)
  | enumInst (r : ResourceId) (args : List Ty)
  | vector (t : Ty)
  | ref (t : Ty)
  | mutRef (t : Ty)
  deriving BEq, Repr

/-- Unsigned / signed integer types, abbreviating the unified `int` type: a
Move integer differs only in its range, so both are one constructor carrying a
`NumType`. -/
abbrev Ty.uint (w : IntWidth) : Ty := .int ⟨w, false⟩
abbrev Ty.sint (w : IntWidth) : Ty := .int ⟨w, true⟩

/-- The dominant integer width, abbreviated. -/
abbrev Ty.u64 : Ty := .uint .w64

/-- Is the type a reference type? -/
def Ty.isRef : Ty → Bool
  | .ref _ | .mutRef _ => true
  | _ => false

/-- Substitute declaration-scoped type parameters with concrete arguments. -/
def Ty.instantiate (args : List Ty) : Ty → Ty
  | .typeParam i => args[i]?.getD (.typeParam i)
  | .struct r => .struct r
  | .structInst r inner => .structInst r (inner.map (·.instantiate args))
  | .enum r => .enum r
  | .enumInst r inner => .enumInst r (inner.map (·.instantiate args))
  | .vector t => .vector (t.instantiate args)
  | .ref t => .ref (t.instantiate args)
  | .mutRef t => .mutRef (t.instantiate args)
  | t => t

/-- Reify a declared type as the runtime tag used in global resource keys. -/
def Ty.toTag : Ty → TypeTag
  | .bool => [.bool]
  | .int nt => [.int nt]
  | .address => [.address]
  | .signer => [.signer]
  | .typeParam index => [.typeParam index]
  | .struct r => [.struct r 0]
  | .structInst r args => .struct r args.length :: (args.flatMap Ty.toTag)
  | .enum r => [.enum r 0]
  | .enumInst r args => .enum r args.length :: (args.flatMap Ty.toTag)
  | .vector elem => .vector :: elem.toTag
  | .ref elem => .ref :: elem.toTag
  | .mutRef elem => .mutRef :: elem.toTag

/-- The storage key for one instantiation of a resource declaration. -/
def resourceKey (resource : ResourceId) (args : List Ty) : ResourceKey :=
  ⟨resource, args.map Ty.toTag⟩

/-- Pointwise substitution of declaration-scoped type parameters. -/
def instantiateTypes (args types : List Ty) : List Ty :=
  types.map (·.instantiate args)

/-- A struct declaration: the field types in offset order. -/
structure StructDecl where
  typeParams : List TypeParamDecl := []
  fields : List Ty
  variants : Option (List (List Ty)) := none

/-- The struct declarations of a program, as a partial map. -/
abbrev StructDecls := ResourceId → Option StructDecl

mutual

/-- `IsValid Δ t v`: the runtime value `v` is well-formed at the declared
type `t`, relative to the struct declarations `Δ` (see module docs). -/
inductive IsValid (Δ : StructDecls) : Ty → Value → Prop where
  | bool (b : Bool) : IsValid Δ .bool (.bool b)
  | intv {nt : NumType} {i : Int} :
      nt.lo ≤ i → i < nt.hi → IsValid Δ (.int nt) (.int i)
  | address (a : Address) : IsValid Δ .address (.address a)
  | signer (a : Address) : IsValid Δ .signer (.address a)
  | typeParam {i : Nat} {v : Value} : v.refFree → IsValid Δ (.typeParam i) v
  | ref {t : Ty} {v : Value} : IsValid Δ t v → IsValid Δ (.ref t) v
  | mutRef {t : Ty} {rt : RefTarget} {v : Value} :
      IsValid Δ t v → IsValid Δ (.mutRef t) (.mut rt v)
  | struct {r : ResourceId} {d : StructDecl} {fs : List Value} :
      Δ r = some d → IsValidList Δ d.fields fs →
      IsValid Δ (.struct r) (.struct fs)
  | structInst {r : ResourceId} {d : StructDecl} {args : List Ty}
      {fs : List Value} :
      Δ r = some d → args.length = d.typeParams.length →
      IsValidList Δ (d.fields.map (·.instantiate args)) fs →
      IsValid Δ (.structInst r args) (.struct fs)
  | variant {r : ResourceId} {d : StructDecl} {variants : List (List Ty)}
      {tag : Nat} {ts : List Ty} {fs : List Value} :
      Δ r = some d → d.variants = some variants → variants[tag]? = some ts →
      IsValidList Δ ts fs → IsValid Δ (.enum r) (.variant tag fs)
  | variantInst {r : ResourceId} {d : StructDecl} {variants : List (List Ty)}
      {args : List Ty} {tag : Nat} {ts : List Ty} {fs : List Value} :
      Δ r = some d → args.length = d.typeParams.length →
      d.variants = some variants → variants[tag]? = some ts →
      IsValidList Δ (ts.map (·.instantiate args)) fs →
      IsValid Δ (.enumInst r args) (.variant tag fs)
  | vector {t : Ty} {es : List Value} :
      es.length < U64_SIZE → (∀ v ∈ es, IsValid Δ t v) →
      IsValid Δ (.vector t) (.vector es)

/-- Pointwise well-formedness of a value list at a type list. -/
inductive IsValidList (Δ : StructDecls) : List Ty → List Value → Prop where
  | nil : IsValidList Δ [] []
  | cons {t : Ty} {v : Value} {ts : List Ty} {vs : List Value} :
      IsValid Δ t v → IsValidList Δ ts vs → IsValidList Δ (t :: ts) (v :: vs)

end

/-! ## Inversion pack

`iff` characterizations of `IsValid` at each type constructor, for `simp`. -/

/-- Characterize values valid at Boolean type. -/
@[simp] theorem isValid_bool_iff {Δ : StructDecls} {v : Value} :
    IsValid Δ .bool v ↔ ∃ b, v = .bool b := by
  constructor
  · intro h; cases h with | bool b => exact ⟨b, rfl⟩
  · rintro ⟨b, rfl⟩; exact .bool b

/-- Characterize values valid at an integer type: the unbounded carrier
constrained to the type's range `[nt.lo, nt.hi)`. -/
@[simp] theorem isValid_int_iff {Δ : StructDecls} {nt : NumType} {v : Value} :
    IsValid Δ (.int nt) v ↔ ∃ i, v = .int i ∧ nt.lo ≤ i ∧ i < nt.hi := by
  constructor
  · intro h; cases h with | intv h0 h => exact ⟨_, rfl, h0, h⟩
  · rintro ⟨i, rfl, h0, h⟩; exact .intv h0 h

/-- The unsigned validity constructor, over the width's `[0, size)` range. -/
theorem IsValid.uintv {Δ : StructDecls} {w : IntWidth} {i : Int}
    (h0 : 0 ≤ i) (h : i < (w.size : Int)) : IsValid Δ (.uint w) (.int i) :=
  .intv (by simpa [NumType.lo] using h0) (by simpa [NumType.hi, NumType.size] using h)

/-- The signed validity constructor, over the width's two's-complement range. -/
theorem IsValid.sintv {Δ : StructDecls} {w : IntWidth} {i : Int}
    (h0 : -(w.halfSize : Int) ≤ i) (h : i < (w.halfSize : Int)) :
    IsValid Δ (.sint w) (.int i) :=
  .intv (by simpa [NumType.lo] using h0) (by simpa [NumType.hi] using h)

/-- Characterize values valid at an unsigned integer type (`$IsValid'uN'`). -/
@[simp] theorem isValid_uint_iff {Δ : StructDecls} {w : IntWidth} {v : Value} :
    IsValid Δ (.uint w) v ↔ ∃ i, v = .int i ∧ 0 ≤ i ∧ i < (w.size : Int) := by
  simp [Ty.uint, isValid_int_iff, NumType.lo, NumType.hi]

/-- Characterize values valid at a signed integer type (`$IsValid'iN'`). -/
@[simp] theorem isValid_sint_iff {Δ : StructDecls} {w : IntWidth} {v : Value} :
    IsValid Δ (.sint w) v ↔
      ∃ i, v = .int i ∧ -(w.halfSize : Int) ≤ i ∧ i < (w.halfSize : Int) := by
  simp [Ty.sint, isValid_int_iff, NumType.lo, NumType.hi]

/-- Characterize values valid at bounded `u64` type, with the natural-number
view of the nonnegative carrier. -/
theorem isValid_u64_iff {Δ : StructDecls} {v : Value} :
    IsValid Δ .u64 v ↔ ∃ n, v = .u64 n ∧ n < U64_SIZE := by
  rw [isValid_uint_iff]
  simp only [u64_size_eq]
  constructor
  · rintro ⟨i, rfl, h0, h⟩
    refine ⟨i.toNat, ?_, by omega⟩
    simp [Value.u64, Int.toNat_of_nonneg h0]
  · rintro ⟨n, rfl, h⟩
    exact ⟨n, rfl, by omega, by exact_mod_cast h⟩

/-- The `u64` validity constructor, abbreviated at the dominant width. -/
theorem IsValid.u64 {Δ : StructDecls} {n : Nat} (h : n < U64_SIZE) :
    IsValid Δ .u64 (.u64 n) :=
  .uintv (by omega) (by rw [u64_size_eq]; exact_mod_cast h)

/-- Characterize values valid at address type. -/
@[simp] theorem isValid_address_iff {Δ : StructDecls} {v : Value} :
    IsValid Δ .address v ↔ ∃ a, v = .address a := by
  constructor
  · intro h; cases h with | address a => exact ⟨a, rfl⟩
  · rintro ⟨a, rfl⟩; exact .address a

/-- Characterize values valid at signer type. -/
@[simp] theorem isValid_signer_iff {Δ : StructDecls} {v : Value} :
    IsValid Δ .signer v ↔ ∃ a, v = .address a := by
  constructor
  · intro h; cases h with | signer a => exact ⟨a, rfl⟩
  · rintro ⟨a, rfl⟩; exact .signer a

/-- Boundary validity of an immutable reference is validity of its value. -/
@[simp] theorem isValid_ref_iff {Δ : StructDecls} {t : Ty} {v : Value} :
    IsValid Δ (.ref t) v ↔ IsValid Δ t v := by
  constructor
  · intro h; cases h with | ref h => exact h
  · exact .ref

/-- Characterize valid mutation values representing mutable references. -/
@[simp] theorem isValid_mutRef_iff {Δ : StructDecls} {t : Ty} {v : Value} :
    IsValid Δ (.mutRef t) v ↔
      ∃ rt w, v = .mut rt w ∧ IsValid Δ t w := by
  constructor
  · intro h
    cases h with
    | mutRef h => exact ⟨_, _, rfl, h⟩
  · rintro ⟨rt, w, rfl, h⟩
    exact .mutRef h

/-- Characterize valid struct values by declaration and field validity. -/
@[simp] theorem isValid_struct_iff {Δ : StructDecls} {r : ResourceId}
    {v : Value} :
    IsValid Δ (.struct r) v ↔
      ∃ d fs, Δ r = some d ∧ v = .struct fs ∧ IsValidList Δ d.fields fs := by
  constructor
  · intro h; cases h with | struct hd hfs => exact ⟨_, _, hd, rfl, hfs⟩
  · rintro ⟨d, fs, hd, rfl, hfs⟩; exact .struct hd hfs

/-- Characterize an instantiated generic structure by its substituted fields. -/
@[simp] theorem isValid_structInst_iff {Δ : StructDecls} {r : ResourceId}
    {args : List Ty} {v : Value} :
    IsValid Δ (.structInst r args) v ↔
      ∃ d fs, Δ r = some d ∧ args.length = d.typeParams.length ∧
        v = .struct fs ∧
        IsValidList Δ (instantiateTypes args d.fields) fs := by
  constructor
  · intro h
    cases h with
    | structInst hd hargs hfs => exact ⟨_, _, hd, hargs, rfl, hfs⟩
  · rintro ⟨d, fs, hd, hargs, rfl, hfs⟩
    exact .structInst hd hargs hfs

/-- The payload of a mutation valid at `&mut t` is valid at `t`. -/
theorem IsValid.mutRef_payload {Δ : StructDecls} {t : Ty} {rt : RefTarget}
    {w : Value} (h : IsValid Δ (.mutRef t) (.mut rt w)) : IsValid Δ t w := by
  cases h with
  | mutRef h => exact h

/-- A value valid at mutable-reference type is a mutation value and hence is
not reference-free. -/
theorem IsValid.mutRef_not_refFree {Δ : StructDecls} {t : Ty} {v : Value}
    (h : IsValid Δ (.mutRef t) v) : v.refFree = false := by
  cases h
  rfl

/-- Characterize valid enum values by their declared variant payload. -/
@[simp] theorem isValid_enum_iff {Δ : StructDecls} {r : ResourceId} {v : Value} :
    IsValid Δ (.enum r) v ↔
      ∃ d variants tag ts fs, Δ r = some d ∧ d.variants = some variants ∧
        variants[tag]? = some ts ∧ v = .variant tag fs ∧ IsValidList Δ ts fs := by
  constructor
  · intro h
    cases h with
    | variant hd hv htag hfs => exact ⟨_, _, _, _, _, hd, hv, htag, rfl, hfs⟩
  · rintro ⟨d, variants, tag, ts, fs, hd, hv, htag, rfl, hfs⟩
    exact .variant hd hv htag hfs

/-- Characterize an instantiated generic enum by its substituted payload. -/
@[simp] theorem isValid_enumInst_iff {Δ : StructDecls} {r : ResourceId}
    {args : List Ty} {v : Value} :
    IsValid Δ (.enumInst r args) v ↔
      ∃ d variants tag ts fs,
        Δ r = some d ∧ args.length = d.typeParams.length ∧
        d.variants = some variants ∧ variants[tag]? = some ts ∧
        v = .variant tag fs ∧
        IsValidList Δ (instantiateTypes args ts) fs := by
  constructor
  · intro h
    cases h with
    | variantInst hd hargs hv htag hfs =>
      exact ⟨_, _, _, _, _, hd, hargs, hv, htag, rfl, hfs⟩
  · rintro ⟨d, variants, tag, ts, fs, hd, hargs, hv, htag, rfl, hfs⟩
    exact .variantInst hd hargs hv htag hfs

/-- Characterize valid vectors by bounded length and element validity. -/
@[simp] theorem isValid_vector_iff {Δ : StructDecls} {t : Ty} {v : Value} :
    IsValid Δ (.vector t) v ↔
      ∃ es, v = .vector es ∧ es.length < U64_SIZE ∧
        ∀ w ∈ es, IsValid Δ t w := by
  constructor
  · intro h; cases h with | vector hlen hes => exact ⟨_, rfl, hlen, hes⟩
  · rintro ⟨es, rfl, hlen, hes⟩; exact .vector hlen hes

/-- A valid value list for no types is exactly the empty list. -/
@[simp] theorem isValidList_nil_iff {Δ : StructDecls} {vs : List Value} :
    IsValidList Δ [] vs ↔ vs = [] := by
  constructor
  · intro h; cases h with | nil => rfl
  · rintro rfl; exact .nil

/-- Invert validity of a nonempty typed value list into head and tail validity. -/
@[simp] theorem isValidList_cons_iff {Δ : StructDecls} {t : Ty}
    {ts : List Ty} {vs : List Value} :
    IsValidList Δ (t :: ts) vs ↔
      ∃ v vs', vs = v :: vs' ∧ IsValid Δ t v ∧ IsValidList Δ ts vs' := by
  constructor
  · intro h; cases h with | cons hv hvs => exact ⟨_, _, rfl, hv, hvs⟩
  · rintro ⟨v, vs', rfl, hv, hvs⟩; exact .cons hv hvs

end MoveModel.IR

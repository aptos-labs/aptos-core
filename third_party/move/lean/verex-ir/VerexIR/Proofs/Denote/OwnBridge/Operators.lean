-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Reframe
import VerexIR.Proofs.Maps
import VerexIR.Proofs.Denote.NativeMaps

/-!
# Agreement of a clause's operators across the move to a declaration's frame

A declaration states its data invariant at the frame its type arguments induce
over the runtime frame, at the unit's base function values. A caller holds a
value of the declaration at its own frame, at an instance row.
`Skolems.reframeAtInstance` carries the caller's value to the declaration's
frame and keeps its encoding.

The laws here state, per operator a clause may be built from, that the
operator read at the declaration's frame over moved operands is the operator
read at the caller's frame over the operands themselves. Each is stated for a
symbolic caller frame and a symbolic instance row; the declaration's side reads
the declaration's own types, the caller's side those types substituted by the
row. Together they are the clause language of the bridge
(`VerexIR.Proofs.Denote.OwnBridge.Bridge`): a clause built from them alone
reads alike at both frames, and no other clause is claimed to.
-/

namespace VerexIR.Proofs.Denote

section Agreement
variable {unit : Validation.ValidatedUnit} {functions base : FunctionCarriers unit}
  (sameCarrier : functions.carrier = base.carrier)
  (sameFunctions : HEq functions.functions base.functions)
  (sameNominals : functions.nominals = base.nominals)
  (Θ : @Skolems unit functions) (params : NRow)
  (inhabitable : (params.map Θ.resolve).inhabitable = true)

/-! ## The move itself -/

/-- The move keeps a value's encoding, so it is injective. -/
theorem Skolems.reframeAtInstance_inj {τ : NTy}
    (bounded : τ.paramsAll (fun index => decide (index < params.length)) = true)
    (left right : @NTy.carrier (Θ.toCarriers).toSorts (τ.subst params)) :
    Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable bounded left =
      Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable bounded right ↔
      left = right := by
  constructor
  · intro same
    apply @NTy.encode_injective (Θ.toCarriers).toSorts (τ.subst params)
    rw [← Skolems.encode_reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable bounded left,
      ← Skolems.encode_reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable bounded right,
      same]
  · intro same; rw [same]

/-- Every value at the declaration's frame is the move of a caller's value: the
two frames' transports to the runtime family are inverse pairs. A binder over
the declaration's carrier therefore ranges over exactly the moved values. -/
theorem Skolems.reframeAtInstance_surjective {τ : NTy}
    (bounded : τ.paramsAll (fun index => decide (index < params.length)) = true)
    (target : @NTy.carrier
      ((Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers).toSorts τ) :
    ∃ value : @NTy.carrier (Θ.toCarriers).toSorts (τ.subst params),
      Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
        bounded value = target := by
  let induced := Skolems.atArguments unit base (params.map Θ.resolve) inhabitable
  let carriers := Skolems.runtimeCarrierEqAtInstance sameCarrier sameFunctions sameNominals Θ
    params bounded
  refine ⟨@Skolems.ofRuntime unit functions Θ (τ.subst params)
    (cast carriers.symm (@Skolems.toRuntime unit base induced τ target)), ?_⟩
  unfold Skolems.reframeAtInstance
  rw [@Skolems.toRuntime_ofRuntime unit functions Θ, cast_cast, cast_eq]
  exact @Skolems.ofRuntime_toRuntime unit base induced τ target

/-! ## Order and equality

Both are functions of the encodings (`NTy.order_codec`, `NTy.equal_codec`),
which the move keeps: one law at every type, a type parameter included. -/

/-- The order of two moved values, with the same ranks, is the caller's order
of the values at the substituted type. -/
theorem Skolems.order_reframeAtInstance {τ : NTy}
    (bounded : τ.paramsAll (fun index => decide (index < params.length)) = true)
    (rank : ValueRanks) (left right : @NTy.carrier (Θ.toCarriers).toSorts (τ.subst params)) :
    @NTy.order ((Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers).toSorts rank τ
        (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable bounded left)
        (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable bounded right) =
      @NTy.order (Θ.toCarriers).toSorts rank (τ.subst params) left right := by
  rw [← @NTy.order_codec ((Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers).toSorts,
    ← @NTy.order_codec (Θ.toCarriers).toSorts]
  simp only [NTy.codec_encode, Skolems.encode_reframeAtInstance]

/-- Equality without handle identity of two moved values is the caller's. -/
theorem Skolems.equal_reframeAtInstance {τ : NTy}
    (bounded : τ.paramsAll (fun index => decide (index < params.length)) = true)
    (left right : @NTy.carrier (Θ.toCarriers).toSorts (τ.subst params)) :
    @NTy.equal ((Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers).toSorts false τ
        (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable bounded left)
        (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable bounded right) =
      @NTy.equal (Θ.toCarriers).toSorts false (τ.subst params) left right := by
  rw [@NTy.equal_codec ((Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers).toSorts,
    @NTy.equal_codec (Θ.toCarriers).toSorts]
  simp only [NTy.codec_encode, Skolems.encode_reframeAtInstance]
  rfl

/-! ## Fields

A struct's row moves field by field. The declaration's side names the struct
by its handle, its arguments and its fields; the caller's side is that struct
at the row, whose fields are the substituted ones. A declaration's descriptor
unfolds to that shape, so the laws are instantiated at a descriptor by its
handle, arguments and fields and need no syntactic match on the type. -/

theorem paramsHead {f : Nat → Bool} {τ : NTy} {rest : NRow}
    (bounded : NRow.paramsAll f (.cons τ rest) = true) : τ.paramsAll f = true :=
  (Bool.and_eq_true _ _ |>.mp bounded).1

theorem paramsTail {f : Nat → Bool} {τ : NTy} {rest : NRow}
    (bounded : NRow.paramsAll f (.cons τ rest) = true) : rest.paramsAll f = true :=
  (Bool.and_eq_true _ _ |>.mp bounded).2

theorem paramsFields {f : Nat → Bool} {source : StructHandle} {arguments fields : NRow}
    (bounded : (NTy.struct source arguments fields).paramsAll f = true) : fields.paramsAll f = true :=
  (Bool.and_eq_true _ _ |>.mp bounded).2

theorem paramsStruct {f : Nat → Bool} {source : StructHandle} {arguments fields : NRow}
    (arguments' : arguments.paramsAll f = true) (fields' : fields.paramsAll f = true) :
    (NTy.struct source arguments fields).paramsAll f = true :=
  Bool.and_eq_true _ _ |>.mpr ⟨arguments', fields'⟩

/-- The head field of a struct over a declaration's parameters is over them. -/
theorem NTy.paramsAll_struct_head {f : Nat → Bool} {source : StructHandle}
    {arguments : NRow} {τ : NTy} {rest : NRow}
    (bounded : (NTy.struct source arguments (.cons τ rest)).paramsAll f = true) :
    τ.paramsAll f = true :=
  paramsHead (paramsFields bounded)

/-- The struct of the remaining fields is over them too. -/
theorem NTy.paramsAll_struct_tail {f : Nat → Bool} {source : StructHandle}
    {arguments : NRow} {τ : NTy} {rest : NRow}
    (bounded : (NTy.struct source arguments (.cons τ rest)).paramsAll f = true) :
    (NTy.struct source arguments rest).paramsAll f = true :=
  paramsStruct (Bool.and_eq_true _ _ |>.mp bounded).1 (paramsTail (paramsFields bounded))

/-- The encodings of a struct row's fields, from the row's, at two families
and two spellings of the declaration's arguments and fields. -/
theorem encodeFields {Φ Ψ : Sorts} {source : StructHandle}
    {arguments arguments' : NRow} {τ τ' : NTy} {rest rest' : NRow}
    (left : @HList Φ (.cons τ rest)) (right : @HList Ψ (.cons τ' rest'))
    (same : @NTy.encode Φ (.struct source arguments (.cons τ rest)) left =
      @NTy.encode Ψ (.struct source arguments' (.cons τ' rest')) right) :
    @NTy.encode Φ τ left.1 = @NTy.encode Ψ τ' right.1 ∧
      @NTy.encode Φ (.struct source arguments rest) left.2 =
        @NTy.encode Ψ (.struct source arguments' rest') right.2 := by
  simp only [NTy.encode_struct, HList.encode_cons, RuntimeValue.nominal.injEq, true_and] at same
  have fields := List.toArray_inj same
  simp only [List.cons.injEq] at fields
  exact ⟨fields.1, by simp only [NTy.encode_struct, fields.2]⟩

/-- A struct's head field moves with it: the field of the moved row is the
moved field, at the field's own type. -/
theorem Skolems.reframeAtInstance_struct_fst {source : StructHandle}
    {arguments : NRow} {τ : NTy} {rest : NRow} (bounded)
    (value : @NTy.carrier (Θ.toCarriers).toSorts
      (.struct source (NRow.subst params arguments)
        (.cons (τ.subst params) (NRow.subst params rest)))) :
    (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
        (τ := .struct source arguments (.cons τ rest)) bounded value).1 =
      Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
        (τ := τ) (NTy.paramsAll_struct_head bounded) value.1 := by
  apply @NTy.encode_injective
    ((Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers).toSorts τ
  rw [(encodeFields _ _ (Skolems.encode_reframeAtInstance sameCarrier sameFunctions sameNominals
      Θ params inhabitable (τ := .struct source arguments (.cons τ rest)) bounded value)).1,
    Skolems.encode_reframeAtInstance]

/-- And so do the rest of its fields. -/
theorem Skolems.reframeAtInstance_struct_snd {source : StructHandle}
    {arguments : NRow} {τ : NTy} {rest : NRow} (bounded)
    (value : @NTy.carrier (Θ.toCarriers).toSorts
      (.struct source (NRow.subst params arguments)
        (.cons (τ.subst params) (NRow.subst params rest)))) :
    (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
        (τ := .struct source arguments (.cons τ rest)) bounded value).2 =
      Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
        (τ := .struct source arguments rest) (NTy.paramsAll_struct_tail bounded) value.2 := by
  apply @NTy.encode_injective
    ((Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers).toSorts
    (.struct source arguments rest)
  rw [(encodeFields _ _ (Skolems.encode_reframeAtInstance sameCarrier sameFunctions sameNominals
      Θ params inhabitable (τ := .struct source arguments (.cons τ rest)) bounded value)).2]
  exact (Skolems.encode_reframeAtInstance sameCarrier sameFunctions sameNominals Θ params
    inhabitable (τ := .struct source arguments rest) (NTy.paramsAll_struct_tail bounded)
    value.2).symm

/-! ## Tuples, variants, ghost slots and closed types

A tuple's components move as a struct's fields do. A value under a variant's
name moves under that name, its row moving with it, so a reader of a variant
reads a moved value where it reads the value. A ghost slot moves with its
value. A value whose carrier and encoding do not read the frame stays itself. -/

/-- The encodings of a tuple's head and rest, from the tuple's, at two
families and two spellings of its elements. -/
theorem encodeElements {Φ Ψ : Sorts} {τ τ' : NTy} {rest rest' : NRow}
    (left : @HList Φ (.cons τ rest)) (right : @HList Ψ (.cons τ' rest'))
    (same : @NTy.encode Φ (.tuple (.cons τ rest)) left =
      @NTy.encode Ψ (.tuple (.cons τ' rest')) right) :
    @NTy.encode Φ τ left.1 = @NTy.encode Ψ τ' right.1 ∧
      @NTy.encode Φ (.tuple rest) left.2 = @NTy.encode Ψ (.tuple rest') right.2 := by
  simp only [NTy.encode_tuple, HList.encode_cons, RuntimeValue.tuple.injEq] at same
  have elements := List.toArray_inj same
  simp only [List.cons.injEq] at elements
  exact ⟨elements.1, by simp only [NTy.encode_tuple, elements.2]⟩

/-- A tuple's head component moves with it. -/
theorem Skolems.reframeAtInstance_tuple_fst {τ : NTy} {rest : NRow} (bounded)
    (value : @NTy.carrier (Θ.toCarriers).toSorts
      (.tuple (.cons (τ.subst params) (NRow.subst params rest)))) :
    (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
        (τ := .tuple (.cons τ rest)) bounded value).1 =
      Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
        (τ := τ) (paramsHead bounded) value.1 := by
  apply @NTy.encode_injective
    ((Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers).toSorts τ
  rw [(encodeElements _ _ (Skolems.encode_reframeAtInstance sameCarrier sameFunctions sameNominals
      Θ params inhabitable (τ := .tuple (.cons τ rest)) bounded value)).1,
    Skolems.encode_reframeAtInstance]

/-- And so does the rest of it. -/
theorem Skolems.reframeAtInstance_tuple_snd {τ : NTy} {rest : NRow} (bounded)
    (value : @NTy.carrier (Θ.toCarriers).toSorts
      (.tuple (.cons (τ.subst params) (NRow.subst params rest)))) :
    (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
        (τ := .tuple (.cons τ rest)) bounded value).2 =
      Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
        (τ := .tuple rest) (paramsTail bounded) value.2 := by
  apply @NTy.encode_injective
    ((Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers).toSorts (.tuple rest)
  rw [(encodeElements _ _ (Skolems.encode_reframeAtInstance sameCarrier sameFunctions sameNominals
      Θ params inhabitable (τ := .tuple (.cons τ rest)) bounded value)).2]
  exact (Skolems.encode_reframeAtInstance sameCarrier sameFunctions sameNominals Θ params
    inhabitable (τ := .tuple rest) (paramsTail bounded) value.2).symm

private theorem paramsVariant {f : Nat → Bool} {source : StructHandle}
    {arguments fields : NRow} {name : String} {names : List String} {rest : NRows}
    {distinct : (name :: names).Nodup}
    (bounded : (NTy.enum source arguments (name :: names) (.cons fields rest) distinct).paramsAll f
      = true) : (NTy.tuple fields).paramsAll f = true := by
  simp only [NTy.paramsAll, NRows.paramsAll, Bool.and_eq_true] at bounded ⊢
  exact bounded.2.1

theorem paramsVariants {f : Nat → Bool} {source : StructHandle}
    {arguments fields : NRow} {name : String} {names : List String} {rest : NRows}
    {distinct : (name :: names).Nodup}
    (bounded : (NTy.enum source arguments (name :: names) (.cons fields rest) distinct).paramsAll f
      = true) :
    (NTy.enum source arguments names rest (List.nodup_cons.mp distinct).2).paramsAll f = true := by
  simp only [NTy.paramsAll, NRows.paramsAll, Bool.and_eq_true] at bounded ⊢
  exact ⟨bounded.1, bounded.2.2⟩

/-- A value under a variant's name moves under that name, its row moving with
it. -/
theorem Skolems.reframeAtInstance_enum_inl {source : StructHandle}
    {arguments fields : NRow} {name : String} {names : List String} {rest : NRows}
    {distinct : (name :: names).Nodup} (bounded)
    (value : @HList (Θ.toCarriers).toSorts (NRow.subst params fields)) :
    Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
        (τ := .enum source arguments (name :: names) (.cons fields rest) distinct) bounded
        (.inl value) =
      .inl (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
        (τ := .tuple fields) (paramsVariant bounded) value) := by
  let Ψ := (Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers
  apply @NTy.encode_injective (Ψ).toSorts
    (.enum source arguments (name :: names) (.cons fields rest) distinct)
  have row : @NTy.encode (Ψ).toSorts (.tuple fields)
        (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
          (τ := .tuple fields) (paramsVariant bounded) value) =
      @NTy.encode (Θ.toCarriers).toSorts (.tuple (NRow.subst params fields)) value :=
    Skolems.encode_reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
      (τ := .tuple fields) (paramsVariant bounded) value
  simp only [NTy.encode_tuple, RuntimeValue.tuple.injEq] at row
  refine (Skolems.encode_reframeAtInstance sameCarrier sameFunctions sameNominals Θ params
    inhabitable (τ := .enum source arguments (name :: names) (.cons fields rest) distinct)
    bounded (.inl value)).trans ?_
  show @NTy.encode (Θ.toCarriers).toSorts
      (.enum source (NRow.subst params arguments) (name :: names)
        (.cons (NRow.subst params fields) (NRows.subst params rest)) distinct) (.inl value) = _
  simp only [NTy.encode_enum_inl, row]

/-- A value under a later variant's name moves under the later names. -/
theorem Skolems.reframeAtInstance_enum_inr {source : StructHandle}
    {arguments fields : NRow} {name : String} {names : List String} {rest : NRows}
    {distinct : (name :: names).Nodup} (bounded)
    (value : @variantCarrier (Θ.toCarriers).toSorts names (NRows.subst params rest)) :
    Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
        (τ := .enum source arguments (name :: names) (.cons fields rest) distinct) bounded
        (.inr value) =
      .inr (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
        (τ := .enum source arguments names rest (List.nodup_cons.mp distinct).2)
        (paramsVariants bounded) value) := by
  let Ψ := (Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers
  apply @NTy.encode_injective (Ψ).toSorts
    (.enum source arguments (name :: names) (.cons fields rest) distinct)
  have later : @NTy.encode (Ψ).toSorts (.enum source arguments names rest (List.nodup_cons.mp distinct).2)
        (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
          (τ := .enum source arguments names rest (List.nodup_cons.mp distinct).2)
          (paramsVariants bounded) value) =
      @NTy.encode (Θ.toCarriers).toSorts
        (.enum source (NRow.subst params arguments) names (NRows.subst params rest)
          (List.nodup_cons.mp distinct).2) value :=
    Skolems.encode_reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
      (τ := .enum source arguments names rest (List.nodup_cons.mp distinct).2)
      (paramsVariants bounded) value
  refine (Skolems.encode_reframeAtInstance sameCarrier sameFunctions sameNominals Θ params
    inhabitable (τ := .enum source arguments (name :: names) (.cons fields rest) distinct)
    bounded (.inr value)).trans ?_
  show @NTy.encode (Θ.toCarriers).toSorts
      (.enum source (NRow.subst params arguments) (name :: names)
        (.cons (NRow.subst params fields) (NRows.subst params rest)) distinct) (.inr value) = _
  simp only [NTy.encode_enum_inr, later]

/-- A ghost slot moves with its value. -/
theorem Skolems.reframeAtInstance_ghost {slot : NTy} (bounded)
    (value : @NTy.carrier (Θ.toCarriers).toSorts (.ghost (slot.subst params))) :
    Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
        (τ := .ghost slot) bounded value =
      Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
        (τ := slot) (by simpa only [NTy.paramsAll] using bounded) value := by
  let Ψ := (Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers
  apply @NTy.encode_injective (Ψ).toSorts (.ghost slot)
  refine (Skolems.encode_reframeAtInstance sameCarrier sameFunctions sameNominals Θ params
    inhabitable (τ := .ghost slot) bounded value).trans ?_
  rw [@NTy.encode_ghost (Ψ).toSorts, Skolems.encode_reframeAtInstance]
  exact @NTy.encode_ghost (Θ.toCarriers).toSorts (slot.subst params) value

/-- A value whose carrier and encoding do not read the frame stays itself. -/
theorem Skolems.reframeAtInstance_of_encode {τ : NTy} (bounded)
    (value : @NTy.carrier (Θ.toCarriers).toSorts (τ.subst params))
    (carriers : @NTy.carrier (Θ.toCarriers).toSorts (τ.subst params) =
      @NTy.carrier ((Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers).toSorts τ)
    (encoded : ∀ value,
      @NTy.encode ((Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers).toSorts τ
        (cast carriers value) = @NTy.encode (Θ.toCarriers).toSorts (τ.subst params) value) :
    Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable bounded
        value = cast carriers value := by
  apply @NTy.encode_injective
    ((Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers).toSorts τ
  rw [Skolems.encode_reframeAtInstance, encoded]

/-- A boolean stays itself. -/
theorem Skolems.reframeAtInstance_bool (bounded) (value : Bool) :
    Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
      (τ := .bool) bounded value = value :=
  Skolems.reframeAtInstance_of_encode sameCarrier sameFunctions sameNominals Θ params inhabitable
    (τ := .bool) bounded value rfl fun _ => rfl

/-- An integer stays itself. -/
theorem Skolems.reframeAtInstance_int {width : Nat} {signed : Bool} (bounded)
    (value : SpecInt (.bits width) signed) :
    Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
      (τ := .int width signed) bounded value = value :=
  Skolems.reframeAtInstance_of_encode sameCarrier sameFunctions sameNominals Θ params inhabitable
    (τ := .int width signed) bounded value rfl fun _ => rfl

/-- An address stays itself. -/
theorem Skolems.reframeAtInstance_address (bounded) (value : String) :
    Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
      (τ := .address) bounded value = value :=
  Skolems.reframeAtInstance_of_encode sameCarrier sameFunctions sameNominals Θ params inhabitable
    (τ := .address) bounded value rfl fun _ => rfl

/-! ## Vectors and their reads

A vector's elements move with it, so its size is the caller's and a read
inside its bounds is the moved read. The fallback of a read outside the bounds
is an arbitrary value of the element's carrier at each frame
(`NTy.plainFallback`), and the two carriers are related by the move only, so
no law states that the fallback moves: a read agrees where the clause's guard
places its index inside the bounds, with whatever fallback each side spells. -/

/-- A vector's elements move with it. -/
theorem Skolems.reframeAtInstance_vector_values {element : NTy} (bounded)
    (vector : @NTy.carrier (Θ.toCarriers).toSorts (.vector (element.subst params))) :
    (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
        (τ := .vector element) bounded vector).values =
      vector.values.map (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ
        params inhabitable (τ := element) (by simpa only [NTy.paramsAll] using bounded)) := by
  let Ψ := (Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers
  have encoded : @NTy.encode (Ψ).toSorts (.vector element)
        (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
          (τ := .vector element) bounded vector) =
      @NTy.encode (Θ.toCarriers).toSorts (.vector (element.subst params)) vector :=
    Skolems.encode_reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
      (τ := .vector element) bounded vector
  rw [@NTy.encode_vector (Ψ).toSorts, @NTy.encode_vector (Θ.toCarriers).toSorts] at encoded
  simp only [RuntimeValue.vector.injEq] at encoded
  apply Array.map_inj_right (f := @NTy.encode (Ψ).toSorts element)
    (fun _ _ same => @NTy.encode_injective (Ψ).toSorts element _ _ same) |>.mp
  rw [Array.map_map]
  refine encoded.trans ?_
  apply Array.map_congr_left
  intro value _
  exact (Skolems.encode_reframeAtInstance sameCarrier sameFunctions sameNominals Θ params
    inhabitable _ value).symm

/-- A moved vector has the caller's size. -/
theorem Skolems.reframeAtInstance_vector_size {element : NTy} (bounded)
    (vector : @NTy.carrier (Θ.toCarriers).toSorts (.vector (element.subst params))) :
    (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
        (τ := .vector element) bounded vector).values.size = vector.values.size := by
  rw [Skolems.reframeAtInstance_vector_values, Array.size_map]

/-- An optional read of a moved vector is the moved optional read. -/
theorem Skolems.reframeAtInstance_vector_read? {element : NTy} (bounded)
    (vector : @NTy.carrier (Θ.toCarriers).toSorts (.vector (element.subst params))) (index : Nat) :
    (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
        (τ := .vector element) bounded vector).values[index]? =
      vector.values[index]?.map (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals
        Θ params inhabitable (τ := element) (by simpa only [NTy.paramsAll] using bounded)) := by
  rw [Skolems.reframeAtInstance_vector_values, Array.getElem?_map]

/-- A read of a moved vector inside its bounds is the moved read, whatever
fallback either side names: the bound makes the fallback's branch unreachable
at both frames. -/
theorem Skolems.reframeAtInstance_vector_read {element : NTy} (bounded)
    (vector : @NTy.carrier (Θ.toCarriers).toSorts (.vector (element.subst params))) (index : Nat)
    (within : index < vector.values.size)
    (fallback : @NTy.carrier
      ((Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers).toSorts element)
    (fallback' : @NTy.carrier (Θ.toCarriers).toSorts (element.subst params)) :
    (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
        (τ := .vector element) bounded vector).values[index]?.getD fallback =
      Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
        (τ := element) (by simpa only [NTy.paramsAll] using bounded)
        (vector.values[index]?.getD fallback') := by
  rw [Skolems.reframeAtInstance_vector_read?, Array.getElem?_eq_getElem within]
  rfl

/-- A read of a moved vector, with no bound, is the moved read where the
declaration's fallback is the moved caller's fallback: the premise a clause
owes where nothing bounds its read. It holds where the element's carrier does
not read the frame, since the move is then the identity
(`Skolems.reframeAtInstance_of_encode`); no law gives it at a carrier that
reads the frame. -/
theorem Skolems.reframeAtInstance_vector_read_of_fallback {element : NTy} (bounded)
    (vector : @NTy.carrier (Θ.toCarriers).toSorts (.vector (element.subst params))) (index : Nat)
    (fallback : @NTy.carrier
      ((Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers).toSorts element)
    (fallback' : @NTy.carrier (Θ.toCarriers).toSorts (element.subst params))
    (moves : Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
        (τ := element) (by simpa only [NTy.paramsAll] using bounded) fallback' = fallback) :
    (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
        (τ := .vector element) bounded vector).values[index]?.getD fallback =
      Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
        (τ := element) (by simpa only [NTy.paramsAll] using bounded)
        (vector.values[index]?.getD fallback') := by
  rw [Skolems.reframeAtInstance_vector_read?]
  cases vector.values[index]? with
  | none => exact moves.symm
  | some _ => rfl

/-! ## Tables

A table's entries move pairwise. The validity of the typed map a clause reads
off a table depends on its key reader and its discipline: for the sequence
discipline on whether two keys read alike, for an ordered one on how they
compare. So the installed clause's map, whatever key reader it was generated
with, is valid exactly where the caller's map with the caller's key reader is,
given that the two readers agree on that relation across the move. The value
readers are free on both sides: no discipline observes values. -/

private theorem tableValueBounded {allowed : Nat → Bool} {key value : NTy}
    (bounded : (NTy.table key value).paramsAll allowed = true) : value.paramsAll allowed = true := by
  simp only [NTy.paramsAll, Bool.and_eq_true] at bounded; exact bounded.2

/-- A table's key type over a declaration's parameters is over them. -/
theorem NTy.paramsAll_table_key {allowed : Nat → Bool} {key value : NTy}
    (bounded : (NTy.table key value).paramsAll allowed = true) : key.paramsAll allowed = true := by
  simp only [NTy.paramsAll, Bool.and_eq_true] at bounded; exact bounded.1

/-- A table's entries move with it, each at its substituted type. -/
theorem Skolems.reframeAtInstance_table_entries {key value : NTy} (bounded)
    (table : @NTy.carrier (Θ.toCarriers).toSorts (.table (key.subst params) (value.subst params))) :
    (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
        (τ := .table key value) bounded table).entries =
      table.entries.map fun entry =>
        (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
           (NTy.paramsAll_table_key bounded) entry.1,
         Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
           (tableValueBounded bounded) entry.2) := by
  let Ψ := (Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers
  have encoded : @NTy.encode (Ψ).toSorts (.table key value)
        (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
          (τ := .table key value) bounded table) =
      @NTy.encode (Θ.toCarriers).toSorts (.table (key.subst params) (value.subst params)) table :=
    Skolems.encode_reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
      (τ := .table key value) bounded table
  rw [@NTy.encode_table (Ψ).toSorts, @NTy.encode_table (Θ.toCarriers).toSorts] at encoded
  simp only [RuntimeValue.vector.injEq] at encoded
  have encoded := List.toArray_inj encoded
  rw [show (table.entries.map fun entry =>
        RuntimeValue.tuple #[@NTy.encode (Θ.toCarriers).toSorts (key.subst params) entry.1,
          @NTy.encode (Θ.toCarriers).toSorts (value.subst params) entry.2]) =
      (table.entries.map fun entry =>
        (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
           (NTy.paramsAll_table_key bounded) entry.1,
         Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
           (tableValueBounded bounded) entry.2)).map
        (fun entry => RuntimeValue.tuple #[@NTy.encode (Ψ).toSorts key entry.1, @NTy.encode (Ψ).toSorts value entry.2]) by
      simp only [List.map_map, Function.comp_def, Ψ, Skolems.encode_reframeAtInstance]] at encoded
  have injective : ∀ a b : @NTy.carrier (Ψ).toSorts key × @NTy.carrier (Ψ).toSorts value,
      RuntimeValue.tuple #[@NTy.encode (Ψ).toSorts key a.1, @NTy.encode (Ψ).toSorts value a.2] =
      RuntimeValue.tuple #[@NTy.encode (Ψ).toSorts key b.1, @NTy.encode (Ψ).toSorts value b.2] → a = b := by
    intro a b same
    simp only [RuntimeValue.tuple.injEq, Array.mk.injEq, List.cons.injEq, and_true] at same
    exact Prod.ext (@NTy.encode_injective (Ψ).toSorts key _ _ same.1) (@NTy.encode_injective (Ψ).toSorts value _ _ same.2)
  exact List.map_inj_right (fun a b same => injective a b same) |>.mp encoded

/-- The sequence validity of the map read off a moved table with one key
reader is that of the map read off the table with another, where the two
readers identify the same keys across the move. -/
theorem Skolems.valid_sequence_reframeAtInstance_of_agree {key value : NTy} (bounded)
    (table : @NTy.carrier (Θ.toCarriers).toSorts (.table (key.subst params) (value.subst params)))
    {K V K' V' : Type}
    (readKey : @NTy.carrier ((Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers).toSorts key → K)
    (readValue : @NTy.carrier ((Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers).toSorts value → V)
    (readKey' : @NTy.carrier (Θ.toCarriers).toSorts (key.subst params) → K')
    (readValue' : @NTy.carrier (Θ.toCarriers).toSorts (value.subst params) → V')
    (agree : ∀ a b,
      readKey (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
        (NTy.paramsAll_table_key bounded) a) =
      readKey (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
        (NTy.paramsAll_table_key bounded) b) ↔ readKey' a = readKey' b) :
    SpecMap.Valid .sequence
        (SpecMap.ofEntries readKey readValue
          (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
            (τ := .table key value) bounded table)) ↔
      SpecMap.Valid .sequence (SpecMap.ofEntries readKey' readValue' table) := by
  simp only [SpecMap.Valid, SpecMap.Distinct, SpecMap.ofEntries,
    Skolems.reframeAtInstance_table_entries, List.map_map, List.Nodup, List.pairwise_map,
    Function.comp_def]
  have reads : ∀ a b,
      readKey (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
        (NTy.paramsAll_table_key bounded) a) ≠
      readKey (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
        (NTy.paramsAll_table_key bounded) b) ↔
      readKey' a ≠ readKey' b := fun a b => not_congr (agree a b)
  simp only [reads]

/-- The sequence validity under injective key readers on both sides: injective
readers identify the keys the move identifies, which are the equal ones. -/
theorem Skolems.valid_sequence_reframeAtInstance {key value : NTy} (bounded)
    (table : @NTy.carrier (Θ.toCarriers).toSorts (.table (key.subst params) (value.subst params)))
    {K V K' V' : Type}
    (readKey : @NTy.carrier ((Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers).toSorts key → K)
    (readValue : @NTy.carrier ((Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers).toSorts value → V)
    (readKey' : @NTy.carrier (Θ.toCarriers).toSorts (key.subst params) → K')
    (readValue' : @NTy.carrier (Θ.toCarriers).toSorts (value.subst params) → V')
    (injective : Function.Injective readKey) (injective' : Function.Injective readKey') :
    SpecMap.Valid .sequence
        (SpecMap.ofEntries readKey readValue
          (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
            (τ := .table key value) bounded table)) ↔
      SpecMap.Valid .sequence (SpecMap.ofEntries readKey' readValue' table) :=
  Skolems.valid_sequence_reframeAtInstance_of_agree sameCarrier sameFunctions sameNominals Θ params
    inhabitable bounded table readKey readValue readKey' readValue' fun a b =>
      ⟨fun same => congrArg readKey'
          ((Skolems.reframeAtInstance_inj sameCarrier sameFunctions sameNominals Θ params
            inhabitable _ a b).mp (injective same)),
        fun same => by rw [injective' same]⟩

/-- The sequence validity of a table's map as the declaration states it, with
the identity key reader, is the validity of the caller's map with the caller's
typed key reader (`NTy.read`), which is injective at every type. -/
theorem Skolems.valid_sequence_reframeAtInstance_read {key value : NTy} (bounded)
    (table : @NTy.carrier (Θ.toCarriers).toSorts (.table (key.subst params) (value.subst params)))
    {V V' : Type}
    (readValue : @NTy.carrier ((Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers).toSorts value → V)
    (readValue' : @NTy.carrier (Θ.toCarriers).toSorts (value.subst params) → V') :
    SpecMap.Valid .sequence
        (SpecMap.ofEntries (fun key => key) readValue
          (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
            (τ := .table key value) bounded table)) ↔
      SpecMap.Valid .sequence
        (SpecMap.ofEntries (@NTy.read (Θ.toCarriers).toSorts (key.subst params)) readValue' table) :=
  Skolems.valid_sequence_reframeAtInstance sameCarrier sameFunctions sameNominals Θ params
    inhabitable bounded table (fun key => key) readValue
    (@NTy.read (Θ.toCarriers).toSorts (key.subst params)) readValue'
    (fun _ _ same => same) (@NativeMaps.NTy.read_injective Θ.toCarriers.toSorts (key.subst params))

/-- The validity of an ordered map read off a moved table is that of the map
read off the table, where the two key orders compare the keys the two readers
give alike across the move. Injectivity of the readers is not enough here: an
ordered discipline observes how keys compare, not only whether they differ. -/
theorem Skolems.valid_ordered_reframeAtInstance {key value : NTy} (bounded)
    (table : @NTy.carrier (Θ.toCarriers).toSorts (.table (key.subst params) (value.subst params)))
    {K V K' V' : Type}
    (readKey : @NTy.carrier ((Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers).toSorts key → K)
    (readValue : @NTy.carrier ((Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers).toSorts value → V)
    (readKey' : @NTy.carrier (Θ.toCarriers).toSorts (key.subst params) → K')
    (readValue' : @NTy.carrier (Θ.toCarriers).toSorts (value.subst params) → V')
    (order : SpecMap.KeyOrder K) (order' : SpecMap.KeyOrder K')
    (agree : ∀ a b,
      order.compare
          (readKey (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params
            inhabitable (NTy.paramsAll_table_key bounded) a))
          (readKey (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params
            inhabitable (NTy.paramsAll_table_key bounded) b)) =
        order'.compare (readKey' a) (readKey' b)) :
    SpecMap.Valid (.ordered order)
        (SpecMap.ofEntries readKey readValue
          (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
            (τ := .table key value) bounded table)) ↔
      SpecMap.Valid (.ordered order') (SpecMap.ofEntries readKey' readValue' table) := by
  simp only [SpecMap.Valid, SpecMap.Ascending, SpecMap.ofEntries,
    Skolems.reframeAtInstance_table_entries, List.map_map, List.pairwise_map, Function.comp_def]
  simp only [agree]

/-- An ordered map as the declaration states it, keyed by the native values
under their structural order (`NTy.keyOrder`), is valid exactly where the
caller's map is under any key reader and key order that compare two read keys
as the caller's structural order compares the keys: the structural order is a
function of the encodings, which the move keeps
(`Skolems.order_reframeAtInstance`). -/
theorem Skolems.valid_ordered_reframeAtInstance_keyOrder {key value : NTy} (bounded)
    (table : @NTy.carrier (Θ.toCarriers).toSorts (.table (key.subst params) (value.subst params)))
    {V K' V' : Type} (rank : ValueRanks)
    (readValue : @NTy.carrier ((Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers).toSorts value → V)
    (readKey' : @NTy.carrier (Θ.toCarriers).toSorts (key.subst params) → K')
    (readValue' : @NTy.carrier (Θ.toCarriers).toSorts (value.subst params) → V')
    (order' : SpecMap.KeyOrder K')
    (reads : ∀ a b, order'.compare (readKey' a) (readKey' b) =
      @NTy.order (Θ.toCarriers).toSorts rank (key.subst params) a b) :
    SpecMap.Valid
        (.ordered (@NTy.keyOrder
          ((Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers).toSorts rank key))
        (SpecMap.ofEntries (fun key => key) readValue
          (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
            (τ := .table key value) bounded table)) ↔
      SpecMap.Valid (.ordered order') (SpecMap.ofEntries readKey' readValue' table) :=
  Skolems.valid_ordered_reframeAtInstance sameCarrier sameFunctions sameNominals Θ params
    inhabitable bounded table (fun key => key) readValue readKey' readValue' _ order'
    fun a b => (Skolems.order_reframeAtInstance sameCarrier sameFunctions sameNominals Θ params
      inhabitable (NTy.paramsAll_table_key bounded) rank a b).trans (reads a b).symm

/-- An ordered map as the declaration states it at its own frame, keyed by
the family's typed read of the key type under the family's order of the typed
keys, is valid exactly where the caller's map is under any key reader and key
order that compare two read keys as the caller's structural order compares
the keys: the family's typed reads compare as its values do
(`NTy.readOrder_read_order`), and the structural order is the caller's across
the move (`Skolems.order_reframeAtInstance`). -/
theorem Skolems.valid_ordered_reframeAtInstance_read {key value : NTy} (bounded)
    (table : @NTy.carrier (Θ.toCarriers).toSorts (.table (key.subst params) (value.subst params)))
    {V K' V' : Type} (rank : ValueRanks)
    (readValue : @NTy.carrier ((Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers).toSorts value → V)
    (readKey' : @NTy.carrier (Θ.toCarriers).toSorts (key.subst params) → K')
    (readValue' : @NTy.carrier (Θ.toCarriers).toSorts (value.subst params) → V')
    (order' : SpecMap.KeyOrder K')
    (reads : ∀ a b, order'.compare (readKey' a) (readKey' b) =
      @NTy.order (Θ.toCarriers).toSorts rank (key.subst params) a b) :
    SpecMap.Valid
        (.ordered (@NTy.readOrder
          ((Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers).toSorts rank key))
        (SpecMap.ofEntries
          (@NTy.read ((Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers).toSorts key)
          readValue
          (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
            (τ := .table key value) bounded table)) ↔
      SpecMap.Valid (.ordered order') (SpecMap.ofEntries readKey' readValue' table) :=
  Skolems.valid_ordered_reframeAtInstance sameCarrier sameFunctions sameNominals Θ params
    inhabitable bounded table _ readValue readKey' readValue' _ order'
    fun a b =>
      (@NTy.readOrder_read_order
        ((Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers).toSorts rank key _ _).trans
        ((Skolems.order_reframeAtInstance sameCarrier sameFunctions sameNominals Θ params
          inhabitable (NTy.paramsAll_table_key bounded) rank a b).trans (reads a b).symm)

end Agreement

/-- A certified integer is read by its value injectively: the key reader of a
table keyed by integers identifies exactly the equal keys. -/
theorem _root_.VerexIR.SpecInt.val_injective {width : IntWidth} {signed : Bool} :
    Function.Injective (@SpecInt.val width signed) :=
  fun _ _ same => SpecInt.ext same


end VerexIR.Proofs.Denote

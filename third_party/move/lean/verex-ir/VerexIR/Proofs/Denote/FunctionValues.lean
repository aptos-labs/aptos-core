-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Term
import VerexIR.Proofs.GlobalTyping

/-!
# Function values of a link set

The pieces a link set's generated function values are built from
(`doc/plan/native-function-values.md`, "Step 2b, decided"): the carriers of
native types and their codecs over an explicit function arm, so that a
generated family can decode and encode the captures of its closure sites
before its instance exists; the closures of a function type no site
encodes (`ForeignClosure`); and the laws relating the codecs, which the
generated laws instantiate.

The codecs agree with the base frame's (`Carriers.runtime`): at a concrete
type, `NTy.carrierWith (FunctionCarriers.carrier unit) τ` is the base
frame's carrier by definition, and the codecs are the base frame's at the
instance's function arm.
-/

namespace VerexIR.Proofs.Denote

open VerexIR.SemanticOperations

section Carriers

variable (C : NRow → Sharing → NRow → Type)

mutual
/-- The carrier of a native type at the base frame, its function types
carried by `C`. -/
@[reducible] def NTy.carrierWith : NTy → Type
  | .unit => Unit
  | .bool => Bool
  | .int width signed => SpecInt (.bits width) signed
  | .num => SpecInt .unbounded true
  | .address => String
  | .signer => String
  | .string => String
  | .bytes => Array UInt8
  | .tuple elements => HListWith elements
  | .struct _ _ fields => HListWith fields
  | .enum _ _ names rows _ => variantCarrierWith names rows
  | .vector element => SpecVector (NTy.carrierWith element)
  | .table key value => SpecEntries (NTy.carrierWith key × NTy.carrierWith value)
  | .ghost slot => NTy.carrierWith slot
  | .ref referent => NTy.carrierWith referent × NTy.carrierWith referent
  | .param _ => PlainValue
  | .function parameters shared results => C parameters shared results

/-- A row of values, as `HList` at the base frame. -/
def HListWith : NRow → Type
  | .nil => Unit
  | .cons τ rest => NTy.carrierWith τ × HListWith rest

/-- A variant value, as `variantCarrier` at the base frame. -/
def variantCarrierWith : List String → NRows → Type
  | _, .nil => Empty
  | [], .cons _ _ => Empty
  | _ :: names, .cons fields rest => HListWith fields ⊕ variantCarrierWith names rest
end

end Carriers

section Codecs

variable {C : NRow → Sharing → NRow → Type}

section Encode

variable (encode : (parameters : NRow) → (shared : Sharing) → (results : NRow) →
  C parameters shared results → ClosureValue)

mutual
/-- The runtime value of a value, its function values encoded by `encode`. -/
def NTy.encodeWith : (τ : NTy) → τ.carrierWith C → RuntimeValue
  | .unit, _ => .unit
  | .bool, value => .bool value
  | .int _ _, value => .integer value.val
  | .num, value => .integer value.val
  | .address, value => .address value
  | .signer, value => .signer value
  | .string, value => .string value
  | .bytes, value => .bytes value
  | .tuple elements, values => .tuple (NRow.encodeWith elements values).toArray
  | .struct source _ fields, values => .nominal source none (NRow.encodeWith fields values).toArray
  | .enum source _ names rows _, value => NRows.encodeWith source names rows value
  | .vector element, values => .vector (values.values.map (NTy.encodeWith element))
  | .table key value, entries =>
      .vector (entries.entries.map fun entry =>
        RuntimeValue.tuple #[NTy.encodeWith key entry.1, NTy.encodeWith value entry.2]).toArray
  | .ghost slot, value => .ghost (NTy.encodeWith slot value)
  | .ref referent, value => .tuple #[NTy.encodeWith referent value.1, NTy.encodeWith referent value.2]
  | .param _, value => value.val
  | .function parameters shared results, value => (encode parameters shared results value).encode

/-- The runtime values of a row's values. -/
def NRow.encodeWith : (row : NRow) → HListWith C row → List RuntimeValue
  | .nil, _ => []
  | .cons τ rest, values => NTy.encodeWith τ values.1 :: NRow.encodeWith rest values.2

/-- The runtime value of a variant value under its declaration's handle. -/
def NRows.encodeWith (source : StructHandle) :
    (names : List String) → (rows : NRows) → variantCarrierWith C names rows → RuntimeValue
  | _, .nil, value => nomatch value
  | [], .cons _ _, value => nomatch value
  | name :: _, .cons fields _, .inl values =>
      .nominal source (some name) (NRow.encodeWith fields values).toArray
  | _ :: names, .cons _ rest, .inr value => NRows.encodeWith source names rest value
end

end Encode

section Decode

variable (decode : (parameters : NRow) → (shared : Sharing) → (results : NRow) →
  ClosureValue → Option (C parameters shared results))

mutual
/-- The value a runtime value encodes, its function values decoded by
`decode`. -/
def NTy.decodeWith : (τ : NTy) → RuntimeValue → Option (τ.carrierWith C)
  | .unit, raw => Codec.unit.decode? raw
  | .bool, raw => Codec.bool.decode? raw
  | .int width signed, raw => (Codec.specInt (.bits width) signed).decode? raw
  | .num, raw => (Codec.specInt .unbounded true).decode? raw
  | .address, raw => Codec.address.decode? raw
  | .signer, raw => Codec.signer.decode? raw
  | .string, raw => Codec.string.decode? raw
  | .bytes, raw => Codec.bytes.decode? raw
  | .tuple elements, .tuple values => NRow.decodeWith elements values.toList
  | .struct source _ fields, .nominal actual none values =>
      if actual = source then NRow.decodeWith fields values.toList else none
  | .enum source _ names rows _, .nominal actual (some name) values =>
      if actual = source then NRows.decodeWith names rows name values.toList else none
  | .vector element, .vector values =>
      (values.toList.mapM (NTy.decodeWith element)).bind fun decoded =>
        if bound : decoded.length < 2 ^ 64 then
          some ⟨decoded.toArray, by simpa only [List.size_toArray] using bound⟩
        else none
  | .table key value, .vector entries =>
      (entries.toList.mapM fun (entry : RuntimeValue) => match entry with
        | .tuple pair => match pair.toList with
          | [k, v] => (NTy.decodeWith key k).bind fun k => (NTy.decodeWith value v).map fun v => (k, v)
          | _ => none
        | _ => none).bind fun decoded =>
        if bound : decoded.length < 2 ^ 64 then some ⟨decoded, bound⟩ else none
  | .ghost slot, .ghost contents => NTy.decodeWith slot contents
  | .ref referent, .tuple values => match values.toList with
    | [current, prophecy] =>
        (NTy.decodeWith referent current).bind fun current =>
          (NTy.decodeWith referent prophecy).map fun prophecy => (current, prophecy)
    | _ => none
  | .param _, raw => if plain : Plain raw then some ⟨raw, plain⟩ else none
  | .function parameters shared results, raw =>
      (ClosureValue.decode? raw).bind (decode parameters shared results)
  | _, _ => none

/-- The values of a row runtime values encode, in order. -/
def NRow.decodeWith : (row : NRow) → List RuntimeValue → Option (HListWith C row)
  | .nil, [] => some ()
  | .cons τ rest, raw :: raws =>
      (NTy.decodeWith τ raw).bind fun value =>
        (NRow.decodeWith rest raws).map fun values => (value, values)
  | _, _ => none

/-- The variant value of a variant's name and fields. -/
def NRows.decodeWith : (names : List String) → (rows : NRows) → String → List RuntimeValue →
    Option (variantCarrierWith C names rows)
  | _, .nil, _, _ => none
  | [], .cons _ _, _, _ => none
  | first :: names, .cons fields rest, name, raws =>
      if name = first then (NRow.decodeWith fields raws).map Sum.inl
      else (NRows.decodeWith names rest name raws).map Sum.inr
end

end Decode

end Codecs

section Sizes

theorem RuntimeValue.sizeOf_lt_tuple {values : Array RuntimeValue} {raw : RuntimeValue}
    (member : raw ∈ values) : sizeOf raw < sizeOf (RuntimeValue.tuple values) := by
  have := Array.sizeOf_lt_of_mem member
  simp only [RuntimeValue.tuple.sizeOf_spec]
  omega

theorem RuntimeValue.sizeOf_lt_vector {values : Array RuntimeValue} {raw : RuntimeValue}
    (member : raw ∈ values) : sizeOf raw < sizeOf (RuntimeValue.vector values) := by
  have := Array.sizeOf_lt_of_mem member
  simp only [RuntimeValue.vector.sizeOf_spec]
  omega

theorem RuntimeValue.sizeOf_lt_nominal {source : StructHandle} {variant : Option String}
    {values : Array RuntimeValue} {raw : RuntimeValue} (member : raw ∈ values) :
    sizeOf raw < sizeOf (RuntimeValue.nominal source variant values) := by
  have := Array.sizeOf_lt_of_mem member
  simp only [RuntimeValue.nominal.sizeOf_spec]
  omega

theorem RuntimeValue.sizeOf_lt_closure {function : FunctionHandle} {mask : Nat}
    {typeInstantiation : Array (TypeId × TypeId)} {values : Array RuntimeValue} {raw : RuntimeValue}
    (member : raw ∈ values) :
    sizeOf raw < sizeOf (RuntimeValue.closure function mask typeInstantiation values) := by
  have := Array.sizeOf_lt_of_mem member
  simp only [RuntimeValue.closure.sizeOf_spec]
  omega

theorem RuntimeValue.sizeOf_lt_ghost (raw : RuntimeValue) :
    sizeOf raw < sizeOf (RuntimeValue.ghost raw) := by
  simp only [RuntimeValue.ghost.sizeOf_spec]
  omega

/-- A capture is smaller than its closure: the measure a generated decoder
recurses on. -/
theorem ClosureValue.sizeOf_capture_lt (closure : ClosureValue) {raw : RuntimeValue}
    (member : raw ∈ closure.captures.toList) : sizeOf raw < sizeOf closure.encode := by
  have := Array.sizeOf_lt_of_mem (Array.mem_toList_iff.mp member)
  simp only [ClosureValue.encode, RuntimeValue.closure.sizeOf_spec]
  omega

end Sizes

section Lists
variable {α β γ : Type}

theorem List.mapM_map_eq_some {f : α → β} {g : β → Option α} :
    (values : List α) → (∀ value ∈ values, g (f value) = some value) →
      (values.map f).mapM g = some values
  | [], _ => rfl
  | value :: values, round => by
      simp only [List.map_cons, List.mapM_cons, round value List.mem_cons_self,
        List.mapM_map_eq_some values fun other member => round other (List.mem_cons_of_mem _ member),
        Option.bind_eq_bind, Option.bind_some, Option.pure_def]

theorem List.mapM_eq_some_of_forall {g : α → Option α} :
    (values : List α) → (∀ value ∈ values, g value = some value) → values.mapM g = some values
  | [], _ => rfl
  | value :: values, same => by
      simp only [List.mapM_cons, same value List.mem_cons_self,
        List.mapM_eq_some_of_forall values fun other member => same other (List.mem_cons_of_mem _ member),
        Option.bind_eq_bind, Option.bind_some, Option.pure_def]

theorem List.map_eq_of_mapM_eq_some {g : β → Option α} {f : α → β} :
    (raws : List β) → (values : List α) → (∀ raw ∈ raws, ∀ value, g raw = some value → f value = raw) →
      raws.mapM g = some values → values.map f = raws
  | [], values, _, decoded => by
      simp only [List.mapM_nil, Option.pure_def, Option.some.injEq] at decoded
      subst decoded; rfl
  | raw :: raws, values, tight, decoded => by
      simp only [List.mapM_cons, Option.bind_eq_bind, Option.pure_def, Option.bind_eq_some_iff,
        Option.some.injEq] at decoded
      obtain ⟨value, head, rest, tail, rfl⟩ := decoded
      simp only [List.map_cons, tight raw List.mem_cons_self value head,
        List.map_eq_of_mapM_eq_some raws rest
          (fun other member => tight other (List.mem_cons_of_mem _ member)) tail]

theorem List.mapM_congr_mem {g g' : β → Option α} :
    (raws : List β) → (∀ raw ∈ raws, g raw = g' raw) → raws.mapM g = raws.mapM g'
  | [], _ => rfl
  | raw :: raws, agree => by
      simp only [List.mapM_cons, agree raw List.mem_cons_self,
        List.mapM_congr_mem raws fun other member => agree other (List.mem_cons_of_mem _ member)]

theorem List.isSome_mapM_congr {g : β → Option α} {g' : β → Option γ} :
    (raws : List β) → (∀ raw ∈ raws, (g raw).isSome = (g' raw).isSome) →
      (raws.mapM g).isSome = (raws.mapM g').isSome
  | [], _ => rfl
  | raw :: raws, agree => by
      have rest := List.isSome_mapM_congr raws fun other member =>
        agree other (List.mem_cons_of_mem _ member)
      have head := agree raw List.mem_cons_self
      simp only [List.mapM_cons, Option.bind_eq_bind, Option.pure_def]
      cases hg : g raw <;> cases hg' : g' raw <;> simp_all <;>
        cases hr : raws.mapM g <;> cases hr' : raws.mapM g' <;> simp_all

/-- A bounded decoding of a list is defined where the elements' decodings
are and the list is within the bound. -/
theorem List.isSome_mapM_bind_bound {g : β → Option α} (raws : List β) (bound : Nat)
    (k : (values : List α) → values.length < bound → γ) :
    ((raws.mapM g).bind fun values =>
        if h : values.length < bound then some (k values h) else none).isSome =
      ((raws.mapM g).isSome && decide (raws.length < bound)) := by
  cases decoded : raws.mapM g with
  | none => rfl
  | some values =>
      have := List.mapM_length decoded
      by_cases h : values.length < bound
      · simp [h, ← this]
      · simp [h, ← this]

end Lists


section Laws

variable {C : NRow → Sharing → NRow → Type}

section Encode

variable (encode : (parameters : NRow) → (shared : Sharing) → (results : NRow) →
  C parameters shared results → ClosureValue)

mutual
/-- Encodings hold no loan. -/
theorem NTy.plain_encodeWith : (τ : NTy) → (value : τ.carrierWith C) →
    Plain (NTy.encodeWith encode τ value)
  | .unit, _ => .unit
  | .bool, _ => .bool _
  | .int _ _, _ => .integer _
  | .num, _ => .integer _
  | .address, _ => .address _
  | .signer, _ => .signer _
  | .string, _ => .string _
  | .bytes, _ => .bytes _
  | .tuple elements, values => .tuple _ fun raw member =>
      NRow.plain_encodeWith elements values raw (List.mem_toArray.mp member)
  | .struct _ _ fields, values => .nominal _ _ _ fun raw member =>
      NRow.plain_encodeWith fields values raw (List.mem_toArray.mp member)
  | .enum source _ names rows _, value => NRows.plain_encodeWith source names rows value
  | .vector element, values => .vector _ fun raw member => by
      obtain ⟨value, _, rfl⟩ := Array.mem_map.mp member
      exact NTy.plain_encodeWith element value
  | .table key value, entries => .vector _ fun raw member => by
      obtain ⟨entry, _, rfl⟩ := List.mem_map.mp (List.mem_toArray.mp member)
      refine .tuple _ fun raw member => ?_
      simp only [List.mem_toArray, List.mem_cons, List.not_mem_nil, or_false] at member
      rcases member with rfl | rfl
      · exact NTy.plain_encodeWith key entry.1
      · exact NTy.plain_encodeWith value entry.2
  | .ghost slot, value => .ghost _ (NTy.plain_encodeWith slot value)
  | .ref referent, value => .tuple _ fun raw member => by
      simp only [List.mem_toArray, List.mem_cons, List.not_mem_nil, or_false] at member
      rcases member with rfl | rfl
      · exact NTy.plain_encodeWith referent value.1
      · exact NTy.plain_encodeWith referent value.2
  | .param _, value => value.property
  | .function parameters shared results, value =>
      .closure _ _ _ _ (encode parameters shared results value).plain

theorem NRow.plain_encodeWith : (row : NRow) → (values : HListWith C row) →
    ∀ raw ∈ NRow.encodeWith encode row values, Plain raw
  | .nil, _, _, member => nomatch member
  | .cons τ rest, values, raw, member => by
      simp only [NRow.encodeWith, List.mem_cons] at member
      rcases member with rfl | member
      · exact NTy.plain_encodeWith τ values.1
      · exact NRow.plain_encodeWith rest values.2 raw member

theorem NRows.plain_encodeWith (source : StructHandle) : (names : List String) → (rows : NRows) →
    (value : variantCarrierWith C names rows) → Plain (NRows.encodeWith encode source names rows value)
  | _, .nil, value => nomatch value
  | [], .cons _ _, value => nomatch value
  | _ :: _, .cons fields _, .inl values => .nominal _ _ _ fun raw member =>
      NRow.plain_encodeWith fields values raw (List.mem_toArray.mp member)
  | _ :: names, .cons _ rest, .inr value => NRows.plain_encodeWith source names rest value
end

end Encode


section RoundTrip

variable (encode : (parameters : NRow) → (shared : Sharing) → (results : NRow) →
  C parameters shared results → ClosureValue)
  (decode : (parameters : NRow) → (shared : Sharing) → (results : NRow) →
  ClosureValue → Option (C parameters shared results)) (bound : Nat)
  (round : ∀ parameters shared results (value : C parameters shared results),
    sizeOf (encode parameters shared results value).encode < bound →
      decode parameters shared results (encode parameters shared results value) = some value)

include round in
mutual
/-- Decoding an encoding gives the value back, where the function arm does
for the encodings below the bound. -/
theorem NTy.decodeWith_encodeWith : (τ : NTy) → (value : τ.carrierWith C) →
    sizeOf (NTy.encodeWith encode τ value) < bound →
      NTy.decodeWith decode τ (NTy.encodeWith encode τ value) = some value
  | .unit, _, _ => rfl
  | .bool, value, _ => Codec.bool.decode_encode value
  | .int width signed, value, _ => (Codec.specInt (.bits width) signed).decode_encode value
  | .num, value, _ => (Codec.specInt .unbounded true).decode_encode value
  | .address, value, _ => Codec.address.decode_encode value
  | .signer, value, _ => Codec.signer.decode_encode value
  | .string, value, _ => Codec.string.decode_encode value
  | .bytes, value, _ => Codec.bytes.decode_encode value
  | .tuple elements, values, small => by
      simp only [NTy.encodeWith, NTy.decodeWith]
      exact NRow.decodeWith_encodeWith elements values fun raw member =>
        Nat.lt_trans (RuntimeValue.sizeOf_lt_tuple (List.mem_toArray.mpr member)) small
  | .struct source _ fields, values, small => by
      simp only [NTy.encodeWith, NTy.decodeWith, if_true]
      exact NRow.decodeWith_encodeWith fields values fun raw member =>
        Nat.lt_trans (RuntimeValue.sizeOf_lt_nominal (List.mem_toArray.mpr member)) small
  | .enum source _ names rows distinct, value, small => by
      obtain ⟨name, raws, encoded, -, decoded⟩ :=
        NRows.decodeWith_encodeWith source names rows distinct value small
      simp only [NTy.encodeWith]
      rw [encoded]
      simpa only [NTy.decodeWith, if_true] using decoded
  | .vector element, values, small => by
      simp only [NTy.encodeWith, NTy.decodeWith, Array.toList_map]
      rw [List.mapM_map_eq_some _ fun value member => NTy.decodeWith_encodeWith element value
        (Nat.lt_trans (RuntimeValue.sizeOf_lt_vector (Array.mem_map_of_mem
          (Array.mem_toList_iff.mp member))) small)]
      simp only [Option.bind_some, Array.length_toList, values.bounded, dif_pos, Array.toArray_toList]
  | .table key value, entries, small => by
      simp only [NTy.encodeWith] at small
      have each : ∀ entry ∈ entries.entries,
          sizeOf (NTy.encodeWith encode key entry.1) < bound ∧
            sizeOf (NTy.encodeWith encode value entry.2) < bound := by
        intro entry member
        have outer := Nat.lt_trans (RuntimeValue.sizeOf_lt_vector (List.mem_toArray.mpr
          (List.mem_map_of_mem (f := fun entry : _ × _ => RuntimeValue.tuple
            #[NTy.encodeWith encode key entry.1, NTy.encodeWith encode value entry.2]) member))) small
        exact ⟨Nat.lt_trans (RuntimeValue.sizeOf_lt_tuple (by simp)) outer,
          Nat.lt_trans (RuntimeValue.sizeOf_lt_tuple (by simp)) outer⟩
      simp only [NTy.encodeWith, NTy.decodeWith, List.mapM_map]
      rw [List.mapM_eq_some_of_forall _ fun entry member => by
        simp only [Function.comp_apply]
        rw [NTy.decodeWith_encodeWith key entry.1 (each entry member).1,
          NTy.decodeWith_encodeWith value entry.2 (each entry member).2]
        rfl]
      simp only [Option.bind_some, entries.bounded, dif_pos]
  | .ghost slot, value, small => by
      simp only [NTy.encodeWith, NTy.decodeWith]
      exact NTy.decodeWith_encodeWith slot value
        (Nat.lt_trans (RuntimeValue.sizeOf_lt_ghost _) small)
  | .ref referent, value, small => by
      simp only [NTy.encodeWith, NTy.decodeWith]
      rw [NTy.decodeWith_encodeWith referent value.1
          (Nat.lt_trans (RuntimeValue.sizeOf_lt_tuple (by simp)) small),
        NTy.decodeWith_encodeWith referent value.2
          (Nat.lt_trans (RuntimeValue.sizeOf_lt_tuple (by simp)) small)]
      rfl
  | .param _, value, _ => by
      simp only [NTy.encodeWith, NTy.decodeWith, dif_pos value.property]
  | .function parameters shared results, value, small => by
      simp only [NTy.encodeWith, NTy.decodeWith]
      rw [show ClosureValue.decode? (encode parameters shared results value).encode =
          some (encode parameters shared results value) from Codec.closure.decode_encode _]
      exact round parameters shared results value small

theorem NRow.decodeWith_encodeWith : (row : NRow) → (values : HListWith C row) →
    (∀ raw ∈ NRow.encodeWith encode row values, sizeOf raw < bound) →
      NRow.decodeWith decode row (NRow.encodeWith encode row values) = some values
  | .nil, (), _ => rfl
  | .cons τ rest, values, small => by
      simp only [NRow.encodeWith, NRow.decodeWith]
      rw [NTy.decodeWith_encodeWith τ values.1 (small _ List.mem_cons_self),
        NRow.decodeWith_encodeWith rest values.2 fun raw member =>
          small raw (List.mem_cons_of_mem _ member)]
      rfl

theorem NRows.decodeWith_encodeWith (source : StructHandle) : (names : List String) →
    (rows : NRows) → names.Nodup → (value : variantCarrierWith C names rows) →
    sizeOf (NRows.encodeWith encode source names rows value) < bound →
      ∃ name raws, NRows.encodeWith encode source names rows value =
          .nominal source (some name) raws ∧ name ∈ names ∧
        NRows.decodeWith decode names rows name raws.toList = some value
  | _, .nil, _, value, _ => nomatch value
  | [], .cons _ _, _, value, _ => nomatch value
  | name :: names, .cons fields rest, _, .inl values, small => by
      refine ⟨name, _, rfl, List.mem_cons_self, ?_⟩
      simp only [NRows.decodeWith, if_true]
      rw [NRow.decodeWith_encodeWith fields values fun raw member =>
        Nat.lt_trans (RuntimeValue.sizeOf_lt_nominal (List.mem_toArray.mpr member)) small]
      rfl
  | first :: names, .cons fields rest, distinct, .inr value, small => by
      obtain ⟨name, raws, encoded, member, decoded⟩ := NRows.decodeWith_encodeWith source names rest
        (List.nodup_cons.mp distinct).2 value small
      have different : name ≠ first := fun same => (List.nodup_cons.mp distinct).1 (same ▸ member)
      refine ⟨name, raws, encoded, List.mem_cons_of_mem _ member, ?_⟩
      simp only [NRows.decodeWith, different, if_false, decoded, Option.map_some]
end

end RoundTrip


section Tight

variable (encode : (parameters : NRow) → (shared : Sharing) → (results : NRow) →
  C parameters shared results → ClosureValue)
  (decode : (parameters : NRow) → (shared : Sharing) → (results : NRow) →
  ClosureValue → Option (C parameters shared results)) (bound : Nat)
  (tight : ∀ parameters shared results (closure : ClosureValue) (value : C parameters shared results),
    sizeOf closure.encode < bound → decode parameters shared results closure = some value →
      encode parameters shared results value = closure)

include tight in
mutual
/-- A runtime value that decodes is the encoding of its decoding, where the
function arm's are below the bound. -/
theorem NTy.encodeWith_of_decodeWith : (τ : NTy) → (raw : RuntimeValue) → sizeOf raw < bound →
    (value : τ.carrierWith C) → NTy.decodeWith decode τ raw = some value →
      NTy.encodeWith encode τ value = raw
  | .unit, raw, _, value, decoded => unit_tight raw value decoded
  | .bool, raw, _, value, decoded => bool_tight raw value decoded
  | .int width signed, raw, _, value, decoded => specInt_tight (.bits width) signed raw value decoded
  | .num, raw, _, value, decoded => specInt_tight .unbounded true raw value decoded
  | .address, raw, _, value, decoded => address_tight raw value decoded
  | .signer, raw, _, value, decoded => signer_tight raw value decoded
  | .string, raw, _, value, decoded => string_tight raw value decoded
  | .bytes, raw, _, value, decoded => bytes_tight raw value decoded
  | .tuple elements, raw, small, values, decoded => by
      cases raw <;> (try simp only [NTy.decodeWith, reduceCtorEq] at decoded)
      case tuple raws =>
        simp only [NTy.encodeWith]
        rw [NRow.encodeWith_of_decodeWith elements raws.toList (fun raw member =>
          Nat.lt_trans (RuntimeValue.sizeOf_lt_tuple (Array.mem_toList_iff.mp member)) small)
          values decoded, Array.toArray_toList]
  | .struct source _ fields, raw, small, values, decoded => by
      cases raw <;> (try simp only [NTy.decodeWith, reduceCtorEq] at decoded)
      case nominal actual variant raws =>
        cases variant <;> (try simp only [NTy.decodeWith, reduceCtorEq] at decoded)
        split at decoded
        · next same =>
            subst same
            simp only [NTy.encodeWith]
            rw [NRow.encodeWith_of_decodeWith fields raws.toList (fun raw member =>
              Nat.lt_trans (RuntimeValue.sizeOf_lt_nominal (Array.mem_toList_iff.mp member)) small)
              values decoded, Array.toArray_toList]
        · cases decoded
  | .enum source _ names rows _, raw, small, value, decoded => by
      cases raw <;> (try simp only [NTy.decodeWith, reduceCtorEq] at decoded)
      case nominal actual variant raws =>
        cases variant <;> (try simp only [NTy.decodeWith, reduceCtorEq] at decoded)
        case some name =>
          split at decoded
          · next same =>
              subst same
              simp only [NTy.encodeWith]
              rw [NRows.encodeWith_of_decodeWith actual names rows name raws.toList (fun raw member =>
                Nat.lt_trans (RuntimeValue.sizeOf_lt_nominal (Array.mem_toList_iff.mp member)) small)
                value decoded, Array.toArray_toList]
          · cases decoded
  | .vector element, raw, small, values, decoded => by
      cases raw <;> (try simp only [NTy.decodeWith, reduceCtorEq] at decoded)
      case vector raws =>
        simp only [Option.bind_eq_some_iff] at decoded
        obtain ⟨list, mapped, rest⟩ := decoded
        split at rest
        · cases rest
          simp only [NTy.encodeWith]
          congr 1
          apply Array.toList_inj.mp
          rw [Array.toList_map, List.toList_toArray]
          exact List.map_eq_of_mapM_eq_some raws.toList list (fun raw member value decoded =>
            NTy.encodeWith_of_decodeWith element raw (Nat.lt_trans (RuntimeValue.sizeOf_lt_vector
              (Array.mem_toList_iff.mp member)) small) value decoded) mapped
        · cases rest
  | .table key value, raw, small, entries, decoded => by
      cases raw <;> (try simp only [NTy.decodeWith, reduceCtorEq] at decoded)
      case vector raws =>
        simp only [Option.bind_eq_some_iff] at decoded
        obtain ⟨list, mapped, rest⟩ := decoded
        split at rest
        · cases rest
          simp only [NTy.encodeWith]
          congr 1
          apply Array.toList_inj.mp
          rw [List.toList_toArray]
          refine List.map_eq_of_mapM_eq_some raws.toList list (fun entry member pair decoded => ?_) mapped
          have outer := Nat.lt_trans (RuntimeValue.sizeOf_lt_vector
            (Array.mem_toList_iff.mp member)) small
          cases entry <;> simp only [reduceCtorEq] at decoded
          case tuple items =>
            split at decoded
            · next k v listed =>
                have hk : k ∈ items := Array.mem_toList_iff.mp (by simp [listed])
                have hv : v ∈ items := Array.mem_toList_iff.mp (by simp [listed])
                simp only [Option.bind_eq_some_iff, Option.map_eq_some_iff] at decoded
                obtain ⟨dk, kd, dv, vd, rfl⟩ := decoded
                rw [NTy.encodeWith_of_decodeWith key k
                    (Nat.lt_trans (RuntimeValue.sizeOf_lt_tuple hk) outer) dk kd,
                  NTy.encodeWith_of_decodeWith value v
                    (Nat.lt_trans (RuntimeValue.sizeOf_lt_tuple hv) outer) dv vd]
                congr 1
                apply Array.toList_inj.mp
                rw [listed]
                try rfl
            · cases decoded
        · cases rest
  | .ghost slot, raw, small, value, decoded => by
      cases raw <;> (try simp only [NTy.decodeWith, reduceCtorEq] at decoded)
      case ghost contents =>
        simp only [NTy.encodeWith]
        rw [NTy.encodeWith_of_decodeWith slot contents
          (Nat.lt_trans (RuntimeValue.sizeOf_lt_ghost _) small) value decoded]
  | .ref referent, raw, small, value, decoded => by
      cases raw <;> (try simp only [NTy.decodeWith, reduceCtorEq] at decoded)
      case tuple items =>
        split at decoded
        · next current prophecy listed =>
            have hc : current ∈ items := Array.mem_toList_iff.mp (by simp [listed])
            have hp : prophecy ∈ items := Array.mem_toList_iff.mp (by simp [listed])
            simp only [Option.bind_eq_some_iff, Option.map_eq_some_iff] at decoded
            obtain ⟨dc, cd, dp, pd, rfl⟩ := decoded
            simp only [NTy.encodeWith]
            rw [NTy.encodeWith_of_decodeWith referent current
                (Nat.lt_trans (RuntimeValue.sizeOf_lt_tuple hc) small) dc cd,
              NTy.encodeWith_of_decodeWith referent prophecy
                (Nat.lt_trans (RuntimeValue.sizeOf_lt_tuple hp) small) dp pd]
            congr 1
            apply Array.toList_inj.mp
            rw [listed]
            try rfl
        · cases decoded
  | .param _, raw, _, value, decoded => by
      simp only [NTy.decodeWith] at decoded
      split at decoded
      · cases decoded; rfl
      · cases decoded
  | .function parameters shared results, raw, small, value, decoded => by
      simp only [NTy.decodeWith, Option.bind_eq_some_iff] at decoded
      obtain ⟨closure, read, typed⟩ := decoded
      have encoded := Codec.closure_tight raw closure read
      simp only [NTy.encodeWith]
      rw [tight parameters shared results closure value
        (by rw [show closure.encode = raw from encoded]; exact small) typed]
      exact encoded

theorem NRow.encodeWith_of_decodeWith : (row : NRow) → (raws : List RuntimeValue) →
    (∀ raw ∈ raws, sizeOf raw < bound) → (values : HListWith C row) →
      NRow.decodeWith decode row raws = some values → NRow.encodeWith encode row values = raws
  | .nil, [], _, _, _ => rfl
  | .nil, _ :: _, _, _, decoded => by simp [NRow.decodeWith] at decoded
  | .cons _ _, [], _, _, decoded => by simp [NRow.decodeWith] at decoded
  | .cons τ rest, raw :: raws, small, values, decoded => by
      simp only [NRow.decodeWith, Option.bind_eq_some_iff, Option.map_eq_some_iff] at decoded
      obtain ⟨head, hd, tail, td, rfl⟩ := decoded
      simp only [NRow.encodeWith]
      rw [NTy.encodeWith_of_decodeWith τ raw (small raw List.mem_cons_self) head hd,
        NRow.encodeWith_of_decodeWith rest raws (fun other member =>
          small other (List.mem_cons_of_mem _ member)) tail td]

theorem NRows.encodeWith_of_decodeWith (source : StructHandle) : (names : List String) →
    (rows : NRows) → (name : String) → (raws : List RuntimeValue) →
    (∀ raw ∈ raws, sizeOf raw < bound) → (value : variantCarrierWith C names rows) →
      NRows.decodeWith decode names rows name raws = some value →
        NRows.encodeWith encode source names rows value = .nominal source (some name) raws.toArray
  | _, .nil, _, _, _, _, decoded => by simp [NRows.decodeWith] at decoded
  | [], .cons _ _, _, _, _, _, decoded => by simp [NRows.decodeWith] at decoded
  | first :: names, .cons fields rest, name, raws, small, value, decoded => by
      simp only [NRows.decodeWith] at decoded
      split at decoded
      · next same =>
          subst same
          simp only [Option.map_eq_some_iff] at decoded
          obtain ⟨values, vd, rfl⟩ := decoded
          simp only [NRows.encodeWith]
          rw [NRow.encodeWith_of_decodeWith fields raws small values vd]
      · simp only [Option.map_eq_some_iff] at decoded
        obtain ⟨later, ld, rfl⟩ := decoded
        simp only [NRows.encodeWith]
        exact NRows.encodeWith_of_decodeWith source names rest name raws small later ld
end

end Tight


section Admits

theorem NRows.mem_of_variant? : (names : List String) → (rows : NRows) → {name : String} →
    {row : NRow} → NRows.variant? names rows name = some row → name ∈ names
  | [], _, _, _, selected => by simp [NRows.variant?] at selected
  | _ :: _, .nil, _, _, selected => by simp [NRows.variant?] at selected
  | candidate :: names, .cons _ rows, name, _, selected => by
      simp only [NRows.variant?] at selected
      split at selected
      · next same => simp only [beq_iff_eq] at same; simp [same]
      · exact List.mem_cons_of_mem _ (NRows.mem_of_variant? names rows selected)

variable (unit : Validation.ValidatedUnit)
  (encode : (parameters : NRow) → (shared : Sharing) → (results : NRow) →
  C parameters shared results → ClosureValue) (bound : Nat)
  (admitted : ∀ parameters shared results (value : C parameters shared results),
    sizeOf (encode parameters shared results value).encode < bound →
      NTy.admits unit (.function parameters shared results)
        (encode parameters shared results value).encode = true)

include admitted in
mutual
/-- An encoding admits its type, where the function arm's below the bound
admit theirs. -/
theorem NTy.admits_encodeWith : (τ : NTy) → (value : τ.carrierWith C) →
    sizeOf (NTy.encodeWith encode τ value) < bound →
      NTy.admits unit τ (NTy.encodeWith encode τ value) = true
  | .unit, _, _ => by simp [NTy.encodeWith, NTy.admits]
  | .bool, _, _ => by simp [NTy.encodeWith, NTy.admits]
  | .int width signed, value, _ => by simp [NTy.encodeWith, NTy.admits, decodeInt?_val]
  | .num, _, _ => by simp [NTy.encodeWith, NTy.admits]
  | .address, _, _ => by simp [NTy.encodeWith, NTy.admits]
  | .signer, _, _ => by simp [NTy.encodeWith, NTy.admits]
  | .string, _, _ => by simp [NTy.encodeWith, NTy.admits]
  | .bytes, _, _ => by simp [NTy.encodeWith, NTy.admits]
  | .tuple elements, values, small => by
      simp only [NTy.encodeWith, NTy.admits]
      exact NRow.admits_encodeWith elements values fun raw member =>
        Nat.lt_trans (RuntimeValue.sizeOf_lt_tuple (List.mem_toArray.mpr member)) small
  | .struct source _ fields, values, small => by
      simp only [NTy.encodeWith, NTy.admits, beq_self_eq_true, Bool.true_and]
      exact NRow.admits_encodeWith fields values fun raw member =>
        Nat.lt_trans (RuntimeValue.sizeOf_lt_nominal (List.mem_toArray.mpr member)) small
  | .enum source _ names rows distinct, value, small => by
      obtain ⟨name, raws, encoded, row, selected, rowAdmitted⟩ :=
        NRows.admits_encodeWith source names rows distinct value small
      simp only [NTy.encodeWith]
      rw [encoded]
      simp only [NTy.admits, beq_self_eq_true, selected, rowAdmitted, Bool.true_and]
  | .vector element, values, small => by
      simp only [NTy.encodeWith, NTy.admits, Array.toList_map]
      exact @NTy.admitsEach_of_forall unit (FunctionCarriers.runtime unit) _ _ fun raw member => by
        obtain ⟨item, itemMember, rfl⟩ := List.mem_map.mp member
        exact NTy.admits_encodeWith element item (Nat.lt_trans (RuntimeValue.sizeOf_lt_vector
          (Array.mem_map_of_mem (Array.mem_toList_iff.mp itemMember))) small)
  | .table key value, entries, small => by
      simp only [NTy.encodeWith] at small ⊢
      simp only [NTy.admits]
      have each : ∀ entry ∈ entries.entries,
          NTy.admits unit key (NTy.encodeWith encode key entry.1) = true ∧
            NTy.admits unit value (NTy.encodeWith encode value entry.2) = true := by
        intro entry member
        have outer := Nat.lt_trans (RuntimeValue.sizeOf_lt_vector (List.mem_toArray.mpr
          (List.mem_map_of_mem (f := fun entry : _ × _ => RuntimeValue.tuple
            #[NTy.encodeWith encode key entry.1, NTy.encodeWith encode value entry.2]) member))) small
        exact ⟨NTy.admits_encodeWith key entry.1
            (Nat.lt_trans (RuntimeValue.sizeOf_lt_tuple (by simp)) outer),
          NTy.admits_encodeWith value entry.2
            (Nat.lt_trans (RuntimeValue.sizeOf_lt_tuple (by simp)) outer)⟩
      clear small
      revert each
      generalize entries.entries = list
      intro each
      induction list with
      | nil => simp [NTy.admitsEntries]
      | cons entry rest ih =>
          simp only [List.map_cons, NTy.admitsEntries, NTy.admitsEntry,
            (each entry List.mem_cons_self).1, (each entry List.mem_cons_self).2, Bool.and_self,
            Bool.true_and]
          exact ih fun other member => each other (List.mem_cons_of_mem _ member)
  | .ghost slot, value, _ => by
      simp only [NTy.encodeWith, NTy.admits]
      exact (SemanticOperations.plain?_iff _).mpr (NTy.plain_encodeWith encode slot value)
  | .ref referent, value, small => by
      simp only [NTy.encodeWith, NTy.admits, NTy.admitsPair]
      rw [NTy.admits_encodeWith referent value.1
          (Nat.lt_trans (RuntimeValue.sizeOf_lt_tuple (by simp)) small),
        NTy.admits_encodeWith referent value.2
          (Nat.lt_trans (RuntimeValue.sizeOf_lt_tuple (by simp)) small)]
      rfl
  | .param _, _, _ => by simp [NTy.admits]
  | .function parameters shared results, value, small => admitted parameters shared results value small

theorem NRow.admits_encodeWith : (row : NRow) → (values : HListWith C row) →
    (∀ raw ∈ NRow.encodeWith encode row values, sizeOf raw < bound) →
      NRow.admits unit row (NRow.encodeWith encode row values) = true
  | .nil, _, _ => by simp [NRow.encodeWith, NRow.admits]
  | .cons τ rest, values, small => by
      simp only [NRow.encodeWith, NRow.admits]
      rw [NTy.admits_encodeWith τ values.1 (small _ List.mem_cons_self),
        NRow.admits_encodeWith rest values.2 fun raw member =>
          small raw (List.mem_cons_of_mem _ member)]
      rfl

theorem NRows.admits_encodeWith (source : StructHandle) : (names : List String) →
    (rows : NRows) → names.Nodup → (value : variantCarrierWith C names rows) →
    sizeOf (NRows.encodeWith encode source names rows value) < bound →
      ∃ name raws, NRows.encodeWith encode source names rows value =
          .nominal source (some name) raws ∧
        ∃ row, NRows.variant? names rows name = some row ∧
          NRow.admits unit row raws.toList = true
  | _, .nil, _, value, _ => nomatch value
  | [], .cons _ _, _, value, _ => nomatch value
  | name :: names, .cons fields rest, _, .inl values, small => by
      refine ⟨name, _, rfl, fields, by simp [NRows.variant?], ?_⟩
      rw [List.toList_toArray]
      exact NRow.admits_encodeWith fields values fun raw member =>
        Nat.lt_trans (RuntimeValue.sizeOf_lt_nominal (List.mem_toArray.mpr member)) small
  | first :: names, .cons fields rest, distinct, .inr value, small => by
      obtain ⟨name, raws, encoded, row, selected, rowAdmitted⟩ := NRows.admits_encodeWith source
        names rest (List.nodup_cons.mp distinct).2 value small
      have member : name ∈ names := NRows.mem_of_variant? names rest selected
      have different : (first == name) = false := by
        simp only [beq_eq_false_iff_ne]
        rintro rfl
        exact (List.nodup_cons.mp distinct).1 member
      exact ⟨name, raws, encoded, row, by simp [NRows.variant?, different, selected], rowAdmitted⟩
end

end Admits


section Congruence

variable (decode decode' : (parameters : NRow) → (shared : Sharing) → (results : NRow) →
  ClosureValue → Option (C parameters shared results)) (bound : Nat)
  (agree : ∀ parameters shared results (closure : ClosureValue), sizeOf closure.encode < bound →
    decode parameters shared results closure = decode' parameters shared results closure)

include agree in
mutual
/-- Decoding reads the function arm only at the closures inside the value:
those below a bound the value is below. A decoder recursing through its
captures is unfolded by it. -/
theorem NTy.decodeWith_congr : (τ : NTy) → (raw : RuntimeValue) → sizeOf raw < bound →
    NTy.decodeWith decode τ raw = NTy.decodeWith decode' τ raw
  | .unit, _, _ | .bool, _, _ | .int _ _, _, _ | .num, _, _ | .address, _, _ | .signer, _, _
  | .string, _, _ | .bytes, _, _ | .param _, _, _ => rfl
  | .tuple elements, raw, small => by
      cases raw <;> try rfl
      case tuple raws =>
        simp only [NTy.decodeWith]
        exact NRow.decodeWith_congr elements raws.toList fun raw member =>
          Nat.lt_trans (RuntimeValue.sizeOf_lt_tuple (Array.mem_toList_iff.mp member)) small
  | .struct source _ fields, raw, small => by
      cases raw <;> try rfl
      case nominal actual variant raws =>
        cases variant <;> try rfl
        simp only [NTy.decodeWith]
        rw [NRow.decodeWith_congr fields raws.toList fun raw member =>
          Nat.lt_trans (RuntimeValue.sizeOf_lt_nominal (Array.mem_toList_iff.mp member)) small]
  | .enum source _ names rows _, raw, small => by
      cases raw <;> try rfl
      case nominal actual variant raws =>
        cases variant <;> try rfl
        case some name =>
          simp only [NTy.decodeWith]
          rw [NRows.decodeWith_congr names rows name raws.toList fun raw member =>
            Nat.lt_trans (RuntimeValue.sizeOf_lt_nominal (Array.mem_toList_iff.mp member)) small]
  | .vector element, raw, small => by
      cases raw <;> try rfl
      case vector raws =>
        simp only [NTy.decodeWith]
        rw [List.mapM_congr_mem _ fun raw member => NTy.decodeWith_congr element raw
          (Nat.lt_trans (RuntimeValue.sizeOf_lt_vector (Array.mem_toList_iff.mp member)) small)]
  | .table key value, raw, small => by
      cases raw <;> try rfl
      case vector raws =>
        simp only [NTy.decodeWith]
        congr 1
        apply List.mapM_congr_mem
        intro entry member
        have outer := Nat.lt_trans (RuntimeValue.sizeOf_lt_vector
          (Array.mem_toList_iff.mp member)) small
        cases entry <;> try rfl
        case tuple items =>
          simp only
          split
          · next k v listed =>
              have hk : k ∈ items := Array.mem_toList_iff.mp (by simp [listed])
              have hv : v ∈ items := Array.mem_toList_iff.mp (by simp [listed])
              rw [NTy.decodeWith_congr key k (Nat.lt_trans (RuntimeValue.sizeOf_lt_tuple hk) outer),
                NTy.decodeWith_congr value v (Nat.lt_trans (RuntimeValue.sizeOf_lt_tuple hv) outer)]
          · rfl
  | .ghost slot, raw, small => by
      cases raw <;> try rfl
      case ghost contents =>
        exact NTy.decodeWith_congr slot contents (Nat.lt_trans (RuntimeValue.sizeOf_lt_ghost _) small)
  | .ref referent, raw, small => by
      cases raw <;> try rfl
      case tuple items =>
        simp only [NTy.decodeWith]
        split
        · next current prophecy listed =>
            have hc : current ∈ items := Array.mem_toList_iff.mp (by simp [listed])
            have hp : prophecy ∈ items := Array.mem_toList_iff.mp (by simp [listed])
            rw [NTy.decodeWith_congr referent current
                (Nat.lt_trans (RuntimeValue.sizeOf_lt_tuple hc) small),
              NTy.decodeWith_congr referent prophecy
                (Nat.lt_trans (RuntimeValue.sizeOf_lt_tuple hp) small)]
        · rfl
  | .function parameters shared results, raw, small => by
      simp only [NTy.decodeWith]
      cases read : ClosureValue.decode? raw with
      | none => rfl
      | some closure =>
          have encoded := Codec.closure_tight raw closure read
          exact agree parameters shared results closure
            (by rw [show closure.encode = raw from encoded]; exact small)

theorem NRow.decodeWith_congr : (row : NRow) → (raws : List RuntimeValue) →
    (∀ raw ∈ raws, sizeOf raw < bound) →
      NRow.decodeWith decode row raws = NRow.decodeWith decode' row raws
  | .nil, [], _ => rfl
  | .nil, _ :: _, _ => rfl
  | .cons _ _, [], _ => rfl
  | .cons τ rest, raw :: raws, small => by
      simp only [NRow.decodeWith]
      rw [NTy.decodeWith_congr τ raw (small raw List.mem_cons_self),
        NRow.decodeWith_congr rest raws fun other member =>
          small other (List.mem_cons_of_mem _ member)]

theorem NRows.decodeWith_congr : (names : List String) → (rows : NRows) → (name : String) →
    (raws : List RuntimeValue) → (∀ raw ∈ raws, sizeOf raw < bound) →
      NRows.decodeWith decode names rows name raws = NRows.decodeWith decode' names rows name raws
  | _, .nil, _, _, _ => rfl
  | [], .cons _ _, _, _, _ => rfl
  | first :: names, .cons fields rest, name, raws, small => by
      simp only [NRows.decodeWith]
      rw [NRow.decodeWith_congr fields raws small,
        NRows.decodeWith_congr names rest name raws small]
end

end Congruence


section Shape

variable {C' : NRow → Sharing → NRow → Type}
  (decode : (parameters : NRow) → (shared : Sharing) → (results : NRow) →
  ClosureValue → Option (C parameters shared results))
  (decode' : (parameters : NRow) → (shared : Sharing) → (results : NRow) →
  ClosureValue → Option (C' parameters shared results)) (bound : Nat)
  (agree : ∀ parameters shared results (closure : ClosureValue), sizeOf closure.encode < bound →
    (decode parameters shared results closure).isSome =
      (decode' parameters shared results closure).isSome)

include agree in
mutual
/-- Whether a runtime value decodes depends on the function arm only
through whether the closures inside it decode. -/
theorem NTy.isSome_decodeWith : (τ : NTy) → (raw : RuntimeValue) → sizeOf raw < bound →
    (NTy.decodeWith decode τ raw).isSome = (NTy.decodeWith decode' τ raw).isSome
  | .unit, _, _ | .bool, _, _ | .int _ _, _, _ | .num, _, _ | .address, _, _ | .signer, _, _
  | .string, _, _ | .bytes, _, _ | .param _, _, _ => rfl
  | .tuple elements, raw, small => by
      cases raw <;> try rfl
      case tuple raws =>
        simp only [NTy.decodeWith]
        exact NRow.isSome_decodeWith elements raws.toList fun raw member =>
          Nat.lt_trans (RuntimeValue.sizeOf_lt_tuple (Array.mem_toList_iff.mp member)) small
  | .struct source _ fields, raw, small => by
      cases raw <;> try rfl
      case nominal actual variant raws =>
        cases variant <;> try rfl
        simp only [NTy.decodeWith]
        split
        · exact NRow.isSome_decodeWith fields raws.toList fun raw member =>
            Nat.lt_trans (RuntimeValue.sizeOf_lt_nominal (Array.mem_toList_iff.mp member)) small
        · rfl
  | .enum source _ names rows _, raw, small => by
      cases raw <;> try rfl
      case nominal actual variant raws =>
        cases variant <;> try rfl
        case some name =>
          simp only [NTy.decodeWith]
          split
          · exact NRows.isSome_decodeWith names rows name raws.toList fun raw member =>
              Nat.lt_trans (RuntimeValue.sizeOf_lt_nominal (Array.mem_toList_iff.mp member)) small
          · rfl
  | .vector element, raw, small => by
      cases raw <;> try rfl
      case vector raws =>
        simp only [NTy.decodeWith]
        rw [List.isSome_mapM_bind_bound, List.isSome_mapM_bind_bound,
          List.isSome_mapM_congr (g' := NTy.decodeWith decode' element) _ fun raw member =>
            NTy.isSome_decodeWith element raw
              (Nat.lt_trans (RuntimeValue.sizeOf_lt_vector (Array.mem_toList_iff.mp member)) small)]
  | .table key value, raw, small => by
      cases raw <;> try rfl
      case vector raws =>
        simp only [NTy.decodeWith]
        rw [List.isSome_mapM_bind_bound, List.isSome_mapM_bind_bound]
        congr 1
        apply List.isSome_mapM_congr
        intro entry member
        have outer := Nat.lt_trans (RuntimeValue.sizeOf_lt_vector
          (Array.mem_toList_iff.mp member)) small
        cases entry <;> try rfl
        case tuple items =>
          simp only
          split
          · next k v listed =>
              have hk : k ∈ items := Array.mem_toList_iff.mp (by simp [listed])
              have hv : v ∈ items := Array.mem_toList_iff.mp (by simp [listed])
              have sk := NTy.isSome_decodeWith key k (Nat.lt_trans (RuntimeValue.sizeOf_lt_tuple hk) outer)
              have sv := NTy.isSome_decodeWith value v (Nat.lt_trans (RuntimeValue.sizeOf_lt_tuple hv) outer)
              cases h1 : NTy.decodeWith decode key k <;> cases h2 : NTy.decodeWith decode' key k <;>
                cases h3 : NTy.decodeWith decode value v <;> cases h4 : NTy.decodeWith decode' value v <;>
                simp_all
          · rfl
  | .ghost slot, raw, small => by
      cases raw <;> try rfl
      case ghost contents =>
        exact NTy.isSome_decodeWith slot contents (Nat.lt_trans (RuntimeValue.sizeOf_lt_ghost _) small)
  | .ref referent, raw, small => by
      cases raw <;> try rfl
      case tuple items =>
        simp only [NTy.decodeWith]
        split
        · next current prophecy listed =>
            have hc : current ∈ items := Array.mem_toList_iff.mp (by simp [listed])
            have hp : prophecy ∈ items := Array.mem_toList_iff.mp (by simp [listed])
            have sc := NTy.isSome_decodeWith referent current
              (Nat.lt_trans (RuntimeValue.sizeOf_lt_tuple hc) small)
            have sp := NTy.isSome_decodeWith referent prophecy
              (Nat.lt_trans (RuntimeValue.sizeOf_lt_tuple hp) small)
            cases h1 : NTy.decodeWith decode referent current <;>
              cases h2 : NTy.decodeWith decode' referent current <;>
              cases h3 : NTy.decodeWith decode referent prophecy <;>
              cases h4 : NTy.decodeWith decode' referent prophecy <;> simp_all
        · rfl
  | .function parameters shared results, raw, small => by
      simp only [NTy.decodeWith]
      cases read : ClosureValue.decode? raw with
      | none => rfl
      | some closure =>
          have encoded := Codec.closure_tight raw closure read
          exact agree parameters shared results closure
            (by rw [show closure.encode = raw from encoded]; exact small)

theorem NRow.isSome_decodeWith : (row : NRow) → (raws : List RuntimeValue) →
    (∀ raw ∈ raws, sizeOf raw < bound) →
      (NRow.decodeWith decode row raws).isSome = (NRow.decodeWith decode' row raws).isSome
  | .nil, [], _ => rfl
  | .nil, _ :: _, _ => rfl
  | .cons _ _, [], _ => rfl
  | .cons τ rest, raw :: raws, small => by
      simp only [NRow.decodeWith]
      have head := NTy.isSome_decodeWith τ raw (small raw List.mem_cons_self)
      have tail := NRow.isSome_decodeWith rest raws fun other member =>
        small other (List.mem_cons_of_mem _ member)
      cases h1 : NTy.decodeWith decode τ raw <;> cases h2 : NTy.decodeWith decode' τ raw <;>
        cases h3 : NRow.decodeWith decode rest raws <;> cases h4 : NRow.decodeWith decode' rest raws <;>
        simp_all

theorem NRows.isSome_decodeWith : (names : List String) → (rows : NRows) → (name : String) →
    (raws : List RuntimeValue) → (∀ raw ∈ raws, sizeOf raw < bound) →
      (NRows.decodeWith decode names rows name raws).isSome =
        (NRows.decodeWith decode' names rows name raws).isSome
  | _, .nil, _, _, _ => rfl
  | [], .cons _ _, _, _, _ => rfl
  | first :: names, .cons fields rest, name, raws, small => by
      simp only [NRows.decodeWith]
      split
      · simp only [Option.isSome_map]
        exact NRow.isSome_decodeWith fields raws small
      · simp only [Option.isSome_map]
        exact NRows.isSome_decodeWith names rest name raws small
end

end Shape


section FunctionFree

variable (encode encode' : (parameters : NRow) → (shared : Sharing) → (results : NRow) →
  C parameters shared results → ClosureValue)

mutual
/-- A type without function types encodes without the function arm. -/
theorem NTy.encodeWith_closureFree : (τ : NTy) → τ.closureFree = true → (value : τ.carrierWith C) →
    NTy.encodeWith encode τ value = NTy.encodeWith encode' τ value
  | .unit, _, _ | .bool, _, _ | .int _ _, _, _ | .num, _, _ | .address, _, _ | .signer, _, _
  | .string, _, _ | .bytes, _, _ | .param _, _, _ => rfl
  | .tuple elements, free, values => by
      simp only [NTy.encodeWith, NRow.encodeWith_closureFree elements free values]
  | .struct _ _ fields, free, values => by
      simp only [NTy.encodeWith, NRow.encodeWith_closureFree fields free values]
  | .enum source _ names rows _, free, value => by
      simp only [NTy.encodeWith]
      exact NRows.encodeWith_closureFree source names rows free value
  | .vector element, free, values => by
      simp only [NTy.encodeWith]
      congr 1
      exact Array.map_congr_left fun value _ => NTy.encodeWith_closureFree element free value
  | .table key value, free, entries => by
      simp only [NTy.closureFree, Bool.and_eq_true] at free
      simp only [NTy.encodeWith]
      congr 2
      exact List.map_congr_left fun entry _ => by
        rw [NTy.encodeWith_closureFree key free.1 entry.1,
          NTy.encodeWith_closureFree value free.2 entry.2]
  | .ghost slot, free, value => by
      simp only [NTy.encodeWith, NTy.encodeWith_closureFree slot free value]
  | .ref referent, free, value => by
      simp only [NTy.encodeWith, NTy.encodeWith_closureFree referent free value.1,
        NTy.encodeWith_closureFree referent free value.2]
  | .function _ _ _, free, _ => by simp [NTy.closureFree] at free

theorem NRow.encodeWith_closureFree : (row : NRow) → row.closureFree = true →
    (values : HListWith C row) → NRow.encodeWith encode row values = NRow.encodeWith encode' row values
  | .nil, _, _ => rfl
  | .cons τ rest, free, values => by
      simp only [NRow.closureFree, Bool.and_eq_true] at free
      simp only [NRow.encodeWith, NTy.encodeWith_closureFree τ free.1 values.1,
        NRow.encodeWith_closureFree rest free.2 values.2]

theorem NRows.encodeWith_closureFree (source : StructHandle) : (names : List String) →
    (rows : NRows) → rows.closureFree = true → (value : variantCarrierWith C names rows) →
      NRows.encodeWith encode source names rows value = NRows.encodeWith encode' source names rows value
  | _, .nil, _, value => nomatch value
  | [], .cons _ _, _, value => nomatch value
  | _ :: _, .cons fields _, free, .inl values => by
      simp only [NRows.closureFree, Bool.and_eq_true] at free
      simp only [NRows.encodeWith, NRow.encodeWith_closureFree fields free.1 values]
  | _ :: names, .cons _ rest, free, .inr value => by
      simp only [NRows.closureFree, Bool.and_eq_true] at free
      simp only [NRows.encodeWith]
      exact NRows.encodeWith_closureFree source names rest free.2 value
end

variable (decode decode' : (parameters : NRow) → (shared : Sharing) → (results : NRow) →
  ClosureValue → Option (C parameters shared results))

mutual
/-- A type without function types decodes without the function arm. -/
theorem NTy.decodeWith_closureFree : (τ : NTy) → τ.closureFree = true → (raw : RuntimeValue) →
    NTy.decodeWith decode τ raw = NTy.decodeWith decode' τ raw
  | .unit, _, _ | .bool, _, _ | .int _ _, _, _ | .num, _, _ | .address, _, _ | .signer, _, _
  | .string, _, _ | .bytes, _, _ | .param _, _, _ => rfl
  | .tuple elements, free, raw => by
      cases raw <;> try rfl
      case tuple raws =>
        simp only [NTy.decodeWith]
        exact NRow.decodeWith_closureFree elements free raws.toList
  | .struct source _ fields, free, raw => by
      cases raw <;> try rfl
      case nominal actual variant raws =>
        cases variant <;> try rfl
        simp only [NTy.decodeWith, NRow.decodeWith_closureFree fields free raws.toList]
  | .enum source _ names rows _, free, raw => by
      cases raw <;> try rfl
      case nominal actual variant raws =>
        cases variant <;> try rfl
        case some name =>
          simp only [NTy.decodeWith, NRows.decodeWith_closureFree names rows free name raws.toList]
  | .vector element, free, raw => by
      cases raw <;> try rfl
      case vector raws =>
        simp only [NTy.decodeWith]
        rw [List.mapM_congr_mem _ fun raw _ => NTy.decodeWith_closureFree element free raw]
  | .table key value, free, raw => by
      simp only [NTy.closureFree, Bool.and_eq_true] at free
      cases raw <;> try rfl
      case vector raws =>
        simp only [NTy.decodeWith]
        congr 1
        apply List.mapM_congr_mem
        intro entry _
        cases entry <;> try rfl
        case tuple items =>
          simp only
          split
          · rw [NTy.decodeWith_closureFree key free.1, NTy.decodeWith_closureFree value free.2]
          · rfl
  | .ghost slot, free, raw => by
      cases raw <;> try rfl
      case ghost contents => exact NTy.decodeWith_closureFree slot free contents
  | .ref referent, free, raw => by
      cases raw <;> try rfl
      case tuple items =>
        simp only [NTy.decodeWith]
        split
        · rw [NTy.decodeWith_closureFree referent free, NTy.decodeWith_closureFree referent free]
        · rfl
  | .function _ _ _, free, _ => by simp [NTy.closureFree] at free

theorem NRow.decodeWith_closureFree : (row : NRow) → row.closureFree = true →
    (raws : List RuntimeValue) → NRow.decodeWith decode row raws = NRow.decodeWith decode' row raws
  | .nil, _, [] => rfl
  | .nil, _, _ :: _ => rfl
  | .cons _ _, _, [] => rfl
  | .cons τ rest, free, raw :: raws => by
      simp only [NRow.closureFree, Bool.and_eq_true] at free
      simp only [NRow.decodeWith, NTy.decodeWith_closureFree τ free.1 raw,
        NRow.decodeWith_closureFree rest free.2 raws]

theorem NRows.decodeWith_closureFree : (names : List String) → (rows : NRows) →
    rows.closureFree = true → (name : String) → (raws : List RuntimeValue) →
      NRows.decodeWith decode names rows name raws = NRows.decodeWith decode' names rows name raws
  | _, .nil, _, _, _ => rfl
  | [], .cons _ _, _, _, _ => rfl
  | first :: names, .cons fields rest, free, name, raws => by
      simp only [NRows.closureFree, Bool.and_eq_true] at free
      simp only [NRows.decodeWith, NRow.decodeWith_closureFree fields free.1 raws,
        NRows.decodeWith_closureFree names rest free.2 name raws]
end

end FunctionFree

end Laws


/-! ## Closure sites and their family

A link set's function values (`doc/plan/native-function-values.md`):
a constructor per closure site holding the site's captures natively, and
`foreign` holding every other closure the unit types at a function type.
The generated block provides the family, its constructors, and its
encoder (`SiteFamily`); decoding and the laws of `FunctionCarriers` hold
of every such family. -/

/-- A closure site's key: its target, its mask, and the runtime type
instantiation it is created at. -/
structure SiteKey where
  function : FunctionHandle
  mask : Nat
  typeInstantiation : Array (TypeId × TypeId)
  deriving DecidableEq, Repr, Inhabited

/-- Whether a closure is at a key. -/
def SiteKey.matches (key : SiteKey) (closure : ClosureValue) : Bool :=
  decide (key.function = closure.function ∧ key.mask = closure.mask ∧
    key.typeInstantiation = closure.typeInstantiation)

theorem SiteKey.eq_of_matches {left right : SiteKey} {closure : ClosureValue}
    (leftMatches : left.matches closure = true) (rightMatches : right.matches closure = true) :
    left = right := by
  simp only [SiteKey.matches, decide_eq_true_eq] at leftMatches rightMatches
  cases left; cases right
  simp only [SiteKey.mk.injEq]
  exact ⟨leftMatches.1.trans rightMatches.1.symm, leftMatches.2.1.trans rightMatches.2.1.symm,
    leftMatches.2.2.trans rightMatches.2.2.symm⟩

/-- The closure at a key capturing runtime values; a loan among them
leaves no closure of the key. -/
def SiteKey.closure (key : SiteKey) (captures : List RuntimeValue) : ClosureValue :=
  if plain : captures.all SemanticOperations.plain? = true then
    ⟨key.function, key.mask, key.typeInstantiation, captures.toArray, fun capture member => by
      rw [List.all_eq_true] at plain
      exact (SemanticOperations.plain?_iff _).mp (plain capture (List.mem_toArray.mp member))⟩
  else default

theorem SiteKey.closure_of_plain (key : SiteKey) {captures : List RuntimeValue}
    (plain : ∀ capture ∈ captures, Plain capture) :
    key.closure captures =
      ⟨key.function, key.mask, key.typeInstantiation, captures.toArray, fun capture member =>
        plain capture (List.mem_toArray.mp member)⟩ := by
  have checked : captures.all SemanticOperations.plain? = true := List.all_eq_true.mpr
    fun capture member => (SemanticOperations.plain?_iff _).mpr (plain capture member)
  simp only [SiteKey.closure, dif_pos checked]

theorem SiteKey.matches_closure (key : SiteKey) {captures : List RuntimeValue}
    (plain : ∀ capture ∈ captures, Plain capture) : key.matches (key.closure captures) = true := by
  rw [key.closure_of_plain plain]
  simp [SiteKey.matches]

/-- A closure site at closed rows: its key, the function type its closures
are values of, and its captured row. -/
structure SiteSpec where
  key : SiteKey
  parameters : NRow
  shared : Sharing
  results : NRow
  captured : NRow
  deriving Repr, Inhabited

/-- Whether the unit types a site's closures at its function type: its
target's rows at its instantiation are the site's, as are its sharing marks,
and the instantiation is faithful. Decided once per link set. -/
def SiteSpec.typed (unit : Validation.ValidatedUnit) (site : SiteSpec) : Bool :=
  closureFaithful unit site.key.function site.key.typeInstantiation &&
    closureRowsIn? unit site.key.function site.key.mask site.key.typeInstantiation ==
      some (site.captured, site.parameters, site.results) &&
    closureShared unit site.key.function site.key.mask == site.shared

theorem SiteSpec.typed_of_all {unit : Validation.ValidatedUnit} {sites : List SiteSpec}
    (all : sites.all (·.typed unit) = true) (index : Fin sites.length) :
    (sites.get index).typed unit = true :=
  List.all_eq_true.mp all _ (List.getElem_mem index.isLt)

/-- The first site at a closure's key. -/
def siteIndex? (sites : List SiteSpec) (closure : ClosureValue) : Option (Fin sites.length) :=
  (List.finRange sites.length).find? fun index => (sites.get index).key.matches closure

/-- Whether sites have distinct keys, by structural recursion (decided by
the kernel). -/
def sitesDistinct : List SiteSpec → Bool
  | [] => true
  | site :: rest => rest.all (fun other => site.key != other.key) && sitesDistinct rest

theorem sitesDistinct_pairwise : {sites : List SiteSpec} → sitesDistinct sites = true →
    sites.Pairwise (fun left right => left.key ≠ right.key)
  | [], _ => List.Pairwise.nil
  | _ :: _, distinct => by
      simp only [sitesDistinct, Bool.and_eq_true, List.all_eq_true, bne_iff_ne] at distinct
      exact List.Pairwise.cons distinct.1 (sitesDistinct_pairwise distinct.2)

theorem siteIndex?_matches {sites : List SiteSpec} {closure : ClosureValue}
    {index : Fin sites.length} (found : siteIndex? sites closure = some index) :
    (sites.get index).key.matches closure = true :=
  List.find?_some (p := fun index => (sites.get index).key.matches closure)
    (l := List.finRange sites.length) found

theorem siteIndex?_of_matches {sites : List SiteSpec} (distinct : sitesDistinct sites = true)
    {closure : ClosureValue} (index : Fin sites.length)
    (matched : (sites.get index).key.matches closure = true) :
    siteIndex? sites closure = some index := by
  have some : (siteIndex? sites closure).isSome = true :=
    List.find?_isSome.mpr ⟨index, List.mem_finRange index, matched⟩
  obtain ⟨other, found⟩ := Option.isSome_iff_exists.mp some
  have same := SiteKey.eq_of_matches (siteIndex?_matches found) matched
  have pairwise := List.pairwise_iff_getElem.mp (sitesDistinct_pairwise distinct)
  rcases Nat.lt_trichotomy other.val index.val with before | equal | after
  · exact absurd same (pairwise other index other.isLt index.isLt before)
  · rw [found, Fin.ext equal]
  · exact absurd same.symm (pairwise index other index.isLt other.isLt after)

/-- The unit's typing of closures, as a function arm decoding no value. -/
def closureTyping (unit : Validation.ValidatedUnit) (parameters : NRow) (shared : Sharing)
    (results : NRow) (closure : ClosureValue) : Option Unit :=
  if NTy.admits unit (.function parameters shared results) closure.encode then some () else none

/-- Whether a closure's runtime captures decode at a row, the closures among
them typed by the unit. -/
def NRow.decodable (unit : Validation.ValidatedUnit) (row : NRow) (captures : List RuntimeValue) :
    Bool :=
  (NRow.decodeWith (closureTyping unit) row captures).isSome

/-- Whether a closure is a site's encoding at a function type: at a site's
key, of the site's function type, its captures decoding at the site's row. -/
def isSiteWith (unit : Validation.ValidatedUnit) (sites : List SiteSpec) (parameters : NRow)
    (shared : Sharing) (results : NRow) (closure : ClosureValue) : Bool :=
  match siteIndex? sites closure with
  | some index =>
      decide (parameters = (sites.get index).parameters ∧ shared = (sites.get index).shared ∧
        results = (sites.get index).results) &&
        NRow.decodable unit (sites.get index).captured closure.captures.toList
  | none => false

/-- A closure the unit types at a function type that no site encodes: the
values a link set does not enumerate, from parameters and storage. -/
structure ForeignClosure (unit : Validation.ValidatedUnit)
    (isSite : NRow → Sharing → NRow → ClosureValue → Bool) (parameters : NRow) (shared : Sharing)
    (results : NRow) where
  closure : ClosureValue
  admitted : NTy.admits unit (.function parameters shared results) closure.encode = true
  notSite : isSite parameters shared results closure = false

theorem ForeignClosure.ext {unit : Validation.ValidatedUnit}
    {isSite : NRow → Sharing → NRow → ClosureValue → Bool} {parameters : NRow} {shared : Sharing}
    {results : NRow} {left right : ForeignClosure unit isSite parameters shared results}
    (same : left.closure = right.closure) : left = right := by
  cases left; cases right; cases same; rfl

/-- A value of a family at one function type, at another equal to it. -/
@[reducible] def castIndex {F : NRow → Sharing → NRow → Type} {parameters : NRow} {shared : Sharing}
    {results : NRow} {parameters' : NRow} {shared' : Sharing} {results' : NRow}
    (same : parameters = parameters' ∧ shared = shared' ∧ results = results')
    (value : F parameters' shared' results') : F parameters shared results :=
  same.1 ▸ same.2.1 ▸ same.2.2 ▸ value

@[simp] theorem castIndex_rfl {F : NRow → Sharing → NRow → Type} {parameters : NRow}
    {shared : Sharing} {results : NRow} (same : parameters = parameters ∧ shared = shared ∧
      results = results) (value : F parameters shared results) :
    castIndex same value = value := rfl

/-- A link set's function values: a family with a constructor per closure
site and `foreign`, its encoder, and the facts the unit's evaluation
decides of its sites. The generated block provides one. -/
structure SiteFamily (unit : Validation.ValidatedUnit) where
  F : NRow → Sharing → NRow → Type
  sites : List SiteSpec
  site : (index : Fin sites.length) → HListWith F (sites.get index).captured →
    F (sites.get index).parameters (sites.get index).shared (sites.get index).results
  foreign : ∀ {parameters shared results},
    ForeignClosure unit (isSiteWith unit sites) parameters shared results →
      F parameters shared results
  encode : ∀ {parameters shared results}, F parameters shared results → ClosureValue
  encode_site : ∀ index captures, encode (site index captures) =
    (sites.get index).key.closure
      (NRow.encodeWith (fun _ _ _ value => encode value) (sites.get index).captured captures)
  encode_foreign : ∀ {parameters shared results}
    (key : ForeignClosure unit (isSiteWith unit sites) parameters shared results),
    encode (foreign key) = key.closure
  cases : ∀ {parameters shared results} (value : F parameters shared results),
    (∃ index captures, ∃ same : parameters = (sites.get index).parameters ∧
        shared = (sites.get index).shared ∧ results = (sites.get index).results,
      value = castIndex same (site index captures)) ∨
    ∃ key, value = foreign key
  distinct : sitesDistinct sites = true
  typed : ∀ index, (sites.get index).typed unit = true

namespace SiteFamily

variable {unit : Validation.ValidatedUnit} (Φ : SiteFamily unit)

/-- The value a runtime closure is: a site's constructor where it is that
site's encoding, `foreign` where the unit types it otherwise. Recursion is
on the closure's size; a capture's closures are smaller than it. -/
noncomputable def decode (parameters : NRow) (shared : Sharing) (results : NRow) (closure : ClosureValue) :
    Option (Φ.F parameters shared results) :=
  if admitted : NTy.admits unit (.function parameters shared results) closure.encode = true then
    if site : isSiteWith unit Φ.sites parameters shared results closure = true then
      match siteIndex? Φ.sites closure with
      | some index =>
          if same : parameters = (Φ.sites.get index).parameters ∧
              shared = (Φ.sites.get index).shared ∧ results = (Φ.sites.get index).results then
            (NRow.decodeWith (fun parameters' shared' results' inner =>
                if _smaller : sizeOf inner.encode < sizeOf closure.encode then
                  decode parameters' shared' results' inner
                else none)
              (Φ.sites.get index).captured closure.captures.toList).map fun captures =>
              castIndex same (Φ.site index captures)
          else none
      | none => none
    else some (Φ.foreign ⟨closure, admitted, Bool.eq_false_iff.mpr site⟩)
  else none
termination_by sizeOf closure.encode

/-- A closure decodes only where the unit types it. -/
theorem admits_of_decode {parameters : NRow} {shared : Sharing} {results : NRow}
    {closure : ClosureValue} {value : Φ.F parameters shared results}
    (decoded : Φ.decode parameters shared results closure = some value) :
    NTy.admits unit (.function parameters shared results) closure.encode = true := by
  rw [decode] at decoded
  split at decoded
  · assumption
  · cases decoded

/-- Every closure the unit types decodes. -/
theorem decodes : (bound : Nat) → ∀ parameters shared results (closure : ClosureValue),
    sizeOf closure.encode < bound →
    NTy.admits unit (.function parameters shared results) closure.encode = true →
      (Φ.decode parameters shared results closure).isSome = true
  | 0, _, _, _, _, small, _ => absurd small (Nat.not_lt_zero _)
  | bound + 1, parameters, shared, results, closure, small, admitted => by
      rw [decode, dif_pos admitted]
      split
      · next site =>
          have site' := site
          simp only [isSiteWith] at site'
          cases found : siteIndex? Φ.sites closure with
          | none => simp [found] at site'
          | some index =>
              simp only [found, Bool.and_eq_true, decide_eq_true_eq] at site'
              obtain ⟨same, decodable⟩ := site'
              simp only [dif_pos same, Option.isSome_map]
              rw [NRow.isSome_decodeWith _ (closureTyping unit) (sizeOf closure.encode)
                (fun parameters' shared' results' inner smaller => by
                  simp only [dif_pos smaller, closureTyping]
                  by_cases typed : NTy.admits unit (.function parameters' shared' results')
                      inner.encode = true
                  · simp only [typed, if_true, Option.isSome_some]
                    exact decodes bound _ _ _ inner (by omega) typed
                  · simp only [typed, Bool.false_eq_true, if_false, Option.isSome_none]
                    cases decoded : Φ.decode parameters' shared' results' inner with
                    | none => rfl
                    | some value => exact absurd (Φ.admits_of_decode decoded) typed)
                _ _ fun capture member => closure.sizeOf_capture_lt member]
              exact decodable
      · rfl

/-- Every value's closure is one the unit types at its function type. -/
theorem admits_encode : (bound : Nat) → ∀ parameters shared results
    (value : Φ.F parameters shared results), sizeOf (Φ.encode value).encode < bound →
      NTy.admits unit (.function parameters shared results) (Φ.encode value).encode = true
  | 0, _, _, _, _, small => absurd small (Nat.not_lt_zero _)
  | bound + 1, parameters, shared, results, value, small => by
      rcases Φ.cases value with ⟨index, captures, same, rfl⟩ | ⟨key, rfl⟩
      · obtain ⟨rfl, rfl, rfl⟩ := same
        rw [Φ.encode_site] at small ⊢
        have plain := NRow.plain_encodeWith (fun _ _ _ value => Φ.encode value)
          (Φ.sites.get index).captured captures
        rw [SiteKey.closure_of_plain _ plain] at small ⊢
        have typed := Φ.typed index
        simp only [SiteSpec.typed, Bool.and_eq_true, beq_iff_eq] at typed
        obtain ⟨⟨faithful, rows⟩, sharing⟩ := typed
        simp only [ClosureValue.encode]
        unfold NTy.admits
        simp only [faithful, rows, sharing, beq_self_eq_true, Bool.true_and]
        exact NRow.admits_encodeWith unit _ (sizeOf (RuntimeValue.closure
            (Φ.sites.get index).key.function (Φ.sites.get index).key.mask
            (Φ.sites.get index).key.typeInstantiation
            (NRow.encodeWith (fun _ _ _ value => Φ.encode value) (Φ.sites.get index).captured
              captures).toArray))
          (fun parameters' shared' results' inner smaller =>
            admits_encode bound _ _ _ inner (by simp only [ClosureValue.encode] at small; omega))
          _ captures fun raw member => RuntimeValue.sizeOf_lt_closure (List.mem_toArray.mpr member)
      · rw [Φ.encode_foreign]
        exact key.admitted

/-- Inside a closure, the decoder's function arm decodes what the unit
types. -/
private theorem inner_isSome (closure : ClosureValue) : ∀ parameters shared results
    (capture : ClosureValue), sizeOf capture.encode < sizeOf closure.encode →
      ((fun parameters' shared' results' inner =>
          if _smaller : sizeOf inner.encode < sizeOf closure.encode then
            Φ.decode parameters' shared' results' inner
          else none) parameters shared results capture).isSome =
        (closureTyping unit parameters shared results capture).isSome := by
  intro parameters shared results capture smaller
  simp only [dif_pos smaller, closureTyping]
  by_cases typed : NTy.admits unit (.function parameters shared results) capture.encode = true
  · simp only [typed, if_true, Option.isSome_some]
    exact Φ.decodes _ _ _ _ capture (Nat.lt_succ_self _) typed
  · simp only [typed, Bool.false_eq_true, if_false, Option.isSome_none]
    cases decoded : Φ.decode parameters shared results capture with
    | none => rfl
    | some value => exact absurd (Φ.admits_of_decode decoded) typed

/-- Decoding a value's closure gives the value back. -/
theorem decode_encode : (bound : Nat) → ∀ parameters shared results
    (value : Φ.F parameters shared results), sizeOf (Φ.encode value).encode < bound →
      Φ.decode parameters shared results (Φ.encode value) = some value
  | 0, _, _, _, _, small => absurd small (Nat.not_lt_zero _)
  | bound + 1, parameters, shared, results, value, small => by
      have admitted := Φ.admits_encode _ parameters shared results value (Nat.lt_succ_self _)
      rcases Φ.cases value with ⟨index, captures, same, rfl⟩ | ⟨key, rfl⟩
      · obtain ⟨rfl, rfl, rfl⟩ := same
        rw [Φ.encode_site] at small admitted ⊢
        have plain := NRow.plain_encodeWith (fun _ _ _ value => Φ.encode value)
          (Φ.sites.get index).captured captures
        have found := siteIndex?_of_matches Φ.distinct index
          ((Φ.sites.get index).key.matches_closure plain)
        have listed : ((Φ.sites.get index).key.closure (NRow.encodeWith
            (fun _ _ _ value => Φ.encode value) (Φ.sites.get index).captured captures)).captures.toList =
            NRow.encodeWith (fun _ _ _ value => Φ.encode value) (Φ.sites.get index).captured
              captures := by
          rw [SiteKey.closure_of_plain _ plain, List.toList_toArray]
        generalize (Φ.sites.get index).key.closure (NRow.encodeWith
          (fun _ _ _ value => Φ.encode value) (Φ.sites.get index).captured captures) = closure
          at small admitted found listed
        -- The captures decode back, their closures below this one.
        have decoded := NRow.decodeWith_encodeWith (fun _ _ _ value => Φ.encode value)
          (fun parameters' shared' results' inner =>
            if smaller : sizeOf inner.encode < sizeOf closure.encode then
              Φ.decode parameters' shared' results' inner
            else none)
          (sizeOf closure.encode)
          (fun parameters' shared' results' inner smaller => by
            rw [dif_pos smaller]
            exact decode_encode bound _ _ _ inner (by omega))
          _ captures fun raw member => closure.sizeOf_capture_lt (listed ▸ member)
        have site : isSiteWith unit Φ.sites (Φ.sites.get index).parameters
            (Φ.sites.get index).shared (Φ.sites.get index).results closure = true := by
          simp only [isSiteWith, found, and_self, decide_true, Bool.true_and, NRow.decodable]
          rw [← NRow.isSome_decodeWith _ _ _ (Φ.inner_isSome closure) _ _
            fun capture member => closure.sizeOf_capture_lt member, listed, decoded]
          rfl
        rw [decode, dif_pos admitted, dif_pos site]
        simp only [found, and_self, dite_true]
        rw [listed, decoded]
        try rfl
      · rw [Φ.encode_foreign] at admitted ⊢
        rw [decode, dif_pos key.admitted, dif_neg (by simp [key.notSite])]

/-- A closure that decodes is its value's closure. -/
theorem encode_decode : (bound : Nat) → ∀ parameters shared results (closure : ClosureValue),
    sizeOf closure.encode < bound → ∀ value : Φ.F parameters shared results,
      Φ.decode parameters shared results closure = some value → Φ.encode value = closure
  | 0, _, _, _, _, small, _, _ => absurd small (Nat.not_lt_zero _)
  | bound + 1, parameters, shared, results, closure, small, value, decoded => by
      rw [decode] at decoded
      split at decoded
      · split at decoded
        · split at decoded
          · next index found =>
              split at decoded
              · next same =>
                  simp only [Option.map_eq_some_iff] at decoded
                  obtain ⟨captures, capturesDecoded, rfl⟩ := decoded
                  obtain ⟨rfl, rfl, rfl⟩ := same
                  rw [Φ.encode_site, NRow.encodeWith_of_decodeWith (fun _ _ _ value => Φ.encode value)
                    _ (sizeOf closure.encode) (fun parameters' shared' results' inner value' inside
                      innerDecoded => by
                        simp only [dif_pos inside] at innerDecoded
                        exact encode_decode bound _ _ _ inner (by omega) value' innerDecoded)
                    _ _ (fun capture member => closure.sizeOf_capture_lt member) captures
                    capturesDecoded]
                  have matched := siteIndex?_matches found
                  simp only [SiteKey.matches, decide_eq_true_eq] at matched
                  rw [SiteKey.closure_of_plain _ fun capture member =>
                    closure.plain capture (Array.mem_toList_iff.mp member)]
                  obtain ⟨function, mask, typeInstantiation, raws, plain⟩ := closure
                  simp only at matched
                  obtain ⟨rfl, rfl, rfl⟩ := matched
                  simp only [Array.toArray_toList]
              · cases decoded
          · cases decoded
        · cases decoded
          rw [Φ.encode_foreign]
      · cases decoded

/-- A link set's function values at a function type. -/
@[reducible] noncomputable def functions (parameters : NRow) (shared : Sharing) (results : NRow) :
    TypedFunctions (Φ.F parameters shared results) where
  closure := Φ.encode
  ofClosure? := Φ.decode parameters shared results
  ofClosure?_closure := fun value =>
    Φ.decode_encode _ parameters shared results value (Nat.lt_succ_self _)
  closure_of_ofClosure? := fun closure value decoded =>
    Φ.encode_decode _ parameters shared results closure (Nat.lt_succ_self _) value decoded

/-- A link set's function values, the unit's instance. -/
@[reducible] noncomputable def functionCarriers : FunctionCarriers unit where
  carrier := Φ.F
  functions := Φ.functions
  admits := fun parameters shared results value =>
    Φ.admits_encode _ parameters shared results value (Nat.lt_succ_self _)
  decodes := fun parameters shared results closure admitted =>
    Φ.decodes _ parameters shared results closure (Nat.lt_succ_self _) admitted

/-- A site's closure: its key, capturing its captures' encodings. -/
theorem encode_site_eq (index : Fin Φ.sites.length)
    (captures : HListWith Φ.F (Φ.sites.get index).captured) :
    Φ.encode (Φ.site index captures) =
      ⟨(Φ.sites.get index).key.function, (Φ.sites.get index).key.mask,
        (Φ.sites.get index).key.typeInstantiation,
        (NRow.encodeWith (fun _ _ _ value => Φ.encode value) (Φ.sites.get index).captured
          captures).toArray, fun capture member =>
          NRow.plain_encodeWith _ _ captures capture (List.mem_toArray.mp member)⟩ := by
  rw [Φ.encode_site, SiteKey.closure_of_plain _ (NRow.plain_encodeWith _ _ captures)]

/-- At the base frame, construction is the value whose closure it creates:
a site's constructor where the closure is the site's (`encode_site_eq`),
whatever its instantiation. -/
theorem construct_eq {parameters : NRow} {shared : Sharing} {results : NRow}
    {handle : FunctionHandle} {mask : Nat} {typeInstantiation : Array (TypeId × TypeId)}
    {σs : NRow} (captures : @HList (@Carriers.runtime unit Φ.functionCarriers) σs)
    (typed : @SiteTyped unit Φ.functionCarriers (@Skolems.runtime unit Φ.functionCarriers)
      parameters shared results handle mask typeInstantiation σs)
    (value : Φ.F parameters shared results)
    (closed : @closureOf unit Φ.functionCarriers (@Skolems.runtime unit Φ.functionCarriers)
      handle mask typeInstantiation σs captures = Φ.encode value) :
    @Skolems.construct unit Φ.functionCarriers (@Skolems.runtime unit Φ.functionCarriers)
      parameters shared results handle mask typeInstantiation σs captures typed = value :=
  (Φ.functions parameters shared results).closure_injective
    ((@Skolems.closure_construct unit Φ.functionCarriers (@Skolems.runtime unit Φ.functionCarriers)
      parameters shared results handle mask typeInstantiation σs captures typed).trans closed)

end SiteFamily

/-- The generated equations of a link set's closure site without type
arguments, at the base frame: construction is its constructor, the
constructor's closure is the site's, invoking it is calling its target on
the woven arguments, the unit's check types it, and the value naming its
target is typed. -/
structure SiteEquations where
  /-- The unit constant the site's family is declared at. -/
  unit : Lean.Name
  constructor : Lean.Name
  construct : Lean.Name
  closure : Lean.Name
  invoke : Option Lean.Name
  check : Lean.Name
  /-- The typing of the value naming the site's target, where the site
  captures values. -/
  named : Option Lean.Name
  /-- The site's target, by its name. -/
  target : String
  /-- The family's constructor of values no site encodes. -/
  foreign : Lean.Name
  deriving Inhabited

/-- The generated equations of every closure site, by unit constant and by
constructor. -/
structure SiteEquationTable where
  byUnit : Lean.NameMap (Array SiteEquations) := {}
  byConstructor : Lean.NameMap SiteEquations := {}
  foreignConstructors : Lean.NameSet := {}
  deriving Inhabited

/-- A table with a site's equations added. -/
def SiteEquationTable.add (table : SiteEquationTable) (entry : SiteEquations) :
    SiteEquationTable :=
  { byUnit := table.byUnit.insert entry.unit (((table.byUnit.find? entry.unit).getD #[]).push entry)
    byConstructor := table.byConstructor.insert entry.constructor entry
    foreignConstructors := table.foreignConstructors.insert entry.foreign }

initialize siteEquationsExtension :
    Lean.SimplePersistentEnvExtension SiteEquations SiteEquationTable ←
  Lean.registerSimplePersistentEnvExtension {
    addEntryFn := SiteEquationTable.add
    addImportedFn := fun entries => Lean.mkStateFromImportedEntries SiteEquationTable.add {} entries }

/-- The generated equations of a unit's sites. -/
def siteEquationsOf (environment : Lean.Environment) (unit : Lean.Name) : Array SiteEquations :=
  ((siteEquationsExtension.getState environment).byUnit.find? unit).getD #[]

/-- Whether a constant is a generated family's constructor of values no site
encodes. -/
def isForeignConstructor (environment : Lean.Environment) (name : Lean.Name) : Bool :=
  (siteEquationsExtension.getState environment).foreignConstructors.contains name

/-- The generated equations of a site by its constructor. -/
def siteEquationsAt? (environment : Lean.Environment) (constructor : Lean.Name) :
    Option SiteEquations :=
  (siteEquationsExtension.getState environment).byConstructor.find? constructor

end VerexIR.Proofs.Denote

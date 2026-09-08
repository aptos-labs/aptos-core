-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Contract
import LeanerIR.Proofs.Representation

/-!
# Certified typed views of runtime functions

The executable and big-step semantics deliberately use `RuntimeValue` rows.
Verification-facing functions should not: their arguments and results have
ordinary Lean types, connected to the runtime representation by a certified
codec.  `decodeSpec` is the boundary between those two views.

An undecodable successful runtime result is represented as `undefined` in the
typed view.  Consequently, proving a typed contract also proves that every
reachable result has the declared representation; the adapter never discards
an execution merely because it failed to decode.
-/

namespace LeanerIR.Proofs

/-- A partial decoding with a certified left inverse.  The left inverse is
enough to make `encode` injective and is exactly the roundtrip generated for
typed struct twins. -/
structure Codec (Native Runtime : Type) where
  encode : Native → Runtime
  decode? : Runtime → Option Native
  decode_encode : ∀ value, decode? (encode value) = some value

namespace Codec

variable {Native Runtime : Type}

@[simp] theorem decode_encode_apply (codec : Codec Native Runtime)
    (value : Native) : codec.decode? (codec.encode value) = some value :=
  codec.decode_encode value

/-- A certified encoder cannot identify two native values. -/
theorem encode_injective (codec : Codec Native Runtime) :
    Function.Injective codec.encode := by
  intro left right equal
  have decoded := congrArg codec.decode? equal
  simpa [codec.decode_encode] using decoded

@[simp] theorem encode_eq_encode (codec : Codec Native Runtime)
    (left right : Native) :
    codec.encode left = codec.encode right ↔ left = right := by
  constructor
  · intro equal
    exact codec.encode_injective equal
  · intro equal
    exact congrArg codec.encode equal

/-- Runtime values themselves form the representation used for an abstract
storage-parametric carrier. -/
def identity (Runtime : Type) : Codec Runtime Runtime where
  encode := id
  decode? := some
  decode_encode := by simp

/-- Certified fixed-width or unbounded integers. -/
def specInt (width : IntWidth) (signed : Bool) :
    Codec (SpecInt width signed) RuntimeValue where
  encode := fun value => .integer value.val
  decode? := decodeInt? width signed
  decode_encode := decodeInt?_val

def bool : Codec Bool RuntimeValue where
  encode := .bool
  decode? := decodeBool?
  decode_encode := decodeBool?_bool

def string : Codec String RuntimeValue where
  encode := .string
  decode? := decodeString?
  decode_encode := decodeString?_string

def address : Codec String RuntimeValue where
  encode := .address
  decode? := decodeAddress?
  decode_encode := decodeAddress?_address

def signer : Codec String RuntimeValue where
  encode := .signer
  decode? := decodeSigner?
  decode_encode := decodeSigner?_signer

def bytes : Codec (Array UInt8) RuntimeValue where
  encode := .bytes
  decode? := decodeBytes?
  decode_encode := decodeBytes?_bytes

def unit : Codec Unit RuntimeValue where
  encode := fun _ => .unit
  decode? := decodeUnit?
  decode_encode := by intro value; cases value; rfl

/-! A tuple local keeps a heterogeneous nested product natively.  The row
codec is assembled one statically typed component at a time, then wrapped in
the VM's flat `RuntimeValue.tuple`.  No universal value appears in the local. -/

def tupleNil : Codec Unit (List RuntimeValue) where
  encode := fun _ => []
  decode?
    | [] => some ()
    | _ => none
  decode_encode := by intro value; cases value; rfl

def tupleCons (head : Codec Head RuntimeValue)
    (tail : Codec Tail (List RuntimeValue)) :
    Codec (Head × Tail) (List RuntimeValue) where
  encode := fun value => head.encode value.1 :: tail.encode value.2
  decode?
    | first :: rest => do
        let decodedHead ← head.decode? first
        let decodedTail ← tail.decode? rest
        pure (decodedHead, decodedTail)
    | [] => none
  decode_encode := by
    intro value
    simp [head.decode_encode, tail.decode_encode]

def tuple (row : Codec Native (List RuntimeValue)) :
    Codec Native RuntimeValue where
  encode := fun value => .tuple (row.encode value).toArray
  decode?
    | .tuple values => row.decode? values.toList
    | _ => none
  decode_encode := by
    intro value
    simp [row.decode_encode]

/-- Lift a certified element representation pointwise through runtime
vectors.  The generated wrapper therefore exposes `Array Native` while the
core evaluator continues to use `RuntimeValue.vector`. -/
def vector (codec : Codec Native RuntimeValue) :
    Codec (Array Native) RuntimeValue where
  encode := fun values => .vector (values.map codec.encode)
  decode?
    | .vector values => (values.toList.mapM codec.decode?).map List.toArray
    | _ => none
  decode_encode := by
    intro values
    change ((values.map codec.encode).toList.mapM codec.decode?).map List.toArray =
      some values
    simp only [Array.toList_map, List.mapM_map]
    have decoded : values.toList.mapM (codec.decode? ∘ codec.encode) =
        some values.toList := by
      induction values.toList with
      | nil => simp
      | cons value values ih =>
          simp [Function.comp_apply, codec.decode_encode, ih]
    simp [decoded]

/-- Expose pointwise encoding without unfolding the vector decoder. -/
theorem vector_encode (codec : Codec Native RuntimeValue)
    (values : Array Native) :
    (vector codec).encode values = .vector (values.map codec.encode) := rfl

/-- Move vectors retain their native length certificate at the boundary.
The raw array codec remains available to language-neutral and Rust clients. -/
def boundedVector (codec : Codec Native RuntimeValue) :
    Codec (SpecVector Native) RuntimeValue where
  encode := fun value => .vector (value.values.map codec.encode)
  decode?
    | .vector values => do
        let decoded ← values.toList.mapM codec.decode?
        if bound : decoded.length < 2 ^ 64 then
          some ⟨decoded.toArray, by simpa only [List.size_toArray] using bound⟩
        else none
    | _ => none
  decode_encode := by
    intro ⟨values, bounded⟩
    have decoded : values.toList.mapM (codec.decode? ∘ codec.encode) =
        some values.toList := by
      induction values.toList with
      | nil => simp
      | cons value values ih =>
          simp [Function.comp_apply, codec.decode_encode, ih]
    simp [List.mapM_map, decoded, bounded]

theorem boundedVector_encode (codec : Codec Native RuntimeValue)
    (value : SpecVector Native) :
    (boundedVector codec).encode value = .vector (value.values.map codec.encode) := rfl

/-- Normalize mapping over the list-backed representation emitted for a
source vector literal. -/
theorem array_map_reverse_toArray (map : Native → Runtime)
    (values : List Native) :
    values.reverse.toArray.map map = (values.map map).reverse.toArray := by
  simp

attribute [lir_data_norm high] vector_encode boundedVector_encode array_map_reverse_toArray
-- A preceding operation may already have normalized the literal's reverse.
-- Pointwise encoding must normalize in that spelling as well.
attribute [lir_data_norm] List.map_toArray List.map_cons List.map_nil
attribute [lir_data_norm] identity specInt bool string address signer bytes unit
  tupleNil tupleCons tuple vector boundedVector
attribute [lir_data_norm] decodeBool?_bool decodeString?_string decodeAddress?_address
  decodeSigner?_signer decodeBytes?_bytes decodeUnit?_unit

end Codec

/-- Native argument carried by one mutable-reference parameter.  The dynamic
loan identity is part of the call representation; the value is native. -/
structure MutableArgument (Value : Type) where
  loan : Nat
  value : Value

namespace Codec

/-- Lift a value codec through the prophetic mutable-reference runtime
representation. -/
def mutable (codec : Codec Native RuntimeValue) :
    Codec (MutableArgument Native) RuntimeValue where
  encode := fun argument => .borrow argument.loan (codec.encode argument.value)
  decode?
    | .borrow loan value => (codec.decode? value).map fun decoded => ⟨loan, decoded⟩
    | _ => none
  decode_encode := by
    intro argument
    cases argument
    simp [codec.decode_encode]

attribute [lir_data_norm] mutable

/-! ### Decoding is exact

A caller consuming a callee's summary learns the callee's runtime results
only through `decode?`; these inversions recover the runtime row from the
decoded value, which the summary speaks about through `encode`. -/

/-- A certified integer decodes only from its own encoding. -/
theorem specInt_decode?_eq_some {width : IntWidth} {signed : Bool}
    {runtime : RuntimeValue} {value : SpecInt width signed}
    (decoded : (specInt width signed).decode? runtime = some value) :
    runtime = .integer value.val := by
  cases runtime <;> simp only [specInt, decodeInt?, reduceCtorEq] at decoded
  split at decoded
  · cases Option.some.inj decoded
    rfl
  · exact absurd decoded (by simp)

private theorem decodedList_shape (codec : Codec Native RuntimeValue)
    (elementShape : ∀ {runtime value}, codec.decode? runtime = some value →
      runtime = codec.encode value)
    {runtime : List RuntimeValue} {values : List Native}
    (decoded : runtime.mapM codec.decode? = some values) :
    runtime = values.map codec.encode := by
  induction runtime generalizing values with
  | nil => simpa using decoded.symm
  | cons head tail ih =>
      simp only [List.mapM_cons, bind, Option.bind_eq_some_iff] at decoded
      obtain ⟨value, headDecoded, rest⟩ := decoded
      obtain ⟨values, tailDecoded, result⟩ := rest
      cases result
      simp only [List.map_cons, elementShape headDecoded, ih tailDecoded]

/-- Exact element decoding lifts through a bounded vector. A codec's left
inverse alone does not imply this property, so retain the element premise. -/
theorem boundedVector_decode?_eq_some (codec : Codec Native RuntimeValue)
    (elementShape : ∀ {runtime value}, codec.decode? runtime = some value →
      runtime = codec.encode value)
    {runtime : RuntimeValue} {value : SpecVector Native}
    (decoded : (boundedVector codec).decode? runtime = some value) :
    runtime = .vector (value.values.map codec.encode) := by
  cases runtime <;> try contradiction
  case vector elements =>
    simp only [boundedVector, bind, Option.bind_eq_some_iff] at decoded
    obtain ⟨values, decoded, bounded⟩ := decoded
    split at bounded <;> try contradiction
    cases bounded
    have shape := decodedList_shape codec elementShape decoded
    have := congrArg List.toArray shape
    simpa using congrArg RuntimeValue.vector this

/-- A mutable reference decodes only from a borrow of its loan whose
current decodes to its value. -/
theorem mutable_decode?_eq_some {codec : Codec Native RuntimeValue}
    {runtime : RuntimeValue} {argument : MutableArgument Native}
    (decoded : codec.mutable.decode? runtime = some argument) :
    ∃ current, runtime = .borrow argument.loan current ∧
      codec.decode? current = some argument.value := by
  cases runtime <;> simp only [mutable, reduceCtorEq] at decoded
  rename_i loan current
  simp only [Option.map_eq_some_iff] at decoded
  obtain ⟨value, valueEq, argumentEq⟩ := decoded
  subst argumentEq
  exact ⟨current, rfl, valueEq⟩

end Codec

/-- Decode the successful results of a runtime computation.  A successful
result outside the codec image is an explicit undefined behavior of this
view, so contract satisfaction must rule it out. -/
def decodeSpec (codec : Codec NativeResult RuntimeResult)
    (action : Spec State Error RuntimeResult) : Spec State Error NativeResult where
  ok := fun initial result final =>
    ∃ runtimeResult,
      action.ok initial runtimeResult final ∧
      codec.decode? runtimeResult = some result
  aborts := action.aborts
  undefined := fun initial =>
    action.undefined initial ∨
      ∃ runtimeResult final,
        action.ok initial runtimeResult final ∧
        codec.decode? runtimeResult = none

/-- Give a runtime function a typed argument/result boundary. -/
def typedFunction (arguments : Codec NativeArgs RuntimeArgs)
    (results : Codec NativeResult RuntimeResult)
    (function : RuntimeArgs → Spec State Error RuntimeResult) :
    NativeArgs → Spec State Error NativeResult :=
  fun args => decodeSpec results (function (arguments.encode args))

namespace Contract

/-- Pull a runtime-row contract back to its native argument and result
boundary.  Generated V3 contracts use this operation as their native view;
the existing clause translation remains the source of the predicates, while
the codecs remove runtime rows from the function signature. -/
def typed (arguments : Codec NativeArgs RuntimeArgs)
    (results : Codec NativeResult RuntimeResult)
    (contract : Contract State Error RuntimeArgs RuntimeResult) :
    Contract State Error NativeArgs NativeResult where
  requires := fun args initial => contract.requires (arguments.encode args) initial
  ensures := fun args initial result final =>
    contract.ensures (arguments.encode args) initial (results.encode result) final
  aborts := fun args initial error =>
    contract.aborts (arguments.encode args) initial error
  mayAbort := fun args initial => contract.mayAbort (arguments.encode args) initial
  mustAbort := fun args initial => contract.mustAbort (arguments.encode args) initial
  frame := fun args initial final =>
    contract.frame (arguments.encode args) initial final

/-- Runtime-row form of a typed contract.  Arguments must be in the image of
their codec; normal results must decode.  This is the generated
`FunctionContract` presented to the authoritative runtime semantics. -/
def runtime (arguments : Codec NativeArgs RuntimeArgs)
    (results : Codec NativeResult RuntimeResult)
    (contract : Contract State Error NativeArgs NativeResult) :
    Contract State Error RuntimeArgs RuntimeResult where
  requires := fun runtimeArgs initial =>
    ∃ args, arguments.encode args = runtimeArgs ∧ contract.requires args initial
  ensures := fun runtimeArgs initial runtimeResult final =>
    ∃ args result,
      arguments.encode args = runtimeArgs ∧
      results.decode? runtimeResult = some result ∧
      contract.ensures args initial result final
  aborts := fun runtimeArgs initial error =>
    ∃ args, arguments.encode args = runtimeArgs ∧ contract.aborts args initial error
  mayAbort := fun runtimeArgs initial =>
    ∃ args, arguments.encode args = runtimeArgs ∧ contract.mayAbort args initial
  mustAbort := fun runtimeArgs initial =>
    ∃ args, arguments.encode args = runtimeArgs ∧ contract.mustAbort args initial
  frame := fun runtimeArgs initial final =>
    ∃ args, arguments.encode args = runtimeArgs ∧ contract.frame args initial final

end Contract

/-- Weakest precondition of a decoded result view.  Besides the native
continuation, the runtime computation must establish that every successful
result is in the codec image. -/
@[lir_wp_norm] theorem wp_decodeSpec
    (codec : Codec NativeResult RuntimeResult)
    (action : Spec State Error RuntimeResult)
    (ensures : NativeResult → State → Prop) (aborts : Error → Prop)
    (initial : State) :
    wp (decodeSpec codec action) ensures aborts initial ↔
      wp action
        (fun runtimeResult final =>
          ∃ result, codec.decode? runtimeResult = some result ∧
            ensures result final)
        aborts initial := by
  constructor
  · rintro ⟨normal, failing, defined⟩
    refine ⟨?_, failing, ?_⟩
    · intro runtimeResult final execution
      cases equation : codec.decode? runtimeResult with
      | none =>
          exact False.elim (defined (.inr
            ⟨runtimeResult, final, execution, equation⟩))
      | some result =>
          exact ⟨result, rfl, normal result final
            ⟨runtimeResult, execution, equation⟩⟩
    · exact fun undefined => defined (.inl undefined)
  · rintro ⟨normal, failing, defined⟩
    refine ⟨?_, failing, ?_⟩
    · rintro result final ⟨runtimeResult, execution, decoded⟩
      obtain ⟨other, otherDecoded, established⟩ :=
        normal runtimeResult final execution
      rw [decoded] at otherDecoded
      cases otherDecoded
      exact established
    · intro undefined
      cases undefined with
      | inl obligation => exact defined obligation
      | inr undecodable =>
          obtain ⟨runtimeResult, final, execution, equation⟩ := undecodable
          obtain ⟨result, decoded, _⟩ := normal runtimeResult final execution
          rw [equation] at decoded
          cases decoded

/-- The generated typed function exposes `wp_decodeSpec` without requiring
the proof script to unfold the adapter itself. -/
@[lir_wp_norm] theorem wp_typedFunction
    (arguments : Codec NativeArgs RuntimeArgs)
    (results : Codec NativeResult RuntimeResult)
    (function : RuntimeArgs → Spec State Error RuntimeResult)
    (args : NativeArgs) (ensures : NativeResult → State → Prop)
    (aborts : Error → Prop) (initial : State) :
    wp (typedFunction arguments results function args) ensures aborts initial ↔
      wp (function (arguments.encode args))
        (fun runtimeResult final =>
          ∃ result, results.decode? runtimeResult = some result ∧
            ensures result final)
        aborts initial :=
  wp_decodeSpec results (function (arguments.encode args)) ensures aborts initial

/-- A proof over the typed view transports to the runtime semantics and its
generated runtime-row contract.  Absence of `decodeSpec.undefined` supplies
the nontrivial fact that every reachable runtime result decodes. -/
theorem satisfies_runtime
    (arguments : Codec NativeArgs RuntimeArgs)
    (results : Codec NativeResult RuntimeResult)
    (function : RuntimeArgs → Spec State Error RuntimeResult)
    (contract : Contract State Error NativeArgs NativeResult)
    (verified : Satisfies (typedFunction arguments results function) contract) :
    Satisfies function (contract.runtime arguments results) := by
  intro runtimeArgs initial permitted
  obtain ⟨args, encodedArgs, typedPermitted⟩ := permitted
  subst runtimeArgs
  have established := verified args initial typedPermitted
  refine ⟨?_, ?_, ?_⟩
  · intro runtimeResult final execution
    have decoded : ∃ result, results.decode? runtimeResult = some result := by
      cases equation : results.decode? runtimeResult with
      | none =>
          exact False.elim (established.2.2 (.inr
            ⟨runtimeResult, final, execution, equation⟩))
      | some result => exact ⟨result, rfl⟩
    obtain ⟨result, decoded⟩ := decoded
    have typedExecution :
        (typedFunction arguments results function args).ok initial result final :=
      ⟨runtimeResult, execution, decoded⟩
    obtain ⟨postcondition, framed, notMustAbort⟩ :=
      established.1 result final typedExecution
    refine ⟨?_, ⟨args, rfl, framed⟩, ?_⟩
    · intro noRuntimeAbort
      refine ⟨args, result, rfl, decoded, postcondition ?_⟩
      intro typedAbort
      exact noRuntimeAbort ⟨args, rfl, typedAbort⟩
    · rintro ⟨otherArgs, sameEncoding, otherMustAbort⟩
      have sameArgs : otherArgs = args :=
        arguments.encode_injective sameEncoding
      subst otherArgs
      exact notMustAbort otherMustAbort
  · intro error execution
    exact ⟨args, rfl, established.2.1 error execution⟩
  · intro undefined
    exact established.2.2 (.inl undefined)

end LeanerIR.Proofs

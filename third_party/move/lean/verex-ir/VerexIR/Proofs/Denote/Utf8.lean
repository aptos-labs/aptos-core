-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Types
import Init.Data.String.Basic

/-!
# Checked UTF-8 validity of specification bytes

The source library uses an existential string encoding relation, not a decoder
primitive. This bridge proves it equivalent to Lean's executable UTF-8 validator.
A string length in a contract wraps at 64 bits; an oversized string's `stringBytes`
fallback is empty, which is itself valid UTF-8, so that case remains harmless.
-/

namespace VerexIR.Proofs.Denote.Utf8

/-- A runtime byte as its certified specification integer. -/
def ofByte (byte : UInt8) : SpecInt (.bits 8) false :=
  SpecInt.ofInt ⟨0, by decide⟩ (Int.ofNat byte.toNat)

/-- The exact runtime byte carried by a certified unsigned 8-bit integer. -/
def toByte (byte : SpecInt (.bits 8) false) : UInt8 := UInt8.ofNat byte.val.toNat

theorem ofByte_val (byte : UInt8) : (ofByte byte).val = Int.ofNat byte.toNat := by
  apply SpecInt.val_ofInt
  have bound := byte.toNat_lt_size
  simp only [UInt8.size] at bound
  simp [IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?]
  omega

@[simp] theorem decode_encode (byte : UInt8) : toByte (ofByte byte) = byte := by
  simp [toByte, ofByte_val]

@[simp] theorem encode_decode (byte : SpecInt (.bits 8) false) : ofByte (toByte byte) = byte := by
  apply SpecInt.ext
  rw [ofByte_val]
  have bounds := byte.unsigned_bounds
  have bound : byte.val.toNat < UInt8.size := by simp only [UInt8.size]; omega
  simp only [toByte, UInt8.toNat_ofNat_of_lt' bound]
  exact Int.toNat_of_nonneg bounds.1

/-- Encode a specification byte vector without changing any byte. -/
def toByteArray (bytes : SpecVector (SpecInt (.bits 8) false)) : ByteArray :=
  ⟨bytes.values.map toByte⟩

@[simp] theorem toByteArray_size (bytes : SpecVector (SpecInt (.bits 8) false)) :
    (toByteArray bytes).size = bytes.values.size := by simp [toByteArray, ByteArray.size]

theorem toByteArray_stringBytes (text : String) (bound : text.utf8ByteSize < 2 ^ 64) :
    toByteArray (stringBytes text) = text.toUTF8 := by
  unfold stringBytes
  rw [dif_pos (by simpa using bound)]
  simp only [toByteArray, Array.map_map]
  change ByteArray.mk (text.toUTF8.data.map (fun byte => toByte (ofByte byte))) = _
  simp

theorem bounded_valid_iff (bytes : SpecVector (SpecInt (.bits 8) false)) :
    (∃ text : String, text.utf8ByteSize = bytes.values.size ∧ stringBytes text = bytes) ↔
      (toByteArray bytes).validateUTF8 = true := by
  constructor
  · rintro ⟨text, length, encoded⟩
    have bound : text.utf8ByteSize < 2 ^ 64 := by rw [length]; exact bytes.bounded
    have same := toByteArray_stringBytes text bound
    rw [encoded] at same
    rw [same, ByteArray.validateUTF8_eq_true_iff]
    exact text.isValidUTF8
  · intro valid
    let text := String.fromUTF8 (toByteArray bytes) (ByteArray.validateUTF8_eq_true_iff.mp valid)
    have length : text.utf8ByteSize = bytes.values.size := by
      simpa only [text, String.fromUTF8, String.utf8ByteSize_ofByteArray] using toByteArray_size bytes
    refine ⟨text, length, ?_⟩
    apply SpecVector.ext
    unfold stringBytes
    rw [dif_pos (by simpa [length] using bytes.bounded)]
    change ((toByteArray bytes).data.map ofByte) = bytes.values
    simp [toByteArray, Array.map_map, Function.comp_def]

theorem encoding_exists_iff (bytes : SpecVector (SpecInt (.bits 8) false)) :
    (∃ text : String, stringBytes text = bytes) ↔
      (toByteArray bytes).validateUTF8 = true := by
  constructor
  · rintro ⟨text, encoded⟩
    by_cases bound : text.utf8ByteSize < 2 ^ 64
    · have same := toByteArray_stringBytes text bound
      rw [encoded] at same
      rw [same, ByteArray.validateUTF8_eq_true_iff]
      exact text.isValidUTF8
    · have empty : bytes.values = #[] := by
        rw [← encoded]
        simp [stringBytes, bound]
      change (ByteArray.mk (bytes.values.map toByte)).validateUTF8 = true
      rw [empty]
      simp only [Array.map_empty]
      exact ByteArray.validateUTF8_eq_true_iff.mpr ByteArray.isValidUTF8_empty
  · intro valid
    obtain ⟨text, _, encoded⟩ := (bounded_valid_iff bytes).mpr valid
    exact ⟨text, encoded⟩

/-- Encoding a string always yields valid UTF-8, including the empty fallback
for strings exceeding the specification vector bound. Keep this symbolic so
normalization never unfolds the decoder on an unknown string. -/
theorem validate_stringBytes (text : String) :
    (toByteArray (stringBytes text)).validateUTF8 = true :=
  (encoding_exists_iff (stringBytes text)).mp ⟨text, rfl⟩

/-- The exact wrapped-length relation generated for a source UTF-8 contract. -/
theorem valid_iff (bytes : SpecVector (SpecInt (.bits 8) false)) :
    (∃ text : String, wrapInt 64 false (text.utf8ByteSize : Int) =
      (bytes.values.size : Int) ∧ stringBytes text = bytes) ↔
      (toByteArray bytes).validateUTF8 = true := by
  constructor
  · rintro ⟨text, _, encoded⟩
    exact (encoding_exists_iff bytes).mp ⟨text, encoded⟩
  · intro valid
    obtain ⟨text, length, encoded⟩ := (bounded_valid_iff bytes).mpr valid
    refine ⟨text, ?_, encoded⟩
    rw [length, wrapInt_unsigned]
    apply Int.emod_eq_of_lt (Int.natCast_nonneg _)
    have := bytes.bounded
    exact_mod_cast this

end VerexIR.Proofs.Denote.Utf8

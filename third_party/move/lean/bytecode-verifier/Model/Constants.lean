-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Spec.Syntax
import Model.Result

/-!
# Constants

UNTRUSTED. Transcribes `move-bytecode-verifier/src/constants.rs` and the BCS
decoding it relies on (`Constant::deserialize_constant`, which runs
`MoveValue::simple_deserialize`, which is `bcs::from_bytes`): a constant's type
contains only primitive types and vectors, and its data is exactly one
canonical BCS value of that type.
-/

namespace MoveBytecode.Model

/-- `SignatureToken::is_valid_for_constant`. -/
def isValidForConstant : SignatureToken → Bool
  | .bool | .u8 | .u16 | .u32 | .u64 | .u128 | .u256 | .address => true
  | .i8 | .i16 | .i32 | .i64 | .i128 | .i256 => true
  | .vector t => isValidForConstant t
  | .signer | .function .. | .struct _ | .structInstantiation .. | .reference _
  | .mutableReference _ | .typeParameter _ => false

/-- BCS's `parse_u32_from_uleb128`: a canonical ULEB128 encoding of a value
that fits in 32 bits. -/
def uleb128 : List UInt8 → Nat → Nat → Option (Nat × List UInt8)
  | [], _, _ => none
  | b :: rest, shift, acc =>
    let digit := b.toNat % 128
    let value := acc + digit * 2 ^ shift
    if value ≥ 2 ^ 32 then none
    else if b.toNat < 128 then
      -- A multi-byte encoding may not end in a zero byte.
      if shift > 0 && b.toNat == 0 then none else some (value, rest)
    else if shift ≥ 28 then none
    else uleb128 rest (shift + 7) value

/-- BCS's maximum sequence length, `MAX_SEQUENCE_LENGTH = 2^31 - 1`. -/
def maxSequenceLength : Nat := 2 ^ 31 - 1

/-- Decode one BCS value of the given layout, returning the remaining bytes. -/
def bcsSkip : SignatureToken → List UInt8 → Option (List UInt8)
  | .bool, b :: rest => if b.toNat ≤ 1 then some rest else none
  | .bool, [] => none
  | .u8, data | .i8, data => if data.length ≥ 1 then some (data.drop 1) else none
  | .u16, data | .i16, data => if data.length ≥ 2 then some (data.drop 2) else none
  | .u32, data | .i32, data => if data.length ≥ 4 then some (data.drop 4) else none
  | .u64, data | .i64, data => if data.length ≥ 8 then some (data.drop 8) else none
  | .u128, data | .i128, data => if data.length ≥ 16 then some (data.drop 16) else none
  | .u256, data | .i256, data => if data.length ≥ 32 then some (data.drop 32) else none
  | .address, data => if data.length ≥ 32 then some (data.drop 32) else none
  | .vector t, data => do
    let (len, rest) ← uleb128 data 0 0
    if len > maxSequenceLength then none
    else
      let rec elems : Nat → List UInt8 → Option (List UInt8)
        | 0, d => some d
        | n + 1, d => do elems n (← bcsSkip t d)
      elems len rest
  | .signer, _ | .function .., _ | .struct _, _ | .structInstantiation .., _
  | .reference _, _ | .mutableReference _, _ | .typeParameter _, _ => none

/-- `verify_constant`. -/
def verifyConstant (c : Constant) : VResult Unit :=
  if !isValidForConstant c.type then reject "INVALID_CONSTANT_TYPE"
  else
    match bcsSkip c.type c.data with
    | some [] => pure ()
    | some (_ :: _) => reject "MALFORMED_CONSTANT_DATA"
    | none => reject "MALFORMED_CONSTANT_DATA"

/-- `constants::verify_module`. -/
def verifyConstants (m : CompiledModule) : VResult Unit :=
  m.constantPool.toList.forM verifyConstant

end MoveBytecode.Model

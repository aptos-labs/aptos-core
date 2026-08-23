-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
-- Test category: Move language.

import Move
import MoveModel.Tests.Common

/-! All Move integer widths: arithmetic across widths, casts, and the bit
operations, compiled and interpreted, with checked-semantics proofs for the
aborting operations. -/

namespace Tests.MovePrograms

open Move
open scoped Move Move.Compiler Move.Spec

module Integers where

  fun typed_u8 : U8 := 112u8

  fun grouped_u64 : U64 := 1_234_567u64

  def COMPLEX : U64 := 1 + 2 * 3
  def SIGNED_COMPLEX : I64 := -5 + 2
  def SHIFTED_COMPLEX : U64 := (1 : U64) <<< (4 : U8)

  /-! ## Functions -/

  fun small_sum (left right : U8) : U8 :=
    left + right

  spec small_sum (left : U8) (right : U8) where
    ensures True;
    aborts_if ¬left.toNat + right.toNat < U8.size
      with Semantics.Checked.arithmeticAbortCode

  fun wide_product (left right : U128) : U128 :=
    left * right

  spec wide_product (left : U128) (right : U128) where
    ensures True;
    aborts_if ¬left.toNat * right.toNat < U128.size
      with Semantics.Checked.arithmeticAbortCode

  fun narrow (value : U64) : U8 :=
    (value.cast : U8)

  spec narrow (value : U64) where
    ensures result.toNat = value.toNat;
    aborts_if ¬value.toNat < U8.size
      with Semantics.Checked.arithmeticAbortCode

  fun widen (value : U8) : U256 :=
    (value.cast : U256)

  spec widen (value : U8) where
    ensures result.toNat = value.toNat;
    aborts_if False

  fun masked (value mask : U64) : U64 :=
    value &&& mask

  spec masked (value : U64) (mask : U64) where
    ensures result.toNat = value.toNat &&& mask.toNat;
    aborts_if False

  fun combined (left right : U32) : U32 :=
    (left ||| right) ^^^ right

  fun shifted (value : U64) (amount : U8) : U64 :=
    value <<< amount

  spec shifted (value : U64) (amount : U8) where
    ensures result.toNat = (value.toNat <<< amount.toNat) % U64.size;
    aborts_if ¬amount.toNat < 64 with Semantics.Checked.arithmeticAbortCode

  fun halved (value : U16) : U16 :=
    value >>> (1 : U8)

  fun complex_constant : U64 :=
    COMPLEX

  fun signed_complex_constant : I64 :=
    SIGNED_COMPLEX

  fun shifted_complex_constant : U64 :=
    SHIFTED_COMPLEX

  fun classify_primitive (value : U64) : U64 :=
    match value with
    | 0 => 10
    | 1..4 if value != 2 => 20
    | 4..=6 => 30
    | _ => 40

  fun primitive_match_effect (selector value : U64) : Action U64 := do
    pure (match selector with
      | 0 => value + 1
      | _ => 0)

  fun classify_open_range (value : U64) : U64 :=
    match value with
    | ..3 => 1
    | 3.. => 2
    | _ => 3

  fun primitive_match_reference (value : U64) : Action U64 := do
    let reference ← &value
    (match reference with
      | 0 => 1
      | 1..=9 => 2
      | _ => 3)

  spec classify_primitive (value : U64) where
    ensures result = match value with
      | 0 => 10
      | 1..4 if value != 2 => 20
      | 4..=6 => 30
      | _ => 40;
    aborts_if False

  spec primitive_match_effect (selector : U64) (value : U64) where
    ensures result = match selector with
      | 0 => value + 1
      | _ => 0;
    aborts_if (selector == 0) = true ∧ ¬value.toNat + 1 < U64.size
      with Semantics.Checked.arithmeticAbortCode

  spec classify_open_range (value : U64) where
    ensures result = match value with
      | ..3 => 1
      | 3.. => 2
      | _ => 3;
    aborts_if False

  spec primitive_match_reference (value : U64) where
    ensures result = match value with
      | 0 => 1
      | 1..=9 => 2
      | _ => 3;
    aborts_if False

  spec complex_constant where
    ensures result = 7;
    aborts_if False

  spec halved (value : U16) where
    ensures result.toNat = value.toNat >>> 1;
    aborts_if False

  /-! ## Proofs -/

  verify small_sum

  verify wide_product

  verify widen

  verify masked

  verify halved

  verify narrow

  verify shifted

  verify complex_constant by
    contract_intro
    rw [Move.Verify.wp_pure]
    constructor
    · apply MoveInt.ext
      native_decide
    · rfl

  verify classify_primitive

  verify primitive_match_effect by
    contract_intro
    cases h : (args.1 == 0) <;> simp [wp_norm, move_norm]

  verify classify_open_range

  verify primitive_match_reference

  /-! ## Tests -/

  def compiled : MoveModel.IR.Module := lowerToIR ``Tests.MovePrograms.Integers

  private def run := Tests.run compiled

  #test run "typed_u8" [] [] = Tests.okVals [.int 112]
  #test run "grouped_u64" [] [] = Tests.okU64 1_234_567

  #test run "small_sum" [] [.int 100, .int 55] = Tests.okRet [] [.int 155]
  #test run "small_sum" [] [.int 200, .int 56] = Tests.aborted 0
  #test run "wide_product" [] [.int (2 ^ 100), .int 4] =
    Tests.okRet [] [.int (2 ^ 102)]
  #test run "narrow" [] [.int 255] = Tests.okRet [] [.int 255]
  #test run "narrow" [] [.int 256] = Tests.aborted 0
  #test run "widen" [] [.int 255] = Tests.okRet [] [.int 255]
  #test run "masked" [] [.int 0b1100, .int 0b1010] =
    Tests.okRet [] [.int 0b1000]
  #test run "combined" [] [.int 0b1100, .int 0b1010] =
    Tests.okRet [] [.int 0b0100]
  #test run "shifted" [] [.int 1, .int 63] = Tests.okRet [] [.int (2 ^ 63)]
  #test run "shifted" [] [.int (2 ^ 63), .int 1] = Tests.okRet [] [.int 0]
  #test run "shifted" [] [.int 1, .int 64] = Tests.aborted 0
  #test run "halved" [] [.int 9] = Tests.okRet [] [.int 4]
  #test run "complex_constant" [] [] = Tests.okU64 7
  #test run "signed_complex_constant" [] [] = Tests.okVals [.int (-3)]
  #test run "shifted_complex_constant" [] [] = Tests.okU64 16
  #test run "classify_primitive" [] [.u64 0] = Tests.okU64 10
  #test run "classify_primitive" [] [.u64 1] = Tests.okU64 20
  #test run "classify_primitive" [] [.u64 2] = Tests.okU64 40
  #test run "classify_primitive" [] [.u64 4] = Tests.okU64 30
  #test run "classify_primitive" [] [.u64 6] = Tests.okU64 30
  #test run "classify_primitive" [] [.u64 7] = Tests.okU64 40
  #test run "primitive_match_effect" [] [.u64 1, .u64 18446744073709551615] =
    Tests.okU64 0
  #test run "primitive_match_effect" [] [.u64 0, .u64 5] = Tests.okU64 6
  #test run "primitive_match_effect" [] [.u64 0, .u64 18446744073709551615] =
    Tests.aborted 0
  #test run "classify_open_range" [] [.u64 2] = Tests.okU64 1
  #test run "classify_open_range" [] [.u64 3] = Tests.okU64 2
  #test run "primitive_match_reference" [] [.u64 0] = Tests.okU64 1
  #test run "primitive_match_reference" [] [.u64 5] = Tests.okU64 2
  #test run "primitive_match_reference" [] [.u64 10] = Tests.okU64 3

namespace Integers

open MoveModel.IR

-- The declared signatures carry the exact widths.
#guard (compiled.funDecl? "small_sum").map (·.returns) ==
  some [.uint .w8]
#guard (compiled.funDecl? "widen").map (·.returns) ==
  some [.uint .w256]
#guard (compiled.funDecl? "shifted").map (fun decl => decl.localsList.take 2) ==
  some [.uint .w64, .uint .w8]

end Integers

end Tests.MovePrograms

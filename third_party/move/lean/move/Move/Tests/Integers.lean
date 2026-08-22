-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move
import MoveModel.Tests.Common

/-! All Move integer widths: arithmetic across widths, casts, and the bit
operations, compiled and interpreted, with checked-semantics proofs for the
aborting operations. -/

namespace Tests.MovePrograms

open Move
open MoveModel.Frontend.XIR
open scoped Move Move.Compiler Move.Spec

module Integers where

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

  /-! ## Tests -/

  def compiled : MModule := module% "IntegersTest"

  private def run := Tests.run compiled

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

namespace Integers

open MoveModel.IR

-- The declared signatures carry the exact widths.
#guard (compiled.funs.find? (·.name == "small_sum")).map (·.returns) ==
  some [.uint .w8]
#guard (compiled.funs.find? (·.name == "widen")).map (·.returns) ==
  some [.uint .w256]
#guard (compiled.funs.find? (·.name == "shifted")).map (·.locals.take 2) ==
  some [.uint .w64, .uint .w8]

end Integers

end Tests.MovePrograms

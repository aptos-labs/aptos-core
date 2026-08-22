-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move
import MoveModel.Tests.Common

/-! Arithmetic abort behavior after lowering a Leaner resource program. -/

namespace Tests.MovePrograms

open Move
open MoveModel.Frontend.XIR
open scoped Move Move.Compiler Move.Spec

module Arithmetic where

  /-! ## Functions -/

  fun add_values (left right : U64) : Action U64 :=
    pure (left + right)

  spec add_values (left : U64) (right : U64) where
    ensures True;
    aborts_if ¬left.toNat + right.toNat < U64.size
      with Semantics.Checked.arithmeticAbortCode

  fun explicit_add (left right : U64) : Action U64 :=
    pure (Move.UInt.add left right)

  spec explicit_add (left : U64) (right : U64) where
    ensures True;
    aborts_if ¬left.toNat + right.toNat < U64.size
      with Semantics.Checked.arithmeticAbortCode

  fun explicit_div (left right : U64) : Action U64 :=
    pure (Move.UInt.div left right)

  spec explicit_div (left : U64) (right : U64) where
    ensures True;
    aborts_if right.toNat = 0 with Semantics.Checked.arithmeticAbortCode

  struct Counter has Key where
    value : U64

  entry fun multiply (addr : Address) (factor : U64) : Action Unit := do
    let value ← &mut Counter[addr].value
    let old ← *value
    value := old * factor

  spec multiply (addr : Address) (factor : U64) where
    requires existsAt<Counter>(addr);
    modifies Counter[addr];
    ensures
      Counter[addr].value = old(Counter[addr].value) * factor;
    aborts_if
      ¬old(Counter[addr].value).toNat * factor.toNat < U64.size
      with Semantics.Checked.arithmeticAbortCode

  entry fun divide (addr : Address) (divisor : U64) : Action Unit := do
    let value ← &mut Counter[addr].value
    let old ← *value
    value := old / divisor

  spec divide (addr : Address) (divisor : U64) where
    requires existsAt<Counter>(addr);
    modifies Counter[addr];
    ensures
      Counter[addr].value = old(Counter[addr].value) / divisor;
    aborts_if divisor.toNat = 0 with Semantics.Checked.arithmeticAbortCode

  -- Comparison spellings: `<=`, `>`, `>=`, `!=`, and a comparison in value
  -- position (`a < b` as a `Bool`) all lower to the width-agnostic `lt` /
  -- `le` / `eq` instructions.
  fun at_most (left right : U64) : Action U64 := do
    if left <= right then pure 1 else pure 0

  spec at_most (left : U64) (right : U64) where
    ensures result = if left.toNat ≤ right.toNat then 1 else 0;
    aborts_if False

  fun exceeds (left right : U64) : Action U64 := do
    if left > right then pure 1 else pure 0

  spec exceeds (left : U64) (right : U64) where
    ensures result = if right.toNat < left.toNat then 1 else 0;
    aborts_if False

  fun at_least (left right : U64) : Action U64 := do
    if left >= right then pure 1 else pure 0

  spec at_least (left : U64) (right : U64) where
    ensures result = if right.toNat ≤ left.toNat then 1 else 0;
    aborts_if False

  fun differs (left right : U64) : Action U64 := do
    if left != right then pure 1 else pure 0

  spec differs (left : U64) (right : U64) where
    ensures result = if left.toNat = right.toNat then 0 else 1;
    aborts_if False

  fun is_less (left right : U64) : Bool := left < right

  spec is_less (left : U64) (right : U64) where
    ensures result = true ↔ left.toNat < right.toNat

  /-! ## Proofs -/

  verify add_values

  verify explicit_add

  verify explicit_div

  verify at_most

  verify exceeds

  verify at_least

  verify differs

  verify is_less

  verify multiply

  verify divide

  /-! ## Tests -/

  def compiled : MModule := lowerToIR ``Tests.MovePrograms.Arithmetic

  #guard compiled.name == "Arithmetic"

  private def counterId := compiled.resourceId "Counter"
  private def memory (addr value : Nat) : MoveModel.IR.IMem :=
    [(counterId, addr, .struct [.u64 value])]
  private def run := Tests.run compiled

  #test run "add_values" [] [.u64 6, .u64 7] = Tests.okU64 13
  #test run "add_values" [] [.u64 18446744073709551615, .u64 1] = Tests.aborted 0
  #test run "explicit_add" [] [.u64 6, .u64 7] = Tests.okU64 13
  #test run "explicit_add" [] [.u64 18446744073709551615, .u64 1] = Tests.aborted 0
  #test run "explicit_div" [] [.u64 17, .u64 5] = Tests.okU64 3
  #test run "explicit_div" [] [.u64 17, .u64 0] = Tests.aborted 0
  #test run "at_most" [] [.u64 2, .u64 2] = Tests.okU64 1
  #test run "at_most" [] [.u64 3, .u64 2] = Tests.okU64 0
  #test run "exceeds" [] [.u64 3, .u64 2] = Tests.okU64 1
  #test run "exceeds" [] [.u64 2, .u64 2] = Tests.okU64 0
  #test run "at_least" [] [.u64 2, .u64 2] = Tests.okU64 1
  #test run "at_least" [] [.u64 1, .u64 2] = Tests.okU64 0
  #test run "differs" [] [.u64 1, .u64 2] = Tests.okU64 1
  #test run "differs" [] [.u64 2, .u64 2] = Tests.okU64 0
  #test run "is_less" [] [.u64 1, .u64 2] = Tests.okBool true
  #test run "is_less" [] [.u64 2, .u64 2] = Tests.okBool false
  #test run "multiply" (memory 2 6) [.address 2, .u64 7]
    = Tests.okRet (memory 2 42) []
  #test run "multiply" (memory 2 18446744073709551615) [.address 2, .u64 2]
    = Tests.abortedIn (memory 2 18446744073709551615) 0
  #test run "divide" (memory 2 17) [.address 2, .u64 5]
    = Tests.okRet (memory 2 3) []
  #test run "divide" (memory 2 17) [.address 2, .u64 0]
    = Tests.abortedIn (memory 2 17) 0
  #test run "divide" [] [.address 2, .u64 1] = Tests.aborted 0

  #emit_leaner_xir compiled

end Tests.MovePrograms

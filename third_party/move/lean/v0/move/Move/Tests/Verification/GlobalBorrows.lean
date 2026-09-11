-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
-- Test category: specification and verification.

import Move
import MoveModel.Tests.Common
open Move
open scoped Move Move.Compiler Move.Spec

/-! Direct global borrows — `&mut R[a]` / `&R[a]` without a field path — and
the borrows that chain through references: a field borrow through a
whole-resource reference, an element borrow through the active `&mut Vector`
parameter, and an element-field borrow of a local vector.  Each function is
verified from its retained source. -/

module GlobalBorrows where
  struct Counter has Key where
    value : U64

  struct Pair has Copy, Drop, Store where
    left : U64
    right : U64

  -- `&mut R[a]`: the whole resource is replaced through the reference.
  entry fun replace (addr : Address) (amount : U64) : Action Unit := do
    let counter ← &mut Counter[addr]
    counter := { value := amount }

  spec replace (addr : Address) (amount : U64) where
    requires existsAt<Counter>(addr);
    modifies Counter[addr];
    ensures Counter[addr].value = amount;
    aborts_if False

  verify replace

  -- `&R[a]`: the whole resource is read.
  fun read_whole (addr : Address) : Action U64 := do
    let counter ← &Counter[addr]
    let current ← *counter
    pure current.value

  spec read_whole (addr : Address) where
    requires existsAt<Counter>(addr);
    ensures result = old(Counter[addr].value);
    aborts_if False

  verify read_whole

  -- A field borrow through the whole-resource reference: the write lands in
  -- the resource while the outer loan is still open.
  entry fun bump_through (addr : Address) : Action Unit := do
    let counter ← &mut Counter[addr]
    let value ← &mut counter.value
    value := *value + 1

  spec bump_through (addr : Address) where
    requires existsAt<Counter>(addr);
    modifies Counter[addr];
    ensures Counter[addr].value = old(Counter[addr].value) + 1;
    aborts_if ¬old(Counter[addr].value).toNat + 1 < U64.size
      with Semantics.Checked.arithmeticAbortCode

  verify bump_through

  -- An element borrow through the active `&mut Vector` parameter.
  fun bump_first (values : &mut Vector U64) : Action Unit := do
    let first ← &mut values[0]
    first := *first + 1

  spec bump_first (values : &mut Vector U64) where
    requires 0 < values.toList.length;
    ensures values.toList[0]? = some (old(values).toList[0]?.getD 0 + 1);
    aborts_if ¬(values.toList[0]?.getD 0).toNat + 1 < U64.size
      with Semantics.Checked.arithmeticAbortCode

  verify bump_first

  -- Disjoint fields of one live mutable reference may be borrowed together.
  -- Reading the first after the second is created keeps both loans live.
  fun read_siblings (pair : &mut Pair) : Action U64 := do
    let left ← &mut pair.left
    let right ← &mut pair.right
    let rightValue ← *right
    let leftValue ← *left
    pure (leftValue + rightValue)

  spec read_siblings (pair : &mut Pair) where
    ensures result = pair.left + pair.right;
    aborts_if ¬pair.left.toNat + pair.right.toNat < U64.size
      with Semantics.Checked.arithmeticAbortCode

  verify read_siblings

  fun run_read_siblings : Action U64 := do
    let pair : Pair := { left := 4, right := 7 }
    let pairRef ← &mut pair
    read_siblings pairRef

  -- An element-field borrow of a local vector, then a read of the element.
  fun bump_left : Action U64 := do
    let pairs : Vector Pair := vector![{ left := 1, right := 2 }]
    let left ← &mut pairs[0].left
    left := *left + 5
    let pair ← &pairs[0]
    let value ← *pair
    pure value.left

  spec bump_left where
    ensures result = 6;
    aborts_if False

  verify bump_left

  /-! ## Tests -/

  def compiled : MoveModel.IR.Module := lowerToIR ``GlobalBorrows

  private def counterId := compiled.resourceId "Counter"
  private def memory (addr value : Nat) : MoveModel.IR.IMem :=
    [(counterId, addr, .struct [.u64 value])]
  private def run := Tests.run compiled

  #test run "replace" (memory 7 1) [.address 7, .u64 5] = Tests.okRet (memory 7 5) []
  #test run "read_whole" (memory 7 9) [.address 7] = Tests.okRet (memory 7 9) [.u64 9]
  #test run "bump_through" (memory 7 9) [.address 7] = Tests.okRet (memory 7 10) []
  #test run "run_read_siblings" [] [] = Tests.okU64 11
  #test run "bump_left" [] [] = Tests.okU64 6

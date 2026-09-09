-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
-- Test category: specification and verification.

import Move
import MoveModel.Tests.Common
open Move
open scoped Move Move.Compiler Move.Spec

/-! The explicitly spelled core primitives under `verify`: the borrow
primitives, `read`/`readImm`/`freeze`/`write`, `Move.abort`, and the checked
vector accessors `Move.Vector.get`/`Move.Vector.set`.  They lower to the same
operations as the surface forms (`&mut R[a].f`, `*r`, `r := v`, `abort c`),
and are given the same semantics. -/

module CorePrimitives where
  struct Counter has Key where
    value : U64

  -- `borrowGlobalMut`, `borrowFieldMut`, `read`, `write`.
  entry fun explicit_read_write (addr : Address) (amount : U64) : Action Unit := do
    let value ← borrowGlobalMut Counter addr
    let slot ← borrowFieldMut value (fieldOfProjection Counter.value)
    let old ← read slot
    write slot (old + amount)

  spec explicit_read_write (addr : Address) (amount : U64) where
    requires existsAt<Counter>(addr);
    modifies Counter[addr];
    ensures Counter[addr].value = old(Counter[addr].value) + amount;
    aborts_if ¬old(Counter[addr].value).toNat + amount.toNat < U64.size
      with Semantics.Checked.arithmeticAbortCode

  verify explicit_read_write

  -- `borrowLocalMut`, `write`, `freeze`, `readImm`.
  fun explicit_local (value : U64) : Action U64 := do
    let slot ← borrowLocalMut value
    write slot 7
    let view ← freeze slot
    readImm view

  spec explicit_local (value : U64) where
    ensures result = 7;
    aborts_if False

  verify explicit_local

  -- `Move.abort` in statement position.
  fun explicit_abort (value : U64) : Action U64 := do
    if value < 10 then Move.abort 4
    pure value

  spec explicit_abort (value : U64) where
    ensures result = value;
    aborts_if value.toNat < 10 with 4

  verify explicit_abort

  -- Checked element access in value position: the bounds abort is sequenced.
  fun vector_get (index : U64) : Action U64 := do
    let values : Vector U64 := vector![10, 20, 30]
    pure (Move.Vector.get values index)

  spec vector_get (index : U64) where
    ensures result = (vector![10, 20, 30] : Vector U64).toList[index.toNat]?.getD 0;
    aborts_if 3 ≤ index.toNat with Semantics.Vector.indexOutOfBounds

  verify vector_get

  fun vector_set (index : U64) : Action (Vector U64) := do
    let values : Vector U64 := vector![10, 20, 30]
    pure (Move.Vector.set values index 7)

  spec vector_set (index : U64) where
    ensures result.toList = [10, 20, 30].set index.toNat 7;
    aborts_if 3 ≤ index.toNat with Semantics.Vector.indexOutOfBounds

  verify vector_set

  /-! ## Tests -/

  def compiled : MoveModel.IR.Module := lowerToIR ``CorePrimitives

  private def counterId := compiled.resourceId "Counter"
  private def memory (addr value : Nat) : MoveModel.IR.IMem :=
    [(counterId, addr, .struct [.u64 value])]
  private def run := Tests.run compiled

  #test run "explicit_read_write" (memory 7 9) [.address 7, .u64 4] =
    Tests.okRet (memory 7 13) []
  #test run "explicit_local" [] [.u64 3] = Tests.okU64 7
  #test run "explicit_abort" [] [.u64 3] = Tests.aborted 4
  #test run "explicit_abort" [] [.u64 30] = Tests.okU64 30
  #test run "vector_get" [] [.u64 1] = Tests.okU64 20
  #test run "vector_set" [] [.u64 1] =
    Tests.okVals [.vector [.u64 10, .u64 7, .u64 30]]

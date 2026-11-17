-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexE2ETests.CheckSupport

/-!
# Frames of a literal closure over a model's meaning

In a unit verified through its model, what a function-typed parameter without
`modifies_of` owes is stated over the model's invocation meaning
(`KeepsMemoryAtOf`). A literal site's routing is stated at the base frame, so
a generic callee's obligation, which stands at the frame its type arguments
induce, is first carried to the outer frame by the meaning's coherence
(`FramedAtOf.of_instantiate`). A closure that writes memory owes it all the
same, and does not meet it.
-/

namespace VerexSL.Tests.Check.Closures.ModelFrames

verex module 0x4f::model_frames where
  struct «Ghost$calls» has Copy, Drop, Store, Key where
    v : u64

  spec «Ghost$calls» where
    pragma ghost_memory

  struct Counter has Key where
    value : u64

  fun apply_once {T}(f : Fn(T) -> T, x : T) -> T := invoke(f, x)

  spec apply_once where
    pragma opaque
    pragma verify = false
    aborts_if aborts_of<f>(x)
    ensures ensures_of<f>(x, result)

  fun apply_once_u64(f : Fn(u64) -> u64, x : u64) -> u64 := invoke(f, x)

  spec apply_once_u64 where
    pragma opaque
    pragma verify = false
    aborts_if aborts_of<f>(x)
    ensures ensures_of<f>(x, result)

  fun halve(v : u64) -> u64 := v / 2

  fun bump(v : u64) -> u64 := do
    let counter := &mut Counter[@0x4f]
    counter.value := v
    v

  spec bump where
    modifies global<Counter>(@0x4f)
    aborts_if !exists<Counter>(@0x4f)
    ensures result == v

  -- A generic callee's obligation stands at the frame its type arguments
  -- induce; the closure keeps memory at the outer frame.
  fun generic_caller(x : u64) -> u64 := apply_once(function[Fn(u64) -> u64](halve), x)

  spec generic_caller where
    pragma verify

  -- The same callee without type arguments: the obligation already stands at
  -- the base frame, where the routing is stated.
  fun monomorphic_caller(x : u64) -> u64 :=
    apply_once_u64(function[Fn(u64) -> u64](halve), x)

  spec monomorphic_caller where
    pragma verify

  -- A closure that writes a resource does not keep memory, which the
  -- callee's function-typed parameter owes.
  fun writing_caller(x : u64) -> u64 :=
    apply_once(function[Fn(u64) -> u64](bump), x) -- error: the closure writes global memory

  spec writing_caller where
    pragma verify

end VerexSL.Tests.Check.Closures.ModelFrames

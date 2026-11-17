-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexE2ETests.CheckSupport

/-!
# The frame of an invocation the proof does not see

An invocation of a function-valued parameter is modelled at the frame the
value's assumption gives: a parameter without access declarations keeps
global memory, and a parameter whose frame a caller declares keeps what that
frame leaves.  The frame is a fact of a run to a valid result, stated over
the carriers of the invocation itself, so the memory a loop of invocations
does not touch is read back after it, and a helper that declares exactly the
frame it forwards keeps it.  A frame that is open, or memory the caller
writes itself, is rejected by the clause it breaks.
-/

namespace VerexSL.Tests.Check.Closures.UnseenFrames

verex module 0x42::unseen_frames where
  struct R has Key where
    x : u64

  struct Counter has Key where
    x : u64

  -- A parameter without access declarations keeps memory across a loop of
  -- its invocations, so the read after the loop is the read before it.
  public fun pure_framed(f : Fn(u64) -> u64 has Copy, Drop, addr : Address, n : u64) -> u64 := do
    let mut i := 0u64
    while i < n do
      let _ := invoke(f, i)
      i := i + 1u64
    where
      invariant i <= n
    R[addr].x
  spec pure_framed where
    pragma aborts_if_is_partial
    requires exists<R>(addr)
    ensures R[addr].x == old(R[addr].x)

  -- A helper that declares exactly the frame it forwards: what that frame
  -- leaves, the invocation keeps.
  public fun forwarded_frame(f : Fn(Address) -> Unit has Copy, Drop, a : Address, addr : Address)
      -> Unit :=
    invoke(f, a)
  spec forwarded_frame where
    pragma aborts_if_is_partial
    requires exists<R>(addr)
    modifies_of<f>(x : Address) global<Counter>(x)
    modifies global<Counter>(a)
    ensures R[addr].x == old(R[addr].x)

  -- The caller's own write is not the parameter's: the frame of the
  -- invocation says nothing about it.
  public fun pure_framed_fails(f : Fn(u64) -> u64 has Copy, Drop, addr : Address, n : u64) -> u64 := do
    let _ := invoke(f, n)
    R[addr].x := R[addr].x + 1u64
    R[addr].x
  spec pure_framed_fails where
    pragma aborts_if_is_partial
    requires exists<R>(addr)
    modifies global<R>(addr)
    ensures R[addr].x == old(R[addr].x) -- error: the caller writes the memory itself

  -- A forwarded frame is of the memory it names: another resource is not
  -- kept by it.
  public fun forwarded_frame_fails(f : Fn(Address) -> Unit has Copy, Drop, a : Address) -> Unit :=
    invoke(f, a)
  spec forwarded_frame_fails where
    pragma aborts_if_is_partial
    modifies_of<f> *
    ensures exists<Counter>(a) == old(exists<Counter>(a)) -- error: `f` may change any memory
    modifies *

end VerexSL.Tests.Check.Closures.UnseenFrames

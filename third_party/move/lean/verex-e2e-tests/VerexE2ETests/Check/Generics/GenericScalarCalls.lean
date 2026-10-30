-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-!
Concrete scalar callers consume the generic callee's one typed theorem: the
callees are opaque, so callers reason through their contracts. The result's
canonical decoder supplies its runtime shape by a shared lemma; the caller
neither reverifies the body nor splits every runtime constructor.
-/

namespace VerexSL.Tests.Check.Generics.GenericScalarCalls

verex module 0x42::generic_scalar_calls where
  fun carry {T}(value : T) -> T := value
  spec carry where
    pragma opaque
    ensures result == value
    aborts_if false

  fun carry_u8(value : u8) -> u8 := core.call carry::<u8>(value)
  spec carry_u8 where
    ensures result == value
    aborts_if false

  fun carry_u64(value : u64) -> u64 := core.call carry::<u64>(value)
  spec carry_u64 where
    ensures result == value
    aborts_if false

  fun carry_bool(value : Bool) -> Bool := core.call carry::<Bool>(value)
  spec carry_bool where
    ensures result == value
    aborts_if false

  fun carry_address(value : Address) -> Address := core.call carry::<Address>(value)
  spec carry_address where
    ensures result == value
    aborts_if false

  fun carry_string(value : string) -> string := core.call carry::<string>(value)
  spec carry_string where
    ensures result == value
    aborts_if false

  fun carry_signer(value : Signer) -> Signer := core.call carry::<Signer>(value)
  spec carry_signer where
    ensures result == value
    aborts_if false

  fun carry_bytes(value : Bytes) -> Bytes := core.call carry::<Bytes>(value)
  spec carry_bytes where
    ensures result == value
    aborts_if false

  -- An omitted abort condition in a strict specification has no authored
  -- Obligation marker. Consuming the summary must not require one.
  fun carry_strict {T}(value : T) -> T := value
  spec carry_strict where
    pragma opaque
    pragma aborts_if_is_strict
    ensures result == value

  fun carry_u64_strict(value : u64) -> u64 := core.call carry_strict::<u64>(value)
  spec carry_u64_strict where
    pragma aborts_if_is_strict
    ensures result == value

  -- A callee's result at an integer instance carries the type's range, with
  -- no clause relating it to an argument.
  fun unrelated {T}(value : T) -> T := value
  spec unrelated where
    pragma opaque
    aborts_if false

  fun widened(value : u128) -> u256 := (core.call unrelated::<u128>(value)) as u256 + 1
  spec widened where
    aborts_if false
    ensures result <= 340282366920938463463374607431768211456

-- Each concrete caller's proof reuses its generic callee's theorem instead of
-- re-proving the callee at the concrete type.
open Lean Elab Command in
run_cmd do
  let base := `VerexSL.Tests.Check.Generics.GenericScalarCalls.«0x42».generic_scalar_calls
  for (caller, callee) in [("carry_u8", "carry"), ("carry_u64", "carry"),
      ("carry_bool", "carry"), ("carry_address", "carry"), ("carry_string", "carry"),
      ("carry_signer", "carry"), ("carry_bytes", "carry"), ("carry_u64_strict", "carry_strict")] do
    let name := base ++ Name.mkSimple caller ++ `typedVerified
    -- The theorem is declared as the proof attempt that proved it: the
    -- constants the proof uses are those of the caller's own theorems.
    let mut pending := [name]
    let mut visited : NameSet := {}
    let mut used : NameSet := {}
    while let next :: rest := pending do
      pending := rest
      if visited.contains next then continue
      visited := visited.insert next
      let some proof := (← getEnv).find? next |>.bind (·.value? (allowOpaque := true))
        | throwError "missing native scalar caller theorem {next}"
      for constant in proof.getUsedConstants do
        used := used.insert constant
        if (base ++ Name.mkSimple caller).isPrefixOf constant then pending := constant :: pending
    unless used.contains (base ++ Name.mkSimple callee ++ `typedSemanticsVerified) do
      throwError "native scalar caller {caller} does not reuse its generic callee theorem"

end VerexSL.Tests.Check.Generics.GenericScalarCalls

-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

set_option leaner.route "native"

/-!
Concrete scalar callers consume the generic callee's one typed theorem.
The result's canonical decoder supplies its runtime shape by a shared lemma;
the caller neither reverifies the body nor splits every runtime constructor.
-/

namespace LeanerLang.Tests.VerificationGenericScalarCalls

leaner module 0x42::generic_scalar_calls where
  fun carry {T : type}(value : T) -> T := value

  spec carry where
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
  fun carry_strict {T : type}(value : T) -> T := value

  spec carry_strict where
    pragma aborts_if_is_strict
    ensures result == value

  fun carry_u64_strict(value : u64) -> u64 := core.call carry_strict::<u64>(value)

  spec carry_u64_strict where
    pragma aborts_if_is_strict
    ensures result == value

  verify carry
  verify carry_u8
  verify carry_u64
  verify carry_bool
  verify carry_address
  verify carry_string
  verify carry_signer
  verify carry_bytes
  verify carry_strict
  verify carry_u64_strict

#leaner_require_native 0x42::generic_scalar_calls::carry
#leaner_require_native 0x42::generic_scalar_calls::carry_u8
#leaner_require_native 0x42::generic_scalar_calls::carry_u64
#leaner_require_native 0x42::generic_scalar_calls::carry_bool
#leaner_require_native 0x42::generic_scalar_calls::carry_address
#leaner_require_native 0x42::generic_scalar_calls::carry_string
#leaner_require_native 0x42::generic_scalar_calls::carry_signer
#leaner_require_native 0x42::generic_scalar_calls::carry_bytes
#leaner_require_native 0x42::generic_scalar_calls::carry_strict
#leaner_require_native 0x42::generic_scalar_calls::carry_u64_strict

open Lean Elab Command in
run_cmd do
  let base := `LeanerLang.Tests.VerificationGenericScalarCalls.«0x42».generic_scalar_calls
  for (caller, callee) in [("carry_u8", "carry"), ("carry_u64", "carry"),
      ("carry_bool", "carry"), ("carry_address", "carry"), ("carry_string", "carry"),
      ("carry_signer", "carry"), ("carry_bytes", "carry"), ("carry_u64_strict", "carry_strict")] do
    let name := base ++ Name.mkSimple caller ++ `pureVerified
    let some proof := (← getEnv).find? name |>.bind (·.value? (allowOpaque := true))
      | throwError "missing native scalar caller theorem {name}"
    unless proof.getUsedConstants.contains (base ++ Name.mkSimple callee ++ `pureVerified) do
      throwError "native scalar caller {caller} does not reuse its generic callee theorem"

end LeanerLang.Tests.VerificationGenericScalarCalls

#leaner_require_native_all

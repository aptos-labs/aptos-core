-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
open VerexIR.Proofs.Denote
set_option quotPrecheck false
local notation "arguments" => (⟨.cons (.int 64 false) .nil, rfl⟩ : TypeArgs)
local notation "calleeType" => (NTy.enum (⟨⟨0⟩, 0⟩ : VerexIR.StructHandle) .nil ["End", "Value"]
  (.cons .nil (.cons (.cons (.param 0) .nil) .nil)) (by decide))
local notation "callerType" => (NTy.enum (⟨⟨0⟩, 0⟩ : VerexIR.StructHandle) .nil ["End", "Value"]
  (.cons .nil (.cons (.cons (.int 64 false) .nil) .nil)) (by decide))
-- A callee's value at its instantiated family is the caller's value at the
-- substituted type, spelled as the caller spells it.
example [family : Carriers]
    (value : @NTy.carrier (Carriers.instantiate arguments family) calleeType)
    (other : @NTy.carrier family callerType) (same : value = other) :
    @Eq (@NTy.carrier family callerType) value other := by
  fail_if_success with_reducible exact same
  verex_canonical_families
  with_reducible exact same

-- Rewriting under a binder preserves its scope.
example [family : Carriers]
    (value : @NTy.carrier (Carriers.instantiate arguments family) calleeType)
    (equal : ∀ other : @NTy.carrier (Carriers.instantiate arguments family) calleeType,
      other = value)
    (other : @NTy.carrier family callerType) :
    @Eq (@NTy.carrier family callerType) other value := by
  fail_if_success with_reducible exact equal other
  verex_canonical_families
  with_reducible exact equal other

example [family : Carriers]
    (value : @NTy.carrier (Carriers.instantiate arguments family) (.ghost calleeType))
    (other : @NTy.carrier family (.ghost callerType)) (same : value = other) :
    @Eq (@NTy.carrier family (.ghost callerType)) value other := by
  fail_if_success with_reducible exact same
  verex_canonical_families
  with_reducible exact same

-- Nested variants and collections are spelled at the caller's types too.
example [family : Carriers]
    (values : @NTy.carrier (Carriers.instantiate arguments family) (.vector calleeType))
    (others : @NTy.carrier family (.vector callerType)) (same : values = others) :
    @Eq (@NTy.carrier family (.vector callerType)) values others := by
  fail_if_success with_reducible exact same
  verex_canonical_families
  with_reducible exact same

example [family : Carriers]
    (value : @NTy.carrier (Carriers.instantiate arguments family)
      (.tuple (.cons calleeType (.cons (.ghost calleeType) .nil))))
    (other : @NTy.carrier family (.tuple (.cons callerType (.cons (.ghost callerType) .nil))))
    (same : value = other) :
    @Eq (@NTy.carrier family (.tuple (.cons callerType (.cons (.ghost callerType) .nil))))
      value other := by
  fail_if_success with_reducible exact same
  verex_canonical_families
  with_reducible exact same

-- A successful search through a caller's vector supplies the existential
-- witness for the same vector returned through a generic callee. Preparing
-- the search must also remove the element transport and rebuilt certificate.
section
local notation "addressArguments" => (⟨.cons .address .nil, rfl⟩ : TypeArgs)

example [family : Carriers] (original : VerexIR.SpecVector String)
    (returned : @NTy.carrier (Carriers.instantiate addressArguments family)
      (.vector (.param 0))) (needle : String)
    (linked : returned = @NTy.toSkolem family addressArguments (.vector (.param 0)) original)
    (found : (findIndex? (@NTy.eqb family .address) original.values needle
      original.values.size 0).isSome = true) :
    ∃ i : Int, 0 ≤ i ∧ i < returned.values.size ∧ returned.values[i.toNat]? = some needle := by
  verex_leaf
end

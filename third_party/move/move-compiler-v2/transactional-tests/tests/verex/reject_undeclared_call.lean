-- Copyright © Aptos Foundation

--# publish

import VerexMove

/-- A host Lean definition, not a declaration of the Verex module. -/
def ordinaryHelper (value : Nat) : Nat := value + value

verex module 0x0::VerexRejectUndeclaredCall where
  fun caller(value : u64) -> u64 := ordinaryHelper(value)

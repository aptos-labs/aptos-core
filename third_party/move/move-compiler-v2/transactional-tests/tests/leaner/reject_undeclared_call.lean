-- Copyright © Aptos Foundation

--# publish

import LeanerMove

/-- A host Lean definition, not a declaration of the Leaner module. -/
def ordinaryHelper (value : Nat) : Nat := value + value

leaner module 0x0::LeanerRejectUndeclaredCall where
  fun caller(value : u64) -> u64 := ordinaryHelper(value)

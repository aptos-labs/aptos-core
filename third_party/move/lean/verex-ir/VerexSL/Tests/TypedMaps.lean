-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

-- Preparation modifiers must not reserve ordinary identifiers in imported code.
example (values typed : Nat) : values + typed = typed + values := by omega

open VerexIR VerexIR.Proofs.Denote

-- The key and value types are independent. The two removal disciplines
-- preserve their distinct source-visible enumeration orders.
example : SpecMap.remove .sequence [(1, "one"), (2, "two"), (3, "three")] (1 : Int) =
    [(3, "three"), (2, "two")] := by decide

example : SpecMap.remove (.ordered SpecMap.intOrder)
    [(1, "one"), (2, "two"), (3, "three")] 1 = [(2, "two"), (3, "three")] := by decide

example : SpecMap.update (.ordered SpecMap.intOrder) [(1, true), (3, false)] 2 false =
    [(1, true), (2, false), (3, false)] := by decide

-- An absent read is the default value and creates no entry at its key.
example : SpecMap.valueAt ([] : SpecMap Int Int) 0 = 0 ∧
    SpecMap.hasKey ([] : SpecMap Int Int) 0 = false := by decide

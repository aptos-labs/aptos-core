-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

-- The fast path proves arithmetic without processing unrelated hypotheses.
example (x y : Int) (h : x ≤ y) (p : Prop) (_irrelevant : p ∨ ¬p) : x < y + 1 := by
  leaner_denote_arithmetic_only

-- Mixed propositions are deliberately outside the fast path. The full
-- solver must still be available when such a proposition supplies the bound.
example (x y : Int) (p : Prop) (h : x ≤ y ∧ p) : x ≤ y := by
  fail_if_success leaner_denote_arithmetic_only
  leaner_denote_omega

-- Call summaries can reuse one binder name. Rewriting by identity must find
-- the first scalar equation even after another `consumed` shadows it.
example (amount other : LeanerIR.SpecInt (.bits 64) false) (coins shares total : Int)
    (calculation : amount.val = coins * shares / total)
    (irrelevant : other.val = 0) (bound : amount.val ≤ 100) :
    coins * shares / total ≤ 100 := by
  have consumed := calculation
  clear calculation
  have consumed := irrelevant
  clear irrelevant
  leaner_denote_scalar_equalities
  omega

open LeanerIR LeanerIR.Proofs LeanerIR.Proofs.Denote in
-- Membership, equal cardinality and distinctness jointly rule out a failed
-- search. This is the branch pool_u64 leaves when it ignores index_of's flag.
example (map : RuntimeValue) (keys : Array String) (key : String)
    (covered : ∀ i : Int, 0 ≤ i → i < keys.size →
      Maps.hasKey map (.address ((Option.map Codec.address.encode keys[i.toNat]?).getD .unit).asString) = true)
    (cardinality : Maps.size map = keys.size)
    (distinct : ∀ i : Int, 0 ≤ i → i < keys.size → ∀ j : Int, 0 ≤ j → j < keys.size →
      ((Option.map Codec.address.encode keys[i.toNat]?).getD .unit).asString =
      ((Option.map Codec.address.encode keys[j.toNat]?).getD .unit).asString → i = j)
    (present : Maps.hasKey map (.address key) = true)
    (missing : ∀ i : Int, 0 ≤ i → i < keys.size → ¬keys[i.toNat]? = some key) :
    Maps.hasKey map (.address key) = false := by
  leaner_denote_map_coverage_prepared

open LeanerIR LeanerIR.Proofs LeanerIR.Proofs.Denote in
-- Equal cardinality is essential: without it the vector can omit a key.
example (map : RuntimeValue) (keys : Array String) (key : String)
    (_covered : ∀ i : Int, 0 ≤ i → i < keys.size →
      Maps.hasKey map (.address ((Option.map Codec.address.encode keys[i.toNat]?).getD .unit).asString) = true)
    (_distinct : ∀ i : Int, 0 ≤ i → i < keys.size → ∀ j : Int, 0 ≤ j → j < keys.size →
      ((Option.map Codec.address.encode keys[i.toNat]?).getD .unit).asString =
      ((Option.map Codec.address.encode keys[j.toNat]?).getD .unit).asString → i = j)
    (present : Maps.hasKey map (.address key) = true)
    (_missing : ∀ i : Int, 0 ≤ i → i < keys.size → ¬keys[i.toNat]? = some key) :
    Maps.hasKey map (.address key) = true := by
  fail_if_success leaner_denote_map_coverage_prepared
  exact present

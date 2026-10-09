-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

-- Concrete inputs should be substituted before solving nested modular
-- arithmetic. These shapes arise from calls between bitvector spec helpers.
set_option maxHeartbeats 10000 in
example (x : LeanerIR.SpecInt (.bits 8) false) (h : x.val = 255) :
    ((((((x.val % 256 + 256) % 256 % 256 + 256) % 256 % 256 + 256) % 256 + 1)
      % 256 + 256) % 256 % 256 + 256) % 256 = 0 := by
  leaner_denote_decide_cheap

example (x : Int) (h : -3 = x) : (x.tmod 256 + 256).tmod 256 = 253 := by
  leaner_denote_ground_arithmetic

example (x : Nat) (h : x = 255) : (x + 1) % 256 = 0 := by
  leaner_denote_ground_arithmetic

example (x : Int) (_h : x = 255) : True := by
  fail_if_success have : (x + 1) % 256 = 1 := by leaner_denote_ground_arithmetic
  trivial

example (x y : Int) (_h : x = 255) : True := by
  fail_if_success have : (x + y) % 256 = 0 := by leaner_denote_ground_arithmetic
  trivial

-- Conditional equations are proof facts, but expanding their own left side
-- would loop. Both context simplifiers must retain the fact without using it
-- as a rewrite rule, including after its condition has been discharged.
example (x y : Nat) (p : Prop) (hp : p) (h : p → x = x + y) : p ∧ x + y = x := by
  leaner_denote_simp_by_context
  have equation := h hp
  omega

example (x y : Nat) (p : Prop) (hp : p) (h : p → x = x + y) : p ∧ x + y = x := by
  leaner_simp_all only
  exact ⟨trivial, (h trivial).symm⟩

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

-- At the new endpoint the current accumulator equation rewrites inside
-- a nonlinear term. Old positions still require the quantified invariant.
example (f g : Int → Int) (acc i j limit : Int)
    (accumulator : acc = f i)
    (prior : ∀ k : Int, 0 ≤ k → k < i → f k * g k ≤ limit)
    (current : acc * g i ≤ limit) (lo : 0 ≤ j) (hi : j < i + 1) :
    f j * g j ≤ limit := by
  leaner_denote_range_instance

-- A branch can select a summand before the range invariant is instantiated.
example (f : Int → Int) (even : Int → Prop) [DecidablePred even]
    (acc i j limit : Int) (accumulator : acc = f i)
    (prior : ∀ k : Int, 0 ≤ k → k < i → f k + (if even k then 1 else 0) ≤ limit)
    (current : acc + (if even i then 1 else 0) ≤ limit)
    (lo : 0 ≤ j) (hi : j < i + 1) (branch : even j) : f j + 1 ≤ limit := by
  leaner_denote_range_instance

example (f : Int → Int) (even : Int → Prop) [DecidablePred even]
    (acc i j limit : Int) (accumulator : acc = f i)
    (prior : ∀ k : Int, 0 ≤ k → k < i → f k + (if even k then 1 else 0) ≤ limit)
    (current : acc + (if even i then 1 else 0) ≤ limit)
    (lo : 0 ≤ j) (hi : j < i + 1) (branch : ¬even j) : f j + 0 ≤ limit := by
  leaner_denote_range_instance

-- Specializing the conditional cannot manufacture its missing branch or the
-- bound at the new endpoint.
example (f : Int → Int) (p : Int → Prop) [DecidablePred p]
    (i j limit : Int)
    (_prior : ∀ k : Int, 0 ≤ k → k < i → f k + (if p k then 0 else 1) ≤ limit)
    (_current : f i + (if p i then 0 else 1) ≤ limit)
    (_lo : 0 ≤ j) (_hi : j < i + 1) : True := by
  fail_if_success have : f j + 1 ≤ limit := by leaner_denote_range_instance
  trivial

example (f : Int → Int) (p : Int → Prop) [DecidablePred p]
    (i j limit : Int)
    (_prior : ∀ k : Int, 0 ≤ k → k < i → f k + (if p k then 1 else 0) ≤ limit)
    (_lo : 0 ≤ j) (_hi : j < i + 1) (_branch : p j) : True := by
  fail_if_success have : f j + 1 ≤ limit := by leaner_denote_range_instance
  trivial

-- The fact for the old prefix does not imply the bound at the new endpoint.
example (f g : Int → Int) (acc i j limit : Int)
    (_accumulator : acc = f i)
    (_prior : ∀ k : Int, 0 ≤ k → k < i → f k * g k ≤ limit)
    (_lo : 0 ≤ j) (_hi : j < i + 1) : True := by
  fail_if_success have : f j * g j ≤ limit := by leaner_denote_range_instance
  trivial

-- Nor may an unrelated accumulator supply that bound.
example (f g : Int → Int) (acc i j limit : Int)
    (_prior : ∀ k : Int, 0 ≤ k → k < i → f k * g k ≤ limit)
    (_current : acc * g i ≤ limit) (_lo : 0 ≤ j) (_hi : j < i + 1) : True := by
  fail_if_success have : f j * g j ≤ limit := by leaner_denote_range_instance
  trivial

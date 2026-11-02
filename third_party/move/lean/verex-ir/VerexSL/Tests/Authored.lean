-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

open VerexIR VerexIR.Proofs

-- Preparation runs before clauses split. Each following step reaches both
-- branches without an `all_goals`; the clause names survive preparation.
example (flag : Bool) (n : Int) :
    Obligation "authored" 0 1 (n < n + 1) ∧
    Obligation "authored" 2 3 (n < n + 2) := by
  verex_proof [] naming [("first", "authored", 0, 1), ("second", "authored", 2, 3)] preparing
    run_tac do
      unless (← Lean.Elab.Tactic.getMainTarget).isAppOfArity ``And 2 do
        throwError "preparation ran after splitting the clauses"
    cases flag
    have shared : n < n + 1 := by omega
    verex_prepare values
  | first => exact shared
  | second => omega
  | _ => fail "lost a source clause"

-- A closed branch must not discard an unproved clause or the other case.
example (p : Prop) (flag : Bool) : True := by
  fail_if_success
    have : p ∧ True := by
      verex_proof [] preparing
        cases flag
        try trivial
      | _ => assumption
  trivial

-- The grammar leaves the ordinary source identifiers available.
example (values typed splitting : Nat) : values + typed + splitting = splitting + values + typed := by
  omega

example : SpecMap.update (.ordered SpecMap.intOrder) [(1, true), (3, false)] 2 false =
    [(1, true), (2, false), (3, false)] := by
  verex_map_simp

-- Scalar evaluation also runs after a context equality exposes a literal map.
-- Values and keys need not have the same type.
example (m : SpecMap Int Bool) (contents : m = [(1, true), (3, false)]) :
    SpecMap.size m = 2 ∧ SpecMap.hasKey m 2 = false ∧ SpecMap.valueAt m 1 = true := by
  verex_map_simp

example : True := by
  fail_if_success
    have : SpecMap.hasKey ([(1, 4)] : SpecMap Int Int) 2 = true := by
      verex_map_simp
  trivial

-- A symbolic observation is still stated in the map language.
example (m : SpecMap Int Int) (present : SpecMap.hasKey m 1 = true) :
    SpecMap.hasKey m 1 = true := by
  verex_map_simp

-- Generated call and loop metadata must reach the high-level tactic, just
-- as it reaches `verex_obligations` in an ordinary authored proof.
verex module 0x42::authored where
  fun source(x : u64) -> u64 := x
  spec source where
    pragma opaque
    aborts_if false
    ensures result == x

  fun count(limit : u64) -> u64 := do
    let mut i := 0
    while i < limit do
      i := source(i) + 1
    spec do
      invariant i <= limit
    source(i)
  spec count where
    aborts_if false
    ensures result == limit
    ensures result <= limit

verify 0x42::authored::source
verify 0x42::authored::count by lean =>
  verex_proof preparing
    verex_prepare values

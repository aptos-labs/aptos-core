-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Solve.Decide

open VerexIR VerexIR.Proofs.Denote

-- Negative: Two distinct reads can consistently return different answers.
example {α : Type} (read : α → Bool) (first second : α)
    (_yes : read first = true) (_no : read second = false) : True := by
  fail_if_success
    have : False := by verex_boolean_decisions
  trivial

-- Negative: Repeated positive observations do not refute a path.
example {α : Type} (read : α → Bool) (value : α)
    (_yes _again : read value = true) : True := by
  fail_if_success
    have : False := by verex_boolean_decisions
  trivial

-- Positive: Opposite observations of the same uninterpreted read refute the path.
example {α : Type} (read : α → Bool) (value : α)
    (yes : read value = true) (no : read value = false) : False := by
  verex_boolean_decisions

-- Positive: Equality orientation does not change the observed read.
example {α : Type} (read : α → Bool) (value : α)
    (yes : true = read value) (no : false = read value) : False := by
  verex_boolean_decisions

-- Negative: Ignoring `Prop` range proofs must still distinguish different `SpecInt` values.
example (first second : Int)
    (firstFits : IntegerValueFits (.bits 64) false first)
    (secondFits : IntegerValueFits (.bits 64) false second)
    (read : SpecInt (.bits 64) false → Bool)
    (_yes : read ⟨first, firstFits⟩ = true)
    (_no : read ⟨second, secondFits⟩ = false) : True := by
  fail_if_success
    have : False := by verex_leaf
  trivial

-- Positive: Two `SpecInt` values with the same `.val` and distinct proof terms are equal by proof irrelevance.
example (n : Int)
    (first second : IntegerValueFits (.bits 64) false n)
    (read : SpecInt (.bits 64) false → Bool)
    (yes : read ⟨n, first⟩ = true)
    (no : read ⟨n, second⟩ = false) : False := by
  verex_leaf

-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Solve.Decide

open VerexIR.Proofs.Denote

-- Opposite observations of the same uninterpreted read refute the path.
example {α : Type} (read : α → Bool) (value : α)
    (yes : read value = true) (no : read value = false) : False := by
  verex_boolean_decisions

-- Equality orientation does not change the observed read.
example {α : Type} (read : α → Bool) (value : α)
    (yes : true = read value) (no : false = read value) : False := by
  verex_boolean_decisions

-- Two distinct reads can consistently return different answers.
example {α : Type} (read : α → Bool) (first second : α)
    (_yes : read first = true) (_no : read second = false) : True := by
  fail_if_success
    have : False := by verex_boolean_decisions
  trivial

-- Repeated positive observations do not refute a path.
example {α : Type} (read : α → Bool) (value : α)
    (_yes _again : read value = true) : True := by
  fail_if_success
    have : False := by verex_boolean_decisions
  trivial

-- A transported certificate and the original certify the same plain value.
-- Their proof terms differ, while the observed value is definitionally equal.
example [Carriers] (τ : NTy) (value : τ.plain)
    (first second : τ.deepHolds value) (read : τ.carrier → Bool)
    (yes : read (NTy.ofDeepHolds τ value first) = true)
    (no : read (NTy.ofDeepHolds τ value second) = false) : False := by
  verex_boolean_decisions

-- Ignoring certificate evidence must still distinguish the certified values.
example [Carriers] (τ : NTy) (first second : τ.plain)
    (firstHolds : τ.deepHolds first) (secondHolds : τ.deepHolds second)
    (read : τ.carrier → Bool)
    (_yes : read (NTy.ofDeepHolds τ first firstHolds) = true)
    (_no : read (NTy.ofDeepHolds τ second secondHolds) = false) : True := by
  fail_if_success
    have : False := by verex_boolean_decisions
  trivial

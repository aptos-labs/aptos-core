-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-! ## A range invariant over a result its goal names

A call's rule names the result a clause reads of an invocation
(`∀ default, ResultOf … default = w`), and the step of a range invariant
stated on such reads then asks `w = v`. The range decider reads that goal
through the naming equation (`namedResultView`): at the candidate's default,
for this invocation only, and it decides the bound by the context as it
does for a goal stated on the read. The controls take the path within the
range and the path at the bound, a goal stated on the read itself, and what
the view must not accept: no naming equation, one at a single default, one
under a condition, the result of another function or of another state, a
position past the bound, and a bound of which the context states nothing.
-/

namespace VerexSL.Tests.RangeNamedResults

open VerexIR VerexIR.Proofs VerexIR.Proofs.Denote

section
variable {unit : Validation.ValidatedUnit} [FunctionCarriers unit] [Skolems unit]
variable (invoke : InvokeMeaning) (σs : NRow) (shared : Sharing) (shape : ResultShape)
  (function other : (NTy.function σs shared shape.row).carrier) (entries : Int → HList σs)
  (state later : Memory unit) (w absent fallback : shape.carrier) (i j : Int)

-- Negative: nothing names the result.
set_option linter.unusedVariables false in
example
    (prior : ∀ k : Int, 0 ≤ k → k < i →
      ResultOf invoke σs shared shape function (entries k) state fallback = absent)
    (current : ∀ default,
      ResultOf invoke σs shared shape function (entries i) state default = absent)
    (lo : 0 ≤ j) (hi : j < i + 1) : True := by
  fail_if_success have : w = absent := by verex_range_instance
  trivial

-- Negative: an equation at one default names the result at that default
-- only, and one under a condition names it where the condition holds.
set_option linter.unusedVariables false in
example
    (named : ResultOf invoke σs shared shape function (entries j) state fallback = w)
    (prior : ∀ k : Int, 0 ≤ k → k < i →
      ResultOf invoke σs shared shape function (entries k) state fallback = absent)
    (current : ∀ default,
      ResultOf invoke σs shared shape function (entries i) state default = absent)
    (lo : 0 ≤ j) (hi : j < i + 1) : True := by
  fail_if_success have : w = absent := by verex_range_instance
  trivial

set_option linter.unusedVariables false in
example (valid : Prop)
    (named : valid → ∀ default,
      ResultOf invoke σs shared shape function (entries j) state default = w)
    (prior : ∀ k : Int, 0 ≤ k → k < i →
      ResultOf invoke σs shared shape function (entries k) state fallback = absent)
    (current : ∀ default,
      ResultOf invoke σs shared shape function (entries i) state default = absent)
    (lo : 0 ≤ j) (hi : j < i + 1) : True := by
  fail_if_success have : w = absent := by verex_range_instance
  trivial

-- Negative: the name is another function's result, or the result in
-- another state.
set_option linter.unusedVariables false in
example
    (named : ∀ default, ResultOf invoke σs shared shape other (entries j) state default = w)
    (prior : ∀ k : Int, 0 ≤ k → k < i →
      ResultOf invoke σs shared shape function (entries k) state fallback = absent)
    (current : ∀ default,
      ResultOf invoke σs shared shape function (entries i) state default = absent)
    (lo : 0 ≤ j) (hi : j < i + 1) : True := by
  fail_if_success have : w = absent := by verex_range_instance
  trivial

set_option linter.unusedVariables false in
example
    (named : ∀ default, ResultOf invoke σs shared shape function (entries j) later default = w)
    (prior : ∀ k : Int, 0 ≤ k → k < i →
      ResultOf invoke σs shared shape function (entries k) state fallback = absent)
    (current : ∀ default,
      ResultOf invoke σs shared shape function (entries i) state default = absent)
    (lo : 0 ≤ j) (hi : j < i + 1) : True := by
  fail_if_success have : w = absent := by verex_range_instance
  trivial

-- Negative: a position two past the range, and a bound the context states
-- nothing of.
set_option linter.unusedVariables false in
example
    (named : ∀ default, ResultOf invoke σs shared shape function (entries j) state default = w)
    (prior : ∀ k : Int, 0 ≤ k → k < i →
      ResultOf invoke σs shared shape function (entries k) state fallback = absent)
    (current : ∀ default,
      ResultOf invoke σs shared shape function (entries i) state default = absent)
    (lo : 0 ≤ j) (hi : j < i + 2) : True := by
  fail_if_success have : w = absent := by verex_range_instance
  trivial

set_option linter.unusedVariables false in
example
    (named : ∀ default, ResultOf invoke σs shared shape function (entries j) state default = w)
    (prior : ∀ k : Int, 0 ≤ k → k < i →
      ResultOf invoke σs shared shape function (entries k) state fallback = absent)
    (lo : 0 ≤ j) (hi : j < i + 1) : True := by
  fail_if_success have : w = absent := by verex_range_instance
  trivial

-- A failed attempt leaves the goal and its context as they were.
set_option linter.unusedVariables false in
example
    (named : ∀ default, ResultOf invoke σs shared shape function (entries j) state default = w)
    (prior : ∀ k : Int, 0 ≤ k → k < i →
      ResultOf invoke σs shared shape function (entries k) state fallback = absent)
    (lo : 0 ≤ j) (hi : j < i + 1) (given : w = absent) : w = absent := by
  first | verex_range_instance | skip
  guard_target = w = absent
  guard_hyp named : ∀ default,
    ResultOf invoke σs shared shape function (entries j) state default = w
  exact given

-- The goal on the named result: within the range it is the invariant's
-- instance, read through the naming equation at the invariant's default;
-- at the bound the context states the result.
example
    (named : ∀ default, ResultOf invoke σs shared shape function (entries j) state default = w)
    (prior : ∀ k : Int, 0 ≤ k → k < i →
      ResultOf invoke σs shared shape function (entries k) state fallback = absent)
    (current : ∀ default,
      ResultOf invoke σs shared shape function (entries i) state default = absent)
    (lo : 0 ≤ j) (hi : j < i + 1) : w = absent := by
  verex_range_instance

-- The two paths apart: a position within the range needs no fact of the
-- bound, and the position at the bound none of the range.
example
    (named : ∀ default, ResultOf invoke σs shared shape function (entries j) state default = w)
    (prior : ∀ k : Int, 0 ≤ k → k < i →
      ResultOf invoke σs shared shape function (entries k) state fallback = absent)
    (lo : 0 ≤ j) (hi : j < i) : w = absent := by
  verex_range_instance

example
    (named : ∀ default, ResultOf invoke σs shared shape function (entries j) state default = w)
    (prior : ∀ k : Int, 0 ≤ k → k < i →
      ResultOf invoke σs shared shape function (entries k) state fallback = absent)
    (current : ∀ default,
      ResultOf invoke σs shared shape function (entries i) state default = absent)
    (lo : 0 ≤ j) (low : i ≤ j) (hi : j < i + 1) : w = absent := by
  verex_range_instance

-- The goal stated on the read itself.
example
    (prior : ∀ k : Int, 0 ≤ k → k < i →
      ResultOf invoke σs shared shape function (entries k) state fallback = absent)
    (current : ∀ default,
      ResultOf invoke σs shared shape function (entries i) state default = absent)
    (lo : 0 ≤ j) (hi : j < i + 1) :
    ResultOf invoke σs shared shape function (entries j) state fallback = absent := by
  verex_range_instance

end

end VerexSL.Tests.RangeNamedResults

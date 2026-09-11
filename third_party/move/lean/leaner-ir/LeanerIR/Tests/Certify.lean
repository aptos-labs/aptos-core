-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Certify

-- Native Boolean clauses are logical conditionals, not arithmetic leaves.
set_option maxHeartbeats 1000 in
example (flag left right : Bool) (selected : flag = true) :
    left = true ↔ (if flag then left = true else right = true) := by
  leaner_certified_close!

set_option maxHeartbeats 1000 in
example (flag left right : Bool) (selected : flag ≠ true) :
    right = true ↔ (if flag then left = true else right = true) := by
  leaner_certified_close!

namespace LeanerIR.Tests.Certify

open LeanerIR.Proofs

-- A disabled branch cannot require an abort, regardless of an uninspected
-- enum's variant predicate. Do not enumerate that enum to refute the clause.
set_option maxHeartbeats 1000 in
example (flag : Bool) (variant payload : Prop) [Decidable variant]
    (disabled : flag ≠ true) : ¬(if variant then flag = true ∧ payload else False) := by
  leaner_certified_close!

set_option maxHeartbeats 1000 in
example (flag : Bool) (variant payload : Prop) [Decidable variant]
    (disabled : flag ≠ true) : ¬(if variant then payload ∧ flag = true else False) := by
  leaner_certified_close!

-- Tuple-match fallthrough carries implications, not decided parameters.
-- Closing the result must not enumerate unrelated Boolean locals.
set_option maxHeartbeats 1000 in
example (left right : Bool) (_unrelated₁ _unrelated₂ _unrelated₃ : Bool)
    (first : left = true → right = false) (second : left = true → right = true) :
    (0 : Int) = if left then if right then 2 else 1 else 0 := by
  leaner_certified_close!

-- The vacuous direction must close from the newly introduced constructor
-- contradiction, without treating the other direction as an equation.
example (value : Int) : (false = true ↔ value + 1 = value) := by
  leaner_certified_close!

example (value : Int) : (true = true ↔ value + 1 > value) := by
  leaner_certified_close!

-- A caller consumes its callee's Boolean contract without splitting it
-- into directions and losing the match across storage-key spellings.
example (globals : GlobalMap) (address : String) (result : Bool)
    (callee : result = true ↔
      (globals.lookup ⟨⟨0⟩, ⟨7⟩, (RuntimeValue.address address).storageKey⟩).isSome = true) :
    result = true ↔
      (globals.lookup ⟨⟨0⟩, ⟨7⟩, StorageKey.address address⟩).isSome = true := by
  leaner_certified_close!

-- A per-family frame reduces at the key hypothesis, independently of the
-- contents or encoding of any other key.
example (contents : StorageKey → Option RuntimeValue) (key written : StorageKey)
    (value : RuntimeValue) (different : key ≠ written) :
    (if key = written then some value else contents key) = contents key := by
  leaner_certified_close!

-- Anonymous generated frame hypotheses need their own key normalization,
-- even beside other anonymous locals with the same displayed name.
set_option maxHeartbeats 1000 in
example (globals : GlobalMap) (address : String) (value : RuntimeValue) :
    ∀ (_ : RuntimeValue) (key : GlobalKey),
      key ≠ ⟨⟨0⟩, ⟨7⟩, (RuntimeValue.address address).storageKey⟩ →
      ((globals.insert ⟨⟨0⟩, ⟨7⟩, .address address⟩ (.loanHole 0)).insert
        ⟨⟨0⟩, ⟨7⟩, .address address⟩ value).lookup key = globals.lookup key := by
  leaner_certified_close!

-- A composed continuation can introduce the same premise before entering
-- the closer, rather than leaving it under a forall in the target.
set_option maxHeartbeats 1000 in
example (globals : GlobalMap) (address : String) (value : RuntimeValue) (key : GlobalKey)
    (different : key ≠ ⟨⟨0⟩, ⟨7⟩, (RuntimeValue.address address).storageKey⟩) :
    (globals.insert ⟨⟨0⟩, ⟨7⟩, .address address⟩ value).lookup key = globals.lookup key := by
  leaner_certified_close!

-- Runtime structural inequality between integer values must feed source-level
-- integer specifications without splitting the whole RuntimeValue type.
example (left right : Int)
    (different : (RuntimeValue.integer left != RuntimeValue.integer right) = true) :
    left ≠ right := by
  leaner_certified_close!

-- Address equality crosses the runtime codec without retaining the universal
-- value comparison in the logical clause.
example (left right : String)
    (equal : (RuntimeValue.address left == RuntimeValue.address right) = true) :
    left = right := by
  leaner_certified_close!

example {T : Type} (erase : T → RuntimeValue)
    (contents : StorageKey → Option T) (globals : GlobalMap)
    (namespaceId : NamespaceId) (typeId : TypeId) (key : StorageKey)
    (represented : FamilyRepresentation erase namespaceId typeId contents globals)
    (present : (globals.lookup ⟨namespaceId, typeId, key⟩).isSome = true) :
    (contents key).isSome = true := by
  leaner_certified_close!

/-- A vector branch and a different literal clause are contradictory;
the closer needs neither the vector's elements nor their range proofs. -/
example (values : Array RuntimeValue)
    (branch : values = #[.integer 103, .integer 111])
    (clause : RuntimeValue.vector values = .vector #[.integer 0]) : (1 : Int) = 2 := by
  leaner_certified_close!

/-- Negative classification clauses use the same logical vector guard. -/
example (values : Array RuntimeValue)
    (branch : values = #[.integer 103, .integer 111])
    (clause : ¬RuntimeValue.vector values = .vector #[.integer 103, .integer 111] ∧ True) :
    (1 : Int) = 0 := by
  leaner_certified_close!

end LeanerIR.Tests.Certify

namespace LeanerIR.Tests.CertifyVectors

set_option maxHeartbeats 1000 in
/-- Expose the Move vector's native bound only when a leaf needs it. -/
example (values : SpecVector α) :
    (values.values.size : Int) % 18446744073709551616 = values.values.size := by
  leaner_certified_close!

set_option maxHeartbeats 1000 in
/-- Erasure maps elements but preserves the certified length. -/
example (values : SpecVector α) (encode : α → RuntimeValue) :
    (values.values.size : Int) % 18446744073709551616 =
      Int.ofNat (values.values.map encode).size := by
  leaner_certified_close!

end LeanerIR.Tests.CertifyVectors

namespace LeanerIR.Tests.CertifySigned

open LeanerIR.Proofs

example (value : SpecInt (.bits 32) true) :
    -2147483648 ≤ value.val ∧ value.val ≤ 2147483647 := by
  leaner_certified_close!

example (value : Int) (fits : IntegerValueFits (.bits 16) true value) :
    -32768 ≤ value ∧ value ≤ 32767 := by
  leaner_certified_close!

example (left right : Int)
    (fits : IntegerValueFits (.bits 32) true (left - right)) :
    ¬ (left - right < -2147483648 ∨ left - right > 2147483647) := by
  leaner_certified_close!

example (value : Int) (overflow : -128 ≤ value → 127 < value) :
    value < -128 ∨ 127 < value := by
  leaner_certified_close!

example (value : Int)
    (overflow : -2 ^ (64 - 1) ≤ value → 2 ^ (64 - 1) - 1 < value) :
    value < -9223372036854775808 ∨ 9223372036854775807 < value := by
  leaner_certified_close!

example (left right : Int) (nonzero : right ≠ 0)
    (overflow : -2147483648 ≤ left.tdiv right → 2147483647 < left.tdiv right) :
    (right = 0 ∨ left.tdiv right < -2147483648) ∨ 2147483647 < left.tdiv right := by
  leaner_certified_close!

end LeanerIR.Tests.CertifySigned

namespace LeanerIR.Tests.CertifyBooleanEquality

open LeanerIR.Proofs

set_option maxHeartbeats 1000 in
example (left right : Bool) :
    (left == right) = true ↔ (left = true ↔ right = true) := by
  leaner_certified_close!

set_option maxHeartbeats 1000 in
example (left right : Bool) :
    (left != right) = true ↔ ¬(left = true ↔ right = true) := by
  leaner_certified_close!

set_option maxHeartbeats 1000 in
example (flag : Bool) (condition : Prop) (absent : ¬ flag = true) :
    (false = true ↔ flag = true ∧ condition) := by
  leaner_certified_close!

set_option maxHeartbeats 1000 in
example (flag : Bool) (condition : Prop) (absent : ¬ flag = true) :
    (flag = true ∧ condition ↔ false = true) := by
  leaner_certified_close!

set_option maxHeartbeats 1000 in
example (left right : Bool) : (left && right) = true ↔ left = true ∧ right = true := by
  leaner_certified_close!

set_option maxHeartbeats 1000 in
example (left right : Bool) : (left || right) = true ↔ left = true ∨ right = true := by
  leaner_certified_close!

set_option maxHeartbeats 1000 in
example (value : Bool) : (!value) = true ↔ ¬ value = true := by
  leaner_certified_close!

end LeanerIR.Tests.CertifyBooleanEquality

set_option maxHeartbeats 1000 in
example (value : Int) : (!decide (value + 1 = 2)) = true ↔ value ≠ 1 := by
  leaner_certified_close!

-- Compound native guards contain both Boolean and arithmetic facts. Split
-- only the result's conditional, without enumerating unrelated parameters.
set_option maxHeartbeats 1000 in
example (flag : Bool) (value : Int)
    (selected : flag = true ∧ value + 1 = 2) :
    (1 : Int) = if flag = true ∧ value = 1 then 1 else 0 := by
  leaner_certified_close!

set_option maxHeartbeats 1000 in
example (flag : Bool) (value : Int)
    (selected : ¬(flag = true ∧ value + 1 = 2)) :
    (0 : Int) = if flag = true ∧ value = 1 then 1 else 0 := by
  leaner_certified_close!

set_option maxHeartbeats 1000 in
example (flag : Bool) (value : Int)
    (selected : flag = true ∨ value + 1 = 2) :
    (1 : Int) = if flag = true ∨ value = 1 then 1 else 0 := by
  leaner_certified_close!

-- Refuting the wrong Boolean branch must reduce `!true`/`!false` before
-- admitting a local equation as a rewrite rule (`true = !true` would loop).
set_option maxHeartbeats 1000 in
example (result value : Bool) (computed : result = !value) :
    result = true ↔ ¬(value = true) := by
  leaner_certified_close!

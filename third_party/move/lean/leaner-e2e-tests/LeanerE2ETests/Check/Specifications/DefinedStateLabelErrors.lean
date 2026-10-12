-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-! Naming a memory neither assumes that a transition succeeds nor assumes a
predicate that appears under negation or in an implication's antecedent. -/

namespace LeanerLang.Tests.Check.Specifications.DefinedStateLabelErrors

leaner module 0x42::defined_state_label_errors where
  struct R has Key where
    value : u64

  fun negated(addr : Address) -> Unit := do
    let r := &mut R[addr]
    r.value := 1
  spec negated where
    aborts_if !exists<R>(addr)
    modifies global<R>(addr)
    ensures !(..S |~ update<R>(addr, new R { value := 1 })) &&
      (S |~ exists<R>(addr)) -- error: the update does hold
    ensures false -- error: no assumed definition can make this vacuous

  fun antecedent(addr : Address) -> Unit := do
    let r := &mut R[addr]
    r.value := 1
  spec antecedent where
    aborts_if !exists<R>(addr)
    modifies global<R>(addr)
    ensures ((..S |~ update<R>(addr, new R { value := 1 })) ==> false) &&
      (S |~ exists<R>(addr)) -- error: the antecedent holds
    ensures false -- error: the antecedent is not an assumption

  fun absent(addr : Address) -> u64 := do
    let R { value := value } := move_from<R>(addr)
    value
  spec absent where
    aborts_if false -- error: removal can abort
    modifies global<R>(addr)
    ensures (..S |~ remove<R>(addr)) && !(S |~ exists<R>(addr))

end LeanerLang.Tests.Check.Specifications.DefinedStateLabelErrors

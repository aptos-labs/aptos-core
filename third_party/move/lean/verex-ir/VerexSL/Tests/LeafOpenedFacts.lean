-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexSL.Tests.LoanDeathSupport

set_option Elab.async false

/-! ## The fact a read of storage states

A read of storage gives a plain value and, where it succeeds, the fact of that
value by the read's own law (`Skolems.holds_plainAt`), which holds of any
memory, one a write has made included. The step of the walk that introduces a
read's value states that fact beside the read's equation, so a clause about the
value read is decided from it, at the written key and at any other.

A read of a resource at a key a write of that resource does not fix keeps the
cases of the key relation the read-over-write normalization left
(`reducePlainAtWritePlain`): the leaf splits the alias and the read of each
branch states its fact there (`verex_memory_aliases`). The leaf splits an alias
only for a value its target states: a read equation whose value the target does
not mention is left whole, whatever frame, type or memory the two share.

A function's entry states one fact per parameter, so a construction owing the
invariants of a parameter's value reads that parameter's fact. The fact is
taken from the argument row's validity at the parameter's position, before the
setup normalizes it (`verex_entry_validity`), so its name is its parameter's
whichever other parameters state nothing; a parameter whose fact reads several
atoms leaves one hypothesis per atom (`verex_entry_atoms`). The Given entry
states its facts the same way.

Negatives first: a clause the invariant does not give, and a construction of a
value that does not hold. -/

open Lean Meta Elab Tactic VerexIR VerexSL

verex module 0x42::leaf_opened_facts where
  pragma verify = false

  struct Tally has Key where
    count : u64

  spec Tally where
    invariant count >= 10

  struct Share has Copy, Drop, Store where
    part : u64

  spec Share where
    invariant part <= 100

  struct Pair has Copy, Drop, Store where
    low : Share
    high : Share

  public fun read_other_key_bad(addr : Address, other : Address) -> u64 := do
    let t := &mut Tally[addr]
    t.count := t.count + 1
    Tally[other].count

  spec read_other_key_bad where
    pragma aborts_if_is_partial
    modifies global<Tally>(addr), *
    ensures result >= 11

  fun assemble_bad(low : Share) -> Pair :=
    new Pair { low, high := new Share { part := 101u64 } }

  spec assemble_bad where
    aborts_if false

  public fun read_other_key(addr : Address, other : Address) -> u64 := do
    let t := &mut Tally[addr]
    t.count := t.count + 1
    Tally[other].count

  spec read_other_key where
    pragma aborts_if_is_partial
    modifies global<Tally>(addr), *
    ensures result >= 10

  public fun read_written_key(addr : Address) -> u64 := do
    let t := &mut Tally[addr]
    t.count := t.count + 1
    Tally[addr].count

  spec read_written_key where
    pragma aborts_if_is_partial
    modifies global<Tally>(addr), *
    ensures result >= 11

  fun assemble(low : Share, high : Share) -> Pair :=
    new Pair { low, high }

  spec assemble where
    aborts_if false
    ensures result == new Pair { low, high }

  -- The first parameter states nothing.
  fun second_states(scalar : u64, item : Share) -> u64 := item.part

  spec second_states where
    aborts_if false
    ensures result <= 100

  -- A parameter in the middle states nothing.
  fun middle_free(low : Share, count : u64, high : Share) -> u64 := high.part

  spec middle_free where
    aborts_if false
    ensures result <= 100

  -- The last parameter states nothing, after one whose fact reads two atoms.
  fun trailing_free(pair : (Share, Share), tail : u64) -> u64 := pair[1u32].part

  spec trailing_free where
    aborts_if false
    ensures result <= 100

  -- A generic function, which the Given route proves where it is selected: a
  -- type parameter's value between two values that state a fact.
  fun given_between {T}(low : Share, value : T, high : Share) -> u64 := high.part

  spec given_between where
    aborts_if false
    ensures result <= 100

-- ============================================================================
-- 1. Negative controls
-- ============================================================================

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».leaf_opened_facts.read_other_key_bad ["ensures result >= 11"]
    (← `(command| verify 0x42::leaf_opened_facts::read_other_key_bad))
  expectCuts `«0x42».leaf_opened_facts.assemble_bad [("invariant part <= 100", construction)]
    (← `(command| verify 0x42::leaf_opened_facts::assemble_bad)) (some [])

-- ============================================================================
-- 2. Positive controls
-- ============================================================================

-- The read at the written key: the read gives the value written, so the clause
-- may be that value's.
verify 0x42::leaf_opened_facts::read_written_key

-- The read at another key: the write leaves the read conditional on the keys,
-- and each branch of the alias states the fact of what it reads.
verify 0x42::leaf_opened_facts::read_other_key

-- The construction out of two parameters: each parameter's fact is its own
-- hypothesis at the entry.
verify 0x42::leaf_opened_facts::assemble

-- ============================================================================
-- 3. The names of the entry's facts
-- ============================================================================

/-- An entry fact expected in the context, with the parameter it is stated of. -/
syntax entryFact := ident " of " ident

/-- The entry's facts in the context are exactly those listed, in their order,
and each states a local of its parameter and of no other parameter listed. -/
elab "expect_entry_facts " "[" entries:entryFact,* "]" : tactic => withMainContext do
  let expected ← entries.getElems.mapM fun entry => match entry with
    | `(entryFact| $fact:ident of $parameter:ident) =>
        pure (fact.getId, parameter.getId.toString)
    | _ => throwUnsupportedSyntax
  let facts := (← getLCtx).foldl (init := #[]) fun facts decl =>
    if !decl.isImplementationDetail && (decl.userName.toString.splitOn "_valid").length > 1 then
      facts.push decl
    else facts
  unless facts.map (·.userName) == expected.map (·.1) do
    throwError "the entry leaves the facts {facts.map (·.userName)}, expected {expected.map (·.1)}"
  for decl in facts, (fact, parameter) in expected do
    let type ← instantiateMVars decl.type
    let locals ← (collectFVars {} type).fvarIds.mapM (·.getUserName)
    let states (parameter : String) := locals.any fun name =>
      name.toString == parameter || name.toString.startsWith (parameter ++ "_")
    unless states parameter do
      throwError "`{fact}` states no local of `{parameter}`: {type}"
    for (_, other) in expected do
      if other != parameter && states other then
        throwError "`{fact}` states a local of `{other}`: {type}"

-- A fact is named after the parameter it is stated of, not after the first
-- parameter: the check refuses the first parameter's name for it.
verify 0x42::leaf_opened_facts::second_states by lean =>
  fail_if_success expect_entry_facts [scalar_valid of item]
  expect_entry_facts [item_valid of item]
  verex_obligations
  all_goals (verex_prepare; all_goals verex_decide)

-- A parameter stating nothing between two that state a fact shifts no name.
verify 0x42::leaf_opened_facts::middle_free by lean =>
  fail_if_success expect_entry_facts [low_valid of low, count_valid of high]
  expect_entry_facts [low_valid of low, high_valid of high]
  verex_obligations
  all_goals (verex_prepare; all_goals verex_decide)

-- The two atoms of the tuple are the tuple's, numbered in its order, with a
-- parameter stating nothing after it.
verify 0x42::leaf_opened_facts::trailing_free by lean =>
  fail_if_success expect_entry_facts [pair_valid of pair, tail_valid of pair]
  expect_entry_facts [pair_valid_1 of pair, pair_valid_2 of pair]
  verex_obligations
  all_goals (verex_prepare; all_goals verex_decide)

-- The Given entry states its facts the same way, each named after the local
-- its argument pattern binds at the fact's position; the type parameter's
-- value between the two states nothing there.
set_option verex.givenRoute true in
verify 0x42::leaf_opened_facts::given_between by lean =>
  fail_if_success expect_entry_facts [argument_0_valid of argument_0, argument_1_valid of argument_2]
  expect_entry_facts [argument_0_valid of argument_0, argument_2_valid of argument_2]
  verex_obligations
  all_goals (verex_prepare; all_goals verex_decide)

-- ============================================================================
-- 4. The alias a leaf splits
-- ============================================================================

section AliasRelevance

open VerexIR.Proofs.Denote

set_option linter.unusedVariables false

variable {unit : Validation.ValidatedUnit} [FunctionCarriers unit] [Θ : Skolems unit]

-- Negative: the leaf holds a read conditional on an alias, whose value is `v`,
-- while its target is about `w` and `w'`, values of the same frame and type:
-- the target states nothing the alias decides, and the step declines.
example (τ : NTy) (arguments : NRow) (key other : StorageKey) (memory : Memory unit)
    (written v w w' : τ.carrier)
    (read : (if other = key then some written
      else Skolems.plainAt τ arguments other memory) = some v)
    (held : w = w') : w = w' := by
  fail_if_success verex_memory_aliases
  exact held

-- Negative: the same where the value read is written as a tuple of locals,
-- whose types and frame the target shares.
example (τ σ : NTy) (arguments : NRow) (key other : StorageKey) (memory : Memory unit)
    (written : (NTy.tuple (.cons τ (.cons σ .nil))).carrier) (a w w' : τ.carrier) (b : σ.carrier)
    (read : (if other = key then some written
      else Skolems.plainAt (.tuple (.cons τ (.cons σ .nil))) arguments other memory) =
        some (a, b, ()))
    (held : w = w') : w = w' := by
  fail_if_success verex_memory_aliases
  exact held

-- Positive: the target states the value read.
example (τ : NTy) (arguments : NRow) (key other : StorageKey) (memory : Memory unit)
    (written v : τ.carrier) (P : τ.carrier → Prop)
    (read : (if other = key then some written
      else Skolems.plainAt τ arguments other memory) = some v)
    (held : ∀ value, P value) : P v := by
  verex_memory_aliases
  all_goals exact held _

-- Positive: the target states a component of the value read.
example (τ σ : NTy) (arguments : NRow) (key other : StorageKey) (memory : Memory unit)
    (written : (NTy.tuple (.cons τ (.cons σ .nil))).carrier) (a : τ.carrier) (b : σ.carrier)
    (P : σ.carrier → Prop)
    (read : (if other = key then some written
      else Skolems.plainAt (.tuple (.cons τ (.cons σ .nil))) arguments other memory) =
        some (a, b, ()))
    (held : ∀ value, P value) : P b := by
  verex_memory_aliases
  all_goals exact held _

end AliasRelevance

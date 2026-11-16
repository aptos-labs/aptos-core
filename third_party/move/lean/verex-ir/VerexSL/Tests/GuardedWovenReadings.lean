-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-! # Every guarded behavior of a statement is read

A closure site whose captured row states a data invariant states its woven
behaviors under the captures' fact (`guardedWovenAt`). No normalization holds
that fact, so the leaf reads them (`readGuardedWoven`): every application its
statements make of such a site, each at the captures it spells, where the
leaf holds those captures' fact. One statement may name several sites, or
one site at several captures, and a capture the leaf holds no fact of does
not keep the reading from a later one.

The controls run the leaf's reader itself on a leaf whose statement keeps
its applications together under a relation the reader does not open, and
compare the leaf it leaves with the statement expected of it. A value of the
declared type is the pair of its field and the unit, and the fact of a
capture is the one a parameter of that type enters a leaf with.

Negatives first: a leaf that holds no capture's fact keeps its statement.
-/

verex module 0x42::guarded_woven_readings where
  pragma verify = false

  struct Percent has Copy, Drop where
    value : u64
  spec Percent where
    invariant value <= 100

  fun scale(p : Percent, x : u64) -> u64 := p.value * x
  spec scale where
    aborts_if p.value * x > MAX_U64
    ensures result == p.value * x

  fun scale_again(p : Percent, x : u64) -> u64 := p.value * x
  spec scale_again where
    aborts_if p.value * x > MAX_U64
    ensures result == p.value * x

  struct Pair has Copy, Drop where
    first : Fn(u64) -> u64 has Copy, Drop
    second : Fn(u64) -> u64 has Copy, Drop

  -- Two sites capturing a value of the declared type. The clauses name each
  -- target as a value, which is what a site's woven behaviors are stated at.
  fun two_sites(p : Percent) -> Pair :=
    new Pair {
      first := function[Fn(u64) -> u64 has Copy, Drop](scale, p),
      second := function[Fn(u64) -> u64 has Copy, Drop](scale_again, p) }
  spec two_sites where
    aborts_if false
    ensures ∀ (x : u64),
      aborts_of<result.first>(x) == aborts_of<function[Fn(Percent, u64) -> u64](scale)>(p, x)
    ensures ∀ (x : u64),
      aborts_of<result.second>(x) ==
        aborts_of<function[Fn(Percent, u64) -> u64](scale_again)>(p, x)

/--
info: site0 : |u64| -> (u64) := scale mask 1 captures (Percent)
site1 : |u64| -> (u64) := scale_again mask 1 captures (Percent)
site2 : |Percent, u64| -> (u64) := scale mask 0 captures ()
site3 : |Percent, u64| -> (u64) := scale_again mask 0 captures ()
-/
#guard_msgs in
#verex_function_values 0x42::guarded_woven_readings

namespace VerexSL.Tests.GuardedWovenReadings

open Lean Meta Elab Tactic Command VerexIR VerexIR.Proofs VerexIR.Proofs.Denote
open «0x42».guarded_woven_readings

/-! Both capturing sites state their behaviors under the captures' fact, so
neither is skipped as a site that has nothing to read. -/

run_meta do
  for site in [``FunctionValue.site0, ``FunctionValue.site1] do
    if (guardedWovenAt (← getEnv) site).isEmpty then
      throwError "the site {site} states no behavior under its captures' fact"

/-- Whether a term states a behavior of a site that states its behaviors
under its captures' fact. -/
def statesGuardedSite (environment : Environment) (term : Lean.Expr) : Bool :=
  term.isAppOfArity ``AbortsOfEntries 10 &&
    match (term.getArg! 7).consumeMData with
    | .app (.const site _) _ => !(guardedWovenAt environment site).isEmpty
    | _ => false

/-- Runs the reader of guarded behaviors on the leaf `statement` states,
whose binders are the leaf's context, and requires the leaf it leaves to
state `expected` over the same context: a fact the reading added, or a
hypothesis it removed, is a binder `expected` does not have.

The two are compared as statements, which a site's behavior and its
target's are not equal as; and a behavior of a site the leaf read still
states is the term the leaf given stated, so what the reading does not pay
it leaves as it was. -/
elab "#guard_guarded_readings " statement:term " ⊢ " expected:term : command => liftTermElabM do
  let statement ← Term.elabType statement
  let expected ← Term.elabType expected
  Term.synthesizeSyntheticMVarsNoPostponing
  let statement ← instantiateMVars statement
  let expected ← instantiateMVars expected
  if statement != expected then
    if ← isDefEq statement expected then
      throwError "the statement expected is the statement given"
  let environment ← getEnv
  let goal ← mkFreshExprSyntheticOpaqueMVar statement
  let (_, leaf) ← goal.mvarId!.introNP statement.getNumHeadForalls
  let given ← leaf.withContext do
    let mut statements := #[← instantiateMVars (← leaf.getType)]
    for declaration in ← getLCtx do
      statements := statements.push (← instantiateMVars declaration.type)
    pure (sitesWhere (statesGuardedSite environment) statements)
  if given.isEmpty then throwError "the leaf states no behavior of a guarded site"
  let [read] ← Tactic.run leaf do
      let some next ← VerexIR.Proofs.Denote.readGuardedWoven leaf
        | throwError "the reading closed the leaf"
      setGoals [next]
    | throwError "the reading left no single leaf"
  let actual ← read.withContext do
    let mut context := #[]
    let mut statements := #[← instantiateMVars (← read.getType)]
    for declaration in ← getLCtx do
      if declaration.isImplementationDetail then continue
      context := context.push declaration.toExpr
      statements := statements.push (← instantiateMVars declaration.type)
    for kept in sitesWhere (statesGuardedSite environment) statements do
      unless given.contains kept do
        throwError "the reading left a behavior the leaf did not state{indentExpr kept}"
    instantiateMVars (← mkForallFVars context (← read.getType))
  unless ← isDefEq actual expected do
    throwError "the leaf read states{indentExpr actual}\nexpected{indentExpr expected}"

/-- `aborts_of` of a closure of one of the two sites, as a clause states it
at the entry values of the closure's argument. -/
local macro "site_aborts% " executable:term:max site:term:max entries:term:max state:term:max :
    term =>
  `(@AbortsOfEntries unit _ (Skolems.runtime unit)
      (@closureMeaning unit _ $executable (Skolems.runtime unit))
      (NRow.cons (NTy.int 64 false) NRow.nil) ⟨[false], [false]⟩
      (ResultShape.one (NTy.int 64 false)) $site $entries $state)

/-- `aborts_of` of the value naming a target, at the target's own row. -/
local macro "target_aborts% " executable:term:max target:term:max entries:term:max
    state:term:max : term =>
  `(@AbortsOfEntries unit _ (Skolems.runtime unit)
      (@closureMeaning unit _ $executable (Skolems.runtime unit))
      FunctionValue.row1 FunctionValue.sharing1 (ResultShape.one FunctionValue.type1)
      $target $entries $state)

local macro "U64" : term => `(@NTy.carrier (Carriers.runtime unit) (NTy.int 64 false))

/-- The fact of a value of the declared type, as its parameter states it. -/
local macro "valid% " value:term:max : term =>
  `(@NTy.Holds (Carriers.runtime unit) FunctionValue.type0 ($value, ()))

/-! ## No capture's fact

The leaf holds no fact of either capture: both behaviors stay. -/

#guard_guarded_readings
  ∀ (executable : Validation.ExecutableUnit unit) (p q x : U64) (state : Memory unit)
    (related : Prop → Prop → Prop),
    related (site_aborts% executable (FunctionValue.site0 ((p, ()), ())) (x, ()) state)
      (site_aborts% executable (FunctionValue.site1 ((q, ()), ())) (x, ()) state)
  ⊢
  ∀ (executable : Validation.ExecutableUnit unit) (p q x : U64) (state : Memory unit)
    (related : Prop → Prop → Prop),
    related (site_aborts% executable (FunctionValue.site0 ((p, ()), ())) (x, ()) state)
      (site_aborts% executable (FunctionValue.site1 ((q, ()), ())) (x, ()) state)

/-! ## Two sites in one statement

The goal relates the behaviors of two sites at a capture the leaf holds the
fact of: each is read at its own target. -/

#guard_guarded_readings
  ∀ (executable : Validation.ExecutableUnit unit) (p x : U64) (state : Memory unit)
    (related : Prop → Prop → Prop) (_p_valid : valid% p),
    related (site_aborts% executable (FunctionValue.site0 ((p, ()), ())) (x, ()) state)
      (site_aborts% executable (FunctionValue.site1 ((p, ()), ())) (x, ()) state)
  ⊢
  ∀ (executable : Validation.ExecutableUnit unit) (p x : U64) (state : Memory unit)
    (related : Prop → Prop → Prop) (_p_valid : valid% p),
    related (target_aborts% executable (FunctionValue.site2 ()) ((p, ()), x, ()) state)
      (target_aborts% executable (FunctionValue.site3 ()) ((p, ()), x, ()) state)

/-! ## One site at two captures, in a hypothesis

A hypothesis relates the behaviors of one site at two captures, as a
callee's postcondition leaves them, and the goal names no site: both are
read, each at its capture, and the goal stays. -/

#guard_guarded_readings
  ∀ (executable : Validation.ExecutableUnit unit) (p q x : U64) (state : Memory unit)
    (related : Prop → Prop → Prop) (conclusion : Prop)
    (_p_valid : valid% p) (_q_valid : valid% q)
    (_observed : related
      (site_aborts% executable (FunctionValue.site0 ((p, ()), ())) (x, ()) state)
      (site_aborts% executable (FunctionValue.site0 ((q, ()), ())) (x, ()) state)),
    conclusion
  ⊢
  ∀ (executable : Validation.ExecutableUnit unit) (p q x : U64) (state : Memory unit)
    (related : Prop → Prop → Prop) (conclusion : Prop)
    (_p_valid : valid% p) (_q_valid : valid% q)
    (_observed : related
      (target_aborts% executable (FunctionValue.site2 ()) ((p, ()), x, ()) state)
      (target_aborts% executable (FunctionValue.site2 ()) ((q, ()), x, ()) state)),
    conclusion

/-! ## A capture without a fact before one with a fact

The leaf holds the fact of the second capture only: the second behavior is
read, the first stays as it was, and the leaf gains no fact of the first
capture. -/

#guard_guarded_readings
  ∀ (executable : Validation.ExecutableUnit unit) (p q x : U64) (state : Memory unit)
    (related : Prop → Prop → Prop) (_q_valid : valid% q),
    related (site_aborts% executable (FunctionValue.site0 ((p, ()), ())) (x, ()) state)
      (site_aborts% executable (FunctionValue.site0 ((q, ()), ())) (x, ()) state)
  ⊢
  ∀ (executable : Validation.ExecutableUnit unit) (p q x : U64) (state : Memory unit)
    (related : Prop → Prop → Prop) (_q_valid : valid% q),
    related (site_aborts% executable (FunctionValue.site0 ((p, ()), ())) (x, ()) state)
      (target_aborts% executable (FunctionValue.site2 ()) ((q, ()), x, ()) state)

/-! The same with two sites, and with the capture that has no fact second. -/

#guard_guarded_readings
  ∀ (executable : Validation.ExecutableUnit unit) (p q x : U64) (state : Memory unit)
    (related : Prop → Prop → Prop) (_q_valid : valid% q),
    related (site_aborts% executable (FunctionValue.site0 ((p, ()), ())) (x, ()) state)
      (site_aborts% executable (FunctionValue.site1 ((q, ()), ())) (x, ()) state)
  ⊢
  ∀ (executable : Validation.ExecutableUnit unit) (p q x : U64) (state : Memory unit)
    (related : Prop → Prop → Prop) (_q_valid : valid% q),
    related (site_aborts% executable (FunctionValue.site0 ((p, ()), ())) (x, ()) state)
      (target_aborts% executable (FunctionValue.site3 ()) ((q, ()), x, ()) state)

#guard_guarded_readings
  ∀ (executable : Validation.ExecutableUnit unit) (p q x : U64) (state : Memory unit)
    (related : Prop → Prop → Prop) (_p_valid : valid% p),
    related (site_aborts% executable (FunctionValue.site0 ((p, ()), ())) (x, ()) state)
      (site_aborts% executable (FunctionValue.site0 ((q, ()), ())) (x, ()) state)
  ⊢
  ∀ (executable : Validation.ExecutableUnit unit) (p q x : U64) (state : Memory unit)
    (related : Prop → Prop → Prop) (_p_valid : valid% p),
    related (target_aborts% executable (FunctionValue.site2 ()) ((p, ()), x, ()) state)
      (site_aborts% executable (FunctionValue.site0 ((q, ()), ())) (x, ()) state)

end VerexSL.Tests.GuardedWovenReadings

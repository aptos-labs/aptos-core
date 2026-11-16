-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-! Joined reference carriers retain their lender correlations. Opening an
entry must link its tuple and constructor equalities before general search;
returned loans also retain their destinations through a loop break. -/

set_option maxHeartbeats 180000
set_option verex.verifyHeartbeats 180000
set_option verex.sharedContinuations true

verex module 0x42::shared_reference_joins where
  fun choose_three(takeFirst : Bool, takeSecond : Bool, first : &mut u64,
      second : &mut u64, third : &mut u64) -> &mut u64 :=
    if takeFirst then first
    else if takeSecond then second else third
  spec choose_three where
    ensures result == (if takeFirst then old(first)
        else if takeSecond then old(second) else old(third)) &&
      first == old(first) && second == old(second) && third == old(third)
    aborts_if false

  fun write_chosen(takeFirst : Bool, takeSecond : Bool, first : &mut u64,
      second : &mut u64, third : &mut u64) -> Unit := do
    let returned := choose_three(takeFirst, takeSecond, first, second, third)
    *returned := 31
  spec write_chosen where
    ensures if takeFirst then
        (first == 31 && second == old(second) && third == old(third))
      else if takeSecond then
        (first == old(first) && second == 31 && third == old(third))
      else (first == old(first) && second == old(second) && third == 31)
    aborts_if false

  fun return_pair(left : &mut u64, right : &mut u64) -> (&mut u64, &mut u64) :=
    (left, right)
  spec return_pair where
    ensures spec.result[0] == old(left) && spec.result[1] == old(right) &&
      left == old(left) && right == old(right)
    aborts_if false

  fun break_pair(left : &mut u64, right : &mut u64, stop : Bool) -> Unit := do
    let mut i : u64 := 0
    while i < 2 do
      let (returnedLeft, returnedRight) := return_pair(left, right)
      if stop then break
      *returnedLeft := 7
      *returnedRight := 8
      i := i + 1
    where
      invariant i <= 2
    *left := 10
    *right := 11
  spec break_pair where
    ensures left == 10 && right == 11
    aborts_if false

  fun wrong(takeFirst : Bool, takeSecond : Bool, first : &mut u64,
      second : &mut u64, third : &mut u64) -> Unit := do
    let returned := choose_three(takeFirst, takeSecond, first, second, third)
    *returned := 31
  spec wrong where
    pragma verify = false
    ensures first == old(first) && second == old(second) && third == old(third)
    aborts_if false

/--
error: the specification clause `ensures first == old(first) && second == old(second) && third == old(third)` is not established
---
error: verex verification failed: the automatic verification of `wrong` failed; provide a proof in the module:
  verify wrong by lean =>
    verex_obligations
    verex_on
    | ensures_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
verify 0x42::shared_reference_joins::wrong

/-! A write through a reference a callee yields into a slice, joined with the
path that yields none: the exit's facts are stated over the join's row, and
opening the entry puts the slice reference the arrival rebuilt under their
projections. They read the arrival's values in the walk normal form, so the
loan's end defines the slice's final contents before the callee's conditional
clause is discharged. -/

verex namespace core where
  pragma verify = false

  mod option where
    enum Option {T} has Copy where
      | None = 0
      | Some (0 : T) = 1

  mod primitive where
    impl slice where
      @[shape_only]
      native fun get_mut {T} {I} {Output}(
        _1 : &mut Slice<T>, _2 : I
      ) -> option::Option<&mut Output>

  mod primitive where
    impl slice where
      spec get_mut at {T} {usize} {T} where
        ensures _2 < old(_1).length ==> result is Some
        ensures result is Some ==> result.0 == old(_1)[_2]
        ensures _2 >= old(_1).length ==> result is None
        ensures _2 < old(_1).length ==> _1 == update(old(_1), _2, final(result.0))
        ensures _2 >= old(_1).length ==> _1 == old(_1)

verex namespace shared_slice_reference_joins where
  use core::option::Option
  use core::primitive::slice::get_mut

  fun kept(values : &mut Slice<u32>, i : usize) -> Unit := do
    let _3 := get_mut::<u32, usize, u32>(values, i)
    match core.readDiscriminant[isize](_3) with
      | 1 => do
        let x := move(_3.0)
        *x := *x + 1u32
      | 0 => ()
  spec kept where
    pragma verify = false
    ensures i < old(values).length ==> values[i] == old(values)[i]

  fun bumped(values : &mut Slice<u32>, i : usize) -> Unit := do
    let _3 := get_mut::<u32, usize, u32>(values, i)
    match core.readDiscriminant[isize](_3) with
      | 1 => do
        let x := move(_3.0)
        *x := *x + 1u32
      | 0 => ()
  spec bumped where
    pragma verify = false
    ensures i < old(values).length ==> values[i] == (old(values)[i] + 1) % 4294967296

/--
error: the specification clause `ensures i < old(values).length ==> values[i] == old(values)[i]` is not established
---
error: verex verification failed: the automatic verification of `kept` failed; provide a proof in the module:
  verify kept by lean =>
    verex_obligations
    verex_on
    | ensures_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
verify shared_slice_reference_joins::kept

verify shared_slice_reference_joins::bumped

/-! Linking rewrites an entry relation in place where it is one fact or one
predicate, and a fact it restates keeps its identity too: the hypothesis is
the one the context held before, at another type. The normalization after the
linking reads the types, so these are in the walk normal form afterwards,
while a fact the linking did not touch and the target stay exactly as they
were, whatever their form. -/

namespace LinkedFacts

opaque weight : Nat → Nat
opaque heavy : Nat → Prop

/-- An entry relation that is one equation neither side of which is a
variable or a constructor: linking keeps it as a fact. -/
def FactEntry (x y : Nat) : Prop := weight (x, 0).fst = weight y

/-- An entry relation that is one predicate. -/
def PredicateEntry (x : Nat) : Prop := heavy (x, 0).fst

open Lean Meta Elab Tactic VerexIR.Proofs.Denote in
run_cmd Command.liftTermElabM do
  let read (value : Lean.Expr) : MetaM Lean.Expr := do
    mkAppM ``Prod.fst #[← mkAppM ``Prod.mk #[value, mkNatLit 0]]
  let reads (type : Lean.Expr) : Bool := (type.find? (·.isAppOf ``Prod.fst)).isSome
  withLocalDeclD `x (mkConst ``Nat) fun x => withLocalDeclD `y (mkConst ``Nat) fun y => do
  let untouchedType ← mkEq (← read y) y
  let targetType := mkAnd (← mkEq (← read x) y) (mkConst ``True)
  let (factType, predicateType, restatedType) :=
    (← mkAppM ``FactEntry #[x, y], ← mkAppM ``PredicateEntry #[x], ← mkEq x y)
  withLocalDeclD `fact factType fun fact =>
  withLocalDeclD `predicate predicateType fun predicate =>
  withLocalDeclD `restated restatedType fun restated =>
  withLocalDeclD `untouched untouchedType fun untouched => do
    let goal := (← mkFreshExprMVar targetType).mvarId!
    let known ← getLCtx
    let [linked] ← SharedContinuations.exposeEntries goal #[``FactEntry, ``PredicateEntry]
      | throwError "linking one fact and one predicate leaves one goal"
    let linked ← linked.replaceLocalDeclDefEq restated.fvarId! (← mkEq (← read x) y)
    -- The linking kept the three identities and left a read of a pair in each.
    linked.withContext do
      for hypothesis in [fact, predicate, restated] do
        unless known.contains hypothesis.fvarId! do
          throwError "the linking gave {hypothesis} another identity"
        unless reads (← hypothesis.fvarId!.getType) do
          throwError "the linking left no read in {← hypothesis.fvarId!.getType}"
    let [after] ← Tactic.run linked (do renormalizeSubstituted (← normalization) known)
      | throwError "the normalization leaves one goal"
    after.withContext do
      let lctx ← getLCtx
      for (name, wanted) in [(`fact, ← mkEq (← mkAppM ``weight #[x]) (← mkAppM ``weight #[y])),
          (`predicate, ← mkAppM ``heavy #[x]), (`restated, ← mkEq x y)] do
        let some decl := lctx.findFromUserName? name | throwError "{name} is gone"
        unless (← instantiateMVars decl.type) == wanted do
          throwError "{name} is not in normal form after the linking: {decl.type}"
      let some decl := lctx.findFromUserName? `untouched | throwError "untouched is gone"
      unless decl.fvarId == untouched.fvarId! && decl.type == untouchedType do
        throwError "a fact the linking did not touch was changed: {decl.type}"
      unless (← after.getType) == targetType do
        throwError "the target was changed: {← after.getType}"

end LinkedFacts

/-! Two clauses share the suffix after the join, so its entry is opened for
the group before the clauses are routed to their owners. The result is read
through the join's row: once the arrival's slice reference stands in its
place, the first clause is settled by the walk normal form, and each routed
leaf's target is in that form before a decider reads it. The clause beside it
is routed as before and keeps its report, at its own source range. -/

verex namespace shared_slice_join_targets where
  use core::option::Option
  use core::primitive::slice::get_mut

  fun counted_kept(values : &mut Slice<u32>, i : usize) -> usize := do
    let _3 := get_mut::<u32, usize, u32>(values, i)
    match core.readDiscriminant[isize](_3) with
      | 1 => do
        let x := move(_3.0)
        *x := *x + 1u32
      | 0 => ()
    values.length
  spec counted_kept where
    pragma verify = false
    ensures result == old(values).length
    ensures i < old(values).length ==> values[i] == old(values)[i]

  fun counted_bumped(values : &mut Slice<u32>, i : usize) -> usize := do
    let _3 := get_mut::<u32, usize, u32>(values, i)
    match core.readDiscriminant[isize](_3) with
      | 1 => do
        let x := move(_3.0)
        *x := *x + 1u32
      | 0 => ()
    values.length
  spec counted_bumped where
    pragma verify = false
    ensures result == old(values).length
    ensures i < old(values).length ==> values[i] == (old(values)[i] + 1) % 4294967296

/--
@ +0:4...66
error: the specification clause `ensures i < old(values).length ==> values[i] == old(values)[i]` is not established
---
@ +1:7...46
error: verex verification failed: the automatic verification of `counted_kept` failed; provide a proof in the module:
  verify counted_kept by lean =>
    verex_obligations
    verex_on
    | ensures_2 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs (positions := true) in
verify shared_slice_join_targets::counted_kept

/-! The normalization of a routed leaf's target is a step of the walk: it
runs once per routed leaf, before the leaf is handed to its decisions, and
inside the run that debits the clause's owner. The solver's trace states each
normalization (`routed target`) and each owned run with the debit the
ledger took for it (`owned by`); its profile counts the steps. A leaf whose
target was normal before its decisions closes a settled clause without
opening an entry, and only the other clause's goals reach the deciders. -/

open Lean Elab Command in
/-- Runs `command` and checks that `normalized` routed targets were
normalized, each before its leaf was decided and inside an owned run whose
debit covers the normalization's cost, none of them exhausted; that
`exposures` leaves then still tried to open an entry; and that `decisions`
goals reached the deciders. -/
elab "#guard_routed_targets " normalized:num " exposures " exposures:num
    " decisions " decisions:num " in " command:command : command => do
  let environment ← getEnv
  let messages := (← get).messages
  let (output, ()) ← IO.FS.withIsolatedStreams (isolateStderr := false) <|
    elabCommand (← `(command|
      set_option verex.solverProfile true in
      set_option verex.solverDebug true in $command))
  let failed := (← get).messages.hasErrors
  -- The trace's messages are not the command's.
  modify fun state => { state with messages }
  setEnv environment
  if failed then throwError "the command reports an error"
  let lines := output.splitOn "\n"
  -- The figure in thousands of heartbeats after `start`, as the trace prints it.
  let thousands (line start : String) : Option Nat :=
    if line.startsWith start then ((line.drop start.length).takeWhile Char.isDigit).toNat? else none
  let steps (label : String) : Nat :=
    (lines.findSome? fun line => thousands line s!"  step {label} ×").getD 0
  let mut awaiting : Option Nat := none
  let mut debited := 0
  for line in lines do
    if let some cost := thousands line "  routed target: " then
      if awaiting.isSome then throwError "two normalizations of a routed target in one run"
      awaiting := some cost
    else if line.startsWith "  owned by " then
      let some cost := awaiting | continue
      let some debit := ((line.splitOn ": ").getLast?.bind (thousands · ""))
        | throwError "an owned run without its debit: {line}"
      if debit < cost || line.endsWith ", exhausted" then
        throwError "the owner's debit does not cover the normalization ({cost}k): {line}"
      awaiting := none
      debited := debited + 1
    else if line.startsWith "leaf " && awaiting.isSome then
      throwError "a routed target was normalized outside its owner's allowance, \
        before {line}"
  if awaiting.isSome then throwError "a routed target was normalized outside its owner's allowance"
  unless debited == normalized.getNat && steps "routed target" == normalized.getNat do
    throwError "{debited} routed targets normalized and debited to their owners, \
      {steps "routed target"} normalizations, expected {normalized.getNat}"
  unless steps "join exposure" == exposures.getNat do
    throwError "{steps "join exposure"} leaves tried to open an entry, expected {exposures.getNat}"
  unless steps "join decide" == decisions.getNat do
    throwError "{steps "join decide"} goals reached the deciders, expected {decisions.getNat}"

#guard_routed_targets 6 exposures 5 decisions 3 in
verify shared_slice_join_targets::counted_bumped

/-! The normalization honours the owned run it is inside. Its cost is the
debit the ledger takes from the owner. Where the owner's allowance has less
left than the normalization needs, the run ends exhausted with the target as
it was and nothing left of the allowance: the normalization recovers from
its own failures, and still neither commits its result nor works on past the
allowance. -/

namespace OwnedTargetNormalization

open Lean Meta Elab Tactic VerexIR.Proofs.Denote in
run_cmd Command.liftTermElabM do
  let reads (type : Lean.Expr) : Bool := (type.find? (·.isAppOf ``Prod.fst)).isSome
  withLocalDeclD `x (mkConst ``Nat) fun x => withLocalDeclD `y (mkConst ``Nat) fun y => do
  -- Forty clauses, each a read of a pair the normal form evaluates.
  let clause (index : Nat) : MetaM Lean.Expr := do
    mkEq (← mkAppM ``Prod.fst #[← mkAppM ``Prod.mk #[x, mkNatLit index]]) y
  let mut targetType ← clause 0
  for index in [1:40] do targetType := mkAnd targetType (← clause index)
  let normalize : TacticM Unit := do
    normalizeAround (fixedPoints := walkFixedPoints) (← normalization) #[]
  -- An owner with allowance to spare: the debit is the normalization's cost.
  let goal := (← mkFreshExprMVar targetType).mvarId!
  let ledger ← IO.mkRef (({} : SharedBudgets.Ledger).allocate 20000000).1
  let measured ← IO.mkRef (false, 0)
  let [normal] ← Tactic.run goal (do
      let (outcome, cost) ← SharedBudgets.run ledger #[0] .leaf normalize
      measured.set (outcome matches .done _, cost))
    | throwError "the normalization leaves one goal"
  let (outcome, cost) ← measured.get
  unless outcome && !reads (← instantiateMVars (← normal.getType)) do
    throwError "the target is not in normal form: {← normal.getType}"
  let owner := (← ledger.get).owners[0]!
  unless cost > 0 && owner.spent == cost && owner.remaining == some (20000000 - cost) do
    throwError "the normalization's cost {cost} is not the owner's debit {owner.spent}"
  -- The same owner with less left than the normalization needs.
  let goal := (← mkFreshExprMVar targetType).mvarId!
  let left := cost / 4
  let ledger ← IO.mkRef ({ owners := #[{ limit := 20000000, spent := 20000000 - left }] } :
    SharedBudgets.Ledger)
  let goals ← Tactic.run goal do
    let (outcome, cost) ← SharedBudgets.run ledger #[0] .leaf normalize
    measured.set (outcome matches .exhausted, cost)
  let (outcome, spent) ← measured.get
  unless outcome do throwError "an allowance of {left} carried a normalization that costs {cost}"
  unless goals == [goal] && !(← goal.isAssigned) && (← goal.getType) == targetType do
    throwError "an exhausted normalization changed its target"
  unless (← ledger.get).allowance #[0] == some 0 do
    throwError "an exhausted normalization left allowance behind"
  unless left <= spent && spent < cost do
    throwError "the normalization spent {spent} of an allowance of {left}; unbounded it costs {cost}"

end OwnedTargetNormalization

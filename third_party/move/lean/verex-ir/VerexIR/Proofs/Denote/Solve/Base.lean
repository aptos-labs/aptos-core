-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.NominalQuote
import VerexIR.Proofs.Denote.Agreement
import VerexIR.Proofs.Denote.Closures
import VerexIR.Proofs.Denote.FunctionValues
import VerexIR.Proofs.Behavior
import VerexIR.Proofs.ClosureTyping
import VerexIR.Proofs.Order
import VerexIR.Proofs.Denote.SimpAll
import VerexIR.Proofs.Maps
import VerexIR.Proofs.Denote.MapCoverage
import VerexIR.Proofs.Denote.VectorOperations
import VerexIR.Proofs.Denote.BitLift
import VerexIR.Proofs.Denote.LemmaSteps
import VerexIR.Proofs.Denote.LoopUnroll
import VerexIR.Proofs.Denote.NativeOnly
import VerexIR.Proofs.Denote.SharedContinuations
import VerexIR.Proofs.Denote.SharedBudgets
import VerexIR.Proofs.Denote.PreparationCache
import VerexIR.Proofs.Denote.SharedCases
import VerexIR.Proofs.Denote.FamilyTransport
import VerexIR.Proofs.Denote.StatedFacts
import VerexIR.Proofs.Denote.StrictCalls
import VerexIR.Proofs.Denote.NativeMaps
import VerexIR.Proofs.Denote.TypeReflection
import VerexIR.Proofs.Serialization

/-!
# The solver: options, accounting, and leaf facts

The first part of the solver (`VerexIR.Proofs.Denote.Solve`): its options,
heartbeat accounting and profiling, the provenance of obligations, the
bounds and named facts a leaf is given, and the caches of normal forms.
-/

namespace VerexIR.Proofs.Denote

open Lean Elab Tactic Meta

deriving instance ToExpr for Sharing

/-- Run a tactic on the main goal alone, the other goals kept after those it
leaves: Verex's tactics act on the main goal. Without goals it runs on none,
where a tactic such as `done` succeeds. -/
def focusGoal (x : TacticM α) : TacticM α := do
  if (← getUnsolvedGoals).isEmpty then x else focus x

/-- A transport of a value between an induced family and its outer family,
applied: the transports take the outer family's inhabitants as well
(`Carriers.instantiate`), which are synthesized here.  None where the
family has none, so that no step builds a transport it cannot type. -/
def transportAt? (transport : Name) (outer : Lean.Expr) (arguments : Array Lean.Expr) :
    MetaM (Option Lean.Expr) := do
  let nonempty? ← (try
      pure (some (← synthInstance
        (mkApp (mkConst ``VerexIR.Proofs.Denote.CarriersNonempty) outer)))
    catch _ => pure none)
  let some nonempty := nonempty? | return none
  return some (mkAppN (mkConst transport) (#[outer, nonempty] ++ arguments))

/-- A row's inhabitability built entry by entry: a type parameter's
resolution by the frame's law (`Skolems.type_inhabitable`), a resolution of
a type without parameters as that type, any other entry decided where it is
frame-free.  None where an entry is none of these. -/
partial def rowInhabitableEntries? (row : Lean.Expr) : MetaM (Option Lean.Expr) := do
  let row := row.consumeMData
  if row.isConstOf ``NRow.nil then return some (mkConst ``NRow.inhabitable_nil)
  unless row.isAppOfArity ``NRow.cons 2 do return none
  let (τ, rest) := (row.getArg! 0, row.getArg! 1)
  let resolved := τ.consumeMData
  let decided (τ : Lean.Expr) : MetaM (Option Lean.Expr) := do
    decidedFrameFree? (← mkEq (mkApp (mkConst ``NTy.inhabitable) τ) (mkConst ``Bool.true))
  let head? ← if resolved.isAppOfArity ``Skolems.resolve 4 then
      let resolvedType := resolved.appArg!.consumeMData
      if resolvedType.isAppOfArity ``NTy.param 1 then
        pure (some (mkAppN (mkConst ``Skolems.type_inhabitable)
          (resolved.getAppArgs.pop.push resolvedType.appArg!)))
      else
        -- A type without parameters resolves to itself
        -- (`Skolems.resolve_paramFree`).
        let some free ← decidedFrameFree? (← mkEq (mkApp (mkConst ``NTy.paramFree) resolvedType)
            (mkConst ``Bool.true)) | pure none
        let some held ← decided resolvedType | pure none
        let resolvedArgs := resolved.getAppArgs
        let nonempty := mkApp2 (mkConst ``Skolems.runtimeNonempty) resolvedArgs[0]! resolvedArgs[1]!
        let itself := mkAppN (mkConst ``Skolems.resolve_paramFree)
          #[resolvedArgs[0]!, resolvedArgs[1]!, mkApp nonempty resolvedArgs[2]!, resolvedArgs[2]!,
            resolvedType, free]
        pure (some (← mkEqTrans (← mkCongrArg (mkConst ``NTy.inhabitable) itself) held))
    else decided τ
  let some head := head? | return none
  let some tail ← rowInhabitableEntries? rest | return none
  return some (mkApp4 (mkConst ``NRow.inhabitable_cons) τ rest head tail)

/-- A proof that a row of type arguments has values, the premise every
statement of a declaration's data invariant of an encoding takes
(`OwnAtArguments`).  A ground row is decided; a row the frame resolves — what
a generic declaration's arguments are — is built entry by entry, a type
parameter's resolution from the frame's own law that it resolves every
parameter to an inhabitable type (`Skolems.type_inhabitable`, which is what
`NRow.inhabitable_map_resolve` proves of a whole row).  None where an entry
is neither, so that no step builds a statement it cannot type. -/
def rowInhabitable? (row : Lean.Expr) : TacticM (Option Lean.Expr) := do
  let wanted ← mkEq (mkApp (mkConst ``NRow.inhabitable) row) (mkConst ``Bool.true)
  -- A ground row is decided.  A decision over a row with a frame's
  -- resolution in it must not be taken: `Skolems.resolve` reduces for the
  -- elaborator, being a projection of a local frame, and not for the
  -- kernel, which then rejects the `Eq.refl` the decision leaves.  Such a
  -- row is read entry by entry instead, each entry's own law closing it.
  if let some decided ← decidedFrameFree? wanted then return some decided
  if let some built ← rowInhabitableEntries? row then
    if ← isTypeCorrect built then return some built
  let held ← mkFreshExprSyntheticOpaqueMVar wanted
  let proved ← (try
      let remaining ← Lean.Elab.Tactic.run held.mvarId! (evalTactic (← `(tactic|
        simp only [VerexIR.Proofs.Denote.NRow.map, VerexIR.Proofs.Denote.NRow.inhabitable,
          Bool.and_eq_true, VerexIR.Proofs.Denote.Skolems.type_inhabitable, and_true, true_and,
          VerexIR.Proofs.Denote.NRow.inhabitable_nil])))
      pure remaining.isEmpty
    catch _ => pure false)
  if proved then return some (← instantiateMVars held) else return none

/-- A program point of a term, as a number: a site is a numeral in the
compiled form, but a generator may name one with a definition of its own
(`Function.exitSite`), so a site that does not read as a numeral is reduced
and read again.  Every step reads a site through this, so that naming one
never makes a step silently skip its goal. -/
def siteOf? (site : Lean.Expr) : MetaM (Option Nat) := do
  if let some value ← (evalNat site).run then return some value
  (evalNat (← withTransparency .all (whnf site))).run

/-- What a type spells of a lent value: its family, its payload, and
whether the spelling is the worker over a plain family
(`NTy.plainOf (Carriers.toPlain Θ) τ`) rather than the wrapper
(`NTy.plain Θ τ`).  A binder's type shows the worker wherever it was built
by unfolding the wrapper, and the two are the same type, so every matcher
reads a lent type through this and rebuilds in the worker's spelling
(`PlainSpelling.tyConst`, `PlainSpelling.rowConst`,
`PlainSpelling.variantConst` over `PlainSpelling.plainFamily`), which is
the one canonical spelling of the plain layer.

The worker is canonical because it is the spelling a congruence closure
keeps.  The wrapper's heads are reducible, so a closure that normalizes at
reducible transparency unfolds them and goes on to unfold the plain family
itself into its instance literal; the worker's heads are not, so the term
it internalizes is the term the goal states.  A value read once at each
spelling therefore enters the closure as unrelated terms, and no equation
of the leaf relates them. -/
inductive PlainPayload where
  /-- One value of a type. -/
  | ty (τ : Lean.Expr)
  /-- A row of values. -/
  | row (row : Lean.Expr)
  /-- A value of one of the named variants. -/
  | variant (names rows : Lean.Expr)

/-- A lent type's spelling (`PlainPayload`). -/
structure PlainSpelling where
  /-- The family the spelling names: a `Carriers` for a wrapper, the plain
  family itself for a worker. -/
  family : Lean.Expr
  /-- Whether the spelling is the worker's. -/
  worker : Bool
  /-- What the type spells. -/
  payload : PlainPayload

/-- The lent spelling of a type, wrapper or worker alike. -/
def plainSpelling? (type : Lean.Expr) : Option PlainSpelling :=
  let type := type.consumeMData
  if type.isAppOfArity ``NTy.plain 2 then
    some ⟨type.getArg! 0, false, .ty (type.getArg! 1)⟩
  else if type.isAppOfArity ``NTy.plainOf 2 then
    some ⟨type.getArg! 0, true, .ty (type.getArg! 1)⟩
  else if type.isAppOfArity ``HListPlain 2 then
    some ⟨type.getArg! 0, false, .row (type.getArg! 1)⟩
  else if type.isAppOfArity ``HListPlainOf 2 then
    some ⟨type.getArg! 0, true, .row (type.getArg! 1)⟩
  else if type.isAppOfArity ``variantPlain 3 then
    some ⟨type.getArg! 0, false, .variant (type.getArg! 1) (type.getArg! 2)⟩
  else if type.isAppOfArity ``variantPlainOf 3 then
    some ⟨type.getArg! 0, true, .variant (type.getArg! 1) (type.getArg! 2)⟩
  else none

/-- The family of certified values a spelling's lent layer is that of: a
wrapper's own family, and the family a worker's plain family is the lent
layer of.  `none` where the worker's plain family is not one
(`Carriers.toPlain`), which the operations over certified values need. -/
def PlainSpelling.carriers? (spelling : PlainSpelling) : Option Lean.Expr :=
  if !spelling.worker then some spelling.family
  else if spelling.family.isAppOfArity ``Carriers.toPlain 1 then some spelling.family.appArg!
  else none

/-- The plain family a spelling is rebuilt over: the worker's own, and a
wrapper's family at its lent layer (`Carriers.toPlain`). -/
def PlainSpelling.plainFamily (spelling : PlainSpelling) : Lean.Expr :=
  if spelling.worker then spelling.family
  else mkApp (mkConst ``Carriers.toPlain) spelling.family

/-- The name one lent value's type is rebuilt under. -/
def PlainSpelling.tyConst : Name := ``NTy.plainOf

/-- The name a lent row's type is rebuilt under. -/
def PlainSpelling.rowConst : Name := ``HListPlainOf

/-- The name a lent variant value's type is rebuilt under. -/
def PlainSpelling.variantConst : Name := ``variantPlainOf

/-- One lent value's type in the canonical spelling. -/
def PlainSpelling.tyOf (spelling : PlainSpelling) (τ : Lean.Expr) : Lean.Expr :=
  mkApp2 (mkConst PlainSpelling.tyConst) spelling.plainFamily τ

/-- A lent row's type in the canonical spelling. -/
def PlainSpelling.rowOf (spelling : PlainSpelling) (row : Lean.Expr) : Lean.Expr :=
  mkApp2 (mkConst PlainSpelling.rowConst) spelling.plainFamily row

/-- A lent variant value's type in the canonical spelling. -/
def PlainSpelling.variantOf (spelling : PlainSpelling) (names rows : Lean.Expr) : Lean.Expr :=
  mkApp3 (mkConst PlainSpelling.variantConst) spelling.plainFamily names rows

/-- A closed decidable proposition proved by evaluation, as an auxiliary
theorem: the proof is a constant of the proposition's type, type-correct at
every transparency, where the decision's own term is correct only by
evaluating the instance, which rewriting at instances transparency and
`grind` do not. For a certificate carried inside a value, such as a
literal's bound. -/
def decidedLemma (statement : Lean.Expr) : MetaM Lean.Expr := do
  -- The theorem closes over the free variables the statement mentions
  -- (the elements of a literal vector, whose size its bound decides).
  mkAuxTheorem statement (← mkDecideProof statement)

/-- A unit's function values (`FunctionCarriers`): the instance declared
with a unit constant, or the context's of a unit variable. -/
def functionCarriersOf (unit : Lean.Expr) : MetaM Lean.Expr :=
  synthInstance (mkApp (mkConst ``FunctionCarriers) unit)

/-- The function values a frame (`Skolems unit functions`) is at. -/
def functionCarriersOfFrame (frame : Lean.Expr) : MetaM Lean.Expr := do
  let type ← whnfR (← inferType frame)
  unless type.isAppOfArity ``Skolems 2 do throwError "internal: {frame} is not a frame"
  return type.getArg! 1

/-- The frame instance in scope (`Skolems`, whose unit and function values
are outputs). -/
def frameInScope : MetaM Lean.Expr := do
  let unit ← mkFreshExprMVar (mkConst ``Validation.ValidatedUnit)
  let functions ← mkFreshExprMVar (mkApp (mkConst ``FunctionCarriers) unit)
  synthInstance (mkApp2 (mkConst ``Skolems) unit functions)

/-- `focusGoal` for a tactic sequence. -/
elab "verex_focus " steps:tacticSeq : tactic => focusGoal (evalTactic steps)

/-- Hand over the leaves of one clause as one goal over their paths. -/
register_option verex.combine : Bool := {
  defValue := true
  descr := "hand over the leaves of one clause as one goal over their paths"
}

/-- The join scheduler: automatic continuations are shared at acyclic joins
whose arrivals agree in shape. Authored obligations keep their existing path
names and ordering. -/
register_option verex.sharedContinuations : Bool := {
  defValue := true
  descr := "share automatic continuations at acyclic joins"
}

/-- Internal to the shared preparation block of `verex_obligations`: keep
source-clause markers until the walker names the remaining obligations. -/
register_option verex.keepObligationMarkers : Bool := {
  defValue := false
  descr := "preserve clause locations during shared authored preparation"
}

register_option verex.keepTargetLocals : Bool := {
  defValue := false
  descr := "substitute only the locals the goal does not read, as a path's preparation \
    leaves its clauses to their leaves"
}

/-- Show the residual goal of a clause the solver cannot establish. -/
register_option verex.certifyDebug : Bool := {
  defValue := false
  descr := "show the residual goal of a specification clause the solver cannot establish"
}

/-- Marks a function's arguments and state at its start, what the `old` of
its loop invariants reads: the verified function's, and each inlined
callee's at its call. A hypothesis rather than an equation, so that
substitution rewrites it instead of eliminating it. -/
def FunctionStart {α σ : Type} (_function : VerexIR.FunctionHandle) (_arguments : α)
    (_state : σ) : Prop := True

theorem FunctionStart.intro {α σ : Type} (function : VerexIR.FunctionHandle) (arguments : α)
    (state : σ) : FunctionStart function arguments state := trivial

/-- Marks the locals and state a state anchor at a site records, what the
in-body assertions reading it see. -/
def AnchorSaved {α σ : Type} (_site : Nat) (_env : α) (_state : σ) : Prop :=
  True

theorem AnchorSaved.intro {α σ : Type} (site : Nat) (env : α) (state : σ) :
    AnchorSaved site env state := trivial

/-- The range an `Obligation` marker names, when the expression is one. -/
def obligationRange? (e : Lean.Expr) : Option ObligationRange := do
  guard <| e.isAppOfArity ``Obligation 4
  let .lit (.strVal file) := (e.getArg! 0).consumeMData | none
  let startByte ← (e.getArg! 1).nat? <|> (e.getArg! 1).rawNatLit?
  let endByte ← (e.getArg! 2).nat? <|> (e.getArg! 2).rawNatLit?
  pure { file, startByte, endByte }

/-! ## The state of a target

The solver keeps per target the names of its clauses, the reports of its
leaves and the profile of its steps, and the caches its leaves share:
normal forms, bounds, and rewrite rules, which sibling leaves restating
one context fill for each other. Targets elaborate in tasks of their own,
so the state is the theorem's being proved, kept in the environment of its
elaboration branch (`Environment.asyncPrefix?`); a synchronous elaboration
keeps the anonymous state in the main environment. -/

/-- Where a path aborts: in a call, its number among the target's calls of
the callee and among all calls; at the target's `number`-th `abort` or
`assert!`, aborting operation, or invocation, in source order. -/
inductive AbortSite where
  | call (number : Nat) (callee : String) (overall : Nat)
  | abort (number : Nat)
  | op (number : Nat)
  | invoke (number : Nat)
  deriving BEq, Inhabited

/-- A program point a proof places facts at (`doc/designs/proof-language.md`,
"Points"): the function's entry, the entry of its `loop`-th loop or the start
of an iteration of it, where a path continues after a site, or the end of a
path. -/
inductive ProgramPointKey where
  | entry
  | exit
  | loopEntry (loop : Nat)
  | loopStep (loop : Nat)
  | after (site : AbortSite)
  deriving BEq, Inhabited

/-- The point as a proof spells it: `at entry`, `at loop 1 step`, `after
call 2 of g`, `after op 1`. -/
def ProgramPointKey.text : ProgramPointKey → String
  | .entry => "at entry"
  | .exit => "at exit"
  | .loopEntry loop => s!"at loop {loop} entry"
  | .loopStep loop => s!"at loop {loop} step"
  | .after (.call number "" _) => s!"after call {number}"
  | .after (.call number callee _) => s!"after call {number} of {callee}"
  | .after (.abort number) => s!"after abort {number}"
  | .after (.op number) => s!"after op {number}"
  | .after (.invoke number) => s!"after invoke {number}"

/-- Whether a point is one a pattern names, a call without its callee
counted among all calls. -/
def ProgramPointKey.selects (pattern point : ProgramPointKey) : Bool :=
  match pattern, point with
  | .after (.call number "" _), .after (.call _ _ overall) => number == overall
  | .after (.call number callee _), .after (.call other otherCallee _) =>
      number == other && callee == otherCallee
  | _, _ => pattern == point

/-- A returning path's obligation that no abort condition of the target holds,
which the walk meets as the marked conclusion of an authored proof's goal and
takes off (`Provenance.returns`); the proposition itself, reducibly, so that the
normal form's rules read it as they read the statement (`imp_self`, `and_true`). -/
@[reducible] def NoAbort (p : Prop) : Prop := p

/-- What a leaf owes, as a proof addresses it (`doc/designs/proof-language.md`,
"Obligation addressing and provenance"): a clause by its kind and number
among the clauses of that kind, a loop invariant at entry or at a step, a
callee's precondition by the callee's clause and the call, or the origin of
a leaf no clause names. -/
structure ObligationKey where
  /-- `ensures`, `aborts_if`, `invariant`, `assert`, …; `requires` for a
  callee's precondition; `after` for what follows a call; an origin's words
  (`data invariant`); or the name a clause of raw obligations has. -/
  kind : String
  index : Option Nat := none
  /-- `entry` or `step`, for a loop invariant. -/
  phase : Option String := none
  callee : Option String := none
  /-- The number of the call among the target's calls of the callee, in
  source order. -/
  call : Option Nat := none
  /-- The number of the loop an invariant is of, in source order, where the
  function has several. -/
  loop : Option Nat := none
  /-- The point an `assert` of a proof's point step is at. -/
  point : Option ProgramPointKey := none
  /-- The other clauses a leaf owing several at once owes: a returning path, that
  no abort condition of the target holds. -/
  also : Array Nat := #[]
  /-- The struct a data invariant's clause is of, where a proof routes by it. -/
  owner : Option String := none
  deriving BEq, Inhabited

/-- The key as a proof and a message spell it: `ensures 1`, `invariant 2
step`, `invariant 3 of loop 2 entry`, `requires 1 of iter_next at call 2`,
`after call 1 of add_shares`, `assert 1 of at entry`. -/
def ObligationKey.text (key : ObligationKey) : String :=
  let index := (key.index.map (s!" {·}")).getD "" ++
    String.join (key.also.toList.map (s!", {·}")) ++ (key.owner.map (s!" of {·}")).getD ""
  let call := (key.call.map (s!" at call {·}")).getD ""
  match key.kind, key.callee with
  | "requires", some callee => s!"requires{index} of {callee}{call}"
  | "after", some callee => match key.call with
      | some number => s!"after call {number} of {callee}"
      | none => s!"after {callee}"
  | kind, _ => s!"{kind}{index}" ++ (key.loop.map (s!" of loop {·}")).getD "" ++
      (key.phase.map (s!" {·}")).getD "" ++ (key.point.map (s!" of {·.text}")).getD ""

/-- The key as a goal's tag: its text with underscores (`ensures_1`,
`requires_1_of_iter_next_at_call_2`). -/
def ObligationKey.tag (key : ObligationKey) : Name :=
  Name.mkSimple ((key.text.replace ", " "_").map fun c => if c == ' ' then '_' else c)

/-- Whether a leaf's key is one a pattern selects: the same kind, and every
part the pattern states the same. -/
def ObligationKey.selects (pattern key : ObligationKey) : Bool :=
  pattern.kind == key.kind &&
    (pattern.index.all fun index => some index == key.index || key.also.contains index) &&
    (pattern.owner.all (some · == key.owner)) &&
    (pattern.phase.all (some · == key.phase)) &&
    (pattern.callee.all (some · == key.callee)) && (pattern.call.all (some · == key.call)) &&
    (pattern.loop.all (some · == key.loop)) &&
    -- An assert of the body is no point's.
    (match pattern.point, key.point with
      | none, none => true
      | some point, some other => point.selects other
      | _, _ => false)

/-- Whether a key is that of a function's failing paths no clause permits,
which a path's abort site tells apart. -/
def ObligationKey.isNoFailure (key : ObligationKey) : Bool :=
  key.kind == "no panic" || key.kind == "no abort"

/-- The spellings of the failure clause permitting a profile's exceptional
outcome: `aborts_if` for Move, `panics_if` for Rust
(`doc/designs/rust-verification.md`, "Failure contract"). The clause is one
condition kind of VIR, so a key keeps the spelling its target's profile
gives it. -/
def failureClauseKinds : List String := ["aborts_if", "panics_if"]

/-- Whether a key is a failure clause's, under either profile's spelling. -/
def ObligationKey.isFailureClause (key : ObligationKey) : Bool :=
  failureClauseKinds.contains key.kind

/-- A source decision on a leaf's path (`doc/designs/proof-language.md`,
"Obligation addressing and provenance"): the `number`-th `if` taken, `match`
taking a variant, loop exiting or continuing, or where the path aborts. -/
inductive Decision where
  | branch (number : Nat) (taken : Bool)
  | variant (number : Nat) (name : String)
  | loop (number : Nat) (exits : Bool)
  | aborts (site : AbortSite)
  /-- The `number`-th invocation taking the arm of a closure site of
  `target`, or of `foreign`. -/
  | invoke (number : Nat) (target : String)
  /-- The path returns: it is complete and aborts nowhere. -/
  | returns
  deriving BEq, Inhabited

/-- The decision as a selector spells it: `if 1 then`, `match 2 Some`,
`loop 1 exits`, `aborts at call 1 of add_shares`. -/
def Decision.text : Decision → String
  | .branch number taken => s!"if {number} {if taken then "then" else "else"}"
  | .variant number name => s!"match {number} {name}"
  | .loop number exits => s!"loop {number} {if exits then "exits" else "continues"}"
  | .aborts (.call number callee _) => s!"aborts at call {number} of {callee}"
  | .aborts (.abort number) => s!"aborts at abort {number}"
  | .aborts (.op number) => s!"aborts at op {number}"
  | .aborts (.invoke number) => s!"aborts at invoke {number}"
  | .invoke number target => s!"invoke {number} {target}"
  | .returns => "returns"

/-- Where a path aborts, of its decisions. -/
def abortSiteOf? (decisions : Array Decision) : Option AbortSite :=
  decisions.findSome? fun
    | .aborts site => some site
    | .branch .. | .variant .. | .loop .. | .invoke .. | .returns => none

/-- Whether a path's decision is one a selector states: the same, a call
without its callee counted among all calls. -/
def Decision.selects (pattern decision : Decision) : Bool :=
  match pattern, decision with
  | .aborts (.call number "" _), .aborts (.call _ _ overall) => number == overall
  | .aborts (.call number callee _), .aborts (.call other otherCallee _) =>
      number == other && callee == otherCallee
  | _, _ => pattern == decision


/-- Where a goal the solver splits off comes from, when no clause marker
locates it. -/
inductive Provenance where
  /-- A callee's precondition, at the callee's `call`-th call in the target's
  body where the call is the target's own. -/
  | precondition (callee : String) (call : Option Nat)
  /-- What follows a call, at the callee's `call`-th call where the call is
  the target's own. -/
  | continuation (callee : String) (call : Option Nat)
  /-- A loop's invariant at its entry, the loop's number where the target
  has several loops. -/
  | loopEntry (loop : Option Nat)
  /-- A loop's invariant after an iteration. -/
  | loopIteration (loop : Option Nat)
  | assertion
  | mutationEnd
  | construction
  /-- A value copied out of the open current of a mutable reference, which
  is certified at the copy (`doc/designs/data-invariants.md`). -/
  | copyOut
  /-- Where a mutable reference is handed over to a call: the callee
  assumes its current's data invariants at entry, so the caller owes them
  there (`CertifyKind.handOver`). -/
  | handOver
  /-- Where a mutable parameter's loan ends, at the function's exit. -/
  | exitMutationEnd
  | memoryWritten
  | callReturned
  | lemmaRequirement
  /-- A path of a loop unrolled by `pragma unroll = bound` that iterates
  further. -/
  | unrollBound (bound : Nat)
  /-- The data invariant of a value a write stores in memory
  (`DataInvariants`). -/
  | storedWrite
  /-- That an `update`'s value lies in its ghost field type's domain. -/
  | ghostWrite
  /-- The `index`-th `assert` of a proof's steps at a point. -/
  | pointAssert (point : ProgramPointKey) (index : Nat)
  /-- That no abort condition of the target holds, on a returning path. -/
  | returns
  deriving BEq

/-- A hypothesis's normal form up to the names of its free variables: the
result and its proof abstracted over the hypothesis's variables, then over
the facts of the context the proof reads, whose types are recorded
abstracted the same way. -/
structure NormalForm where
  /-- Derived rewrites may be reused only by owners charged for this pass. -/
  owners : Array SharedBudgets.OwnerId
  facts : Array Lean.Expr
  expr : Lean.Expr
  proof? : Option Lean.Expr
  /-- The side conditions the discharger left undecided, abstracted the
  same way: a context deciding one normalizes the hypothesis further. -/
  undecided : Array Lean.Expr := #[]

/-- Whether a target's goals stated a runtime value
(`doc/designs/native-verification.md`): the first found, or reported. -/
inductive RuntimeMention where
  | none
  | found (message : MessageData)
  | reported

/-- How the step block being run reads its context: at a program point,
where its facts are only given and never substituted into the goal the walk
continues over, and the scope names an `on` block reads, where an entry a
binding site gave keeps its definition only if the block reads it. -/
structure StepMode where
  pointFacts : Bool := false
  blockReads : Option (Array String) := none

/-- Where the budget stopped on `isMaxHeartbeat`: the position the walker was
at when heartbeats ran out. -/
structure BudgetStop where
  /-- Leaves decided before the budget ran out. -/
  leavesDecided : Nat := 0
  /-- The callee being stepped into and the call's number among its calls in
  this target, if the step was at or under a call. -/
  callee : Option String := none
  callNumber : Option Nat := none
  /-- The last stage charged before the budget ran out. -/
  lastStage : Option String := none

/-- A target's expansion of value structure: the entry relations opened, the
alternatives exposure returned, the branches of program splits walked, and the
locals split by their variants. -/
structure StructureCounts where
  woken : Nat := 0
  alternatives : Nat := 0
  walks : Nat := 0
  variantSplits : Nat := 0

/-- The solver's state of one target. -/
structure TargetState where
  /-- The name of each clause of the target by its source range
  (`invariant_3`, `requires_1`): a hypothesis stating a clause takes it. -/
  clauseRanges : IO.Ref (Array (ObligationRange × String))
  /-- The number of each precondition among its function's, by its source
  range: what a callee's precondition is keyed by. -/
  preconditionNumbers : IO.Ref (Array (ObligationRange × Nat))
  /-- The key of each invariant clause, by its source range: `data
  invariant` for a struct's, `module invariant` for a global one. -/
  invariantKeys : IO.Ref (Array (ObligationRange × String))
  /-- The source range of each site a failing path names, by the selector's
  words (`aborts at op 1`). -/
  failureSites : IO.Ref (Array (String × ObligationRange × Bool))
  /-- The last target's undecided leaves with their actual diagnostic
  locations, including foreign files and fallback locations. -/
  failedLeaves : IO.Ref (Array (Message × String))
  /-- The last target's failed leaves: those that exhausted an attempt's
  allowance, and those whose deciders all rejected them. -/
  exhaustedLeaves : IO.Ref Nat
  undecidedLeaves : IO.Ref Nat
  /-- The first runtime value a goal of the target states
  (`recordRuntimeValues`), reported once (`reportRuntimeMention`) outside
  the states a failed attempt restores. -/
  runtimeMention : IO.Ref RuntimeMention
  /-- The statements found native in this target, which the check passes
  over: the leaves restate the same context. Emptied for every target. -/
  nativeStatements : IO.Ref (Std.HashSet Lean.Expr)
  /-- The names of the conjuncts of the loop invariant a step assumes, in the
  conjunction's order, `none` where a conjunct is no named clause: the
  hypothesis `loopInvariant` is split into them (`recordAssumedClauses`). -/
  assumedClauses : IO.Ref (Array (Option String))
  /-- The conjuncts of the assumed invariant reading slots (`Option.elim`), by
  position, with how many clauses each holds: the rest of the conjunction
  continues after them. -/
  assumedSlotSizes : IO.Ref (Array (Nat × Nat))
  /-- Per solver step label: runs, failures, heartbeats in succeeding runs, and
  heartbeats in failing runs, under `verex.solverProfile`. -/
  stepProfile : IO.Ref (Std.HashMap String (Nat × Nat × Nat × Nat))
  /-- Under `verex.solverProfile`, the labels and starting heartbeats of the
  `timedStep`s running, outermost first. -/
  activeSteps : IO.Ref (Array (String × Nat))
  /-- Under `verex.solverProfile`, `activeSteps` where the heartbeat limit was
  last met, until a step completes after it. -/
  interruptedSteps : IO.Ref (Array (String × Nat))
  /-- Preparation requests are shared only during an automatic shared-region
  walk. Authored tactics and subsequent verification targets have no access
  to its cache; nested invocations restore their enclosing cache. -/
  preparationProofs : IO.Ref (Option (IO.Ref PreparationCache.Cache))
  /-- Encoding equalities also serve authored preparation after the walker
  hands over its goals. Dependencies and paying owners delimit their reuse. -/
  familyProofs : IO.Ref (PreparationCache.Cache)
  /-- A request is reusable only by the owners paying for its derivation.
  An empty set disables requests outside an owned operation, including the
  temporary ledger used before clause allocation. -/
  preparationOwners : IO.Ref (Array SharedBudgets.OwnerId)
  /-- The statements of bounds with their literals reduced, in this target:
  the same certificates are asserted at every leaf. Emptied for every target. -/
  reducedBounds : IO.Ref (Std.HashMap Lean.Expr Lean.Expr)
  /-- The elements a context's hypotheses read, by the types of those hypotheses. A
  witness search asks once per premise of the same context. -/
  statedReads : IO.Ref (Std.HashMap (Array Lean.Expr) (Array (Lean.Expr × Lean.Expr)))
  /-- The hypothesis types in which no projection of a constructor
  application remained to reduce, in this target: the leaves restate the
  same context, and traversing a large contract fact for nothing is what the
  pass would cost at each of them. Emptied for every target. -/
  projectionFreeTypes : IO.Ref (Std.HashSet Lean.Expr)
  /-- The premises a call's consumption could not decide, with the
  propositions connected to them: under the same facts the same premise
  stays undecided, and every later call would ask it again. Emptied for
  every target. -/
  undecidedPremises : IO.Ref (Std.HashSet (Lean.Expr × Array Lean.Expr))
  /-- The goals the ground evaluation failed on, by content, with the
  heartbeats left when it failed (`none`: unbounded): with no more left, it
  fails again, as the cheap deciders would retry it right after the leaf's
  own attempt. Emptied for every target. -/
  groundFailures : IO.Ref (Std.HashMap (Lean.Expr × Array Lean.Expr) (Option Nat))
  /-- The integer locals substituted by their pinned values so far, which a
  step reads to see whether it fixed one. -/
  pinnedCount : IO.Ref (Nat)
  /-- The bounds asserted on a goal's ancestors, by the goal's content: the
  rewriting pass restates each bound, or drops it as known, so asserting it
  again would only be undone by the next pass. Emptied for every target. -/
  assertedBounds : IO.Ref (Std.HashMap (Lean.Expr × Array Lean.Expr) (Array Lean.Expr))
  /-- The hypothesis types a normalization pass of a prepared leaf left
  normal in this target, up to the names of their free variables: the leaves
  share their context, and restate it over the variables each destructures,
  so a hypothesis normal at one leaf is not normalized again at the next,
  where the pass would traverse it for nothing. Emptied for every target. -/
  normalHypotheses : IO.Ref (Std.HashSet (Lean.Expr × Array Lean.Expr))
  /-- The simp contexts of the normalization passes over the context
  (`normalizeUncached`), by pass, built once per target: a pass's `simp only`
  set names global lemmas and simp sets only, the same at every call. -/
  normalSimpContexts : IO.Ref (Array (Nat × Simp.Context × Simp.SimprocsArray))
  /-- Under `verex.solverDebug`: the discharger's calls, their heartbeats,
  and the conditions asked. -/
  dischargeLog : IO.Ref (Nat × Nat × Array String)
  /-- The hypotheses a stage's `simp at *` normalization already left normal,
  by shape: the stage set differs from the leaf set, so this is a cache of
  its own. -/
  stageNormalHypotheses : IO.Ref (Std.HashSet (Lean.Expr × Array Lean.Expr))
  /-- The normal forms a prepared leaf's normalization computed in this
  target, by the hypothesis normalized (`normalKey`): sibling leaves restate
  one context over the variables each destructures, so a hypothesis
  normalized at one leaf is not normalized again at the next. A result
  depends on the hypothesis and on the facts that decided its side
  conditions, which the next leaf's context is searched for, and is taken
  while that context decides none of the conditions left undecided. Emptied
  for every target. -/
  normalForms : IO.Ref (Std.HashMap (Lean.Expr × Array Lean.Expr) NormalForm)
  /-- The rewrites of hypotheses by the memory reads a leaf fixes
  (`verex_context_memory_reads`), as `normalForms`, by the hypothesis
  together with the reads. Emptied for every target. -/
  readForms : IO.Ref (Std.HashMap (Lean.Expr × Array Lean.Expr) NormalForm)
  /-- The propositions the theorem binds (`verex_proof`): facts of the
  unit, the same at every leaf, which no leaf's facts connect through. -/
  moduleFacts : IO.Ref (Std.HashSet FVarId)
  /-- Whether a hypothesis type reads the slot a read equates, by both
  (`mentionsRead`): the leaves restate one context and its reads, and the
  comparison unfolds well-formedness proofs. Emptied for every target. -/
  readMentions : IO.Ref (Std.HashMap (Lean.Expr × Lean.Expr) Bool)
  /-- The hypotheses the family normalization of selected cases already left
  normal, by shape. Emptied for every target. -/
  familyNormalHypotheses : IO.Ref (Std.HashSet (Lean.Expr × Array Lean.Expr))
  /-- The rewrite rules a hypothesis gives `simpByContext`, by the hypothesis
  and its type: none for one that would rewrite a term into one containing
  it. A leaf's deciders rewrite by much the same context several times, and
  its siblings by the context they share. Emptied for every target. -/
  contextRules : IO.Ref (Std.HashMap (FVarId × Lean.Expr) (Array SimpTheorem))
  /-- The reports of the target's undecided leaves and the clauses they
  rejected: once a clause fails on one path, its other paths of the same
  origin are not decided again. Emptied for every target. -/
  leafReports : IO.Ref (Array (ObligationRange × String) × Array (Option Provenance × ObligationRange))
  /-- What each leaf the obligations hand over owes, for its report. Emptied
  for every target. -/
  leafOrigins : IO.Ref (Std.HashMap MVarId (Option Provenance × Array ObligationRange))
  /-- What each handed-over leaf owes, by its tag: what `verex_on` selects
  by. -/
  leafKeys : IO.Ref (Std.HashMap Name ObligationKey)
  /-- Each selector of the proof's `verex_on` alternatives, by its syntax:
  its text, whether it took a path, and the decisions of the paths of the
  obligations it names. A proof can run an alternative on several goals, one
  per case of an enum parameter. -/
  selectorUses : IO.Ref (Array (Syntax × String × Bool × Array String))
  /-- The simp contexts of the proof steps' normal form, by their call, built
  once per target. -/
  stepSimpContexts : IO.Ref (Array (Nat × Simp.Context × Simp.SimprocsArray))
  /-- The simp theorems of a hypothesis as a rule of the proof steps' normal
  form, by the hypothesis, the direction it is read in and whether it
  rewrites before its subterms: a function of the hypothesis. -/
  stepRuleTheorems : IO.Ref (Std.HashMap (FVarId × Bool × Bool) (Array SimpTheorem))
  /-- The rules of the proof steps' normal form at a goal, by the goal: a
  function of its context, its kinds of rules in the steps' order. -/
  stepBlockRules : IO.Ref (Std.HashMap MVarId (Array (Array FVarId)))
  /-- The normal forms of the proof steps' statements, by the statement, the
  rules, and the context's locals: a function of these, so that a statement
  normalized again in the same context (a lemma's premises, the premises of
  the context's implications at every `decide`) is normalized once. -/
  stepNormalForms : IO.Ref (Std.HashMap
    (Lean.Expr × Array FVarId × Array FVarId × Array FVarId × Array FVarId)
    (Lean.Expr × Option Lean.Expr))
  /-- The source decisions on each handed-over leaf's path, and on each
  path of a clause's goal once taken apart: what a selector of `verex_on`
  reads. -/
  leafDecisions : IO.Ref (Std.HashMap MVarId (Array Decision))
  /-- The accessible names of hypotheses a path's decisions introduced, which a
  proof may read and so keep their names (`isPathDecision`). -/
  decisionNames : IO.Ref NameSet
  /-- The paths of a clause's goal: each leaf's tag, its origin, which of
  the binders it quantifies over were implementation details, and its
  decisions. -/
  leafPaths : IO.Ref (Std.HashMap MVarId
    (Array (Name × Option Provenance × Array ObligationRange × Array Bool × Array Decision)))
  /-- The normal forms of callee contracts, by the contract and the pattern of
  arguments they were found for: the closed proof of each condition's normal
  form, and the constants the proofs use. A constant a backtracked attempt
  added is absent from the environment; its proofs are found again. -/
  contractNormalForms : IO.Ref (Std.HashMap Lean.Expr (Array Lean.Expr × Array Name))
  /-- The expressions whose family spellings are canonical
  (`verex_canonical_families`): a pass skips them, as their canonical
  spelling is themselves. -/
  canonicalForms : IO.Ref (Std.HashSet Lean.Expr)
  /-- The pairs a respelling of families compared, and whether they convert;
  the same terms recur across a target's hypotheses. -/
  familyConversions : IO.Ref (Std.HashMap (Lean.Expr × Lean.Expr) Bool)
  /-- The contract patterns met once (`contractNormalForms`): a pattern's
  normal forms are found where it is met again, so that a callee called once
  is normalized at its call, without the cost of normal forms. -/
  contractPatternsMet : IO.Ref (Std.HashSet Lean.Expr)
  /-- The theorem being proved (`verex_proof`), which declares the
  auxiliary theorems its proof uses (`declareHiddenTheorems`). -/
  proofName : IO.Ref Name
  /-- The auxiliary theorems hiding the target's proofs (`hiddenTheorem`), by
  name: a backtracked attempt removes one from the environment while a proof
  a cache keeps still uses it, which the theorem then declares again
  (`declareHiddenTheorems`). -/
  hiddenTheorems : IO.Ref (Std.HashMap Name Declaration)
  /-- The auxiliary theorems hidden so far: each takes a kind of its own
  (`hiddenTheorem`), as an attempt that is undone frees its name. -/
  hiddenCount : IO.Ref Nat
  /-- The auxiliary theorem of each closed statement: an attempt that is
  undone removes it with `mkAuxTheorem`'s cache, and the next proof of the
  statement takes it again. -/
  hiddenStatements : IO.Ref (Std.HashMap (List Name × Lean.Expr) Name)
  /-- The names of a step proof's scope, which get scope entries where they
  are bound (`ScopeEntry`), and those of its mutable references; none for
  any other proof. -/
  scopeNames : IO.Ref (Option (List String × List String))
  /-- The source names of a step proof's locals by declaration index, where
  a borrow's prophecy reads the name of the local it borrows. -/
  scopeLocals : IO.Ref (Array String)
  /-- The row of a step proof's locals: a source `let` of another frame (an
  inlined callee's) binds no name of the scope. -/
  scopeRow : IO.Ref (Option Lean.Expr)
  /-- The names a source `let` or the exit binds (`bindLet`, `bindResult`):
  a block keeps their definitions only where it reads them. -/
  bindingNames : IO.Ref (Array String)
  /-- The goals a point's `assert` owes, by goal, with the point and the
  assert's number: obligations the walk queues (`.pointAssert`). -/
  pointOwed : IO.Ref (Std.HashMap MVarId (ProgramPointKey × Nat))
  /-- The obligations a proof's `on` steps take (`verex_step_routing`), and
  whether an `on _` takes the rest: a path is prepared once for them
  (`verex_step_prepare`) where one of its clauses is among them. -/
  routedKeys : IO.Ref (Option (Array ObligationKey × Bool))
  /-- Whether the path being prepared has a clause a proof's `on` step takes. -/
  pathRouted : IO.Ref Bool
  /-- How the step block being run reads its context. -/
  stepMode : IO.Ref StepMode
  /-- The statement of each closure site's generated `closure` equation over
its captures, its literals unfolded (`literalValue?`), by constructor. -/
  siteClosureTypes : IO.Ref (NameMap Lean.Expr)
  /-- The terms a simp context left unchanged without reading the local
  context, by the context's stamp (`FixedPoints`). -/
  fixedPoints : IO.Ref (Std.HashMap Name (IO.Ref (Std.HashSet Lean.Expr)))
  /-- The results of the dsimp calls of a simp context that read no local
  context, by the context's stamp (`FixedPoints`). -/
  dsimpResults : IO.Ref (Std.HashMap Name (IO.Ref (Std.HashMap Lean.Expr Lean.Expr)))
  /-- The fixed-point lookups that hit and missed, and the terms recorded;
  then the same for dsimp calls. -/
  fixedPointCounts : IO.Ref (Array Nat)
  /-- How far this target expanded value structure. -/
  structureCounts : IO.Ref StructureCounts
  /-- Where the budget stopped when the last target hit `isMaxHeartbeat`,
  populated by `traverse` and read via `ProofReport`. -/
  budgetStop : IO.Ref (Option BudgetStop)
  /-- The parameters a derived summary is stated over (`deriveSummary`),
  which no substitution or case split of its walk eliminates: a leaf's
  conjunct relates what its path introduced to them by the equations kept
  instead, where a parameter eliminated would leave its components
  unrelated to the call. -/
  summaryParameters : IO.Ref FVarIdSet
  /-- The locals of an open shared continuation's fold context (`bindStep`),
  which the walk keeps alike, so that the node's arrivals share that
  context. A leaf is compared with no arrival: its pipeline lifts them
  (`withSharedPinsLifted`). -/
  sharedPins : IO.Ref FVarIdSet
  /-- The facts the walk keeps on purpose rather than substitutes: a call's
  contract frame, other than a frame keeping memory, which is substituted.
  The behavior statements over its final memory read it. A shared continuation's arrival preparation
  and the pins' substitution never eliminate a local through them
  (`SharedContinuations.prepareArrivals`, `substitutePinEquations`): the
  suffix and its leaves read them as the default walk leaves them. -/
  keptFacts : IO.Ref FVarIdSet
  /-- The names of contract calls' frame premises, until the
  walk introduces them as `keptFacts` (`noteKeptPremise`). -/
  keptPremises : IO.Ref NameSet

/-- The state a target begins with. -/
def TargetState.new : IO TargetState := do
  return {
    clauseRanges := ← IO.mkRef #[]
    preconditionNumbers := ← IO.mkRef #[]
    invariantKeys := ← IO.mkRef #[]
    failureSites := ← IO.mkRef #[]
    failedLeaves := ← IO.mkRef #[]
    exhaustedLeaves := ← IO.mkRef 0
    undecidedLeaves := ← IO.mkRef 0
    runtimeMention := ← IO.mkRef .none
    nativeStatements := ← IO.mkRef {}
    assumedClauses := ← IO.mkRef #[]
    assumedSlotSizes := ← IO.mkRef #[]
    stepProfile := ← IO.mkRef {}
    activeSteps := ← IO.mkRef #[]
    interruptedSteps := ← IO.mkRef #[]
    preparationProofs := ← IO.mkRef none
    familyProofs := ← IO.mkRef {}
    preparationOwners := ← IO.mkRef #[]
    reducedBounds := ← IO.mkRef {}
    statedReads := ← IO.mkRef {}
    projectionFreeTypes := ← IO.mkRef {}
    undecidedPremises := ← IO.mkRef {}
    groundFailures := ← IO.mkRef {}
    pinnedCount := ← IO.mkRef 0
    assertedBounds := ← IO.mkRef {}
    normalHypotheses := ← IO.mkRef {}
    normalSimpContexts := ← IO.mkRef #[]
    dischargeLog := ← IO.mkRef (0, 0, #[])
    stageNormalHypotheses := ← IO.mkRef {}
    normalForms := ← IO.mkRef {}
    readForms := ← IO.mkRef {}
    moduleFacts := ← IO.mkRef {}
    readMentions := ← IO.mkRef {}
    familyNormalHypotheses := ← IO.mkRef {}
    contextRules := ← IO.mkRef {}
    leafReports := ← IO.mkRef (#[], #[])
    leafOrigins := ← IO.mkRef {}
    leafKeys := ← IO.mkRef {}
    leafDecisions := ← IO.mkRef {}
    decisionNames := ← IO.mkRef {}
    selectorUses := ← IO.mkRef #[]
    stepSimpContexts := ← IO.mkRef #[]
    stepRuleTheorems := ← IO.mkRef {}
    stepBlockRules := ← IO.mkRef {}
    stepNormalForms := ← IO.mkRef {}
    leafPaths := ← IO.mkRef {}
    contractNormalForms := ← IO.mkRef {}
    canonicalForms := ← IO.mkRef {}
    familyConversions := ← IO.mkRef {}
    contractPatternsMet := ← IO.mkRef {}
    proofName := ← IO.mkRef .anonymous
    hiddenTheorems := ← IO.mkRef {}
    hiddenCount := ← IO.mkRef 0
    hiddenStatements := ← IO.mkRef {}
    scopeNames := ← IO.mkRef none
    scopeLocals := ← IO.mkRef #[]
    scopeRow := ← IO.mkRef none
    bindingNames := ← IO.mkRef #[]
    pointOwed := ← IO.mkRef {}
    routedKeys := ← IO.mkRef none
    pathRouted := ← IO.mkRef true
    stepMode := ← IO.mkRef {}
    siteClosureTypes := ← IO.mkRef {}
    fixedPoints := ← IO.mkRef {}
    dsimpResults := ← IO.mkRef {}
    fixedPointCounts := ← IO.mkRef #[0, 0, 0, 0, 0, 0]
    structureCounts := ← IO.mkRef {}
    budgetStop := ← IO.mkRef none
    summaryParameters := ← IO.mkRef {}
    sharedPins := ← IO.mkRef {}
    keptFacts := ← IO.mkRef {}
    keptPremises := ← IO.mkRef {}
  }

/-- The state of the target being proved, in its elaboration branch's
environment, with the branch it was allocated for: a branch forked from
the main environment inherits the main thread's state and allocates its
own. Local to the branch: nothing is reported back, and the state is
released with the branch. Read from the environment, without a shared
reference the proof tasks would contend for at every access. -/
initialize targetStateExt : EnvExtension (Option (Name × TargetState)) ←
  registerEnvExtension (pure none) (asyncMode := .local)

/-- The state of the target being proved: the theorem's whose elaboration
branch this is. Allocated at the first access in the branch, before the
proof's tactics (`verex_proof`), so that a tactic's backtracking never
reaches an environment without it. -/
def targetState : CoreM TargetState := do
  let env ← getEnv
  let key := env.asyncPrefix?.getD .anonymous
  if let some (branch, state) := targetStateExt.getState env then
    if branch == key then return state
  let fresh ← TargetState.new
  modifyEnv (targetStateExt.setState · (some (key, fresh)))
  return fresh

def countStructure (update : StructureCounts → StructureCounts) : CoreM Unit := do
  (← targetState).structureCounts.modify update

/-- The target's state where it exists: this read does not create one. -/
private def existingTargetState? : CoreM (Option TargetState) := do
  let env ← getEnv
  let some (branch, state) := targetStateExt.getState env | return none
  unless branch == env.asyncPrefix?.getD .anonymous do return none
  return some state

/-- The locals no substitution or case split eliminates: the parameters of
the derived summary being stated (`TargetState.summaryParameters`) and the
shared fold pins in force (`TargetState.sharedPins`); none outside a
target's state. -/
def summaryParameters : CoreM FVarIdSet := do
  let some state ← existingTargetState? | return {}
  let pins ← state.sharedPins.get
  return pins.foldl (·.insert ·) (← state.summaryParameters.get)

/-- Whether a local stays (`summaryParameters`). -/
def isSummaryParameter (x : FVarId) : CoreM Bool :=
  return (← summaryParameters).contains x

/-- The shared fold pins in force (`TargetState.sharedPins`). -/
def sharedPins : CoreM FVarIdSet := do
  let some state ← existingTargetState? | return {}
  state.sharedPins.get

/-- The facts the walk keeps on purpose (`TargetState.keptFacts`). -/
def keptFacts : CoreM FVarIdSet := do
  let some state ← existingTargetState? | return {}
  state.keptFacts.get

/-- Record a fact the walk keeps on purpose (`TargetState.keptFacts`). -/
def noteKeptFact (fact : FVarId) : CoreM Unit := do
  (← targetState).keptFacts.modify (·.insert fact)

/-- Record the premise names of a call whose facts the walk keeps, for the
binders they name (`noteKeptPremise`). -/
def noteKeptPremiseNames (names : List Name) : CoreM Unit := do
  (← targetState).keptPremises.modify fun found => names.foldl (·.insert ·) found

/-- A premise just introduced: kept where its binder was a call's frame
premise (`noteKeptPremiseNames`). -/
def noteKeptPremise (goal : MVarId) (introduced : FVarId) : MetaM Unit := do
  let some state ← (existingTargetState? : CoreM _) | return
  let names ← (state.keptPremises.get : CoreM _)
  if names.isEmpty then return
  let name ← goal.withContext introduced.getUserName
  if names.contains name then (state.keptFacts.modify (·.insert introduced) : CoreM _)

/-- `action` with the shared fold pins lifted, as a leaf's pipeline runs:
only the derived summary's parameters stay. -/
def withSharedPinsLifted [Monad m] [MonadLiftT CoreM m] [MonadFinally m] (action : m α) : m α := do
  let some state ← (existingTargetState? : CoreM _) | action
  let pins ← (state.sharedPins.get : CoreM _)
  if pins.isEmpty then return ← action
  (state.sharedPins.set {} : CoreM _)
  try action finally (state.sharedPins.set pins : CoreM _)

/-! ## Scope entries

A step proof reads a function's names (`functionScope`) at a leaf through
scope entries: each local of a source name gets an entry where it is bound,
an inert fact of the context naming its value, so that a substitution or a
case split rewrites the value as it rewrites the leaf. A renaming of the
local renames its entry. The entries exist only in step proofs, whose target
state holds the scope's names (`TargetState.scopeNames`). -/

/-- A name of a step proof's scope and its value at a leaf, at the binding
`index` that orders the bindings of one name: the latest binding is the
one in scope (`doc/designs/proof-language.md`, "Semantics and lowering"). -/
structure ScopeEntry (name : String) (index : Nat) {α : Sort u} (value : α) : Prop where
  intro ::

/-- The value the `&mut` parameter `name` holds at the function's exit: its
prophecy, an inert fact beside its scope entry, which a substitution
rewrites as it rewrites the leaf; an `ensures` binds it as `name'`
(`bindFinals`). -/
structure FinalEntry (name : String) {α : Sort u} (value : α) : Prop where
  intro ::

/-- What a step proof's target owes of its returned value `value`: `holds`, a
binding site of `result` the walk meets where the postcondition meets the
returned value (`bindResult`). Not reducible: the normal form rewrites in it
but does not distribute across it. -/
def ResultAt {α : Sort u} (_value : α) (holds : Prop) : Prop :=
  holds

/-- A postcondition read under `ResultAt` gives the postcondition. -/
theorem wp_resultAt {σ ε Result : Type} {action : Spec σ ε Result}
    {ensures : Result → σ → Prop} {aborts : ε → Prop} {initial : σ}
    (entered : wp action (fun result final => ResultAt result (ensures result final)) aborts initial) :
    wp action ensures aborts initial :=
  wp_mono entered (fun _ _ holds => holds) (fun _ holds => holds)

/-- A scope entry's name, binding index and value. -/
def scopeEntry? (type : Lean.Expr) : Option (String × Nat × Lean.Expr) := do
  guard (type.isAppOfArity ``ScopeEntry 4)
  let .lit (.strVal name) := type.getArg! 0 | none
  -- A case split restates a raw index as a numeral.
  let index ← (type.getArg! 1).rawNatLit? <|> (type.getArg! 1).nat?
  some (name, index, type.getArg! 3)

/-- The names of the step proof's scope, and those of its mutable references:
none outside a step proof. -/
def scopeTable? : CoreM (Option (List String × List String)) := do
  (← targetState).scopeNames.get

/-- The binding index after the context's scope entries. -/
def nextScopeIndex : MetaM Nat := do
  (← getLCtx).foldlM (init := 0) fun found decl => do
    return match scopeEntry? (← instantiateMVars decl.type) with
      | some (_, index, _) => max found (index + 1)
      | none => found

/-- Give a goal the scope entries of `entries`, names and values, at binding
indices after the context's. -/
def addScopeEntries (goal : MVarId) (entries : Array (String × Lean.Expr)) : MetaM MVarId := do
  if entries.isEmpty then return goal
  let mut next ← goal.withContext nextScopeIndex
  let mut goal := goal
  for (name, value) in entries do
    let (entry, proof) ← goal.withContext do
      return (← mkAppOptM ``ScopeEntry #[mkStrLit name, mkRawNatLit next, none, value],
        ← mkAppOptM ``ScopeEntry.intro #[mkStrLit name, mkRawNatLit next, none, value])
    let (fvarId, next') ← (← goal.assert (Name.mkSimple s!"{name}_scope") entry proof).intro1P
    goal ← next'.withContext do
      next'.setFVarKind fvarId .implDetail
      pure next'
    next := next + 1
  return goal

/-- The goal's target `∀ name : domain, body`, where `name` is one of the
scope's, with the binder's scope entry as a premise under it, at binding
`index`: the entry is introduced with the binder, and a substitution of the
binder rewrites it. -/
def scopedBinderGoal (goal : MVarId) (name : Name) (domain body : Lean.Expr) (info : BinderInfo)
    (index : Nat) : MetaM MVarId := goal.withContext do
  let plain := Lean.Expr.forallE name domain body info
  let some (names, references) ← scopeTable? | goal.replaceTargetDefEq plain
  unless names.contains name.toString do return ← goal.replaceTargetDefEq plain
  -- A mutable reference's entry holds its current value, the first of its
  -- pair.
  let (level, read, value) ← if references.contains name.toString then
      match ← whnfD domain with
      | .app (.app (.const ``Prod [u, v]) current) final =>
          pure (← getLevel current, current, mkApp3 (mkConst ``Prod.fst [u, v]) current final (.bvar 0))
      | _ => throwError "internal: a mutable reference's carrier that is no pair"
    else pure (← getLevel domain, domain, .bvar 0)
  let arguments := #[mkStrLit name.toString, mkRawNatLit index, read, value]
  -- A reference's prophecy, the second of its pair, as `name'`.
  let written := name.toString ++ "'"
  let prophecy? ← if references.contains name.toString && names.contains written then
      match ← whnfD domain with
      | .app (.app (.const ``Prod [u, v]) current) final =>
          pure (some (#[mkStrLit written, mkRawNatLit index, final,
            mkApp3 (mkConst ``Prod.snd [u, v]) current final (.bvar 0)], ← getLevel final))
      | _ => pure none
    else pure none
  let body := body.liftLooseBVars 0 (if prophecy?.isSome then 2 else 1)
  let body := match prophecy? with
    | some (finalArguments, finalLevel) =>
        .forallE (Name.mkSimple (written ++ "_scope"))
          (mkAppN (mkConst ``ScopeEntry [finalLevel]) (finalArguments.map (·.liftLooseBVars 0 1)))
          body .default
    | none => body
  let withEntry := Lean.Expr.forallE name domain
    (.forallE (name.appendAfter "_scope") (mkAppN (mkConst ``ScopeEntry [level]) arguments)
      body .default) info
  let next ← mkFreshExprSyntheticOpaqueMVar withEntry (← goal.getTag)
  let proofs := #[mkAppN (mkConst ``ScopeEntry.intro [level]) arguments] ++
    match prophecy? with
    | some (finalArguments, finalLevel) => #[mkAppN (mkConst ``ScopeEntry.intro [finalLevel]) finalArguments]
    | none => #[]
  goal.assign (.lam name domain (mkAppN next (#[.bvar 0] ++ proofs)) info)
  return next.mvarId!

/-- Give the locals `fvarIds` of the scope's names their scope entries: their
binding. -/
def bindScopeEntries (goal : MVarId) (fvarIds : Array FVarId) : MetaM MVarId := do
  if fvarIds.isEmpty then return goal
  goal.withContext do
    let some (names, references) ← scopeTable? | return goal
    -- A value of the name, not a hypothesis that takes it; a reference's
    -- current value.
    let entries ← fvarIds.filterMapM fun fvarId => do
      let decl ← fvarId.getDecl
      let name := decl.userName.toString
      if !names.contains name || (← isProp decl.type) then return none
      let type ← instantiateMVars decl.type
      if references.contains name && type.isAppOfArity ``NTy.carrier 2 &&
          (type.getArg! 1).isAppOf ``NTy.ref then
        return some (name, ← mkAppM ``Prod.fst #[mkFVar fvarId])
      return some (name, mkFVar fvarId)
    -- A reference's prophecy, its value at the exit.
    let mut goal ← addScopeEntries goal entries
    for fvarId in fvarIds do
      let (name, type) ← goal.withContext do
        let decl ← fvarId.getDecl
        return (decl.userName.toString, ← instantiateMVars decl.type)
      unless references.contains name && type.isAppOfArity ``NTy.carrier 2 &&
          (type.getArg! 1).isAppOf ``NTy.ref do continue
      let (entry, proof) ← goal.withContext do
        let final ← mkAppM ``Prod.snd #[mkFVar fvarId]
        return (← mkAppOptM ``FinalEntry #[mkStrLit name, none, final],
          ← mkAppOptM ``FinalEntry.intro #[mkStrLit name, none, final])
      let (entryId, next) ← (← goal.assert (Name.mkSimple s!"{name}_final_scope") entry proof).intro1P
      goal ← next.withContext do
        next.setFVarKind entryId .implDetail
        pure next
    return goal

/-- `verex_scope_names [names] [references] [locals]`: the target is a step
proof over these scope names (`TargetState.scopeNames`); `references` are its
mutable references, `locals` the source names of its locals by declaration
index (`TargetState.scopeLocals`). -/
elab "verex_scope_names " "[" names:str,* "]" " [" references:str,* "]" " [" locals:str,* "]" :
    tactic => do
  (← targetState).scopeNames.set (some (names.getElems.toList.map (·.getString),
    references.getElems.toList.map (·.getString)))
  (← targetState).scopeLocals.set (locals.getElems.map (·.getString))

/-- The declaration index of a variable (`Var.index`), read structurally. -/
private partial def varIndex? (x : Lean.Expr) : Option Nat :=
  if x.isAppOfArity ``VerexIR.Proofs.Denote.Var.here 2 then some 0
  else if x.isAppOfArity ``VerexIR.Proofs.Denote.Var.there 4 then
    (varIndex? x.appArg!.consumeMData).map (· + 1)
  else none

/-- A borrow's prophecy mark (`Prophecy x path value`): the declaration index
of the borrowed local where the borrow takes it whole, a reference's
referent included, and the value. -/
def prophecyMark? (type : Lean.Expr) : Option (Option Nat × Lean.Expr) := do
  guard (type.isAppOf ``VerexIR.Proofs.Denote.Prophecy)
  let arguments := type.getAppArgs
  guard (arguments.size ≥ 3)
  let path := arguments[arguments.size - 2]!.consumeMData
  let nil (path : Lean.Expr) := path.consumeMData.isAppOfArity ``VerexIR.Proofs.Denote.Proj.nil 1
  let whole := nil path ||
    (path.isAppOfArity ``VerexIR.Proofs.Denote.Proj.deref 3 && nil path.appArg!)
  some (if whole then varIndex? arguments[arguments.size - 3]!.consumeMData else none,
    arguments[arguments.size - 1]!)

/-- Consume a borrow's prophecy mark the goal holds as `mark`: in a step
proof, a borrow of a whole local `x` binds the prophecy as the value written
back, `x'` (`ScopeEntry`); any other mark is dropped. -/
def consumeProphecy (goal : MVarId) (mark : FVarId) : MetaM MVarId := do
  let some (index?, value) ← goal.withContext do
      return prophecyMark? (← instantiateMVars (← mark.getType))
    | return goal
  let goal ← goal.clear mark
  let some (names, _) ← scopeTable? | return goal
  let some index := index? | return goal
  let some name := (← (← targetState).scopeLocals.get)[index]? | return goal
  -- The mark is the pair of the component's prophecy and the owner's
  -- (`borrowPlace`, `doc/designs/data-invariants.md`): the
  -- component keeps the name a step proof writes, `x'`, and the owner's
  -- value is named after it.
  let written := name ++ "'"
  let owner := name ++ "_owner"
  let pair? ← goal.withContext do
    let type ← instantiateMVars (← inferType value)
    if (← whnfD type).isAppOfArity ``Prod 2 then
      return some (← mkAppM ``Prod.fst #[value], ← mkAppM ``Prod.snd #[value])
    return none
  let entries := match pair? with
    | some (component, held) =>
        (if names.contains written then #[(written, component)] else #[]) ++
          (if names.contains owner then #[(owner, held)] else #[])
    | none => if names.contains written then #[(written, value)] else #[]
  if entries.isEmpty then return goal
  addScopeEntries goal entries

/-- Record names a binding site bound (`TargetState.bindingNames`). -/
def noteBindingNames (names : Array String) : CoreM Unit := do
  (← targetState).bindingNames.modify fun found =>
    names.foldl (fun found name => if found.contains name then found else found.push name) found

/-- The values a source `let` binds (`LetIn`, `LetRowIn`): each target
local's declaration index where it is a numeral path, and its value. -/
def letBindings (binding : Lean.Expr) : MetaM (Array (Option Nat × Lean.Expr)) := do
  let arguments := binding.getAppArgs
  let size := arguments.size
  if binding.isAppOf ``LetIn then
    return #[(varIndex? arguments[size - 3]!.consumeMData, arguments[size - 2]!)]
  unless binding.isAppOf ``LetRowIn do return #[]
  let mut found := #[]
  let mut targets := arguments[size - 3]!.consumeMData
  let mut values := arguments[size - 2]!
  while targets.isAppOf ``Vars.cons do
    let target := (targets.getArg! (targets.getAppNumArgs - 2)).consumeMData
    if target.isAppOfArity ``Option.some 2 then
      found := found.push (varIndex? target.appArg!.consumeMData, ← mkAppM ``Prod.fst #[values])
    targets := targets.appArg!.consumeMData
    values ← mkAppM ``Prod.snd #[values]
  return found

/-- A goal of what a source `let`'s body owes (`LetIn`, `LetRowIn`): in a
step proof, the locals' scope entries where the scope names them, then what
the body owes, which it is by definition. -/
def bindLet (goal : MVarId) : MetaM (Option MVarId) := goal.withContext do
  -- Only a step proof's walk binds a `let` (`bindLetAt`).
  let some (names, _) ← scopeTable? | return none
  let target := (← instantiateMVars (← goal.getType)).consumeMData
  unless target.isAppOf ``LetIn || target.isAppOf ``LetRowIn do return none
  let goal ← goal.replaceTargetDefEq target.appArg!
  -- Only the target's own frame: an inlined callee's locals are not the
  -- scope's.
  let some row ← (← targetState).scopeRow.get | return some goal
  let arguments := target.getAppArgs
  let frame := arguments[1]!
  unless ← withDefault (isDefEq frame row) do return some goal
  let locals ← (← targetState).scopeLocals.get
  let mut entries := #[]
  for (index?, value) in ← letBindings target do
    let some index := index? | continue
    let some name := locals[index]? | continue
    unless names.contains name do continue
    -- A mutable reference's entry holds its current value, its prophecy
    -- the value written back, as a binder's does (`scopedBinderGoal`).
    -- Names can be reused by inlined helpers for a value of another type.
    -- The declaration's frame slot, rather than a name used by any reference
    -- in the function, determines whether this binding has a prophecy.
    let mut slot := frame
    for _ in [:index] do
      slot := (← whnfR slot).appArg!
    slot ← whnfR slot
    let reference := slot.isAppOfArity ``NRow.cons 2 &&
      (← whnfR (slot.getArg! 0)).isAppOf ``NTy.ref
    if reference then
      entries := entries.push (name, ← mkAppM ``Prod.fst #[value])
      if names.contains (name ++ "'") then
        entries := entries.push (name ++ "'", ← mkAppM ``Prod.snd #[value])
    else entries := entries.push (name, value)
  noteBindingNames (entries.map (·.1))
  return some (← addScopeEntries goal entries)

/-- At an obligation of the function's exit, each `&mut` parameter's value
there, its prophecy (`FinalEntry`), bound as the value written back, `x'`. -/
def bindFinals (goal : MVarId) : MetaM MVarId := goal.withContext do
  let some (names, _) ← scopeTable? | return goal
  let finals ← (← getLCtx).foldlM (init := #[]) fun found decl => do
    let type ← instantiateMVars decl.type
    unless type.isAppOfArity ``FinalEntry 3 do return found
    let .lit (.strVal name) := type.getArg! 0 | return found
    let written := name ++ "'"
    return if names.contains written then found.push (written, type.getArg! 2) else found
  addScopeEntries goal finals

/-- The components of a returned value bound as the step proof's scope names
them: `result` the value, or with several results `result_k` the `k`-th, a
right-nested pair's projections, and `result` the first. -/
private def resultEntries (names : List String) (value : Lean.Expr) :
    MetaM (Array (String × Lean.Expr)) := do
  let count := (List.range names.length).takeWhile (fun k => names.contains s!"result_{k}") |>.length
  if count == 0 then
    return if names.contains "result" then #[("result", value)] else #[]
  let mut entries := #[]
  let mut rest := value
  -- A row of results ends in a unit (`HList`), a pair does not.
  let rowEnd (pair : Lean.Expr) : MetaM Bool := do
    let type ← whnfD (← inferType pair)
    unless type.isAppOfArity ``Prod 2 do return false
    let tail ← whnfD (type.getArg! 1)
    return tail.isConstOf ``PUnit || tail.isConstOf ``Unit || tail.isAppOf ``PUnit
  for k in [0:count] do
    let component ← if k + 1 == count && !(← rowEnd rest) then pure rest
      else whnfR (← mkAppM ``Prod.fst #[rest])
    entries := entries.push (s!"result_{k}", component)
    if k == 0 && names.contains "result" then entries := entries.push ("result", component)
    if k + 1 < count then rest ← whnfR (← mkAppM ``Prod.snd #[rest])
  return entries

/-- The returned values a goal states what it owes of (`ResultAt`), outside
the binders of a postcondition still to be met: none, without a walk, where
it states none, or outside a step proof, whose walk alone marks the returned
value (`enterResult`). Each shared subterm is visited once. -/
def resultSites (target : Lean.Expr) : MetaM (Array Lean.Expr) := do
  if (← scopeTable?).isNone then return #[]
  if (target.find? (·.isAppOf ``ResultAt)).isNone then return #[]
  -- Binders are not entered with fresh locals: a site under one reads its
  -- bound variables as loose and is not met yet.
  let mut found := #[]
  let mut visited : Std.HashSet Lean.Expr := {}
  let mut pending := #[target]
  while let some e := pending.back? do
    pending := pending.pop
    if visited.contains e then continue
    visited := visited.insert e
    if e.isAppOfArity ``ResultAt 3 && !e.hasLooseBVars then
      found := found.push e
      continue
    match e with
    | .app f a => pending := pending.push f |>.push a
    | .lam _ t b _ | .forallE _ t b _ => pending := pending.push t |>.push b
    | .mdata _ b => pending := pending.push b
    | .letE _ t v b _ => pending := pending.push t |>.push v |>.push b
    | .proj _ _ b => pending := pending.push b
    | _ => pure ()
  return found

/-- A goal stating what the target owes of its returned value (`ResultAt`),
met where the postcondition met the value: in a step proof, `result` and the
components the scope names bound to it; the marks dropped, which they are by
definition. -/
def bindResult (goal : MVarId) : MetaM (Option MVarId) := goal.withContext do
  if (← scopeTable?).isNone then return none
  let target ← instantiateMVars (← goal.getType)
  let sites ← resultSites target
  if sites.isEmpty then return none
  let stripped := target.replace fun e =>
    if e.isAppOfArity ``ResultAt 3 && !e.hasLooseBVars then some (e.getArg! 2) else none
  let goal ← goal.replaceTargetDefEq stripped
  let some (names, _) ← scopeTable? | return some goal
  -- One returned value on every path the goal states: its entries.
  let values := sites.map (·.getArg! 1)
  unless values.all (· == values[0]!) do return some goal
  let entries ← resultEntries names values[0]!
  noteBindingNames (entries.map (·.1))
  return some (← addScopeEntries goal entries)

/-- Record the row of a step proof's locals, the frame its body starts in
(`initialEnv`), for `bindLet`. -/
def captureScopeRow (goal : MVarId) : MetaM Unit := goal.withContext do
  let some _ ← scopeTable? | return
  let target ← instantiateMVars (← goal.getType)
  let some env := target.find? (·.isAppOfArity ``initialEnv 6) | return
  let type ← inferType env
  let type ← if type.isAppOf ``HEnv then pure type else whnfR type
  if type.isAppOf ``HEnv then (← targetState).scopeRow.set (some type.appArg!)

/-- `verex_scope_row`: record the row of the step proof's locals
(`captureScopeRow`), before the body's denotation is normalized. -/
elab "verex_scope_row" : tactic => focusGoal do
  (← targetState).scopeRow.set none
  captureScopeRow (← getMainGoal)

/-- A step proof's goal whose postcondition reads the returned value under its
`ResultAt`, where the target's scope names `result`: the goal of a
weakest precondition, after the theorem's introductions. -/
def enterResult (goal : MVarId) : MetaM MVarId := do
  let some (names, _) ← scopeTable? | return goal
  unless names.contains "result" do return goal
  goal.withContext do
    let target := (← instantiateMVars (← goal.getType)).consumeMData
    unless target.isAppOfArity ``VerexIR.Proofs.wp 7 do return goal
    let .lam resultName resultType body info := target.getArg! 4 | return goal
    let .lam finalName finalType inner finalInfo := body | return goal
    let level ← getLevel resultType
    let entered := Lean.Expr.lam resultName resultType
      (.lam finalName finalType (mkApp3 (mkConst ``ResultAt [level]) resultType (.bvar 1) inner)
        finalInfo) info
    let next ← mkFreshExprSyntheticOpaqueMVar
      (mkAppN target.getAppFn (target.getAppArgs.set! 4 entered)) (← goal.getTag)
    goal.assign (← mkAppM ``wp_resultAt #[next])
    return next.mvarId!

/-- The scope name of the memory a step proof reads where it stands
(`doc/designs/proof-language.md`, "Steps": a step reads the state of its
position); its earliest binding, the entry memory, is what `old` reads. -/
def stateScopeName : String := "$state"

/-- In a step proof, bind the memory `state` as the state of the steps after
this point: a scope entry of `stateScopeName`. -/
def bindStateEntry (goal : MVarId) (state : Lean.Expr) : MetaM MVarId := do
  let some _ ← scopeTable? | return goal
  let isMemory ← goal.withContext do
    return (← whnfR (← inferType state)).isAppOf ``VerexIR.Proofs.Denote.Memory
  unless isMemory do return goal
  addScopeEntries goal #[(stateScopeName, state)]

/-- `verex_scope_state x`: the memory `x` is the state of the steps after it. -/
elab "verex_scope_state " x:ident : tactic => focusGoal do
  let goal ← getMainGoal
  let state ← goal.withContext do return mkFVar (← getFVarId x)
  replaceMainGoal [← bindStateEntry goal state]

/-- `verex_scope_bind [x, …]`: the locals' scope entries, their binding. -/
elab "verex_scope_bind " "[" locals:ident,* "]" : tactic => focusGoal do
  let goal ← getMainGoal
  let fvarIds ← goal.withContext do locals.getElems.mapM fun x => getFVarId x
  replaceMainGoal [← bindScopeEntries goal fvarIds]

/-- Rename a local, and its scope entry with it. -/
def renameScoped (goal : MVarId) (fvarId : FVarId) (name : Name) : MetaM MVarId := do
  let goal ← goal.rename fvarId name
  let entry? ← goal.withContext do
    (← getLCtx).findDeclM? fun decl => do
      let some (_, index, value) := scopeEntry? (← instantiateMVars decl.type) | return none
      return if value.consumeMData == mkFVar fvarId then some (decl.fvarId, index) else none
  let some (entry, index) := entry? | return goal
  let (type, proof) ← goal.withContext do
    let value := mkFVar fvarId
    return (← mkAppOptM ``ScopeEntry #[mkStrLit name.toString, mkRawNatLit index, none, value],
      ← mkAppOptM ``ScopeEntry.intro #[mkStrLit name.toString, mkRawNatLit index, none, value])
  let (renamed, goal) ← (← (← goal.clear entry).assert (name.appendAfter "_scope") type proof).intro1P
  goal.withContext do goal.setFVarKind renamed .implDetail
  return goal

/-- Rename the latest scope entry of `name` whose value is not `except`, a
value whose local is gone: an earlier binding a later one displaces. -/
def renameLatestEntry (goal : MVarId) (name : String) (except : Option FVarId) (newName : Name) :
    MetaM MVarId := do
  let latest? ← goal.withContext do
    (← getLCtx).foldlM (init := none) fun (found : Option (FVarId × Nat × Lean.Expr)) decl => do
      let some (entryName, index, value) := scopeEntry? (← instantiateMVars decl.type) | return found
      if entryName != name || except.any (value.consumeMData == mkFVar ·) then return found
      return match found with
        | some (_, best, _) => if index > best then some (decl.fvarId, index, value) else found
        | none => some (decl.fvarId, index, value)
  let some (entry, index, value) := latest? | return goal
  let (type, proof) ← goal.withContext do
    return (← mkAppOptM ``ScopeEntry #[mkStrLit newName.toString, mkRawNatLit index, none, value],
      ← mkAppOptM ``ScopeEntry.intro #[mkStrLit newName.toString, mkRawNatLit index, none, value])
  let (renamed, goal) ← (← (← goal.clear entry).assert (newName.appendAfter "_scope") type proof).intro1P
  goal.withContext do goal.setFVarKind renamed .implDetail
  return goal

/-- Whether a name is taken in the context: by a local, or by a scope entry. -/
def nameTaken (name : Name) : MetaM Bool := do
  if ((← getLCtx).findFromUserName? name).isSome then return true
  (← getLCtx).anyM fun decl => do
    return (scopeEntry? (← instantiateMVars decl.type)).any (·.1 == name.toString)

/-- A proof as an auxiliary theorem of the theorem being proved, as
`mkAuxTheorem` makes one, registered with the target: of a kind of its own,
so that an attempt that is undone never frees its name for another
statement, and declared again where the proof is used after such an
attempt removed it (`declareHiddenTheorems`). -/
def hiddenTheorem (type value : Lean.Expr) : MetaM Lean.Expr := do
  let state ← targetState
  let index ← state.hiddenCount.modifyGet fun index => (index, index + 1)
  let proof ← mkAuxTheorem type value (zetaDelta := true)
    (kind? := some (Name.mkSimple s!"_hidden_{index}"))
  let name := proof.getAppFn.constName!
  let some (.thmInfo info) := (← getEnv).find? name | return proof
  -- A statement an undone attempt hid already: its theorem, again.
  if let some earlier := (← state.hiddenStatements.get)[(info.levelParams, info.type)]? then
    if earlier != name then
      unless (← getEnv).contains earlier do
        if let some declaration := (← state.hiddenTheorems.get)[earlier]? then addDecl declaration
      return mkAppN (mkConst earlier proof.getAppFn.constLevels!) proof.getAppArgs
  unless (← state.hiddenTheorems.get).contains name do
    state.hiddenTheorems.modify (·.insert name (.thmDecl info))
    state.hiddenStatements.modify (·.insert (info.levelParams, info.type) name)
  return proof

/-- Declare the auxiliary theorems a proof uses (`hiddenTheorem`) that a
backtracked attempt removed from the environment. -/
def declareHiddenTheorems (proof : Lean.Expr) : CoreM Unit := do
  let hidden ← (← targetState).hiddenTheorems.get
  if hidden.isEmpty then return
  for name in proof.getUsedConstants do
    unless (← getEnv).contains name do
      if let some declaration := hidden[name]? then addDecl declaration

/-- The facts connected to `seeds`: each sharing a key with a seed or with a
fact reached, to a fixpoint, in context order per round. A fact not reached
says nothing of the seeds' keys: variables, or arithmetic atoms. -/
def connectedBy {κ : Type} [BEq κ] [Hashable κ] (facts : Array (α × Array κ))
    (seeds : Std.HashSet κ) : Array α := Id.run do
  let mut reached := seeds
  let mut relevant := #[]
  let mut remaining := facts
  let mut grown := true
  while grown do
    grown := false
    let mut rest := #[]
    for (fact, keys) in remaining do
      if keys.any reached.contains then
        relevant := relevant.push fact
        reached := keys.foldl (fun found key => found.insert key) reached
        grown := true
      else rest := rest.push (fact, keys)
    remaining := rest
  return relevant

/-- The facts connected to `seeds` through shared variables (`connectedBy`). -/
def connectedFacts (facts : Array (α × FVarIdSet)) (seeds : FVarIdSet) : Array α :=
  connectedBy (facts.map fun (fact, variables) => (fact, variables.toArray))
    (seeds.foldl (fun found x => found.insert x) {})

/-- Close a goal by omega over `facts`, its proof an auxiliary theorem
(`hiddenTheorem`), as the `omega` tactic stores its own: the proof stays
small for the passes that follow. -/
def omegaHidden (facts : List Lean.Expr) (goal : MVarId) : MetaM Unit := goal.withContext do
  let type ← instantiateMVars (← goal.getType)
  let hidden ← mkFreshExprSyntheticOpaqueMVar type
  Lean.Elab.Tactic.Omega.omega facts hidden.mvarId! {}
  let proof ← instantiateMVars hidden
  -- Outside a theorem's proof (`verex_proof`) nothing declares again what
  -- an attempt removed: the proof stays in place.
  if type.hasMVar || proof.hasMVar || (← (← targetState).proofName.get).isAnonymous then
    goal.assign proof
  else goal.assign (← hiddenTheorem type proof)

/-- Record the first runtime value `goal` states, in its goal or a
hypothesis, for the target's report (`reportRuntimeMention`): verification
is native (`doc/designs/native-verification.md`). -/
def recordRuntimeValues (what : MessageData) (goal : MVarId) : MetaM Unit := goal.withContext do
  let state ← targetState
  unless (← state.runtimeMention.get) matches .none do return
  let mut statements := #[(m!"its goal", ← instantiateMVars (← goal.getType))]
  for decl in ← getLCtx do
    unless decl.isImplementationDetail do
      statements := statements.push (m!"its hypothesis `{decl.userName}`", ← instantiateMVars decl.type)
  for (part, statement) in statements do
    if (← state.nativeStatements.get).contains statement then continue
    if let some message ← runtimeValuesMessage? m!"{what} ({part})" statement then
      state.runtimeMention.set (.found message)
      return
    unless statement.hasMVar do state.nativeStatements.modify (·.insert statement)

/-- Report the runtime value the target's goals stated, once. -/
def reportRuntimeMention : CoreM Unit := do
  let state ← targetState
  if let .found message ← state.runtimeMention.get then
    logError message
    state.runtimeMention.set .reported

/-- Order automatic reports by file and source range. Equal ranges retain
their reporting order, including the entry/iteration distinction of a loop.
Unlocated obligations use the verification item's location. -/
private def reportBefore (a b : Message) : Bool :=
  if a.fileName != b.fileName then a.fileName < b.fileName
  else if a.pos.line != b.pos.line then a.pos.line < b.pos.line
  else if a.pos.column != b.pos.column then a.pos.column < b.pos.column
  else if (a.endPos.getD a.pos).line != (b.endPos.getD b.pos).line then
    (a.endPos.getD a.pos).line < (b.endPos.getD b.pos).line
  else (a.endPos.getD a.pos).column ≤ (b.endPos.getD b.pos).column

/-- The proof alternatives follow source-ordered diagnostics, with a name
appearing only at its first report. Authored goal/path order is independent. -/
def failedLeafNames : CoreM (Array String) := do
  let reports := (← (← targetState).failedLeaves.get).mergeSort fun a b => reportBefore a.1 b.1
  return reports.foldl (init := #[]) fun names (_, name) =>
    if names.contains name then names else names.push name

/-- Keep the usual immediate diagnostics and their contexts, marking just
these messages for source ordering when the automatic traversal ends. -/
def recordAutomaticReports (name : String) (action : TacticM α) : TacticM α := do
  let start := (← Core.getMessageLog).unreported.size
  let result ← action
  let log ← Core.getMessageLog
  let messages := log.unreported.toArray
  let reports := (messages.extract start).map fun message =>
    { message with data := .tagged `verex.obligation message.data }
  (← targetState).failedLeaves.modify (· ++ reports.map (·, name))
  Core.setMessageLog { log with unreported := (messages.extract 0 start ++ reports).toPArray' }
  return result

/-- Reorder only automatic obligation reports from this traversal; other
messages retain their slots. Also runs when the target's budget runs out. -/
def orderAutomaticReports (start : Nat) : TacticM Unit := do
  let log ← Core.getMessageLog
  let messages := log.unreported.toArray
  let isReport (message : Message) := message.data.hasTag (· == `verex.obligation)
  let reports := ((messages.extract start).filter isReport).mergeSort reportBefore
  let mut next := 0
  let mut ordered := messages
  for i in [start:messages.size] do
    if isReport messages[i]! then
      ordered := ordered.set! i reports[next]!
      next := next + 1
  Core.setMessageLog { log with unreported := ordered.toPArray' }

/-- The ranges of the `Obligation` markers in an expression. -/
partial def obligationRanges (expression : Lean.Expr) : Array ObligationRange :=
  -- A shared subterm is visited once: a leaf's literal values share
  -- subterms, which a walk of the tree would visit exponentially often.
  let rec walk (e : Lean.Expr) : StateM (Std.HashSet Lean.Expr × Array ObligationRange) Unit := do
    if (← get).1.contains e then return
    modify fun (seen, found) => (seen.insert e, found)
    if let some range := obligationRange? e then
      modify fun (seen, found) => (seen, found.push range)
    match e with
    | .app function argument => walk function; walk argument
    | .lam _ type body _ | .forallE _ type body _ => walk type; walk body
    | .letE _ type value body _ => walk type; walk value; walk body
    | .mdata _ body | .proj _ _ body => walk body
    | _ => pure ()
  ((walk expression).run ({}, #[])).2.2

def Provenance.describe : Provenance → String
  | .precondition callee none => s!"the precondition of `{callee}`"
  | .precondition callee (some call) => s!"the precondition of `{callee}` at call {call}"
  | .continuation callee none => s!"the continuation after `{callee}`"
  | .continuation callee (some call) => s!"the continuation after call {call} of `{callee}`"
  | .loopEntry none => "a loop invariant at entry"
  | .loopEntry (some loop) => s!"an invariant of loop {loop} at entry"
  | .loopIteration none => "a loop invariant at an iteration"
  | .loopIteration (some loop) => s!"an invariant of loop {loop} at an iteration"
  | .assertion => "an in-body assertion"
  | .mutationEnd => "a data invariant where a mutation ends"
  | .construction => "a data invariant where a value is constructed"
  | .copyOut =>
      "a data invariant where a value is copied out of a mutable reference"
  | .handOver => "a data invariant where a mutable reference is passed to a call"
  | .exitMutationEnd =>
      "a data invariant where a mutable parameter's loan ends, at the function's exit"
  | .memoryWritten => "a module invariant where a write of memory ends"
  | .callReturned => "a module invariant where a call returns"
  | .lemmaRequirement => "the requirement of an applied lemma"
  | .unrollBound bound => s!"the bound `pragma unroll = {bound}` of a loop"
  | .storedWrite => "the data invariant of a value written to memory"
  | .ghostWrite => "the domain of a ghost field an update writes"
  | .pointAssert point index => s!"the assertion {index} {point.text}"
  | .returns => "that no abort condition holds where the function returns"

/-- The width a hypothesis is truncated to in the solver's debug trace of an
undecided leaf (`verex_trace_state`). -/
register_option verex.solverDebugWidth : Nat := {
  defValue := 220
  descr := "the characters of a hypothesis the solver's debug trace prints"
}

/-- The heartbeats one complete automatic leaf attempt may spend, in
thousands, including call observations, cheap deciders and preparation.
Past them the leaf is reported for the proof file. Twenty million by default
(`doc/designs/proof-strategy.md`, "One leaf cap"); `0` leaves a leaf the
target's budget. Authored proofs bypass this attempt unless they request it
with `verex_solve`. -/
register_option verex.leafHeartbeats : Nat := {
  defValue := 20000
  descr := "heartbeat budget of one automatic leaf attempt (thousands): a leaf up to its \
    prepared form, or one prepared goal, the prepared form or a case of its variant split; \
    0 for the function's budget; past it the leaf is reported for the proof file"
}

register_option verex.solverDebug : Bool := {
  defValue := false
  descr := "show the normalized verification condition of a denotation before closing"
}

register_option verex.contractLemmas : Bool := {
  defValue := true
  descr := "call through a callee's contract normalized once per contract and argument pattern"
}

/-- The heartbeats, in thousands, the walk deriving a contract-free callee's
summary may spend; past them the callee is inlined. -/
register_option verex.summaryHeartbeats : Nat := {
  defValue := 3000
  descr := "heartbeat budget of the walk deriving a contract-free callee's summary (thousands)"
}

/-- The most conjuncts, one per path, a contract-free callee's summary may
have; a callee with more paths is inlined. -/
register_option verex.summaryLeaves : Nat := {
  defValue := 8
  descr := "the most conjuncts a contract-free callee's derived summary may have"
}

register_option verex.solverProfile : Bool := {
  defValue := false
  descr := "report, per verified function, solver-step heartbeats and folded-continuation arrivals"
}

/-- A program point of the solver's run: the value and state it handed a
continuation at a call boundary. A hypothesis of every leaf below the
point, in program order, which the context's rewrites carry along. A state
label's existential is witnessed at one: the state for its memory, the
values of the locals for its copies of the mutable parameters. -/
def ProgramPoint [Carriers] {State : Type} {ρ : ResultShape} {Γ : NRow}
    {α : Type} (value : Flow ρ Γ α) (state : State) : Prop := True

theorem ProgramPoint.intro [Carriers] {State : Type} {ρ : ResultShape}
    {Γ : NRow} {α : Type} (value : Flow ρ Γ α) (state : State) :
    ProgramPoint value state := trivial

/-- The goal with the program point `(value, state)` as an implementation
detail of its context: only the witness stage reads it, and the deciders of
a leaf skip it. The goal unchanged where `value` is not a flow. -/
def noteProgramPoint (goal : MVarId) (value state : Lean.Expr)
    (deduplicate : Bool := false) : MetaM MVarId :=
  goal.withContext do
    if deduplicate then
      for decl in ← getLCtx do
        let type ← instantiateMVars decl.type
        if type.isAppOfArity ``ProgramPoint 7 &&
            type.getArg! 5 == value && type.getArg! 6 == state then return goal
    let flow ← instantiateMVars (← inferType value)
    unless flow.isAppOfArity ``Flow 4 do return goal
    -- Take the exact carrier family from the flow and state from its value.
    let arguments := #[flow.getArg! 0, ← inferType state] ++
      flow.getAppArgs.extract 1 4 ++ #[value, state]
    let proof := mkAppN (mkConst ``ProgramPoint.intro) arguments
    let fvarId ← mkFreshFVarId
    let lctx := (← getLCtx).mkLocalDecl fvarId `programPoint
      (mkAppN (mkConst ``ProgramPoint) arguments) .default .implDetail
    let next ← mkFreshExprMVarAt lctx (← getLocalInstances) (← goal.getType) .syntheticOpaque
      (← goal.getTag)
    let abstracted ← withLCtx lctx (← getLocalInstances) (mkLambdaFVars #[mkFVar fvarId] next)
    goal.assign (mkApp abstracted proof)
    pure next.mvarId!

private def recordStep (label : String) (failed : Bool) (cost : Nat) : CoreM Unit := do
  (← targetState).stepProfile.modify fun profile =>
    let (runs, failures, succeeding, failing) := profile.getD label (0, 0, 0, 0)
    profile.insert label (if failed then (runs + 1, failures + 1, succeeding, failing + cost)
      else (runs + 1, failures, succeeding + cost, failing))

/-- Under `verex.solverProfile`, what a target spent before its proof's first
tactic. -/
def recordStatementCost : CoreM Unit := do
  if verex.solverProfile.get (← getOptions) then
    recordStep "statement" false ((← IO.getNumHeartbeats) - (← readThe Core.Context).initHeartbeats)

private def beginStep (label : String) : CoreM (Nat × Nat) := do
  let start ← IO.getNumHeartbeats
  let depth ← if verex.solverProfile.get (← getOptions) then do
      let active := (← targetState).activeSteps
      let depth := (← active.get).size
      active.modify (·.push (label, start))
      pure depth
    else pure 0
  return (start, depth)

private def completeStep (label : String) (start depth : Nat) (note : String) : CoreM Unit := do
  let cost := (← IO.getNumHeartbeats) - start
  if verex.solverProfile.get (← getOptions) then
    recordStep label false cost
    let state ← targetState
    state.activeSteps.modify (·.shrink depth)
    state.interruptedSteps.set #[]
  if verex.solverDebug.get (← getOptions) then
    IO.println s!"  {label}: {cost / 1000}k{note}"

private def failStep (label : String) (start depth : Nat) (failure : Exception) : CoreM Nat := do
  let cost := (← IO.getNumHeartbeats) - start
  if verex.solverProfile.get (← getOptions) then
    recordStep label true cost
    let state ← targetState
    let active ← state.activeSteps.get
    -- The innermost step sees the whole chain; those around it, as the
    -- failure passes through them, see part of what is recorded already.
    if failure.isMaxHeartbeat && !active.isPrefixOf (← state.interruptedSteps.get) then
      state.interruptedSteps.set active
    state.activeSteps.modify (·.shrink depth)
  return cost

/-- Run a step and, under `verex.solverDebug`, report its heartbeats; under
`verex.solverProfile`, add them to its label's totals. -/
def timedStep (label : String) (step : TacticM Unit) : TacticM Unit := do
  let (start, depth) ← beginStep label
  tryCatchRuntimeEx
    (do
      step
      completeStep label start depth s!", {(← getGoals).length} goals")
    fun failure => do
      let cost ← failStep label start depth failure
      if verex.solverDebug.get (← getOptions) then
        let reason ← match failure with
          | .internal id _ => pure s!"internal exception {← id.getName}"
          | _ => pure ((← failure.toMessageData.toString).take 200).toString
        IO.println s!"  {label}: {cost / 1000}k, failed\
          {if failure.isRuntime then " (runtime)" else ""}: {reason}"
      -- A step exhausting the recursion depth fails as a step, as one
      -- exhausting its budget does (`withBudget`): the next alternative runs.
      if failure.isMaxRecDepth then throwError "the step `{label}` exceeded the recursion depth"
      throw failure

/-- `timedStep` for a part of a step that returns a value. Its failure passes
through unchanged: the step around it decides what it means, and formatting
it here would charge the step around it for a failure that step recovers from. -/
def timedPart {α : Type} (label : String) (step : MetaM α) : MetaM α := do
  let (start, depth) ← beginStep label
  tryCatchRuntimeEx
    (do
      let result ← step
      completeStep label start depth ""
      return result)
    fun failure => do
      let cost ← failStep label start depth failure
      if verex.solverDebug.get (← getOptions) then
        IO.println s!"  {label}: {cost / 1000}k, failed"
      throw failure

/-- `timedStep` as a tactic. -/
elab "verex_timed " label:str step:tactic : tactic => timedStep label.getString (evalTactic step)

/-- The unsigned certified integer a `.val` projection reads, with its width. -/
private def unsignedValue? (e : Lean.Expr) : Option (Lean.Expr × Lean.Expr) :=
  if e.isAppOfArity ``VerexIR.SpecInt.val 3 && (e.getArg! 1).isConstOf ``Bool.false &&
      (e.getArg! 0).isAppOfArity ``VerexIR.IntWidth.bits 1 && !e.hasLooseBVars then
    some ((e.getArg! 0).getArg! 0, e.getArg! 2)
  else none
/-- The signed certified integer a `.val` projection reads, with its width. -/
private def signedValue? (e : Lean.Expr) : Option (Lean.Expr × Lean.Expr) :=
  if e.isAppOfArity ``VerexIR.SpecInt.val 3 && (e.getArg! 1).isConstOf ``Bool.true &&
      (e.getArg! 0).isAppOfArity ``VerexIR.IntWidth.bits 1 && !e.hasLooseBVars then
    some ((e.getArg! 0).getArg! 0, e.getArg! 2)
  else none

/-- A product of consecutive integers, `n * (n + 1)` or `n * (n - 1)` in either
order of its factors, with the lemma stating its facts and `n`. -/
private def consecutiveProduct? (e : Lean.Expr) : Option (Name × Lean.Expr) := do
  guard (e.isAppOfArity ``HMul.hMul 6 && (e.getArg! 0).isConstOf ``Int)
  let (left, right) := (e.getArg! 4, e.getArg! 5)
  let isOne (e : Lean.Expr) := (e.int? <|> e.nat?.map Int.ofNat) == some 1
  let offset (e : Lean.Expr) : Option (Bool × Lean.Expr) :=
    if e.isAppOfArity ``HAdd.hAdd 6 && isOne (e.getArg! 5) then some (true, e.getArg! 4)
    else if e.isAppOfArity ``HSub.hSub 6 && isOne (e.getArg! 5) then some (false, e.getArg! 4)
    else none
  if let some (up, base) := offset right then
    if base == left then
      return (if up then ``VerexIR.Proofs.Denote.mul_succ_facts
        else ``VerexIR.Proofs.Denote.mul_pred_facts, left)
  let some (up, base) := offset left | none
  guard (base == right)
  return (if up then ``VerexIR.Proofs.Denote.succ_mul_facts
    else ``VerexIR.Proofs.Denote.pred_mul_facts, right)

/-- A proof that an integer term is nonnegative, from its structure: a
certified unsigned value, a nonnegative literal, and sums, products,
shifts, conjunctions, and remainders of nonnegative terms. -/
private partial def nonnegative? (e : Lean.Expr) : MetaM (Option Lean.Expr) := do
  if let some (_, value) := unsignedValue? e then
    return some (← mkAppM ``And.left #[← mkAppM ``VerexIR.SpecInt.unsigned_bounds #[value]])
  if let some literal := e.int? <|> (e.nat?.map Int.ofNat) then
    if 0 ≤ literal then
      return some (← mkDecideProof (← mkAppM ``LE.le #[mkIntLit 0, e]))
    return none
  let both (lemma : Name) : MetaM (Option Lean.Expr) := do
    match ← nonnegative? (e.getArg! 4), ← nonnegative? (e.getArg! 5) with
    | some left, some right => some <$> mkAppM lemma #[left, right]
    | _, _ => pure none
  if e.isAppOfArity ``HAdd.hAdd 6 then return ← both ``Int.add_nonneg
  if let some (lemma, n) := consecutiveProduct? e then
    return some (← mkAppM ``And.left #[mkApp (mkConst lemma) n])
  if e.isAppOfArity ``HMul.hMul 6 then return ← both ``Int.mul_nonneg
  if e.isAppOfArity ``Int.shiftLeft 2 then
    let some value ← nonnegative? (e.getArg! 0) | return none
    return some (← mkAppM ``VerexIR.Proofs.Denote.shiftLeft_nonneg #[e.getArg! 0, e.getArg! 1, value])
  if e.isAppOfArity ``VerexIR.Proofs.IntegerArithmetic.bitwiseAnd 2 then
    let some left ← nonnegative? (e.getArg! 0) | return none
    let some right ← nonnegative? (e.getArg! 1) | return none
    return some (← mkAppM ``VerexIR.Proofs.IntegerArithmetic.bitwiseAnd_nonneg #[left, right])
  if e.isAppOfArity ``Int.tmod 2 then
    let some value ← nonnegative? (e.getArg! 0) | return none
    return some (← mkAppM ``Int.tmod_nonneg #[e.getArg! 1, value])
  if e.isAppOfArity ``HMod.hMod 6 then
    let modulus := e.getArg! 5
    let some literal := modulus.int? <|> (modulus.nat?.map Int.ofNat) | return none
    unless literal != 0 do return none
    let nonzero ← mkDecideProof (← mkAppM ``Ne #[modulus, mkIntLit 0])
    return some (← mkAppM ``Int.emod_nonneg #[e.getArg! 4, nonzero])
  return none

/-- A Euclidean quotient or remainder of nonnegative operands is between
zero and the dividend. -/
theorem ediv_bounds_of_nonneg {left right : Int} (leftNonnegative : 0 ≤ left)
    (rightNonnegative : 0 ≤ right) : 0 ≤ left / right ∧ left / right ≤ left :=
  ⟨Int.ediv_nonneg leftNonnegative rightNonnegative, Int.ediv_le_self right leftNonnegative⟩

theorem emod_bounds_of_nonneg {left right : Int} (leftNonnegative : 0 ≤ left)
    (rightNonnegative : 0 ≤ right) : 0 ≤ left % right ∧ left % right ≤ left := by
  have quotient := Int.ediv_nonneg leftNonnegative rightNonnegative
  have product := Int.mul_nonneg rightNonnegative quotient
  have definition := Int.emod_def left right
  rcases Int.lt_or_eq_of_le rightNonnegative with positive | zero
  · exact ⟨Int.emod_nonneg _ (by omega), by omega⟩
  · subst zero; simp only [Int.emod_zero]; omega

/-- Both facts, either, or none. -/
private def conjoin? : Option Lean.Expr → Option Lean.Expr → MetaM (Option Lean.Expr)
  | some left, some right => some <$> mkAppM ``And.intro #[left, right]
  | some fact, none | none, some fact => pure (some fact)
  | none, none => pure none

/-- The range of a truncating quotient or remainder: from its operands'
signs, their signed widths, or its operands' magnitudes. -/
private def tdivBounds? (sub : Lean.Expr) : MetaM (Option Lean.Expr) := do
  match ← nonnegative? (sub.getArg! 0), ← nonnegative? (sub.getArg! 1) with
  | some leftNonnegative, some rightNonnegative =>
      let lemma := if sub.isAppOfArity ``Int.tdiv 2
        then ``VerexIR.Proofs.Denote.tdiv_bounds_of_nonneg
        else ``VerexIR.Proofs.Denote.tmod_bounds_of_nonneg
      return some (← mkAppM lemma #[leftNonnegative, rightNonnegative])
  | _, _ =>
      match signedValue? (sub.getArg! 0), signedValue? (sub.getArg! 1) with
      | some (width, left), some (_, right) =>
          let lemma := if sub.isAppOfArity ``Int.tdiv 2
            then ``VerexIR.Proofs.Denote.tdiv_signed_range
            else ``VerexIR.Proofs.Denote.tmod_signed_range
          return some (mkApp3 (mkConst lemma) width left right)
      | _, _ =>
          let lemma := if sub.isAppOfArity ``Int.tdiv 2
            then ``VerexIR.Proofs.Denote.tdiv_facts
            else ``VerexIR.Proofs.Denote.tmod_facts
          return some (mkApp2 (mkConst lemma) (sub.getArg! 0) (sub.getArg! 1))

/-- Solve a goal by omega over its hypotheses, as the tactic does: by
contradiction, over the local context. -/
def decideByOmega (goal : MVarId) : MetaM Unit := do
  let some goal ← goal.falseOrByContra | return
  goal.withContext do
    omegaHidden (← getLocalHyps).toList goal

/-- A proof of a statement in a goal's context by omega, if omega finds one. -/
private def omegaProofCore? (goal : MVarId) (statement : Lean.Expr) : MetaM (Option Lean.Expr) :=
  goal.withContext do
    let proof ← mkFreshExprMVar statement
    try decideByOmega proof.mvarId!; some <$> instantiateMVars proof
    catch _ => pure none

/-- A proof of a statement by omega from no hypothesis: a fact of the
integers alone. -/
def isolatedOmegaProof? (statement : Lean.Expr) : MetaM (Option Lean.Expr) := do
  let proof ← mkFreshExprMVar statement
  let known := (← getLCtx).numIndices
  try
    let some goal ← proof.mvarId!.falseOrByContra | return some (← instantiateMVars proof)
    goal.withContext do
      let lctx ← getLCtx
      let negated := if lctx.numIndices > known then lctx.lastDecl.toList.map (·.toExpr) else []
      omegaHidden negated goal
    some <$> instantiateMVars proof
  catch _ => pure none

/-- A fact with premises, its premises decided by the integers alone: none
where a premise fails, so that the fact says nothing, and the premises that
hold discharged. -/
partial def decidePremises (fact : Lean.Expr) : MetaM (Option Lean.Expr) := do
  let .forallE name premise body info ← instantiateMVars (← inferType fact) | return some fact
  if body.hasLooseBVars || !(← isProp premise) then return some fact
  if (← isolatedOmegaProof? (mkNot premise)).isSome then return none
  if let some holds ← isolatedOmegaProof? premise then return ← decidePremises (mkApp fact holds)
  withLocalDecl name info premise fun assumed => do
    let some rest ← decidePremises (mkApp fact assumed) | return none
    return some (← mkLambdaFVars #[assumed] rest)

/-- Reuse arithmetic side facts only under their checked dependencies.
Added hypotheses invalidate an earlier no-progress result and permit a new
attempt within the same automatic allowance. -/
def omegaProof? (goal : MVarId) (statement : Lean.Expr) : MetaM (Option Lean.Expr) := do
  let some cache ← (← targetState).preparationProofs.get | return ← omegaProofCore? goal statement
  let owners ← (← targetState).preparationOwners.get
  if owners.isEmpty then return ← omegaProofCore? goal statement
  goal.withContext do
    PreparationCache.run cache statement (stage := `omega) (owners := owners) do
      let saved ← Meta.saveState
      let proof? ← omegaProofCore? goal statement
      if proof?.isNone then saved.restore
      return proof?

/-- What a leaf reads of the division sites it holds, collected once: the
remainders it states, and the operands of its products. -/
private structure DivisionReads where
  remainders : Array Lean.Expr
  factors : Array Lean.Expr
  /-- The equalities between integers the leaf states, as `(lhs, rhs)`. -/
  equalities : Array (Lean.Expr × Lean.Expr)

/-- Whether a product the leaf reads has `term` as a factor, up to the leaf's
equalities: a quotient an opaque callee returns is multiplied through the
variable its contract equates with it. -/
private def DivisionReads.readsFactor (reads : DivisionReads) (term : Lean.Expr) : Bool :=
  Id.run do
    let mut related := #[term]
    let mut grown := true
    while grown do
      grown := false
      for (lhs, rhs) in reads.equalities do
        if related.contains lhs && !related.contains rhs then
          related := related.push rhs; grown := true
        else if related.contains rhs && !related.contains lhs then
          related := related.push lhs; grown := true
    return reads.factors.any related.contains

/-- The bound of one bit operation, when its operands are certified. A
division site's facts are chosen by what the leaf reads (`reads`), and
its remainder's bound by the divisor's positivity, which omega decides over
the leaf's hypotheses (`goal`). -/
private def operationFact? (goal : MVarId) (reads : DivisionReads) (sub : Lean.Expr) :
    MetaM (Option Lean.Expr) := do
  if sub.isAppOfArity ``VerexIR.Proofs.IntegerArithmetic.bitwiseAnd 2 then
    match unsignedValue? (sub.getArg! 0), unsignedValue? (sub.getArg! 1) with
    | some (_, left), some (_, right) =>
        return some (← mkAppM ``VerexIR.Proofs.Denote.BitOp.eval_bounds
          #[mkConst ``VerexIR.Proofs.Denote.BitOp.and, left, right])
    | _, _ =>
        match ← nonnegative? (sub.getArg! 0), ← nonnegative? (sub.getArg! 1) with
        | some left, some right =>
            return some (← mkAppM ``VerexIR.Proofs.IntegerArithmetic.bitwiseAnd_bounds
              #[left, right])
        | _, _ => return none
  -- A disjunction or exclusive disjunction of certified unsigned values, as
  -- the runtime computes it, lies within their width.
  if sub.isAppOfArity ``Int.ofNat 1 then
    let bits := sub.appArg!
    let op? := if bits.isAppOfArity ``HOr.hOr 6 then some ``VerexIR.Proofs.Denote.BitOp.or
      else if bits.isAppOfArity ``HXor.hXor 6 then some ``VerexIR.Proofs.Denote.BitOp.xor
      else none
    let operand? (e : Lean.Expr) : Option Lean.Expr :=
      if e.isAppOfArity ``Int.toNat 1 then (unsignedValue? e.appArg!).map (·.2) else none
    -- Exclusive disjunction with the ones of a width at least the value's:
    -- the value's complement there.
    if bits.isAppOfArity ``HXor.hXor 6 then
      for (valueSide, onesSide, swapped) in
          [(bits.getArg! 4, bits.getArg! 5, false), (bits.getArg! 5, bits.getArg! 4, true)] do
        let some ones := onesSide.nat? <|> onesSide.rawNatLit? | continue
        let k := (ones + 1).log2
        unless 2 ^ k == ones + 1 && valueSide.isAppOfArity ``Int.toNat 1 do continue
        let some (width, value) := unsignedValue? valueSide.appArg! | continue
        let some w := width.nat? <|> width.rawNatLit? | continue
        unless w ≤ k do continue
        let wide ← mkDecideProof (← mkAppM ``LE.le #[width, mkNatLit k])
        let complement ← mkAppM ``VerexIR.Proofs.Denote.SpecInt.toNat_xor_ones #[value, wide]
        let proof ← if swapped then
            mkEqTrans (← mkAppM ``Nat.xor_comm #[onesSide, valueSide]) complement
          else pure complement
        return some (← mkExpectedTypeHint proof
          (← mkEq bits (← mkAppM ``HSub.hSub #[onesSide, valueSide])))
    let some op := op? | return none
    let some left := operand? (bits.getArg! 4) | return none
    let some right := operand? (bits.getArg! 5) | return none
    let bounds ← mkAppM ``VerexIR.Proofs.Denote.BitOp.eval_bounds #[mkConst op, left, right]
    -- The operation's `eval` is the site, by definition.
    let evaluated := mkApp3 (mkConst ``VerexIR.Proofs.Denote.BitOp.eval) (mkConst op)
      (bits.getArg! 4).appArg! (bits.getArg! 5).appArg!
    let stated := (← instantiateMVars (← inferType bounds)).replace fun e =>
      if e == evaluated then some sub else none
    return some (← mkExpectedTypeHint bounds stated)
  -- A shift by a literal distance is a multiplication or division by a
  -- literal, which `omega` reads.
  let literalDistance := (sub.getArg! 1).nat?.isSome || (sub.getArg! 1).rawNatLit?.isSome
  if sub.isAppOfArity ``Int.shiftRight 2 then
    let division ← if literalDistance then
        some <$> mkAppM ``VerexIR.Proofs.Denote.shiftRight_eq_div #[sub.getArg! 0, sub.getArg! 1]
      else pure none
    let bounds ← match unsignedValue? (sub.getArg! 0) with
      | some (_, value) =>
          some <$> mkAppM ``VerexIR.Proofs.Denote.shiftRight_bounds #[value, sub.getArg! 1]
      | none => pure none
    return ← conjoin? bounds division
  if sub.isAppOfArity ``HMul.hMul 6 && (sub.getArg! 0).isConstOf ``Int then
    -- A product of consecutive integers is even and nonnegative.
    let consecutive := (consecutiveProduct? sub).map fun (lemma, n) => mkApp (mkConst lemma) n
    let unsigned ← match unsignedValue? (sub.getArg! 4), unsignedValue? (sub.getArg! 5) with
      | some (_, left), some (_, right) =>
          some <$> mkAppM ``VerexIR.Proofs.Denote.mul_unsigned_bounds #[left, right]
      | _, _ => pure none
    return ← conjoin? unsigned consecutive
  if sub.isAppOfArity ``Int.tdiv 2 || sub.isAppOfArity ``Int.tmod 2 then
    let bounds ← tdivBounds? sub
    let (dividend, divisor) := (sub.getArg! 0, sub.getArg! 1)
    if (divisor.int? <|> (divisor.nat?.map Int.ofNat)).isSome then return bounds
    -- At a divisor that is not a literal: the division algorithm of
    -- nonnegative operands, where the leaf reads the remainder or a product
    -- with the divisor or the quotient, up to its equalities (the identity
    -- constrains nothing else), with the remainder below a divisor the hypotheses make
    -- positive, and conditionally on positivity only where the remainder is
    -- read; and the quotient of a dividend the divisor divides.
    let quotient := mkApp2 (mkConst ``Int.tdiv) dividend divisor
    let readsRemainder := reads.remainders.contains (mkApp2 (mkConst ``Int.tmod) dividend divisor)
    let needsIdentity := readsRemainder ||
      reads.readsFactor divisor || reads.readsFactor quotient
    let algorithm ← if !needsIdentity then pure none else
      match ← nonnegative? dividend, ← nonnegative? divisor with
      | some dividendNonnegative, some divisorNonnegative =>
          let identity ← mkAppM ``Int.tdiv_mul_add_tmod #[dividend, divisor]
          let nonnegative ← mkAppM ``Int.tmod_nonneg #[divisor, dividendNonnegative]
          let positive ← mkAppM ``LT.lt #[mkIntLit 0, divisor]
          let positivity ← omegaProof? goal
            (← mkArrow (← inferType divisorNonnegative) positive)
          let below ← match positivity with
            | some proof =>
                some <$> mkAppM ``Int.tmod_lt_of_pos #[dividend, proof.app divisorNonnegative]
            | none =>
                if !readsRemainder then pure none else
                  some <$> withLocalDeclD `positive positive fun h => do
                    mkLambdaFVars #[h] (← mkAppM ``Int.tmod_lt_of_pos #[dividend, h])
          conjoin? (some identity) (← conjoin? (some nonnegative) below)
      | _, _ => pure none
    let cancellation ← if !sub.isAppOfArity ``Int.tdiv 2 then pure none
      else if dividend == divisor then some <$> mkAppOptM ``Int.tdiv_self #[dividend]
      else if dividend.isAppOfArity ``HMul.hMul 6 && dividend.getArg! 5 == divisor then
        some <$> mkAppOptM ``Int.mul_tdiv_cancel #[dividend.getArg! 4, divisor]
      else if dividend.isAppOfArity ``HMul.hMul 6 && dividend.getArg! 4 == divisor then
        some <$> mkAppOptM ``Int.mul_tdiv_cancel_left #[divisor, dividend.getArg! 5]
      else pure none
    return ← conjoin? (← conjoin? bounds algorithm) cancellation
  if (sub.isAppOfArity ``HDiv.hDiv 6 || sub.isAppOfArity ``HMod.hMod 6) &&
      (sub.getArg! 0).isConstOf ``Int then
    let (dividend, divisor) := (sub.getArg! 4, sub.getArg! 5)
    -- omega reads a quotient or remainder by a literal itself; one under a
    -- binder is no term of the leaf's context.
    if (divisor.int? <|> (divisor.nat?.map Int.ofNat)).isSome || sub.hasLooseBVars then return none
    let isQuotient := sub.isAppOfArity ``HDiv.hDiv 6
    let bounds ← match ← nonnegative? dividend, ← nonnegative? divisor with
      | some leftNonnegative, some rightNonnegative =>
          let lemma := if isQuotient then ``VerexIR.Proofs.Denote.ediv_bounds_of_nonneg
            else ``VerexIR.Proofs.Denote.emod_bounds_of_nonneg
          some <$> mkAppM lemma #[leftNonnegative, rightNonnegative]
      | _, _ => pure none
    let cancellation ← if !isQuotient then pure none
      else if dividend == divisor then some <$> mkAppOptM ``Int.ediv_self #[dividend]
      else if dividend.isAppOfArity ``HMul.hMul 6 && dividend.getArg! 5 == divisor then
        some <$> mkAppOptM ``Int.mul_ediv_cancel #[dividend.getArg! 4, divisor]
      else if dividend.isAppOfArity ``HMul.hMul 6 && dividend.getArg! 4 == divisor then
        some <$> mkAppOptM ``Int.mul_ediv_cancel_left #[divisor, dividend.getArg! 5]
      else pure none
    -- The division algorithm of nonnegative operands where the leaf reads
    -- the remainder or a product with the divisor or the quotient, as at a
    -- truncating division (`Int.tdiv`, above): the same facts in the one
    -- spelling of a quotient.
    let quotient ← mkAppM ``HDiv.hDiv #[dividend, divisor]
    let remainder ← mkAppM ``HMod.hMod #[dividend, divisor]
    let readsRemainder := reads.remainders.contains remainder
    let needsIdentity := readsRemainder ||
      reads.readsFactor divisor || reads.readsFactor quotient
    let algorithm ← if !needsIdentity then pure none else
      match ← nonnegative? dividend, ← nonnegative? divisor with
      | some dividendNonnegative, some divisorNonnegative =>
          let identity ← mkAppM ``Int.ediv_mul_add_emod #[dividend, divisor]
          let nonnegative ← mkAppM ``And.left
            #[← mkAppM ``VerexIR.Proofs.Denote.emod_bounds_of_nonneg
              #[dividendNonnegative, divisorNonnegative]]
          let positive ← mkAppM ``LT.lt #[mkIntLit 0, divisor]
          let positivity ← omegaProof? goal
            (← mkArrow (← inferType divisorNonnegative) positive)
          let below ← match positivity with
            | some proof =>
                some <$> mkAppM ``Int.emod_lt_of_pos #[dividend, proof.app divisorNonnegative]
            | none =>
                if !readsRemainder then pure none else
                  some <$> withLocalDeclD `positive positive fun h => do
                    mkLambdaFVars #[h] (← mkAppM ``Int.emod_lt_of_pos #[dividend, h])
          conjoin? (some identity) (← conjoin? (some nonnegative) below)
      | _, _ => pure none
    return ← conjoin? (← conjoin? bounds algorithm) cancellation
  if sub.isAppOfArity ``Int.shiftLeft 2 then
    let product ← if literalDistance then
        some <$> mkAppM ``VerexIR.Proofs.Denote.shiftLeft_eq_mul #[sub.getArg! 0, sub.getArg! 1]
      else pure none
    let nonnegative ← nonnegative? sub
    return ← conjoin? nonnegative product
  return none

/-- A conjunction of a nonnegative value with a literal mask of low ones, on
either side, as the remainder modulo the mask's successor (`n & 1 = n % 2`),
which omega decides. -/
private def maskFact? (sub : Lean.Expr) : MetaM (Option Lean.Expr) := do
  unless sub.isAppOfArity ``VerexIR.Proofs.IntegerArithmetic.bitwiseAnd 2 do return none
  let literal? (e : Lean.Expr) : Option Nat := do
    let mask ← e.int?
    guard (0 ≤ mask)
    let successor := mask.toNat + 1
    guard (successor &&& mask.toNat = 0)
    pure successor.log2
  let left := sub.getArg! 0
  let right := sub.getArg! 1
  let (value, mask, bits, swapped) ← match literal? right, literal? left with
    | some bits, _ => pure (left, right, bits, false)
    | none, some bits => pure (right, left, bits, true)
    | none, none => return none
  let some nonnegative ← nonnegative? value | return none
  let power ← mkAppM ``HSub.hSub
    #[← mkAppM ``HPow.hPow #[mkIntLit 2, mkNatLit bits], mkIntLit 1]
  let isMask ← mkDecideProof (← mkEq mask power)
  let fact ← mkAppM ``VerexIR.Proofs.IntegerArithmetic.bitwiseAnd_mask
    #[mkNatLit bits, nonnegative, isMask]
  if swapped then
    mkEqTrans (← mkAppM ``VerexIR.Proofs.IntegerArithmetic.bitwiseAnd_comm #[mask, value]) fact
  else pure fact

/-- The distinct subterms of some terms a predicate accepts, in the order
first met, each shared subterm visited once: a term built by repeated updates
shares its parts, and is exponentially larger read as a tree. -/
partial def sitesWhere (accept : Lean.Expr → Bool) (terms : Array Lean.Expr) :
    Array Lean.Expr :=
  (terms.forM go |>.run ({}, #[])).2.2
where
  go (e : Lean.Expr) : StateM (Std.HashSet Lean.Expr × Array Lean.Expr) Unit := do
    if (← get).1.contains e then return
    modify fun (visited, found) => (visited.insert e, if accept e then found.push e else found)
    match e with
    | .app f a => go f; go a
    | .lam _ t b _ | .forallE _ t b _ => go t; go b
    | .letE _ t v b _ => go t; go v; go b
    | .mdata _ b | .proj _ _ b => go b
    | _ => pure ()

/-- The operations propositions apply whose results have bounds, outside
binders (no loose bound variables). -/
private def operationSites (terms : Array Lean.Expr) : Array Lean.Expr :=
  sitesWhere (fun e => !e.hasLooseBVars &&
    (e.isAppOfArity ``VerexIR.Proofs.IntegerArithmetic.bitwiseAnd 2 ||
    (e.isAppOfArity ``Int.ofNat 1 &&
      (e.appArg!.isAppOfArity ``HOr.hOr 6 || e.appArg!.isAppOfArity ``HXor.hXor 6)) ||
    e.isAppOfArity ``Int.shiftRight 2 || e.isAppOfArity ``Int.shiftLeft 2 ||
    e.isAppOfArity ``Int.tdiv 2 || e.isAppOfArity ``Int.tmod 2 ||
    e.isAppOfArity ``HDiv.hDiv 6 || e.isAppOfArity ``HMod.hMod 6 ||
    e.isAppOfArity ``HMul.hMul 6)) terms

/-- The closed applications of structure projections propositions mention. -/
private def projectionSites (env : Environment) (terms : Array Lean.Expr) : Array Lean.Expr :=
  sitesWhere (fun e => match e.getAppFn with
    | .const name _ => e.isApp && !e.hasLooseBVars && (env.getProjectionFnInfo? name).isSome
    | _ => false) terms

/-- The distinct terms a linear reading of some terms reaches: through sums,
differences, products, and negations, which `omega` reads, and not into any
other operation, whose result it reads as an atom. -/
private partial def linearOperands (terms : Array Lean.Expr) : Array Lean.Expr :=
  (terms.forM go |>.run ({}, #[])).2.2
where
  go (e : Lean.Expr) : StateM (Std.HashSet Lean.Expr × Array Lean.Expr) Unit := do
    if (← get).1.contains e then return
    modify fun (visited, found) => (visited.insert e, found)
    if e.isAppOfArity ``HAdd.hAdd 6 || e.isAppOfArity ``HSub.hSub 6 ||
        e.isAppOfArity ``HMul.hMul 6 then
      go (e.getArg! 4); go (e.getArg! 5)
    else if e.isAppOfArity ``Neg.neg 3 then go (e.getArg! 2)
    else modify fun (visited, found) => (visited, found.push e)

/-- The divisions a leaf's goal and hypotheses read. -/
private def divisionReads (terms : Array Lean.Expr) : DivisionReads :=
  let products := sitesWhere (·.isAppOfArity ``HMul.hMul 6) terms
  let equalities := sitesWhere
    (fun e => e.isAppOfArity ``Eq 3 && (e.getArg! 0).isConstOf ``Int) terms
  { remainders := sitesWhere (fun e => e.isAppOfArity ``Int.tmod 2 || e.isAppOfArity ``HMod.hMod 6) terms
    factors := products.flatMap fun product => #[product.getArg! 4, product.getArg! 5]
    equalities := equalities.map fun equality => (equality.getArg! 1, equality.getArg! 2) }

/-- A value's type in weak head normal form for its bound: the carrier of a
callee's result shape, a constant for each callee, unfolds to the type it
denotes, an instantiated type parameter's included. -/
def boundType (type : Lean.Expr) : MetaM Lean.Expr := do
  let type ← instantiateMVars type
  if type.isAppOf ``VerexIR.Proofs.Denote.ResultShape.carrier then whnfD type else whnfR type

/-- The bound a value carries by its type `type` (in weak head normal form):
a fixed-width integer's range, a character's, a vector's length bound. -/
def typedBound? (value type : Lean.Expr) : MetaM (Option Lean.Expr) := do
  if type.isConstOf ``VerexIR.SpecChar then
    return some (← mkAppM ``VerexIR.SpecChar.bounds #[value])
  if type.isAppOfArity ``VerexIR.SpecVector 1 then
    return some (← mkAppM ``VerexIR.SpecVector.bounded #[value])
  -- A map's entries are as many as a Move vector holds.
  if type.isAppOfArity ``VerexIR.SpecEntries 1 then
    return some (← mkAppM ``VerexIR.SpecEntries.bounded #[value])
  if type.isAppOfArity ``VerexIR.SpecInt 2 then
    let width := type.getArg! 0
    if width.isAppOfArity ``VerexIR.IntWidth.bits 1 then
      let lemma := if (type.getArg! 1).isConstOf ``Bool.true
        then ``VerexIR.SpecInt.signed_bounds else ``VerexIR.SpecInt.unsigned_bounds
      return some (mkApp2 (mkConst lemma) (width.getArg! 0) value)
  return none

/-- The bounds of the values a goal reads from typed maps whose keys or values
fit an integer type (`m_keys`, `m_values`; `doc/designs/proof-language.md`,
"Normal-form fixes"): a key at a position, a key the context states present,
a key's successor or predecessor, a value at a key. Each
as the pair of bounds a parameter's certificate gives, under which the
normal form reads a cut value's round trip as the value (`val_ofInt_*`). -/
def containerBounds (expressions : Array Lean.Expr) : MetaM (Array Lean.Expr) := do
  -- The maps whose keys or values fit, with their facts.
  let mut keyed : Array (Lean.Expr × Lean.Expr × Nat × Bool) := #[]
  let mut valued : Array (Lean.Expr × Lean.Expr × Nat × Bool) := #[]
  let mut present : Array (Lean.Expr × Lean.Expr) := #[]
  -- A successor's or predecessor's statement, with its lemma.
  let mut neighbours : Array (Lean.Expr × Lean.Expr × Name) := #[]
  let fitting (property : Lean.Expr) : Option (Nat × Bool) := do
    guard (property.isAppOfArity ``VerexIR.IntegerValueFits 2)
    let width := property.getArg! 0
    guard (width.isAppOfArity ``VerexIR.IntWidth.bits 1)
    let bits ← (width.getArg! 0).nat?
    guard (bits != 0)
    pure (bits, (property.getArg! 1).isConstOf ``Bool.true)
  for decl in ← getLCtx do
    if decl.isImplementationDetail then continue
    let type ← instantiateMVars decl.type
    if type.isAppOfArity ``VerexIR.SpecMap.KeysSatisfy 4 then
      if let some (bits, signed) := fitting (type.getArg! 2) then
        keyed := keyed.push (type.getArg! 3, decl.toExpr, bits, signed)
    else if type.isAppOfArity ``VerexIR.SpecMap.ValuesSatisfy 4 then
      if let some (bits, signed) := fitting (type.getArg! 2) then
        valued := valued.push (type.getArg! 3, decl.toExpr, bits, signed)
    else if let some (_, lhs, rhs) := type.eq? then
      if lhs.isAppOf ``VerexIR.SpecMap.hasKey && rhs.isConstOf ``Bool.true then
        present := present.push (lhs.appFn!.appArg!, decl.toExpr)
      else if rhs.isAppOfArity ``Option.some 2 then
        if lhs.isAppOf ``VerexIR.SpecMap.nextKey? then
          neighbours := neighbours.push (lhs.appFn!.appArg!, decl.toExpr,
            ``VerexIR.SpecMap.nextKey_satisfies)
        else if lhs.isAppOf ``VerexIR.SpecMap.prevKey? then
          neighbours := neighbours.push (lhs.appFn!.appArg!, decl.toExpr,
            ``VerexIR.SpecMap.prevKey_satisfies)
  if keyed.isEmpty && valued.isEmpty then return #[]
  -- A certificate's bounds.
  let pair (fits : Lean.Expr) (signed : Bool) : MetaM Lean.Expr :=
    mkAppM (if signed then ``VerexIR.Proofs.Denote.bounds_of_fits_signed
      else ``VerexIR.Proofs.Denote.bounds_of_fits_unsigned) #[fits]
  -- The default a read outside the map takes fits: an integer's is zero.
  let fallback (bits : Nat) (signed : Bool) : MetaM Lean.Expr := do
    mkAppM ``VerexIR.Proofs.Denote.zero_fits
      #[mkNatLit bits, toExpr signed, ← mkDecideProof (← mkAppM ``Ne #[mkNatLit bits, mkNatLit 0])]
  let attempt (fact : MetaM Lean.Expr) : MetaM (Option Lean.Expr) :=
    try some <$> fact catch _ => pure none
  let mut facts := #[]
  for (map, keys, bits, signed) in keyed do
    for read in sitesWhere (fun e => e.isAppOf ``VerexIR.SpecMap.keyAt && e.getAppNumArgs ≥ 2 &&
        e.appFn!.appArg! == map && !e.hasLooseBVars) expressions do
      let fact ← attempt do
        let fits ← mkAppM ``VerexIR.SpecMap.keyAt_satisfies_all
          #[keys, ← fallback bits signed, read.appArg!]
        pair fits signed
      if let some fact := fact then facts := facts.push fact
    for (presentMap, stated) in present do
      unless presentMap == map do continue
      let fact ← attempt do
        let fits ← mkAppM ``VerexIR.SpecMap.hasKey_satisfies #[keys, stated]
        pair fits signed
      if let some fact := fact then facts := facts.push fact
    for (neighbourMap, found, lemma) in neighbours do
      unless neighbourMap == map do continue
      let fact ← attempt do
        let fits ← mkAppM lemma #[keys, found]
        pair fits signed
      if let some fact := fact then facts := facts.push fact
  for (map, values, bits, signed) in valued do
    for read in sitesWhere (fun e => e.isAppOf ``VerexIR.SpecMap.valueAt && e.getAppNumArgs ≥ 2 &&
        e.appFn!.appArg! == map && !e.hasLooseBVars) expressions do
      let fact ← attempt do
        let fits ← mkAppM ``VerexIR.SpecMap.valueAt_satisfies
          #[values, ← fallback bits signed, read.appArg!]
        pair fits signed
      if let some fact := fact then facts := facts.push fact
  return facts

/-- The user name of an inaccessible hypothesis a path's decision introduced: a
branch's condition, an arm's test, a failed check, an abort condition. The parts
of such a hypothesis keep it, as rewriting and substitution do. -/
def pathDecisionName : Name := `pathDecision

/-- Whether a user name marks a path's decision. -/
def isPathDecision (name : Name) : CoreM Bool := do
  return name.eraseMacroScopes == pathDecisionName ||
    (← (← targetState).decisionNames.get).contains name

/-- The terms each atom collection below has read, by collection: the atoms
a term contributes are already found once it was read. -/
private abbrev AtomsM := StateRefT (Std.HashSet (Nat × Lean.Expr)) MetaM

/-- Whether collection `kind` already read `e`; marks it read. -/
private def alreadyRead (kind : Nat) (e : Lean.Expr) : AtomsM Bool := do
  if (← get).contains (kind, e) then return true
  Core.checkSystem "atoms"
  modify (·.insert (kind, e))
  return false

mutual
/-- The atoms `omega` reads of a term: through sums, differences, negations,
literal multiples, quotients and remainders by literals, and coercions. Any
other term is one atom: a read, a projection chain, an uninterpreted
application. A constructor reads as its fields, a conditional as its condition
and branches. A term with no local is no atom; one under a binder reads as the
closed terms it is built from. -/
private partial def termAtoms (e : Lean.Expr) (found : Std.HashSet Lean.Expr) :
    AtomsM (Std.HashSet Lean.Expr) := do
  let e := e.consumeMData
  unless e.hasFVar do return found
  if ← alreadyRead 0 e then return found
  if e.hasLooseBVars then return ← closedParts e found
  if e.isAppOfArity ``HAdd.hAdd 6 || e.isAppOfArity ``HSub.hSub 6 then
    return ← termAtoms (e.getArg! 5) (← termAtoms (e.getArg! 4) found)
  if e.isAppOfArity ``HMul.hMul 6 && (!(e.getArg! 4).hasFVar || !(e.getArg! 5).hasFVar) then
    return ← termAtoms (e.getArg! 5) (← termAtoms (e.getArg! 4) found)
  if (e.isAppOfArity ``HDiv.hDiv 6 || e.isAppOfArity ``HMod.hMod 6) && !(e.getArg! 5).hasFVar then
    return ← termAtoms (e.getArg! 4) found
  if e.isAppOfArity ``Neg.neg 3 || e.isAppOfArity ``Nat.cast 3 ||
      e.isAppOfArity ``IntCast.intCast 3 || e.isAppOfArity ``Int.ofNat 1 ||
      e.isAppOfArity ``Int.toNat 1 then
    return ← termAtoms e.appArg! found
  if e.isAppOfArity ``ite 5 then
    return ← termAtoms (e.getArg! 4) (← termAtoms (e.getArg! 3) (← propositionAtoms (e.getArg! 1) found))
  if let .const name _ := e.getAppFn then
    if (← getEnv).isConstructor name then return ← explicitAtoms e found
  return found.insert e

/-- The closed terms with a local that a term under a binder is built from. -/
private partial def closedParts (e : Lean.Expr) (found : Std.HashSet Lean.Expr) :
    AtomsM (Std.HashSet Lean.Expr) := do
  unless e.hasFVar do return found
  if ← alreadyRead 1 e then return found
  unless e.hasLooseBVars do return ← termAtoms e found
  match e with
  | .app function argument => closedParts argument (← closedParts function found)
  | .lam _ _ body _ | .forallE _ _ body _ | .mdata _ body | .proj _ _ body => closedParts body found
  | .letE _ _ value body _ => closedParts body (← closedParts value found)
  | _ => return found

/-- The atoms of an application's explicit arguments. -/
private partial def explicitAtoms (e : Lean.Expr) (found : Std.HashSet Lean.Expr) :
    AtomsM (Std.HashSet Lean.Expr) := do
  if ← alreadyRead 2 e then return found
  let info ← getFunInfo e.getAppFn
  let mut found := found
  for argument in e.getAppArgs, index in [0:e.getAppNumArgs] do
    let explicit := match info.paramInfo[index]? with
      | some parameter => parameter.binderInfo.isExplicit
      | none => true
    if explicit then found ← termAtoms argument found
  return found

/-- The atoms a proposition reads: of the sides of its comparisons, through its
connectives and binders, and of a predicate's explicit arguments. -/
private partial def propositionAtoms (e : Lean.Expr) (found : Std.HashSet Lean.Expr) :
    AtomsM (Std.HashSet Lean.Expr) := do
  let e := e.consumeMData
  unless e.hasFVar do return found
  if ← alreadyRead 3 e then return found
  if e.isAppOfArity ``VerexIR.Proofs.Obligation 4 then return ← propositionAtoms (e.getArg! 3) found
  if e.isAppOfArity ``Not 1 then return ← propositionAtoms e.appArg! found
  if e.isAppOfArity ``And 2 || e.isAppOfArity ``Or 2 || e.isAppOfArity ``Iff 2 then
    return ← propositionAtoms (e.getArg! 1) (← propositionAtoms (e.getArg! 0) found)
  if let .forallE _ domain body _ := e then
    return ← propositionAtoms body (← propositionAtoms domain found)
  if e.isAppOfArity ``Exists 2 then
    if let .lam _ _ body _ := e.appArg!.consumeMData then return ← propositionAtoms body found
  if e.isAppOfArity ``Eq 3 || e.isAppOfArity ``Ne 3 then
    return ← termAtoms (e.getArg! 2) (← termAtoms (e.getArg! 1) found)
  if e.isAppOfArity ``LE.le 4 || e.isAppOfArity ``LT.lt 4 ||
      e.isAppOfArity ``GE.ge 4 || e.isAppOfArity ``GT.gt 4 then
    return ← termAtoms (e.getArg! 3) (← termAtoms (e.getArg! 2) found)
  if e.hasLooseBVars then return ← closedParts e found
  if e.isApp then return ← explicitAtoms e found
  return found.insert e
end

/-- Of a context's terms, those of the obligation: the goal, the path's
decisions, what is not a proposition, and the facts connected to those through
shared atoms (`connectedBy`). With no atom in the goal and the decisions, the
whole context. -/
private def obligationTerms (target : Lean.Expr) (decisions terms : Array Lean.Expr) :
    MetaM (Array Lean.Expr) := do
  let seeds ← (do
    decisions.foldlM (fun found decision => propositionAtoms decision found)
      (← propositionAtoms target {})).run' {}
  if seeds.isEmpty then return terms
  let mut facts : Array (Lean.Expr × Array Lean.Expr) := #[]
  let mut others : Array Lean.Expr := #[]
  for term in terms do
    if ← isProp term then
      facts := facts.push (term, (← (propositionAtoms term {}).run' {}).toArray)
    else others := others.push term
  let connected : Std.HashSet Lean.Expr := (connectedBy facts seeds).foldl (·.insert ·) {}
  return terms.filter fun term => connected.contains term || others.contains term

/-- Assert the facts a leaf needs beside its hypotheses: the bounds of every
certified local in context, and the bounds of the operation sites of the goal,
of the path's decisions and of the facts connected to them
(`obligationTerms`). A decider that brings
in another fact asserts them again, for what that fact connects. An authored
preparation (`typedOnly`) reads every term. A fact whose statement is in
`skip` is not asserted. A bounded arithmetic attempt can omit magnitude
facts, whose signed case splits may be unnecessary after call results are
rewritten. The statements asserted are returned. -/
def assertBounds (skip : Array Lean.Expr := #[]) (includeMagnitudes : Bool := true)
    (typedOnly : Bool := false) : TacticM (Array Lean.Expr) := focusGoal do
  if (← getGoals).isEmpty then return #[]
  let goal ← getMainGoal
  let facts ← goal.withContext do
    let mut facts : Array Lean.Expr := #[]
    let target ← instantiateMVars (← goal.getType)
    let mut expressions := #[target]
    let mut decisions := #[]
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let ty ← boundType decl.type
      expressions := expressions.push (← instantiateMVars decl.type)
      if ← isPathDecision decl.userName then decisions := decisions.push expressions.back!
      if let some fact ← typedBound? decl.toExpr ty then facts := facts.push fact
    unless typedOnly do expressions ← obligationTerms target decisions expressions
    -- A vector a term builds carries its length bound, which its writes and
    -- concatenations read (`SpecVector.values_push`, `values_append`).
    for built in sitesWhere (fun e => e.isAppOfArity ``VerexIR.SpecVector.mk 3 && !e.hasLooseBVars)
        expressions do
      facts := facts.push built.appArg!
    -- No value's serialization is empty.
    for serialized in sitesWhere
        (fun e => e.isAppOfArity ``VerexIR.Proofs.serializedBytes 4 && !e.hasLooseBVars) expressions do
      facts := facts.push (mkAppN (mkConst ``VerexIR.Proofs.serializedBytes_nonempty)
        serialized.getAppArgs)
    -- Certified integers read through a structure projection, such as a
    -- twin's field, carry their bounds too. A direct `.val` also exposes
    -- certificates of computed values, including a total optional read.
    for site in projectionSites (← getEnv) expressions do
      let value := if site.isAppOfArity ``VerexIR.SpecInt.val 3 then site.getArg! 2 else site
      let ty ← boundType (← inferType value)
      if ty.isAppOfArity ``VerexIR.SpecInt 2 then
        let width := ty.getArg! 0
        if width.isAppOfArity ``VerexIR.IntWidth.bits 1 then
          let lemma := if (ty.getArg! 1).isConstOf ``Bool.true
            then ``VerexIR.SpecInt.signed_bounds else ``VerexIR.SpecInt.unsigned_bounds
          facts := facts.push (mkApp2 (mkConst lemma) (width.getArg! 0) value)
    -- Authored preparation also bounds the values read from typed maps;
    -- the automatic leaf reads its maps as entries.
    if typedOnly then return facts ++ (← containerBounds expressions)
    let reads := divisionReads expressions
    let sites := operationSites expressions
    for site in sites do
      if let some fact ← operationFact? goal reads site then facts := facts.push fact
      if let some fact ← maskFact? site then facts := facts.push fact
    -- A conjunction mentioned with its operands in both orders is one value,
    -- which omega reads as two atoms otherwise.
    for site in sites do
      unless site.isAppOfArity ``VerexIR.Proofs.IntegerArithmetic.bitwiseAnd 2 do continue
      let left := site.getArg! 0
      let right := site.getArg! 1
      let swapped := mkApp2 site.appFn!.appFn! right left
      if left != right && sites.contains swapped && Lean.Expr.lt left right then
        facts := facts.push (← mkAppM ``VerexIR.Proofs.IntegerArithmetic.bitwiseAnd_comm
          #[left, right])
    -- A proof that does not assemble is not a fact.
    let attempt (fact : MetaM Lean.Expr) : MetaM (Option Lean.Expr) :=
      try pure (some (← fact)) catch _ => pure none
    -- Range certificates and their negations in context yield bounds as
    -- separate facts; rewriting them would leave casts in the terms that
    -- depend on them.
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let ty ← instantiateMVars decl.type
      let equation := if ty.isAppOfArity ``VerexIR.Proofs.Obligation 4 then ty.getArg! 3 else ty
      if equation.isAppOfArity ``Not 1 && (equation.getArg! 0).isAppOfArity ``Eq 3 then
        let inner := equation.getArg! 0
        -- Distinct certified values differ in their fields.
        let carrier ← whnfR (inner.getArg! 0)
        if carrier.isAppOfArity ``VerexIR.SpecInt 2 then
          facts := facts.push (← mkAppM ``VerexIR.Proofs.Denote.SpecInt.val_ne_of_ne #[decl.toExpr])
        else if carrier.isAppOfArity ``VerexIR.SpecVector 1 then
          facts := facts.push
            (← mkAppM ``VerexIR.Proofs.Denote.SpecVector.values_ne_of_ne #[decl.toExpr])
      if equation.isAppOfArity ``Eq 3 then
        let proof ← if ty.isAppOfArity ``VerexIR.Proofs.Obligation 4 then
            mkAppM ``Iff.mp #[← mkAppOptM ``VerexIR.Proofs.Obligation_iff
              #[ty.getArg! 0, ty.getArg! 1, ty.getArg! 2, equation], decl.toExpr]
          else pure decl.toExpr
        let lhs := equation.getArg! 1
        let rhs := equation.getArg! 2
        -- A position an array reads a value at lies within it, where the
        -- goal reads the array's size.
        if lhs.isAppOfArity ``GetElem?.getElem? 7 && rhs.isAppOfArity ``Option.some 2 then
          let array := lhs.getArg! 5
          if (target.find? fun e => e.isAppOfArity ``Array.size 2 && e.appArg! == array).isSome then
            if let some fact ← attempt (mkAppM
                ``VerexIR.Proofs.Denote.lt_size_of_getElem?_eq_some #[proof]) then
              facts := facts.push fact
      let (negated, fits) := if ty.isAppOfArity ``Not 1 then (true, ty.getArg! 0) else (false, ty)
      unless fits.isAppOfArity ``VerexIR.IntegerValueFits 3 do continue
      let width := fits.getArg! 0
      unless width.isAppOfArity ``VerexIR.IntWidth.bits 1 do continue
      let signed := (fits.getArg! 1).isConstOf ``Bool.true
      if negated then
        let some bits := (width.getArg! 0).nat? | continue
        if bits == 0 then continue
        let nonzero ← mkDecideProof (← mkAppM ``Ne #[width.getArg! 0, mkNatLit 0])
        let lemma := if signed then ``VerexIR.Proofs.Denote.bounds_of_not_fits_signed
          else ``VerexIR.Proofs.Denote.bounds_of_not_fits_unsigned
        facts := facts.push (← mkAppM lemma #[nonzero, decl.toExpr])
      else
        let lemma := if signed then ``VerexIR.Proofs.Denote.bounds_of_fits_signed
          else ``VerexIR.Proofs.Denote.bounds_of_fits_unsigned
        facts := facts.push (← mkAppM lemma #[decl.toExpr])
    pure facts
  -- A fact already in context is not added again: as stated, or as the
  -- normalized context states it, its literals reduced. The bound
  -- `2 ^ 64 - 1` of a certificate is otherwise asserted again beside its
  -- normalized copy at every pass, and renormalized.
  let mut goal := goal
  let reduceCtx ← Simp.mkContext {} (simpTheorems := #[]) (congrTheorems := ← getSimpCongrTheorems)
  let reduceProcs : Simp.SimprocsArray := #[← Simp.getSimprocs]
  let reduceLiterals (type : Lean.Expr) : MetaM Lean.Expr := do
    if let some reduced := (← (← targetState).reducedBounds.get)[type]? then return reduced
    let (result, _) ← Simp.main type reduceCtx (methods := Simp.mkDefaultMethodsCore reduceProcs)
    unless type.hasMVar do (← targetState).reducedBounds.modify (·.insert type result.expr)
    pure result.expr
  let mut known ← goal.withContext do
    (← getLCtx).foldlM (init := skip) fun known decl => do
      if decl.isImplementationDetail then pure known
      else pure (known.push (← instantiateMVars decl.type))
  let mut added := #[]
  for proof in facts do
    let type ← goal.withContext (do instantiateMVars (← inferType proof))
    if !includeMagnitudes && (type.find? (·.isConstOf ``Int.natAbs)).isSome then continue
    if known.contains type then continue
    let reduced ← goal.withContext (reduceLiterals type)
    if known.contains reduced then continue
    -- Its parts, once split, state it.
    let stated (part : Lean.Expr) : MetaM Bool := do
      return known.contains part || known.contains (← reduceLiterals part)
    if type.isAppOfArity ``And 2 then
      if (← goal.withContext (stated (type.getArg! 0))) &&
          (← goal.withContext (stated (type.getArg! 1))) then continue
    known := known.push type
    added := added.push type
    let asserted ← goal.assert `bounds type proof
    let (_, next) ← asserted.intro1P
    goal := next
    -- The projections of literal values in the fact reduce, so that a
    -- decision procedure reads the value, not the projection.
    setGoals [goal]
    let name := mkIdent `bounds
    evalTactic (← `(tactic| try dsimp only at $name:ident))
    goal ← getMainGoal
  replaceMainGoal [goal]
  return added

/-- Assert the bounds a leaf needs beside its hypotheses (`assertBounds`). -/
elab "verex_bounds" : tactic => discard assertBounds

/-- The value a fact bounds from both sides by closed terms (`0 ≤ v ∧ v ≤
2 ^ 64 - 1`), or the value of one side (`0 ≤ v`; `v ≤ 2 ^ 64 - 1`). -/
private def boundedSides (type : Lean.Expr) :
    Array (Lean.Expr × Bool × (Lean.Expr → Lean.Expr)) := Id.run do
  let side (part : Lean.Expr) : Option (Lean.Expr × Bool) :=
    if part.isAppOfArity ``LE.le 4 && (part.getArg! 0).isConstOf ``Int then
      let low := part.getArg! 2
      let high := part.getArg! 3
      if !low.hasFVar && !low.hasMVar then some (high, true)
      else if !high.hasFVar && !high.hasMVar then some (low, false)
      else none
    else none
  if type.isAppOfArity ``And 2 then
    let mut found := #[]
    if let some (value, lower) := side (type.getArg! 0) then
      found := found.push (value, lower, fun proof => mkProj ``And 0 proof)
    if let some (value, lower) := side (type.getArg! 1) then
      found := found.push (value, lower, fun proof => mkProj ``And 1 proof)
    return found
  match side type with
  | some (value, lower) => #[(value, lower, id)]
  | none => #[]

/-- Read the round trips of bounded values as the values: a value cut back to
an integer carrier (`SpecInt.ofInt`) whose bounds facts of the context state,
by the cut's lemma at those facts (`SpecInt.val_ofInt_*`). Only the goal and
the facts that mention such a cut are rewritten. -/
def cutBoundedRoundTrips : TacticM Unit := focusGoal do
  if (← getGoals).isEmpty then return
  let goal ← getMainGoal
  let found ← goal.withContext do
    let mut lows : Std.HashMap Lean.Expr (Array Lean.Expr) := {}
    let mut highs : Std.HashMap Lean.Expr (Array Lean.Expr) := {}
    let mut facts := #[]
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let type ← instantiateMVars decl.type
      facts := facts.push (decl.fvarId, type)
      for (value, lower, part) in boundedSides type do
        if lower then lows := lows.alter value (some <| ·.getD #[] |>.push (part decl.toExpr))
        else highs := highs.alter value (some <| ·.getD #[] |>.push (part decl.toExpr))
    let bounded (e : Lean.Expr) : Bool :=
      e.isAppOfArity ``VerexIR.SpecInt.ofInt 4 && !e.hasLooseBVars &&
        lows.contains e.appArg! && highs.contains e.appArg!
    let target ← instantiateMVars (← goal.getType)
    let mut lemmas := #[]
    for cut in sitesWhere bounded (facts.map (·.2) |>.push target) do
      let type ← inferType cut
      let width := type.getArg! 0
      unless width.isAppOfArity ``VerexIR.IntWidth.bits 1 do continue
      let some bits := width.appArg!.nat? | continue
      if bits == 0 then continue
      let lemma := if (type.getArg! 1).isConstOf ``Bool.true
        then ``VerexIR.Proofs.Denote.SpecInt.val_ofInt_signed
        else ``VerexIR.Proofs.Denote.SpecInt.val_ofInt_unsigned
      let value := cut.appArg!
      let lemma ← (lows.getD value #[]).findSomeM? fun low => (highs.getD value #[]).findSomeM? fun high =>
        observing? (mkAppM lemma #[mkNatLit (bits - 1), cut.getArg! 2, low, high])
      if let some lemma := lemma then lemmas := lemmas.push lemma
    if lemmas.isEmpty then return none
    let mentions (e : Lean.Expr) := (e.find? bounded).isSome
    return some (lemmas, facts.filterMap fun (fvarId, type) =>
      if mentions type then some fvarId else none, mentions target)
  let some (lemmas, targets, target) := found | return
  let mut equations : SimpTheorems := {}
  for name in [``VerexIR.SpecInt.ofInt_eq_iff, ``VerexIR.SpecInt.eq_ofInt_iff] do
    equations ← equations.addConst name
  let mut rules : SimpTheoremsArray := #[equations]
  for lemma in lemmas do rules ← rules.addTheorem (.other `cut) lemma
  let ctx ← Simp.mkContext { failIfUnchanged := false } rules (← getSimpCongrTheorems)
  let (result, _) ← simpGoal goal ctx #[] none (simplifyTarget := target) (fvarIdsToSimp := targets)
  replaceMainGoal (result.map (·.2) |>.toList)

/-- The certificates carried by the locals' types and the bounds of the values
read from typed maps (`assertBounds`), without deriving facts about every
operation in the context, and the round trips of the bounded values read as
the values. Manual proofs request the operations' lemmas at the positions they
actually use. -/
def typeBounds : TacticM Unit := do
  discard (assertBounds (typedOnly := true))
  cutBoundedRoundTrips

elab "verex_type_bounds" : tactic => typeBounds

/-- Smaller bounds for a speculative call-observation proof. The ordinary
solver retains every signed and magnitude fact when this attempt fails. -/
elab "verex_observation_bounds" : tactic =>
  discard (assertBounds (includeMagnitudes := false))

/-- The value projections emitted by cases in the same application form as
clause and write-back producers. Infer the parameters from the actual owner;
primitive projections omit them and otherwise become distinct solver atoms. -/
def canonicalValueProjection? (e : Lean.Expr) : MetaM (Option Lean.Expr) := do
  if e.hasLooseBVars then return none
  let .proj name index value := e | return none
  let projection ← match name, index with
    | ``Prod, 0 => pure ``Prod.fst
    | ``Prod, 1 => pure ``Prod.snd
    | ``Subtype, 0 => pure ``Subtype.val
    | _, _ => return none
  return some (← mkAppM projection #[value])

/-- A structure projection applied to a constructor application, reduced. -/
def reduceProjection? (e : Lean.Expr) : MetaM (Option Lean.Expr) := do
  if let .proj sourceName index value := e then
    let .const constructor _ := value.getAppFn | return none
    let some (.ctorInfo info) := (← getEnv).find? constructor | return none
    unless info.induct == sourceName do return none
    return value.getAppArgs[info.numParams + index]?
  let .const name _ := e.getAppFn | return none
  let some info ← getProjectionFnInfo? name | return none
  let arguments := e.getAppArgs
  let some value := arguments[info.numParams]? | return none
  let .const constructor _ := value.getAppFn | return none
  unless constructor == info.ctorName do return none
  let some field := value.getAppArgs[info.numParams + info.i]? | return none
  return some (mkAppN field (arguments.extract (info.numParams + 1) arguments.size))

/-- Reduce every projection of a constructor application, in the goal and in
every hypothesis. A substituted value leaves `{ val := e, … }.val` behind,
which arithmetic cannot see through; the reduction is a definitional change,
so it reaches the hypotheses other terms depend on, which `simp at *`
leaves alone. -/
elab "verex_reduce_projections" : tactic => focusGoal do
  let mut goal ← getMainGoal
  let reduce (e : Lean.Expr) : MetaM Lean.Expr :=
    -- Projection reduction reads constructor arguments only. Keep quantified
    -- clauses closed instead of rebuilding their telescopes on every pass.
    Core.transform e (post := fun e => do
      match ← reduceProjection? e with
      | some reduced => return .visit reduced
      | none => return .continue)
  for decl in ← goal.withContext getLCtx do
    if decl.isImplementationDetail || decl.isLet then continue
    let type ← goal.withContext (instantiateMVars decl.type)
    if (← (← targetState).projectionFreeTypes.get).contains type then continue
    let reduced ← goal.withContext (reduce type)
    if reduced != type then
      goal ← goal.replaceLocalDeclDefEq decl.fvarId reduced
    else unless type.hasMVar do (← targetState).projectionFreeTypes.modify (·.insert type)
  let target ← goal.withContext (instantiateMVars (← goal.getType))
  let reduced ← goal.withContext (reduce target)
  if reduced != target then goal ← goal.replaceTargetDefEq reduced
  replaceMainGoal [goal]

/-- Clear every hypothesis that speaks about computations rather than
values: the loop hypotheses and recursive iterations a leaf never needs. -/
elab "verex_clear_computations" : tactic => focusGoal do
  if (← getGoals).isEmpty then return
  let goal ← getMainGoal
  let victims ← goal.withContext do
    let mut found := #[]
    let mut computations : Std.HashSet FVarId := {}
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let ty ← instantiateMVars decl.type
      let constants := ty.getUsedConstants
      -- Saved continuations have a plain function type; their bodies carry
      -- the computation. Leaving unused ones here makes simp unfold the
      -- rest of the program again on an already extracted leaf.
      let savedComputation := decl.value?.any fun value =>
        let constants := value.getUsedConstants
        constants.contains ``VerexIR.Proofs.wp || constants.contains ``VerexIR.Proofs.Spec ||
          -- Continuation folding can introduce aliases of saved computations.
          -- Locals occur after their dependencies, so aliases of aliases are
          -- recognized in this same pass. Otherwise the last alias keeps the
          -- whole chain alive when the candidates are cleared in reverse.
          match value.consumeMData with
          | .fvar previous => computations.contains previous
          | _ => false
      if savedComputation || constants.contains ``VerexIR.Proofs.wp || constants.contains ``VerexIR.Proofs.Spec ||
          constants.contains ``VerexIR.Validation.prepareExecution ||
          constants.contains ``VerexIR.Validation.ExecutableUnit ||
          constants.contains ``VerexIR.Validation.SemanticsRegistry then
        found := found.push decl.fvarId
        computations := computations.insert decl.fvarId
    pure found
  let mut goal := goal
  for fvarId in victims.reverse do
    goal ← goal.tryClear fvarId
  replaceMainGoal [goal]

/-- Whether a term reads the slot a hypothesis equates (`memory r k`), up
to unfolding at reducible transparency: a clause and the program spell a
resource type with different proofs of its well-formedness. Each shared
subterm is visited once: a fact's term is a DAG whose tree is exponential. -/
private def mentionsRead (lhs : Lean.Expr) (e : Lean.Expr) : MetaM Bool := do
  let rec visit (e : Lean.Expr) : StateRefT (Std.HashSet Lean.Expr) MetaM Bool := do
    if (← get).contains e then return false
    if e.getAppFn == lhs.getAppFn && e.getAppNumArgs == 2 && !e.hasLooseBVars then
      if e == lhs then return true
      if ← withReducible (isDefEq e lhs) then return true
    let found ← match e with
      | .app f a => visit f <||> visit a
      | .lam _ d b _ | .forallE _ d b _ => visit d <||> visit b
      | .letE _ t v b _ => visit t <||> visit v <||> visit b
      | .mdata _ b | .proj _ _ b => visit b
      | _ => pure false
    unless found do modify (·.insert e)
    return found
  (visit e).run' {}

/-- `mentionsRead`, remembered for terms without metavariables. -/
private def mentionsReadCached (lhs : Lean.Expr) (e : Lean.Expr) : MetaM Bool := do
  if lhs.hasMVar || e.hasMVar then return ← mentionsRead lhs e
  let state ← targetState
  if let some known := (← state.readMentions.get)[(lhs, e)]? then return known
  let result ← mentionsRead lhs e
  state.readMentions.modify (·.insert (lhs, e) result)
  return result

/-- Read memory as the leaf's case splits fixed it: a slot a hypothesis
equates to a value (`memory r k = some v`, from a program read's split) is
that value wherever the goal reads it, in normal form, so that a clause's
read meets the program's before the arithmetic is normalized. -/
elab "verex_memory_reads" : tactic => focusGoal do
  if (← getGoals).isEmpty then return
  let goal ← getMainGoal
  let reads ← goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    let mut found : Array FVarId := #[]
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let ty ← instantiateMVars decl.type
      let some (_, lhs, _) := ty.eq? | continue
      let head := lhs.getAppFn
      unless head.isFVar && lhs.getAppNumArgs == 2 do continue
      unless (← whnfR (← inferType head)).isAppOfArity ``VerexIR.Proofs.Denote.Memory 2 do continue
      unless ← mentionsReadCached lhs target do continue
      found := found.push decl.fvarId
    pure found
  if reads.isEmpty then throwError "the goal reads no memory the leaf fixes"
  let lemmas ← goal.withContext do
    reads.mapM fun fvarId => do
      `(Lean.Parser.Tactic.simpLemma| $(← Lean.Elab.Term.exprToSyntax (.fvar fvarId)):term)
  evalTactic (← `(tactic| simp only [$lemmas,*, vir_denote_norm]))

/-- The heartbeats a leaf's attempt at an arbitrary instance may spend. The
attempt is extra work on a leaf its deciders already failed, so it must
neither exhaust the target's budget nor turn the leaf's report into a
timeout; a decided instance costs a few million. -/
def instanceAttemptHeartbeats : Nat := 20000000

/-- The heartbeats a side decision may spend: a condition a step decides to
choose the form it rewrites to, such as a discharger's side condition, an
implication's premise or an unfolding's guard. Undecided within them, the
step proceeds without the decision; a decided condition costs a few hundred
thousand. -/
def sideDecisionHeartbeats : Nat := 1000000

/-- Run an action within `allowance` heartbeats and within what is left of
the enclosing budget; exhausting it is an ordinary failure. -/
def withBudget (allowance : Nat) (act : TacticM α) : TacticM α := do
  let context ← readThe Core.Context
  let now ← IO.getNumHeartbeats
  let remaining := if context.maxHeartbeats == 0 then allowance
    else context.initHeartbeats + context.maxHeartbeats - now
  let budget := max 1 (min remaining allowance)
  tryCatchRuntimeEx
    (withTheReader Core.Context
      (fun context => { context with initHeartbeats := now, maxHeartbeats := budget }) act)
    fun failure => do
      if failure.isMaxHeartbeat then throwError "the attempt exceeded its heartbeat budget"
      throw failure

def evalBudgeted (allowance : Nat) (step : Syntax) : TacticM Unit :=
  withBudget allowance (evalTactic step)

/-- Run a tactic within `instanceAttemptHeartbeats` (and within what is left
of the enclosing budget); exhausting it is an ordinary failure, which the
enclosing `first` recovers from. -/
elab "verex_budgeted " step:tactic : tactic =>
  evalBudgeted instanceAttemptHeartbeats step

/-- Leave room for an authored proof: each speculative residual attempt may
spend at most five percent of the target's budget, capped at two million raw
heartbeats. A large target budget should not multiply the cost of unsuccessful
search on every leaf. Read the target's option rather than a temporary nested
solver cap. -/
def residualAllowance : CoreM Nat := do
  let targetBudget := Core.getMaxHeartbeats (← getOptions)
  return if targetBudget == 0 then 2000000 else min 2000000 (targetBudget / 20)

/-- Run a speculative residual attempt within `residualAllowance`. -/
elab "verex_residual_budgeted " step:tactic : tactic => do
  evalBudgeted (← residualAllowance) step

/-- Whether the goal or a hypothesis has a subterm satisfying `p`. -/
def mentionsAnywhere (goal : MVarId) (p : Lean.Expr → Bool) (skipLets := false) :
    MetaM Bool := goal.withContext do
  if ((← instantiateMVars (← goal.getType)).find? p).isSome then return true
  (← getLCtx).anyM fun decl => do
    if decl.isImplementationDetail || (skipLets && decl.isLet) then return false
    return ((← instantiateMVars decl.type).find? p).isSome

/-- Bound speculative solvers on leaves involving function values. Their
encoded behavior can trigger expensive definitional reduction even in the
arithmetic solver. Ordinary vector and reflection proofs retain their
existing solver budgets. -/
elab "verex_function_budgeted " step:tactic : tactic => do
  if (← getGoals).isEmpty then return
  let involvesFunction ← mentionsAnywhere (← getMainGoal)
    (·.isConstOf ``VerexIR.Proofs.Denote.NTy.function) (skipLets := true)
  if involvesFunction then
    evalTactic (← `(tactic| verex_budgeted $step))
  else evalTactic step

/-- Facts the arithmetic fast path can use without inspecting stored-state
invariants, closure contracts, or equalities between encoded values. -/
private partial def arithmeticProposition (type : Lean.Expr) : Bool :=
  if type.isAppOfArity ``Eq 3 then
    (type.getArg! 0).isConstOf ``Int || (type.getArg! 0).isConstOf ``Nat
  else if type.isAppOfArity ``LE.le 4 || type.isAppOfArity ``LT.lt 4 ||
      type.isAppOfArity ``GE.ge 4 || type.isAppOfArity ``GT.gt 4 then
    (type.getArg! 0).isConstOf ``Int || (type.getArg! 0).isConstOf ``Nat
  else if type.isAppOfArity ``Not 1 then arithmeticProposition (type.getArg! 0)
  else if type.isAppOfArity ``And 2 || type.isAppOfArity ``Or 2 then
    arithmeticProposition (type.getArg! 0) && arithmeticProposition (type.getArg! 1)
  else type.isConstOf ``False

/-- The arithmetic consequences of a fact: the fact where it is arithmetic,
else those of its conjuncts, of the negated disjuncts of a negated
disjunction, and of a marked clause's statement. -/
private partial def arithmeticParts (proof : Lean.Expr) : MetaM (Array Lean.Expr) := do
  let type ← instantiateMVars (← inferType proof)
  if arithmeticProposition type then return #[proof]
  if type.isAppOfArity ``VerexIR.Proofs.Obligation 4 then
    return ← arithmeticParts (← mkExpectedTypeHint proof (type.getArg! 3))
  if let some (left, right) := type.and? then
    return (← arithmeticParts (mkApp3 (mkConst ``And.left) left right proof)) ++
      (← arithmeticParts (mkApp3 (mkConst ``And.right) left right proof))
  let some negated := type.not? | return #[]
  if negated.isAppOfArity ``VerexIR.Proofs.Obligation 4 then
    return ← arithmeticParts (← mkExpectedTypeHint proof (mkNot (negated.getArg! 3)))
  let some (left, right) := negated.app2? ``Or | return #[]
  let parts ← mkAppM ``Iff.mp #[← mkAppOptM ``not_or #[left, right], proof]
  return (← arithmeticParts (← mkAppM ``And.left #[parts])) ++
    (← arithmeticParts (← mkAppM ``And.right #[parts]))

/-- Try only arithmetic hypotheses first, each fact's arithmetic
consequences (`arithmeticParts`). Even checked integer bounds become
expensive when omega preprocesses unrelated resource and function facts.
This speculative attempt is small; the caller retains the full-context solver
when its proof needs facts outside this syntactic subset. -/
elab "verex_arithmetic_only" : tactic => withBudget 500000 do
  let original ← getMainGoal
  let some goal ← original.falseOrByContra | replaceMainGoal []; return
  goal.withContext do
    let facts ← (← getLCtx).foldlM (init := #[]) fun facts decl => do
      if decl.isImplementationDetail || !(← isProp decl.type) then return facts
      return facts ++ (← arithmeticParts decl.toExpr)
    omegaHidden facts.toList goal
  replaceMainGoal []

/-- Filtering is useful only where memory-dependent facts would burden the
arithmetic solver. Pure arithmetic keeps its original proof construction,
whose terms are smaller than the speculative refutation's on simple bounds. -/
elab "verex_has_memory_facts" : tactic => focusGoal do
  let goal ← getMainGoal
  let applies ← goal.withContext do
    let mut memories : FVarIdSet := {}
    for decl in ← getLCtx do
      if decl.type.isAppOfArity ``VerexIR.Proofs.Denote.Memory 2 then
        memories := memories.insert decl.fvarId
    if memories.isEmpty then return false
    (← getLCtx).anyM fun decl => do
      if decl.isImplementationDetail || decl.isLet then return false
      unless ← isProp decl.type do return false
      return (decl.type.find? fun e => e.isFVar && memories.contains e.fvarId!).isSome
  unless applies do throwError "the leaf has no memory-dependent hypothesis"

/-- Whether omega reads a fact without splitting cases: atoms, conjunctions,
and negations of atoms or of disjunctions. Omega case-splits on the others
(implications, iffs, disjunctions), exponentially in their number: over a
few dozen conditional facts about one map a decision costs some 50k
heartbeats without them and 20M to 40M with them. -/
partial def splitFree (e : Lean.Expr) : Bool :=
  if e.isAppOfArity ``And 2 then splitFree (e.getArg! 0) && splitFree (e.getArg! 1)
  else if e.isAppOfArity ``Or 2 || e.isAppOfArity ``Iff 2 || e.isForall then false
  else if e.isAppOfArity ``Not 1 then
    let inner := e.appArg!
    if inner.isAppOfArity ``Not 1 then splitFree inner.appArg!
    else if inner.isAppOfArity ``Or 2 then
      splitFree (mkNot (inner.getArg! 0)) && splitFree (mkNot (inner.getArg! 1))
    else !(inner.isAppOfArity ``And 2 || inner.isAppOfArity ``Iff 2 || inner.isForall)
  else true

/-- Whether omega splits a fact into two cases it reads without splitting:
a disjunction, a negated conjunction or an implication of split-free facts,
as a checked operation's failed range is (`¬(0 ≤ x ∧ x ≤ max)`). -/
def oneSplit (e : Lean.Expr) : MetaM Bool := do
  if e.isAppOfArity ``Or 2 then return splitFree (e.getArg! 0) && splitFree (e.getArg! 1)
  if e.isAppOfArity ``Not 1 && e.appArg!.isAppOfArity ``And 2 then
    return splitFree (mkNot (e.appArg!.getArg! 0)) && splitFree (mkNot (e.appArg!.getArg! 1))
  unless e.isArrow do return false
  unless ← isProp e.bindingDomain! do return false
  return splitFree (mkNot e.bindingDomain!) && splitFree e.bindingBody!

/-- The facts with one case split omega takes beside the split-free ones:
a few, those sharing a variable with the goal first, so that the cases stay
bounded (`splitFree`). -/
def splitFactsBound : Nat := 3

/-- Linear arithmetic over the facts omega reads without splitting cases
(`splitFree`), a few with one split (`oneSplit`), and the goal's negation:
the fast path of `verex_linear`. -/
elab "verex_omega_unsplit" : tactic => focusGoal do
  let original ← getMainGoal
  let (before, goalVars) ← original.withContext do
    let known := (← getLCtx).foldl (init := ({} : FVarIdSet)) fun known decl => known.insert decl.fvarId
    return (known, (collectFVars {} (← instantiateMVars (← original.getType))).fvarSet)
  let some goal ← original.falseOrByContra | replaceMainGoal []; return
  goal.withContext do
    let mut facts := []
    let mut connected := #[]
    let mut other := #[]
    for decl in ← getLCtx do
      if decl.isImplementationDetail || !(← isProp decl.type) then continue
      let type ← instantiateMVars decl.type
      if !before.contains decl.fvarId || splitFree type then
        facts := decl.toExpr :: facts
      else if ← oneSplit type then
        if (collectFVars {} type).fvarSet.toList.any goalVars.contains then
          connected := connected.push decl.toExpr
        else other := other.push decl.toExpr
    let split := (connected ++ other).extract 0 splitFactsBound
    omegaHidden (facts ++ split.toList) goal
  replaceMainGoal []

/-- Linear arithmetic with case splits, the general fallback's
(`verex_pipeline`): over the facts omega reads without splitting cases first
(`verex_omega_unsplit`), then over the whole context within a speculative
attempt's allowance (`verex_residual_budgeted`), as its case splits on the
conditional facts are a search. The deciders before it read only the facts
omega needs no case split for. -/
macro "verex_linear" : tactic => `(tactic| verex_focus (
  first | verex_omega_unsplit | verex_residual_budgeted omega))

macro "verex_omega" : tactic => `(tactic| verex_focus (
  first | (verex_has_memory_facts; verex_arithmetic_only) | verex_omega_unsplit))


/-- Abstract the rank tables of the structural order (`valueRanks` of a
literal) to variables: no lemma reads inside one, and as literals they make
up much of a map term. Every occurrence of a table is abstracted at once; a
table in the type of a local stays. -/
def abstractRankTables (goal : MVarId) : MetaM MVarId := do
  let tables ← goal.withContext do
    let mut terms := #[← instantiateMVars (← goal.getType)]
    for decl in ← getLCtx do
      unless decl.isImplementationDetail do terms := terms.push (← instantiateMVars decl.type)
    return sitesWhere (fun e => e.isAppOfArity ``VerexIR.SemanticOperations.valueRanks 1 &&
      !e.hasFVar && !e.hasMVar && !e.hasLooseBVars) terms
  let mut goal := goal
  for table in tables do
    let hypotheses? ← goal.withContext do
      let mut hypotheses := #[]
      for decl in ← getLCtx do
        if decl.isImplementationDetail then continue
        let type ← instantiateMVars decl.type
        if table.occurs type then
          unless ← isProp type do return none
          hypotheses := hypotheses.push decl.fvarId
      return some hypotheses
    let some hypotheses := hypotheses? | continue
    -- A generalization that fails has reverted the hypotheses, assigning the
    -- goal: the state is restored, so the goal stays the one to prove.
    let saved ← saveState
    try
      let (_, _, next) ← goal.withContext <|
        goal.generalizeHyp #[{ expr := table, xName? := some `ranks }] hypotheses
      goal := next
    catch _ => saved.restore
  return goal

/-- Abstract the runtime frame (`Skolems.runtime`) to a variable, as the rank
tables are (`abstractRankTables`). Every native type of a leaf names the
frame's carriers, and a congruence closure unfolds the reducible frame down to
its instance literal, along paths that leave two occurrences of one term
apart: an opaque specification function applied to a value of a declared type
in a callee's fact and in the goal. Nothing a leaf decides reads the frame's
definition, and as a local of its class it is the instance the closure
canonicalizes the carriers to. The locals whose types name it are generalized
with it. -/
def abstractRuntimeFrames (goal : MVarId) : MetaM MVarId := do
  let frames ← goal.withContext do
    let mut terms := #[← instantiateMVars (← goal.getType)]
    for decl in ← getLCtx do
      unless decl.isImplementationDetail do terms := terms.push (← instantiateMVars decl.type)
    return sitesWhere (fun e => e.isAppOfArity ``Skolems.runtime 3 &&
      !e.hasMVar && !e.hasLooseBVars) terms
  let mut goal := goal
  for frame in frames do
    let hypotheses ← goal.withContext do
      let mut hypotheses := #[]
      for decl in ← getLCtx do
        if decl.isImplementationDetail then continue
        if frame.occurs (← instantiateMVars decl.type) then
          hypotheses := hypotheses.push decl.fvarId
      return hypotheses
    -- A generalization that fails has reverted the hypotheses, assigning the
    -- goal: the state is restored, so the goal stays the one to prove.
    let saved ← saveState
    try
      let (_, _, next) ← goal.withContext <|
        goal.generalizeHyp #[{ expr := frame, xName? := some `runtimeFrame }] hypotheses
      goal := next
    catch _ => saved.restore
  return goal

/-- An expression with the literal abbreviations a generated block names
unfolded (rows, sharing marks, types, site keys, and weaves: reducible
constants without parameters of those types) and the site keys' fields
read, so that a generated equation's sides are spelled as the program
spells its values. -/
def unfoldLiterals (e : Lean.Expr) : MetaM Lean.Expr := do
  let environment ← getEnv
  let literalType (type : Lean.Expr) : Bool :=
    type.isConstOf ``NRow || type.isConstOf ``NTy || type.isConstOf ``Sharing ||
      type.isConstOf ``SiteKey || type.isAppOf ``Weave
  Meta.transform e (post := fun e => do
    if let .const name [] := e then
      if let some (.defnInfo info) := environment.find? name then
        if literalType info.type && (← getReducibilityStatus name) == .reducible then
          return .visit info.value
    if e.isAppOfArity ``SiteKey.function 1 || e.isAppOfArity ``SiteKey.mask 1 ||
        e.isAppOfArity ``SiteKey.typeInstantiation 1 then
      return .done (← whnfR e)
    return .done e)

/-- A known function value: a construction (`Skolems.construct`) at any
frame, or a closure site's constructor at the base frame. Its frame, target,
mask, instantiation, captured row, captures, and the proof that its closure
is the closure of those (`Skolems.closure_construct`, a site's generated
`closure` equation). -/
structure LiteralValue where
  value : Lean.Expr
  /-- The parameter row of its function type. -/
  parameters : Lean.Expr
  frame : Lean.Expr
  handle : Lean.Expr
  mask : Lean.Expr
  instantiation : Lean.Expr
  captured : Lean.Expr
  captures : Lean.Expr
  closed : Lean.Expr

/-- Whether an expression is a closure site's constructor applied to its
captures. -/
def isSiteValue (environment : Environment) (e : Lean.Expr) : Bool :=
  match e.getAppFn with
  | .const name _ => e.getAppNumArgs == 1 && (siteEquationsAt? environment name).isSome
  | _ => false

/-- Whether an expression is a known function value (`LiteralValue`). -/
def isLiteralValue (environment : Environment) (e : Lean.Expr) : Bool :=
  e.isAppOfArity ``Skolems.construct 12 || isSiteValue environment e

/-- The arguments a site's generated equation takes at a value's captures:
the instance arguments the unit's family and the inhabitants of its
carriers give, resolved in the context the step runs in, and the captures at
the equation's one explicit binder.  An equation whose binders are any other
declines, so an equation is applied only where it is stated of the value at
hand and never with a capture read as an instance. -/
def siteEquationArguments? (statement captures : Lean.Expr) :
    MetaM (Option (Array Lean.Expr)) := do
  let capturesType ← inferType captures
  let mut arguments : Array Lean.Expr := #[]
  let mut type := statement
  let mut given := false
  while type.isForall do
    let domain := type.bindingDomain!
    let argument ← if type.bindingInfo! == .instImplicit then
        match ← trySynthInstance domain with
        | .some found => pure found
        | _ => return none
      else if given then return none
      else if ← isDefEq capturesType domain then
        given := true
        pure captures
      else return none
    arguments := arguments.push argument
    type := type.bindingBody!.instantiate1 argument
  if given then return some arguments else return none

/-- The parts of a known function value. -/
def literalValue? (e : Lean.Expr) : MetaM (Option LiteralValue) := do
  let e := e.consumeMData
  if e.isAppOfArity ``Skolems.construct 12 then
    return some {
      value := e, parameters := e.getArg! 3, frame := e.getArg! 2, handle := e.getArg! 6
      mask := e.getArg! 7, instantiation := e.getArg! 8, captured := e.getArg! 9
      captures := e.getArg! 10, closed := mkAppN (mkConst ``Skolems.closure_construct) e.getAppArgs }
  let .const constructor _ := e.getAppFn | return none
  unless e.getAppNumArgs == 1 do return none
  let some entry := siteEquationsAt? (← getEnv) constructor | return none
  -- The equation's statement over its captures, its literals unfolded once.
  let cache := (← targetState).siteClosureTypes
  let statement ← match (← cache.get).find? constructor with
    | some statement => pure statement
    | none =>
        let statement ← unfoldLiterals (← inferType (mkConst entry.closure))
        cache.modify (·.insert constructor statement)
        pure statement
  let some arguments ← siteEquationArguments? statement e.appArg! | return none
  let closedType ← instantiateForall statement arguments
  let closed ← mkExpectedTypeHint (mkAppN (mkConst entry.closure) arguments) closedType
  let some (_, read, closure) := closedType.eq? | return none
  unless closure.isAppOfArity ``closureOf 8 && read.isAppOfArity ``TypedFunctions.closure 3 do
    return none
  let functions := read.getArg! 1
  unless functions.isAppOfArity ``Carriers.functions 4 do return none
  return some {
    value := e, parameters := functions.getArg! 1, frame := closure.getArg! 2
    handle := closure.getArg! 3, mask := closure.getArg! 4, instantiation := closure.getArg! 5
    captured := closure.getArg! 6, captures := e.appArg!, closed }

/-- The unit constant of the base frame (`Skolems.runtime` at a unit
constant), if a frame is it: `Skolems.runtime` takes the unit and the two
instances the family and its inhabitants give, so an applied base frame
carries three arguments.

The steps gated on this read a term at the base frame: `constructSite` and
`siteOfConstruct?` read a construction there as the site's constructor, and
`requiresOfConstruct` declines there, the construction's own equation
having moved it already. -/
def baseFrameUnit? (frame : Lean.Expr) : Option Name :=
  let frame := frame.consumeMData
  if frame.isAppOfArity ``Skolems.runtime 3 then (frame.getArg! 0).constName? else none

/-- Whether two applications of one constant have defeq arguments, compared
one by one: a mismatch does not unfold the constant, which over a unit's
checks would evaluate them. -/
def argumentsDefEq (left right : Lean.Expr) : MetaM Bool := do
  let (left, right) := (left.consumeMData, right.consumeMData)
  unless left.getAppFn.isConst && left.getAppFn.constName? == right.getAppFn.constName? &&
      left.getAppNumArgs == right.getAppNumArgs do return false
  for (l, r) in left.getAppArgs.zip right.getAppArgs do
    unless ← isDefEq l r do return false
  return true

/-- The closure site's constructor a construction at the base frame is,
read off its target, mask, instantiation and captures alone, whatever
proof types it (a clause's may be bound). -/
def siteOfConstruct? (e : Lean.Expr) : MetaM (Option LiteralValue) := withDefault do
  let e := e.consumeMData
  unless e.isAppOfArity ``Skolems.construct 12 do return none
  let arguments := e.getAppArgs
  if arguments.pop.any (·.hasLooseBVars) then return none
  let some unit := baseFrameUnit? (arguments[2]!) | return none
  let closure := mkAppN (mkConst ``closureOf)
    #[arguments[0]!, arguments[1]!, arguments[2]!, arguments[6]!, arguments[7]!, arguments[8]!,
      arguments[9]!, arguments[10]!]
  for entry in siteEquationsOf (← getEnv) unit do
    let saved ← saveState
    try
      let statement ← inferType (mkConst entry.closure)
      let some closureArguments ← siteEquationArguments? statement arguments[10]! | continue
      let some (_, _, read) :=
        (← instantiateMVars (← instantiateForall statement closureArguments)).eq? | continue
      if ← argumentsDefEq read closure then
        return ← literalValue? (mkApp (mkConst entry.constructor) arguments[10]!)
    catch _ => pure ()
    saved.restore
  return none

/-- A closed equation premise of closed data, by definitional equality
(`Eq.refl`). -/
def reflEquationPremise (condition : Lean.Expr) : MetaM (Option Lean.Expr) := do
  let_expr Eq _ left right := condition | return none
  if condition.hasMVar then return none
  unless ← isDefEq left right do return none
  return some (← mkEqRefl left)

/-- Rewrite by an equation whose left-hand side unifies with the
expression, its open equation premises closed by definitional equality. -/
def rewriteByEquation? (rule : Name) (e : Lean.Expr) : MetaM (Option Simp.Step) :=
    withDefault do
  let some (rhs, proof) ← rewriteByEquationCore? rule e fun _ => reflEquationPremise
    | return none
  return some (.visit { expr := ← unfoldLiterals rhs, proof? := some proof })

/-- The certified captures a site's statements are quantified at, read from
the plain row a value holds: the row under exactly one `HList.forget` layer
where the value spells it, and otherwise the row whose forgetting the plain
row is, found by `isDefEq` at default transparency — a plain value of a
certificate-free type is the certified value itself, and a certified
value's plain reading is its `.val`.  `none` where no row of the site's own
forgets to it, so that no step states a site's equation of captures it
cannot name.

Built at the row type by `HList.nil` and `HList.cons`, not as the unit and
the pair they unfold to: `HList` is deliberately not reducible, so a bare
pair is type-correct only where it unfolds, and a step whose result a
tactic reads again is checked without unfolding it. -/
partial def certifiedCaptures? (family row plain : Lean.Expr) : MetaM (Option Lean.Expr) :=
  withDefault do
    if plain.isAppOfArity ``HList.forget 3 then
      if ← isDefEq (plain.getArg! 1) row then return some (plain.getArg! 2)
    let row ← whnfR row
    if row.isAppOfArity ``NRow.nil 0 then
      return some (← mkAppOptM ``HList.nil #[some family])
    unless row.isAppOfArity ``NRow.cons 2 do return none
    let (τ, rest) := (row.getArg! 0, row.getArg! 1)
    -- An entry: the certified value the plain one forgets, by `isDefEq` at
    -- the entry itself, so that the equation compares two values of the
    -- type and never a projection of a row yet to be found.
    let entry ← mkFreshExprMVar (mkApp2 (mkConst ``NTy.carrier) family τ)
    let head ← mkAppOptM ``Prod.fst #[none, none, plain]
    unless ← isDefEq (mkApp3 (mkConst ``NTy.forget) family τ entry) head do return none
    let entry ← instantiateMVars entry
    if entry.hasExprMVar then return none
    let some tail ← certifiedCaptures? family rest (← mkAppOptM ``Prod.snd #[none, none, plain])
      | return none
    return some (← mkAppOptM ``HList.cons
      #[some family, some τ, some rest, some entry, some tail])

open Lean Meta Simp in
/-- A construction at the base frame of a closure site's key is the site's
constructor (the generated `construct` equations). -/
simproc [vir_denote, vir_denote_norm] constructSite
    (@Skolems.construct _ ?functions ?frame _ _ _ _ _ _ _ _ _) := fun e => do
  unless e.isAppOfArity ``Skolems.construct 12 do return .continue
  let some unit := baseFrameUnit? (e.getArg! 2) | return .continue
  for entry in siteEquationsOf (← getEnv) unit do
    if let some step ← rewriteByEquation? entry.construct e then return step
  return .continue

open Lean Meta Simp in
/-- `requires_of` of a construction at a frame, of rows without type
parameters, is the requires meaning of the construction at the base frame
(`requiresOf_construct_paramFree`), which `constructSite` reads.

Declines at the base frame, where `constructSite` reads the construction to
the site's constructor instead; the two steps are one chain, and which of
them a term reaches depends on `baseFrameUnit?`, whose arity recognises no
frame today. -/
simproc [vir_denote, vir_denote_norm] requiresOfConstruct
    (@VerexIR.Proofs.RequiresOf _ ?functions ?frame _ _ _ _ _ _ _) := fun e => do
  unless e.isAppOfArity ``VerexIR.Proofs.RequiresOf 10 do return .continue
  unless (e.getArg! 7).consumeMData.isAppOfArity ``Skolems.construct 12 do return .continue
  if (baseFrameUnit? (e.getArg! 2)).isSome then return .continue
  if let some step ← rewriteByEquation? ``VerexIR.Proofs.requiresOf_construct_paramFree e then
    return step
  return .continue

open Lean Meta Simp in
/-- Invoking a closure site's constructor at the base frame is calling its
target on the woven arguments (the generated `invoke` equations).

The equation is stated of the captures under one `HList.forget` layer, as a
site's statements are quantified at their certified row.  A value whose
captures the normal forms collapsed to the plain row no longer spells that
forgetting, and matching the equation would have to invert `HList.forget`,
which is not a pattern: the certified row is recovered from the plain one
and the value respelled at it (`certifiedCaptures?`), as a site's requires
arm recovers it (`siteRequiresArm?`).  The respelling is definitional, so
the equation's proof states the rewriting of the value as it stands.
Declines where no row of the site's own forgets to the value's. -/
simproc [vir_denote, vir_denote_norm] invokeSite
    (@closureMeaning _ ?functions _ ?frame _ _ _ _ _) := fun e => do
  unless e.getAppNumArgs == 9 do return .continue
  let function := (e.getArg! 7).consumeMData
  let .const constructor _ := function.getAppFn | return .continue
  let some entry := siteEquationsAt? (← getEnv) constructor | return .continue
  let some invoke := entry.invoke | return .continue
  let mut subject := e
  if function.getAppNumArgs == 1 &&
      !function.appArg!.consumeMData.isAppOfArity ``HList.forget 3 then
    -- The row the equation is stated at, read from its own statement: a
    -- site's captured row is closed there, while the family is the one the
    -- meaning is taken at, which is the family the captures live at.
    let (_, _, body) ← forallMetaTelescope (← inferType (mkConst invoke))
    let some (_, left, _) := (← whnfR body).eq? | return .continue
    unless left.getAppNumArgs == 9 do return .continue
    let stated := (left.getArg! 7).consumeMData
    unless stated.getAppNumArgs == 1 do return .continue
    let forgotten := stated.appArg!.consumeMData
    unless forgotten.isAppOfArity ``HList.forget 3 do return .continue
    -- The row with its generated literal unfolded: a row named by a
    -- constant makes the forgetting type-correct only where that constant
    -- unfolds, and the value the step builds is read again at the
    -- transparency a tactic checks it at.
    let row ← unfoldLiterals (← instantiateMVars (forgotten.getArg! 1))
    if row.hasExprMVar then return .continue
    let family := mkAppN (mkConst ``Skolems.toCarriers)
      #[e.getArg! 0, e.getArg! 1, e.getArg! 3]
    let some recovered ← certifiedCaptures? family row function.appArg! | return .continue
    subject := mkAppN e.getAppFn ((e.getAppArgs).set! 7 (mkApp function.getAppFn
      (← mkAppOptM ``HList.forget #[some family, some row, some recovered])))
  let some (.visit result) ← rewriteByEquation? invoke subject | return .continue
  -- The arm, marked for the invocation's site (`dispatchKnownInvocation`).
  let some proof := result.proof? | return .continue
  -- `closureMeaning`'s unit and function values lead its arguments; the
  -- invocation's result carrier is its type's last argument.
  let type ← whnfR (← inferType e)
  let some α := type.getAppArgs.back?
    | return .visit { result with
        proof? := some (← mkExpectedTypeHint proof (← mkEq e result.expr)) }
  let marked := mkAppN (mkConst ``dispatchAt) #[type.getArg! 0, α, mkNatLit 0,
    mkStrLit entry.target, result.expr]
  return .visit { expr := marked, proof? := some (← mkExpectedTypeHint proof (← mkEq e marked)) }

/-- Rewrite by an equation about two known function values: its left-hand
side unified with the expression, then its proofs that their closures are
their targets' (`closedLeft`, `closedRight`) by the values' own, and its
open premises decided. -/
private def rewriteKnownPair? (rule : Name) (e : Lean.Expr) (left right : LiteralValue) :
    MetaM (Option Simp.Step) := withDefault do
  let premise (_ : Name) (condition : Lean.Expr) : MetaM (Option Lean.Expr) := do
    unless ← isProp condition do return none
    decidedFrameFree? condition
  let some (rhs, proof) ← rewriteByEquationCore? rule e premise
      #[(`closedLeft, left.closed), (`closedRight, right.closed)]
    | return none
  return some (.visit { expr := ← unfoldLiterals rhs, proof? := some proof })

/-- Whether two known function values are at one key: target, mask and
instantiation. -/
private def sameKey (left right : LiteralValue) : MetaM Bool := withDefault do
  isDefEq left.handle right.handle <&&> isDefEq left.mask right.mask <&&>
    isDefEq left.instantiation right.instantiation

open Lean Meta Simp in
/-- The equality keys of two known function values: their captures' equality
without ghosts at one key (`erased_eq_closureOf`), unequal at different keys
(`erased_eq_closureOf_ne`). -/
simproc [vir_denote, vir_denote_norm] knownFunctionsEqual
    (@Eq _ (TypedFunctions.erased _ _) (TypedFunctions.erased _ _)) := fun e => do
  let_expr Eq _ leftKey rightKey := e | return .continue
  unless leftKey.isAppOfArity ``TypedFunctions.erased 3 &&
      rightKey.isAppOfArity ``TypedFunctions.erased 3 do return .continue
  let some left ← literalValue? leftKey.appArg! | return .continue
  let some right ← literalValue? rightKey.appArg! | return .continue
  let rule := if ← sameKey left right then ``erased_eq_closureOf else ``erased_eq_closureOf_ne
  return (← rewriteKnownPair? rule e left right).getD .continue

open Lean Meta Simp in
/-- The order of two known function values: their captures' at one key
(`order_closureOf`), their keys' at different keys of one instantiation
(`order_closureOf_ne`). -/
simproc [vir_denote, vir_denote_norm] knownFunctionsOrder
    (TypedFunctions.order _ _ _ _) := fun e => do
  unless e.isAppOfArity ``TypedFunctions.order 5 do return .continue
  let some left ← literalValue? (e.getArg! 3) | return .continue
  let some right ← literalValue? (e.getArg! 4) | return .continue
  let rule := if ← sameKey left right then ``order_closureOf else ``order_closureOf_ne
  return (← rewriteKnownPair? rule e left right).getD .continue

attribute [vir_denote_norm] closureKeyOrder

open Lean Meta Simp in
/-- A source `let` (`head` is `letAt` or `letRowAt`) outside a step proof is
its body: only a step proof's walk names values by their source names. -/
private def eraseLetSite (head : Name) (e : Lean.Expr) : SimpM DStep := do
  if (← scopeTable?).isSome then return .continue
  unless e.isAppOf head do return .continue
  return .visit e.appArg!

open Lean Meta Simp in
/-- In a step proof, a source `let` (`head`) under `wp` is a binding site:
what its body owes, under the binding (`binder`, whose arguments `implicits`
selects from the `let`'s), which the walk meets (`bindLet`). By definition. -/
private def bindLetSite (head binder : Name) (implicits : Array Lean.Expr → Array Lean.Expr)
    (e : Lean.Expr) : SimpM DStep := do
  if (← scopeTable?).isNone then return .continue
  unless e.isAppOfArity ``VerexIR.Proofs.wp 7 do return .continue
  let action := (e.getArg! 3).consumeMData
  unless action.isAppOf head do return .continue
  let arguments := action.getAppArgs
  let body := mkAppN e.getAppFn (e.getAppArgs.set! 3 arguments.back!)
  return .visit (mkAppN (mkConst binder) (implicits arguments |>.push body))

/-- A source `let` outside a step proof is its body (`eraseLetSite`). -/
dsimproc ↓ [vir_denote, vir_denote_norm] eraseLetAt (@letAt _ _ _ _ _ _ _ _) :=
  eraseLetSite ``letAt

/-- In a step proof, a source `let` is a binding site (`LetIn`,
`bindLetSite`); `LetIn`'s implicit arguments are `letAt`'s after its result
type. -/
dsimproc [vir_denote, vir_denote_norm] bindLetAt
    (@VerexIR.Proofs.wp _ _ _ (@letAt _ _ _ _ _ _ _ _) _ _ _) :=
  bindLetSite ``letAt ``LetIn fun arguments => arguments.extract 2 (arguments.size - 1)

/-- A row's `let` in a step proof, as `bindLetAt` (`LetRowIn`). -/
dsimproc [vir_denote, vir_denote_norm] bindLetRowAt
    (@VerexIR.Proofs.wp _ _ _ (@letRowAt ?carriers _ _ _ _ _ _ _) _ _ _) :=
  bindLetSite ``letRowAt ``LetRowIn fun arguments =>
    #[arguments[0]!] ++ arguments.extract 3 (arguments.size - 1)

/-- A row's `let` outside a step proof is its body (`eraseLetSite`). -/
dsimproc ↓ [vir_denote, vir_denote_norm] eraseLetRowAt
    (@letRowAt ?carriers _ _ _ _ _ _ _) :=
  eraseLetSite ``letRowAt


/-- An invocation at a site, its arm known: the arm a site's constructor
takes, which the invocation's rewriting marked (`invokeSite`) without the
site, gets the site; an invocation of the family's `foreign` is marked. So
the path records the arm (`dispatchAt`). -/
dsimproc [vir_denote, vir_denote_norm] dispatchKnownInvocation (@opAt _ _ _ _) :=
    fun e => do
  unless e.isAppOfArity ``opAt 4 do return .continue
  let arguments := e.getAppArgs
  let invocation := arguments[3]!.consumeMData
  let mark (target invocation : Lean.Expr) : Lean.Expr :=
    mkAppN e.getAppFn (arguments.set! 3 (mkAppN (.const ``dispatchAt e.getAppFn.constLevels!)
      #[arguments[0]!, arguments[1]!, arguments[2]!, target, invocation]))
  if invocation.isAppOfArity ``dispatchAt 5 then
    -- Marked without the site where the invocation was rewritten.
    if (invocation.getArg! 2).rawNatLit? == some 0 || (invocation.getArg! 2).nat? == some 0 then
      return .done (mark (invocation.getArg! 3) (invocation.getArg! 4))
    return .continue
  let head := invocation.getAppFn
  unless head.isConstOf ``closureMeaning || head.isFVar do return .continue
  unless invocation.getAppNumArgs ≥ 2 do return .continue
  let function := (invocation.getArg! (invocation.getAppNumArgs - 2)).consumeMData
  let .const constructor _ := function.getAppFn | return .continue
  unless isForeignConstructor (← getEnv) constructor do return .continue
  return .done (mark (mkStrLit "foreign") invocation)

/-- The unit's check of a literal function value a clause writes, where the
literal is a closure site's: the site's generated `check` theorem. -/
def siteCheckProof? (check : Lean.Expr) : MetaM (Option Lean.Expr) := withDefault do
  let_expr Eq _ checked _ := check | return none
  unless checked.isAppOfArity ``closureTypingCheck 7 do return none
  let some unit := (checked.getArg! 0).constName? | return none
  for entry in siteEquationsOf (← getEnv) unit do
    let proof := mkConst entry.check
    let saved ← saveState
    let some (_, stated, _) := (← inferType proof).eq? | continue
    if ← argumentsDefEq stated checked then return some proof
    saved.restore
  return none

/-- Abstract the sites `select` picks in a goal to variables named `name`,
every occurrence of a site at once; a site in the type of a local that is no
proposition stays. -/
private def abstractSites (goal : MVarId) (select : Lean.Expr → Bool) (name : Name) :
    MetaM MVarId := do
  let values ← goal.withContext do
    let mut terms := #[← instantiateMVars (← goal.getType)]
    for decl in ← getLCtx do
      unless decl.isImplementationDetail do terms := terms.push (← instantiateMVars decl.type)
    return sitesWhere (fun e => select e && !e.hasMVar && !e.hasLooseBVars) terms
  let mut goal := goal
  for value in values do
    let hypotheses? ← goal.withContext do
      let mut hypotheses := #[]
      for decl in ← getLCtx do
        if decl.isImplementationDetail then continue
        let type ← instantiateMVars decl.type
        if value.occurs type then
          unless ← isProp type do return none
          hypotheses := hypotheses.push decl.fvarId
      return some hypotheses
    let some hypotheses := hypotheses? | continue
    -- As for rank tables: a failed generalization leaves the state as it was.
    let saved ← saveState
    try
      let (_, _, next) ← goal.withContext <|
        goal.generalizeHyp #[{ expr := value, xName? := some name }] hypotheses
      goal := next
    catch _ => saved.restore
  return goal

/-- Abstract the function values of a goal to variables: `grind` reads a
function value (`Skolems.construct`, `functionOfGeneric`, a site's constructor: a
closure with its rows, sharing, captures and typing certificate) only as a whole, equal to itself,
and its parts make up much of a behavior fact. -/
def abstractFunctionValues (goal : MVarId) : MetaM MVarId := do
  let environment ← getEnv
  abstractSites goal (fun e => e.isAppOf ``VerexIR.Proofs.Denote.Skolems.construct ||
    e.isAppOf ``VerexIR.Proofs.Denote.functionOfGeneric || isSiteValue environment e) `function

/-- Introduce the binders and premises a goal states, through the
`Obligation` markers heading them; whether any was introduced. -/
def introduceThroughMarkers (goal : MVarId) : MetaM (MVarId × Bool) := do
  let mut goal := goal
  let mut introduced := #[]
  repeat
    let target ← goal.withContext (instantiateMVars (← goal.getType))
    if target.isAppOfArity ``VerexIR.Proofs.Obligation 4 then
      if verex.keepObligationMarkers.get (← getOptions) then break
      goal ← goal.replaceTargetDefEq (target.getArg! 3)
    else if target.isForall then
      let (local_, next) ← goal.intro1P
      goal := next
      introduced := introduced.push local_
    else break
  return (goal, !introduced.isEmpty)

/-- Reads of one array at positions `omega` proves equal, spelled alike:
`grind` relates its arithmetic to its congruence closure only through the
terms it is given, so `xs[i.toNat]` and `xs[n - 1 - (↑n - 1 - i).toNat]?`
stay apart for it. A position is rewritten, in the goal and the
hypotheses, to the shallowest one equal to it; the goal's binders are
introduced first. -/
private def identifyReads (goal : MVarId) : TacticM MVarId := do
  let (goal, _) ← introduceThroughMarkers goal
  let isRead (e : Lean.Expr) : Bool :=
    (e.isAppOfArity ``GetElem?.getElem? 7 || e.isAppOfArity ``GetElem.getElem 8) &&
      (e.getArg! 1).isConstOf ``Nat && !e.hasLooseBVars && !e.hasMVar
  let reads ← goal.withContext do
    let mut terms := #[← instantiateMVars (← goal.getType)]
    for decl in ← getLCtx do
      unless decl.isImplementationDetail do
        let type ← instantiateMVars decl.type
        if ← isProp type then terms := terms.push type
    return sitesWhere isRead terms
  let mut groups : Array (Lean.Expr × Array Lean.Expr) := #[]
  for read in reads do
    let collection := read.getArg! 5
    let position := read.getArg! 6
    match groups.findIdx? (·.1 == collection) with
    | some index =>
        unless groups[index]!.2.contains position do
          groups := groups.modify index fun (collection, positions) =>
            (collection, positions.push position)
    | none => groups := groups.push (collection, #[position])
  let mut goal := goal
  for (_, positions) in groups do
    let ordered := positions.qsort (·.approxDepth < ·.approxDepth)
    for deeper in [1:ordered.size] do
      for shallower in [0:deeper] do
        let proof? ← goal.withContext do
          let proof ← mkFreshExprMVar (← mkEq ordered[deeper]! ordered[shallower]!)
          try
            let remaining ← Lean.Elab.Tactic.run proof.mvarId! (evalTactic (← `(tactic| omega)))
            if remaining.isEmpty then return some (← instantiateMVars proof)
            return none
          catch _ => return none
        let some proof := proof? | continue
        goal ← goal.withContext do
          let mut goal := goal
          for decl in ← getLCtx do
            if decl.isImplementationDetail then continue
            let type ← instantiateMVars decl.type
            unless ← (pure (ordered[deeper]!.occurs type) <&&> isProp type) do continue
            try
              let rewritten ← goal.rewrite type proof
              goal := (← goal.replaceLocalDecl decl.fvarId rewritten.eNew rewritten.eqProof).mvarId
            catch _ => pure ()
          try
            let rewritten ← goal.rewrite (← goal.getType) proof
            goal ← goal.replaceTargetEq rewritten.eNew rewritten.eqProof
          catch _ => pure ()
          return goal
        break
  return goal

/-- An equation between arrays, element by element: the reads of its two
sides, by the element equations of the operations that built them. `grind`
finds the extensionality itself only at many times the cost. `none` when
the equations decide it. -/
private def arraysByElements (goal : MVarId) : TacticM (Option MVarId) := do
  let (goal, _) ← introduceThroughMarkers goal
  let target ← goal.withContext do instantiateMVars (← goal.getType)
  let some (type, _, _) := target.eq? | return some goal
  unless type.isAppOfArity ``Array 1 do return some goal
  let saved ← saveState
  try
    let [next] ← goal.apply (← mkConstWithFreshMVarLevels ``Array.ext_getElem?)
      | saved.restore; return some goal
    let (_, next) ← next.intro1P
    setGoals [next]
    evalTactic (← `(tactic| try simp only [vir_denote_norm, Array.getElem?_eraseIdx,
      Array.getElem?_insertIdx, Array.getElem?_setIfInBounds, Array.getElem?_push,
      Array.getElem?_append]))
    return (← getGoals).head?
  catch _ =>
    saved.restore
    return some goal

/-- The products of two non-literal integer factors a term holds. -/
private def integerProducts (e : Lean.Expr) : Array (Lean.Expr × Lean.Expr) :=
  sitesWhere (fun e => e.isAppOfArity ``HMul.hMul 6 && (e.getArg! 0).isConstOf ``Int &&
    !e.hasLooseBVars && (e.getArg! 4).int?.isNone && (e.getArg! 5).int?.isNone &&
    (e.getArg! 4).nat?.isNone && (e.getArg! 5).nat?.isNone) #[e]
  |>.map fun product => (product.getArg! 4, product.getArg! 5)

/-- Whether a linear integer term reads a factor as one of its summands,
possibly scaled by a literal. -/
private partial def readsLinearly (factor : Lean.Expr) (term : Lean.Expr) : Bool :=
  term == factor ||
    if term.isAppOfArity ``HAdd.hAdd 6 || term.isAppOfArity ``HSub.hSub 6 then
      readsLinearly factor (term.getArg! 4) || readsLinearly factor (term.getArg! 5)
    else if term.isAppOfArity ``Neg.neg 3 then readsLinearly factor (term.getArg! 2)
    else if term.isAppOfArity ``HMul.hMul 6 then
      ((term.getArg! 4).int?.isSome && readsLinearly factor (term.getArg! 5)) ||
        ((term.getArg! 5).int?.isSome && readsLinearly factor (term.getArg! 4))
    else false

/-- Where a leaf multiplies integers, the products of the bounds of one
factor with the bounds of the other, as nonnegative facts: for `0 ≤ x` and
`y ≤ u`, `0 ≤ x * (u - y)`, which bounds `x * y` linearly in the product as
an atom (the McCormick bounds of a product). A bound of a factor is an
integer comparison that reads it linearly. -/
private def assertProductBounds (goal : MVarId) : MetaM MVarId := goal.withContext do
  let mut products := integerProducts (← instantiateMVars (← goal.getType))
  let mut bounds : Array (Lean.Expr × Lean.Expr) := #[]
  for decl in ← getLCtx do
    if decl.isImplementationDetail then continue
    let type ← instantiateMVars decl.type
    products := products ++ integerProducts type
    -- `x ≤ y` gives `0 ≤ y - x`, `x < y` gives `0 ≤ y - (x + 1)`.
    let isInt (e : Lean.Expr) := (e.getArg! 0).isConstOf ``Int
    if type.isAppOfArity ``LE.le 4 && isInt type then
      let difference ← mkAppM ``HSub.hSub #[type.getArg! 3, type.getArg! 2]
      bounds := bounds.push (difference, ← mkAppM ``Int.sub_nonneg_of_le #[decl.toExpr])
    else if type.isAppOfArity ``LT.lt 4 && isInt type then
      let lower ← mkAppM ``HAdd.hAdd #[type.getArg! 2, mkIntLit 1]
      let difference ← mkAppM ``HSub.hSub #[type.getArg! 3, lower]
      bounds := bounds.push (difference,
        ← mkAppM ``Int.sub_nonneg_of_le #[← mkAppM ``Int.add_one_le_of_lt #[decl.toExpr]])
  -- A factor's literal bounds that omega derives from the context, where no
  -- hypothesis states one: `i < n` and `n < 1000` bound `i` by `1000`.
  let mut literals : Array Int := #[]
  for decl in ← getLCtx do
    if decl.isImplementationDetail then continue
    let type ← instantiateMVars decl.type
    if (type.isAppOfArity ``LE.le 4 || type.isAppOfArity ``LT.lt 4) &&
        (type.getArg! 0).isConstOf ``Int then
      if let some c := (type.getArg! 3).int? then
        if 0 < c && !literals.contains c then literals := literals.push c
  literals := literals.qsort (· < ·)
  let mut bounded : Array Lean.Expr := #[]
  for (left, right) in products do
    for factor in #[left, right] do
      if factor.int?.isSome || bounded.contains factor then continue
      bounded := bounded.push factor
      if let some proof ← omegaProof? goal (← mkAppM ``LE.le #[mkIntLit 0, factor]) then
        bounds := bounds.push (← mkAppM ``HSub.hSub #[factor, mkIntLit 0],
          ← mkAppM ``Int.sub_nonneg_of_le #[proof])
      for c in literals do
        if let some proof ← omegaProof? goal (← mkAppM ``LE.le #[factor, mkIntLit c]) then
          bounds := bounds.push (← mkAppM ``HSub.hSub #[mkIntLit c, factor],
            ← mkAppM ``Int.sub_nonneg_of_le #[proof])
          break
  let boundsOf (factor : Lean.Expr) := bounds.filter fun (difference, _) =>
    readsLinearly factor difference
  let mut goal := goal
  let mut seen : Array (Lean.Expr × Lean.Expr) := #[]
  for (left, right) in products do
    if seen.contains (left, right) then continue
    seen := seen.push (left, right)
    for (_, leftProof) in boundsOf left do
      for (_, rightProof) in boundsOf right do
        let proof ← mkAppM ``Int.mul_nonneg #[leftProof, rightProof]
        let (_, next) ← (← goal.assert `product (← inferType proof) proof).intro1P
        goal := next
  return goal

/-- The applications in a term whose function is a constant outside `skip`. -/
private partial def applicationAtoms (skip : List Name) (found : Array Lean.Expr)
    (sub : Lean.Expr) : Array Lean.Expr :=
  match sub with
  | .mdata _ inner => applicationAtoms skip found inner
  | .proj _ _ inner => applicationAtoms skip found inner
  | .app .. =>
    let found := sub.getAppArgs.foldl (applicationAtoms skip) found
    match sub.getAppFn.constName? with
    | some name => if skip.contains name then found else found.push sub
    | none => found
  | _ => found

/-- The instance arguments a term applies its functions at that instance
resolution does not find, by their classes: a frame a leaf runs at, such as
`Skolems.runtime unit functions nonempty`, is not a declared instance. -/
private partial def unresolvedInstances (e : Lean.Expr)
    (found : Array (Lean.Expr × Lean.Expr)) : MetaM (Array (Lean.Expr × Lean.Expr)) := do
  match e with
  | .mdata _ inner | .proj _ _ inner => unresolvedInstances inner found
  | .app .. =>
    let fn := e.getAppFn
    let arguments := e.getAppArgs
    let mut found ← arguments.foldlM (fun acc a => unresolvedInstances a acc) found
    unless fn.isConst do return found
    let info ← getFunInfoNArgs fn arguments.size
    for i in [0:min info.paramInfo.size arguments.size] do
      unless info.paramInfo[i]!.binderInfo.isInstImplicit do continue
      let argument := arguments[i]!
      if argument.hasLooseBVars || argument.hasMVar || argument.isFVar then continue
      let type ← instantiateMVars (← inferType argument)
      if type.hasLooseBVars || found.any (·.1 == type) then continue
      unless (← isClass? type).isSome do continue
      let resolved ← (try synthInstance? type catch _ => pure none)
      if let some resolved := resolved then
        if ← withReducible (isDefEq resolved argument) then continue
      found := found.push (type, argument)
    return found
  | _ => return found

/-- A leaf's goal with each instance argument instance resolution does not
find (`unresolvedInstances`) bound as a local instance: `grind` canonicalizes
an instance argument by resolving its class, and one it cannot resolve splits
equal terms over it into separate classes. -/
private def installLeafInstances (goal : MVarId) : MetaM MVarId := goal.withContext do
  let mut found := ← unresolvedInstances (← instantiateMVars (← goal.getType)) #[]
  for decl in ← getLCtx do
    if decl.isImplementationDetail then continue
    if ← isProp decl.type then
      found ← unresolvedInstances (← instantiateMVars decl.type) found
  let mut goal := goal
  for (type, value) in found do
    let (_, next) ← (← goal.define `verexInstance type value).intro1P
    goal := next
  return goal

/-- `grind` over a leaf's own facts, within an attempt's budget. The callees'
contracts, which the call rule has consumed, and the continuations bound in
the context are cleared first; `grind` would instantiate the contracts at
every term they match, and so are the facts not connected to the goal
(`connectedFacts`), unless it is `False` and states no variable. Rank tables
and function values are abstracted to
variables, which is all `grind` reads of them. An equation between arrays is taken element by
element (`arraysByElements`), and reads at equal positions are spelled alike
(`identifyReads`). Vectors and certified integers compare as themselves,
not by their arrays and values, the saturation's form of a goal about them
(`SpecVector.eq_iff_values`, `SpecInt.eq_iff_val`): a value an uninterpreted
function reads is equal by congruence. -/
elab "verex_grind" : tactic => focusGoal do
  let goal ← getMainGoal
  let victims ← goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    let mut found := #[]
    let mut propositions := #[]
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let type ← instantiateMVars decl.type
      if decl.isLet || type.getUsedConstants.contains ``VerexIR.Proofs.Satisfies then
        found := found.push decl.fvarId
      else if ← isProp type then
        propositions := propositions.push (decl.fvarId, (collectFVars {} type).fvarSet)
    if target.isConstOf ``False then
      if verex.solverProfile.get (← getOptions) then recordStep "grind refutation" false 0
      return found
    let connected := connectedFacts propositions (collectFVars {} target).fvarSet
    for (fvarId, _) in propositions do
      unless connected.contains fvarId do found := found.push fvarId
    return found
  let mut goal := goal
  for fvarId in victims.reverse do
    goal ← goal.tryClear fvarId
  let abstracted ← abstractRuntimeFrames (← abstractFunctionValues (← abstractRankTables goal))
  -- `grind` proves the goal handed to it, never an assigned one, which
  -- `getMainGoal` would skip.
  if ← abstracted.isAssigned then throwError "internal: the abstracted leaf is assigned"
  let some elements ← arraysByElements abstracted
    | replaceMainGoal []
  setGoals [elements]
  evalTactic (← `(tactic|
    try simp only [← VerexIR.SpecVector.eq_iff_values, ← VerexIR.SpecInt.eq_iff_val] at *))
  let some elements := (← getGoals).head? | return
  replaceMainGoal [← assertProductBounds (← identifyReads elements)]
  replaceMainGoal [← installLeafInstances (← getMainGoal)]
  evalTactic (← `(tactic| verex_budgeted grind))

/-- A quotient of integers by a divisor that is not a literal, as `(dividend,
divisor)`: `omega` reads one as an atom, unlike a quotient by a literal. -/
private def variableQuotient? (e : Lean.Expr) : Option (Lean.Expr × Lean.Expr) :=
  if e.isAppOfArity ``HDiv.hDiv 6 && (e.getArg! 0).isConstOf ``Int &&
      ((e.getArg! 5).int? <|> (e.getArg! 5).nat?.map Int.ofNat).isNone then
    some (e.getArg! 4, e.getArg! 5)
  else none

/-- A comparison with a quotient by a positive divisor that is not a literal
becomes one with a product, as authored proofs take it: `s / d ≤ b` from
`s ≤ b * d` (`Int.ediv_le_of_le_mul`), `s / d < b` from `s < b * d`, and
`a ≤ s / d` from `a * d ≤ s`, for `0 < d` by omega. A side a hypothesis
equates with such a quotient is rewritten to it first. The comparison of
products is then decided by omega over its atoms, or, its products
distributed over sums so that they share their monomials, by `verex_grind`
with the monomials' bounds. -/
elab "verex_quotient" : tactic => focusGoal do
  let goal ← getMainGoal
  let isInt (e : Lean.Expr) := (e.getArg! 0).isConstOf ``Int
  let comparison (target : Lean.Expr) : Option (Name × Lean.Expr × Lean.Expr) :=
    if target.isAppOfArity ``LE.le 4 && isInt target then
      some (``LE.le, target.getArg! 2, target.getArg! 3)
    else if target.isAppOfArity ``LT.lt 4 && isInt target then
      some (``LT.lt, target.getArg! 2, target.getArg! 3)
    else none
  -- A hypothesis `side = q` (or `q = side`) with `q` a quotient by a variable.
  let definition? (side : Lean.Expr) : MetaM (Option Lean.Expr) := goal.withContext do
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let type ← instantiateMVars decl.type
      unless type.isAppOfArity ``Eq 3 && (type.getArg! 0).isConstOf ``Int do continue
      if type.getArg! 1 == side && (variableQuotient? (type.getArg! 2)).isSome then
        return some decl.toExpr
      if type.getArg! 2 == side && (variableQuotient? (type.getArg! 1)).isSome then
        return some (← mkEqSymm decl.toExpr)
    return none
  let some (relation, lhs, rhs) := comparison (← instantiateMVars (← goal.getType))
    | throwError "the goal compares no integers"
  let mut goal := goal
  -- The side to reduce: a quotient, or what a hypothesis equates with one.
  for side in #[lhs, rhs] do
    if (variableQuotient? side).isNone then
      if let some equation ← definition? side then
        let target ← instantiateMVars (← goal.getType)
        let result ← goal.withContext (goal.rewrite target equation)
        goal ← goal.replaceTargetEq result.eNew result.eqProof
  let some (relation, lhs, rhs) := comparison (← instantiateMVars (← goal.getType))
    | throwError "the goal compares no integers"
  let (lemma, args, divisor) ←
    if let some (dividend, divisor) := variableQuotient? lhs then
      let lemma := if relation == ``LE.le then ``Int.ediv_le_of_le_mul else ``Int.ediv_lt_of_lt_mul
      pure (lemma, #[dividend, rhs, divisor], divisor)
    else if let some (dividend, divisor) := variableQuotient? rhs then
      unless relation == ``LE.le do throwError "no reduction of `<` below a quotient"
      pure (``Int.le_ediv_of_mul_le, #[lhs, dividend, divisor], divisor)
    else throwError "no quotient by a variable"
  let some positive ← omegaProof? goal (← mkAppM ``LT.lt #[mkIntLit 0, divisor])
    | throwError "the divisor is not positive"
  let premises ← goal.apply (mkAppN (mkConst lemma) (args.push positive))
  setGoals premises
  evalTactic (← `(tactic| all_goals first
    | omega
    | (verex_bounds; omega)
    | (simp only [Int.mul_add, Int.add_mul, Int.mul_sub, Int.sub_mul]; verex_grind)
    | verex_grind))

/-- Introduce the binders and premises a leaf's goal states, through the
`Obligation` markers heading them: a quantified clause is decided at an
arbitrary instance, whose premises the deciders then use as hypotheses.
Fails when there is nothing to introduce. A leaf takes this only after its
deciders failed on the goal as stated, since `decide` needs a closed goal
and every other leaf keeps its path. -/
elab "verex_intro" : tactic => focusGoal do
  if (← getGoals).isEmpty then return
  let (goal, introduced) ← introduceThroughMarkers (← getMainGoal)
  unless introduced do throwError "the goal states no binder or premise"
  replaceMainGoal [goal]

/-- Split the conjunctions a goal states, through the `Obligation` markers
heading them, into one goal per conjunct: a clause quantified in each
conjunct is then introduced and decided on its own. Fails when the goal
states no conjunction. -/
private partial def splitConjunctions (keep : Bool) (goal : MVarId) : MetaM (List MVarId) := do
  let target ← goal.withContext (instantiateMVars (← goal.getType))
  let body := if target.isAppOfArity ``VerexIR.Proofs.Obligation 4 && !keep
    then target.getArg! 3 else target
  unless body.isAppOfArity ``And 2 do return [goal]
  let goal ← if body != target then goal.replaceTargetDefEq body else pure goal
  let conjuncts ← goal.apply (mkConst ``And.intro)
  conjuncts.foldlM (fun goals conjunct => do return goals ++ (← splitConjunctions keep conjunct)) []

elab "verex_split_conjunction_goal" : tactic => focusGoal do
  if (← getGoals).isEmpty then return
  let goals ← splitConjunctions (verex.keepObligationMarkers.get (← getOptions)) (← getMainGoal)
  if goals.length ≤ 1 then throwError "the goal states no conjunction"
  replaceMainGoal goals

/-- The alternatives a successful path refutes: `¬(A ∨ B)` as `¬A` and `¬B`,
through clause markers; the goal itself where it negates no disjunction. -/
partial def negatedAlternatives (goal : MVarId) : MetaM (List MVarId) :=
  goal.withContext do
    let target := (← instantiateMVars (← goal.getType)).consumeMData
    let some inner := target.not? | return [goal]
    let inner := inner.consumeMData
    let inner := if inner.isAppOfArity ``VerexIR.Proofs.Obligation 4 then inner.getArg! 3 else inner
    let some (left, right) := inner.app2? ``Or | return [goal]
    let goal ← goal.replaceTargetDefEq (mkNot (mkOr left right))
    let parts ← goal.apply (mkApp2 (mkConst ``not_or_intro) left right)
    parts.foldlM (fun goals part => do return goals ++ (← negatedAlternatives part)) []

/-- Split a negated disjunction goal into the negations of its alternatives.
Fails when the goal negates no disjunction. -/
elab "verex_split_negated_goal" : tactic => focusGoal do
  if (← getGoals).isEmpty then return
  let goals ← negatedAlternatives (← getMainGoal)
  if goals.length ≤ 1 then throwError "the goal negates no disjunction"
  replaceMainGoal goals

/-- `verex_intro` where the goal may state nothing to introduce. -/
elab "verex_intro_any" : tactic => focusGoal do
  if (← getGoals).isEmpty then return
  let (goal, _) ← introduceThroughMarkers (← getMainGoal)
  replaceMainGoal [goal]

/-- Introduce the binders and premises of the goal and, where it is a
negation, the negated statement as a hypothesis toward `False`: the
deciders that refute a context by instantiation take the goal this way. -/
elab "verex_intro_negation" : tactic => focusGoal do
  if (← getGoals).isEmpty then return
  let (goal, _) ← introduceThroughMarkers (← getMainGoal)
  let target ← goal.withContext (instantiateMVars (← goal.getType))
  if target.isAppOfArity ``Not 1 then
    let (_, next) ← (← goal.replaceTargetDefEq
      (.forallE `negated (target.getArg! 0) (mkConst ``False) .default)).intro1P
    replaceMainGoal [next]
  else replaceMainGoal [goal]

/-- The propositions of a goal's context with their variables, but the
unit's facts the theorem binds (`TargetState.moduleFacts`): they share the
unit's variables with every leaf fact, and connect nothing of the leaf. -/
def leafPropositions (goal : MVarId) : MetaM (Array (Lean.Expr × FVarIdSet)) := goal.withContext do
  let moduleFacts ← (← targetState).moduleFacts.get
  (← goal.getNondepPropHyps).filterMapM fun fvarId => do
    if moduleFacts.contains fvarId then return none
    let type ← instantiateMVars (← fvarId.getType)
    return some (type, (collectFVars {} type).fvarSet)

/-- The propositions of the context connected to a type through shared
variables, transitively. -/
private def connectedPropositions (propositions : Array (Lean.Expr × FVarIdSet)) (type : Lean.Expr) :
    Array Lean.Expr :=
  connectedFacts propositions (collectFVars {} type).fvarSet

/-- A proof of an arithmetic premise by omega from the hypotheses connected
to it through shared variables, within a bounded allowance, as the
normalization's discharger decides its side conditions; none when omega
fails, the allowance ends, or the same question failed before under the
same connected facts. -/
private def connectedOmega? (goal : MVarId) (premise : Lean.Expr) : TacticM (Option Lean.Expr) :=
  goal.withContext do
    let propositions ← leafPropositions goal
    let connected := connectedPropositions propositions premise
    let key := (premise, connected)
    if (← (← targetState).undecidedPremises.get).contains key then return none
    let facts ← (← getLCtx).foldlM (init := #[]) fun found decl => do
      if decl.isImplementationDetail then return found
      let type ← instantiateMVars decl.type
      return if connected.contains type then found.push decl.toExpr else found
    let enclosing ← readThe Core.Context
    let now ← IO.getNumHeartbeats
    let remaining := if enclosing.maxHeartbeats == 0 then sideDecisionHeartbeats
      else enclosing.initHeartbeats + enclosing.maxHeartbeats - now
    let budget := max 1 (min remaining sideDecisionHeartbeats)
    let saved ← saveState
    let proof ← mkFreshExprMVar premise
    let result ← tryCatchRuntimeEx
      (withTheReader Core.Context
        (fun context => { context with initHeartbeats := now, maxHeartbeats := budget }) do
        let some refutation ← proof.mvarId!.falseOrByContra | return some (← instantiateMVars proof)
        refutation.withContext do
          let negated ← (← getLCtx).foldlM (init := #[]) fun found decl => do
            if decl.isImplementationDetail || facts.contains decl.toExpr then return found
            if ← isProp decl.type then return found.push decl.toExpr
            return found
          omegaHidden (facts ++ negated).toList refutation
        return some (← instantiateMVars proof))
      fun failure => do
        if failure.isMaxHeartbeat then return none
        throw failure
    if result.isNone then
      saved.restore
      (← targetState).undecidedPremises.modify (·.insert key)
    return result

/-- Whether an equation has a parameter of the derived summary being stated
on a side (`isSummaryParameter`). -/
def fixesSummaryParameter (equation : Lean.Expr) : MetaM Bool := do
  let some (_, lhs, rhs) ← matchEq? equation | return false
  for side in [lhs, rhs] do
    if let .fvar x := side then
      if ← isSummaryParameter x then return true
  return false

/-- `Lean.Meta.subst` on an equation `lhs = rhs`, except that a side which
only reduces to a local is restated in place: Lean's `subst` asserts the
reduced equation and clears the original, which fails when a hypothesis
depends on it, while `substCore` reverts such dependents. -/
private def substReducedEquation (goal : MVarId) (equation : FVarId) (lhs rhs : Lean.Expr) :
    MetaM MVarId := goal.withContext do
  let rhs' ← whnf rhs
  if rhs'.isFVar then
    let goal ← if rhs == rhs' then pure goal
      else goal.replaceLocalDeclDefEq equation (← mkEq lhs rhs')
    return (← substCore goal equation (symm := true) (tryToSkip := true)).2
  let lhs' ← whnf lhs
  unless lhs'.isFVar do
    throwError "the equation does not equate a local: {← equation.getType}"
  let goal ← if lhs == lhs' then pure goal
    else goal.replaceLocalDeclDefEq equation (← mkEq lhs' rhs)
  return (← substCore goal equation (symm := false) (tryToSkip := true)).2

/-- Substitute an equation of the context as `subst` does, except that a
parameter of the derived summary being stated stays (`isSummaryParameter`):
the other side goes where it is a local, and the substitution throws where
it is not. A failure leaves the state as it was: `substCore` reverts the
goal before it can fail. -/
def substEquation (goal : MVarId) (equation : FVarId) : MetaM MVarId := goal.withContext do
  let saved ← saveState
  try
    let some (_, lhs, rhs) ← matchEq? (← instantiateMVars (← equation.getType))
      | Lean.Meta.subst goal equation
    let keep (x : Lean.Expr) : MetaM Bool := do
      let .fvar x := x | return false
      isSummaryParameter x
    match ← keep lhs, ← keep rhs with
    | false, false => substReducedEquation goal equation lhs rhs
    | true, true => throwError "the equation relates two parameters of a summary"
    | keepLeft, _ =>
        unless (if keepLeft then rhs else lhs).isFVar do
          throwError "the equation fixes a parameter of a summary"
        -- `symm`: the right side goes.
        return (← substCore goal equation (symm := keepLeft) (tryToSkip := true)).2
  catch error =>
    saved.restore
    throw error

/-- `substEquation`, except that an equation it cannot substitute because
it fixes a summary's parameter stays: none, the state being restored by
`substEquation`. Any other failure is thrown. -/
def substEquation? (goal : MVarId) (equation : FVarId) : MetaM (Option MVarId) := do
  try some <$> substEquation goal equation
  catch error =>
    if ← goal.withContext do fixesSummaryParameter (← instantiateMVars (← equation.getType)) then
      return none
    throw error

/-- Consume the hypotheses a call leaves behind before a computation goal
is normalized again: an implication whose premise is decided by omega is
specialized, existentials and conjunctions are destructured, and the
equations on state components and on witnesses rewrite the goal. -/
elab "verex_consume" : tactic => focusGoal do
  if (← getGoals).isEmpty then return
  let mut goal ← getMainGoal
  -- Specialize decided implications, each once: an implication something
  -- else depends on stays in the context beside its specialization.
  let mut progress := true
  let mut applied : Array FVarId := #[]
  while progress do
    progress := false
    let candidate ← goal.withContext do
      (← getLCtx).findDeclM? fun decl => do
        if decl.isImplementationDetail || applied.contains decl.fvarId then return none
        let ty ← instantiateMVars decl.type
        unless ty.isArrow do return none
        let premise := ty.bindingDomain!
        if premise.hasLooseBVars then return none
        unless ← isProp premise do return none
        if premise.getUsedConstants.contains ``VerexIR.Proofs.wp then return none
        return some (decl.fvarId, premise, ty.bindingBody!)
    if let some (fvarId, premise, conclusion) := candidate then
      let saved ← saveState
      try
        let premiseProof ← goal.withContext do
          let syntactic ← (← getLCtx).findDeclM? fun decl => do
            if decl.isImplementationDetail then return none
            return if (← instantiateMVars decl.type) == premise then some decl.toExpr else none
          match syntactic with
          | some proof => pure proof
          | none =>
              let some proof ← connectedOmega? goal premise
                | throwError "the premise is not decided"
              pure proof
        let specialized ← goal.withContext do
          instantiateMVars (mkApp (mkFVar fvarId) premiseProof)
        -- A clause specialized keeps the clause's name.
        let name ← goal.withContext do
          let user := (← fvarId.getDecl).userName
          pure (if user.isAnonymous || user.hasMacroScopes then `consumed else user)
        let asserted ← goal.assert name conclusion specialized
        let (_, next) ← asserted.intro1P
        goal ← next.tryClear fvarId
        applied := applied.push fvarId
        progress := true
      catch _ =>
        restoreState saved
        -- Leave it; a later leaf may still use it.
        pure ()
      if !progress then break
  -- Destructure existentials and conjunctions.
  progress := true
  while progress do
    progress := false
    let compound ← goal.withContext do
      (← getLCtx).findDeclM? fun decl => do
        if decl.isImplementationDetail then return none
        let ty ← instantiateMVars decl.type
        return if ty.isAppOfArity ``And 2 || ty.isAppOfArity ``Exists 2 then some decl.fvarId else none
    if let some fvarId := compound then
      match ← goal.cases fvarId with
      | #[subgoal] =>
          goal := subgoal.mvarId
          progress := true
      | _ => break
  -- Substitute witness equations, of locals and of memory alike. One
  -- fixing a summary's parameter stays (`substEquation`) and the search
  -- goes on past it; any other failure ends the substitution.
  progress := true
  let mut kept : Array FVarId := #[]
  while progress do
    progress := false
    let witness ← goal.withContext do
      (← getLCtx).findDeclM? fun decl => do
        if decl.isImplementationDetail || kept.contains decl.fvarId then return none
        let ty ← instantiateMVars decl.type
        unless ty.isAppOfArity ``Eq 3 do return none
        if (ty.getArg! 2).isFVar then return some (decl.fvarId, ty)
        if (ty.getArg! 1).isFVar then return some (decl.fvarId, ty)
        return none
    if let some (fvarId, ty) := witness then
      try
        goal ← substEquation goal fvarId
        progress := true
      catch _ =>
        if ← fixesSummaryParameter ty then
          kept := kept.push fvarId
          progress := true
  setGoals [goal]

/-- The constructor a term applies, when it is a full application of a
constructor with fields. -/
private def fieldConstructor? (e : Lean.Expr) : MetaM (Option Name) := do
  let .const name _ := e.getAppFn | return none
  match (← getEnv).find? name with
  | some (.ctorInfo info) =>
      return if info.numFields > 0 && e.getAppNumArgs == info.numParams + info.numFields then
        some name else none
  | _ => return none

/-- A loop invariant's elimination over a slot it asserts defined: the
slot and the invariant at its value. -/
private def slotElimination? (ty : Lean.Expr) : MetaM (Option (FVarId × Lean.Expr)) := do
  if ty.isAppOfArity ``Option.elim 5 && (ty.getArg! 3).isConstOf ``False then
    -- The slot is a projection of the row until the row is destructured.
    match ← whnfR (ty.getArg! 2) with
    | .fvar slot => return some (slot, ty.getArg! 4)
    | _ => return none
  else return none

/-- Each hypothesis `¬a = b` also as `¬b = a`, unless the context states
it: a rewrite with a hypothesis matches its equation only as stated. -/
private def symmetricDisequalities (goal : MVarId) : MetaM MVarId := do
  let facts ← goal.withContext do
    let mut stated : Array Lean.Expr := #[]
    for decl in ← getLCtx do
      unless decl.isImplementationDetail do stated := stated.push (← instantiateMVars decl.type)
    let mut facts : Array (Lean.Expr × Lean.Expr) := #[]
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let some proposition := (← instantiateMVars decl.type).not? | continue
      let some (_, lhs, rhs) := proposition.eq? | continue
      if lhs == rhs then continue
      let symmetric := mkNot (← mkEq rhs lhs)
      if stated.contains symmetric then continue
      stated := stated.push symmetric
      facts := facts.push (symmetric, ← mkAppM ``Ne.symm #[decl.toExpr])
    pure facts
  let mut goal := goal
  for (type, proof) in facts do
    goal ← goal.withContext do
      let (_, next) ← (← goal.assert `symmetric type proof).intro1P
      pure next
  return goal

/-- A goal with a local replaced by a value it is definitionally equal to,
in its target and hypotheses, and cleared. -/
private def substituteDefEq (goal : MVarId) (replaced : FVarId) (value : Lean.Expr) : MetaM MVarId := do
  let mut goal := goal
  let dependents ← goal.withContext do
    (← getLCtx).foldlM (init := #[]) fun found decl => do
      if decl.fvarId == replaced then return found
      return if (← instantiateMVars decl.type).containsFVar replaced then found.push decl.fvarId else found
  for dependent in dependents do
    goal ← goal.withContext do
      let type ← instantiateMVars (← dependent.getType)
      goal.replaceLocalDeclDefEq dependent (type.replaceFVar (.fvar replaced) value)
  goal ← goal.withContext do
    goal.replaceTargetDefEq ((← instantiateMVars (← goal.getType)).replaceFVar (.fvar replaced) value)
  goal.tryClear replaced

/-- The value a new local named `name` shadows reads as `old(x)` in the
source: the visible data local of that name, other than `except`, is renamed
`old_name`, where that name is free. -/
private def renameShadowed (goal : MVarId) (name : Name) (except : FVarId) (binding := true) :
    MetaM MVarId := do
  let shadowed ← goal.withContext do
    (← getLCtx).findDeclM? fun other => do
      if other.isImplementationDetail || other.fvarId == except || other.userName != name then
        return none
      return if ← isProp other.type then none else some other.fvarId
  let old := Name.mkSimple s!"old_{name}"
  if ← goal.withContext (nameTaken old) then return goal
  match shadowed with
  | some other => goal.withContext (renameScoped goal other old)
  -- A new binding of the name displaces the latest entry of an earlier one
  -- whose local is gone; a value's part taking its name is no new binding.
  | none => if binding then renameLatestEntry goal name.toString (some except) old else pure goal

/-- An `old_x` whose `x` is gone, substituted by it, is the source's `x`
again: the value did not change. -/
private def restoreShadowed (goal : MVarId) : MetaM MVarId := do
  let mut goal := goal
  let olds ← goal.withContext do
    (← getLCtx).foldlM (init := #[]) fun found decl => do
      if decl.isImplementationDetail then return found
      let .str .anonymous s := decl.userName | return found
      unless s.startsWith "old_" do return found
      let base := Name.mkSimple (s.drop 4).toString
      if ((← getLCtx).findFromUserName? base).isSome then return found
      return found.push (decl.fvarId, base)
  for (fvarId, base) in olds do
    goal ← goal.withContext (renameScoped goal fvarId base)
  return goal

/-- The names the components of a row-valued local take, from the local's
source name `x`: an intrinsic map takes `x` itself, as the Move source
names the whole value, as does the one component of a one-field row; any
other component is `x_k` by position, and the row's tail carries the
position on. A visible local the name shadows is the value the source
reads as `old(x)`, renamed `old_x`. Where the local has no source name,
`cases` names the components. -/
private def componentNames (goal : MVarId) (fvarId : FVarId) :
    MetaM (MVarId × Array AltVarNames) := do
  let decl ← fvarId.getDecl
  let user := decl.userName
  if user.isAnonymous || user.hasMacroScopes || user.isInaccessibleUserName then return (goal, #[])
  let (base, index) := match user with
    | .num (.str base "tail") k => (base, k)
    | _ => (user, 0)
  let type ← instantiateMVars decl.type
  -- A reference's carrier is its value and its final value, also where the
  -- type is spelled through a result shape.
  let mut unfolded := type
  for _ in [:8] do
    if unfolded.isAppOfArity ``VerexIR.Proofs.Denote.NTy.carrier 2 &&
        (unfolded.getArg! 1).isAppOfArity ``VerexIR.Proofs.Denote.NTy.ref 1 then
      return (goal, #[{ varNames := [base, Name.mkSimple s!"{base}_final"] }])
    let some next ← unfoldDefinition? unfolded | break
    unfolded ← whnfCore next
  -- A struct's or a variant's carrier is its row under the data invariants
  -- (`NTy.carrier`): the row keeps the local's name, and the certificate is
  -- a proposition no proof reads by name.
  if (← whnfD type).isAppOfArity ``Subtype 2 then
    return (goal, #[{ varNames := [base.appendIndexAfter index,
      Name.mkSimple s!"{base.appendIndexAfter index}_invariant"] }])
  let .app (.app (.const ``Prod _) head) tail ← whnfD type | return (goal, #[])
  let head ← instantiateMVars head
  let isMap := (head.isAppOfArity ``VerexIR.Proofs.Denote.NTy.carrier 2 &&
      (head.getArg! 1).isAppOfArity ``VerexIR.Proofs.Denote.NTy.table 2) ||
    (← whnfD head).isAppOfArity ``VerexIR.SpecEntries 1
  let tailUnit := (← whnfD tail).isConstOf ``PUnit
  let headName := if isMap || (index == 0 && tailUnit) then base else base.appendIndexAfter index
  let tailName := Name.mkNum (Name.str base "tail") (index + 1)
  let goal ← renameShadowed goal headName fvarId (binding := false)
  return (goal, #[{ varNames := [headName, tailName] }])

/-- The carrier a typed map reads: `SpecMap.ofEntries key value x` is the map
of the table carrier `x`. -/
def typedMap? (e : Lean.Expr) : Option FVarId := do
  guard (e.isAppOfArity ``VerexIR.SpecMap.ofEntries 7)
  let .fvar x := e.getArg! 6 | none
  some x

/-- The source owner of a table field observation. This only identifies a
forward reading or naming anchor; a projected table is never reconstructed
from its model map. -/
partial def tableViewOwner? (value : Lean.Expr) : Option FVarId :=
  let value := value.consumeMData
  match value with
  | .fvar owner => some owner
  | .proj source _ parent =>
      if source == ``Prod || source == ``Subtype then tableViewOwner? parent else none
  | _ =>
      if value.isAppOfArity ``Prod.fst 3 || value.isAppOfArity ``Prod.snd 3 ||
          value.isAppOfArity ``Subtype.val 3 then tableViewOwner? (value.getArg! 2)
      else none

/-- The carrier a map value reads. -/
def sourceMap? (e : Lean.Expr) : Option FVarId :=
  typedMap? e

/-- A hypothesis stating a marked clause, under premises: the clause's range
and the statement with the marker unwrapped. -/
private partial def markedClause? (ty : Lean.Expr) : Option (ObligationRange × Lean.Expr) :=
  if ty.isAppOfArity ``VerexIR.Proofs.Obligation 4 then
    (obligationRange? ty).map fun range => (range, ty.getArg! 3)
  else if ty.isArrow then
    (markedClause? ty.bindingBody!).map fun (range, body) =>
      (range, ty.updateForallE! ty.bindingDomain! body)
  else none

/-- The conclusion of an implication, under its premises. -/
private partial def conclusionOf (e : Lean.Expr) : Lean.Expr :=
  if e.isArrow then conclusionOf e.bindingBody! else e

/-- The conjuncts of a conjunction, left to right. -/
private partial def conjunctParts (e : Lean.Expr) (acc : Array Lean.Expr) : Array Lean.Expr :=
  if e.isAppOfArity ``And 2 then conjunctParts (e.getArg! 1) (conjunctParts (e.getArg! 0) acc)
  else acc.push e

/-- Record the names of the conjuncts a loop step's invariant hypothesis
assumes, in order, before its markers are unwrapped: a marked conjunct,
under its premises or under the slots it reads (`Option.elim`), by its
clause's name; any other conjunct unnamed. -/
private partial def assumedClauseNames (normalization : Simp.Context × Simp.SimprocsArray)
    (names : Array (ObligationRange × String)) (ty : Lean.Expr) (found : Array (Option String)) :
    MetaM (Array (Option String)) := do
  if ty.isAppOfArity ``And 2 then
    assumedClauseNames normalization names (ty.getArg! 1)
      (← assumedClauseNames normalization names (ty.getArg! 0) found)
  else if ty.isAppOfArity ``Option.elim 5 && (ty.getArg! 3).isConstOf ``False then
    -- The slot's value is a local while its clauses are read.
    let inner ← lambdaTelescope (ty.getArg! 4) fun _ body =>
      assumedClauseNames normalization names body found
    (← targetState).assumedSlotSizes.modify (·.push (found.size, inner.size - found.size))
    return inner
  else match markedClause? ty with
    | some (range, unwrapped) =>
        -- Unwrapped, the clause is normalized, and a conjunction it then
        -- concludes is split into parts: one name per part.
        let name? := (names.find? (·.1 == range)).map (·.2)
        let (ctx, simprocs) := normalization
        let (result, _) ← Simp.main unwrapped ctx (methods := Simp.mkDefaultMethodsCore simprocs)
        let parts := conjunctParts (conclusionOf result.expr) #[]
        if parts.size ≤ 1 then return found.push name?
        return (List.range parts.size).foldl (init := found) fun found i =>
          found.push (name?.map (s!"{·}_{i + 1}"))
    | none => return found.push none

def recordAssumedClauses (normalization : Simp.Context × Simp.SimprocsArray) (goal : MVarId)
    (hypothesis : FVarId) : MetaM Unit := goal.withContext do
  (← targetState).assumedSlotSizes.set #[]
  let names ← assumedClauseNames normalization (← (← targetState).clauseRanges.get)
    (← instantiateMVars (← hypothesis.getType)) #[]
  if verex.solverDebug.get (← getOptions) then
    IO.println s!"    assumed clauses of {(← hypothesis.getDecl).userName}: {names}"
  (← targetState).assumedClauses.set names

/-- The local a bound speaks of: `x`, `x.val`, or the size of the map `x`. -/
private def boundedLocal? (e : Lean.Expr) : Option (FVarId × Bool) :=
  match e with
  | .fvar x => some (x, false)
  | .proj _ 0 (.fvar x) => some (x, false)
  | _ =>
    if e.isAppOfArity ``VerexIR.SpecInt.val 3 then
      match e.getArg! 2 with | .fvar x => some (x, false) | _ => none
    else if e.isAppOfArity ``VerexIR.SpecMap.size 3 then (typedMap? (e.getArg! 2)).map (·, true)
    else none

/-- Name the facts the context states of a source-named local, where they
have no name: its lower and upper bounds (`i_lower`, `i_upper`, and
`m_size_lower` for a map's size), and a map's validity (`m_valid`). -/
private def nameFacts (goal : MVarId) : MetaM MVarId := do
  let mut goal := goal
  let decls ← goal.withContext do
    (← getLCtx).foldlM (init := #[]) fun found decl => do
      if decl.isImplementationDetail || !decl.userName.hasMacroScopes then return found
      let ty ← instantiateMVars decl.type
      unless ← isProp ty do return found
      return found.push (decl.fvarId, ty)
  for (fvarId, ty) in decls do
    -- A bound against a literal names the local it bounds; an equation
    -- names the local it defines.
    let mut statement := ty
    while statement.isArrow do statement := statement.bindingBody!
    let suggestion? : Option (FVarId × String) :=
      if statement.isAppOfArity ``LE.le 4 || statement.isAppOfArity ``LT.lt 4 then
        let (lower, upper) := (statement.getArg! 2, statement.getArg! 3)
        match boundedLocal? lower, boundedLocal? upper with
        | some (x, size), _ =>
            if upper.hasFVar then none else some (x, if size then "size_upper" else "upper")
        | _, some (x, size) =>
            if lower.hasFVar then none else some (x, if size then "size_lower" else "lower")
        | none, none => none
      else if statement.isAppOfArity ``VerexIR.SpecMap.Valid 4 then
        (typedMap? (statement.getArg! 3)).map (·, "valid")
      else if let some (_, lhs, rhs) := statement.eq? then
        match sourceMap? lhs, boundedLocal? lhs with
        | some x, _ => some (x, "eq")
        | none, some (x, false) => some (x, "eq")
        | none, _ => let _ := rhs; none
      else none
    let some (x, suffix) := suggestion? | continue
    let base ← goal.withContext do pure (← x.getDecl).userName
    if base.isAnonymous || base.hasMacroScopes then continue
    let name := Name.mkSimple s!"{base}_{suffix}"
    let taken ← goal.withContext do return ((← getLCtx).findFromUserName? name).isSome
    if taken then continue
    goal ← goal.withContext (goal.rename fvarId name)
  return goal

/-- Name the maps the context writes: a map equated to a model operation on
a map the source names (`m' = update m k v`, under the premises the
equation has) is that map's next value, `m'`, where it has no source name
of its own, as a callee's contract names the value it writes back. -/
private def nameWrittenMaps (goal : MVarId) : MetaM MVarId := do
  let mut goal := goal
  let mut again := true
  while again do
    again := false
    let toRename ← goal.withContext do
      (← getLCtx).findDeclM? fun decl => do
        if decl.isImplementationDetail then return none
        let mut ty ← instantiateMVars decl.type
        while ty.isArrow do ty := ty.bindingBody!
        let some (_, lhs, rhs) := ty.eq? | return none
        let some written := sourceMap? lhs | return none
        let writtenDecl ← written.getDecl
        let name := writtenDecl.userName
        unless name == `value || name.hasMacroScopes do return none
        -- The map the operation reads: the first encoded map among the
        -- operation's arguments.
        guard rhs.isApp
        let some source := rhs.getAppArgs.findSome? sourceMap? | return none
        let sourceName := (← source.getDecl).userName
        if sourceName.isAnonymous || sourceName.hasMacroScopes then return none
        let mut next := Name.mkSimple s!"{sourceName}'"
        while ((← getLCtx).findFromUserName? next).isSome do
          next := Name.mkSimple s!"{next}'"
        return some (written, next)
    if let some (written, next) := toRename then
      goal ← goal.withContext (goal.rename written next)
      again := true
  return goal

/-- The value of a decided proposition: `True`, `False`, or an equation of
closed operands (its type may name the frame). -/
private def decidedSide? (side : Lean.Expr) : MetaM (Option Bool) := do
  if side.isConstOf ``True then return some true
  if side.isConstOf ``False then return some false
  let_expr Eq _ left right := side | return none
  unless frameFreeDecision left && frameFreeDecision right do return none
  try
    let decision ← whnfD (← mkDecide side)
    if decision.isConstOf ``Bool.true then return some true
    if decision.isConstOf ``Bool.false then return some false
    return none
  catch _ => return none

/-- An equivalence with a decided side (`decidedSide?`) and what it states of
the other: `p` where the side holds, `¬p` where it fails; under a clause's
marker, marked so. -/
private partial def decidedEquivalence? (hypothesis type : Lean.Expr) :
    MetaM (Option (Lean.Expr × Lean.Expr)) := do
  if type.isAppOfArity ``VerexIR.Proofs.Obligation 4 then
    let some (statement, proof) ← decidedEquivalence?
        (← mkExpectedTypeHint hypothesis type.appArg!) type.appArg! | return none
    return some (mkApp type.appFn! statement, proof)
  let_expr Iff left right := type | return none
  let held (side : Lean.Expr) : MetaM Lean.Expr := do
    if side.isConstOf ``True then return mkConst ``True.intro
    mkDecideProof side
  let refuted (side : Lean.Expr) : MetaM Lean.Expr := do
    if side.isConstOf ``False then
      return (← withLocalDeclD `falsity side (fun falsity => mkLambdaFVars #[falsity] falsity))
    mkDecideProof (mkNot side)
  if let some value ← decidedSide? left then
    if value then return some (right, ← mkAppM ``Iff.mp #[hypothesis, ← held left])
    return some (mkNot right, ← withLocalDeclD `held right fun other => do
      mkLambdaFVars #[other] (mkApp (← refuted left) (← mkAppM ``Iff.mpr #[hypothesis, other])))
  if let some value ← decidedSide? right then
    if value then return some (left, ← mkAppM ``Iff.mpr #[hypothesis, ← held right])
    return some (mkNot left, ← withLocalDeclD `held left fun other => do
      mkLambdaFVars #[other] (mkApp (← refuted right) (← mkAppM ``Iff.mp #[hypothesis, other])))
  return none

/-- Each equivalence with a decided side replaced by what it states of the
other (`decidedEquivalence?`). -/
def decideEquivalences (goal : MVarId) : MetaM MVarId := do
  let mut goal := goal
  -- Each once: a hypothesis something else depends on stays beside what
  -- replaces it, with its identity, since it precedes the replacement.
  let mut replaced : Array FVarId := #[]
  -- Replacing a hypothesis re-creates the ones after it: scan the new context.
  repeat
    let decided ← goal.withContext do
      (← getLCtx).findDeclM? fun decl => do
        if decl.isImplementationDetail || replaced.contains decl.fvarId then return none
        let type ← instantiateMVars decl.type
        let stated := if type.isAppOfArity ``VerexIR.Proofs.Obligation 4 then type.appArg! else type
        unless stated.isAppOfArity ``Iff 2 do return none
        return (← decidedEquivalence? decl.toExpr type).map fun (statement, proof) =>
          (decl.fvarId, statement, proof)
    let some (hypothesis, statement, proof) := decided | break
    goal := (← goal.replace hypothesis proof (some statement)).mvarId
    replaced := replaced.push hypothesis
  return goal

/-- A fresh name for an earlier value or fact that a later one of the same
name displaces (`old_g_result`, then `old_g_result_1`, …): the latest keeps
the ordinary name. -/
def oldUserName (name : Name) : MetaM Name := do
  let base := Name.mkSimple s!"old_{name}"
  let mut old := base
  let mut index := 1
  while ← nameTaken old do
    old := Name.mkSimple s!"{base}_{index}"
    index := index + 1
  return old

/-- A nominal row's reframing observes its structural operation, while
its surrounding encodings keep their original nominal owner. The theorem
and the restated theorem type must convert at this construction boundary. -/
def nominalPlainReframeEquation (equation type : Lean.Expr) : MetaM Lean.Expr := do
  unless isNominalType (← getEnv) type do return equation
  let view ← nativeTypeView type
  unless view.isAppOfArity ``NTy.struct 3 || view.isAppOfArity ``NTy.enum 5 do return equation
  let statement ← instantiateMVars (← inferType equation)
  let canonical := statement.replace fun term =>
    if term.isAppOfArity ``Skolems.plainReframe 9 && term.getArg! 6 == type then
      some (mkAppN term.getAppFn (term.getAppArgs.set! 6 view))
    else none
  if canonical == statement then return equation
  unless ← withNewMCtxDepth (withDefault (isDefEq statement canonical)) do return equation
  return ← mkExpectedTypeHint equation canonical

/-- A hypothesis stating a declaration's data invariant of an encoding at
the leaf's frame, unsealed: the row moved to the frame the declaration's
type arguments induce (`Skolems.plainEncode_plainReframe`), the premises
derived as the owed direction derives them (`rowInhabitable?`), and the
statement read as the declaration's clauses of the row
(`OwnAtArguments.plainEncode`).  None where the hypothesis states no
declaration's invariant at a leaf frame, or a premise is not decided. -/
private def unsealedOwn? (type hypothesis : Lean.Expr) : TacticM (Option Lean.Expr) := do
  let args := type.getAppArgs
  unless args.size == 7 do return none
  let value := args[6]!.consumeMData
  unless value.isAppOfArity ``VerexIR.Proofs.Denote.NTy.plainEncode 3 do return none
  let carriers := (value.getArg! 0).consumeMData
  unless carriers.isAppOfArity ``VerexIR.Proofs.Denote.Skolems.toCarriers 3 do return none
  let frame := (carriers.getArg! 2).consumeMData
  let σ := (value.getArg! 1).consumeMData
  let view ← nativeTypeView σ
  unless view.isAppOfArity ``VerexIR.Proofs.Denote.NTy.struct 3 ||
      view.isAppOfArity ``VerexIR.Proofs.Denote.NTy.enum 5 do return none
  withTransparency .default do
  let params := (view.getArg! 1).consumeMData
  let some own ← decidedFrameFree? (← mkEq
      (mkApp2 (mkConst ``VerexIR.Proofs.Denote.NRow.isParamsFrom) (mkNatLit 0) params)
      (mkConst ``Bool.true)) | return none
  let some nonempty ← synthInstance? (mkApp (mkConst ``VerexIR.Proofs.Denote.CarriersNonempty)
      (mkApp2 (mkConst ``VerexIR.Proofs.Denote.Carriers.runtime)
        (carriers.getArg! 0) (carriers.getArg! 1))) | return none
  let reframing := mkAppN (mkConst ``VerexIR.Proofs.Denote.Skolems.plainEncode_plainReframe)
    #[carriers.getArg! 0, carriers.getArg! 1, nonempty, frame, params, own, σ]
  let some bounded ← decidedFrameFree? (← whnf (← inferType reframing)).bindingDomain!
    | return none
  let equation ← nominalPlainReframeEquation (mkApp2 reframing bounded (value.getArg! 2)) σ
  let some (_, reframed, _) := (← instantiateMVars (← inferType equation)).eq?
    | return none
  let motive ← withLocalDeclD `value (← inferType args[6]!) fun v =>
    mkLambdaFVars #[v] (mkAppN type.getAppFn (args.set! 6 v))
  let moved ← mkEqMP (← mkCongrArg motive (← mkEqSymm equation)) hypothesis
  let some inhabitable ← rowInhabitable? args[5]! | return none
  let equivalence := mkAppN (mkConst ``VerexIR.Proofs.Denote.OwnAtArguments.plainEncode)
    (args.extract 0 6 ++ #[inhabitable, reframed.getArg! 2])
  let unsealed ← mkAppM ``Iff.mp #[equivalence, moved]
  return some (← instantiateMVars unsealed)


/-- Read the common decoded-row predicate of a nominal invariant. Concrete
frames quantify inhabitability before the row; given declarations select the
frame at their producer. Both then use the same codec inverse law. -/
private def decodedOwnPredicate? (own ownStated : Lean.Expr) :
    TacticM (Option (Lean.Expr × Lean.Expr)) := do
  let mut statement := ownStated
  let premise (e : Lean.Expr) : MetaM (Option Lean.Expr) := do
    let .forallE _ domain _ _ := e | return none
    let_expr Eq _ lhs _ := ← whnfR domain | return none
    let_expr NRow.inhabitable arguments := lhs | return none
    return some arguments
  let mut arguments? ← premise statement
  for _ in [0:8] do
    if arguments?.isSome then break
    let next ← (try withDefault (whnf statement) catch _ => pure statement)
    if next == statement then break
    statement := next
    arguments? ← premise statement
  -- Both interpretations expose the same predicate of a decoded plain row.
  -- The concrete frame first quantifies the arguments' inhabitability; a
  -- given declaration has already selected that frame at its producer.
  let selected? ← match arguments? with
    | some arguments => do
        let some inhabitable ← rowInhabitable? arguments | return none
        let applied ← mkAppM' (← mkExpectedTypeHint own statement) #[inhabitable]
        pure (some (applied, ← whnfD (← inferType applied)))
    | none => pure (some (own, statement))
  let some (own, rowStatement) := selected? | return none
  unless rowStatement.isForall do return none
  return some (own, rowStatement)

/-- Decode a declaration's own fact at the value it encodes. Both local
ownership facts and compound certified reads use this producer, including
its transport through the unit's nominal argument frame. -/
private def decodedOwnFact? (type proof : Lean.Expr) :
    TacticM (Option (Lean.Expr × Lean.Expr)) := do
  unless type.isAppOfArity ``VerexIR.Proofs.Denote.OwnAtArguments 7 ||
      type.isAppOfArity ``Carriers.own 4 do return none
  let row := type.appArg!.consumeMData
  unless row.isAppOfArity ``VerexIR.Proofs.Denote.NTy.plainEncode 3 do return none
  let family := row.getArg! 0
  let σ := row.getArg! 1
  let value := row.getArg! 2
  -- The arguments of a type that has a value are inhabitable: decided at a
  -- ground instantiation, and from the frame's law where the frame
  -- resolves them, which is how a read of a stored value at a generic
  -- frame reaches its own certificate (`rowInhabitable?`).
  let codec := mkApp2 (mkConst ``VerexIR.Proofs.Denote.NTy.plainCodec) family σ
  let applied ← (try
      let some (predicate, statement) ← decodedOwnPredicate? proof type | return none
      let decoded ← mkAppM ``VerexIR.Proofs.Codec.decode_encode #[codec, value]
      let applied ← mkAppM' (← mkExpectedTypeHint predicate statement) #[value, decoded]
      let statement ← inferType applied
      pure (some (applied, statement))
    catch _ => pure none)
  let applied ← match applied with
    | some applied => pure (some applied)
    | none => do
        let some proof ← (try unsealedOwn? type proof catch _ => pure none)
          | return none
        unless ← isTypeCorrect proof do return none
        pure (some (proof, ← inferType proof))
  let some (applied, statement) := applied | return none
  return some (applied, statement)

/-- Observe a selected nominal own clause once, where a producer makes its
proof available. Only the selected declaration predicate is opened; native
logical atoms and the nominal descriptor remain in their canonical form. -/
def selectedOwnClauses? (canonicalize : Lean.Expr → MetaM Lean.Expr)
    (type proof : Lean.Expr) : TacticM (Option (Lean.Expr × Lean.Expr)) := do
  let clauses? ← if type.consumeMData.getAppFn.constName?.any
      (fun name => (name.toString.splitOn "dataOwn.case").length != 1) then
      pure (some proof)
    else pure ((← decodedOwnFact? type proof).map Prod.fst)
  let some clauses := clauses? | return none
  let statement ← canonicalize
    (← whnfR (← deltaExpand (← instantiateMVars (← inferType clauses))
      (fun name => (name.toString.splitOn "dataOwn.case").length != 1)))
  return some (statement, ← mkExpectedTypeHint clauses statement)

/-- A fact stating a declaration's data invariant of an encoding
(`OwnAtArguments`), as the declaration's clauses of the value encoded: a
predicate of an encoding is a statement about every value with it, so the
fact is not destructured — it is applied, to the row the encoding is of, the
inhabitability of its arguments, and the decoding that `Codec.decode_encode`
gives.  The goal with the fact replaced, none where the fact is not of that
shape or its arguments are not seen to be inhabitable
(`doc/designs/data-invariants.md`). -/
def ownAtArgumentsFact? (goal : MVarId) (fact : FVarId) : TacticM (Option MVarId) :=
  goal.withContext do
    let type ← instantiateMVars (← fact.getType)
    let some (applied, statement) ← decodedOwnFact? type (.fvar fact) | return none
    -- Read the clause wrapper while preserving its native logical atoms.
    -- Default transparency would unfold, for example, integer ≤ into NonNeg.
    let statement ← whnfR statement
    let encoded := type.appArg!.consumeMData
    let family := encoded.getArg! 0
    -- Given declarations introduce a fresh clause in their producer's plain
    -- family. Normalize that clause before splitting it into hypotheses; the
    -- surrounding context was normalized before this clause existed. Concrete
    -- own predicates already use their argument frame and need no second pass.
    unless family.consumeMData.isAppOfArity ``GivenFrame.toCarriers 1 do
      return some (← goal.replace fact applied (some statement)).mvarId
    let savedGoals ← getGoals
    setGoals [goal]
    let { ctx, simprocs, .. } ← try
      mkSimpContext
        (← `(tactic| simp only [vir_denote, vir_denote_norm, vir_denote_eval, Prod.fst, Prod.snd]))
        (eraseLocal := false)
      finally setGoals savedGoals
    let (normalized, _) ← Simp.main statement ctx
      (methods := Simp.mkDefaultMethodsCore simprocs)
    let applied ← mkEqMP (← normalized.getProof) applied
    pure (some (← goal.replace fact applied (some normalized.expr)).mvarId)

/-- The readings a `withSome` takes of an option that `e` does not spell:
the application, its option, its continuation and its empty value.  A
reading of a constructor is already a normal form (`withSome_some`,
`withSome_none`); one of a symbolic value is not, and nothing but the
equation that the option holds a value reduces it. -/
private def withSomeReadings (e : Lean.Expr) : MetaM (Array (Array Lean.Expr)) := do
  let found ← IO.mkRef (#[] : Array (Array Lean.Expr))
  e.forEach fun sub => do
    unless sub.isAppOfArity ``withSome 5 do return
    if sub.hasLooseBVars then return
    let arguments := sub.getAppArgs
    let option_ := arguments[2]!
    if option_.isAppOf ``Option.some || option_.isAppOf ``Option.none then return
    found.modify fun known => if known.any (· == arguments) then known else known.push arguments
  found.get

/-- A reading a `withSome` takes of an option the context decides but does
not spell, taken at the value the option holds: the option splits, the
empty case closed by what the context then states of it, and in the case
that remains every reading taken at the value (`withSome_of_some`) and the
option's other readers substituted by the equation the case states.  The
goal that remains, `none` where the empty case does not close, since a
split the context does not decide is the path's to take and not a leaf's.
Taken this way rather than by generalizing the option, whose readers are
under the dependent binder a continuation takes and so follow no
generalization. -/
private def readingsAtValue? (observe : MVarId → Lean.Expr → MetaM MVarId)
    (goal : MVarId) (readings : Array (Array Lean.Expr)) :
    TacticM (Option MVarId) := do
  let option_ := readings[0]![2]!
  -- The readings of an option the case states a value for: the constructor
  -- equations of the readers a normal form leaves of the option
  -- (`Option.map`, `Option.getD`), so that a reader states the value.
  let readers : SimpTheorems ← [``Option.some.injEq, ``Option.map_some, ``Option.getD_some,
      ``Option.map_none, ``Option.getD_none].foldlM (·.addConst ·) {}
  let rewritten (goal : MVarId) (equations : Array Lean.Expr) (kept : Array FVarId) :
      TacticM (Option MVarId) :=
    goal.withContext do
      let mut rules : SimpTheoremsArray := #[readers]
      for equation in equations do rules ← rules.addTheorem (.other `withSome) equation
      let ctx ← Simp.mkContext { failIfUnchanged := false } rules (← getSimpCongrTheorems)
      -- The equation the case states is not rewritten by itself.
      let targets := (← goal.getNondepPropHyps).filter (!kept.contains ·)
      let (result, _) ← simpGoal goal ctx #[] none (fvarIdsToSimp := targets)
      return result.map (·.2)
  let saved ← saveState
  let split ← observing? do
    let decided ← goal.withContext (mkAppM ``Option.eq_none_or_eq_some #[option_])
    let asserted ← goal.assert `optionRead (← goal.withContext (inferType decided)) decided
    let (read, asserted) ← asserted.intro1P
    let branches ← asserted.cases read
    let #[empty, held] := branches | failure
    -- The empty case: every reading is the empty value, and the context's
    -- own statement of the option refutes it.
    let emptyGoal ← empty.mvarId.withContext do
      let equations := readings.map fun arguments =>
        mkAppN (mkConst ``withSome_of_none)
          #[arguments[0]!, arguments[1]!, option_, empty.fields[0]!, arguments[3]!, arguments[4]!]
      rewritten empty.mvarId equations #[empty.fields[0]!.fvarId!]
    if let some emptyGoal := emptyGoal then
      let emptyGoal ← observe emptyGoal empty.fields[0]!
      -- Whether the context refutes the empty case is a side decision: the
      -- split is this leaf's to take only where the context already decides
      -- the option, and a refutation the context states costs a few hundred
      -- thousand heartbeats. Unbounded, the blanket refuter searches the
      -- case's whole context, reducing the value type of each hypothesis it
      -- compares, which in a certified context is the target's whole budget
      -- and more.
      let enclosing ← readThe Core.Context
      let now ← IO.getNumHeartbeats
      -- What the target has left, where it has a budget at all.
      let left? := if enclosing.maxHeartbeats == 0 then none
        else some (enclosing.initHeartbeats + enclosing.maxHeartbeats - now)
      let allowance := max 1 (min (left?.getD sideDecisionHeartbeats) sideDecisionHeartbeats)
      let refuted ← tryCatchRuntimeEx
        (withTheReader Core.Context
          (fun context => { context with initHeartbeats := now, maxHeartbeats := allowance }) do
          let remaining ← Lean.Elab.Tactic.run emptyGoal
            (evalTactic (← `(tactic| contradiction)))
          pure remaining.isEmpty)
        fun failed => do
          -- The side allowance running out declines the decision; the
          -- target's own budget running out stays the target's failure, which
          -- the decision must not swallow.
          let targetSpent := match left? with
            | some left => left ≤ sideDecisionHeartbeats
            | none => false
          if failed.isMaxHeartbeat && targetSpent then throw failed
          pure false
      unless refuted do failure
    -- The case the option holds a value: the readings taken at it.
    let held ← held.mvarId.cases held.fields[0]!.fvarId!
    let #[value] := held | failure
    let valueGoal ← value.mvarId.withContext do
      let equation := value.fields[1]!
      let equations := (readings.map fun arguments =>
        mkAppN (mkConst ``withSome_of_some)
          #[arguments[0]!, arguments[1]!, option_, value.fields[0]!, equation,
            arguments[3]!, arguments[4]!]).push equation
      let next ← rewritten value.mvarId equations #[equation.fvarId!]
      match next with
      | some next => return some (← observe next equation)
      | none => return none
    pure valueGoal
  match split with
  | some next => return next
  | none => saved.restore; return none

/-- Cases replace a row with its constructor, so newly introduced equations
must state its fields directly. Compute only literal constructor projections
in new or changed equations; named definitions and unchanged facts stay closed. -/
def canonicalCaseEquations (previous : LocalContext) (goal : MVarId) : MetaM MVarId := do
  let reduce (e : Lean.Expr) : MetaM Lean.Expr :=
    Core.transform e (post := fun e => do
      match ← reduceProjection? e with
      | some reduced => return .visit reduced
      | none =>
          if let some canonical ← canonicalValueProjection? e then return .visit canonical
          return .continue)
  let equation (type : Lean.Expr) : MetaM Lean.Expr := do
    let some (_, left, right) := type.eq? | return type
    let left' ← reduce left
    let right' ← reduce right
    if left' == left && right' == right then return type
    return mkAppN type.getAppFn #[type.getArg! 0, left', right']
  let mut next := goal
  for decl in ← goal.withContext getLCtx do
    if decl.isImplementationDetail || decl.isLet then continue
    if previous.find? decl.fvarId |>.any (fun old => old.type == decl.type) then continue
    let type ← next.withContext (equation decl.type)
    if type != decl.type then next ← next.replaceLocalDeclDefEq decl.fvarId type
  let target ← next.getType
  let target' ← next.withContext (equation target)
  if target' != target then next ← next.replaceTargetDefEq target'
  return next

/-- A declaration's data invariant a certificate states of every value it
observes, as of a vector's elements (`∀ i, … → ∀ x, v.values[i]? = some x →
OwnAtArguments … x`), instantiated at an observation the context states:
the binders by the observation, which the premise naming the last binder
is unified with, and the other premises by the context.  A map entry's
membership is observed by `rawEntryMemberships`.  Each result is an
unquantified data invariant of an encoding, which `ownAtArgumentsFact?`
applies.  The pairs of a quantified fact and an observation already tried
are `tried`, by their statements, which splitting the context does not
change; the result lists the new pairs, with each proof where one was
found. -/
def observedOwnInstances (tried : Array (Lean.Expr × Lean.Expr)) :
    MetaM (Array ((Lean.Expr × Lean.Expr) × Option Lean.Expr)) := do
  let context ← getLCtx
  let mut found := #[]
  for invariant in context do
    if invariant.isImplementationDetail then continue
    let type ← instantiateMVars invariant.type
    unless type.isForall &&
        type.getForallBody.isAppOfArity ``VerexIR.Proofs.Denote.OwnAtArguments 7 do continue
    for observation in context do
      if observation.isImplementationDetail || observation.fvarId == invariant.fvarId then continue
      let observed ← instantiateMVars observation.type
      unless observed.isAppOfArity ``Eq 3 do continue
      if tried.contains (type, observed) then continue
      let saved ← saveState
      let proof? ← (try
          let (arguments, _, body) ← forallMetaTelescope type
          unless body.isAppOfArity ``VerexIR.Proofs.Denote.OwnAtArguments 7 do
            throwError "not a data invariant"
          let mut props := #[]
          for argument in arguments do
            props := props.push (← isProp (← inferType argument))
          let some last := (List.range arguments.size).reverse.find? (fun i => !props[i]!)
            | throwError "no binder"
          let lastMVar := arguments[last]!.mvarId!
          -- The premise that names the last binder is the observation's.
          let mut premise? := none
          for index in [last + 1:arguments.size] do
            if props[index]! then
              let premiseType ← instantiateMVars (← inferType arguments[index]!)
              if (premiseType.find? (· == .mvar lastMVar)).isSome then
                premise? := some index
                break
          let some premise := premise? | throwError "no observing premise"
          let premiseType ← instantiateMVars (← inferType arguments[premise]!)
          unless premiseType.getAppFn == observed.getAppFn &&
              premiseType.getAppNumArgs == observed.getAppNumArgs do
            throwError "another observation"
          unless ← isDefEq premiseType observed do throwError "another observation"
          arguments[premise]!.mvarId!.assign observation.toExpr
          for index in [0:arguments.size] do
            let mvarId := arguments[index]!.mvarId!
            if ← mvarId.isAssigned then continue
            unless props[index]! do throwError "a binder the observation leaves open"
            let wanted ← instantiateMVars (← mvarId.getType)
            if wanted.hasExprMVar then throwError "a premise the observation leaves open"
            let some held ← context.findDeclM? fun decl => do
                if decl.isImplementationDetail then return none
                if ← withReducible (isDefEq decl.type wanted) then return some decl.toExpr
                return none
              | throwError "a premise the context does not state"
            mvarId.assign held
          let proof ← instantiateMVars (mkAppN invariant.toExpr arguments)
          if proof.hasExprMVar then throwError "an open instance"
          unless ← isTypeCorrect proof do throwError "an ill-typed instance"
          pure (some proof)
        catch _ => pure none)
      saved.restore
      found := found.push ((type, observed), proof?)
  return found

/-- Split every conjunctive or existential hypothesis into its parts,
every row-valued variable into its components, every slot an invariant
asserts defined into its value, every equivalence with a constant side
into what it states of the other, and every reading of an option the
context decides but does not spell into the reading at its value, so that
a leaf is over scalars. -/
def splitHypotheses (observe : MVarId → Lean.Expr → MetaM MVarId) : TacticM Unit := focusGoal do
  if (← getGoals).isEmpty then return
  let mut goal ← getMainGoal
  -- An existential eliminates into a proposition only; a data goal keeps it.
  let propositional ← goal.withContext do isProp (← goal.getType)
  -- The locals of a one-value type already replaced by their value.
  let mut settled : Array FVarId := #[]
  -- The options whose readings were taken at their value (`readingsAtValue?`),
  -- or found undecided there.
  let mut read : Array Lean.Expr := #[]
  -- The quantified data invariants instantiated at an observation, per
  -- pair (`observedOwnInstances`).
  let mut instantiated : Array (Lean.Expr × Lean.Expr) := #[]
  -- Splitting one hypothesis restarts the scan. A large call context can
  -- otherwise reduce every earlier, unchanged declaration for every field.
  -- Recheck declarations whose type or diagnostic name changes, and never
  -- cache an expression with unresolved metavariables.
  let inspected ← IO.mkRef ({} : Std.HashMap FVarId (Name × Lean.Expr))
  let mut progress := true
  while progress do
    progress := false
    let conjunction ← goal.withContext do
      (← getLCtx).findDeclM? fun decl => do
        if decl.isImplementationDetail || settled.contains decl.fvarId then return none
        let ty ← instantiateMVars decl.type
        if (← inspected.get).get? decl.fvarId == some (decl.userName, ty) then return none
        if ty.isAppOfArity ``And 2 || (propositional && ty.isAppOfArity ``Exists 2) then
          return some decl.fvarId
        -- A data invariant of an encoding: applied, not destructured
        -- (`ownAtArgumentsFact?`).
        if ty.isAppOf ``VerexIR.Proofs.Denote.OwnAtArguments then return some decl.fvarId
        let stated := if ty.isAppOfArity ``VerexIR.Proofs.Obligation 4 then ty.appArg! else ty
        if stated.isAppOfArity ``Iff 2 then
          if ← ((Option.isSome <$> decidedSide? (stated.getArg! 0)) <||>
              (Option.isSome <$> decidedSide? (stated.getArg! 1))) then
            return some decl.fvarId
        if let some (range, _) := markedClause? ty then
          -- A clause the registry names, not yet named so.
          if let some (_, name) := (← (← targetState).clauseRanges.get).find? (·.1 == range) then
            if decl.userName != Name.mkSimple name then return some decl.fvarId
        -- The invariant's last conjunct, carrying its position.
        if let .num (.str `loopInvariant "rest") _ := decl.userName then
          if !(ty.isAppOfArity ``Option.elim 5) && !(ty.isAppOfArity ``And 2) then
            return some decl.fvarId
        -- A native row splits into its components; a runtime pair, such as
        -- a failure's payload, stays one value.
        let carrier ← whnfD ty
        let certificate := ty.isAppOf ``NTy.deepHolds ||
          ty.isAppOf ``HListPlain.deepHolds || ty.isAppOf ``variantPlain.deepHolds
        let own := ty.isAppOf ``Carriers.own || ty.isAppOf ``FunctionCarriers.own
        if own ||
            (certificate && carrier.isAppOfArity ``And 2) ||
            (carrier.isAppOfArity ``Prod 2 && (runtimeValueMentions carrier).isEmpty) ||
            carrier.isAppOfArity ``PUnit 0 then
          return some decl.fvarId
        if (← slotElimination? ty).isSome then return some decl.fvarId
        -- Two constructor applications equated, whatever type the equation
        -- is stated at: a pair splits into its components, a variant into
        -- its payload, and different variants refute the context.
        if ty.isAppOfArity ``Eq 3 then
          if (← fieldConstructor? (ty.getArg! 1)).isSome then
            if (← fieldConstructor? (ty.getArg! 2)).isSome then return some decl.fvarId
        unless ty.hasExprMVar do
          inspected.modify (·.insert decl.fvarId (decl.userName, ty))
        return none
    if let some fvarId := conjunction then
      -- The parts of a named conjunction are numbered by its name; a
      -- bound's parts take the names their statements give them.
      let partNames ← goal.withContext do
        let decl ← fvarId.getDecl
        let ty ← instantiateMVars decl.type
        let user := decl.userName
        if !ty.isAppOfArity ``And 2 then return (#[] : Array AltVarNames)
        if user.eraseMacroScopes == pathDecisionName then
          return #[{ varNames := [← mkFreshUserName pathDecisionName,
            ← mkFreshUserName pathDecisionName] }]
        -- The loop invariant's conjuncts take their clauses' names, in order
        -- (the hypothesis is introduced hygienically).
        let invariantPart? := match user.eraseMacroScopes with
          | `loopInvariant => some 0
          | .num (.str `loopInvariant "rest") k => some k
          | _ => none
        if invariantPart?.isNone && (user.isAnonymous || user.hasMacroScopes || user == `bounds) then
          return #[]
        if let some k := invariantPart? then
          let names ← (← targetState).assumedClauses.get
          -- A conjunct reading slots holds several clauses: the rest continues
          -- after them, and the conjunct itself carries the position on.
          let left := ty.getArg! 0
          let slot := left.isAppOfArity ``Option.elim 5 && (left.getArg! 3).isConstOf ``False
          let sizes ← (← targetState).assumedSlotSizes.get
          let leftCount := if slot then (sizes.find? (·.1 == k)).map (·.2) |>.getD 1 else 1
          let rest := Name.mkNum (Name.str `loopInvariant "rest") (k + leftCount)
          let left ← if leftCount != 1 then pure (Name.mkNum (Name.str `loopInvariant "rest") k)
            else match names[k]? with
              | some (some name) => pure (Name.mkSimple name)
              | _ => mkFreshUserName `left
          return #[{ varNames := [left, rest] }]
        return #[{ varNames := [user.appendIndexAfter 1, user.appendIndexAfter 2] }]
      let isConstructorEquation ← goal.withContext do
        let ty ← instantiateMVars (← fvarId.getType)
        if ty.isAppOfArity ``Eq 3 then return (← fieldConstructor? (ty.getArg! 1)).isSome
        return false
      let decided ← goal.withContext do
        decidedEquivalence? (.fvar fvarId) (← instantiateMVars (← fvarId.getType))
      let unit? ← goal.withContext do
        match ← whnfD (← instantiateMVars (← fvarId.getType)) with
        | .const ``PUnit [level] => pure (some (mkConst ``PUnit.unit [level]))
        | _ => pure none
      let clauseName? ← goal.withContext do
        let decl ← fvarId.getDecl
        let ty ← instantiateMVars decl.type
        if ty.isAppOfArity ``And 2 || ty.isAppOfArity ``Option.elim 5 then return none
        if let some (range, _) := markedClause? ty then
          if let some (_, name) := (← (← targetState).clauseRanges.get).find? (·.1 == range) then
            return some (Name.mkSimple name)
        if let .num (.str `loopInvariant "rest") k := decl.userName then
          return some (← match (← (← targetState).assumedClauses.get)[k]? with
            | some (some name) => pure (Name.mkSimple name)
            | _ => mkFreshUserName `frame)
        return none
      if let some (statement, proof) := decided then
        goal := (← goal.replace fvarId proof (some statement)).mvarId
        progress := true
      else if let some name := clauseName? then
        -- A clause assumed takes the clause's source name; a marker stays, as
        -- the clause is normal already and the marker says so. The clause
        -- of an earlier call of the same callee keeps its fact under an old
        -- name, as the earlier call's result does; the same statement again,
        -- a copy a proof made, keeps its own name.
        let (previous?, statement) ← goal.withContext do
          let previous? := ((← getLCtx).findFromUserName? name).filter (·.fvarId != fvarId)
          return (previous?, ← instantiateMVars (← fvarId.getType))
        match previous? with
        | some previous =>
            if (← instantiateMVars previous.type) == statement then
              inspected.modify (·.insert fvarId (← goal.withContext fvarId.getUserName, statement))
            else
              let old ← goal.withContext (oldUserName name)
              goal ← goal.withContext (goal.rename previous.fvarId old)
              inspected.modify (·.insert previous.fvarId (old, ← instantiateMVars previous.type))
              goal ← goal.withContext (goal.rename fvarId name)
        | none => goal ← goal.withContext (goal.rename fvarId name)
        progress := true
      else if let some unit := unit? then
        -- A row's tail of no fields is its one value, by definition.
        goal ← substituteDefEq goal fvarId unit
        settled := settled.push fvarId
        progress := true
      else if isConstructorEquation then
        match ← Lean.Meta.injection goal fvarId with
        | Lean.Meta.InjectionResult.subgoal subgoal _ _ =>
            -- An equation another declaration depends on is not cleared:
            -- its components are stated, so it is settled.
            if (← subgoal.withContext getLCtx).contains fvarId then
              settled := settled.push fvarId
            goal := subgoal
            progress := true
        | Lean.Meta.InjectionResult.solved =>
            replaceMainGoal []
            return
      else if let some (slot, _) ← goal.withContext do
          slotElimination? (← instantiateMVars (← fvarId.getType)) then
        -- A slot a loop invariant asserts defined: the empty case refutes
        -- the context, and the defined case reads the invariant at the
        -- value, by definition.
        let mut defined := none
        for subgoal in ← goal.cases slot do
          let hypothesis := subgoal.subst.get fvarId
          match subgoal.ctorName with
          | some ``Option.none =>
              subgoal.mvarId.withContext do
                subgoal.mvarId.assign (← mkFalseElim (← subgoal.mvarId.getType) hypothesis)
          | some ``Option.some =>
              defined := some (← subgoal.mvarId.withContext do
                let ty ← instantiateMVars (← inferType hypothesis)
                let value := (← whnfR (ty.getArg! 2)).appArg!
                let read := (ty.getArg! 4).beta #[value]
                -- The value takes the name the invariant binds the slot by;
                -- the value it shadows is the source's `old`.
                let name := (ty.getArg! 4).bindingName!
                let shadowed ← renameShadowed subgoal.mvarId name value.fvarId!
                let named ← renameScoped shadowed value.fvarId! name
                let named ← bindScopeEntries named #[value.fvarId!]
                named.replaceLocalDeclDefEq hypothesis.fvarId! read)
          | _ => throwError "a slot has a case besides its definedness"
        let some next := defined | throwError "a slot has no defined case"
        goal := next
        progress := true
      else if ← goal.withContext do
          let ty ← instantiateMVars (← fvarId.getType)
          pure (ty.isAppOf ``Carriers.own || ty.isAppOf ``FunctionCarriers.own) then
        -- Splitting a nominal certificate exposes its own predicate after
        -- the context's normalization pass. Read that predicate now, before
        -- the native leaf is inspected for residual runtime encodings.
        let next ← goal.withContext do
          if let some read ← ownAtArgumentsFact? goal fvarId then return some read
          let type ← instantiateMVars (← fvarId.getType)
          let savedGoals ← getGoals
          setGoals [goal]
          let { ctx, simprocs, .. } ← mkSimpContext
            (← `(tactic| simp only [vir_denote_norm])) (eraseLocal := false)
          setGoals savedGoals
          let (result, _) ← Simp.main type ctx
            (methods := Simp.mkDefaultMethodsCore simprocs)
          if result.expr == type then return none
          let proof ← mkEqMP (← result.getProof) (.fvar fvarId)
          return some (← goal.replace fvarId proof (some result.expr)).mvarId
        match next with
        | some next =>
            goal := next
            -- The old certificate may remain in dependent proof terms. Its
            -- native reading is now the visible premise; keep the encoding
            -- only as internal evidence for those terms.
            if (← goal.withContext getLCtx).contains fvarId then
              goal.modifyLCtx (·.setKind fvarId .implDetail)
            settled := settled.push fvarId
        | none => settled := settled.push fvarId
        progress := true
      else if ← goal.withContext do
          pure ((← instantiateMVars (← fvarId.getType)).isAppOf
            ``VerexIR.Proofs.Denote.OwnAtArguments) then
        -- A data invariant of an encoding is applied, never destructured
        -- (`ownAtArgumentsFact?`); one whose arguments are not seen to be
        -- inhabitable stays as it is.
        match ← goal.withContext (ownAtArgumentsFact? goal fvarId) with
        | some applied =>
            goal := applied
            if (← goal.withContext getLCtx).contains fvarId then
              goal.modifyLCtx (·.setKind fvarId .implDetail)
            settled := settled.push fvarId
        | none => settled := settled.push fvarId
        progress := true
      else
      let expanded ← goal.withContext do whnfD (← fvarId.getType)
      if expanded.isAppOfArity ``And 2 || expanded.isConstOf ``True then
        goal ← goal.replaceLocalDeclDefEq fvarId expanded
      let (goal', names) ← goal.withContext (componentNames goal fvarId)
      goal := goal'
      let previous ← goal.withContext getLCtx
      let subgoals ← goal.cases fvarId (if partNames.isEmpty then names else partNames)
      match subgoals with
      | #[subgoal] =>
          goal ← canonicalCaseEquations previous subgoal.mvarId
          -- Case elimination produces fresh component facts, including a
          -- certified vector lookup exposed from an existential/conjunction.
          for field in subgoal.fields do
            if field.isFVar && (← goal.withContext getLCtx).contains field.fvarId! then
              goal ← observe goal field
          progress := true
      | _ => break
    else
      -- An array equated through its list is substituted as the array of
      -- that list.
      let listEquation ← goal.withContext do
        (← getLCtx).findDeclM? fun decl => do
          if decl.isImplementationDetail then return none
          let ty ← instantiateMVars decl.type
          unless ty.isAppOfArity ``Eq 3 do return none
          let lhs := ty.getArg! 1
          let rhs := ty.getArg! 2
          let asList (side other : Lean.Expr) (symm : Bool) : Option (Lean.LocalDecl × Bool) :=
            if side.isAppOfArity ``Array.toList 2 then
              match side.getArg! 1 with
              | .fvar xs => if other.containsFVar xs then none else some (decl, symm)
              | _ => none
            else none
          return asList lhs rhs false <|> asList rhs lhs true
      match listEquation with
      | some (decl, symm) =>
          let (equation, asserted) ← goal.withContext do
            let h ← if symm then mkEqSymm decl.toExpr else pure decl.toExpr
            let proof ← mkAppM ``VerexIR.Proofs.Denote.array_eq_of_toList_eq #[h]
            let asserted ← goal.assert `arrayEq (← inferType proof) proof
            asserted.intro1P
          goal ← asserted.withContext (Lean.Meta.subst asserted equation)
          progress := true
      | none =>
        -- A data invariant quantified over the values a certificate
        -- observes, instantiated at each equation the context states
        -- (`observedOwnInstances`); the instance is applied as an
        -- unquantified one is.
        let instances ← goal.withContext (observedOwnInstances instantiated)
        instantiated := instantiated ++ instances.map (·.1)
        let proofs := instances.filterMap (·.2)
        let mut fresh := false
        for proof in proofs do
          let observedInstance ← goal.withContext (inferType proof)
          -- An instance is stated once: it stays in the context as internal
          -- evidence beside its clauses, so that a later split of the
          -- context, which starts this list afresh, does not state it again.
          let stated ← goal.withContext do
            (← getLCtx).anyM fun decl => withReducible (isDefEq decl.type observedInstance)
          if stated then continue
          let (evidence, next) ← (← goal.assert `elementInvariant observedInstance proof).intro1P
          next.modifyLCtx (·.setKind evidence .implDetail)
          -- The selected declaration's clauses, as a map entry's invariant
          -- is observed (`selectedOwnClauses?`).
          let (statement, clauses) ← next.withContext do
            match ← selectedOwnClauses? pure observedInstance (.fvar evidence) with
            | some observed => pure observed
            | none => pure (observedInstance, .fvar evidence)
          goal := (← (← next.assert `elementInvariant statement clauses).intro1P).2
          fresh := true
        if fresh then
          progress := true
        else
          -- A reading of an option the context decides but does not spell,
          -- taken at the value the option holds (`readingsAtValue?`).  Last,
          -- as it reads every statement of the context: the splits above
          -- have taken the context apart by then.
          let readings ← goal.withContext do
            let mut readings ← withSomeReadings (← instantiateMVars (← goal.getType))
            for decl in ← getLCtx do
              unless decl.isImplementationDetail do
                readings := readings ++ (← withSomeReadings (← instantiateMVars decl.type))
            return readings
          for option_ in readings.map (·[2]!) do
            if read.contains option_ then continue
            read := read.push option_
            if let some next ← readingsAtValue? observe goal (readings.filter (·[2]! == option_)) then
              goal := next
              progress := true
              break
  replaceMainGoal [← nameFacts (← nameWrittenMaps (← restoreShadowed (← symmetricDisequalities goal)))]

-- Base supports structural splitting without a native family observer.
-- The normal form installs its canonical producer at the same tactic.
elab "verex_split_hypotheses" : tactic => splitHypotheses (fun goal _ => pure goal)

/-- For every subterm of `e` with a free variable, whether it mentions a
local satisfying `p`, and whether it mentions another one. -/
partial def localMentions (e : Lean.Expr) (p : FVarId → Bool) :
    StateM (Std.HashMap Lean.Expr (Bool × Bool)) (Bool × Bool) := do
  if !e.hasFVar then return (false, false)
  if let some known := (← get)[e]? then return known
  let both (a b : Bool × Bool) := (a.1 || b.1, a.2 || b.2)
  let result ← match e with
    | .fvar id => pure (p id, !p id)
    | .app f a => do pure (both (← localMentions f p) (← localMentions a p))
    | .lam _ t b _ | .forallE _ t b _ => do pure (both (← localMentions t p) (← localMentions b p))
    | .letE _ t v b _ => do
      pure <| both (both (← localMentions t p) (← localMentions v p)) (← localMentions b p)
    | .mdata _ b | .proj _ _ b => localMentions b p
    | _ => pure (false, false)
  modify (·.insert e result)
  return result

/-- The proof arguments of an expression that mention a local, each with
the proposition its position expects, when the local occurs nowhere else.
A proof whose proposition itself mentions the local, through the proofs
of an earlier argument, is left for a later pass, once those are
detached. -/
private partial def proofsMentioning (x : FVarId) (e : Lean.Expr) :
    MetaM (Option (Array (Lean.Expr × Lean.Expr))) := do
  let (_, mentioned) := (localMentions e (· == x)).run {}
  let mentions (e : Lean.Expr) : Bool := (mentioned.getD e (false, false)).1
  -- The proofs a subterm contributes, by subterm.
  let contributions ← IO.mkRef ({} : Std.HashMap Lean.Expr (Option (Array (Lean.Expr × Lean.Expr))))
  let rec visit (e : Lean.Expr) : MetaM (Option (Array (Lean.Expr × Lean.Expr))) := do
    if !mentions e then return some #[]
    if let some known := (← contributions.get)[e]? then return known
    let result ← match e with
      | .app function argument => do
          let some found ← visit function | pure none
          if ← (pure (mentions argument && !argument.hasLooseBVars) <&&> Meta.isProof argument) then
            let .forallE _ proposition _ _ ← whnf (← inferType function) | pure none
            let proposition ← instantiateMVars proposition
            if proposition.containsFVar x || proposition.hasLooseBVars then pure (some found)
            else pure (some (found.push (argument, proposition)))
          else pure ((← visit argument).map (found ++ ·))
      | .mdata _ body => visit body
      | _ => pure none
    contributions.modify (·.insert e result)
    return result
  visit e

/-- An equation between a local and a value that mentions the local only
inside proofs, restated so that it does not: each such proof becomes a
hypothesis of its proposition, innermost first.  By proof irrelevance the
restated equation is the same up to definitional equality, and the local
can be substituted. -/
private partial def detachProofs (goal : MVarId) (equation : FVarId) (x : FVarId) :
    MetaM (Option MVarId) := goal.withContext do
  let type ← instantiateMVars (← equation.getType)
  unless type.isAppOfArity ``Eq 3 do return none
  let (value, valueOnLeft) :=
    if type.getArg! 2 == .fvar x then (type.getArg! 1, true) else (type.getArg! 2, false)
  let some proofs ← proofsMentioning x value | return none
  if proofs.isEmpty then return none
  let hypotheses := proofs.map fun (proof, proposition) =>
    ({ userName := `detached, type := proposition, value := proof } : Hypothesis)
  let (detached, goal) ← goal.assertHypotheses hypotheses
  goal.withContext do
    let value := (proofs.zip detached).foldl (fun value ((proof, _), hypothesis) =>
      value.replace fun sub => if sub == proof then some (.fvar hypothesis) else none) value
    let restated := if valueOnLeft then mkApp3 type.getAppFn (type.getArg! 0) value (.fvar x)
      else mkApp3 type.getAppFn (type.getArg! 0) (.fvar x) value
    let goal ← goal.replaceLocalDeclDefEq equation restated
    -- The proofs the detached ones exposed, until the local is gone.
    if value.containsFVar x then detachProofs goal equation x else return some goal

/-- A native value built of constructors: a variant's injection, a row's
pairs, unit. -/
private def nativeConstructor (value : Lean.Expr) : Bool :=
  match value.getAppFn.constName? with
  | some ``Sum.inl | some ``Sum.inr | some ``Prod.mk | some ``Unit.unit | some ``PUnit.unit
  | some ``Subtype.mk | some ``VerexIR.Proofs.Denote.NTy.ofDeepHolds => true
  | _ => false

/-- An equation fixing a local's native value to a constructor. -/
def fixesLocal (type : Lean.Expr) : Bool :=
  match type.eq? with
  | some (_, lhs, rhs) =>
      let fixes (side other : Lean.Expr) : Bool := match side with
        | .fvar x => nativeConstructor other && !other.containsFVar x
        | _ => false
      fixes lhs rhs || fixes rhs lhs
  | none => false

/-! An equation to a caller's lent value in the callee's view is stated in
the caller's view, as one of a certified value is (`NTy.eq_toSkolem_iff`):
the current of a mutable reference and a struct's row cross a call so. -/

section PlainTransportEquations
variable [Θ : Carriers] [Ξ : @CarriersNonempty Θ]

theorem NTy.plainEq_toSkolem_iff (θ : TypeArgs) (τ : NTy)
    (value : @NTy.plain (Carriers.instantiate θ Θ) τ) (caller : (τ.subst θ.1).plain) :
    value = NTy.plainToSkolem θ τ caller ↔ NTy.plainOfSkolem θ τ value = caller :=
  ⟨fun same => same ▸ NTy.plainOfSkolem_toSkolem θ τ caller,
    fun same => same ▸ (NTy.plainToSkolem_ofSkolem θ τ value).symm⟩

theorem HListPlain.eq_toSkolem_iff (θ : TypeArgs) (row : NRow)
    (values : @HListPlain (Carriers.instantiate θ Θ) row)
    (caller : HListPlain (NRow.subst θ.1 row)) :
    values = HListPlain.toSkolem θ row caller ↔ HListPlain.ofSkolem θ row values = caller :=
  ⟨fun same => same ▸ HListPlain.ofSkolem_toSkolem θ row caller,
    fun same => same ▸ (HListPlain.toSkolem_ofSkolem θ row values).symm⟩

theorem variantPlain.eq_toSkolem_iff (θ : TypeArgs) (names : List String) (rows : NRows)
    (value : @variantPlain (Carriers.instantiate θ Θ) names rows)
    (caller : variantPlain names (NRows.subst θ.1 rows)) :
    value = variantPlain.toSkolem θ names rows caller ↔
      variantPlain.ofSkolem θ names rows value = caller :=
  ⟨fun same => same ▸ variantPlain.ofSkolem_toSkolem θ names rows caller,
    fun same => same ▸ (variantPlain.toSkolem_ofSkolem θ names rows value).symm⟩

end PlainTransportEquations

/-- An equation to a caller's value in a generic callee's view,
`x = toSkolem θ τ v`, restated in the caller's view, `ofSkolem θ τ x = v`
(`NTy.eq_toSkolem_iff`), where the caller's value is a local to substitute.
The equation is typed at the caller's family, which no lemma keyed on the
transport's family matches, so the rule is applied to its arguments. -/
private def transportEquation? (goal : MVarId) : MetaM (Option MVarId) := goal.withContext do
  let rule? (side : Lean.Expr) : Option (Name × Array Lean.Expr) :=
    match side.getAppFn.constName?, side.getAppArgs with
    | some ``VerexIR.Proofs.Denote.NTy.toSkolem, #[family, nonempty, θ, τ, v] =>
        some (``VerexIR.Proofs.Denote.NTy.eq_toSkolem_iff, #[family, nonempty, θ, τ, v])
    | some ``VerexIR.Proofs.Denote.HList.toSkolem, #[family, nonempty, θ, row, v] =>
        some (``VerexIR.Proofs.Denote.HList.eq_toSkolem_iff, #[family, nonempty, θ, row, v])
    | some ``VerexIR.Proofs.Denote.variantCarrier.toSkolem,
        #[family, nonempty, θ, names, rows, v] =>
        some (``VerexIR.Proofs.Denote.variantCarrier.eq_toSkolem_iff,
          #[family, nonempty, θ, names, rows, v])
    -- The lent transports: a reference's current and a struct's row cross a
    -- call as a certified value does.
    | some ``VerexIR.Proofs.Denote.NTy.plainToSkolem, #[family, nonempty, θ, τ, v] =>
        some (``VerexIR.Proofs.Denote.NTy.plainEq_toSkolem_iff, #[family, nonempty, θ, τ, v])
    | some ``VerexIR.Proofs.Denote.HListPlain.toSkolem, #[family, nonempty, θ, row, v] =>
        some (``VerexIR.Proofs.Denote.HListPlain.eq_toSkolem_iff, #[family, nonempty, θ, row, v])
    | some ``VerexIR.Proofs.Denote.variantPlain.toSkolem,
        #[family, nonempty, θ, names, rows, v] =>
        some (``VerexIR.Proofs.Denote.variantPlain.eq_toSkolem_iff,
          #[family, nonempty, θ, names, rows, v])
    | _, _ => none
  for decl in ← getLCtx do
    if decl.isImplementationDetail then continue
    let ty ← instantiateMVars decl.type
    let some (_, lhs, rhs) := ty.eq? | continue
    let (proof, rule) ← match rule? rhs, rule? lhs with
      | some rule, _ => pure (decl.toExpr, rule)
      | none, some rule => pure (← mkEqSymm decl.toExpr, rule)
      | none, none => continue
    let (name, arguments) := rule
    let other := if (rule? rhs).isSome then lhs else rhs
    let caller := arguments.back!
    let iff := mkAppN (mkConst name) (arguments.pop.push other |>.push caller)
    let restated ← mkAppM ``Iff.mp #[iff, proof]
    let (_, next) ← (← goal.assert decl.userName (← inferType restated) restated).intro1P
    return some (← next.tryClear decl.fvarId)
  return none

/-- An integer literal, also as a natural number cast. -/
def intLiteral? (e : Lean.Expr) : Option Int :=
  e.int? <|> if e.isAppOfArity ``Nat.cast 3 then (e.appArg!.nat?).map Int.ofNat else none

/-- The integer local whose value a term reads, `x.val`. -/
def valuedLocal? (e : Lean.Expr) : Option FVarId :=
  if e.isAppOfArity ``VerexIR.SpecInt.val 3 then e.appArg!.fvarId? else none

/-- The integer a term is: a certified integer's value, `x.val`, or a plain
`Int` local, as a quantifier's binder is once it is introduced.  The local,
and whether the term read its value.  (`boundedLocal?` above reads the locals
a range bound speaks of, including a map's size, and is not this.) -/
def boundedInteger? (e : Lean.Expr) : MetaM (Option (FVarId × Bool)) := do
  if let some x := valuedLocal? e then return some (x, true)
  let some x := e.fvarId? | return none
  unless (← instantiateMVars (← x.getType)).isConstOf ``Int do return none
  return some (x, false)

/-- A bound's side as a term and an integer offset, `t + k`, with the literal
addends peeled off: `none` where the side is a literal alone, which the
literal bounds record instead.  A strict bound is the non-strict bound at the
neighbouring offset, so one offset carries both. -/
partial def termOffset (e : Lean.Expr) : Option (Lean.Expr × Int) :=
  if (intLiteral? e).isSome then none
  else if e.isAppOfArity ``HAdd.hAdd 6 then
    let (left, right) := (e.getArg! 4, e.getArg! 5)
    match intLiteral? right with
    | some c => (termOffset left).map fun (term, k) => (term, k + c)
    | none => match intLiteral? left with
      | some c => (termOffset right).map fun (term, k) => (term, k + c)
      | none => some (e, 0)
  else if e.isAppOfArity ``HSub.hSub 6 then
    match intLiteral? (e.getArg! 5) with
    | some c => (termOffset (e.getArg! 4)).map fun (term, k) => (term, k - c)
    | none => some (e, 0)
  else some (e, 0)

/-- The term a bound's side gives at its offset: `t`, `t + k` or `t - k`. -/
def offsetTerm (term : Lean.Expr) (k : Int) : MetaM Lean.Expr :=
  if k == 0 then pure term
  else if k > 0 then mkAppM ``HAdd.hAdd #[term, toExpr k]
  else mkAppM ``HSub.hSub #[term, toExpr (-k)]

/-- What the context states of one integer local's value: the tightest
literal bounds, strictly or not, and the terms bounding it below and above,
each with the hypothesis stating it.  A bound that reads the local bounds
nothing, and a literal bound is no term bound: the two kinds are disjoint and
`pinnedInteger?` and `pinnedTerm?` read them.  The term arrays keep the order of the
context, so that the step reading them is a function of the context and not of
a hash. -/
structure ValueBounds where
  /-- Whether the bounds read the local's value, `x.val`, or the local. -/
  certified : Bool := true
  lowerLiteral : Option (Int × Lean.Expr) := none
  upperLiteral : Option (Int × Lean.Expr) := none
  /-- `term + offset ≤ x`, with the hypothesis stating it. -/
  lowerTerms : Array (Lean.Expr × Int × Lean.Expr) := #[]
  /-- `x ≤ term + offset`, with the hypothesis stating it. -/
  upperTerms : Array (Lean.Expr × Int × Lean.Expr) := #[]

/-- The bounds the context states of each integer local's value, in one walk
of the context (`ValueBounds`). -/
def valueBounds (goal : MVarId) : MetaM (Std.HashMap FVarId ValueBounds) :=
  goal.withContext do
  let mut bounds : Std.HashMap FVarId ValueBounds := {}
  for decl in ← getLCtx do
    if decl.isImplementationDetail then continue
    let type ← instantiateMVars decl.type
    let strict := type.isAppOfArity ``LT.lt 4
    unless strict || type.isAppOfArity ``LE.le 4 do continue
    let (left, right) := (type.getArg! 2, type.getArg! 3)
    if let some (x, certified) ← boundedInteger? right then
      let held := { bounds.getD x {} with certified }
      if let some c := intLiteral? left then
        let c := if strict then c + 1 else c
        unless held.lowerLiteral.any (c ≤ ·.1) do
          bounds := bounds.insert x { held with lowerLiteral := some (c, decl.toExpr) }
      else if let some (term, k) := termOffset left then
        unless term.containsFVar x || term.hasLooseBVars do
          let k := if strict then k + 1 else k
          bounds := bounds.insert x
            { held with lowerTerms := held.lowerTerms.push (term, k, decl.toExpr) }
    if let some (x, certified) ← boundedInteger? left then
      let held := { bounds.getD x {} with certified }
      if let some c := intLiteral? right then
        let c := if strict then c - 1 else c
        unless held.upperLiteral.any (·.1 ≤ c) do
          bounds := bounds.insert x { held with upperLiteral := some (c, decl.toExpr) }
      else if let some (term, k) := termOffset right then
        unless term.containsFVar x || term.hasLooseBVars do
          let k := if strict then k - 1 else k
          bounds := bounds.insert x
            { held with upperTerms := held.upperTerms.push (term, k, decl.toExpr) }
  return bounds

/-- The tightest literal bounds the context states of each integer local,
strictly or not, with the hypotheses stating them: the literal part of
`valueBounds`, for the locals it bounds on both sides. -/
def integerBounds (goal : MVarId) :
    MetaM (Std.HashMap FVarId ((Int × Lean.Expr) × (Int × Lean.Expr))) := do
  return (← valueBounds goal).fold (init := {}) fun literal x held =>
    match held.certified, held.lowerLiteral, held.upperLiteral with
    | true, some below, some above => literal.insert x (below, above)
    | _, _, _ => literal

/-- An integer local the context pins to one value, bounded below and above
by literals, strictly or not, that leave one value: the local, the value,
and the two bounds. -/
def pinnedInteger? (goal : MVarId) (tried : Array FVarId) :
    MetaM (Option (FVarId × Int × Lean.Expr × Lean.Expr)) := do
  for (x, ((c, below), (c', above))) in ← integerBounds goal do
    if !tried.contains x && c == c' then return some (x, c, below, above)
  return none

/-- An integer local the context pins to a term, and that term: bounded below
and above by the same term at the same offset, which does not read the local,
so the local is that term.  `pinnedInteger?` is the same step where the bounds
are literals, which decide a value and belong to the preparation every leaf
receives; a term bound is an inference about this leaf — a loop counter the
invariant bounds by a length and the exit of the loop bounds from the other
side — so it is a stage of the decision (`verex_pin_terms`), not of the
preparation, and the leaf an authored proof takes over keeps the local as the
source names it.  A local whose value an equation of the context already
states is settled, so the stage is idempotent.  The local, whether the bounds
read its value, the term, and the two bounds. -/
def pinnedTerm? (goal : MVarId) (tried : Array FVarId) :
    MetaM (Option (FVarId × Bool × Lean.Expr × Lean.Expr × Lean.Expr)) := goal.withContext do
  let bounds ← valueBounds goal
  let mut settled : Array FVarId := #[]
  for decl in ← getLCtx do
    if decl.isImplementationDetail then continue
    let type ← instantiateMVars decl.type
    unless type.isAppOfArity ``Eq 3 do continue
    for side in #[type.getArg! 1, type.getArg! 2] do
      if let some x := valuedLocal? side then settled := settled.push x
      else if let some x := side.fvarId? then settled := settled.push x
  for decl in ← getLCtx do
    if decl.isImplementationDetail || tried.contains decl.fvarId then continue
    if settled.contains decl.fvarId then continue
    let some held := bounds[decl.fvarId]? | continue
    for (bound, k, below) in held.lowerTerms do
      for (bound', k', above) in held.upperTerms do
        if k == k' && (← withNewMCtxDepth (isDefEq bound bound')) then
          return some (decl.fvarId, held.certified, ← offsetTerm bound k, below, above)
  return none

/-- Destructure an integer local and substitute its value, which an equation
`x.val = c` states. -/
def substituteValue (goal : MVarId) (x equation : FVarId) : MetaM MVarId := do
  let #[case] ← goal.cases x | throwError "an integer has one constructor"
  let some (Lean.Expr.fvar val) := case.fields[0]? | throwError "an integer's value"
  let Lean.Expr.fvar equation := case.subst.get equation | throwError "the value's equation"
  case.mvarId.withContext do
    let type ← instantiateMVars (← equation.getType)
    let reduced ← mkEq (mkFVar val) (type.getArg! 2)
    let goal ← case.mvarId.replaceLocalDeclDefEq equation reduced
    Lean.Meta.subst goal equation

/-- Read a pinned local's equation everywhere its value occurs: every
hypothesis that mentions the local, then the goal. The local and the equation
stay. A rewrite a motive refuses is left as it was, since the equation remains
available to the deciders. -/
def rewriteByPinned (goal : MVarId) (x equation : FVarId) : MetaM MVarId := do
  let mut goal := goal
  let targets ← goal.withContext do
    (← getLCtx).decls.toArray.filterMapM fun
      | some decl => do
          if decl.isImplementationDetail || decl.fvarId == equation then return none
          unless ← isProp decl.type do return none
          return if (← instantiateMVars decl.type).containsFVar x then some decl.fvarId else none
      | none => pure none
  for target in targets do
    let rewritten ← goal.withContext do
      try
        let type ← instantiateMVars (← target.getType)
        let result ← goal.rewrite type (mkFVar equation)
        pure (some (← goal.replaceLocalDecl target result.eNew result.eqProof))
      catch _ => pure none
    if let some result := rewritten then goal := result.mvarId
  goal ← goal.withContext do
    try
      let type ← instantiateMVars (← goal.getType)
      unless type.containsFVar x do return goal
      let result ← goal.rewrite type (mkFVar equation)
      goal.replaceTargetEq result.eNew result.eqProof
    catch _ => pure goal
  return goal

/-- Destructure an integer local the context pins to a term and substitute
the term, so that the positions and lengths computed from the local evaluate
and the facts and the goal spell it alike.  The two bounds decide the
equation. -/
def substitutePinnedTerm (goal : MVarId) (x : FVarId) (certified : Bool)
    (value below above : Lean.Expr) : MetaM MVarId := do
  let proof ← goal.withContext do
    let read ← if certified then mkAppM ``VerexIR.SpecInt.val #[mkFVar x]
      else pure (mkFVar x)
    let statement ← mkEq read value
    let proof ← mkFreshExprMVar statement
    let some refutation ← proof.mvarId!.falseOrByContra | throwError "a pinned value"
    refutation.withContext do
      let negated := (← getLCtx).lastDecl.map (·.toExpr)
      omegaHidden ([below, above] ++ negated.toList) refutation
    instantiateMVars proof
  let (equation, goal) ← (← goal.assert `pinned (← goal.withContext (inferType proof)) proof).intro1P
  -- The equation is read everywhere the local's value is: the facts and the
  -- goal spell the term, which is what a decider and a congruence need, and
  -- the local itself and the equation stay, so a leaf an authored proof takes
  -- over still names the source local (`verex_prepare_leaf`). Eliminating a
  -- local remains the work of substituting an equation that names it.
  let goal ← rewriteByPinned goal x equation
  (← targetState).pinnedCount.modify (· + 1)
  return goal

/-- Destructure a certified integer local the context pins to a literal and
substitute the value, so that the positions and lengths computed from it
evaluate: the destructuring is what makes a read such as `v[x.val.toNat]`
compute, which rewriting by the equation alone does not
(`VerexSL/Tests/CallRangeResults.lean`), so the literal case eliminates the
local as it always has while the term case rewrites and keeps
(`substitutePinnedTerm`). -/
def substitutePinned (goal : MVarId) (x : FVarId) (value : Int) (below above : Lean.Expr) :
    MetaM MVarId := do
  let proof ← goal.withContext do
    let read ← mkAppM ``VerexIR.SpecInt.val #[mkFVar x]
    let statement ← mkEq read (toExpr value)
    let proof ← mkFreshExprMVar statement
    let some refutation ← proof.mvarId!.falseOrByContra | throwError "a pinned value"
    refutation.withContext do
      let negated := (← getLCtx).lastDecl.map (·.toExpr)
      omegaHidden ([below, above] ++ negated.toList) refutation
    instantiateMVars proof
  let (equation, goal) ← (← goal.assert `pinned (← goal.withContext (inferType proof)) proof).intro1P
  let goal ← substituteValue goal x equation
  (← targetState).pinnedCount.modify (· + 1)
  return goal

/-- An equation fixing a vector local's elements, `x.values = e` or, as the
normalization states a literal, `l = x.values.toList`, with `x` not on the
other side and `eligible` (in the walk, not among the kept locals,
`isSummaryParameter`), the equations in `skipped` aside: the equation and
the local. -/
private def vectorEquation? (goal : MVarId) (eligible : FVarId → Bool)
    (skipped : FVarIdSet := {}) : MetaM (Option (FVarId × FVarId)) :=
  goal.withContext do
    let fixed (side other : Lean.Expr) : Option FVarId :=
      let values := if side.isAppOfArity ``Array.toList 2 then side.appArg! else side
      if values.isAppOfArity ``VerexIR.SpecVector.values 2 then
        match values.appArg! with
        | .fvar x => if other.containsFVar x || !eligible x then none else some x
        | _ => none
      else none
    for decl in ← getLCtx do
      if decl.isImplementationDetail || skipped.contains decl.fvarId then continue
      let ty ← instantiateMVars decl.type
      unless ty.isAppOfArity ``Eq 3 do continue
      if let some x := fixed (ty.getArg! 1) (ty.getArg! 2) |>.orElse
          fun _ => fixed (ty.getArg! 2) (ty.getArg! 1) then
        return some (decl.fvarId, x)
    return none

/-- An array local whose vector's elements an equation fixes, as
`vectorEquation?` finds a vector local: destructured, its vector is that
local. -/
private def arrayEquation? (goal : MVarId) (eligible : FVarId → Bool) :
    MetaM (Option FVarId) :=
  goal.withContext do
    let fixed (side other : Lean.Expr) : Option FVarId :=
      let values := if side.isAppOfArity ``Array.toList 2 then side.appArg! else side
      let vector := if values.isAppOfArity ``VerexIR.SpecVector.values 2 then values.appArg!
        else values
      if vector.isAppOfArity ``VerexIR.SpecArray.vector 3 then
        match vector.appArg! with
        | .fvar x => if other.containsFVar x || !eligible x then none else some x
        | _ => none
      else none
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let ty ← instantiateMVars decl.type
      unless ty.isAppOfArity ``Eq 3 do continue
      if let some x := fixed (ty.getArg! 1) (ty.getArg! 2) |>.orElse
          fun _ => fixed (ty.getArg! 2) (ty.getArg! 1) then
        return some x
    return none

/-- Destructure a vector local whose elements an equation fixes, and
substitute the elements: arrays have no decision procedure, so what is
known of them is known only where they are spelled alike. -/
private def substituteVector (goal : MVarId) (equation x : FVarId) : MetaM MVarId := do
  let #[case] ← goal.cases x | throwError "a vector has one constructor"
  let goal := case.mvarId
  let some (Lean.Expr.fvar values) := case.fields[0]? | throwError "a vector's elements"
  let Lean.Expr.fvar equation := case.subst.get equation | throwError "the vector's equation"
  goal.withContext do
    let type ← instantiateMVars (← equation.getType)
    let lhs ← whnfR (type.getArg! 1)
    let rhs ← whnfR (type.getArg! 2)
    let proof := Lean.mkFVar equation
    -- The elements' list, as reduction spells it: a projection of the
    -- elements.
    let isList (side : Lean.Expr) : MetaM Bool := do
      let inner? := match side with
        | .proj ``Array 0 inner => some inner
        | _ => if side.isAppOfArity ``Array.toList 2 then some side.appArg! else none
      let some inner := inner? | return false
      return (← whnfR inner) == Lean.mkFVar values
    -- `values = e`, from whichever side and spelling the equation has.
    let (proof, other) ← if lhs == Lean.mkFVar values then pure (proof, type.getArg! 2)
      else if rhs == Lean.mkFVar values then pure (← mkEqSymm proof, type.getArg! 1)
      else if ← isList rhs then
        let literal := type.getArg! 1
        pure (← mkEqSymm (← mkAppM ``Iff.mpr #[← mkAppOptM ``List.toArray_eq_iff
          #[none, literal, Lean.mkFVar values], proof]), ← mkAppM ``List.toArray #[literal])
      else if ← isList lhs then
        let literal := type.getArg! 2
        pure (← mkEqSymm (← mkAppM ``Iff.mpr #[← mkAppOptM ``List.toArray_eq_iff
          #[none, literal, Lean.mkFVar values], ← mkEqSymm proof]),
          ← mkAppM ``List.toArray #[literal])
      else throwError "a vector's equation states neither its elements nor their list"
    let statement ← mkEq (Lean.mkFVar values) other
    let (fixed, goal) ← (← goal.assert `elements statement proof).intro1P
    let goal ← goal.tryClear equation
    Lean.Meta.subst goal fixed

/-- Solve a goal by a case hypothesis `∀ xs, a = b → … → False` that an
instance refutes: the fall-through case of a match whose discriminant is
known to be a pattern's constructor, as a split states it or as the
normalizer leaves it, its equation taken apart. -/
def refuteFallthrough (goal : MVarId) : MetaM Bool := goal.withContext do
  for decl in ← getLCtx do
    if decl.isImplementationDetail then continue
    let type ← instantiateMVars decl.type
    unless type.isForall do continue
    let refuted ← observing? do
      let (arguments, _, body) ← forallMetaTelescopeReducing type
      unless body.isConstOf ``False do failure
      let mut equations := 0
      for argument in arguments do
        let argumentType ← instantiateMVars (← inferType argument)
        let_expr Eq _ left right := argumentType | continue
        unless ← isDefEq left right do failure
        argument.mvarId!.assign (← mkEqRefl left)
        equations := equations + 1
      if equations == 0 then failure
      let proof ← instantiateMVars (mkAppN decl.toExpr arguments)
      if proof.hasExprMVar then failure
      goal.assign (← mkFalseElim (← goal.getType) proof)
    if refuted.isSome then return true
  return false

/-- A `simp only` tactic as a simp context and its simprocs, to run over the
parts of a goal directly. -/
def simpContextOf (stx : Syntax) : TacticM (Simp.Context × Simp.SimprocsArray) := do
  let { ctx, simprocs, .. } ← mkSimpContext stx (eraseLocal := false)
  return (ctx, simprocs)

/-- The normalization of `verex_normalize`, as a simp context and its
simprocs. It reads no hypothesis, so one serves every goal of a target. -/
def normalization : TacticM (Simp.Context × Simp.SimprocsArray) := do
  simpContextOf (← `(tactic| simp only [vir_denote, vir_denote_norm, vir_denote_eval, Prod.fst, Prod.snd]))

/-- The variant names of an enum carrier's type, spelled through its
definitions: `(NTy.enum _ _ names _ _).carrier` or `variantCarrier names _`. -/
private def variantNames? (type : Lean.Expr) : MetaM (Option (List String)) := do
  let mut unfolded := type
  for _ in [:8] do
    let native ← if unfolded.isAppOfArity ``NTy.carrier 2 then
        nativeTypeView (unfolded.getArg! 1)
      else pure unfolded
    let names? :=
      if unfolded.isAppOfArity ``VerexIR.Proofs.Denote.variantCarrier 3 ||
          unfolded.isAppOfArity ``VerexIR.Proofs.Denote.variantPlain 3 ||
          unfolded.isAppOfArity ``VerexIR.Proofs.Denote.variantPlainOf 3 then
        some (unfolded.getArg! 1)
      else if unfolded.isAppOfArity ``VerexIR.Proofs.Denote.NTy.carrier 2 &&
          native.isAppOfArity ``VerexIR.Proofs.Denote.NTy.enum 5 then
        some (native.getArg! 2)
      else none
    if let some names := names? then
      let mut list := names
      let mut found := #[]
      while list.isAppOfArity ``List.cons 3 do
        let .lit (.strVal name) := list.getArg! 1 | return none
        found := found.push name
        list := list.getArg! 2
      return some found.toList
    let some next ← unfoldDefinition? unfolded | return none
    unfolded ← whnfCore next
  return none

/-- The names the alternatives of a variant-valued local take, from its
source name `x`: the payload of the variant it holds is `x_Variant`, the
remaining alternatives carry the position on. -/
private def alternativeNames (value : FVarId) : MetaM (Array AltVarNames) := do
  let decl ← value.getDecl
  let user := decl.userName
  if user.isAnonymous || user.hasMacroScopes then return #[]
  let (base, index) := match user with
    | .num (.str base "variants") k => (base, k)
    | _ => (user, 0)
  -- The names are those of the alternatives the local still has.
  let some names ← variantNames? (← instantiateMVars decl.type) | return #[]
  let some variant := names.head? | return #[]
  return #[{ varNames := [Name.mkSimple s!"{base}_{variant}"] },
    { varNames := [Name.mkNum (Name.str base "variants") (index + 1)] }]

/-- Substitute the equations the walk kept because they fix a shared fold
pin among `pins` (`TargetState.sharedPins`): local to local, local to
literal, a vector's elements. A leaf is compared with no arrival, so it
takes the normal form the walk would give it without the pins. Run with the
pins lifted (`withSharedPinsLifted`): a summary's parameters still stay. An
equation that does not substitute stays. -/
def substitutePinEquations (goal : MVarId) (pins : FVarIdSet) : MetaM MVarId := do
  if pins.isEmpty then return goal
  let kept ← summaryParameters
  let mut goal := goal
  -- A fact the walk keeps on purpose stays (`TargetState.keptFacts`).
  let mut skipped : FVarIdSet := ← keptFacts
  repeat
    let scalar? ← goal.withContext do
      (← getLCtx).findDeclM? fun decl => do
        if decl.isImplementationDetail || skipped.contains decl.fvarId then return none
        let ty ← instantiateMVars decl.type
        unless ty.isAppOfArity ``Eq 3 do return none
        let pinned (x other : Lean.Expr) : Bool := match x with
          | .fvar id => pins.contains id && !other.containsFVar id &&
              (other.isFVar || !(other.hasFVar || other.hasMVar))
          | _ => false
        return if pinned (ty.getArg! 1) (ty.getArg! 2) || pinned (ty.getArg! 2) (ty.getArg! 1)
          then some decl.fvarId else none
    let next? ← match scalar? with
      | some equation => pure (some (equation, none))
      | none => do
          let vector? ← vectorEquation? goal (fun x => pins.contains x && !kept.contains x) skipped
          pure (vector?.map fun (equation, x) => (equation, some x))
    let some (equation, vector?) := next? | break
    let saved ← saveState
    try
      goal ← match vector? with
        | none => substEquation goal equation
        | some x => substituteVector goal equation x
    catch _ =>
      saved.restore
      skipped := skipped.insert equation
  return goal

/-- The cases of a local, as `cases` gives them, except that a parameter of
the derived summary being stated stays (`isSummaryParameter`): its
occurrences are generalized to a local the cases take apart, equated to the
parameter, as the walk's `value_eq` equates a tested term to its value.
Where the context's dependencies admit no generalization, the parameter is
taken apart as `cases` does: the pin only keeps a context alike for sharing,
and no proof depends on it, so losing it costs that sharing, not the split. -/
private def casesLocal (goal : MVarId) (value : FVarId) (names : Array AltVarNames) :
    MetaM (Array CasesSubgoal) := goal.withContext do
  let previous ← getLCtx
  let canonical (cases : Array CasesSubgoal) := cases.mapM fun case => do
    return { case with mvarId := ← canonicalCaseEquations previous case.mvarId }
  unless ← isSummaryParameter value do return ← canonical (← goal.cases value names)
  let hypotheses ← (← getLCtx).foldlM (init := #[]) fun found decl => do
    if decl.isImplementationDetail || decl.fvarId == value then return found
    return if (mkFVar value).occurs (← instantiateMVars decl.type) then found.push decl.fvarId
      else found
  let name ← value.getUserName
  let saved ← saveState
  try
    let (_, introduced, generalized) ← goal.generalizeHyp
      #[{ expr := mkFVar value, xName? := some name, hName? := some (name.appendAfter "_eq") }]
      hypotheses
    let some local_ := introduced[0]? | throwError "the parameter is not generalized"
    canonical (← generalized.cases local_ names)
  catch _ =>
    saved.restore
    canonical (← goal.cases value names)

/-- A field of a case in a goal the destructuring of an earlier field left:
splitting one field substitutes the case's value over the context, so the
remaining fields of that case are declared again, under their names and with
new identities.  The field as this goal holds it, by its identity where that
still stands and by its name otherwise; none where the goal no longer holds
it at all. -/
private def fieldOf? (goal : MVarId) (field : FVarId) (name : Name) :
    MetaM (Option FVarId) := goal.withContext do
  if (← getLCtx).contains field then return some field
  return ((← getLCtx).findFromUserName? name).map (·.fvarId)

/-- Destructure a local along its native carrier: a variant sum into its
alternatives, a row into its fields, until the parts are scalars or
parameters; the parts named from the local (`alternativeNames`,
`componentNames`). -/
private partial def destructureCarrier (goal : MVarId) (value : FVarId) :
    MetaM (Array MVarId) := goal.withContext do
  let type ← whnfD (← instantiateMVars (← value.getType))
  -- A struct's or a variant's value is its row under a certificate
  -- (`NTy.carrier`): splitting it exposes the row, which splits on.
  unless type.isAppOf ``Sum || type.isAppOf ``Prod || type.isAppOf ``PUnit ||
      type.isAppOf ``Empty || type.isAppOfArity ``Subtype 2 do
    return #[goal]
  let (goal, names) ← if type.isAppOf ``Sum then pure (goal, ← alternativeNames value)
    else componentNames goal value
  let cases ← casesLocal goal value names
  cases.foldlM (init := #[]) fun found case => do
    let mut goals := #[case.mvarId]
    for field in case.fields do
      let .fvar part := field | continue
      let name ← case.mvarId.withContext part.getUserName
      goals ← goals.flatMapM fun goal => do
        let some part ← fieldOf? goal part name | return #[goal]
        destructureCarrier goal part
    return found ++ goals

/-- Split the requested variant and its row, leaving variant-valued fields
intact. Their cases are independent of the outer tag a proof is reading. -/
partial def destructureOuterVariant (goal : MVarId) (value : FVarId)
    (row := false) : MetaM (Array MVarId) := goal.withContext do
  let type ← whnfD (← instantiateMVars (← value.getType))
  let variant := type.isAppOf ``Sum
  if variant && row then return #[goal]
  -- A certificate stands between an enum's value and its variants
  -- (`NTy.carrier`); splitting it is not entering a row.
  let certified := type.isAppOfArity ``Subtype 2
  unless variant || certified || type.isAppOf ``Prod || type.isAppOf ``PUnit ||
      type.isAppOf ``Empty do
    return #[goal]
  let (goal, names) ← if variant then pure (goal, ← alternativeNames value)
    else componentNames goal value
  let cases ← casesLocal goal value names
  cases.foldlM (init := #[]) fun found case => do
    let mut goals := #[case.mvarId]
    for field in case.fields do
      let .fvar part := field | continue
      let name ← case.mvarId.withContext part.getUserName
      let inRow := if certified then row
        else row || !variant || case.ctorName == some ``Sum.inl
      goals ← goals.flatMapM fun goal => do
        let some part ← fieldOf? goal part name | return #[goal]
        destructureOuterVariant goal part inRow
    return found ++ goals

/-- Whether a term reads which variant a value holds: its name or a
payload projection, at the certified layer and at the plain layer, whose
spellings read a variant where the certified layer's do
(`reduceVariantComputations`, `variantRedex`).  A value of a declaration's
type is certified and its variants are the plain layer's
(`NTy.carrier`), so a leaf observes both. -/
private def variantReading (e : Lean.Expr) : Bool :=
  e.isAppOf ``VerexIR.Proofs.Denote.variantName ||
    e.isAppOf ``VerexIR.Proofs.Denote.variantPlainName ||
    e.isAppOf ``VerexIR.Proofs.Denote.variantPlainNameOf ||
    e.isAppOf ``VerexIR.Proofs.Denote.Which.project? ||
    e.isAppOf ``VerexIR.Proofs.Denote.Which.projectCertified?

/-- The value a reading of a variant reads (`variantReading`, a selection
of a variant's field), where the reading is applied to all its arguments:
its last one.  `none` where the term is not such a reading, a partial
application among them, whose last argument is not the value read. -/
private def variantReadingSubject? (e : Lean.Expr) : Option Lean.Expr :=
  if e.isAppOfArity ``VerexIR.Proofs.Denote.variantName 4 ||
      e.isAppOfArity ``VerexIR.Proofs.Denote.variantPlainName 4 ||
      e.isAppOfArity ``VerexIR.Proofs.Denote.variantPlainNameOf 4 ||
      e.isAppOfArity ``VerexIR.Proofs.Denote.Which.project? 6 ||
      e.isAppOfArity ``VerexIR.Proofs.Denote.Which.projectCertified? 9 ||
      e.isAppOfArity ``VerexIR.Proofs.Denote.Choices.select? 6 then
    some e.appArg!
  else none

/-- The reading of a variant's name alone (`variantReading`), without a
payload projection. -/
private def variantTagReading (e : Lean.Expr) : Bool :=
  e.isAppOf ``VerexIR.Proofs.Denote.variantName ||
    e.isAppOf ``VerexIR.Proofs.Denote.variantPlainName ||
    e.isAppOf ``VerexIR.Proofs.Denote.variantPlainNameOf

/-- An observed tag test (`variantName x = "V"`, or a Boolean test of
it), without a payload observation or a quantified consequence. These
compute after a variant split without normalizing the rest of the
context. -/
partial def isTagObservation (type : Lean.Expr) : Bool :=
  if type.isAppOfArity ``VerexIR.Proofs.Obligation 4 then
    isTagObservation (type.getArg! 3)
  else if type.isAppOfArity ``Not 1 then isTagObservation type.appArg!
  else type.isAppOfArity ``Eq 3 &&
    (type.find? variantTagReading).isSome &&
    !(type.find? (·.isAppOf ``VerexIR.Proofs.Denote.Choices.select?)).isSome

/-- An observation of a value's variant: a tag test or a payload projection
(`Which.project?`) in an equation, or a negation, conjunction or disjunction
of which one is, without a quantified consequence. After a variant split
it computes; a case it refutes closes before the rest of its context is
normalized. -/
partial def isVariantObservation (type : Lean.Expr) : Bool :=
  if type.isAppOfArity ``VerexIR.Proofs.Obligation 4 then
    isVariantObservation (type.getArg! 3)
  else if type.isAppOfArity ``Not 1 then isVariantObservation type.appArg!
  else if type.isAppOfArity ``And 2 || type.isAppOfArity ``Or 2 then
    isVariantObservation (type.getArg! 0) || isVariantObservation (type.getArg! 1)
  else type.isAppOfArity ``Eq 3 &&
    (type.find? variantReading).isSome &&
    !(type.find? (·.isAppOf ``VerexIR.Proofs.Denote.Choices.select?)).isSome

/-- The variants an authored proof chooses, without enumerating variants
inside their fields. Eliminate cases contradicted by observed tags before
splitting the next value; normalize payload observations afterwards, and
bound the payloads (`verex_type_bounds`). -/
elab "verex_cases " values:ident,+ : tactic => focusGoal do
  let mut goals := #[(← getMainGoal)]
  let (ctx, simprocs) ← normalization
  let ctx ← Simp.mkContext { ctx.config with failIfUnchanged := false } ctx.simpTheorems
    ctx.congrTheorems
  for value in values.getElems do
    goals ← goals.flatMapM fun goal => goal.withContext do
      let fvar ← getFVarId value
      destructureOuterVariant goal fvar
    goals ← goals.filterMapM fun goal => goal.withContext do
      let tags ← (← goal.getNondepPropHyps).filterM fun hyp => do
        return isTagObservation (← instantiateMVars (← hyp.getType))
      if tags.isEmpty then return some goal
      let (result, _) ← simpGoal goal ctx simprocs (simplifyTarget := false)
        (fvarIdsToSimp := tags)
      return result.map (·.2)
  setGoals goals.toList
  -- The payloads' bounds, under which their round trips read as the values.
  evalTactic (← `(tactic| all_goals verex_type_bounds))


/-- Close a goal that holds by reflexivity: an equation, equivalence or
heterogeneous equation whose sides agree up to reducible unfolding, under
its clause marker if it has one. -/
def closeReflexive (goal : MVarId) : MetaM Bool := goal.withContext do
  let mut target := (← instantiateMVars (← goal.getType)).consumeMData
  while target.isAppOfArity ``VerexIR.Proofs.Obligation 4 do
    target := (target.getArg! 3).consumeMData
  let reflexive (a b : Lean.Expr) : MetaM Bool := withReducible (isDefEq a b)
  if target.isAppOfArity ``Eq 3 then
    let a := target.getArg! 1
    unless ← reflexive a (target.getArg! 2) do return false
    goal.assign (← mkEqRefl a)
  else if target.isAppOfArity ``Iff 2 then
    let a := target.getArg! 0
    unless ← reflexive a (target.getArg! 1) do return false
    goal.assign (mkApp (mkConst ``Iff.refl) a)
  else if target.isAppOfArity ``HEq 4 then
    let a := target.getArg! 1
    unless ← reflexive (target.getArg! 0) (target.getArg! 2) do return false
    unless ← reflexive a (target.getArg! 3) do return false
    goal.assign (← mkHEqRefl a)
  else
    return false
  return true

/-- A direct payload or field equation constrains a certified value. Expose
its payload without branching on variants so ordinary variable substitution
can consume the equation. Nominal declarations and invariants remain closed. -/
private def constrainedRow? (goal : MVarId) (parameters : FVarIdSet) :
    MetaM (Option (FVarId × Bool)) := goal.withContext do
  let rec owner? (term : Lean.Expr) : Nat → Option FVarId
    | 0 => none
    | fuel + 1 => do
      if term.isAppOfArity ``Prod.fst 3 || term.isAppOfArity ``Prod.snd 3 then
        owner? term.appArg! fuel
      else
        guard (term.isAppOfArity ``Subtype.val 3)
        let .fvar id := term.appArg! | none
        some id
  (← getLCtx).findDeclM? fun decl => do
    if decl.isImplementationDetail then return none
    let some (_, lhs, rhs) := (← instantiateMVars decl.type).eq? | return none
    for (side, other) in #[(lhs, rhs), (rhs, lhs)] do
      let field := side.isAppOfArity ``Prod.fst 3 || side.isAppOfArity ``Prod.snd 3
      unless field || side.isAppOfArity ``Subtype.val 3 do continue
      let some owner := owner? side 64 | continue
      if parameters.contains owner || other.containsFVar owner then continue
      let type ← instantiateMVars (← owner.getType)
      -- Preserve the record-field path: a whole-record equation must not
      -- expose its fields before ordinary substitution consumes the row.
      if field && type.isAppOfArity ``NTy.carrier 2 then
        if (← nativeTypeView (type.getArg! 1)).isAppOfArity ``NTy.struct 3 then
          return some (owner, false)
      let native? ← if type.isAppOfArity ``NTy.carrier 2 then
          pure (some (type.getArg! 1))
        else if type.isAppOfArity ``ResultShape.carrier 2 then do
          let shape ← whnfD (type.getArg! 1)
          pure <| if shape.isAppOfArity ``ResultShape.one 1 then some shape.appArg! else none
        else pure none
      if let some native := native? then
        let native ← whnfD native
        if native.isAppOfArity ``NTy.enum 5 then
          return some (owner, true)
    return none

/-- Substitute every hypothesis equating a local variable to a term, by
syntactic shape alone: no hypothesis is unfolded to find one. With
`equationsOnly`, only such equations: no vector, transport or pinned-integer
substitution. -/
def substVars (equationsOnly : Bool) : TacticM Unit := focusGoal do
  if (← getGoals).isEmpty then return
  let mut goal ← getMainGoal
  let mut progress := true
  let mut attempted : Array FVarId := #[]
  -- Equations `substEquation` refused: they stay as facts.
  let mut refused : Array FVarId := #[]
  -- A summary's parameters stay (`isSummaryParameter`), as the locals a
  -- path's preparation leaves to its leaves: those its clauses read, so
  -- that the clauses reach the walker and the leaves' deciders as they would.
  let parameters ← summaryParameters
  let mut pinned : Array FVarId := parameters.toArray
  let keepTarget := verex.keepTargetLocals.get (← getOptions)
  while progress do
    progress := false
    let read ← if keepTarget then goal.withContext do
        pure (collectFVars {} (← instantiateMVars (← goal.getType))).fvarSet
      else pure {}
    let read := parameters.foldl (fun read x => read.insert x) read
    let equation ← goal.withContext do
      (← getLCtx).findDeclM? fun decl => do
        if decl.isImplementationDetail || refused.contains decl.fvarId then return none
        let ty ← instantiateMVars decl.type
        unless ty.isAppOfArity ``Eq 3 do return none
        let lhs := ty.getArg! 1
        let rhs := ty.getArg! 2
        let substitutable (x : Lean.Expr) (other : Lean.Expr) : Bool :=
          match x with
          | .fvar id => !other.containsFVar id && !read.contains id
          | _ => false
        if substitutable lhs rhs || substitutable rhs lhs then return some (decl.fvarId, none)
        -- The local occurs on the other side, perhaps only inside proofs.
        if attempted.contains decl.fvarId then return none
        match rhs, lhs with
        | .fvar x, _ => if read.contains x then return none else return some (decl.fvarId, some x)
        | _, .fvar x => if read.contains x then return none else return some (decl.fvarId, some x)
        | _, _ => return none
    match equation with
    | some (fvarId, none) =>
        -- `substEquation` restores the state when it fails.
        try goal ← substEquation goal fvarId
        catch _ => refused := refused.push fvarId
        progress := true
    | some (fvarId, some x) =>
        attempted := attempted.push fvarId
        if let some detached ← detachProofs goal fvarId x then
          goal := detached
        progress := true
    | none =>
        -- A vector local whose elements are fixed, a caller's value fixed in
        -- a callee's view, and an integer pinned by its bounds are known
        -- wherever they occur, the clauses included.
        if keepTarget || equationsOnly then break
        if let some (owner, row) ← constrainedRow? goal parameters then
          let cases ← destructureOuterVariant goal owner (row := row)
          let #[next] := cases | throwError "a certified payload has more than one constructor"
          goal := next
          progress := true
        else if let some (equation, vector) ← vectorEquation? goal (!parameters.contains ·) then
          goal ← substituteVector goal equation vector
          progress := true
        else if let some array ← arrayEquation? goal (!parameters.contains ·) then
          let #[case] ← goal.cases array | throwError "an array has one constructor"
          -- The equation then reads the vector local the array holds.
          setGoals [case.mvarId]
          evalTactic (← `(tactic| verex_reduce_projections))
          goal ← getMainGoal
          progress := true
        else if let some restated ← transportEquation? goal then
          goal := restated
          progress := true
        else if let some (x, value, below, above) ← pinnedInteger? goal pinned then
          pinned := pinned.push x
          let saved ← saveState
          try goal ← substitutePinned goal x value below above
          catch _ => saved.restore
          progress := true
  -- An equivalence a substitution decided a side of states its other side.
  goal ← decideEquivalences goal
  -- A match's fall-through case whose discriminant the substitutions have
  -- fixed to a constructor is refuted.
  if ← refuteFallthrough goal then replaceMainGoal []; return
  -- A target the substitutions left reflexive is closed.
  if ← closeReflexive goal then replaceMainGoal []; return
  replaceMainGoal [goal]

elab "verex_subst_vars" : tactic => substVars false

/-- The locals this leaf's bounds pin to a term, substituted into its facts
and its goal (`pinnedTerm?`, `substitutePinnedTerm`): a decision stage, since
pinning a local to a term is an inference about the leaf rather than a form
every leaf is prepared in. The local and its equation stay, so the step is
idempotent and a leaf an authored proof takes over is unchanged by it; the
pipeline's own backtracking discards the rewriting when the stages after it
do not close the leaf. Fails where the bounds pin nothing. -/
elab "verex_pin_terms" : tactic => focusGoal do
  let mut goal ← getMainGoal
  let mut tried : Array FVarId := #[]
  let mut pinned := 0
  while let some (x, certified, value, below, above) ← pinnedTerm? goal tried do
    tried := tried.push x
    let saved ← saveState
    try
      goal ← substitutePinnedTerm goal x certified value below above
      pinned := pinned + 1
    catch _ => saved.restore
  if pinned == 0 then throwError "the bounds pin no local to a term"
  replaceMainGoal [goal]

/-- The equations of `verex_subst_vars` alone (`substVars`). -/
elab "verex_subst_equations" : tactic => substVars true

/-- The locals whose variant a term tests: by its name, or by a projection
of a variant's payload, which computes once the local is destructured. A
test of a bound variable tests no local. -/
def testedIn (expression : Lean.Expr) (found : Array FVarId) : MetaM (Array FVarId) := do
  let result ← IO.mkRef found
  expression.forEach fun e => do
    if let some subject := variantReadingSubject? e then
      -- A certified ADT's tag is read from its plain value. The case split
      -- belongs to the owning local, not to a second value behind `.val`.
      let subject := subject.consumeMData
      let owner := if subject.isAppOfArity ``Subtype.val 3 then subject.appArg!.consumeMData
        else match subject with
          | .proj ``Subtype 0 value => value.consumeMData
          | _ => subject
      if let .fvar x := owner then
        result.modify fun found => if found.contains x then found else found.push x
  result.get

/-- The locals whose variant an authored preparation splits (`verex_prepare`,
`verex_split_variants`): those its goal tests where there are any; else, of
those its hypotheses test, the ones a hypothesis relates to the most of the
goal's variables. A goal still executing has none: its steps take its
locals apart as they reach them. -/
private def testedLocals (goal : MVarId) : MetaM (Array FVarId) := goal.withContext do
  let target ← instantiateMVars (← goal.getType)
  if (target.find? (·.isConstOf ``VerexIR.Proofs.wp)).isSome then return #[]
  let inGoal ← testedIn target #[]
  unless inGoal.isEmpty do return inGoal
  let goalVariables := (collectFVars {} target).fvarSet
  -- Each tested local with the goal's variables the hypotheses testing it
  -- mention.
  let mut related : Array (FVarId × Array FVarId) := #[]
  for decl in ← getLCtx do
    if decl.isImplementationDetail then continue
    let type ← instantiateMVars decl.type
    let tested ← testedIn type #[]
    if tested.isEmpty then continue
    let shared := (collectFVars {} type).fvarIds.filter goalVariables.contains
    for x in tested do
      related := match related.findIdx? (·.1 == x) with
        | some i => related.modify i fun (x, known) =>
            (x, shared.foldl (fun known y => if known.contains y then known else known.push y) known)
        | none => related.push (x, shared)
  let most := related.foldl (fun most (_, shared) => max most shared.size) 0
  return related.filterMap fun (x, shared) =>
    if most == 0 || shared.size == most then some x else none

/-- The locals whose variant the automatic solver splits, in the order it
splits them, one at a time: those the goal tests; else, unless `goalOnly`,
those its hypotheses test, first the ones whose variant the leaf observes
(`isVariantObservation`), where a split prunes rather than multiplies the
cases, then by how close the testing hypotheses are to the goal's
variables through shared variables. A goal still executing has none. -/
def rankedTestedLocals (goal : MVarId) (goalOnly := false) : MetaM (Array FVarId) :=
    goal.withContext do
  let target ← instantiateMVars (← goal.getType)
  if (target.find? (·.isConstOf ``VerexIR.Proofs.wp)).isSome then return #[]
  let inGoal ← testedIn target #[]
  unless inGoal.isEmpty && !goalOnly do return inGoal
  -- The hypotheses testing a local, each with its variables.
  let mut testing : Array (Array FVarId × Bool × FVarIdSet) := #[]
  for decl in ← getLCtx do
    if decl.isImplementationDetail then continue
    let type ← instantiateMVars decl.type
    let tested ← testedIn type #[]
    if tested.isEmpty then continue
    testing := testing.push (tested, isVariantObservation type, (collectFVars {} type).fvarSet)
  -- The distance of each testing hypothesis from the goal's variables.
  let mut reached := (collectFVars {} target).fvarSet
  let mut distance : Array Nat := Array.replicate testing.size 0
  let mut placed : Array Bool := Array.replicate testing.size false
  let mut level := 1
  let mut grown := true
  while grown do
    grown := false
    let mut next := reached
    for h : i in [:testing.size] do
      if placed[i]! then continue
      let (_, _, variables) := testing[i]
      if variables.any reached.contains then
        placed := placed.set! i true
        distance := distance.set! i level
        next := variables.foldl (fun found x => found.insert x) next
        grown := true
    reached := next
    level := level + 1
  -- Each tested local's rank: observed first, then the closest hypothesis.
  let mut ranked : Array (FVarId × Bool × Nat) := #[]
  for h : i in [:testing.size] do
    let (tested, observed, _) := testing[i]
    let near := if placed[i]! then distance[i]! else level
    for x in tested do
      ranked := match ranked.findIdx? (·.1 == x) with
        | some j => ranked.modify j fun (x, seen, best) => (x, seen || observed, min best near)
        | none => ranked.push (x, observed, near)
  let ordered := ranked.qsort fun (_, seenA, nearA) (_, seenB, nearB) =>
    (seenA && !seenB) || (seenA == seenB && nearA < nearB)
  return ordered.map (·.1)

attribute [variant_decision] Option.some.injEq reduceCtorEq Sum.inl.injEq Sum.inr.injEq
  Prod.mk.injEq String.reduceEq String.reduceNe eq_self_iff_true not_true_eq_false
  not_false_eq_true true_iff iff_true false_iff iff_false true_and and_true false_and and_false
  true_or or_true false_or or_false true_implies false_implies implies_true ite_true ite_false

/-- The decisions of a split's cases (`variant_decision`), not failing where
nothing changes. -/
def variantDecision : TacticM (Simp.Context × Simp.SimprocsArray) := do
  let stx ← `(tactic| simp only [variant_decision])
  let { ctx, simprocs, .. } ← mkSimpContext stx (eraseLocal := false)
  return (← Simp.mkContext { ctx.config with failIfUnchanged := false } ctx.simpTheorems
    ctx.congrTheorems, simprocs)

/-- A term's computations of constructors, evaluated: a tag, payload
projection, selection, field or payload read of a constructor, an option
operation or a match on one, each by a step of its definition, inner first.
The result is definitionally the term: the tests of a constructor compute by
definition, whatever the spelling of the type indices. -/
def reduceVariantComputations (e : Lean.Expr) : MetaM Lean.Expr :=
  let constructor (e : Lean.Expr) : Bool :=
    e.isAppOf ``Sum.inl || e.isAppOf ``Sum.inr || e.isAppOf ``Prod.mk || e.isConstOf ``PUnit.unit ||
      e.isAppOf ``Subtype.mk || e.isAppOf ``VerexIR.Proofs.Denote.NTy.ofDeepHolds ||
      e.isAppOf ``Option.some || e.isAppOf ``Option.none
  -- A reading's worker and the elimination it steps to belong to the list
  -- beside the reading itself: one step of `variantPlainName` is its worker
  -- `variantPlainNameOf`, and one step of that is `Sum.elim` at the value,
  -- so a list holding only the reading leaves the computation stuck one
  -- step short of its name.
  let computation (e : Lean.Expr) : Bool := e.isApp &&
    (e.isAppOf ``VerexIR.Proofs.Denote.variantName ||
      e.isAppOf ``VerexIR.Proofs.Denote.variantPlainName ||
      e.isAppOf ``VerexIR.Proofs.Denote.variantPlainNameOf ||
      e.isAppOf ``VerexIR.Proofs.Denote.Which.project? ||
      e.isAppOf ``VerexIR.Proofs.Denote.Which.projectCertified? ||
      e.isAppOf ``VerexIR.Proofs.Denote.Choices.select? || e.isAppOf ``VerexIR.Proofs.Denote.Var.select ||
      e.isAppOf ``VerexIR.Proofs.Denote.Var.selectPlain ||
      e.isAppOf ``VerexIR.Proofs.Denote.variantPayload || e.isAppOfArity ``Option.map 4 ||
      e.isAppOfArity ``Option.getD 3 || e.isAppOfArity ``Option.bind 4 ||
      e.isAppOfArity ``Prod.fst 3 || e.isAppOfArity ``Prod.snd 3 ||
      e.isAppOfArity ``Subtype.val 3 || e.isAppOfArity ``Sum.elim 6 ||
      e.isAppOfArity ``VerexIR.Proofs.Denote.withSome 5)
  -- One unfolding, its matches on constructors reduced: a step of the
  -- definition, as its equations state it, never a match left stuck.
  let step (sub : Lean.Expr) : MetaM (Option Lean.Expr) := do
    let some unfolded ← unfoldDefinition? sub | return none
    let reduced ← whnfCore unfolded
    if reduced.getAppFn.isMVar then return none
    if let .const name _ := reduced.getAppFn then
      if (← isMatcher name) || name.isStr && name.getString! == "casesOn" then return none
    return some reduced
  Meta.transform e (post := fun sub => do
    if computation sub then
      let decisive := if sub.isAppOfArity ``Option.map 4 then sub.getArg! 3
        else if sub.isAppOfArity ``Option.bind 4 ||
            sub.isAppOfArity ``VerexIR.Proofs.Denote.withSome 5 then sub.getArg! 2
        else if sub.isAppOfArity ``Option.getD 3 then sub.getArg! 1
        else sub.appArg!
      if constructor decisive then
        if let some reduced ← step sub then return .visit reduced
    if let .proj _ _ inner := sub then
      if constructor inner then return .visit (← whnfCore sub)
    if let .reduced value ← reduceMatcher? sub then return .visit value
    return .continue)

/-- Whether two evaluated terms are equal where literals and constructors
decide it: distinct literals or constructors differ, and equal constructors
are equal where their fields are.  Each side is read through the
annotations the elaborator leaves on it (`mdata`), which an evaluated
observation carries and which are not part of the term. -/
partial def decidedEquality (a b : Lean.Expr) : Option Bool :=
  let constructor (e : Lean.Expr) : Option (Name × Array Lean.Expr) :=
    let e := e.consumeMData
    let fields (n : Nat) := e.getAppArgs.extract (e.getAppNumArgs - n) e.getAppNumArgs
    match e.getAppFn with
    | .const name _ =>
        if (name == ``Sum.inl || name == ``Sum.inr) && e.getAppNumArgs == 3 then some (name, fields 1)
        else if name == ``Option.some && e.getAppNumArgs == 2 then some (name, fields 1)
        else if name == ``Prod.mk && e.getAppNumArgs == 4 then some (name, fields 2)
        -- A certified value: its row, the certificate being a proposition
        -- no shaping reads (`NTy.carrier`).
        else if name == ``Subtype.mk && e.getAppNumArgs == 4 then some (name, fields 1)
        else if (name == ``Option.none && e.getAppNumArgs == 1) || name == ``PUnit.unit ||
          name == ``Bool.true || name == ``Bool.false then some (name, #[])
        else none
    | _ => none
  if a == b then some true else
  match a.consumeMData, b.consumeMData with
  | .lit x, .lit y => some (x == y)
  | _, _ =>
    match constructor a, constructor b with
    | some (x, xs), some (y, ys) =>
        if x != y then some false
        else (xs.zip ys).foldl (init := some true) fun known (x, y) =>
          match known, decidedEquality x y with
          | some false, _ | _, some false => some false
          | some true, some true => some true
          | _, _ => none
    | _, _ => if a == b then some true else none

/-- The value of an evaluated observation (`reduceVariantComputations`): its
equations decided (`decidedEquality`) and folded over the connectives in
three values; `none` where it reads more. -/
partial def decidedValue (e : Lean.Expr) : Option Bool :=
  let both (a b : Option Bool) (f : Bool → Bool → Bool) : Option Bool :=
    match a, b with
    | some a, some b => some (f a b)
    | _, _ => none
  -- A term the elaborator annotated is the term it annotates: an
  -- observation carries its source position (`mdata`), and a decision that
  -- read the annotation as the head would see a constructor nowhere.
  let e := e.consumeMData
  if e.isAppOfArity ``VerexIR.Proofs.Obligation 4 then decidedValue e.appArg!
  else if e.isConstOf ``True then some true
  else if e.isConstOf ``False then some false
  else if e.isAppOfArity ``Not 1 then (decidedValue e.appArg!).map (!·)
  else if e.isAppOfArity ``And 2 then
    let (a, b) := (decidedValue (e.getArg! 0), decidedValue (e.getArg! 1))
    if a == some false || b == some false then some false else both a b (· && ·)
  else if e.isAppOfArity ``Or 2 then
    let (a, b) := (decidedValue (e.getArg! 0), decidedValue (e.getArg! 1))
    if a == some true || b == some true then some true else both a b (· || ·)
  else if e.isAppOfArity ``Iff 2 then
    both (decidedValue (e.getArg! 0)) (decidedValue (e.getArg! 1)) (· == ·)
  else if e.isArrow then
    let (a, b) := ((decidedValue e.bindingDomain!).map (!·), decidedValue e.bindingBody!)
    if a == some true || b == some true then some true else both a b (· || ·)
  else if e.isAppOfArity ``Eq 3 then decidedEquality (e.getArg! 1) (e.getArg! 2)
  else none

/-- A proposition without the obligation markers it carries: `decidedValue`
reads through a marker, and the simp that must close a case refuted by a
marked fact does not, `Obligation` being a plain definition. -/
private partial def unmarkedObligation (type : Lean.Expr) : Lean.Expr :=
  if type.isAppOfArity ``VerexIR.Proofs.Obligation 4 then
    unmarkedObligation (type.getArg! 3)
  else type

/-- The cases of destructuring `local_`, outer or along its whole carrier,
that its observations (`isVariantObservation`) leave: in each case its
observations are evaluated in place (`reduceVariantComputations`), and one
that decides false (`decidedValue`) closes the case by `decide`. -/
def prunedCases (goals : Array MVarId) (local_ : FVarId) (outer : Bool)
    (decide : Simp.Context × Simp.SimprocsArray) : TacticM (Array MVarId) :=
  goals.flatMapM fun goal => goal.withContext do
    unless (← getLCtx).contains local_ do return #[goal]
    let before := (← getLCtx).foldl (init := ({} : FVarIdSet)) fun known decl =>
      known.insert decl.fvarId
    let cases ← if outer then destructureOuterVariant goal local_
      else destructureCarrier goal local_
    if cases.size > 1 then
      countStructure fun counts => { counts with variantSplits := counts.variantSplits + 1 }
    cases.filterMapM fun case => case.withContext do
      for hyp in (← case.getNondepPropHyps).filter (!before.contains ·) do
        let type ← instantiateMVars (← hyp.getType)
        unless isVariantObservation type do continue
        let value ← reduceVariantComputations type
        unless decidedValue value == some false do continue
        let saved ← saveState
        -- The refuted observation without its marker (`unmarkedObligation`).
        let evaluated ← case.replaceLocalDeclDefEq hyp (unmarkedObligation value)
        let (result, _) ← simpGoal evaluated decide.1 decide.2 (simplifyTarget := false)
          (fvarIdsToSimp := #[hyp])
        if result.isNone then return none
        saved.restore
      return some case

/-- Whether a term holds what a variant split's case computes
(`reduceVariantComputations`, `variant_decision`): a tag, payload projection,
selection or field read
of a constructor, or an equation of two constructors or two literals. -/
def variantRedex (e : Lean.Expr) : Bool :=
  let constructor (e : Lean.Expr) : Bool :=
    e.isAppOf ``Sum.inl || e.isAppOf ``Sum.inr || e.isAppOf ``Prod.mk || e.isConstOf ``PUnit.unit ||
      e.isAppOf ``Subtype.mk || e.isAppOf ``VerexIR.Proofs.Denote.NTy.ofDeepHolds ||
      e.isAppOf ``Option.some || e.isAppOf ``Option.none
  let literal (e : Lean.Expr) : Bool := e.isLit
  (e.find? fun sub =>
    (match sub with | .proj _ _ value => constructor value | _ => false) ||
    (sub.isAppOfArity ``VerexIR.Proofs.Denote.withSome 5 && constructor (sub.getArg! 2)) ||
    ((sub.isAppOf ``VerexIR.Proofs.Denote.variantName ||
        sub.isAppOf ``VerexIR.Proofs.Denote.variantPlainName ||
        sub.isAppOf ``VerexIR.Proofs.Denote.variantPlainNameOf ||
      sub.isAppOf ``VerexIR.Proofs.Denote.Which.project? ||
      sub.isAppOf ``VerexIR.Proofs.Denote.Which.projectCertified? ||
      sub.isAppOf ``VerexIR.Proofs.Denote.Choices.select? ||
      sub.isAppOf ``VerexIR.Proofs.Denote.Var.select ||
      sub.isAppOf ``VerexIR.Proofs.Denote.Var.selectPlain ||
      sub.isAppOf ``VerexIR.Proofs.Denote.variantPayload ||
      sub.isAppOfArity ``Subtype.val 3) && sub.isApp && constructor sub.appArg!) ||
    (sub.isAppOfArity ``Eq 3 &&
      ((constructor (sub.getArg! 1) && constructor (sub.getArg! 2)) ||
        (literal (sub.getArg! 1) && literal (sub.getArg! 2))))).isSome

/-- Each case's hypotheses that mention a destructured local, rewritten by
`rewrite` (the goal too, with `target`). A case the rewriting refutes
closes. -/
def rewriteCases (goal : MVarId) (cases : Array MVarId)
    (rewrite : Simp.Context × Simp.SimprocsArray) (target : Bool) : TacticM (Array MVarId) := do
  let before ← goal.withContext do
    return (← getLCtx).foldl (init := ({} : FVarIdSet)) fun known decl => known.insert decl.fvarId
  let mut remaining := #[]
  for case in cases do
    let rewritten := (← case.getNondepPropHyps).filter (!before.contains ·)
    let saved ← saveState
    try
      match ← simpGoal case rewrite.1 rewrite.2 (simplifyTarget := target)
          (fvarIdsToSimp := rewritten) with
      | (none, _) => pure ()
      | (some (_, next), _) => remaining := remaining.push next
    catch _ =>
      saved.restore
      remaining := remaining.push case
  return remaining

/-- Each case's hypotheses that hold a computation of the destructured
constructors (`variantRedex`), and its goal with `target`, computed: evaluated
in place (`reduceVariantComputations`), then simplified by `decide` where a
decision remains, such as an equation of two literals. A case the decisions
refute closes. -/
def computeCases (goal : MVarId) (cases : Array MVarId)
    (decide : Simp.Context × Simp.SimprocsArray) (target : Bool) : TacticM (Array MVarId) := do
  let before ← goal.withContext do
    return (← getLCtx).foldl (init := ({} : FVarIdSet)) fun known decl => known.insert decl.fvarId
  cases.filterMapM fun case => do
    let evaluated ← case.withContext do
      let mut current := case
      for hyp in (← case.getNondepPropHyps).filter (!before.contains ·) do
        let type ← instantiateMVars (← hyp.getType)
        unless variantRedex type do continue
        let value ← reduceVariantComputations type
        if value != type then current ← current.replaceLocalDeclDefEq hyp value
      if target then
        let type ← instantiateMVars (← current.getType)
        if variantRedex type then
          let value ← reduceVariantComputations type
          if value != type then current ← current.replaceTargetDefEq value
      return current
    evaluated.withContext do
      let decided ← ((← evaluated.getNondepPropHyps).filter (!before.contains ·)).filterM
        fun hyp => return variantRedex (← instantiateMVars (← hyp.getType))
      let simplifyTarget := target && variantRedex (← instantiateMVars (← evaluated.getType))
      if decided.isEmpty && !simplifyTarget then return some evaluated
      let saved ← saveState
      try
        match ← simpGoal evaluated decide.1 decide.2 (simplifyTarget := simplifyTarget)
            (fvarIdsToSimp := decided) with
        | (none, _) => return none
        | (some (_, next), _) => return some next
      catch _ =>
        saved.restore
        return some evaluated

/-- Destructure requested variants, reducing the observations of variants
before expanding the next local (`prunedCases`). Payloads need
normalization only in the surviving cases; with `computation`, only their
variant computations (`variant_computation`) where they hold one, the leaf
normalizing the rest where it needs to. With `outer`, only the requested
variant and its row are taken apart; with `target`, the goal is rewritten
as well. The surviving cases. -/
def splitVariantCases (goal : MVarId) (tested : Array FVarId) (outer : Bool)
    (computation := false) (target := computation) : TacticM (Array MVarId) := do
  -- Built while the goal is open: destructuring assigns it.
  let (ctx, simprocs) ← normalization
  let observed ← if computation then variantDecision else do
    pure (← Simp.mkContext { ctx.config with failIfUnchanged := false } ctx.simpTheorems
      ctx.congrTheorems, simprocs)
  let mut goals := #[goal]
  for local_ in tested do
    let before := goals.size
    goals ← prunedCases goals local_ outer observed
    if verex.solverDebug.get (← getOptions) then
      let name ← goal.withContext do
        pure ((← getLCtx).find? local_ |>.map (·.userName.toString) |>.getD "?")
      IO.println s!"    destructured {name}: {before} → {goals.size} cases"
  if goals == #[goal] then return goals
  if computation then computeCases goal goals observed target
  else rewriteCases goal goals (ctx, simprocs) target

/-- `splitVariantCases`, its cases the goals. -/
def splitVariants (goal : MVarId) (tested : Array FVarId) (outer : Bool) : TacticM Unit := do
  let cases ← splitVariantCases goal tested outer
  if cases == #[goal] then throwError "no tested local has variants to split"
  replaceMainGoal cases.toList

/-- Split a leaf on the variants of the locals it tests (`testedLocals`),
along their carriers: in each case the tests compute. An authored
preparation's split (`verex_prepare`). -/
elab "verex_split_variants" : tactic => focusGoal do
  let goal ← getMainGoal
  let tested ← testedLocals goal
  if tested.isEmpty then throwError "the leaf tests no local's variant"
  splitVariants goal tested false

/-- Each case of a value of `type` in its outer split
(`destructureOuterVariant`) as a term over a fresh payload, passed to `k` in
the payload's context: one constructor of its variant row each. -/
private partial def forOuterCases (type : Lean.Expr) (k : Lean.Expr → MetaM α) :
    MetaM (Array α) := do
  let type ← whnfD type
  if type.isConstOf ``Empty || type.isAppOf ``PEmpty then return #[]
  let .const ``Sum levels := type.getAppFn | return #[← withLocalDeclD `value type k]
  unless type.getAppNumArgs == 2 do return #[← withLocalDeclD `value type k]
  let (left, right) := (type.getArg! 0, type.getArg! 1)
  let first ← withLocalDeclD `payload left fun payload =>
    k (mkApp3 (.const ``Sum.inl levels) left right payload)
  let rest ← forOuterCases right fun rest => k (mkApp3 (.const ``Sum.inr levels) left right rest)
  return #[first] ++ rest

/-- How many cases of an outer split of a value of `type` its observations
leave: each observation, abstracted over the value, evaluated at each of its
constructors over a fresh payload (`reduceVariantComputations`) and decided
(`decidedValue`), a case surviving unless one decides false. The whole context
is split only when the count makes the split a rewrite. -/
private def survivingCases (type : Lean.Expr) (observations : Array Lean.Expr) : MetaM Nat := do
  let surviving ← forOuterCases type fun term => do
    for observation in observations do
      let value ← reduceVariantComputations (observation.instantiate1 term)
      if decidedValue value == some false then return false
    return true
  return (surviving.filter id).size

/-- Whether a term is a constructor of a variant, option, pair or unit. -/
private def variantConstructor (e : Lean.Expr) : Bool :=
  e.isAppOf ``Sum.inl || e.isAppOf ``Sum.inr || e.isAppOf ``Prod.mk || e.isConstOf ``PUnit.unit ||
      e.isAppOf ``Subtype.mk || e.isAppOf ``VerexIR.Proofs.Denote.NTy.ofDeepHolds ||
    e.isAppOf ``Option.some || e.isAppOf ``Option.none

/-- The value tested by a tag test or payload projection, other than a local
(`testedIn`), a constructor or a conditional: a map's value at a key, a field
of a read. A conditional's branches are the constructors; its split decides
it. -/
private def testedTerm? (env : Environment) (e : Lean.Expr) : Option Lean.Expr :=
  if let some read := variantReadingSubject? e then
    let subject := read.consumeMData
    let constructor := variantConstructor subject
    let conditional := subject.isAppOf ``ite || subject.isAppOf ``dite ||
      isMatcherAppCore env subject
    if subject.isFVar || subject.hasLooseBVars || constructor || conditional then none
    else some subject
  else none

/-- A hypothesis, abstracted over a tested term, equating the term to a
constructor: it determines the term's variant as a tag test does. -/
private def statesConstructor (abstracted : Lean.Expr) : Bool :=
  let constructor (e : Lean.Expr) := e.isAppOf ``Sum.inl || e.isAppOf ``Sum.inr
  abstracted.isAppOfArity ``Eq 3 &&
    ((abstracted.getArg! 1 == .bvar 0 && constructor (abstracted.getArg! 2)) ||
      (abstracted.getArg! 2 == .bvar 0 && constructor (abstracted.getArg! 1)))

/-- Whether a hypothesis can determine a tested term's variant, read before
abstracting: an observation (`isVariantObservation`) or an equation with a
constructor side (`statesConstructor`). -/
private def mayDetermineVariant (type : Lean.Expr) : Bool :=
  let constructor (e : Lean.Expr) := e.isAppOf ``Sum.inl || e.isAppOf ``Sum.inr
  isVariantObservation type ||
    (type.isAppOfArity ``Eq 3 && (constructor (type.getArg! 1) || constructor (type.getArg! 2)))

/-- `kabstract` of every occurrence of `p` in `e`, each shared subterm visited
once and the heartbeat allowance checked at each: `kabstract` walks a term as
a tree, which for a hypothesis sharing large values, such as literal vectors
and their proofs, is far larger than the term and is never interrupted. -/
def kabstractShared (e p : Lean.Expr) : MetaM Lean.Expr := do
  let e ← instantiateMVars e
  if p.isFVar then return e.abstract #[p]
  let head := p.toHeadIndex
  let arity := p.headNumArgs
  let visited ← IO.mkRef ({} : Std.HashMap (Lean.Expr × Nat) Lean.Expr)
  let rec visit (e : Lean.Expr) (offset : Nat) : MetaM Lean.Expr := do
    if let some result := (← visited.get)[(e, offset)]? then return result
    checkSystem "kabstract"
    let children : Unit → MetaM Lean.Expr := fun _ => do
      match e with
      | .app f a => return e.updateApp! (← visit f offset) (← visit a offset)
      | .mdata _ b => return e.updateMData! (← visit b offset)
      | .proj _ _ b => return e.updateProj! (← visit b offset)
      | .letE _ t v b _ =>
          return e.updateLetE! (← visit t offset) (← visit v offset) (← visit b (offset + 1))
      | .lam _ d b _ => return e.updateLambdaE! (← visit d offset) (← visit b (offset + 1))
      | .forallE _ d b _ => return e.updateForallE! (← visit d offset) (← visit b (offset + 1))
      | e => return e
    let candidate := !e.hasLooseBVars && e.toHeadIndex == head && e.headNumArgs == arity
    let result ← do
      if candidate then
        if ← isDefEq e p then pure (mkBVar offset) else children ()
      else children ()
    visited.modify (·.insert (e, offset) result)
    return result
  visit e 0

/-- The tested terms (`testedTerm?`) whose variant the leaf's observations
determine, each generalized to a local and destructured as a local is: a
value read from a map tests no local, however its variant is known. The goal
left, `none` where the observations refute the leaf. Fails where no term is
determined. -/
private def determinedTerms (goal : MVarId) (decide : Simp.Context × Simp.SimprocsArray) :
    TacticM (Option MVarId) := do
  let mut goal := goal
  let mut progress := false
  let mut stuck : Array Lean.Expr := #[]
  let mut changed := true
  while changed do
    changed := false
    let (hyps, target) ← goal.withContext do
      let hyps ← goal.getNondepPropHyps
      return (← hyps.mapM fun hyp => do return (hyp, ← instantiateMVars (← hyp.getType)),
        ← instantiateMVars (← goal.getType))
    let found ← IO.mkRef (#[] : Array Lean.Expr)
    let env ← getEnv
    let collect (e : Lean.Expr) : TacticM Unit := e.forEach fun sub => do
      if let some subject := testedTerm? env sub then
        found.modify fun known =>
          if known.contains subject || stuck.contains subject then known else known.push subject
    collect target
    for (_, type) in hyps do
      if isVariantObservation type then collect type
    for subject in ← found.get do
      -- A term its computations already reduce to a constructor is decided.
      if variantConstructor (← goal.withContext do reduceVariantComputations subject) then
        stuck := stuck.push subject
        continue
      -- Each hypothesis reading the term, abstracted over it at reducible
      -- instances; an observation also at default transparency where that
      -- finds nothing (a callee's facts spell the term's types at its
      -- family). The readers are restated of the term's value, so that a
      -- fact guarded by a test of the term computes in its case.
      let readers ← goal.withContext do
        hyps.filterMapM fun (hyp, type) => do
          -- Most hypotheses do not read this term. Avoid definitional
          -- abstraction over their large types; observations still get the
          -- transparent fallback for terms spelled at another family.
          if (type.find? (· == subject)).isSome then
            let abstracted ← withTransparency .instances do kabstractShared type subject
            if abstracted.hasLooseBVars then return some (hyp, type, abstracted)
          unless mayDetermineVariant type do return none
          let abstracted ← withTransparency .default do kabstractShared type subject
          return if abstracted.hasLooseBVars &&
              (isVariantObservation type || statesConstructor abstracted) then
            some (hyp, type, abstracted) else none
      let type ← goal.withContext do instantiateMVars (← inferType subject)
      -- A raw projection may also occur in a dependent proof's expected
      -- type, through its certified owner. Such a reader cannot be
      -- transported independently of that owner.
      let readers ← goal.withContext do
        readers.filterM fun (_, _, abstracted) =>
          isTypeCorrect (Lean.mkLambda `x .default type abstracted)
      let observations := readers.filter fun (_, type, abstracted) =>
        isVariantObservation type || statesConstructor abstracted
      if ← (pure observations.isEmpty <||>
          goal.withContext do return (← survivingCases type (observations.map (·.2.2))) > 1) then
        stuck := stuck.push subject
        continue
      let saved ← saveState
      -- The term generalized in the goal, each reader restated of the local
      -- by its abstraction: generalizing the whole context compares every
      -- term of it with the term at default transparency.
      let value ← mkFreshUserName `value
      let (local_, generalized) ← goal.withContext do
        let (introduced, next) ← goal.generalize
          #[{ expr := subject, xName? := some value, hName? := some (← mkFreshUserName `value_eq) }]
          (transparency := .instances)
        let (some local_, some equation) := (introduced[0]?, introduced[1]?)
          | throwError "the tested term is not generalized"
        next.withContext do
          let restated ← readers.mapM fun (hyp, _, abstracted) => do
            let motive := Lean.mkLambda `x .default type abstracted
            return { userName := ← hyp.getUserName, type := abstracted.instantiate1 (.fvar local_),
                     value := ← mkEqNDRec motive (.fvar hyp) (.fvar equation) : Hypothesis }
          let (_, next) ← next.assertHypotheses restated
          return (local_, ← next.tryClearMany (readers.map (·.1)))
      let cases ← prunedCases #[generalized] local_ true decide
      match ← computeCases generalized cases decide true with
      | #[] => return none
      | #[next] =>
          goal := next
          progress := true
          changed := true
          break
      | _ =>
          saved.restore
          stuck := stuck.push subject
  unless progress do throwError "no observation determines a tested term"
  return some goal

/-- The variants the leaf's observations determine: a local an observation
(`isVariantObservation`) tests whose outer split leaves one case
(`survivingCases`), that case in place of the goal, its computations
computed (`computeCases`); to a fixpoint, as a field so taken out
may be determined in turn. A rewrite, not a split. Fails when no local is
determined. -/
elab "verex_determined_variants" : tactic => focusGoal do
  let mut goal ← getMainGoal
  -- Built while the goal is open: destructuring assigns it.
  let decide ← variantDecision
  let mut progress := false
  let saved ← saveState
  try
    match ← determinedTerms goal decide with
    | none =>
        replaceMainGoal []
        return
    | some next =>
        goal := next
        progress := true
  catch failure =>
    saved.restore
    if verex.solverDebug.get (← getOptions) then
      IO.println (← (m!"    determined terms: {failure.toMessageData}").toString)
  -- Only an observation of a variant determines one.
  let observed ← goal.withContext do
    (← goal.getNondepPropHyps).anyM fun hyp => do
      return isVariantObservation (← instantiateMVars (← hyp.getType))
  unless observed || progress do throwError "no observation determines a variant"
  let mut stuck : Array FVarId := #[]
  let mut changed := true
  while changed do
    changed := false
    -- Each observed local with the observations testing it.
    let candidates ← goal.withContext do
      let mut found : Array (FVarId × Array FVarId) := #[]
      for hyp in ← goal.getNondepPropHyps do
        let type ← instantiateMVars (← hyp.getType)
        unless isVariantObservation type do continue
        for x in ← testedIn type #[] do
          if stuck.contains x then continue
          found := match found.findIdx? (·.1 == x) with
            | some i => found.modify i fun (x, hyps) => (x, hyps.push hyp)
            | none => found.push (x, #[hyp])
      return found
    for (local_, observations) in candidates do
      let surviving ← goal.withContext do
        let types ← observations.mapM fun hyp => do
          return (← instantiateMVars (← hyp.getType)).abstract #[.fvar local_]
        survivingCases (← instantiateMVars (← local_.getType)) types
      unless surviving ≤ 1 do
        stuck := stuck.push local_
        continue
      let saved ← saveState
      match ← prunedCases #[goal] local_ true decide with
      | #[] =>
          -- The observations refute the leaf.
          replaceMainGoal []
          return
      | #[case] =>
          if case == goal then
            stuck := stuck.push local_
            continue
          match ← computeCases goal #[case] decide true with
          | #[next] =>
              goal := next
              progress := true
              changed := true
              break
          | _ =>
              replaceMainGoal []
              return
      | _ =>
          saved.restore
          stuck := stuck.push local_
  unless progress do throwError "no observation determines a variant"
  replaceMainGoal [goal]

/-- A goal or a fact that already holds a computation of a constructor
(`variantRedex`), computed where it stands (`reduceVariantComputations`)
and then decided (`variant_decision`).

A value made where the leaf stands and read at another frame is a
constructor under its readers with no local to split, so none of the steps
that compute such a reading reaches it: they all compute the cases a split
left (`prunedCases`, `computeCases`) or a term a split generalized
(`determinedTerms`).  A pack obligation of a declaration's data invariant
is the case in point: the value is the constructor the obligation is about,
and the obligation reads its name.  A read's own fact is the same case on
the hypothesis side: the clauses of a value the path determines are stated
of the constructor the path read (`certifiedClauses?`), and the clause
reads its name.  The reduction is the same one, applied to the goal and to
the facts themselves; it declines on one test of each, which is what every
leaf without such a computation pays. -/
elab "verex_variant_redex" : tactic => focusGoal do
  let goal ← getMainGoal
  let decide ← variantDecision
  let (reduced, facts, retarget) ← goal.withContext do
    let mut current := goal
    let mut facts := #[]
    for hyp in ← goal.getNondepPropHyps do
      let type ← instantiateMVars (← hyp.getType)
      unless variantRedex type do continue
      let computed ← reduceVariantComputations type
      if computed == type then continue
      current ← current.replaceLocalDeclDefEq hyp computed
      facts := facts.push hyp
    let target ← instantiateMVars (← current.getType)
    unless variantRedex target do return (current, facts, false)
    let computed ← reduceVariantComputations target
    if computed == target then return (current, facts, false)
    return (← current.replaceTargetDefEq computed, facts, true)
  if facts.isEmpty && !retarget then
    throwError "no goal or fact computes a constructor"
  let (result, _) ← simpGoal reduced decide.1 decide.2 (simplifyTarget := retarget)
    (fvarIdsToSimp := facts)
  replaceMainGoal (result.map (·.2) |>.toList)
  -- Computed payload readings can expose an equation between whole values.
  -- Substitute it before the leaf reads projections of those values again.
  if result.isSome && !facts.isEmpty then substVars true

/-- The walk's split of a goal on the variants the goal itself tests, outer
and one local at a time: those only its hypotheses test are the leaf's to
split, after its deciders (`verex_prepared_case`). -/
elab "verex_split_goal_variants" : tactic => focusGoal do
  let goal ← getMainGoal
  let some first := (← rankedTestedLocals goal (goalOnly := true))[0]?
    | throwError "the goal tests no local's variant"
  splitVariants goal #[first] true

/-- Split a leaf goal into its cases: a conjunction into its parts, and a
binder, implication, or negation into a hypothesis, until the goal is
atomic.  Each case then carries the equations of its branch, which a
rewriting pass turns into closed arithmetic. -/
partial def splitGoal (goal : MVarId) : MetaM (List MVarId) := goal.withContext do
  let target ← whnfR (← instantiateMVars (← goal.getType))
  if target.isAppOfArity ``And 2 then
    let subgoals ← goal.apply (mkConst ``And.intro)
    return (← subgoals.mapM splitGoal).flatten
  if target.isAppOfArity ``Not 1 then
    let (_, next) ← (← goal.change (mkForall `h .default (target.getArg! 0) (mkConst ``False))).intro1
    return ← splitGoal next
  if target.isForall then
    -- A quantified variable keeps its name, for an authored proof.
    let (_, next) ← goal.intro1P
    return ← splitGoal next
  return [goal]

elab "verex_split_goal" : tactic => focusGoal do
  if (← getGoals).isEmpty then return
  let goals ← (← getMainGoal).withContext (splitGoal (← getMainGoal))
  replaceMainGoal goals

/-- What a saturation round can change of a goal: its target and its
hypotheses. The hypotheses are compared as a collection: the rewriting
pass restates the hypotheses it changes after the others, so a round that
proves nothing new may still reorder them. -/
def goalContent (goal : MVarId) : MetaM (Lean.Expr × Array Lean.Expr) := goal.withContext do
  let mut hypotheses := #[]
  for decl in ← getLCtx do
    unless decl.isImplementationDetail do
      hypotheses := hypotheses.push (← instantiateMVars decl.type)
  return (← instantiateMVars (← goal.getType), hypotheses.qsort (·.lt ·))

/-- A goal some hypothesis states, or reflexivity, up to reducible
unfolding. Plain `assumption` compares at default transparency, which
unfolds large verification conditions at length before failing on the
leaves nothing closes. Source-location markers do not change a proposition:
compare their contents, retaining the markers when the attempt fails. -/
elab "verex_assumption" : tactic => focusGoal do
  let goal ← getMainGoal
  if ← closeReflexive goal then
    replaceMainGoal []
    return
  goal.withContext do
    let statement (type : Lean.Expr) : Lean.Expr := Id.run do
      let mut type := type.consumeMData
      while type.isAppOfArity ``VerexIR.Proofs.Obligation 4 do
        type := (type.getArg! 3).consumeMData
      return type
    let target := statement (← instantiateMVars (← goal.getType))
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let supplied := statement (← instantiateMVars decl.type)
      if ← withReducible (isDefEq target supplied) then
        goal.assign decl.toExpr
        replaceMainGoal []
        return
      -- Symmetry is still a direct lookup of the same equation. Keep its
      -- sides syntactic, so unrelated equalities trigger no search.
      if let some (_, left, right) := target.eq? then
        if let some (type, otherLeft, otherRight) := supplied.eq? then
          if left == otherRight && right == otherLeft then
            goal.assign (mkApp4 (mkConst ``Eq.symm [← getLevel type])
              type otherLeft otherRight decl.toExpr)
            replaceMainGoal []
            return
    throwError "no hypothesis states the goal"

/-- Use marked contract facts before decomposing the rest of a leaf's
context. Ordinary assumption lookups remain a direct comparison. -/
elab "verex_stated" : tactic => focusGoal do
  unless ← StatedFacts.close (← getMainGoal) do
    throwError "the stated logical facts do not close the goal"
  replaceMainGoal []

/-- A type up to the names of its free variables: the variables abstracted
in order of occurrence, with their types, which the normalization may
read, abstracted the same way. -/
private def normalKey (type : Lean.Expr) : MetaM (Lean.Expr × Array Lean.Expr) := do
  let variables := (collectFVars {} type).fvarIds.map mkFVar
  let types ← variables.mapM fun fvar => do
    return (← instantiateMVars (← inferType fvar)).abstract variables
  return (type.abstract variables, types)

/-- Whether a hypothesis's normal form depends on its context: the
conditional rules of the pass (a conditional's branch, a truncating
division's form, a cut integer's named local) read the context, so such a
hypothesis may normalize further where more is known. -/
private def contextDependentHeadName (name : Name) : Bool :=
  name == ``ite || name == ``dite || name == ``Int.tmod || name == ``Int.tdiv ||
    name == ``VerexIR.SpecInt.ofInt || name == ``VerexIR.SpecInt.ofSize ||
    name == ``VerexIR.SpecVector.append || name == ``VerexIR.SpecVector.push

private def contextDependent (type : Lean.Expr) : Bool :=
  (type.find? fun e => match e.getAppFn with
    | .const name _ => contextDependentHeadName name
    | _ => false).isSome

/-! ## Fixed points across normalizations

A term a simp context left unchanged is unchanged wherever it recurs, as
long as its traversal read nothing of the local context: no discharger
call, no simproc searching the hypotheses, no subterm taken as normal
without traversal. Such terms are recorded per target and context stamp,
and a later traversal of the context skips them (`.done`, no proof).
Likewise the result of a dsimp call that read nothing of the context (binder
domains, mostly types, recur on every visit of their binder); a subterm a
later call takes as normal is normal, so the result stands there too. -/

/-- Whether simp may read the local context at a term: a
`contextDependent` shape, or one of a simproc searching the hypotheses
(`existsNamedValue`, `givenClosureFrame`, `invokeGenericClosure`). -/
private def contextReadingHead (e : Lean.Expr) : Bool :=
  match e.getAppFn with
  | .const name _ =>
      contextDependentHeadName name || name == ``VerexIR.Proofs.Spec.given ||
        name == ``VerexIR.Proofs.Denote.closureMeaning ||
        (name == ``Exists && e.getAppNumArgs == 2 && (e.getArg! 0).isAppOf ``VerexIR.SpecInt)
  | _ => false

/-- A term being traversed: simp's recursion depth there and the context
reads before it. -/
private structure FixedPointFrame where
  term : Lean.Expr
  depth : Nat
  reads : Nat
  deriving Inhabited

/-- A traversal's view of the fixed points of one simp context. -/
structure FixedPoints where
  /-- The fixed points of the context. -/
  known : IO.Ref (Std.HashSet Lean.Expr)
  /-- The context reads so far. -/
  reads : IO.Ref Nat
  frames : IO.Ref (Array FixedPointFrame)
  /-- The terms whose traversal read the context; simp's own cache returns
  them without a traversal. -/
  tainted : IO.Ref (Std.HashSet Lean.Expr)
  /-- Hits, misses, and terms recorded. -/
  counts : IO.Ref (Array Nat)
  /-- The results of the context's dsimp calls that read no local context. -/
  dsimped : IO.Ref (Std.HashMap Lean.Expr Lean.Expr)
  /-- Whether a recorded dsimp call is running. -/
  dsimping : IO.Ref Bool
  /-- Whether the running dsimp call met a context-reading shape. -/
  dsimpReads : IO.Ref Bool

/-- The stamp of the walk's normalization (`normalization`). -/
def walkFixedPoints : Option Name := some `walk

/-- The fixed points of the context stamped `stamp`, if any. -/
def FixedPoints.new? (stamp : Option Name) : CoreM (Option FixedPoints) := do
  let some stamp := stamp | return none
  let state ← targetState
  let known ← match (← state.fixedPoints.get)[stamp]? with
    | some known => pure known
    | none => do
      let known ← IO.mkRef {}
      state.fixedPoints.modify (·.insert stamp known)
      pure known
  let dsimped ← match (← state.dsimpResults.get)[stamp]? with
    | some dsimped => pure dsimped
    | none => do
      let dsimped ← IO.mkRef {}
      state.dsimpResults.modify (·.insert stamp dsimped)
      pure dsimped
  let reads ← IO.mkRef 0
  let frames ← IO.mkRef #[]
  let tainted ← IO.mkRef {}
  let dsimping ← IO.mkRef false
  let dsimpReads ← IO.mkRef false
  let counts := state.fixedPointCounts
  return some { known, reads, frames, tainted, counts, dsimped, dsimping, dsimpReads }

/-- Whether a term is worth recording: no metavariables, not small. -/
private def fixedPointCandidate (e : Lean.Expr) : Bool :=
  !e.hasMVar && e.approxDepth ≥ 3

/-- End the traversals deeper than `depth`; those that read the context
are tainted. -/
private def FixedPoints.closeAbove (fp : FixedPoints) (depth : Nat) : IO Unit := do
  let mut frames ← fp.frames.swap #[]
  if frames.size > 0 && frames.back!.depth > depth then
    let reads ← fp.reads.get
    while frames.size > 0 && frames.back!.depth > depth do
      let frame := frames.back!
      frames := frames.pop
      if reads != frame.reads then fp.tainted.modify (·.insert frame.term)
  fp.frames.set frames

/-- Whether a child simp visits of `e` was tainted. -/
private def childTainted (tainted : Std.HashSet Lean.Expr) (e : Lean.Expr) : Bool :=
  match e with
  | .app f a => tainted.contains a || spine f
  | .forallE _ d b _ => tainted.contains d || (!b.hasLooseBVars && tainted.contains b)
  | .letE _ t v _ _ => tainted.contains t || tainted.contains v
  | .mdata _ b | .proj _ _ b => tainted.contains b
  | _ => false
where
  spine : Lean.Expr → Bool
    | .app f a => tainted.contains a || spine f
    | f => tainted.contains f

/-- `methods` skipping the recorded fixed points and recording new ones;
a term `assumed` holds is taken as normal, which taints its ancestors. -/
def FixedPoints.wrap (fp : FixedPoints) (methods : Simp.Methods)
    (assumed : Lean.Expr → SimpM Bool := fun _ => pure false) : Simp.Methods :=
  { methods with
    pre := fun e => do
      -- An atom reads no context in its own traversal.
      if e.isAtomic then
        if ← assumed e then
          fp.reads.modify (· + 1)
          return .done { expr := e }
        return ← methods.pre e
      let depth ← MonadRecDepth.getRecDepth
      fp.closeAbove (depth - 1)
      if fixedPointCandidate e then
        if (← fp.known.get).contains e then
          fp.counts.modify (·.modify 0 (· + 1))
          return .done { expr := e }
        fp.counts.modify (·.modify 1 (· + 1))
      let before ← fp.reads.get
      if contextReadingHead e then fp.reads.set (before + 1)
      if ← assumed e then
        fp.reads.modify (· + 1)
        return .done { expr := e }
      let step ← methods.pre e
      fp.closeAbove depth
      match step with
      | .continue _ => fp.frames.modify (·.push { term := e, depth, reads := before })
      | _ => if (← fp.reads.get) != before then fp.tainted.modify (·.insert e)
      return step
    dpre := fun e => do
      if ← assumed e then
        fp.reads.modify (· + 1)
        return .done e
      if !fixedPointCandidate e then
        return ← methods.dpre e
      if let some e' := (← fp.dsimped.get)[e]? then
        fp.counts.modify (·.modify 3 (· + 1))
        return .done e'
      if (← fp.known.get).contains e then
        fp.counts.modify (·.modify 3 (· + 1))
        return .done e
      if ← fp.dsimping.get then
        -- A context-reading shape keeps the enclosing call from recording.
        if contextReadingHead e then fp.dsimpReads.set true
        return ← methods.dpre e
      -- The outermost candidate of a dsimp call: run the call here, to see
      -- its result and whether it read the context.
      fp.counts.modify (·.modify 4 (· + 1))
      let before ← fp.reads.get
      fp.dsimpReads.set (contextReadingHead e)
      fp.dsimping.set true
      let e' ← try Simp.dsimp e finally fp.dsimping.set false
      if (← fp.reads.get) == before && !(← fp.dsimpReads.get) then
        fp.dsimped.modify (·.insert e e')
        fp.counts.modify (·.modify 5 (· + 1))
      return .done e'
    post := fun e => do
      if e.isAtomic then return ← methods.post e
      let depth ← MonadRecDepth.getRecDepth
      if contextReadingHead e then fp.reads.modify (· + 1)
      let step ← methods.post e
      fp.closeAbove depth
      let frames ← fp.frames.swap #[]
      unless frames.size > 0 && frames.back!.depth == depth do
        fp.frames.set frames
        return step
      let frame := frames.back!
      fp.frames.set frames.pop
      if (← fp.reads.get) != frame.reads then
        fp.tainted.modify (·.insert frame.term)
        return step
      let unchanged := match step with
        | .done r | .visit r | .continue (some r) => r.proof?.isNone && r.expr == e
        | .continue none => true
      if unchanged && fixedPointCandidate e && frame.term == e &&
          !childTainted (← fp.tainted.get) e then
        fp.known.modify (·.insert e)
        fp.counts.modify (·.modify 2 (· + 1))
      return step
    discharge? := fun condition => do
      fp.reads.modify (· + 1)
      methods.discharge? condition }

private def contextDischargeCore (context : LocalContext) (condition : Lean.Expr)
    (atoms : FVarIdSet) : MetaM (Option Lean.Expr) := do
  for decl in context do
    if decl.isImplementationDetail then continue
    if ← withReducible (isDefEq decl.type condition) then return some decl.toExpr
  -- Normalization does not search for structural contradictions. Enum tags,
  -- payload validity, and quantified properties need explicit facts or cases.
  -- Even an arithmetic side condition must not make omega split the unrelated
  -- contracts of every call that mentions the same map.
  unless arithmeticProposition condition do return none
  -- A nonnegativity condition of a certified unsigned value, structurally.
  if condition.isAppOfArity ``LE.le 4 && (condition.getArg! 2).int? == some 0 then
    if let some proof ← nonnegative? (condition.getArg! 3) then return some proof
  let proof ← mkFreshExprMVar condition
  try
    let some refutation ← proof.mvarId!.falseOrByContra | return some (← instantiateMVars proof)
    refutation.withContext do
      -- The facts connected to the condition through shared variables; the
      -- negated condition is stated through the assigned metavariable.
      let mut facts ← (← getLCtx).foldlM (init := #[]) fun found decl => do
        if decl.isImplementationDetail then return found
        let type ← instantiateMVars decl.type
        unless arithmeticProposition type do return found
        return found.push (decl.toExpr, (collectFVars {} type).fvarSet)
      -- A truncation to `Nat` is decided only with the sign of what it
      -- truncates, which a certified value's type carries and the context
      -- need not state: the nonnegativity of every certified value the
      -- condition and the connected facts read is offered with them
      -- (`nonnegative?`), as the reduction below a quotient pairs
      -- `verex_bounds` with omega.
      if (condition.find? (·.isAppOfArity ``Int.toNat 1)).isSome then
        let values ← IO.mkRef (#[] : Array Lean.Expr)
        let read (e : Lean.Expr) : MetaM Unit := e.forEach fun sub => do
          if (unsignedValue? sub).isSome || (signedValue? sub).isSome then
            values.modify fun known => if known.contains sub then known else known.push sub
        read condition
        for (fact, _) in facts do read (← instantiateMVars (← inferType fact))
        for value in ← values.get do
          if let some proof ← nonnegative? value then
            let type ← instantiateMVars (← inferType proof)
            facts := facts.push (proof, (collectFVars {} type).fvarSet)
      omegaHidden (connectedFacts facts atoms).toList refutation
    return some (← instantiateMVars proof)
  catch _ => return none

/-- A side decision under its budget: at most `sideDecisionHeartbeats`,
and no more than the enclosing computation has left. Failure restores any
speculative metavariable assignments. -/
private def withSideBudget {m : Type → Type} [Monad m] [MonadLiftT MetaM m]
    [MonadWithReaderOf Core.Context m] [MonadRuntimeException m] [MonadExcept Exception m]
    (decide : m (Option Lean.Expr)) : m (Option Lean.Expr) := do
  let enclosing ← (readThe Core.Context : MetaM _)
  let now ← (IO.getNumHeartbeats : MetaM Nat)
  let remaining := if enclosing.maxHeartbeats == 0 then sideDecisionHeartbeats
    else enclosing.initHeartbeats + enclosing.maxHeartbeats - now
  let budget := max 1 (min remaining sideDecisionHeartbeats)
  let saved ← (saveState : MetaM _)
  let result ← tryCatchRuntimeEx
    (withTheReader Core.Context
      (fun context => { context with initHeartbeats := now, maxHeartbeats := budget }) decide)
    fun failure => do
      if failure.isMaxHeartbeat then return none
      throw failure
  if result.isNone then (saved.restore : MetaM Unit)
  return result

/-- Normalization only asks for cheap side conditions. A large connected
context must not turn an undecided tag or payload test into an unbounded
arithmetic search. -/
private def boundedContextDischarge (context : LocalContext) (condition : Lean.Expr)
    (atoms : FVarIdSet) : MetaM (Option Lean.Expr) :=
  withSideBudget (contextDischargeCore context condition atoms)

/-- A condition in the normal form of the context, by its parts: a
conjunction part by part, as the context splits it. -/
private partial def dischargeParts (context : LocalContext) (condition : Lean.Expr) :
    MetaM (Option Lean.Expr) := do
  if condition.isConstOf ``True then return some (mkConst ``True.intro)
  if let some (left, right) := condition.and? then
    let some first ← dischargeParts context left | return none
    let some second ← dischargeParts context right | return none
    return some (mkApp4 (mkConst ``And.intro) left right first second)
  boundedContextDischarge context condition (collectFVars {} condition).fvarSet

/-- A premise a lemma states in a role's spelling, which the normalization
has rewritten in the hypotheses (`¬AbortsNewFrom` as its specification
form): the premise in the same normal form, matched by its parts. -/
private def normalizedDischarge (context : LocalContext) (condition : Lean.Expr) :
    SimpM (Option Lean.Expr) := withSideBudget do
  let normal ← Simp.simp condition
  if normal.expr == condition then return none
  let some proof ← dischargeParts context normal.expr | return none
  match normal.proof? with
  | some equation => return some (← mkEqMPR equation proof)
  | none => return some proof

/-- The discharger of the normalization's side conditions, over a leaf's
context: a condition about a variable simp introduced under a binder has no
fact in the context and is not attempted; otherwise a hypothesis states it
at reducible transparency, or omega proves it from the hypotheses connected
to it through shared variables, not from the whole context, whose size the
condition's cost would otherwise follow. A lemma's premise is matched in the
normal form the context has, so a lemma may state a role's own atom. -/
private def contextDischarge (context : LocalContext) (decided : IO.Ref (Std.HashMap Lean.Expr (Option Lean.Expr)))
    (arithmetic : Bool := true) :
    Simp.Discharge := fun condition => do
  let condition ← instantiateMVars condition
  if !arithmetic && arithmeticProposition condition then return none
  let atoms := (collectFVars {} condition).fvarSet
  -- A condition asked before in this pass, decided or not, is answered again;
  -- an answer whose proof reads a local of a binder simp entered holds only
  -- under that binder and is not kept.
  if let some known := (← decided.get)[condition]? then return known
  let remember (result : Option Lean.Expr) : MetaM Unit := do
    -- At the leaf's own context, outside any binder simp entered, every
    -- local a proof reads is the leaf's.
    let current ← getLCtx
    let atLeaf := current.decls.size == context.decls.size &&
      current.lastDecl.map (·.fvarId) == context.lastDecl.map (·.fvarId)
    let proofLocals ← match result, atLeaf with
      | some proof, false => pure (collectFVars {} (← instantiateMVars proof)).fvarIds
      | _, _ => pure #[]
    if proofLocals.all context.contains then decided.modify (·.insert condition result)
  -- A closed condition, such as a slot's position against its layout's
  -- count or a map lemma's precondition over literal vectors, is decided by
  -- the normalization's own lemmas.
  if atoms.isEmpty then
    let result ← Simp.dischargeDefault? condition
    remember result
    return result
  if atoms.any fun fvarId => !context.contains fvarId then return none
  let started ← IO.getNumHeartbeats
  let mut result ← boundedContextDischarge context condition atoms
  -- Arithmetic went to omega; any other condition is matched again in the
  -- normal form the context has: a lemma's premise instantiated at a literal
  -- is not in it.
  if result.isNone && !arithmeticProposition condition then
    result ← normalizedDischarge context condition
  remember result
  if verex.solverDebug.get (← getOptions) then
    let finished ← IO.getNumHeartbeats
    let shown ← if result.isSome then pure "" else do
      pure ((toString (← ppExpr condition)).replace "\n" " " |>.take 120).toString
    (← targetState).dischargeLog.modify fun (calls, cost, failed) =>
      (calls + 1, cost + (finished - started), if result.isSome then failed else failed.push shown)
  return result

/-- The normalization's discharger over a goal's context (`contextDischarge`),
for a normal form computed outside a leaf's normalization, such as a step's;
a step's states an arithmetic condition itself. -/
def goalDischarge (goal : MVarId) (arithmetic : Bool := true) : MetaM Simp.Discharge :=
  goal.withContext do
    return contextDischarge (← getLCtx) (← IO.mkRef {}) arithmetic

/-- Record the normal form `result` of a hypothesis of type `type`, if its
proof reads only the hypothesis's variables and facts of the context whose
types they determine. -/
private def recordNormalForm (forms : IO.Ref (Std.HashMap (Lean.Expr × Array Lean.Expr) NormalForm))
    (key : Lean.Expr × Array Lean.Expr) (scope : Lean.Expr)
    (result : Simp.Result) (undecided : Array Lean.Expr) : MetaM Unit := do
  let variables := (collectFVars {} scope).fvarIds.map mkFVar
  let expr ← instantiateMVars result.expr
  let proof? ← result.proof?.mapM instantiateMVars
  if expr.hasMVar || proof?.any (·.hasMVar) then return
  let used := collectFVars (proof?.map (collectFVars {} ·) |>.getD {}) expr
  let mut facts : Array Lean.Expr := #[]
  for decl in ← getLCtx do
    if used.fvarSet.contains decl.fvarId && !variables.contains decl.toExpr then
      unless ← isProp decl.type do return
      facts := facts.push decl.toExpr
  let all := variables ++ facts
  let mut factTypes := #[]
  for (fact, index) in facts.zipIdx do
    let factType := (← instantiateMVars (← inferType fact)).abstract
      (all.extract 0 (variables.size + index))
    if factType.hasFVar then return
    factTypes := factTypes.push factType
  let expr := expr.abstract all
  let proof? := proof?.map (·.abstract all)
  if expr.hasFVar || proof?.any (·.hasFVar) then return
  -- An undecided condition over a binder the simplification entered reads a
  -- local outside the context, which no context decides (`contextDischarge`):
  -- it leaves the form as normal as it is and is not kept.
  let context ← getLCtx
  let mut conditions := #[]
  for condition in undecided do
    if (collectFVars {} condition).fvarIds.any (!context.contains ·) then continue
    let condition := condition.abstract all
    if condition.hasFVar || condition.hasMVar then return
    conditions := conditions.push condition
  let owners ← (← targetState).preparationOwners.get
  forms.modify (·.insert key { owners, facts := factTypes, expr, proof?, undecided := conditions })

/-- The recorded normal form of a hypothesis of type `type`, over its
variables and the facts of this context its proof reads, given by type
(`contextFactsByType`). -/
private def normalFormOf? (forms : IO.Ref (Std.HashMap (Lean.Expr × Array Lean.Expr) NormalForm))
    (facts : Std.HashMap Lean.Expr Lean.Expr)
    (key : Lean.Expr × Array Lean.Expr) (scope : Lean.Expr) (decides : Lean.Expr → MetaM Bool) :
    MetaM (Option Simp.Result) := do
  let some form := (← forms.get)[key]? | return none
  unless PreparationCache.paidBy form.owners (← (← targetState).preparationOwners.get) do return none
  let mut all := (collectFVars {} scope).fvarIds.map mkFVar
  for factType in form.facts do
    let some fact := facts[factType.instantiateRev all]? | return none
    all := all.push fact
  -- A condition this context decides normalizes the hypothesis further.
  for condition in form.undecided do
    if ← decides (condition.instantiateRev all) then return none
  return some { expr := form.expr.instantiateRev all, proof? := form.proof?.map (·.instantiateRev all) }

/-- The hypotheses of the context by type, the first of a type. -/
private def contextFactsByType : MetaM (Std.HashMap Lean.Expr Lean.Expr) := do
  (← getLCtx).foldlM (init := {}) fun facts decl => do
    if decl.isImplementationDetail then return facts
    return facts.insertIfNew (← instantiateMVars decl.type) decl.toExpr

/-- These parameter operation fields are inherited unchanged by a Given
frame's carrier and declaration views. Name their original operation family
without exposing the declaration predicates carried by either view. -/
def givenOperationProjection? (e : Lean.Expr) : Option Lean.Expr := Id.run do
  let some head := e.getAppFn.constName? | return none
  let args := e.getAppArgs
  let some family := args[0]? | return none
  if head == ``Carriers.toPlain then
    unless family.isAppOfArity ``GivenFrame.toCarriers 1 ||
        family.isAppOfArity `VerexIR.Proofs.Denote.GivenFrame.clauseScaffold 1 do return none
    let mut frame := family.appArg!
    while frame.isAppOfArity `VerexIR.Proofs.Denote.GivenFrame.withDeclarations 2 do
      frame := frame.getArg! 0
    let original := mkApp (mkConst ``Carriers.toPlain) (mkApp (mkConst ``GivenFrame.toCarriers) frame)
    if original == e then return none
    return some original
  unless #[``Carriers.carrier, ``Carriers.codec, ``Carriers.orders].contains head do return none
  let mut frame ← if family.isAppOfArity ``GivenFrame.toCarriers 1 ||
      family.isAppOfArity `VerexIR.Proofs.Denote.GivenFrame.clauseScaffold 1 then
        pure family.appArg!
      else if family.isAppOfArity ``GivenFrame.operations 1 &&
          family.appArg!.isAppOfArity `VerexIR.Proofs.Denote.GivenFrame.withDeclarations 2 then
        pure family.appArg!
      else return none
  while frame.isAppOfArity `VerexIR.Proofs.Denote.GivenFrame.withDeclarations 2 do
    frame := frame.getArg! 0
  let original := mkApp (mkConst ``GivenFrame.operations) frame
  return some (mkAppN e.getAppFn (args.set! 0 original))

/-- A fresh expression and all its annotations cross the representation boundary
atomically. Reuse the transform cache across simp callbacks, and leave held caller
values and normal fixed points alone. Semantic indexed types such as Flow keep
those indices: only converting native representations are exchanged. -/
def canonicalEmission (held : Std.HashSet Lean.Expr) (fixed : Option FixedPoints) :
    MetaM (Lean.Expr → MetaM Lean.Expr) := do
  let cache ← IO.mkRef ({} : Std.HashMap ExprStructEq Lean.Expr)
  -- Values differ across emitted applications, but their representation
  -- prefixes recur. Cache conversion decisions at that smaller boundary.
  -- The factory owns one transparency and instance scope; temporary binders
  -- have unique IDs and unresolved expressions never enter this cache.
  let conversions ← IO.mkRef ({} : Std.HashMap (ExprStructEq × ExprStructEq) Bool)
  let converts (a b : Lean.Expr) : MetaM Bool := do
    if a.hasMVar || b.hasMVar then return ← isDefEq a b
    let key := ({ val := a : ExprStructEq }, { val := b : ExprStructEq })
    if let some result := (← conversions.get)[key]? then return result
    let result ← isDefEq a b
    conversions.modify (·.insert key result)
    return result
  let instances ← getLocalInstances
  let outer := (← getLCtx).getFVarIds
  return fun e => do
    -- Every conversion below needs an applied family or descriptor. Atomic
    -- expressions have neither one nor children to inspect.
    if e.isAtomic then return e
    let prior ← if e.hasMVar then pure {} else cache.swap {}
    -- Simp revisits emitted expressions. Serve the traversal's existing
    -- root entry without rebuilding its reader and state transformers.
    if let some result := prior[{ val := e : ExprStructEq }]? then
      cache.set prior
      return result
    withDefault do
      let (result, updated) ← Meta.transformWithCache e prior
        (pre := fun term => do
          if term.isAtomic then return .done term
          if held.contains term then return .done term
          if let some fixed := fixed then
            if (← fixed.known.get).contains term then return .done term
          return .continue)
        (post := fun term => do
          if let some canonical ← canonicalValueProjection? term then
            return .visit canonical
          if let some canonical := givenOperationProjection? term then
            return .visit canonical
          -- Expose just a literal row spine; nominal type descriptors remain
          -- named. This also reaches rows carried in implicit parent types.
          if term.isAppOfArity ``NRow.replicate 2 &&
              ((term.getArg! 0).nat?.isSome || (term.getArg! 0).rawNatLit?.isSome) then
            let reduced ← whnfR term
            if reduced != term then return .visit reduced
          -- Temporary instance binders opened by the traversal must not
          -- influence a cached conversion of an otherwise closed expression.
          let step ← withReader (fun ctx : Meta.Context => { ctx with localInstances := instances })
            (FamilyTransport.respellInducedNode term converts)
          match step with
          | .done rebuilt =>
              let allowed := (collectFVars {} term).fvarIds
              unless (collectFVars {} rebuilt).fvarIds.all fun id =>
                  allowed.contains id || outer.contains id do return .done term
              if rebuilt != term then return .visit rebuilt
              return .done term
          | step => return step)
        (skipConstInApp := true)
      -- Metavariable assignments can change a conversion decision. Opened
      -- locals have unique IDs; synthesis uses only the captured outer scope.
      unless e.hasMVar || result.hasMVar do cache.set updated
      return result

def canonicalEmissionMethods (base : Simp.Methods)
    (held : Std.HashSet Lean.Expr) (fixed : Option FixedPoints) : MetaM Simp.Methods := do
  let canonical ← canonicalEmission held fixed
  let result (r : Simp.Result) : SimpM Simp.Result := do
    return { r with expr := ← canonical r.expr }
  let simpStep (step : Simp.Step) : SimpM Simp.Step := do
    match step with
    | .done r => return .done (← result r)
    | .visit r => return .visit (← result r)
    | .continue (some r) => return .continue (some (← result r))
    | .continue none => return .continue
  let dsimpStep (step : Simp.DStep) : SimpM Simp.DStep := do
    match step with
    | .done e => return .done (← canonical e)
    | .visit e => return .visit (← canonical e)
    | .continue (some e) => return .continue (some (← canonical e))
    | .continue none => return .continue
  return { base with
    pre := fun e => do
      let rebuilt ← canonical e
      if rebuilt != e then return .visit { expr := rebuilt }
      simpStep (← base.pre e)
    post := fun e => do simpStep (← base.post e)
    dpre := fun e => do
      let rebuilt ← canonical e
      if rebuilt != e then return .visit rebuilt
      dsimpStep (← base.dpre e)
    dpost := fun e => do dsimpStep (← base.dpost e) }

/-- `simpGoal` with each hypothesis's normal form taken from `forms` where
known and still normal, and recorded otherwise, keyed by the hypothesis's
`scope`: the hypothesis with what else the rewriting reads by its variables.
With `resultsNormal`, a rewritten hypothesis is recorded normal too. -/
private def simpGoalRecorded (forms : IO.Ref (Std.HashMap (Lean.Expr × Array Lean.Expr) NormalForm))
    (goal : MVarId) (ctx : Simp.Context) (simprocs : Simp.SimprocsArray)
    (discharge : Simp.Discharge) (simplifyTarget : Bool) (fvarIdsToSimp : Array FVarId)
    (scope : Lean.Expr → Lean.Expr := id) (resultsNormal := false)
    (fixedPoints : Option Name := none) : MetaM (Option MVarId) := goal.withContext do
  let mut next := goal
  let mut toAssert := #[]
  let mut sources := #[]
  let mut replaced := #[]
  -- Hypotheses repeat the same carrier types and encodings. Their
  -- definitional reductions and congruence lemmas do not depend on which
  -- hypothesis is being simplified. Keep those caches for this pass, but
  -- reset the rewrite cache: the recorded normal forms must still observe
  -- every undecided side condition of their own simplification.
  let mut simpState : Simp.State := {}
  let undecided ← IO.mkRef #[]
  let counted : Simp.Discharge := fun condition => do
    let result ← discharge condition
    -- Instantiated now: a failed rewrite's assignments are rolled back.
    if result.isNone then undecided.modify (·.push (← instantiateMVars condition))
    return result
  let decides (condition : Lean.Expr) : MetaM Bool := do
    let (result, _) ← Simp.SimpM.run ctx {} (Simp.mkDefaultMethodsCore simprocs)
      (discharge condition)
    return result.isSome
  let facts ← contextFactsByType
  for fvarId in fvarIdsToSimp do
    let localDecl ← fvarId.getDecl
    let type ← instantiateMVars localDecl.type
    let key ← normalKey (scope type)
    let result ← match ← normalFormOf? forms facts key (scope type) decides with
      | some result => pure result
      | none =>
        undecided.set #[]
        let ctx := ctx.setSimpTheorems <| ctx.simpTheorems.eraseTheorem (.fvar localDecl.fvarId)
        -- The terms a traversal of this context left unchanged without
        -- reading the local context are skipped (`FixedPoints`).
        let (result, state) ← next.withContext do
          let methods := Simp.mkMethods simprocs counted (wellBehavedDischarge := false)
          let fp ← FixedPoints.new? fixedPoints
          let methods ← canonicalEmissionMethods methods {} fp
          let methods := match fp with
            | some fp => fp.wrap methods
            | none => methods
          Simp.mainCore type ctx { simpState with cache := {}, numSteps := 0 } methods
        simpState := state
        recordNormalForm forms key (scope type) result (← undecided.get)
        if resultsNormal && result.proof?.isSome then
          let normal := scope (← instantiateMVars result.expr)
          recordNormalForm forms (← normalKey normal) normal { expr := ← instantiateMVars result.expr } #[]
        pure result
    match result.proof? with
    | some proof =>
        -- A fact the context implies stays stated, as the deciders read it
        -- syntactically; a closed evaluation to `True` is cleared.
        if (← instantiateMVars result.expr).isTrue &&
            (← instantiateMVars proof).hasAnyFVar (· != fvarId) then
          replaced := replaced.push fvarId
          continue
        match ← applySimpResult next (mkFVar fvarId) type result with
        | none => return none
        | some (value, type) =>
            toAssert := toAssert.push { userName := localDecl.userName, type, value }
            sources := sources.push fvarId
    | none =>
        if result.expr.isFalse then
          next.assign (← mkFalseElim (← next.getType) (mkFVar fvarId))
          return none
        next ← next.replaceLocalDeclDefEq fvarId result.expr
        replaced := replaced.push fvarId
  if simplifyTarget then
    -- Clause markers identify the remaining goals after shared preparation.
    -- Hypotheses still normalize normally, including marked contradictions.
    let ctx := if verex.keepObligationMarkers.get (← getOptions) then
        ctx.setSimpTheorems (ctx.simpTheorems.eraseTheorem (.decl ``VerexIR.Proofs.Obligation_eq))
      else ctx
    let normalized ← next.withContext do
      let target ← instantiateMVars (← next.getType)
      let methods ← canonicalEmissionMethods
        (Simp.mkMethods simprocs discharge (wellBehavedDischarge := false)) {} none
      let (result, _) ← Simp.main target ctx (methods := methods)
      if result.expr.isTrue then
        next.assign (← match result.proof? with
          | some proof => mkOfEqTrue proof
          | none => pure (mkConst ``True.intro))
        return none
      return some (← applySimpResultToTarget next target result)
    let some normalized := normalized | return none
    next := normalized
  let (introduced, asserted) ← next.assertHypotheses toAssert
  -- A unit fact rewritten stays one.
  let moduleFacts ← (← targetState).moduleFacts.get
  for (source, fvarId) in sources.zip introduced do
    if moduleFacts.contains source then (← targetState).moduleFacts.modify (·.insert fvarId)
  next := asserted
  next ← next.tryClearMany (fvarIdsToSimp.filter fun fvarId => !replaced.contains fvarId)
  return some next

/-- The key under which a pass remembers a hypothesis normal: its type up
to variable names and, where its normal form reads the context, the
propositions connected to it through shared variables, which are all the
discharger of its side conditions can read. Under the same facts, the same
hypothesis normalizes the same way, so a later leaf of the path need not
normalize it, nor ask its undecided conditions, again. -/
private def normalCacheKey (type : Lean.Expr) (connected : Lean.Expr → MetaM (Array Lean.Expr)) :
    MetaM (Lean.Expr × Array Lean.Expr) := do
  let (abstracted, types) ← normalKey type
  if contextDependent type then return (abstracted, types ++ (← connected type))
  return (abstracted, types)

/-- Record the unit's facts (`TargetState.moduleFacts`) normal, once at the
solver's start: they are the same at every leaf, and no rollback forgets them. -/
def recordModuleFactsNormal (goal : MVarId) : MetaM Unit := goal.withContext do
  let state ← targetState
  let propositions ← leafPropositions goal
  let connected (type : Lean.Expr) : MetaM (Array Lean.Expr) :=
    return connectedPropositions propositions type
  for fvarId in ← state.moduleFacts.get do
    let some decl := (← getLCtx).find? fvarId | continue
    let type ← instantiateMVars decl.type
    state.normalHypotheses.modify (·.insert (← normalCacheKey type connected))

/-- The simp context and simprocs of a normalization pass over the context,
by the pass, its theorems and simprocs built once per target
(`TargetState.normalSimpContexts`), the context made for the goal at hand
(`Simp.mkContext` reads the local context's size): a context already normal
is no failure. -/
private def normalSimpContext (pass : Nat) (stx : Lean.TSyntax `tactic) :
    TacticM (Simp.Context × Simp.SimprocsArray) := do
  let cache := (← targetState).normalSimpContexts
  let (ctx, simprocs) ← match (← cache.get).find? (·.1 == pass) with
    | some (_, ctx, simprocs) => pure (ctx, simprocs)
    | none =>
        let { ctx, simprocs, .. } ← mkSimpContext stx (eraseLocal := false)
        cache.modify (·.push (pass, ctx, simprocs))
        pure (ctx, simprocs)
  return (← Simp.mkContext { ctx.config with failIfUnchanged := false } ctx.simpTheorems
    ctx.congrTheorems, simprocs)

/-- Normalize the goal and the hypotheses a cache does not know normal, and
record only hypotheses this pass normalized; with a discharger, each
hypothesis's normal form is taken from and recorded in `normalForms`.
Returns the hypotheses normalized. -/
private def normalizeUncached (cache : IO.Ref (Std.HashSet (Lean.Expr × Array Lean.Expr)))
    (pass : Nat) (stx : Lean.TSyntax `tactic) (discharge? : Option Simp.Discharge)
    (target : Bool := true) (select : FVarId → Bool := fun _ => true)
    (fixedPoints : Option Name := none) : TacticM (Array FVarId) := do
  let known ← cache.get
  let goal ← getMainGoal
  -- The context's propositions, read where a hypothesis's key depends on
  -- its context (`normalCacheKey`), collected at most once per context.
  let propositions ← IO.mkRef (none : Option (Array (Lean.Expr × FVarIdSet)))
  let connectedAt (goal : MVarId) (type : Lean.Expr) : MetaM (Array Lean.Expr) := do
    let collected ← match ← propositions.get with
      | some collected => pure collected
      | none =>
          let collected ← goal.withContext (leafPropositions goal)
          propositions.set (some collected)
          pure collected
    return connectedPropositions collected type
  let mut targets := #[]
  let originalContext ← getLCtx
  for fvarId in ← goal.getNondepPropHyps do
    unless select fvarId do continue
    let type ← instantiateMVars (← fvarId.getType)
    let key ← normalCacheKey type (connectedAt goal)
    unless known.contains key do targets := targets.push fvarId
  -- Nothing to normalize leaves the goal as it is.
  if targets.isEmpty && !target then return targets
  let normalized := targets.foldl (init := ({} : FVarIdSet)) fun ids id => ids.insert id
  let (ctx, simprocs) ← normalSimpContext pass stx
  let forms := (← targetState).normalForms
  let simplified ← match discharge? with
    | some discharge =>
        simpGoalRecorded forms goal ctx simprocs discharge target targets (fixedPoints := fixedPoints)
    | none => do
        let (result, _) ← simpGoal goal ctx simprocs none (simplifyTarget := target)
          (fvarIdsToSimp := targets)
        pure (result.map (·.2))
  match simplified with
  | none => replaceMainGoal []
  | some next =>
      propositions.set none
      next.withContext do
        for decl in ← getLCtx do
          if decl.isImplementationDetail || decl.isLet then continue
          let type ← instantiateMVars decl.type
          -- A dependent or excluded premise was not normalized. Nor was
          -- an already cached premise normalized under this pass's new
          -- context. Keep those earlier cache entries without certifying
          -- untouched hypotheses as normal here.
          if originalContext.contains decl.fvarId && !normalized.contains decl.fvarId then
            continue
          if ← isProp type then
            cache.modify (·.insert (← normalCacheKey type (connectedAt next)))
      replaceMainGoal [next]
  return targets

/-- `verex_normalize at *`, skipping the hypotheses a stage already
left normal, as a loop step normalizes the whole context several times
while most hypotheses do not change. -/
def normalizeContextStage (select : FVarId → Bool := fun _ => true) : TacticM Unit :=
  withMainContext do
    discard <| normalizeUncached (← targetState).stageNormalHypotheses 0
      (← `(tactic| simp only [vir_denote, vir_denote_norm, vir_denote_eval, Prod.fst, Prod.snd]))
      none (select := select)

@[inherit_doc normalizeContextStage]
elab "verex_normalize_context_stage" : tactic => normalizeContextStage

/-- Normalize the goal and the hypotheses after a case split of a source
value: the skolem families and variant names the split exposed, with the
leaf normalization. Hypotheses a previous pass left normal are skipped. -/
elab "verex_normalize_families" : tactic => withMainContext do
  discard <| normalizeUncached (← targetState).familyNormalHypotheses 1
    (← `(tactic| simp only [variantCarrier.ofSkolem, HList.ofSkolem, NTy.ofSkolem,
      variantPlain.ofSkolem, HListPlain.ofSkolem, NTy.plainOfSkolem,
      variantName, vir_denote_norm]))
    none

/-- Normalize the goal and the hypotheses not known normal, as `simp at *`
would normalize the whole context: a conditional whose condition omega
decides, such as a vector's bound after an update, takes its branch, and a
condition a hypothesis states is proved by that hypothesis, so that the
branch's proof term mentions nothing a substitution would then have to
keep. Every hypothesis of the result is recorded normal. Only the hypotheses
`select` accepts are normalized. -/
def normalizeContext (target : Bool) (select : FVarId → Bool := fun _ => true) :
    TacticM Unit := withMainContext do
  -- A side condition a hypothesis states is proved by it at reducible
  -- transparency: plain `assumption` would unfold the arithmetic of every
  -- hypothesis it compares against.
  let stx ← `(tactic| simp only
    [VerexIR.Proofs.Obligation_eq, Nat.reduceAdd, Int.reducePow, Int.reduceSub, Nat.reducePow,
    Nat.reduceSub, vir_denote_norm, dif_pos,
    dif_neg, if_pos, if_neg, Prod.fst, Prod.snd])
  let goal ← getMainGoal
  let started ← IO.getNumHeartbeats
  -- A side condition about a variable simp introduced under a binder
  -- (`fun decoded => …`) has no fact in the context: the discharger is not
  -- run on it, which would try omega over the whole context for nothing.
  let context ← goal.withContext getLCtx
  -- The `leaf` fixed points hold within this pass only: the context differs
  -- from pass to pass.
  (← targetState).fixedPoints.modify (·.erase `leaf)
  (← targetState).dsimpResults.modify (·.erase `leaf)
  let targets ← normalizeUncached (← targetState).normalHypotheses 2 stx
    (some (contextDischarge context (← IO.mkRef {}))) target select (fixedPoints := some `leaf)
  if verex.solverDebug.get (← getOptions) then
    let hypotheses ← goal.withContext goal.getNondepPropHyps
    let (calls, cost, failed) ← (← targetState).dischargeLog.get
    (← targetState).dischargeLog.set (0, 0, #[])
    IO.println s!"    normalize: {targets.size} of {hypotheses.size} hypotheses uncached, {((← IO.getNumHeartbeats) - started) / 1000}k; discharger {calls} calls {cost / 1000}k, {failed.size} failed"
    goal.withContext do
      for target in targets.toList.take 12 do
        if let some decl := (← getLCtx).find? target then
          let type ← instantiateMVars decl.type
          IO.println s!"      uncached {decl.userName}{if contextDependent type then " (context-dependent)" else ""}{if type.hasMVar then " (mvars)" else ""}: {((toString (← ppExpr type)).replace "\n" " ").take 100}"
    for c in failed.toList.take 4 do IO.println s!"      undecided: {c}"
    -- Conditional hypotheses the pass could not touch: dependent ones.
    goal.withContext do
      for decl in ← getLCtx do
        if decl.isImplementationDetail || decl.isLet then continue
        let type ← instantiateMVars decl.type
        unless ← isProp type do continue
        if hypotheses.contains decl.fvarId then continue
        if contextDependent type then
          IO.println s!"      dependent conditional hypothesis {decl.userName}: {((toString (← ppExpr type)).replace "\n" " " |>.take 160).toString}"

elab "verex_normalize_context" : tactic => normalizeContext true

/-- `verex_normalize_context` on the hypotheses only, the target left as it is. -/
elab "verex_normalize_hypotheses" : tactic => normalizeContext false

/-- Values read from runtime storage or a typed map carry their declaration's
certificate even when the read is a compound expression. Collect the reads
without unfolding their definitions or inspecting constructed certificates.
A value under a binder is skipped unless closed in the leaf's context. -/
private partial def observedCertifiedValues (term : Lean.Expr) (found : Array Lean.Expr) :
    Array Lean.Expr :=
  let read := term.isAppOfArity ``Skolems.ofRuntime 5 ||
    term.isAppOfArity ``VerexIR.SpecMap.valueAt 6 ||
    term.isAppOfArity ``VerexIR.SpecMap.keyAt 5
  let found :=
    if read && !term.hasLooseBVars && !found.any (· == term) then found.push term
    else found
  match term with
  | .app f a => observedCertifiedValues a (observedCertifiedValues f found)
  | .lam _ d b _ | .forallE _ d b _ => observedCertifiedValues b (observedCertifiedValues d found)
  | .letE _ t v b _ => observedCertifiedValues b (observedCertifiedValues v (observedCertifiedValues t found))
  | .mdata _ b => observedCertifiedValues b found
  | .proj _ _ b => observedCertifiedValues b found
  | _ => found

open Lean Meta in
/-- Observe an ordinary nominal descriptor only at the certificate boundary.
The descriptor remains named in the value's type and in surrounding goals. -/
private def nominalCertificateView? (type : Lean.Expr) : MetaM (Option (Lean.Expr × Lean.Expr)) := do
  let_expr NTy.carrier carriers type := type | return none
  unless isNominalType (← getEnv) type do return none
  let view ← nativeTypeView type
  unless view.isAppOfArity ``NTy.struct 3 || view.isAppOfArity ``NTy.enum 5 do return none
  return some (carriers, view)

private theorem structOwnCertificate [Carriers] (source : StructHandle)
    (arguments fields : NRow) (value : (NTy.struct source arguments fields).carrier) :
    Carriers.own source arguments
      (NTy.plainEncode (.struct source arguments fields) value.val) :=
  ((NTy.deepHolds_struct source arguments fields value.val).mp value.property).1

private theorem enumOwnCertificate [Carriers] (source : StructHandle) (arguments : NRow)
    (names : List String) (rows : NRows) (distinct : names.Nodup)
    (value : (NTy.enum source arguments names rows distinct).carrier) :
    Carriers.own source arguments
      (NTy.plainEncode (.enum source arguments names rows distinct) value.val) :=
  ((NTy.deepHolds_enum source arguments names rows distinct value.val).mp value.property).1

/-- Select a frame's owned declaration before decoding its certificate.
The frame's ownership law is propositional, so unfolding its opaque field
cannot select the unit's invariant. Observe only the predicate: its encoded
value stays intact for the codec inverse law in `decodedOwnFact?`. -/
private def certificateOwnClauses? (type proof : Lean.Expr) : TacticM (Option Lean.Expr) := do
  let family := type.getArg! 0
  unless type.isAppOfArity ``Carriers.own 4 &&
      family.isAppOfArity ``Skolems.toCarriers 3 do
    return (← decodedOwnFact? type proof).map Prod.fst
  let encoded := type.appArg!
  let (statement, equation) ← withLocalDeclD `encoded (mkConst ``RuntimeValue) fun value => do
    let observed := mkAppN type.getAppFn (type.getAppArgs.set! 3 value)
    let { ctx, simprocs, .. } ← mkSimpContext
      (← `(tactic| simp only [vir_denote_norm])) (eraseLocal := false)
    let (result, _) ← Simp.main observed ctx
      (methods := Simp.mkDefaultMethodsCore simprocs)
    return (← mkLambdaFVars #[value] result.expr, ← mkLambdaFVars #[value] (← result.getProof))
  let statement := statement.beta #[encoded]
  let proof ← mkEqMP (mkApp equation encoded) proof
  return (← decodedOwnFact? statement proof).map Prod.fst

/-- A declaration's data invariant of a certified value, as a fact: a value
of a declaration's type carries the invariant in its certificate, and a read
owes nothing and assumes nothing beyond it
(`doc/designs/data-invariants.md`).

The fact is the declaration's clauses of the value's row, not the
certificate itself: `NTy.deepHolds_struct_val` states that a certified
value's row satisfies the invariant, so a bare `.property` normalizes to
`True` and says nothing.  The `own` conjunct of the certificate is unfolded
to the clauses by the route the owed direction takes
(`verex_own_encoded`): the unit's `own` selects the declaration by its
handle (`<path>.dataOwn`), and the selected predicate of an encoding
(`OwnAtArguments`) is applied to the arguments' inhabitability and to the
decoding of the row's own encoding, which leaves the named clauses
(`<path>.dataOwn.case{i}`) applied to the plain row.  None where the type
is not a declaration's, where the arguments are not known inhabitable, or
where the unit states no invariant — there the selection leaves `True` and
the fact would say nothing. -/
def certifiedClauses? (value : Lean.Expr) : TacticM (Option Lean.Expr) := do
  let type ← whnfR (← inferType value)
  if let some (carriers, view) ← nominalCertificateView? type then
    let proof ← withDefault <| mkAppOptM
      (if view.isAppOfArity ``NTy.struct 3 then ``structOwnCertificate else ``enumOwnCertificate)
      ((#[carriers] ++ view.getAppArgs ++ #[value]).map some)
    return ← certificateOwnClauses? (← inferType proof) proof
  let_expr Subtype α predicate := type | return none
  -- The certificate's statement at a fresh row, as the carrier spells it:
  -- `NTy.carrier` at a declaration is its row under `NTy.deepHolds` of the
  -- declaration's type, and unfolding that would lose the type the clauses
  -- belong to.
  let stated ← withLocalDeclD `row α fun row => do
    return (predicate.beta #[row]).abstract #[row]
  let_expr NTy.deepHolds _ σ _ := stated | return none
  let σ ← whnfD σ
  unless σ.isAppOfArity ``NTy.struct 3 || σ.isAppOfArity ``NTy.enum 5 do return none
  let certificate ← mkAppOptM ``Subtype.property #[α, predicate, value]
  -- The invariants of a declaration's value are its `own` of the row's
  -- encoding and its fields' own: the first conjunct is the declaration's.
  let certified ← whnfD (← inferType certificate)
  let_expr And ownStated _ := certified | return none
  let own ← mkAppM ``And.left #[← mkExpectedTypeHint certificate certified]
  -- A compound read exposes the same owned encoding as a local fact.
  -- The shared producer checks the application to its row and decoding.
  return ← certificateOwnClauses? ownStated own

/-- A nominal certificate also certifies the plain fields recursively.  These
facts belong to the value's current frame, independently of the transport
needed to read the declaration's own predicate. -/
def certifiedFields? (value : Lean.Expr) : TacticM (Option Lean.Expr) := do
  let type ← whnfR (← inferType value)
  if let some (carriers, view) ← nominalCertificateView? type then
    let isStruct := view.isAppOfArity ``NTy.struct 3
    let free := if isStruct then
        mkApp (mkConst ``NRow.certificateFree) (view.getArg! 2)
      else mkApp (mkConst ``NRows.certificateFree) (view.getArg! 3)
    if (← whnfD free).isConstOf ``Bool.true then return none
    return some (← withDefault <| mkAppOptM
      (if isStruct then ``NTy.fieldsHold else ``NTy.variantHolds)
      ((#[carriers] ++ view.getAppArgs ++ #[value]).map some))
  let_expr Subtype α predicate := type | return none
  let stated ← withLocalDeclD `row α fun row => do
    return (predicate.beta #[row]).abstract #[row]
  let_expr NTy.deepHolds _ σ _ := stated | return none
  let σ ← whnfD σ
  unless σ.isAppOfArity ``NTy.struct 3 || σ.isAppOfArity ``NTy.enum 5 do return none
  -- These fields carry no further invariant when their row is certificate-free.
  -- Keep unknown rows and every nominal/parameter field for the recursive reader.
  let free := if σ.isAppOfArity ``NTy.struct 3 then
      mkApp (mkConst ``NRow.certificateFree) (σ.getArg! 2)
    else mkApp (mkConst ``NRows.certificateFree) (σ.getArg! 3)
  if (← whnfD free).isConstOf ``Bool.true then return none
  let certificate ← mkAppOptM ``Subtype.property #[α, predicate, value]
  let certified ← whnfD (← inferType certificate)
  let_expr And _ _ := certified | return none
  return some (← mkAppM ``And.right
    #[← mkExpectedTypeHint certificate certified])

/-- A nominal certificate retained after its value was destructured. Read
its own clauses and retain its fields' certificates as a separate conjunct. -/
private def retainedCertificateClauses? (type proof : Lean.Expr) :
    TacticM (Option Lean.Expr) := do
  let_expr NTy.deepHolds carriers τ value := type.consumeMData | return none
  let view ← nativeTypeView τ
  let isStruct := view.isAppOfArity ``NTy.struct 3
  unless isStruct || view.isAppOfArity ``NTy.enum 5 do return none
  let equivalence := mkAppN
    (mkConst (if isStruct then ``NTy.deepHolds_struct else ``NTy.deepHolds_enum))
    (#[carriers] ++ view.getAppArgs ++ #[value])
  let_expr Iff held _ := ← inferType equivalence | return none
  let certificate ← mkAppM ``Iff.mp
    #[equivalence, ← mkExpectedTypeHint proof held]
  let own ← mkAppM ``And.left #[certificate]
  let fields ← mkAppM ``And.right #[certificate]
  let own ← match ← certificateOwnClauses? (← inferType own) own with
    | some clauses => pure clauses
    | none => pure own
  return some (← mkAppM ``And.intro #[own, fields])

/-- The declaration clauses of every certified value the leaf names
(`certifiedClauses?`), asserted as facts: its locals, and the compound storage
reads in the goal and facts (`observedCertifiedValues`). A value whose
declaration states no invariant contributes nothing: the selection leaves `True` there and the
step declines, so a unit without data invariants gains no hypothesis.  Each
fact asserted is normalized where it stands, so that the deciders read it in
the form they read every other fact; the normalization visits the new facts
alone, and none is asserted where the unit states no invariant. -/
def assertCertifiedClauses (canonicalize : Lean.Expr → MetaM Lean.Expr)
    (introduced : Option (Array Lean.Expr) := none) : TacticM Unit := focusGoal do
  if (← getGoals).isEmpty then return
  let goal ← getMainGoal
  let values ← goal.withContext do
    if let some values := introduced then
      return ← values.filterM fun value => return !(← isProp (← inferType value))
    let mut found := #[]
    for decl in ← getLCtx do
      if decl.isImplementationDetail || decl.isLet then continue
      if ← isProp decl.type then continue
      found := found.push decl.toExpr
    -- Compound reads carry certificates too, even before they receive a local name.
    found := observedCertifiedValues (← instantiateMVars (← goal.getType)) found
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      unless ← isProp decl.type do continue
      found := observedCertifiedValues (← instantiateMVars decl.type) found
    return found
  -- Every fact is found before any is asserted.  The reduction of a
  -- certificate reads the goal the step was entered with, and an assert
  -- replaces that goal, so a fact looked for after one was asserted would be
  -- looked for against a goal that is gone and would be lost.
  let stated ← IO.mkRef (#[] : Array Lean.Expr)
  let found ← IO.mkRef (#[] : Array (Option FVarId × Lean.Expr × Lean.Expr))
  -- Splitting a certificate can already have exposed its selected nominal
  -- predicate. Read it once at this boundary, keeping the original proof
  -- as internal evidence instead of unfolding it at every normalization.
  goal.withContext do
    for decl in ← getLCtx do
      if decl.isImplementationDetail || decl.isLet then continue
      let type ← instantiateMVars decl.type
      let observed? ← selectedOwnClauses? canonicalize type decl.toExpr
      let observed? ← match observed? with
        | some observed => pure (some observed)
        | none => do
            let some clauses ← retainedCertificateClauses? type decl.toExpr | pure none
            let statement ← canonicalize
              (← whnfR (← deltaExpand (← inferType clauses)
                (fun name => (name.toString.splitOn "dataOwn.case").length != 1)))
            pure (some (statement, ← mkExpectedTypeHint clauses statement))
      let some (statement, proof) := observed? | continue
      if statement == type || statement.isTrue then continue
      stated.modify (·.push statement)
      found.modify (·.push (some decl.fvarId, statement, proof))
  for value in values do
    let own ← (try goal.withContext (certifiedClauses? value)
      catch _ => pure none)
    let fields ← (try goal.withContext (certifiedFields? value)
      catch _ => pure none)
    for clauses? in #[own, fields] do
      let some clauses := clauses? | continue
      try
        goal.withContext do
          -- The declaration's clauses, not the constant that holds them: the
          -- selection leaves `<path>.dataOwn.case{i}` of the row, whose
          -- equation states the clauses (`verex_own_encoded`).
          let statement ← canonicalize
            (← whnfR (← deltaExpand (← instantiateMVars (← inferType clauses))
              (fun name => (name.toString.splitOn "dataOwn.case").length != 1)))
          if statement.isConstOf ``True then return
          -- Redundancy is optional: compare only through reducible wrappers.
          -- Opening nominal predicates here repeatedly unfolds unrelated
          -- definitions before the leaf can use its already-stated facts.
          if (← (← stated.get).anyM fun other => withReducible (isDefEq other statement)) then return
          -- The step is taken at the entry of each path, so a fact the
          -- context already states is not stated twice.
          if ← (← getLCtx).anyM fun decl => do
              if decl.isImplementationDetail then return false
              withNewMCtxDepth (withReducible (isDefEq decl.type statement)) then
            return
          stated.modify (·.push statement)
          if verex.solverDebug.get (← getOptions) then
            IO.println s!"    certified clauses: {((toString (← ppExpr statement)).replace "\n" " ").take 400}"
          found.modify (·.push (none, statement, ← mkExpectedTypeHint clauses statement))
      catch _ => pure ()
  let mut current := goal
  let asserted ← IO.mkRef (#[] : Array FVarId)
  for (source?, statement, clauses) in ← found.get do
    current ← (try
        current.withContext do
          let (fact, next) ← (← current.assert `verexInvariant statement clauses).intro1P
          asserted.modify (·.push fact)
          if let some source := source? then
            next.modifyLCtx (·.setKind source .implDetail)
          pure next
      catch _ => pure current)
  replaceMainGoal [current]
  let facts ← asserted.get
  unless facts.isEmpty do
    normalizeContext (target := false) (select := (facts.contains ·))


@[inherit_doc assertCertifiedClauses]
syntax "verex_certified_clauses" : tactic


/-- The role predicates of the context read in their specification forms
(`vir_denote_spec`), then the normal form of what changed. Only the goal and
the facts that mention a predicate the set reads are rewritten; the
hypotheses already normal are cached and skipped. -/
elab "verex_spec_forms" : tactic => focusGoal do
  if (← getGoals).isEmpty then return
  let some extension ← getSimpExtension? `vir_denote_spec | return
  let theorems ← extension.getTheorems
  let heads := [theorems.pre, theorems.post].foldl (init := ({} : NameSet)) fun found tree =>
    tree.root.foldl (init := found) fun found key _ =>
      match key with
      | .const name _ => found.insert name
      | _ => found
  let mentions (e : Lean.Expr) : Bool :=
    (e.find? fun term => match term.getAppFn with
      | .const name _ => heads.contains name
      | _ => false).isSome
  let goal ← getMainGoal
  let (targets, target) ← goal.withContext do
    let targets ← (← goal.getNondepPropHyps).filterM fun fvarId => do
      return mentions (← instantiateMVars (← fvarId.getType))
    return (targets, mentions (← instantiateMVars (← goal.getType)))
  if targets.isEmpty && !target then return
  let { ctx, simprocs, .. } ← mkSimpContext (← `(tactic| simp only [vir_denote_spec]))
    (eraseLocal := false)
  let ctx ← Simp.mkContext { ctx.config with failIfUnchanged := false } ctx.simpTheorems
    ctx.congrTheorems
  let (result, _) ← simpGoal goal ctx simprocs (simplifyTarget := target) (fvarIdsToSimp := targets)
  match result with
  | none => replaceMainGoal []
  | some (_, next) =>
      if next == goal then return
      replaceMainGoal [next]
      normalizeContext true

/-- Add normalized copies of facts used by dependent terms. The original
proofs stay in place, so reducing a carrier conversion cannot invalidate the
types of those terms. Ordinary context simplification skips these facts. -/
elab "verex_normalize_dependent" : tactic => withMainContext do
  let goal ← getMainGoal
  let independent ← goal.getNondepPropHyps
  let { ctx, simprocs, .. } ← mkSimpContext (← `(tactic| simp only [
    variantCarrier.ofSkolem, HList.ofSkolem, NTy.ofSkolem,
    variantPlain.ofSkolem, HListPlain.ofSkolem, NTy.plainOfSkolem,
    variantName, vir_denote_norm])) (eraseLocal := false)
  let mut hypotheses : Array Hypothesis := #[]
  for decl in ← getLCtx do
    if decl.isImplementationDetail || independent.contains decl.fvarId then continue
    let type ← instantiateMVars decl.type
    unless ← isProp type do continue
    let (result, _) ← simp type ctx simprocs
    if let some proof := result.proof? then
      if result.expr.isTrue then continue
      hypotheses := hypotheses.push { userName := decl.userName, type := result.expr, value := ← mkEqMP proof decl.toExpr }
  let (_, next) ← goal.assertHypotheses hypotheses
  replaceMainGoal [next]

/-- `verex_normalize_context` over the hypotheses only, the goal left
as it is. -/
elab "verex_normalize_hypotheses" : tactic => normalizeContext false

/-- Take each goal through `step` on its own, after asserting the bounds it
needs (`assert`), and record the bounds asserted on the goals `step` leaves
and their ancestors: those are not asserted on them again. -/
private def withBounds (assert : Bool) (step : Syntax) : TacticM Unit := do
  let mut remaining := []
  for goal in ← getGoals do
    let known := (← (← targetState).assertedBounds.get)[← goalContent goal]?.getD #[]
    setGoals [goal]
    let asserted := known ++ (← if assert then assertBounds known else pure #[])
    evalTactic step
    let results ← getGoals
    unless asserted.isEmpty do
      for result in results do
        let content ← goalContent result
        (← targetState).assertedBounds.modify (·.insert content asserted)
    remaining := remaining ++ results
  setGoals remaining

/-- Assert the bounds each goal needs, then run a tactic on it. -/
elab "verex_bounded " step:tactic : tactic => withBounds true step

/-- Run a tactic on each goal; the goals it leaves have their ancestor's
bounds. -/
elab "verex_inheriting " step:tactic : tactic => withBounds false step

/-- Whether an expression is a literal: a numeral, a string, or a Boolean
constructor. -/
def isLiteral (e : Lean.Expr) : MetaM Bool := do
  if e.isRawNatLit || e.isStringLit || e.isConstOf ``Bool.true || e.isConstOf ``Bool.false then
    return true
  return (← getNatValue? e).isSome || (← getIntValue? e).isSome

/-- A hypothesis as a rewrite rule: its proof and, for an equation, its
sides as the rule rewrites them. An equation rewrites towards its literal
side: one stating a literal on the left, `1 = x`, is used reversed, as a
rule rewriting the literal would rewrite every occurrence of it, and
against a hypothesis `x = 1` without end. -/
def orientedHypothesis (fvarId : FVarId) : MetaM (Lean.Expr × Option (Lean.Expr × Lean.Expr)) := do
  let proof := mkFVar fvarId
  let type ← instantiateMVars (← fvarId.getType)
  let some (_, lhs, rhs) := type.eq? | return (proof, none)
  if (← isLiteral lhs) && !(← isLiteral rhs) then return (← mkEqSymm proof, some (rhs, lhs))
  return (proof, some (lhs, rhs))

/-- The simp context of a `simp only` call with the given hypotheses as
rewrite rules besides its lemmas (`simpAt`), in the main goal's context. -/
def rulesSimpContext (call : Lean.TSyntax `tactic) (rules : Array FVarId) :
    TacticM (Simp.Context × Simp.SimprocsArray × Simp.DischargeWrapper) := do
  let { ctx, simprocs, dischargeWrapper, .. } ← mkSimpContext call (eraseLocal := false)
  let mut theorems := ctx.simpTheorems
  unless rules.isEmpty do
    let mut extra : SimpTheorems := {}
    -- An equation whose reverse is already a rule is left out.
    let mut equations : Array (Lean.Expr × Lean.Expr) := #[]
    for fvarId in rules do
      let (proof, equation?) ← orientedHypothesis fvarId
      if let some (lhs, rhs) := equation? then
        if equations.contains (rhs, lhs) then continue
        equations := equations.push (lhs, rhs)
      extra ← extra.add (.fvar fvarId) #[] proof
    theorems := theorems.push extra
  let ctx ← Simp.mkContext { ctx.config with failIfUnchanged := false } theorems ctx.congrTheorems
  return (ctx, simprocs, dischargeWrapper)

/-- Run a `simp only` call at the given hypotheses and, when asked, the
goal, with the given hypotheses as rewrite rules besides its lemmas. The
hypotheses are addressed by identity: a name a hypothesis has lost cannot
be spelled in a location. Returns the goal left, `none` when closed. -/
def simpAt (goal : MVarId) (call : Lean.TSyntax `tactic) (rules : Array FVarId)
    (targets : Array FVarId) (simplifyTarget : Bool) : TacticM (Option MVarId) :=
  goal.withContext do
    let (ctx, simprocs, dischargeWrapper) ← rulesSimpContext call rules
    let targets := targets.filter (!rules.contains ·)
    let (result, _) ← dischargeWrapper.with fun discharge? =>
      simpGoal goal ctx simprocs discharge? (simplifyTarget := simplifyTarget)
        (fvarIdsToSimp := targets)
    return result.map (·.2)

/-- `simpAt` as a tactic step over the main goal. -/
def simpMainAt (call : Lean.TSyntax `tactic) (rules : Array FVarId) (targets : Array FVarId)
    (simplifyTarget : Bool) : TacticM Unit := do
  match ← simpAt (← getMainGoal) call rules targets simplifyTarget with
  | none => replaceMainGoal []
  | some goal => replaceMainGoal [goal]

/-- The typed maps the context defines (`SpecMap.ofEntries key value x = e`,
the carrier or projected field's owner absent from `e`). A field definition
is a forward reading; only a bare carrier is eligible for write-back. -/
private def mapDefinitions (goal : MVarId) : MetaM (Array FVarId) := goal.withContext do
  (← getLCtx).foldlM (init := #[]) fun found decl => do
    if decl.isImplementationDetail then return found
    let some (_, lhs, rhs) := (← instantiateMVars decl.type).eq? | return found
    unless lhs.isAppOfArity ``VerexIR.SpecMap.ofEntries 7 do return found
    let some owner := tableViewOwner? lhs.appArg! | return found
    if rhs.containsFVar owner then return found
    return found.push decl.fvarId

/-- Recover a plain table from its typed map only under its actual
certificate. The total plain decoder is not injective outside this boundary. -/
private def plainTableWriteBack? (carrier : FVarId) (definition : FVarId)
    (rhs : Lean.Expr) : MetaM (Option (Lean.Expr × Lean.Expr)) := do
  let type ← instantiateMVars (← carrier.getType)
  let some spelling := plainSpelling? type | return none
  let .ty table := spelling.payload | return none
  let some carriers := spelling.carriers? | return none
  let_expr NTy.table key value := ← whnfD table | return none
  let raw := mkFVar carrier
  let certificate := mkApp3 (mkConst ``NTy.deepHolds) carriers table raw
  let mut held? := none
  for decl in ← getLCtx do
    if decl.isImplementationDetail || !(decl.type.containsFVar carrier) then continue
    if ← withDefault (withNewMCtxDepth (isDefEq decl.type certificate)) then
      held? := some (mkFVar decl.fvarId)
      break
  let some held := held? | return none
  let inhabitable (τ : Lean.Expr) : MetaM Lean.Expr := do
    let target ← mkEq (mkApp (mkConst ``NTy.inhabitable) τ) (mkConst ``Bool.true)
    let proof ← mkEqRefl (mkConst ``Bool.true)
    unless ← withDefault (withNewMCtxDepth (isDefEq (← inferType proof) target)) do failure
    mkExpectedTypeHint proof target
  observing? <| withTransparency .default do
    let nonempty ← synthInstance (mkApp (mkConst ``CarriersNonempty) carriers)
    let proof ← mkAppOptM ``VerexIR.SpecMap.eq_forget_writeBack_of_ofEntries_readPlain
      #[some carriers, some nonempty, some key, some value, some (← inhabitable key),
        some (← inhabitable value), some raw, some held, some rhs, some (mkFVar definition)]
    let written ← inferType proof
    return (written, proof)

/-- The carrier a typed map's definition determines, substituted: a table is
its typed map written back (`SpecMap.eq_writeBack_of_ofEntries`), so
`SpecMap.ofEntries key value x = e` gives `x = writeBack unread e`, the
write-back a specification's map update builds. The definition stays as the
write-back's read (`ofEntries key value (writeBack unread e) = e`), for a
read of the carrier a later unfolding spells again; with `hide`, a bridge
the deciders do not see (`verex_written_back` reads it). -/
def writeBackCarrier (goal : MVarId) (definition : FVarId) (hide : Bool := false) :
    MetaM (Option MVarId) := do
  let some (written, proof) ← goal.withContext do
      let some (_, lhs, rhs) := (← instantiateMVars (← definition.getType)).eq? | return none
      let some carrier := typedMap? lhs | return none
      if let some written ← plainTableWriteBack? carrier definition rhs then
        return some written
      let_expr VerexIR.Proofs.Denote.NTy.carrier carriers table :=
        (← instantiateMVars (← carrier.getType)).consumeMData | return none
      -- A generic callee's table at the call's type arguments.
      let_expr VerexIR.Proofs.Denote.NTy.table κ ν := ← whnfD table | return none
      let some keys ← VerexIR.Proofs.Denote.NativeMaps.typedView? carriers κ | return none
      let some values ← VerexIR.Proofs.Denote.NativeMaps.typedView? carriers ν | return none
      let unread ← VerexIR.Proofs.Denote.NativeMaps.TypedView.entryUnread keys values
      let written ← mkEq (mkFVar carrier) (← mkAppM ``VerexIR.SpecMap.writeBack #[unread, rhs])
      let proof ← mkAppOptM ``VerexIR.SpecMap.eq_writeBack_of_ofEntries
        #[none, none, none, none, some (lhs.getArg! 4), some (lhs.getArg! 5), some keys.unread,
          some values.unread, some (← keys.readBack), some (← values.readBack), none, none,
          some (mkFVar definition)]
      return some (written, ← mkExpectedTypeHint proof written)
    | return none
  let (equation, next) ← (← goal.assert `written written proof).intro1P
  -- The left side is the carrier. `substEq` would reduce the right side to look
  -- for a local, evaluating a closed map.
  let some (substitution, next) ← (try some <$> next.withContext (substCore next equation)
      catch _ => pure none) | return none
  unless hide do return some next
  let .fvar read := substitution.get definition | return some next
  next.withContext do
    let lctx := (← getLCtx).setKind read .implDetail
    let hidden ← mkFreshExprMVarAt lctx (← getLocalInstances) (← next.getType) .syntheticOpaque
      (← next.getTag)
    next.assign hidden
    return some hidden.mvarId!

/-- A typed map the context defines (`SpecMap.ofEntries key value x = e`, its
carrier `x` not in `e`), such as a callee's contract states of a table it
writes: the facts and the goal read the definition, as a local's equation
is substituted, and the carrier is the definition written back
(`writeBackCarrier`). -/
elab "verex_subst_maps" : tactic => focusGoal do
  if (← getGoals).isEmpty then return
  let goal ← getMainGoal
  let definitions ← mapDefinitions goal
  if definitions.isEmpty then return
  let targets ← goal.getNondepPropHyps
  simpMainAt (← `(tactic| simp only [])) definitions targets true
  for _ in [0:definitions.size] do
    let some goal := (← getGoals).head? | return
    let mut substituted := none
    for definition in ← mapDefinitions goal do
      substituted ← writeBackCarrier goal definition (hide := true)
      if substituted.isSome then break
    let some next := substituted | return
    replaceMainGoal [next]

/-- Rewrite the hypotheses that read memory the leaf fixes (`m r k = v`)
with those reads, as `verex_memory_reads` does for the goal: a fact
the contract states over memory then speaks of the value the program read,
such as the variant a precondition fixes. -/
elab "verex_context_memory_reads" : tactic => focusGoal do
  if (← getGoals).isEmpty then return
  let goal ← getMainGoal
  let (reads, targets) ← goal.withContext do
    let mut reads : Array (FVarId × Lean.Expr) := #[]
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let ty ← instantiateMVars decl.type
      let some (_, lhs, _) := ty.eq? | continue
      let head := lhs.getAppFn
      unless head.isFVar && lhs.getAppNumArgs == 2 do continue
      if (← whnfR (← inferType head)).isAppOfArity ``VerexIR.Proofs.Denote.Memory 2 then
        reads := reads.push (decl.fvarId, lhs)
    if reads.isEmpty then return (#[], #[])
    let mut targets := #[]
    for decl in ← getLCtx do
      if decl.isImplementationDetail || reads.any (·.1 == decl.fvarId) then continue
      let ty ← instantiateMVars decl.type
      unless ← isProp ty do continue
      if ← reads.anyM fun (_, lhs) => mentionsReadCached lhs ty then
        targets := targets.push decl.fvarId
    return (reads.map (·.1), targets)
  if targets.isEmpty then throwError "no hypothesis reads memory the leaf fixes"
  -- The leaves restate one context and its reads: a hypothesis rewritten
  -- by the same reads before is taken from `readForms`, as is one the
  -- rewriting already left alone.
  let readTypes ← goal.withContext do reads.mapM fun read => do instantiateMVars (← read.getType)
  let scope (type : Lean.Expr) : Lean.Expr := readTypes.foldl mkAnd type
  let next ← goal.withContext do
    let (ctx, simprocs, _) ← rulesSimpContext (← `(tactic| simp only [vir_denote_norm])) reads
    simpGoalRecorded (← targetState).readForms goal ctx simprocs Simp.dischargeDefault? false
      targets scope (resultsNormal := true)
  match next with
  | none => replaceMainGoal []
  | some next => replaceMainGoal [next]

/-- The hypotheses whose statement satisfies a predicate. -/
def hypothesesWhere (goal : MVarId) (predicate : Lean.Expr → Bool) : MetaM (Array FVarId) :=
  goal.withContext do
    (← goal.getNondepPropHyps).filterM fun fvarId => do
      return predicate (← instantiateMVars (← fvarId.getType))

/-- Unfold the constant at the head of the goal once: a unit's table of
declared preconditions at a target the proof does not know. -/
elab "verex_unfold_head" : tactic => do
  let goal ← getMainGoal
  let target ← goal.withContext (instantiateMVars (← goal.getType))
  let .const name _ := target.getAppFn | throwError "the goal's head is not a constant"
  replaceMainGoal [← Lean.Meta.unfoldTarget goal name]


/-- The reading a unit's requires meaning takes of a closure site's value,
in place of the meaning: the meaning unfolded at the value, whose arm
quantifies over the certified captures the value's plain row forgets.  A
value whose captures the normal forms collapsed to a plain row no longer
spells that forgetting, so its generated equation (`requires…`) does not
state it and the arm is reached by unfolding the meaning instead; a row
still spelling it is read by the equation.

The arm holds of every certified row forgetting to the value's, and a row
is determined by its forgetting (`HList.forget_injective`), so the binder
and its premise are introduced and the row substituted.  Where the
forgetting collapsed to the row itself the premise is already the row's own
equation, which substitutes as it is.  `none` where the goal states no
reading of a site's value. -/
def siteRequiresArm? (goal : MVarId) : MetaM (Option MVarId) := do
  let saved ← saveState
  let armed ← (observing? do
    let head ← goal.withContext do
      let target ← instantiateMVars (← goal.getType)
      let .const head _ := target.getAppFn | failure
      let environment ← getEnv
      unless target.getAppArgs.any fun argument =>
          isSiteValue environment argument.consumeMData do failure
      pure head
    let unfolded ← unfoldTarget goal head
    let armed ← unfolded.withContext do
      let arm ← whnf (← instantiateMVars (← unfolded.getType))
      unless arm.isForall && arm.bindingBody!.isForall &&
          arm.bindingBody!.bindingDomain!.isAppOfArity ``Eq 3 do failure
      unfolded.replaceTargetDefEq arm
    let (_, introduced) ← armed.intro1P
    let (premise, introduced) ← introduced.intro1P
    -- The premise as the row's own equation, where the forgetting collapsed.
    if let some substituted ← observing? (Lean.Meta.subst introduced premise) then return substituted
    introduced.withContext do
      let type ← instantiateMVars (← premise.getType)
      let some (_, forgotten, plain) := type.eq? | failure
      unless forgotten.isAppOfArity ``HList.forget 3 do failure
      let (family, row) := (forgotten.getArg! 0, forgotten.getArg! 1)
      let some certified ← certifiedCaptures? family row plain | failure
      let stated ← mkEq forgotten (mkApp3 (mkConst ``HList.forget) family row certified)
      let determined ← mkAppOptM ``HList.forget_injective
        #[some family, some row, none, none, some (← mkExpectedTypeHint (.fvar premise) stated)]
      let asserted ← introduced.assert `capturedEq (← inferType determined) determined
      let (equation, asserted) ← asserted.intro1P
      Lean.Meta.subst asserted equation)
  match armed with
  | some armed => return some armed
  | none => saved.restore; return none

/-- `requires_of` of a function value the leaf knows at the base frame: the
unit's requires meaning at the value (`requiresOf_runtime`), which reads a
closure site's value as its target's declared precondition at the woven
arguments (the generated `requires…` equations), or, where the value's
captures no longer spell the row the equation is stated at, the arm the
meaning unfolds to there (`siteRequiresArm?`). A value no site encodes
reads an uninterpreted precondition, which only a hypothesis states. -/
elab "verex_behavior" : tactic => focusGoal do
  let normalize ← `(tactic|
    simp (disch := first
        | (simp only [List.length_cons, List.length_nil, NRow.length] <;> fail)
        -- A discharger's last alternative fails in recovery mode, which logs
        -- its failure: the quiet failure leaves the goal unassigned.
        | skip) only
      [VerexIR.Proofs.requiresOf_runtime, vir_denote_norm])
  let state ← saveState
  try
    evalTactic normalize
  catch error =>
    state.restore
    match ← siteRequiresArm? (← getMainGoal) with
    | some armed =>
        replaceMainGoal [armed]
        evalTactic (← `(tactic| try ($normalize:tactic)))
    | none => throw error

/-- Whether a leaf's tactic closes a goal, leaving the goal list as it was. -/
def closesBy (goal : MVarId) (tactic : TSyntax `tactic) : TacticM Bool := do
  let saved ← getGoals
  let state ← saveState
  try
    setGoals [goal]
    Lean.Elab.Tactic.withoutRecover (evalTactic tactic)
    let closed := (← getGoals).isEmpty
    unless closed do state.restore
    setGoals saved
    return closed
  catch _ =>
    state.restore
    setGoals saved
    return false

/-- Canonical family spellings (`canonicalFamilies`). -/
syntax "verex_canonical_families" : tactic

/-- `verex_canonical_families` on the hypotheses only. -/
syntax "verex_canonical_hypotheses" : tactic

/-- Canonicalize only the selected hypotheses, preserving the module boundary
between family spelling and the call-result producer. -/
syntax "verex_canonical_hypotheses" "[" term,* "]" : tactic



end VerexIR.Proofs.Denote

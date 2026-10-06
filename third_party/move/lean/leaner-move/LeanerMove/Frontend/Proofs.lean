-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerMove.Frontend.Xast

/-!
# Move proof blocks as steps

The Move Prover runs a `proof { ... }` block as a sequence of actions
(`spec_translator.rs`, `translate_proof`): those outside `post` at the
function's entry, those under `post` at each return. An `if` guards each
action of its branches with its condition, evaluated where the action runs;
a `let` binds its value where it stands, so a `post` action reads a binding
made outside `post` at the entry state; `calc` asserts each step. This
module computes the actions, with bindings substituted and guards attached.
-/

namespace LeanerMove.Frontend.Proofs

open LeanerMove.Frontend Xast

/-- What one step does. -/
inductive Action where
  | «assert» (exp : Exp)
  | «assume» (exp : Exp)
  /-- Applies a lemma, under universally quantified binders when the step is
  a `forall … apply`. -/
  | apply (binders : List Param) (application : LemmaApplication)
  | split (exp : Exp)
  deriving Inhabited

/-- A step: its path conditions, outermost first, and its action. -/
structure Step where
  loc : Loc
  guards : List Exp
  action : Action
  deriving Inhabited

/-- The names a pattern binds. -/
partial def patternNames : Pattern → List String
  | .mk _ _ (.var name) => [name]
  | .mk _ _ (.tuple elements) => elements.flatMap patternNames
  | .mk _ _ (.struct _ _ _ fields) => fields.flatMap patternNames
  | .mk _ _ .wildcard | .mk _ _ (.literal _) | .mk _ _ (.range ..) => []

/-- Replace the locals `binding` names and the results `result` names,
except where a binder of the same name hides a local. -/
partial def substitute (binding : String → Option Exp) (result : Nat → Option Exp) :
    Exp → Exp
  | .mk ty loc node =>
    let go := substitute binding result
    let unbinding (names : List String) := substitute
      (fun name => if names.contains name then none else binding name) result
    match node with
    | .local name => (binding name).getD (.mk ty loc node)
    | .call (.result index) [] [] _ => (result index).getD (.mk ty loc node)
    | .value .. | .param _ | .loopCont .. | .specBlock _ => .mk ty loc node
    | .call op inst args surface => .mk ty loc (.call op inst (args.map go) surface)
    | .invoke function args => .mk ty loc (.invoke (go function) (args.map go))
    | .block pattern value body =>
        .mk ty loc (.block pattern (value.map go) (unbinding (patternNames pattern) body))
    | .ite cond thenBranch elseBranch =>
        .mk ty loc (.ite (go cond) (go thenBranch) (go elseBranch))
    | .match scrutinee arms =>
        .mk ty loc (.match (go scrutinee) (arms.map fun
          | .mk armLoc pattern guard body =>
            let inner := unbinding (patternNames pattern)
            .mk armLoc pattern (guard.map inner) (inner body)))
    | .sequence exps => .mk ty loc (.sequence (exps.map go))
    | .loop body => .mk ty loc (.loop (go body))
    | .return value => .mk ty loc (.return (go value))
    | .assign pattern value => .mk ty loc (.assign pattern (go value))
    | .mutate target value => .mk ty loc (.mutate (go target) (go value))
    | .quant kind ranges triggers condition body =>
        -- A range's pattern binds in the later domains, the condition, and
        -- the body.
        let (ranges, names) := ranges.foldl (init := ([], [])) fun (ranges, names) range =>
          match range with
          | .mk pattern domain label =>
              (ranges ++ [.mk pattern (unbinding names domain) label],
                names ++ patternNames pattern)
        let inner := unbinding names
        .mk ty loc (.quant kind ranges (triggers.map (·.map inner)) (condition.map inner)
          (inner body))

private def boolExp (loc : Loc) (node : ExpNode) : Exp := .mk .bool loc node

/-- `old(e)`: a binding made at the entry, read at a return. -/
private def atEntry (exp : Exp) : Exp :=
  .mk exp.ty exp.loc (.call .old [] [exp] none)

/-- What a `let` binds: its value, and whether it was bound at the entry. -/
private abbrev Bindings := List (String × Exp × Bool)

private def lookup (bindings : Bindings) (post : Bool) (name : String) : Option Exp :=
  (bindings.find? (fun (bound, _, atEntryState) => bound == name && (post || atEntryState))).map fun (_, value, atEntryState) =>
    if post && atEntryState then atEntry value else value

/-- The steps of a proof, at the entry and at each return. In a lemma's
proof (`splitBranches`), an `if` is a case analysis: a split on its
condition precedes the steps it guards. -/
partial def steps (proof : Proof) (splitBranches : Bool := false)
    (conditions : List Condition := []) : Array Step × Array Step :=
  let rec walk (bindings : Bindings) (guards : List Exp) (post : Bool)
      (acc : Array Step × Array Step) : Proof → Array Step × Array Step × Bindings
    | proof =>
      let close (exp : Exp) := substitute (lookup bindings post) (fun _ => none) exp
      let push (acc : Array Step × Array Step) (step : Step) :=
        if post then (acc.1, acc.2.push step) else (acc.1.push step, acc.2)
      match proof with
      | .let _ name exp =>
          let (entry, exit) := acc
          (entry, exit, (name, close exp, !post) :: bindings)
      | .assert loc exp =>
          let (entry, exit) := push acc { loc, guards, action := .assert (close exp) }
          (entry, exit, bindings)
      -- `assume [trusted] true` marks a lemma taken on trust and assumes
      -- nothing.
      | .assume _ (.mk _ _ (.value (.bool true) _)) => (acc.1, acc.2, bindings)
      | .assume loc exp =>
          let (entry, exit) := push acc { loc, guards, action := .assume (close exp) }
          (entry, exit, bindings)
      | .split loc exp =>
          let (entry, exit) := push acc { loc, guards, action := .split (close exp) }
          (entry, exit, bindings)
      | .apply loc (.mk lemma inst args) =>
          let (entry, exit) := push acc
            { loc, guards, action := .apply [] (.mk lemma inst (args.map close)) }
          (entry, exit, bindings)
      | .forallApply loc binders _ _ (.mk lemma inst args) =>
          -- The binders hide the bindings of their names.
          let names := binders.map (·.name)
          let inner := substitute (fun name =>
            if names.contains name then none else lookup bindings post name) (fun _ => none)
          let (entry, exit) := push acc
            { loc, guards, action := .apply binders (.mk lemma inst (args.map inner)) }
          (entry, exit, bindings)
      | .calc loc calcSteps =>
          let (entry, exit) := calcSteps.foldl (init := acc) fun acc step =>
            push acc { loc, guards, action := .assert (close step) }
          (entry, exit, bindings)
      | .block _ proofs =>
          -- A block's bindings end with it.
          let (entry, exit, _) := proofs.foldl (init := (acc.1, acc.2, bindings))
            fun (entry, exit, bindings) proof => walk bindings guards post (entry, exit) proof
          (entry, exit, bindings)
      | .post _ proof =>
          let (entry, exit, _) := walk bindings guards true acc proof
          (entry, exit, bindings)
      | .ite loc cond thenProof elseProof =>
          let cond := close cond
          let acc := if splitBranches then push acc { loc, guards, action := .split cond } else acc
          let (entry, exit, _) := walk bindings (guards ++ [cond]) post acc thenProof
          match elseProof with
          | none => (entry, exit, bindings)
          | some elseProof =>
              let negated := boolExp loc (.call .not [] [cond] none)
              let (entry, exit, _) :=
                walk bindings (guards ++ [negated]) post (entry, exit) elseProof
              (entry, exit, bindings)
  -- Contract lets are in scope in a proof as well. Close them in declaration
  -- order, retaining their state: pre-state bindings read through `old` in
  -- post steps, while post-state bindings are unavailable at entry.
  let bindings : Bindings := conditions.foldl (init := []) fun bindings condition =>
    match condition with
    | .mk kind _ _ expression .. =>
        match kind with
        | .letPre name | .letPost name =>
            let post := kind matches .letPost _
            let value := substitute (lookup bindings post) (fun _ => none) expression
            (name, value, !post) :: bindings
        | _ => bindings
  let (entry, exit, _) := walk bindings [] false (#[], #[]) proof
  (entry, exit)

/-- An expression under its path conditions: `g₁ ==> … ==> e`. -/
def guarded (guards : List Exp) (exp : Exp) : Exp :=
  guards.foldr (fun guard body => boolExp guard.loc (.call .implies [] [guard, body] none)) exp

end LeanerMove.Frontend.Proofs

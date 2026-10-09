-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerMove.Frontend.Xast

/-!
Loop invariants where the Move Prover takes them. A loop claims the leading
run of `invariant`s of the specification it begins with: the specification
blocks opening the condition of a `while`, which a `for` loop is lowered to
as well, or the start of a `loop` body. Consecutive such blocks are one
specification, which the importer joins (`joinHeader`), so that the loop's
specification has one site. An invariant anywhere else, in the body, after
an assertion or assumption, or after another statement, belongs to no loop,
and the Prover's loop analysis rejects it (`fat_loop.rs`); so does the
importer.
-/

namespace LeanerMove.Frontend.LoopInvariants

open Xast

/-- The expressions an expression evaluates, in order. -/
private partial def children (e : Exp) : List Exp :=
  match e.node with
  | .call _ _ args _ => args
  | .invoke function args => function :: args
  | .block _ binding body => binding.toList ++ [body]
  | .ite cond yes no => [cond, yes, no]
  | .«match» scrutinee arms =>
      scrutinee :: arms.flatMap fun | .mk _ _ guard body => guard.toList ++ [body]
  | .sequence exps => exps
  | .loop body => [body]
  | .«return» value => [value]
  | .assign _ value => [value]
  | .mutate target value => [target, value]
  | _ => []

/-- Two specification blocks run one after the other, as one block. Blocks
with a frame, access clauses, or a proof stay apart. -/
private def joined : Spec → Spec → Option Spec
  | .mk loc pragmas conditions none [] none, .mk _ pragmas' conditions' none [] none =>
      some (.mk loc (pragmas ++ pragmas') (conditions ++ conditions') none [] none)
  | _, _ => none

/-- An expression with the consecutive specification blocks it begins with
joined into one. -/
partial def joinHeader (e : Exp) : Exp :=
  let .mk ty loc node := e
  match node with
  | .ite cond yes no => .mk ty loc (.ite (joinHeader cond) yes no)
  | .sequence exps => .mk ty loc (.sequence (join exps))
  | .block pattern (some binding) body =>
      .mk ty loc (.block pattern (some (joinHeader binding)) body)
  | .block pattern none body => .mk ty loc (.block pattern none (joinHeader body))
  | _ => e
where
  join : List Exp → List Exp
    | first@(.mk ty loc (.specBlock spec)) :: second@(.mk _ _ (.specBlock spec')) :: rest =>
        match joined spec spec' with
        | some spec => join (.mk ty loc (.specBlock spec) :: rest)
        | none => first :: second :: rest
    | first :: rest => joinHeader first :: rest
    | [] => []

/-- The specification block an expression begins with, if it begins with
one: what it evaluates first. -/
private partial def leadingSpec? (e : Exp) : Option Spec :=
  match e.node with
  | .specBlock spec => some spec
  | .ite cond _ _ => leadingSpec? cond
  | .sequence (first :: _) => leadingSpec? first
  | .block _ (some binding) _ => leadingSpec? binding
  | .block _ none body => leadingSpec? body
  | _ => none

private def conditions : Spec → List Condition
  | .mk _ _ conditions .. => conditions

private def isLoopInvariant : Condition → Bool
  | .mk kind .. => kind == .loopInvariant

private def conditionLoc : Condition → Loc
  | .mk _ loc .. => loc

/-- The locations of the loop invariants of a function body that no loop
claims. -/
partial def misplaced (body : Exp) : List Loc :=
  let rec walk (e : Exp) (claimed declared : List Loc) : List Loc × List Loc :=
    let (claimed, declared) := match e.node with
      | .loop loopBody =>
          let header := ((leadingSpec? (joinHeader loopBody)).map fun spec =>
            ((conditions spec).takeWhile isLoopInvariant).map conditionLoc).getD []
          (claimed ++ header, declared)
      | .specBlock spec =>
          (claimed, declared ++ ((conditions spec).filter isLoopInvariant).map conditionLoc)
      | _ => (claimed, declared)
    (children e).foldl (fun (claimed, declared) child => walk child claimed declared)
      (claimed, declared)
  let (claimed, declared) := walk body [] []
  declared.filter (!claimed.contains ·)

/-- The message the Move Prover reports at a misplaced loop invariant. -/
def message : String :=
  "Loop invariants must be declared at the beginning of the loop header in a consecutive sequence"

end LeanerMove.Frontend.LoopInvariants

-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerMove.Frontend.Effects
import LeanerMove.Frontend.Names
import LeanerLang.Frame

/-!
# Move frames made explicit

A LeanerLang frame is closed: without `modifies` a function changes no
global memory, and `modifies R[a]` means nothing else changes. A Move
specification frames only the resource families it names targets for;
every other family is unconstrained. The encoder states Move's reading in
LeanerLang's terms:

- no targets, and the function (transitively) writes global memory →
  `modifies *`;
- no targets and no writes → no clause, the closed frame, which the function
  then owes and trivially meets;
- targets on an `opaque` function → the closed frame over them, since its
  callers see only the contract;
- targets otherwise → `modifies <targets>, *`, closing the named families
  at their keys and leaving the rest open.
-/

namespace LeanerMove.Frontend.Frames

open LeanerMove.Frontend Xast Effects Names

private partial def expChildren : ExpNode → List Exp
  | .value .. | .«local» _ | .param _ | .loopCont .. | .specBlock _ | .quant .. => []
  | .call _ _ arguments _ => arguments
  | .invoke function arguments => function :: arguments
  | .block _ binding body => binding.toList ++ [body]
  | .ite condition thenBranch elseBranch => [condition, thenBranch, elseBranch]
  | .«match» scrutinee arms =>
      scrutinee :: arms.flatMap fun arm => arm.guard.toList ++ [arm.body]
  | .sequence expressions => expressions
  | .loop body => [body]
  | .«return» value => [value]
  | .assign _ value => [value]
  | .mutate target value => [target, value]

/-- Whether evaluating an expression may write global memory, given which
callees do. Invoking a function value may do anything. -/
partial def writesGlobal (callee : QualifiedName → Bool) (expression : Exp) : Bool :=
  let own := match expression.node with
    | .call (.borrowGlobal .mutable) .. | .call .moveTo .. | .call .moveFrom .. => true
    | .call (.moveFunction name) .. => callee name
    | .invoke .. => true
    | _ => false
  own || (expChildren expression.node).any (writesGlobal callee)

/-- The functions of a package that may write global memory: the least
fixed point over the owned and dependency modules. A callee outside them is
assumed to write; `std::vector` and natives do not touch global memory. -/
def globalWriters (package : Package) : List QualifiedName :=
  let functions := (package.modules ++ package.dependencies).flatMap fun module =>
    module.functions.map fun function => ({ module := module.ref, name := function.name }, function)
  let known (name : QualifiedName) := functions.any (·.1 == name)
  let step (writers : List QualifiedName) : List QualifiedName :=
    functions.filterMap fun (name, function) =>
      let callee (target : QualifiedName) :=
        !isPrimitiveModule target.module && (writers.contains target || !known target)
      let writes := match function.kind, function.body with
        | .native, _ => false
        | _, some body => writesGlobal callee body
        | _, none => true
      if writes then some name else none
  let rec iterate (writers : List QualifiedName) : Nat → List QualifiedName
    | 0 => writers
    | fuel + 1 =>
        let next := step writers
        if next == writers then writers else iterate next fuel
  iterate [] (functions.length + 1)

/-- The pragma that leaves the families a frame does not name open. -/
def loosePragma : LeanerIR.Attribute := .assign "leaner_loose_frame" (.constant (.bool true))

/-- A function contract with Move's frame reading made explicit; `closed` keeps
named targets closed, as an `opaque` function's are. -/
def explicit (contract : LeanerIR.FunctionContract) (writes closed : Bool) :
    LeanerIR.FunctionContract :=
  if contract.modifiesAll then contract
  else if contract.modifies.isEmpty then
    if writes then { contract with hasFrame := true, modifiesAll := true } else contract
  else if closed || LeanerLang.hasLooseFrame contract then contract
  else { contract with pragmas := contract.pragmas.push loosePragma }

end LeanerMove.Frontend.Frames

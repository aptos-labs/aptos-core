-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Frontend.Effects
import VerexMove.Frontend.Names
import VerexMove.Frontend.Proofs
import VerexSL.Frame

/-!
# Move frames made explicit

A VerexSL frame is closed: without `modifies` a function changes no
global memory, and `modifies R[a]` means nothing else changes. A Move
specification frames only the resource families it names targets for;
every other family is unconstrained. The encoder states Move's reading in
VerexSL's terms:

- no targets, and the function (transitively) writes global memory →
  `modifies *`;
- no targets and no writes → no clause, the closed frame, which the function
  then owes and trivially meets;
- targets on an `opaque` function → the closed frame over them, since its
  callers see only the contract;
- targets otherwise → `modifies <targets>, *`, closing the named families
  at their keys and leaving the rest open.

Spec variable memory is outside this reading, as the Move Prover neither havocs
it nor asks a frame to declare it: what a function updates, itself or through
its callees, joins its targets, so that an update does not open the function's
frame. A function whose only writes are updates keeps a closed frame, unless one
is in a loop, whose store is unknown after its head.
-/

namespace VerexMove.Frontend.Frames

open VerexMove.Frontend Xast Effects Names

/-- The variables a pattern binds. -/
private partial def patternNames (pattern : Pattern) : List String :=
  match pattern.node with
  | .var name => [name]
  | .tuple elements | .struct _ _ _ elements => elements.flatMap patternNames
  | .wildcard | .literal _ | .range .. => []

/-- The variables an expression binds or assigns. -/
private partial def boundNames (expression : Exp) : List String :=
  let own := match expression.node with
    | .block pattern .. | .assign pattern _ => patternNames pattern
    | .«match» _ arms => arms.flatMap (patternNames ·.pattern)
    | _ => []
  own ++ expression.node.children.flatMap boundNames

/-- The function-typed parameters a body keeps: those without `modifies_of`,
which keep global memory, the frame their callers establish
(`KeepsMemoryAt`), and which the body neither rebinds nor assigns. -/
def keptParameters (params : List Param) (accessOf : List AccessOf) (body : Exp) :
    List String :=
  let bound := boundNames body
  params.filterMap fun param =>
    let declared := accessOf.any fun access =>
      access.name == param.name && (access.modifiesAll || !access.modifies.isEmpty)
    if param.ty matches .function .. && !declared && !bound.contains param.name then
      some param.name
    else none

/-- Whether evaluating an expression may write global memory, given which
callees do. Invoking a function value may do anything, but for a `kept`
parameter, among the function's `params`. -/
partial def writesGlobal (callee : QualifiedName → Bool) (params kept : List String)
    (expression : Exp) : Bool :=
  let own := match expression.node with
    | .call (.borrowGlobal .mutable) .. | .call .moveTo .. | .call .moveFrom .. => true
    | .call (.moveFunction name) .. => callee name
    | .invoke function _ => match function.node with
        | .«local» name => !kept.contains name
        | .param index => !(params[index]?.any kept.contains)
        | _ => true
    | _ => false
  own || expression.node.children.any (writesGlobal callee params kept)

/-- Whether a specification declares modifications of global memory, which
its callers see through its contract. -/
def declaresWrites (spec : Spec) : Bool :=
  spec.frame.any fun frame => !frame.modifies.isEmpty || frame.modifiesAll

/-- The memory of a spec variable at its type arguments: `Ghost$x<args>` at
address 0. -/
abbrev GhostMemory := QualifiedName × List Ty

/-- The spec variable memory a condition updates. -/
private def updatedMemory? (condition : Condition) : Option GhostMemory :=
  match condition.kind, condition.updateTarget with
  | .update, some (.mk _ _ (.call (.select _ _) _
      [.mk _ _ (.call (.global _) [.struct name arguments] _ _)] _)) => some (name, arguments)
  | _, _ => none

/-- What a function's body updates itself, and the functions it calls at their
type arguments. -/
private partial def bodyEffects (expression : Exp) : List GhostMemory × List (QualifiedName × List Ty) :=
  let own : List GhostMemory × List (QualifiedName × List Ty) := match expression.node with
    | .specBlock (.mk _ _ conditions _ _ _) => (conditions.filterMap updatedMemory?, [])
    | .call (.moveFunction name) inst .. => ([], [(name, inst)])
    | _ => ([], [])
  expression.node.children.foldl (init := own) fun (updates, calls) child =>
    let (childUpdates, childCalls) := bodyEffects child
    (updates ++ childUpdates, calls ++ childCalls)

private def addNew [BEq α] (existing : List α) (added : List α) : List α :=
  added.foldl (init := existing) fun list item => if list.contains item then list else list ++ [item]

/-- The spec variable memory each function of a package may update, itself or
through its callees at their type arguments, and the functions the fixed point
did not settle for, whose instantiations keep growing (polymorphic recursion):
those count as writers of all memory. -/
def ghostWrites (package : Package) :
    List (QualifiedName × List GhostMemory) × List QualifiedName :=
  let functions := (package.modules ++ package.dependencies).flatMap fun module =>
    module.functions.map fun function =>
      let (updates, calls) := (function.body.map bodyEffects).getD ([], [])
      (({ module := module.ref, name := function.name } : QualifiedName),
        addNew (function.spec.conditions.filterMap updatedMemory?) updates, addNew [] calls)
  let step (known : List (QualifiedName × List GhostMemory)) :
      List (QualifiedName × List GhostMemory) :=
    functions.filterMap fun (name, updates, calls) =>
      let memory := calls.foldl (init := updates) fun memory (callee, arguments) =>
        addNew memory (((known.lookup callee).getD []).map fun (ghost, parameters) =>
          (ghost, parameters.map (instantiateTypeParameters arguments)))
      if memory.isEmpty then none else some (name, memory)
  let rec iterate (known : List (QualifiedName × List GhostMemory)) :
      Nat → List (QualifiedName × List GhostMemory) × List QualifiedName
    | 0 =>
        let next := step known
        (known, next.filterMap fun (name, memory) =>
          if (known.lookup name).any (·.length == memory.length) then none else some name)
    | fuel + 1 =>
        let next := step known
        if next.length == known.length && next.all (fun (name, memory) =>
            (known.lookup name).any (·.length == memory.length)) then (next, [])
        else iterate next fuel
  iterate [] (functions.length + 1)

/-- Whether an expression updates a spec variable inside a loop, itself or
through a callee. A loop touching the store keeps no frame of its own, so its
function can claim none. -/
private partial def updatesInLoop (writes : List (QualifiedName × List GhostMemory))
    (expression : Exp) : Bool :=
  match expression.node with
  | .loop body =>
      let (updates, calls) := bodyEffects body
      !updates.isEmpty || calls.any (fun (callee, _) => (writes.lookup callee).isSome)
  | node => node.children.any (updatesInLoop writes)

/-- A specification with its `update`s stated as callers see them: the target
equal to the value, in which the target's own reads are those before the call.
Each return applies the update, as in the Move Prover, so the function proves
this and a body that also updates the target does not. -/
def contractUpdates (function : Function) : Except String Spec := do
  let .mk loc pragmas conditions frame accessOf proof := function.spec
  let mut stated : List Condition := []
  for condition in conditions do
    let .mk .update conditionLoc properties value _ _ _ _ (some target) := condition
      | stated := stated ++ [condition]; continue
    unless (updatedMemory? condition).isSome do
      throw s!"`update` of a ghost field in the specification of `{function.name}` is not carried"
    let isTarget (expression : Exp) : Option Exp :=
      match expression.node, target.node with
      | .call (.select owner field) _ [.mk _ _ (.call (.global _) inst _ _)] _,
        .call (.select owner' field') _ [.mk _ _ (.call (.global _) inst' _ _)] _ =>
          if owner == owner' && field == field' && inst == inst' then
            some (.mk expression.ty expression.loc (.call .old [] [expression] none))
          else none
      | _, _ => none
    let before := Proofs.rewriteExp isTarget (fun _ => none) (fun _ => none) value
    let equal : Exp := .mk .bool conditionLoc (.call .eq [target.ty] [target, before] none)
    stated := stated ++ [.mk .ensures conditionLoc properties equal none [] none none none]
  return .mk loc pragmas stated frame accessOf proof

/-- A specification with the spec variable memory its function updates among
its frame's targets, and whether the frame stays closed: it does where these
are its only targets and the function writes nothing else, or where `closed`. -/
def withGhostMemory (spec : Spec) (memory : List GhostMemory) (loc : Loc) (writes closed : Bool) :
    Spec × Bool :=
  let .mk specLoc pragmas conditions frame accessOf proof := spec
  if memory.isEmpty then (spec, closed) else
  let targets := memory.map fun (ghost, arguments) =>
    let ty : Ty := .struct ghost arguments
    let address : Exp := .mk .address loc (.value (.address "0x0") none)
    (.mk ty loc (.call (.global none) [ty] [address] none) : Exp)
  match frame with
  | some (.mk _ _ true _) => (spec, closed)
  | some (.mk modifies reads false readsAll) =>
      if modifies.isEmpty && writes then (spec, closed) else
      (.mk specLoc pragmas conditions (some (.mk (modifies ++ targets) reads false readsAll))
        accessOf proof, closed || modifies.isEmpty)
  | none =>
      if writes then (spec, closed) else
      (.mk specLoc pragmas conditions (some (.mk targets [] false false)) accessOf proof, true)

/-- The functions of a package that may write global memory: the least
fixed point over the owned and dependency modules, a function writing where
its body or its declared frame does. A callee outside them is assumed to
write; `std::vector` and natives do not touch global memory. -/
def globalWriters (package : Package)
    (ghost : List (QualifiedName × List GhostMemory) × List QualifiedName) : List QualifiedName :=
  let functions := (package.modules ++ package.dependencies).flatMap fun module =>
    module.functions.map fun function => ({ module := module.ref, name := function.name }, function)
  let known (name : QualifiedName) := functions.any (·.1 == name)
  let step (writers : List QualifiedName) : List QualifiedName :=
    functions.filterMap fun (name, function) =>
      let callee (target : QualifiedName) :=
        !isPrimitiveModule target.module && (writers.contains target || !known target)
      let writes : Bool := ghost.2.contains name || match function.kind, function.body with
        | .native, _ => false
        | _, some body =>
            declaresWrites function.spec || updatesInLoop ghost.1 body ||
              writesGlobal callee (function.params.map (·.name))
                (keptParameters function.params function.spec.accessOf body) body
        | _, none => true
      if writes then some name else none
  let rec iterate (writers : List QualifiedName) : Nat → List QualifiedName
    | 0 => writers
    | fuel + 1 =>
        let next := step writers
        if next == writers then writers else iterate next fuel
  iterate [] (functions.length + 1)

/-- The pragma that leaves the families a frame does not name open. -/
def loosePragma : VerexIR.Attribute := .assign "verex_loose_frame" (.constant (.bool true))

/-- A function contract with Move's frame reading made explicit; `closed` keeps
named targets closed, as an `opaque` function's are. -/
def explicit (contract : VerexIR.FunctionContract) (writes closed : Bool) :
    VerexIR.FunctionContract :=
  if contract.modifiesAll then contract
  else if contract.modifies.isEmpty then
    if writes then { contract with hasFrame := true, modifiesAll := true } else contract
  else if closed || VerexSL.hasLooseFrame contract then contract
  else { contract with pragmas := contract.pragmas.push loosePragma }

end VerexMove.Frontend.Frames

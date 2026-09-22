-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.IR.Mono.Transform

namespace Tests.IR.Mono

open MoveModel.IR

private def observedEffects : List TagEffect :=
  [⟨.write, 0, [.typeParam 0]⟩,
   ⟨.read, 0, [.u64]⟩,
   ⟨.write, 1, [.typeParam 1]⟩,
   ⟨.read, 1, [.typeParam 0]⟩]

private def collisionArgs := discoverCollisionArgs 2 observedEffects

-- The analysis includes the generic case, each individual collision, and
-- their simultaneous combination.
#guard collisionArgs.any (fun args => args == [.typeParam 0, .typeParam 1])
#guard collisionArgs.any (fun args => args == [.u64, .typeParam 1])
#guard collisionArgs.any (fun args => args == [.typeParam 1, .typeParam 1])
#guard collisionArgs.any (fun args => args == [.u64, .u64])

-- An occurs check rejects recursively growing tag equations.
#guard (unifyTypeArgs 1 [.typeParam 0] [.vector (.typeParam 0)]).isNone

-- Coverage quantifies over real Move instantiations, not the rigid given
-- tokens used internally as representatives.
#guard [Ty.u64, Ty.vector .bool].all Ty.isClosed
#guard ![Ty.u64, Ty.typeParam 0].all Ty.isClosed

private def genericBody : Cfg where
  blocks := fun block => if block = 0 then
    some ⟨[.call [1] (.existsInst 0 [.typeParam 0]) [0]], .ret [1]⟩
  else none
  entry := 0
  size := 1

private def genericDecl : FunDecl where
  typeParams := [{ name := "T" }]
  numParams := 1
  numLocals := 2
  locals := fun i =>
    if i = 0 then some .address
    else if i = 1 then some .bool
    else none
  returns := [.bool]
  body := genericBody
  loopSpecs := fun _ => none
  contract := {
    requires := .value (.bool true)
    aborts := none
    ensures := .value (.bool true)
    modifies := []
  }

private def concreteBody : Cfg where
  blocks := fun block => if block = 0 then
    some ⟨[.call [1] (.functionInst 0 [.u64]) [0]], .ret [1]⟩
  else none
  entry := 0
  size := 1

private def concreteDecl : FunDecl where
  numParams := 1
  numLocals := 2
  locals := genericDecl.locals
  returns := [.bool]
  body := concreteBody
  loopSpecs := fun _ => none
  contract := genericDecl.contract

private def source : Module where
  address := 0
  name := "MonoTest"
  program := {
    funs := fun f => if f = 0 then some genericDecl
      else if f = 1 then some concreteDecl else none
    structs := fun r => if r = 0 then
      some ⟨[{ name := "T" }], [], none⟩ else none
  }
  numStructs := 1
  numFuns := 2
  structMeta := fun r => if r = 0 then
    some ⟨"R", [], none, { key := true }, []⟩ else none
  funMeta := fun f => if f = 0 then
    some ⟨"generic", .public_, false, [], [], [], none⟩
    else if f = 1 then some ⟨"concrete", .public_, false, [], [], [], none⟩
    else none

private def result := source.monomorphizeForVerification

-- Call closure adds the concrete `generic<u64>` instance even though the
-- generic callee itself has no tag collision.
#guard match result with
  | .ok (plan, _) =>
      plan.entries.any (fun key => key == ⟨0, [.u64]⟩)
  | .error _ => false

-- The executable certificate checker rejects a hand-written plan which
-- omits the concrete call-closure instance.
#guard match discoverMonoPlan source with
  | .ok plan =>
      match plan.generatedFunId? ⟨0, [.u64]⟩ with
      | some index =>
          match MonoPlan.validate source { entries := plan.entries.eraseIdx index } with
          | .error _ => true
          | .ok _ => false
      | none => false
  | .error _ => false

-- Materialization removes declaration binders and rewrites the concrete
-- instantiated call to an ordinary generated function call.
#guard match result with
  | .ok (plan, mono) =>
      match plan.generatedFunId? ⟨1, []⟩,
          plan.generatedFunId? ⟨0, [.u64]⟩ with
      | some caller, some callee =>
          match mono.program.funs caller with
          | some d =>
              d.typeParams.isEmpty &&
              match d.body.blocks 0 with
              | some ⟨[.call _ (.function target) _], _⟩ => target == callee
              | _ => false
          | none => false
      | _, _ => false
  | .error _ => false

end Tests.IR.Mono

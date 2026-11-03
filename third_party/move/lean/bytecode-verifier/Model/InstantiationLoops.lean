-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Spec.Syntax
import Model.Result
import Model.Bounds

/-!
# Instantiation loops

UNTRUSTED. Transcribes `move-bytecode-verifier/src/instantiation_loops.rs`.
The nodes are a function definition of this module with one of its type
parameters. A generic call or closure from `f` to `g` adds, for each type
argument of `g`, an identity edge from `f`'s type parameter when the argument
is that parameter, and otherwise a type-constructor edge from each of `f`'s
type parameters the argument mentions. A strongly connected component with a
type-constructor edge inside makes instantiation grow without bound.

Rust finds the components with Kosaraju's algorithm; the model asks, for each
type-constructor edge `u → v`, whether `v` reaches `u`, which holds exactly
when the edge lies inside a component.
-/

namespace MoveBytecode.Model

namespace InstantiationLoops

/-- A node: a function definition and one of its type parameters. -/
abbrev Node := Nat × Nat

/-- An edge, and whether it applies a type constructor. -/
structure Edge where
  src : Node
  dst : Node
  tyCon : Bool
  deriving DecidableEq, Repr

/-- `func_handle_def_map`: the definition of a function handle of this
module, the last one winning. -/
def handleToDef (m : CompiledModule) (h : Nat) : Option Nat :=
  (m.functionDefs.toList.zipIdx.filter (·.1.function == h)).getLast?.map (·.2)

/-- `extract_type_parameters`. -/
def typeParams (ty : SignatureToken) : List Nat :=
  ((preorder ty).filterMap fun t => match t with
    | .typeParameter i => some i
    | _ => none).eraseDups

/-- `build_graph_call`. -/
def callEdges (m : CompiledModule) (caller callee : Nat) (actuals : Nat) : VResult (List Edge) := do
  let tys ← lookup m.signatures actuals "signature_at"
  pure (tys.zipIdx.flatMap fun (ty, formal) => match ty with
    | .typeParameter actual => [⟨(caller, actual), (callee, formal), false⟩]
    | _ => (typeParams ty).map fun p => ⟨(caller, p), (callee, formal), true⟩)

/-- `build_graph_function_def`. -/
def functionEdges (m : CompiledModule) (caller : Nat) (fd : FunctionDefinition) :
    VResult (List Edge) :=
  match fd.code with
  | none => pure []
  | some code => do
    let es ← code.code.toList.mapM fun instr => match instr with
      | .callGeneric i | .packClosureGeneric i _ => do
        let fi ← lookup m.functionInstantiations i "function_instantiation_at"
        match handleToDef m fi.handle with
        | some callee => callEdges m caller callee fi.typeParameters
        | none => pure []
      | _ => pure []
    pure es.flatten

/-- `build_graph`: the edges of every function with code. -/
def edges (m : CompiledModule) : VResult (List Edge) := do
  let es ← m.functionDefs.toList.zipIdx.mapM fun (fd, i) => functionEdges m i fd
  pure es.flatten

/-- Whether `dst` is reachable from `src` along the edges. Each round adds a
node or stops, so the edge count bounds the rounds. -/
def reaches (es : List Edge) (src dst : Node) : Bool :=
  let rec go : Nat → List Node → Bool
    | 0, seen => seen.contains dst
    | fuel + 1, seen =>
      if seen.contains dst then true
      else
        let next := (es.filter fun e => seen.contains e.src && !seen.contains e.dst).map (·.dst)
        if next.isEmpty then false else go fuel (seen ++ next.eraseDups)
  go (es.length + 1) [src]

end InstantiationLoops

/-- `InstantiationLoopChecker::verify_module`. -/
def verifyInstantiationLoops (m : CompiledModule) : VResult Unit := do
  let es ← InstantiationLoops.edges m
  if es.any (fun e => e.tyCon && InstantiationLoops.reaches es e.dst e.src) then
    reject "LOOP_IN_INSTANTIATION_GRAPH"
  else pure ()

end MoveBytecode.Model

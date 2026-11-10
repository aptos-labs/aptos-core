-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Spec.Syntax
import Model.Result

/-!
# Recursive struct definitions

UNTRUSTED. Transcribes `move-bytecode-verifier/src/struct_defs.rs`: the graph
whose edges go from a struct definition to the struct definitions of this
module its fields mention (through vectors, type arguments and function
types) is acyclic. Rust sorts the graph topologically and reports a cycle;
the model detects a cycle by repeatedly removing the definitions no remaining
definition points to, which accepts and rejects the same graphs.
-/

namespace MoveBytecode.Model

namespace StructDefs

/-- `handle_to_def`: the definition of each struct handle of this module, the
last one winning (the duplication checker has made the map one to one). -/
def handleToDef (m : CompiledModule) (h : Nat) : Option Nat :=
  (m.structDefs.toList.zipIdx.filter (·.1.structHandle == h)).getLast?.map (·.2)

mutual
/-- `add_signature_token`: the definitions a field type mentions, or the
invariant violation for a reference outside a function type. -/
def tokenEdges (m : CompiledModule) : SignatureToken → Bool → VResult (List Nat)
  | .bool, _ | .u8, _ | .u16, _ | .u32, _ | .u64, _ | .u128, _ | .u256, _ | .i8, _ | .i16, _
  | .i32, _ | .i64, _ | .i128, _ | .i256, _ | .address, _ | .signer, _
  | .typeParameter _, _ => pure []
  | .reference t, refAllowed | .mutableReference t, refAllowed =>
    if refAllowed then tokenEdges m t false
    else reject "UNKNOWN_INVARIANT_VIOLATION_ERROR"
  | .vector t, _ => tokenEdges m t false
  | .function args results _, _ => do
    pure ((← tokenEdgesList m args true) ++ (← tokenEdgesList m results true))
  | .struct h, _ => pure (handleToDef m h).toList
  | .structInstantiation h args, _ => do
    pure ((handleToDef m h).toList ++ (← tokenEdgesList m args false))

def tokenEdgesList (m : CompiledModule) : List SignatureToken → Bool → VResult (List Nat)
  | [], _ => pure []
  | t :: ts, refAllowed => do
    pure ((← tokenEdges m t refAllowed) ++ (← tokenEdgesList m ts refAllowed))
end

/-- The fields of a definition, across all variants. -/
def fields (sd : StructDefinition) : List FieldDefinition :=
  match sd.fieldInformation with
  | .native => []
  | .declared fs => fs
  | .declaredVariants vs => vs.flatMap (·.fields)

/-- The edges from each definition, in definition order. -/
def edges (m : CompiledModule) : VResult (List (Nat × List Nat)) :=
  m.structDefs.toList.zipIdx.mapM fun (sd, i) => do
    pure (i, (← tokenEdgesList m ((fields sd).map (·.signature)) false))

/-- Whether the graph has a cycle: remove, until nothing changes, the nodes
that no remaining node points to; a cycle remains exactly when the graph is
cyclic. Each round removes a node or stops, so the node count bounds the
rounds. -/
def hasCycle (g : List (Nat × List Nat)) : Bool :=
  let rec go : Nat → List (Nat × List Nat) → Bool
    | 0, g => !g.isEmpty
    | fuel + 1, g =>
      let pointed (n : Nat) := g.any fun (_, succs) => succs.contains n
      let roots := g.filter fun (n, _) => !pointed n
      if roots.isEmpty then !g.isEmpty
      else go fuel (g.filter fun (n, _) => pointed n)
  go (g.length + 1) g

end StructDefs

/-- `RecursiveStructDefChecker::verify_module`. -/
def verifyStructDefs (m : CompiledModule) : VResult Unit := do
  let g ← StructDefs.edges m
  if StructDefs.hasCycle g then reject "RECURSIVE_STRUCT_DEFINITION" else pure ()

end MoveBytecode.Model

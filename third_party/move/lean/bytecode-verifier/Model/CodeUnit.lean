-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Model.ControlFlow
import Model.StackUsage
import Model.TypeSafety
import Model.LocalsSafety
import Model.ReferenceSafety
import Model.Acquires
import Model.StructApi

/-!
# Code-unit verification

UNTRUSTED. Transcribes `move-bytecode-verifier/src/code_unit_verifier.rs`:
the struct-API checks of version 10, control flow, the basic-block and
back-edge limits, stack usage, type safety, locals safety, reference safety,
and the acquires list.

Not modeled: the verifier meter, which bounds the work of the abstract
interpretations and rejects functions too costly to analyze. The model
accepts such functions; only properties that follow from the modeled checks
are proved from it.
-/

namespace MoveBytecode.Model

/-- `CodeUnitVerifier::verify_function`: the number of back edges, or a
rejection. -/
def verifyFunction (config : VerifierConfig) (m : CompiledModule) (index : Nat)
    (fdef : FunctionDefinition) : VResult Nat := do
  match fdef.code with
  | none => pure 0
  | some code =>
    let handle ← lookup m.functionHandles fdef.function "function_handle_at"
    let cfg ← verifyControlFlow config m.version code
    let view ← FunctionView.function m index code handle cfg
    match config.maxBasicBlocks with
    | some limit => if cfg.blocks.length > limit then reject "TOO_MANY_BASIC_BLOCKS"
    | none => pure ()
    let backEdges := cfg.numBackEdges
    match config.maxBackEdgesPerFunction with
    | some limit => if backEdges > limit then reject "TOO_MANY_BACK_EDGES"
    | none => pure ()
    verifyStackUsage config m view
    verifyTypeSafety m view
    let _ ← verifyLocalsSafety m view
    verifyReferenceSafety m view
    verifyAcquires m fdef code
    pure backEdges

/-- `CodeUnitVerifier::verify_module`: from version 10, each function's
struct-API attributes are checked before its code. -/
def verifyCodeUnits (config : VerifierConfig) (m : CompiledModule) : VResult Unit := do
  let ctx? ← StructApi.contextFor m
  let total ← m.functionDefs.toList.zipIdx.foldlM (fun total (fdef, index) => do
    StructApi.check m ctx? fdef
    pure (total + (← verifyFunction config m index fdef))) 0
  match config.maxBackEdgesPerModule with
  | some limit => if total > limit then reject "TOO_MANY_BACK_EDGES"
  | none => pure ()

end MoveBytecode.Model

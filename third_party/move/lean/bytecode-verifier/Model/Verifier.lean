-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Spec.Properties.Safety
import Model.Bounds
import Model.Duplication
import Model.Constants
import Model.CodeUnit
import Model.Dependencies
import Model.Limits
import Model.SignatureV2
import Model.InstructionConsistency
import Model.Friends
import Model.StructDefs
import Model.InstantiationLoops

/-!
# The verifier

UNTRUSTED. Transcribes `verify_module_with_config`
(`move-bytecode-verifier/src/verifier.rs`) with its passes,
in Rust order, and provides the model's `VerifierInterface`.

| Pass | Status |
|---|---|
| `BoundsChecker` | modeled |
| `FeatureVerifier` | modeled for the production configuration |
| `LimitsVerifier` | modeled (`Model/Limits.lean`) |
| `DuplicationChecker` | modeled |
| `signature_v2` | modeled (`Model/SignatureV2.lean`) |
| `InstructionConsistency` | modeled (`Model/InstructionConsistency.lean`) |
| `constants` | modeled |
| `friends` | modeled (`Model/Friends.lean`) |
| `RecursiveStructDefChecker` | modeled (`Model/StructDefs.lean`) |
| `InstantiationLoopChecker` | modeled (`Model/InstantiationLoops.lean`) |
| `CodeUnitVerifier` | modeled but for the meter (`Model/CodeUnit.lean`) |
| `script_signature` | nothing to check for bytecode version 5 and later |

The verifier meter is not modeled; it only rejects, so the model accepts
more than the Rust verifier, and proofs about the model's acceptances hold
for Rust's too. Rust turns a panic in these passes into a rejection, as the
model's `panic` result is.
-/

namespace MoveBytecode.Model

/-- `FeatureVerifier::verify_module`. Only the configurations with function
values enabled are modeled. -/
def verifyFeatures (config : VerifierConfig) (m : CompiledModule) : VResult Unit := do
  if !config.enableFunctionValues then panicAt "feature verification without function values"
  if !config.enableResourceAccessControl then
    for fh in m.functionHandles.toList do
      if fh.accessSpecifiers.isSome then reject "FEATURE_NOT_ENABLED"
  if !config.enableEnumTypes then
    for sd in m.structDefs.toList do
      match sd.fieldInformation with
      | .declaredVariants _ => reject "FEATURE_NOT_ENABLED"
      | .declared _ | .native => pure ()

/-- `verify_module_with_config`. -/
def verifyModule (config : VerifierConfig) (m : CompiledModule) : VResult Unit := do
  if config.scope == .nothing then return ()
  verifyBounds m
  verifyFeatures config m
  verifyLimits config m
  verifyDuplication m
  verifySignatures config m
  verifyInstructionConsistency m
  verifyConstants m
  verifyFriends m
  verifyStructDefs m
  verifyInstantiationLoops m
  verifyCodeUnits config m

/-- `dependencies::verify_module`. -/
def verifyLinking (config : VerifierConfig) (m : CompiledModule) (deps : List CompiledModule) :
    VResult Unit :=
  if config.scope == .nothing then pure () else verifyDependencies m deps

def accepts {α : Type} : VResult α → Bool
  | .ok _ => true
  | .error _ => false

/-- The model's decision procedure, whose soundness the proofs establish. -/
def verifier : VerifierInterface where
  verifyModule config m := accepts (verifyModule config m)
  verifyLinking config m deps := accepts (verifyLinking config m deps)

end MoveBytecode.Model

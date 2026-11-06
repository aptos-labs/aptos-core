-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

/-!
# The production verifier configuration (deliverable A3)

TRUSTED. The fields of `move_bytecode_verifier::VerifierConfig` and the value
that `aptos_prod_verifier_config` computes from the recorded inputs. The
recorder `config-export/` prints the same value; `Tests/Config.lean` checks
that the two agree, and CI regenerates the recorder's output.

Fields fall into three classes (plan §4.1):

- **correctness switches** change what is checked. They are fixed to their
  production values; another setting is another theorem instance;
- **acceptance limits** are only reasons to reject a module. Soundness does not
  depend on their values, except the bounds consumers rely on (L11);
- runtime bounds are not verifier configuration at all; they are `limit`
  outcomes of the semantics.

The inputs are the default feature flags, which are not necessarily mainnet's
on-chain flags. Recording mainnet's flags is open work.
-/

namespace MoveBytecode

inductive VerificationScope where
  | everything
  | nothing
  deriving DecidableEq, Repr, Inhabited

structure VerifierConfig where
  scope : VerificationScope
  -- Correctness switches.
  useSignatureCheckerV2 : Bool
  sigCheckerV2FixScriptTyParamCount : Bool
  enableEnumTypes : Bool
  enableResourceAccessControl : Bool
  enableFunctionValues : Bool
  sigCheckerV2FixFunctionSignatures : Bool
  checkFunctionTypeAbilities : Bool
  -- Acceptance limits; `none` is unbounded.
  maxLoopDepth : Option Nat
  maxFunctionParameters : Option Nat
  maxGenericInstantiationLength : Option Nat
  maxBasicBlocks : Option Nat
  maxValueStackSize : Nat
  maxTypeNodes : Option Nat
  maxPushSize : Option Nat
  maxStructDefinitions : Option Nat
  maxStructVariants : Option Nat
  maxFieldsInStruct : Option Nat
  maxFunctionDefinitions : Option Nat
  maxBackEdgesPerFunction : Option Nat
  maxBackEdgesPerModule : Option Nat
  maxBasicBlocksInScript : Option Nat
  maxPerFunMeterUnits : Option Nat
  maxPerModMeterUnits : Option Nat
  maxFunctionReturnValues : Option Nat
  maxTypeDepth : Option Nat
  deriving DecidableEq, Repr, Inhabited

/-- The inputs of `aptos_prod_verifier_config` for the recorded value. -/
structure ConfigInputs where
  gasFeatureVersion : Nat
  /-- `default`: `Features::default()`. -/
  features : String
  timedFeaturesChain : String
  timedFeaturesTimestampMicros : Nat
  deriving DecidableEq, Repr

/-- Latest gas feature version, default features, mainnet timed features at
2026-10-01T00:00:00Z. -/
def productionInputs : ConfigInputs :=
  { gasFeatureVersion := 54, features := "default", timedFeaturesChain := "mainnet",
    timedFeaturesTimestampMicros := 1790812800000000 }

def VerifierConfig.production : VerifierConfig :=
  { scope := .everything
    useSignatureCheckerV2 := true
    sigCheckerV2FixScriptTyParamCount := true
    enableEnumTypes := true
    enableResourceAccessControl := false
    enableFunctionValues := true
    sigCheckerV2FixFunctionSignatures := true
    checkFunctionTypeAbilities := true
    maxLoopDepth := some 5
    maxFunctionParameters := some 128
    maxGenericInstantiationLength := some 32
    maxBasicBlocks := some 1024
    maxValueStackSize := 1024
    maxTypeNodes := some 128
    maxPushSize := some 10000
    maxStructDefinitions := some 1100
    maxStructVariants := some 127
    maxFieldsInStruct := some 64
    maxFunctionDefinitions := some 1000
    maxBackEdgesPerFunction := none
    maxBackEdgesPerModule := none
    maxBasicBlocksInScript := some 1024
    maxPerFunMeterUnits := some 80000000
    maxPerModMeterUnits := some 80000000
    maxFunctionReturnValues := some 128
    maxTypeDepth := some 20 }

end MoveBytecode

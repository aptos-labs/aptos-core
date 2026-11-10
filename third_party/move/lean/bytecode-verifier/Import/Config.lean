-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Import.Json
import Spec.Config

/-!
# Decoding the recorded verifier configuration

Decodes the output of `config-export/` (format `move-bytecode-lean-config/1`).
-/

namespace MoveBytecode.Import

open Lean (Json)

def configFormatTag : String := "move-bytecode-lean-config/1"

def optNat (j : Json) : Dec (Option Nat) :=
  match j with
  | .null => pure none
  | .str _ => do pure (some (← natString j))
  | _ => do pure (some (← nat j))

def optNatField (j : Json) (key : String) : Dec (Option Nat) := do
  context key (optNat (← get j key))

def verifierConfig (j : Json) : Dec VerifierConfig := do
  let scope ← match ← context "scope" (str (← get j "scope")) with
    | "Everything" => pure VerificationScope.everything
    | "Nothing" => pure .nothing
    | s => throw s!"unknown verification scope {s}"
  pure {
    scope
    useSignatureCheckerV2 := ← boolField j "use_signature_checker_v2"
    sigCheckerV2FixScriptTyParamCount := ← boolField j "sig_checker_v2_fix_script_ty_param_count"
    enableEnumTypes := ← boolField j "enable_enum_types"
    enableResourceAccessControl := ← boolField j "enable_resource_access_control"
    enableFunctionValues := ← boolField j "enable_function_values"
    sigCheckerV2FixFunctionSignatures := ← boolField j "sig_checker_v2_fix_function_signatures"
    checkFunctionTypeAbilities := ← boolField j "check_function_type_abilities"
    maxLoopDepth := ← optNatField j "max_loop_depth"
    maxFunctionParameters := ← optNatField j "max_function_parameters"
    maxGenericInstantiationLength := ← optNatField j "max_generic_instantiation_length"
    maxBasicBlocks := ← optNatField j "max_basic_blocks"
    maxValueStackSize := ← natField j "max_value_stack_size"
    maxTypeNodes := ← optNatField j "max_type_nodes"
    maxPushSize := ← optNatField j "max_push_size"
    maxStructDefinitions := ← optNatField j "max_struct_definitions"
    maxStructVariants := ← optNatField j "max_struct_variants"
    maxFieldsInStruct := ← optNatField j "max_fields_in_struct"
    maxFunctionDefinitions := ← optNatField j "max_function_definitions"
    maxBackEdgesPerFunction := ← optNatField j "max_back_edges_per_function"
    maxBackEdgesPerModule := ← optNatField j "max_back_edges_per_module"
    maxBasicBlocksInScript := ← optNatField j "max_basic_blocks_in_script"
    maxPerFunMeterUnits := ← optNatField j "max_per_fun_meter_units"
    maxPerModMeterUnits := ← optNatField j "max_per_mod_meter_units"
    maxFunctionReturnValues := ← optNatField j "max_function_return_values"
    maxTypeDepth := ← optNatField j "max_type_depth" }

def configInputs (j : Json) : Dec ConfigInputs := do
  pure {
    gasFeatureVersion := ← natField j "gas_feature_version"
    features := ← context "features" (str (← get j "features"))
    timedFeaturesChain := ← context "timed_features_chain" (str (← get j "timed_features_chain"))
    timedFeaturesTimestampMicros := ← context "timed_features_timestamp_micros"
      (natString (← get j "timed_features_timestamp_micros")) }

/-- Decode a recorded configuration: its inputs and its value. -/
def recordedConfig (text : String) : Dec (ConfigInputs × VerifierConfig) := do
  let j ← Json.parse text
  let f ← context "format" (str (← get j "format"))
  if f != configFormatTag then throw s!"unsupported format {f}, expected {configFormatTag}"
  pure (← context "inputs" (configInputs (← get j "inputs")),
        ← context "verifier_config" (verifierConfig (← get j "verifier_config")))

end MoveBytecode.Import

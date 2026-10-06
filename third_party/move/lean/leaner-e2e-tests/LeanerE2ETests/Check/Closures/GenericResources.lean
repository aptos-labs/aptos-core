-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-! Open resource templates remain available to generic frames, while only
closed instances name runtime storage keys. Invocations still require the
resource typing certificate, including for function-valued resource fields. -/

leaner module 0x99::generic_resources where
  struct Flag {T : phantom type} has Key where
    value : Bool

  struct Handler {T : phantom type} has Key where
    f : Fn(u64) -> u64 has Copy, Drop, Store
    g : Fn(u64) -> u64 has Copy, Drop, Store
  spec Handler where
    invariant ∀ (x : u64; r : u64), ensures_of<f>(x, r) ==> x <= r
    invariant ∀ (x : u64; r : u64), ensures_of<g>(x, r) ==> r <= x

  fun apply(f : Fn(u64) -> u64, x : u64) -> u64 := invoke(f, x)
  spec apply where
    ensures ensures_of<f>(x, result)

  fun apply_field(handler : &Handler<Bool>, x : u64) -> u64 := invoke(handler.f, x)
  spec apply_field where
    ensures ensures_of<handler.f>(x, result)

  fun lower_bound(handler : &Handler<Bool>) -> u64 := invoke(handler.f, 5)
  spec lower_bound where
    ensures result >= 5

  fun upper_bound(handler : &Handler<Bool>) -> u64 := invoke(handler.g, 5)
  spec upper_bound where
    ensures result <= 5

  fun read(flag : &Flag<Bool>) -> Bool := flag.value
  spec read where
    ensures result == flag.value

  fun read_generic {T}(flag : &Flag<T>) -> Bool := flag.value
  spec read_generic where
    ensures result == flag.value

open Lean LeanerIR LeanerIR.Proofs LeanerIR.Proofs.Denote in
run_cmd do
  let some unit := LeanerLang.registeredUnit? (← getEnv) `«0x99».generic_resources
    | throwError "missing generic resource fixture"
  unless resourcesTypedCheck unit do throwError "runtime resource typing failed"
  let mut openTemplates := 0
  let mut closedKeys := 0
  for n in [:unit.namespaces.size] do
    for t in [:unit.namespaces[n]!.tables.types.size] do
      if let some resource := resourceOf unit ⟨n⟩ ⟨t⟩ then
        let runtime := runtimeResourceOf unit ⟨n⟩ ⟨t⟩
        if resource.type.paramFree then
          unless runtime == some resource do throwError "closed resource key was lost"
          closedKeys := closedKeys + 1
        else
          unless runtime.isNone do throwError "open template became a runtime key"
          openTemplates := openTemplates + 1
  unless openTemplates > 0 && closedKeys > 0 do
    throwError "fixture must cover both open templates and closed instances"

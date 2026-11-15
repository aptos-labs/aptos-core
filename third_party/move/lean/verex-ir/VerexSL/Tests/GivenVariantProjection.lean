-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-! Generic enum construction and payload observations compose at the same
abstract parameter representation, including across verified function calls. -/

set_option verex.sharedContinuations true

verex module 0x42::given_variant_projection where
  enum Choice {T has Copy, Drop, Store} has Copy, Drop, Store where
    | Left (value : T)
    | Right (value : T)

  fun choose {T has Copy, Drop, Store}(right : Bool, value : T) -> Choice<T> :=
    if right then new Choice<T>::Right { value } else new Choice<T>::Left { value }
  spec choose where
    ensures result == if right then new Choice<T>::Right { value } else new Choice<T>::Left { value }

  fun payload {T has Copy, Drop, Store}(choice : Choice<T>) -> T :=
    match choice with
      | Choice<T>::Left { value := value } => value
      | Choice<T>::Right { value := value } => value
  spec payload where
    ensures result == match choice with
      | Choice<T>::Left { value := value } => value
      | Choice<T>::Right { value := value } => value

  fun choose_payload {T has Copy, Drop, Store}(right : Bool, value : T) -> T :=
    payload(choose(right, value))
  spec choose_payload where
    ensures result == value

open Lean Elab Command VerexSL.Verify in
run_cmd do
  let env ← getEnv
  for member in ["choose", "payload", "choose_payload"] do
    let base := Name.str `«0x42».given_variant_projection member
    unless (givenProofArtifacts.getState env).find? base == some (base ++ `givenTypedVerified) do
      throwError "generic enum composition did not use the Given proof path: {member}"

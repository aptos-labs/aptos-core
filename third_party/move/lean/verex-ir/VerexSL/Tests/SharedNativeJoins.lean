-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-! Opaque generic constructors at a join retain their native encoding
equations. A payload observation must use both the constructor's contract
and the native call's result without searching impossible variants. -/

set_option verex.sharedContinuations true

/--
error: the specification clause `ensures bytes.length == 32 ==> result is None` is not established
---
error: verex verification failed: the automatic verification of `wrong` failed; provide a proof in the module:
  verify wrong by lean =>
    verex_obligations
    verex_on
    | ensures_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
verex module 0x42::shared_native_joins where
  enum Optional {T} has Copy, Drop where
    | None
    | Some (value : T)

  fun none {T}() -> Optional<T> := new Optional<T>::None {}
  spec none where
    pragma opaque
    aborts_if false
    ensures result == new Optional<T>::None {}

  fun some {T}(value : T) -> Optional<T> := new Optional<T>::Some { value }
  spec some where
    pragma opaque
    aborts_if false
    ensures result == new Optional<T>::Some { value }

  struct Payload has Copy, Drop where
    data : Vector<u8>

  spec fun contents(option : Optional<Payload>) : Vector<u8> :=
    match option with
      | Optional<Payload>::Some { value := value } => value.data
      | _ => #[]

  opaque spec fun transformed(bytes : Vector<u8>) : Vector<u8>
  native fun transform(bytes : Vector<u8>) -> Vector<u8>
  spec transform where
    pragma opaque
    aborts_if false
    ensures result == transformed(bytes)

  fun select(bytes : Vector<u8>) -> Optional<Payload> :=
    if bytes.length == 32 then some(new Payload { data := transform(bytes) })
    else none::<Payload>()
  spec select where
    aborts_if false
    ensures bytes.length != 32 ==> result is None
    ensures bytes.length == 32 ==> contents(result) == transformed(bytes)

  -- The constructor's payload does not permit discarding its actual tag.
  fun wrong(bytes : Vector<u8>) -> Optional<Payload> :=
    if bytes.length == 32 then some(new Payload { data := transform(bytes) })
    else none::<Payload>()
  spec wrong where
    aborts_if false
    ensures bytes.length == 32 ==> result is None

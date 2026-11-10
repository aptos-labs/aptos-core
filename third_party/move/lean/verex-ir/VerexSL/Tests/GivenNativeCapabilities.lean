-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-! Native models requiring concrete reflection must select that interpretation
before proof generation. Other native models keep their actual Given proofs. -/

verex module 0x1::string where
  struct String has Copy, Drop, Store where
    bytes : Vector<u8>

verex module 0x1::type_info where
  use 0x1::string::String

  public native fun type_name {T}() -> String

verex module 0x1::aggregator_v2 where
  struct Aggregator {IntElement} has Store, Drop where
    value : IntElement
    max_value : IntElement

  public native fun create_unbounded_aggregator {IntElement}() -> Aggregator<IntElement>

verex module 0x1::hash where
  public native fun sha2_256(bytes : Vector<u8>) -> Vector<u8>

verex module 0x1::cmp where
  enum Ordering has Copy, Drop where
    | Less
    | Equal
    | Greater

  public native fun compare {T}(first : &T, second : &T) -> Ordering

verex module 0x42::given_native_capabilities where
  pragma verify = false
  use 0x1::aggregator_v2::Aggregator
  use 0x1::aggregator_v2::create_unbounded_aggregator
  use 0x1::type_info::type_name
  use 0x1::hash::sha2_256
  use 0x1::cmp::Ordering
  use 0x1::cmp::compare

  fun generic {T}() -> Aggregator<T> := create_unbounded_aggregator::<T>()
  spec generic where
    aborts_if type_name::<T>().bytes != b"u64" && type_name::<T>().bytes != b"u128"

  fun digest {T}(unused : &T, bytes : Vector<u8>) -> Vector<u8> := sha2_256(bytes)
  spec digest where
    aborts_if false
    ensures result.length == 32

  fun comparison {T}(first : &T, second : &T) -> Ordering := compare(first, second)
  spec comparison where
    aborts_if false
    ensures result == compare(first, second)

verify 0x42::given_native_capabilities::generic
verify 0x42::given_native_capabilities::digest
verify 0x42::given_native_capabilities::comparison

open Lean Elab Command VerexSL VerexSL.Verify in
run_cmd do
  let env ← getEnv
  let concrete := `«0x42».given_native_capabilities.generic
  unless env.contains (concrete ++ `typedVerified) do
    throwError "the aggregator model lost its concrete proof path"
  if env.contains (concrete ++ `givenTypedVerified) ||
      env.contains (concrete ++ `givenTransfer) ||
      ((givenProofArtifacts.getState env).find? concrete).isSome then
    throwError "the aggregator model was admitted to an unsupported Given interpretation"
  for supported in [`«0x42».given_native_capabilities.digest,
      `«0x42».given_native_capabilities.comparison] do
    unless (givenProofArtifacts.getState env).find? supported ==
        some (supported ++ `givenTypedVerified) do
      throwError "a supported native model lost its Given proof path"

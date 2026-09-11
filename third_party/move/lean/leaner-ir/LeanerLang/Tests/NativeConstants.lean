-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

set_option Elab.async false
set_option leaner.route "native"
set_option maxHeartbeats 50000
set_option leaner.verifyHeartbeats 50000

#leaner_measure

leaner module 0x42::native_constants where
  const COMPLEX : u64 := 1 + 2 * 3
  const NESTED : u64 := COMPLEX + 1
  fun complex() -> u64 := NESTED
  spec complex where
    ensures result == 8
    aborts_if false
  verify complex

  fun signed_minimum() -> i8 := -128i8
  spec signed_minimum where
    ensures result == -128
    aborts_if false
  verify signed_minimum

  fun wide() -> u256 := 340282366920938463463374607431768211456u256
  spec wide where
    ensures result == 340282366920938463463374607431768211456
    aborts_if false
  verify wide

  fun flag() -> Bool := true
  spec flag where
    ensures result
    aborts_if false
  verify flag

  fun address() -> Address := @0xCAFE
  spec address where
    ensures result == @0xCAFE
    aborts_if false
  verify address

  fun text() -> string := "hello"
  spec text where
    ensures result == "hello"
    aborts_if false
  verify text

  fun bytes() -> Bytes := b[0, 127, 255]
  spec bytes where
    ensures result == b[0, 127, 255]
    aborts_if false
  verify bytes

  fun choose_address(flag : Bool) -> Address := if flag then @0xCAFE else @0x42
  spec choose_address where
    ensures result == if flag then @0xCAFE else @0x42
    aborts_if false
  verify choose_address

  fun wrong_integer() -> u64 := NESTED
  spec wrong_integer where
    ensures result == 7
    aborts_if false

  fun wrong_address() -> Address := @0xCAFE
  spec wrong_address where
    ensures result == @0x42
    aborts_if false

#guard_msgs (drop error) in
#leaner_verify 0x42::native_constants::wrong_integer
#guard_msgs (drop error) in
#leaner_verify 0x42::native_constants::wrong_address

#leaner_require_native 0x42::native_constants::complex
#leaner_require_native 0x42::native_constants::signed_minimum
#leaner_require_native 0x42::native_constants::wide
#leaner_require_native 0x42::native_constants::flag
#leaner_require_native 0x42::native_constants::address
#leaner_require_native 0x42::native_constants::text
#leaner_require_native 0x42::native_constants::bytes
#leaner_require_native 0x42::native_constants::choose_address

open Lean Elab Command in
run_cmd do
  LeanerLang.Perf.measuring.set false
  let samples ← LeanerLang.Perf.recorded.get
  for function in ["complex", "signed_minimum", "wide", "flag", "address", "text", "bytes",
      "choose_address"] do
    let measured := samples.filter (·.target.startsWith s!"«0x42».native_constants::{function} ")
    unless measured.size == 2 do throwError "missing constant stages for {function}"
    let total : Nat := measured.foldl (fun cost sample => cost + sample.heartbeats) 0
    logInfo m!"{function}: {total} heartbeats (all generated stages)"
    unless total ≤ 50000000 do throwError "native constant {function} exceeds aggregate 50M budget"
  for function in [`wrong_integer, `wrong_address] do
    for suffix in [`computation, `nativeSummary, `computationVerified, `computationRepresents] do
      if (← getEnv).contains (`«0x42».native_constants ++ function ++ suffix) then
        throwError "rejected constant leaked {function}.{suffix}"

set_option maxHeartbeats 1000

open LeanerIR LeanerIR.Proofs «0x42».native_constants in
example (state : RuntimeState) : (complex.computation ⟨⟩).ok state ⟨8, by decide⟩ state := by
  exact ⟨rfl, rfl⟩

open LeanerIR LeanerIR.Proofs «0x42».native_constants in
example (state : RuntimeState) : (signed_minimum.computation ⟨⟩).ok state ⟨-128, by decide⟩ state := by
  exact ⟨rfl, rfl⟩

open LeanerIR LeanerIR.Proofs «0x42».native_constants in
example (state : RuntimeState) : (address.computation ⟨⟩).ok state "0xCAFE" state := by
  exact ⟨rfl, rfl⟩

open LeanerIR LeanerIR.Proofs «0x42».native_constants in
example (state : RuntimeState) : (text.computation ⟨⟩).ok state "hello" state := by
  exact ⟨rfl, rfl⟩

open LeanerIR LeanerIR.Proofs «0x42».native_constants in
example (state : RuntimeState) : (bytes.computation ⟨⟩).ok state #[0, 127, 255] state := by
  exact ⟨rfl, rfl⟩

open LeanerIR LeanerIR.Proofs «0x42».native_constants in
example (state : RuntimeState) : (choose_address.computation ⟨false⟩).ok state "0x42" state := by
  exact ⟨rfl, rfl⟩

#leaner_require_native_all

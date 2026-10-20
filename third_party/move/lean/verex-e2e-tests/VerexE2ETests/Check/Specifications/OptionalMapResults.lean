-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexE2ETests.CheckSupport

/-! Optional map results use the declaration's representation, including
enum tags and payloads. Check execution as well as the generated contracts:
an impossible result encoding must not make the caller's proof vacuous. -/
namespace VerexSL.Tests.Check.Specifications.OptionalMapResults
open VerexIR.Proofs.Denote

verex module 0x42::optional_map where
  enum Maybe {V} has Copy, Drop, Store where
    | Absent
    | Present (value : V)

  @[intrinsic_map]
  struct Map {K} {V} has Copy, Drop, Store where
    placeholder : Bool

  @[map_new (Map)]
  native fun new {K} {V}() -> Map<K, V>
  @[map_get (Map)]
  native fun get {K} {V}(m : &Map<K, V>, k : &K) -> Maybe<V>
  @[map_upsert (Map)]
  native fun upsert {K} {V}(m : &mut Map<K, V>, k : K, v : V) -> Maybe<V>

  public fun missing() -> Maybe<u64> := do
    let m := new::<u64, u64>()
    get(&m, &7u64)
  spec missing where
    aborts_if false
    ensures result is Absent
  verify missing

  public fun replaced() -> Maybe<u64> := do
    let mut m := new::<u64, u64>()
    let _ := upsert(&mut m, 7u64, 11u64)
    upsert(&mut m, 7u64, 13u64)
  spec replaced where
    aborts_if false
    ensures result is Present && result.value == 11
  verify replaced

  public fun present() -> Maybe<u64> := do
    let mut m := new::<u64, u64>()
    let _ := upsert(&mut m, 7u64, 11u64)
    get(&m, &7u64)
  spec present where
    aborts_if false
    ensures result is Present && result.value == 11
  verify present

open VerexE2ETests.CheckSupport in
run_cmd assertRuns `«0x42».optional_map #[
  { function := "missing", arguments := #[],
    expected := .returned #[.nominal ⟨⟨0⟩, 0⟩ (some "Absent") #[]] },
  { function := "replaced", arguments := #[],
    expected := .returned #[.nominal ⟨⟨0⟩, 0⟩ (some "Present") #[.integer 11]] },
  { function := "present", arguments := #[],
    expected := .returned #[.nominal ⟨⟨0⟩, 0⟩ (some "Present") #[.integer 11]] }]

-- Existing vector-backed optional result declarations remain supported.
verex module 0x42::optional_vector_map where
  struct Maybe {V} has Copy, Drop, Store where
    values : Vector<V>

  @[intrinsic_map]
  struct Map {K} {V} has Copy, Drop, Store where
    placeholder : Bool

  @[map_new (Map)]
  native fun new {K} {V}() -> Map<K, V>
  @[map_get (Map)]
  native fun get {K} {V}(m : &Map<K, V>, k : &K) -> Maybe<V>
  @[map_upsert (Map)]
  native fun upsert {K} {V}(m : &mut Map<K, V>, k : K, v : V) -> Maybe<V>

  public fun missing() -> Maybe<u64> := do
    let m := new::<u64, u64>()
    get(&m, &7u64)
  spec missing where
    aborts_if false
    ensures len(result.values) == 0
  verify missing

  public fun replaced() -> Maybe<u64> := do
    let mut m := new::<u64, u64>()
    let _ := upsert(&mut m, 7u64, 11u64)
    upsert(&mut m, 7u64, 13u64)
  spec replaced where
    aborts_if false
    ensures len(result.values) == 1 && result.values[0] == 11
  verify replaced by
    verex_obligations
    all_goals verex_prepare
    all_goals try simp_all only [vir_denote_norm]
    all_goals
      have encoded : [RuntimeValue.integer 11] =
          List.map (VerexIR.Proofs.Codec.specInt (.bits 64) false).encode
            upsert_result.values.toList := by assumption
      have length := congrArg List.length encoded
      have read := congrArg (fun xs : List RuntimeValue => RuntimeValue.asInt ((xs[0]?).getD RuntimeValue.unit)) encoded
      simp only [List.length_cons, List.length_nil, List.length_map, Array.length_toList] at length
      simp only [List.getElem?_cons_zero, Option.getD_some, VerexIR.Maps.asInt_integer,
        List.getElem?_map, Array.getElem?_toList, asInt_getD_map_specInt 64 false (by decide)] at read
      exact ⟨by omega, read.symm⟩

open VerexE2ETests.CheckSupport in
run_cmd assertRuns `«0x42».optional_vector_map #[
  { function := "missing", arguments := #[],
    expected := .returned #[.nominal ⟨⟨0⟩, 0⟩ none #[.vector #[]]] },
  { function := "replaced", arguments := #[],
    expected := .returned #[.nominal ⟨⟨0⟩, 0⟩ none #[.vector #[.integer 11]]] }]

end VerexSL.Tests.Check.Specifications.OptionalMapResults

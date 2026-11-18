-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexSL.Tests.Rendered.Support

/-!
# `map_write_block` as a frontend renders it

The module below is the rendering of
`VerexE2ETests/SourceVerify/map_write_block.move` that the source-verification
driver writes to `.verex/map_write_block.move.lean`, with the items of
`VerexE2ETests/SourceVerify/map_write_block.proof.lean` as items of the module
and the `pragma proof_file` that names that file dropped: the driver defers
this fixture's verification to the proof file, which a lone elaboration does
not read, so the snapshot carries the proofs themselves and proves what the
fixture owes. Its `verex sources at` command is dropped and the diagnostics
are those of `map_write_block.exp`, which holds the proof file's messages as
well. Nothing here is edited by hand: `scripts/prover-smoke-renderings.sh
--write` writes this file again, and without `--write` tells whether it is
what the frontend renders and what the proof file says.
-/

/--
error: `decide` does not close the obligation from the block's facts:
  case assert_1
  executable✝ : Validation.ExecutableUnit unit
  assumed permitted : True
  m_final_0 value_0_0 : Proofs.Denote.NTy.address.carrier
  m_final : ((Proofs.Denote.NTy.int 64 false).table (Proofs.Denote.NTy.int 64 false)).carrier
  final : Proofs.Denote.Memory unit
  m_final_scope : Proofs.Denote.FinalEntry "m" (m_final_0, m_final, ())
  verexInvariant✝¹ : SpecMap.Valid SpecMap.Discipline.sequence (SpecMap.ofEntries SpecInt.val SpecInt.val m_final)
  bounds : m_final.entries.length < 2 ^ 64
  left✝¹ : 0 ≤ (Proofs.Denote.NTy.int 64 false).plainFallback ⋯
  right✝¹ : (Proofs.Denote.NTy.int 64 false).plainFallback ⋯ ≤ 2 ^ 64 - 1
  value_0 : SpecMap Int Int
  a✝¹ :
    (0x1.map_write_block.unit.nominal.namespace.0.declaration✝⁻⁰ (Proofs.Denote.NTy.int 64 false)
          (Proofs.Denote.NTy.int 64 false)).Holds
      (value_0_0,
        SpecMap.writeBack
          (fun entry =>
            ((Proofs.Denote.NTy.int 64 false).unread ⋯ entry.fst, (Proofs.Denote.NTy.int 64 false).unread ⋯ entry.snd))
          value_0,
        PUnit.unit)
  value_0_read :
    SpecMap.ofEntries SpecInt.val SpecInt.val
        (SpecMap.writeBack
          (fun entry => ({ val := 0, fits := ⋯ }.ofInt entry.fst, { val := 0, fits := ⋯ }.ofInt entry.snd)) value_0) ==
      value_0
  verexInvariant✝ : SpecMap.Valid SpecMap.Discipline.sequence value_0
  value_0_keys : SpecMap.KeysSatisfy (IntegerValueFits (IntWidth.bits 64) false) value_0
  value_0_values : SpecMap.ValuesSatisfy (IntegerValueFits (IntWidth.bits 64) false) value_0
  m : SpecMap Int Int
  a✝ :
    (0x1.map_write_block.unit.nominal.namespace.0.declaration✝⁻⁰ (Proofs.Denote.NTy.int 64 false)
          (Proofs.Denote.NTy.int 64 false)).Holds
      (value_0_0,
        SpecMap.writeBack
          (fun entry =>
            ((Proofs.Denote.NTy.int 64 false).unread ⋯ entry.fst, (Proofs.Denote.NTy.int 64 false).unread ⋯ entry.snd))
          m,
        PUnit.unit)
  invariantHolds✝ :
    (0x1.map_write_block.unit.nominal.namespace.0.declaration✝⁻⁰ (Proofs.Denote.NTy.int 64 false)
          (Proofs.Denote.NTy.int 64 false)).Holds
      (value_0_0,
        SpecMap.writeBack
          (fun entry =>
            ((Proofs.Denote.NTy.int 64 false).unread ⋯ entry.fst, (Proofs.Denote.NTy.int 64 false).unread ⋯ entry.snd))
          m,
        ())
  m_read :
    SpecMap.ofEntries SpecInt.val SpecInt.val
        (SpecMap.writeBack
          (fun entry => ({ val := 0, fits := ⋯ }.ofInt entry.fst, { val := 0, fits := ⋯ }.ofInt entry.snd)) m) ==
      m
  verexInvariant : SpecMap.Valid SpecMap.Discipline.sequence m
  value_0_eq : value_0 == spec_set(m, 1, 30)
  m_keys : SpecMap.KeysSatisfy (IntegerValueFits (IntWidth.bits 64) false) m
  m_values : SpecMap.ValuesSatisfy (IntegerValueFits (IntWidth.bits 64) false) m
  value_0_size_bounds_1 : 0 ≤ spec_len(value_0)
  value_0_size_bounds_2 : spec_len(value_0) < 18446744073709551616
  m_size_bounds_1 : 0 ≤ spec_len(m)
  m_size_bounds_2 : spec_len(m) < 18446744073709551616
  left✝ : 0 ≤ spec_get(value_0, 1)
  right✝ : spec_get(value_0, 1) ≤ 18446744073709551615
  ⊢ spec_get(value_0, 1) == 30
---
error: verex verification failed: the proof of `write_read` does not establish its specification
-/
#guard_rendering in
verex module 0x1::map_write_block where

  -- Copyright © Aptos Foundation
  -- SPDX-License-Identifier: Apache-2.0
  -- A read of a map after a write, asserted under a block that takes one clause
  -- (`on assert 1 { }`): `decide` does not close it, and `write_read` is a parked
  -- gap. `write_read_rest` is the control: the same body under `on _ { }` verifies.
  @[intrinsic_map]
  struct Map {K : phantom type has Copy, Drop} {V : phantom type} has Store where
    handle : Address

  spec Map where
    pragma intrinsic = map

  @[map_add_override_if_exists (Map)]
  native fun add {K has Copy, Drop} {V}(
    self : &mut Map<K, V>, k : K, v : V
  ) -> Unit

  spec add where
    pragma intrinsic

  @[map_spec_len (Map)]
  opaque spec fun spec_len {K} {V}(t : Map<K, V>) : Int

  @[map_spec_has_key (Map)]
  opaque spec fun spec_contains {K} {V}(t : Map<K, V>, k : K) : Bool

  @[map_spec_get (Map)]
  opaque spec fun spec_get {K} {V}(t : Map<K, V>, k : K) : V

  @[map_spec_set (Map)]
  opaque spec fun spec_set {K} {V}(t : Map<K, V>, k : K, v : V) : Map<K, V>

  @[map_spec_del (Map)]
  opaque spec fun spec_remove {K} {V}(t : Map<K, V>, k : K) : Map<K, V>

  fun write_read(m : &mut Map<u64, u64>) -> Unit := do
    m.add(1, 30)
    spec do
      assert spec_get::<u64, u64>(m, 1) == 30
      assert spec_contains::<u64, u64>(m, 1)

  fun write_read_rest(m : &mut Map<u64, u64>) -> Unit := do
    m.add(1, 30)
    spec do
      assert spec_get::<u64, u64>(m, 1) == 30
      assert spec_contains::<u64, u64>(m, 1)

  verify write_read by
    on assert 1 { }

  verify write_read_rest by
    on _ { }

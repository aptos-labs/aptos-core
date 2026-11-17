-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexSL.Tests.Rendered.Support

/-!
# `map_size_update` as a frontend renders it

The module below is the rendering of
`VerexE2ETests/SourceVerify/map_size_update.move` that the source-verification
driver writes to `.verex/map_size_update.move.lean`, with the items of
`VerexE2ETests/SourceVerify/map_size_update.proof.lean` as items of the module
and the `pragma proof_file` that names that file dropped: the driver defers
this fixture's verification to the proof file, which a lone elaboration does
not read, so the snapshot carries the proofs themselves and proves what the
fixture owes. Its `verex sources at` command is dropped and the diagnostics
are those of `map_size_update.exp`, which holds the proof file's messages as
well. Nothing here is edited by hand: `scripts/prover-smoke-renderings.sh
--write` writes this file again, and without `--write` tells whether it is
what the frontend renders and what the proof file says.
-/

/-!
The size of a map after an update of a key it holds and of one it does
not: the two facts the library states beside their parent
`SpecMap.size_update_sequence`, which the proof file applies by name.
The owner binds the executable role their fact is of, `add`.
-/

/--
error: `decide` does not close the obligation from the block's facts:
  case assert_1
  executable✝ : Validation.ExecutableUnit unit
  k : (Proofs.Denote.NTy.int 8 false).carrier
  v : (Proofs.Denote.NTy.int 64 false).carrier
  assumed permitted : True
  m_final_0 value_0_0 : Proofs.Denote.NTy.address.carrier
  m_final : ((Proofs.Denote.NTy.int 8 false).table (Proofs.Denote.NTy.int 64 false)).carrier
  final : Proofs.Denote.Memory unit
  m_final_scope : Proofs.Denote.FinalEntry "m" (m_final_0, m_final, ())
  verexInvariant✝¹ : SpecMap.Valid SpecMap.Discipline.sequence (SpecMap.ofEntries SpecInt.val SpecInt.val m_final)
  bounds : m_final.entries.length < 2 ^ 64
  k_lower : 0 ≤ k
  k_upper : k ≤ 2 ^ 8 - 1
  v_lower : 0 ≤ v
  v_upper : v ≤ 2 ^ 64 - 1
  value_0 : SpecMap Int Int
  a✝¹ :
    (0x1.map_size_update.unit.nominal.namespace.0.declaration✝⁻⁰ (Proofs.Denote.NTy.int 8 false)
          (Proofs.Denote.NTy.int 64 false)).Holds
      (value_0_0,
        SpecMap.writeBack
          (fun entry =>
            ((Proofs.Denote.NTy.int 8 false).unread ⋯ entry.fst, (Proofs.Denote.NTy.int 64 false).unread ⋯ entry.snd))
          value_0,
        PUnit.unit)
  value_0_read :
    SpecMap.ofEntries SpecInt.val SpecInt.val
        (SpecMap.writeBack
          (fun entry => ({ val := 0, fits := ⋯ }.ofInt entry.fst, { val := 0, fits := ⋯ }.ofInt entry.snd)) value_0) ==
      value_0
  verexInvariant✝ : SpecMap.Valid SpecMap.Discipline.sequence value_0
  value_0_keys : SpecMap.KeysSatisfy (IntegerValueFits (IntWidth.bits 8) false) value_0
  value_0_values : SpecMap.ValuesSatisfy (IntegerValueFits (IntWidth.bits 64) false) value_0
  m : SpecMap Int Int
  a✝ :
    (0x1.map_size_update.unit.nominal.namespace.0.declaration✝⁻⁰ (Proofs.Denote.NTy.int 8 false)
          (Proofs.Denote.NTy.int 64 false)).Holds
      (value_0_0,
        SpecMap.writeBack
          (fun entry =>
            ((Proofs.Denote.NTy.int 8 false).unread ⋯ entry.fst, (Proofs.Denote.NTy.int 64 false).unread ⋯ entry.snd))
          m,
        PUnit.unit)
  invariantHolds✝ :
    (0x1.map_size_update.unit.nominal.namespace.0.declaration✝⁻⁰ (Proofs.Denote.NTy.int 8 false)
          (Proofs.Denote.NTy.int 64 false)).Holds
      (value_0_0,
        SpecMap.writeBack
          (fun entry =>
            ((Proofs.Denote.NTy.int 8 false).unread ⋯ entry.fst, (Proofs.Denote.NTy.int 64 false).unread ⋯ entry.snd))
          m,
        ())
  this✝ :
    Proofs.Denote.FunctionStart { namespaceId := { index := 0 }, functionId := { index := 3 } }
      (((value_0_0,
            SpecMap.writeBack
              (fun entry =>
                ((Proofs.Denote.NTy.int 8 false).unread ⋯ entry.fst,
                  (Proofs.Denote.NTy.int 64 false).unread ⋯ entry.snd))
              m,
            ()),
          m_final_0, m_final, ()),
        k, v, ())
      final
  m_read :
    SpecMap.ofEntries SpecInt.val SpecInt.val
        (SpecMap.writeBack
          (fun entry => ({ val := 0, fits := ⋯ }.ofInt entry.fst, { val := 0, fits := ⋯ }.ofInt entry.snd)) m) ==
      m
  verexInvariant : SpecMap.Valid SpecMap.Discipline.sequence m
  value_0_eq : value_0 == spec_set(m, k, v)
  m_keys : SpecMap.KeysSatisfy (IntegerValueFits (IntWidth.bits 8) false) m
  m_values : SpecMap.ValuesSatisfy (IntegerValueFits (IntWidth.bits 64) false) m
  value_0_size_bounds_1 : 0 ≤ spec_len(value_0)
  value_0_size_bounds_2 : spec_len(value_0) < 18446744073709551616
  m_size_bounds_1 : 0 ≤ spec_len(m)
  m_size_bounds_2 : spec_len(m) < 18446744073709551616
  ⊢ spec_len(value_0) == spec_len(m) + 1
---
error: verex verification failed: the proof of `size_after_unknown` does not establish its specification
-/
#guard_rendering in
verex module 0x1::map_size_update where

  -- Copyright © Aptos Foundation
  -- SPDX-License-Identifier: Apache-2.0
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

  /--
  An entry of a key the map does not hold makes it one longer.
  -/
  fun size_after_insert(m : &mut Map<u8, u64>, k : u8, v : u64) -> Unit := do
    m.add(k, v)
    spec assert spec_len::<u8, u64>(m) == spec_len::<u8, u64>(old(m)) + 1

  spec size_after_insert where
    requires !spec_contains::<u8, u64>(m, k)

  /--
  An entry of a key it holds leaves the size alone.
  -/
  fun size_after_override(m : &mut Map<u8, u64>, k : u8, v : u64) -> Unit := do
    m.add(k, v)
    spec assert spec_len::<u8, u64>(m) == spec_len::<u8, u64>(old(m))

  spec size_after_override where
    requires spec_contains::<u8, u64>(m, k)

  /--
  Negative: of a key that may be held, neither size is stated.
  -/
  fun size_after_unknown(m : &mut Map<u8, u64>, k : u8, v : u64) -> Unit := do
    m.add(k, v)
    spec assert spec_len::<u8, u64>(m) == spec_len::<u8, u64>(old(m)) + 1

  -- error: the key may be held

  -- Every cell has a block, the negative too: a cell left to the automatic
  -- route names this file in its failure, which the snapshot of this module
  -- (the same items among the module's own) words differently.

  verify size_after_insert by
    on assert 1 {
      lean =>
        rw [value_0_eq, VerexIR.SpecMap.size_update_sequence_absent (by assumption)]
    }
    on _ { }

  verify size_after_override by
    on assert 1 {
      lean =>
        rw [value_0_eq, VerexIR.SpecMap.size_update_sequence_present (by assumption)]
    }
    on _ { }

  verify size_after_unknown by
    on _ { }

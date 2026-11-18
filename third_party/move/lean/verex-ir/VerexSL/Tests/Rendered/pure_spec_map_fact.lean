-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexSL.Tests.Rendered.Support

/-!
# `pure_spec_map_fact` as a frontend renders it

The module below is the rendering of
`VerexE2ETests/SourceVerify/pure_spec_map_fact.move` that the
source-verification driver writes to `.verex/pure_spec_map_fact.move.lean`,
with the items of `VerexE2ETests/SourceVerify/pure_spec_map_fact.proof.lean`
as items of the module and the `pragma proof_file` that names that file
dropped: the driver defers this fixture's verification to the proof file,
which a lone elaboration does not read, so the snapshot carries the proofs
themselves and proves what the fixture owes. Its `verex sources at` command is
dropped and the diagnostics are those of `pure_spec_map_fact.exp`, which holds
the proof file's messages as well. Nothing here is edited by hand:
`scripts/prover-smoke-renderings.sh --write` writes this file again, and
without `--write` tells whether it is what the frontend renders and what the
proof file says.
-/

/-!
At a map owner with no executable operations, the length-update lemma and
a precondition equating a parameter with an updated map do not close the
authored assertions of `v_len_set` and `v_set_param`, which are parked
gaps; `v_rank_nonneg` is the control. The cause is not localized.
-/

/--
error: `decide` does not close the obligation from the block's facts:
  case assert_1
  k v : (Proofs.Denote.NTy.int 64 false).carrier
  m_handle : Proofs.Denote.NTy.address.carrier
  initialState✝ : Proofs.Denote.Memory unit
  assumed permitted : True
  k_lower : 0 ≤ k
  k_upper : k ≤ 2 ^ 64 - 1
  v_lower : 0 ≤ v
  v_upper : v ≤ 2 ^ 64 - 1
  m : SpecMap Int Int
  m_valid :
    (0x1.pure_spec_map_fact.unit.nominal.namespace.0.declaration✝⁻⁰ (Proofs.Denote.NTy.int 64 false)
          (Proofs.Denote.NTy.int 64 false)).Holds
      (m_handle,
        SpecMap.writeBack
          (fun entry =>
            ((Proofs.Denote.NTy.int 64 false).unread ⋯ entry.fst, (Proofs.Denote.NTy.int 64 false).unread ⋯ entry.snd))
          m,
        PUnit.unit)
  m_read :
    SpecMap.ofEntries SpecInt.val SpecInt.val
        (SpecMap.writeBack
          (fun entry => ({ val := 0, fits := ⋯ }.ofInt entry.fst, { val := 0, fits := ⋯ }.ofInt entry.snd)) m) ==
      m
  verexInvariant : SpecMap.Valid (SpecMap.Discipline.ordered SpecMap.intOrder) m
  m_keys : SpecMap.KeysSatisfy (IntegerValueFits (IntWidth.bits 64) false) m
  m_values : SpecMap.ValuesSatisfy (IntegerValueFits (IntWidth.bits 64) false) m
  m_size_bounds_1 : 0 ≤ spec_len(m)
  m_size_bounds_2 : spec_len(m) < 18446744073709551616
  ⊢ spec_len(m) ≤
      spec_len(SpecMap.ofEntries SpecInt.val SpecInt.val
          (SpecMap.writeBack
            (fun entry => ({ val := 0, fits := ⋯ }.ofInt entry.fst, { val := 0, fits := ⋯ }.ofInt entry.snd))
            spec_set(m, k, v)))
---
error: verex verification failed: the proof of `v_len_set` does not establish its specification
---
error: `decide` does not close the obligation from the block's facts:
  case assert_1
  k : (Proofs.Denote.NTy.int 8 false).carrier
  v : (Proofs.Denote.NTy.int 64 false).carrier
  m : ((Proofs.Denote.NTy.int 8 false).table (Proofs.Denote.NTy.int 64 false)).carrier
  n_handle : Proofs.Denote.NTy.address.carrier
  initialState✝ : Proofs.Denote.Memory unit
  assumed : True
  verexInvariant✝ :
    SpecMap.Valid (SpecMap.Discipline.ordered SpecMap.intOrder) (SpecMap.ofEntries SpecInt.val SpecInt.val m)
  bounds : m.entries.length < 2 ^ 64
  k_lower : 0 ≤ k
  k_upper : k ≤ 2 ^ 8 - 1
  v_lower : 0 ≤ v
  v_upper : v ≤ 2 ^ 64 - 1
  left✝¹ : 0 ≤ (Proofs.Denote.NTy.int 64 false).plainFallback ⋯
  right✝¹ : (Proofs.Denote.NTy.int 64 false).plainFallback ⋯ ≤ 2 ^ 64 - 1
  n : SpecMap Int Int
  n_valid :
    (0x1.pure_spec_map_fact.unit.nominal.namespace.0.declaration✝⁻⁰ (Proofs.Denote.NTy.int 8 false)
          (Proofs.Denote.NTy.int 64 false)).Holds
      (n_handle,
        SpecMap.writeBack
          (fun entry =>
            ((Proofs.Denote.NTy.int 8 false).unread ⋯ entry.fst, (Proofs.Denote.NTy.int 64 false).unread ⋯ entry.snd))
          n,
        PUnit.unit)
  n_read :
    SpecMap.ofEntries SpecInt.val SpecInt.val
        (SpecMap.writeBack
          (fun entry => ({ val := 0, fits := ⋯ }.ofInt entry.fst, { val := 0, fits := ⋯ }.ofInt entry.snd)) n) ==
      n
  verexInvariant : SpecMap.Valid (SpecMap.Discipline.ordered SpecMap.intOrder) n
  n_keys : SpecMap.KeysSatisfy (IntegerValueFits (IntWidth.bits 8) false) n
  n_values : SpecMap.ValuesSatisfy (IntegerValueFits (IntWidth.bits 64) false) n
  n_size_bounds_1 : 0 ≤ spec_len(n)
  n_size_bounds_2 : spec_len(n) < 18446744073709551616
  left✝ : 0 ≤ spec_get(n, k)
  right✝ : spec_get(n, k) ≤ 18446744073709551615
  ⊢ spec_get(n, k) == v
---
error: verex verification failed: the proof of `v_set_param` does not establish its specification
-/
#guard_rendering in
verex module 0x1::pure_spec_map_fact where

  -- Copyright © Aptos Foundation
  -- SPDX-License-Identifier: Apache-2.0
  @[intrinsic_map]
  struct Map {K : phantom type has Copy, Drop} {V : phantom type} has Store where
    handle : Address

  spec Map where
    pragma intrinsic = map

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

  @[map_spec_key_at (Map)]
  opaque spec fun spec_key_at {K} {V}(t : Map<K, V>, i : Int) : K

  @[map_spec_rank (Map)]
  opaque spec fun spec_rank {K} {V}(t : Map<K, V>, k : K) : Int

  /--
  The size of an updated map, by a lemma with no default hole.
  -/
  fun v_len_set(
    m : &Map<u64, u64>, k : u64, v : u64
  ) -> Unit := spec assert spec_len::<u64, u64>(spec_set::<u64, u64>(m, k, v))
    >= spec_len::<u64, u64>(m)

  /--
  The updated map as a parameter, equated by a precondition.
  -/
  fun v_set_param(
    m : &Map<u8, u64>, n : &Map<u8, u64>, k : u8, v : u64
  ) -> Unit := spec assert spec_get::<u8, u64>(n, k) == v

  spec v_set_param where
    requires n == spec_set::<u8, u64>(m, k, v)

  /--
  Control: a lemma of the parameter map itself.
  -/
  fun v_rank_nonneg(
    m : &Map<u64, u64>, k : u64
  ) -> Unit := spec assert spec_rank::<u64, u64>(m, k) >= 0

  verify v_len_set by
    on assert 1 {
      apply lemma_len_set(m, k, v)
    }
    on _ { }

  verify v_set_param by
    on assert 1 {
      apply lemma_get_set(m, k, v, k)
    }
    on _ { }

  verify v_rank_nonneg by
    on assert 1 {
      apply lemma_rank_nonneg(m, k)
    }
    on _ { }

-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexSL.Tests.Rendered.Support

/-!
# `map_key_default` as a frontend renders it

The module below is the rendering of
`VerexE2ETests/SourceVerify/map_key_default.move` that the source-verification
driver writes to `.verex/map_key_default.move.lean`, with the items of
`VerexE2ETests/SourceVerify/map_key_default.proof.lean` as items of the module
and the `pragma proof_file` that names that file dropped: the driver defers
this fixture's verification to the proof file, which a lone elaboration does
not read, so the snapshot carries the proofs themselves and proves what the
fixture owes. Its `verex sources at` command is dropped and the diagnostics
are those of `map_key_default.exp`, which holds the proof file's messages as
well. Nothing here is edited by hand: `scripts/prover-smoke-renderings.sh
--write` writes this file again, and without `--write` tells whether it is
what the frontend renders and what the proof file says.
-/

/-!
A key a map lemma states is the key an assertion owes: the lemma's
instance reads outside the map by the owner's typed view of the KEYS.
`key_at_rank_narrow` holds values of another width than its keys and
`key_at_rank_generic` keys that are no integers, so a default taken
from the values or synthesized for an integer is another term.
-/

/--
error: `decide` does not close the obligation from the block's facts:
  case assert_1
  m_handle : Proofs.Denote.NTy.address.carrier
  initialState✝ : Proofs.Denote.Memory unit
  assumed : True
  left✝¹ : 0 ≤ (Proofs.Denote.NTy.int 64 false).plainFallback ⋯
  right✝¹ : (Proofs.Denote.NTy.int 64 false).plainFallback ⋯ ≤ 2 ^ 64 - 1
  m : SpecMap Int Int
  m_valid :
    (0x1.map_key_default.unit.nominal.namespace.0.declaration✝⁻⁰ (Proofs.Denote.NTy.int 64 false)
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
  permitted_1 : spec_contains(m, 7)
  permitted_2 : spec_rank(m, 7) == 1
  verexInvariant : SpecMap.Valid (SpecMap.Discipline.ordered SpecMap.intOrder) m
  m_keys : SpecMap.KeysSatisfy (IntegerValueFits (IntWidth.bits 64) false) m
  m_values : SpecMap.ValuesSatisfy (IntegerValueFits (IntWidth.bits 64) false) m
  fact_0_0 : spec_key_at(m, 1) == 7
  m_size_bounds_1 : 0 ≤ spec_len(m)
  m_size_bounds_2 : spec_len(m) < 18446744073709551616
  left✝ right✝ : True
  ⊢ False
---
error: verex verification failed: the proof of `key_at_other` does not establish its specification
---
error: `decide` does not close the obligation from the block's facts:
  case assert_1
  m_handle : Proofs.Denote.NTy.address.carrier
  initialState✝ : Proofs.Denote.Memory unit
  assumed permitted : True
  left✝¹ : 0 ≤ (Proofs.Denote.NTy.int 64 false).plainFallback ⋯
  right✝¹ : (Proofs.Denote.NTy.int 64 false).plainFallback ⋯ ≤ 2 ^ 64 - 1
  m : SpecMap Int Int
  m_valid :
    (0x1.map_key_default.unit.nominal.namespace.0.declaration✝⁻⁰ (Proofs.Denote.NTy.int 64 false)
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
  left✝ : 0 ≤ spec_key_at(m, spec_len(m))
  right✝ : spec_key_at(m, spec_len(m)) ≤ 18446744073709551615
  ⊢ spec_key_at(m, spec_len(m)) == 0
---
error: verex verification failed: the proof of `key_at_outside` does not establish its specification
---
error: the lemma's `map` does not fit its theorem: SpecMap (Proofs.Denote.NTy.param 0).readCarrier Int against SpecMap Int ?V
---
error: verex verification failed: the proof of `key_at_lt_generic` does not establish its specification
-/
#guard_rendering in
verex module 0x1::map_key_default where

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

  -- The intrinsic requires the value roles even where no cell reads one.
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
  The key at a present key's position is the key.
  -/
  fun key_at_rank(
    m : &Map<u64, u64>, k : u64
  ) -> Unit := spec assert spec_key_at::<u64, u64>(
    m, spec_rank::<u64, u64>(m, k)
  )
    == k

  spec key_at_rank where
    requires spec_contains::<u64, u64>(m, k)

  /--
  The same at a ground position.
  -/
  fun key_at_ground(
    m : &Map<u64, u64>
  ) -> Unit := spec assert spec_key_at::<u64, u64>(m, 1) == 7

  spec key_at_ground where
    requires spec_contains::<u64, u64>(m, 7)
    requires spec_rank::<u64, u64>(m, 7) == 1

  /--
  The same where the values are of another width than the keys.
  -/
  fun key_at_rank_narrow(
    m : &Map<u64, u8>, k : u64
  ) -> Unit := spec assert spec_key_at::<u64, u8>(m, spec_rank::<u64, u8>(m, k))
    == k

  spec key_at_rank_narrow where
    requires spec_contains::<u64, u8>(m, k)

  /--
  The same at an owner whose keys are a type parameter.
  -/
  fun key_at_rank_generic {K has Copy, Drop}(
    m : &Map<K, u64>, k : K
  ) -> Unit := spec assert spec_key_at::<K, u64>(m, spec_rank::<K, u64>(m, k))
    == k

  spec key_at_rank_generic where
    requires spec_contains::<K, u64>(m, k)

  /--
  Negative: the lemma's fact is of another key.
  -/
  fun key_at_other(
    m : &Map<u64, u64>
  ) -> Unit := spec assert spec_key_at::<u64, u64>(m, 1) == 8

  spec key_at_other where
    requires spec_contains::<u64, u64>(m, 7)
    requires spec_rank::<u64, u64>(m, 7) == 1

  -- error: the key at position 1 is 7
  /--
  Negative: a read outside the map has no stated value.
  -/
  fun key_at_outside(
    m : &Map<u64, u64>
  ) -> Unit := spec assert spec_key_at::<u64, u64>(m, spec_len::<u64, u64>(m))
    == 0

  -- error: unspecified outside
  /--
  Negative: an integer-keyed law at keys that are no integers is
  refused outright, and nothing is synthesized for it.
  -/
  fun key_at_lt_generic {K has Copy, Drop}(
    m : &Map<K, u64>
  ) -> Unit := spec assert spec_rank::<K, u64>(m, spec_key_at::<K, u64>(m, 0))
    == 0

  spec key_at_lt_generic where
    requires spec_len::<K, u64>(m) > 0

  -- error: the lemma does not fit

  -- Every cell has a block, the negatives too: a cell left to the automatic
  -- route names this file in its failure, which the snapshot of this module
  -- (the same items among the module's own) words differently.

  verify key_at_rank by
    on assert 1 {
      apply lemma_key_at_rank(m, k)
    }
    on _ { }

  verify key_at_ground by
    on assert 1 {
      apply lemma_key_at_rank(m, 7)
    }
    on _ { }

  verify key_at_rank_narrow by
    on assert 1 {
      apply lemma_key_at_rank(m, k)
    }
    on _ { }

  verify key_at_rank_generic by
    on assert 1 {
      apply lemma_key_at_rank(m, k)
    }
    on _ { }

  verify key_at_other by
    on assert 1 {
      apply lemma_key_at_rank(m, 7)
    }
    on _ { }

  verify key_at_outside by
    on _ { }

  verify key_at_lt_generic by
    on assert 1 {
      apply lemma_key_at_lt(m, 0, 1)
    }
    on _ { }

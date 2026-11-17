-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexSL.Tests.Rendered.Support

/-!
# `map_value_default` as a frontend renders it

The module below is the rendering of
`VerexE2ETests/SourceVerify/map_value_default.move` that the
source-verification driver writes to `.verex/map_value_default.move.lean`,
with the items of `VerexE2ETests/SourceVerify/map_value_default.proof.lean` as
items of the module and the `pragma proof_file` that names that file dropped:
the driver defers this fixture's verification to the proof file, which a lone
elaboration does not read, so the snapshot carries the proofs themselves and
proves what the fixture owes. Its `verex sources at` command is dropped and
the diagnostics are those of `map_value_default.exp`, which holds the proof
file's messages as well. Nothing here is edited by hand:
`scripts/prover-smoke-renderings.sh --write` writes this file again, and
without `--write` tells whether it is what the frontend renders and what the
proof file says.
-/

/-!
A value a map lemma states is the value an assertion owes: the lemma's
instance reads outside the map by the owner's typed view of the VALUES.
The keys are bytes and the values words, which both read as integers,
so a default taken from the keys is another term. The two executable
roles are what a lemma of an updated or a removed map needs: its fact
is of the map after the operation, which the owner's own `add` and
`erase` give it.
-/

/--
error: `decide` does not close the obligation from the block's facts:
  case assert_1
  k : (Proofs.Denote.NTy.int 8 false).carrier
  m_handle : Proofs.Denote.NTy.address.carrier
  initialState✝ : Proofs.Denote.Memory unit
  assumed : True
  k_lower : 0 ≤ k
  k_upper : k ≤ 2 ^ 8 - 1
  left✝¹ : 0 ≤ (Proofs.Denote.NTy.int 64 false).plainFallback ⋯
  right✝¹ : (Proofs.Denote.NTy.int 64 false).plainFallback ⋯ ≤ 2 ^ 64 - 1
  m : SpecMap Int Int
  m_valid :
    (0x1.map_value_default.unit.nominal.namespace.0.declaration✝⁻⁰ (Proofs.Denote.NTy.int 8 false)
          (Proofs.Denote.NTy.int 64 false)).Holds
      (m_handle,
        SpecMap.writeBack
          (fun entry =>
            ((Proofs.Denote.NTy.int 8 false).unread ⋯ entry.fst, (Proofs.Denote.NTy.int 64 false).unread ⋯ entry.snd))
          m,
        PUnit.unit)
  m_read :
    SpecMap.ofEntries SpecInt.val SpecInt.val
        (SpecMap.writeBack
          (fun entry => ({ val := 0, fits := ⋯ }.ofInt entry.fst, { val := 0, fits := ⋯ }.ofInt entry.snd)) m) ==
      m
  permitted : !spec_contains(m, k)
  verexInvariant : SpecMap.Valid SpecMap.Discipline.sequence m
  m_keys : SpecMap.KeysSatisfy (IntegerValueFits (IntWidth.bits 8) false) m
  m_values : SpecMap.ValuesSatisfy (IntegerValueFits (IntWidth.bits 64) false) m
  m_size_bounds_1 : 0 ≤ map_spec_len(m)
  m_size_bounds_2 : map_spec_len(m) < 18446744073709551616
  left✝ : 0 ≤ spec_get(m, k)
  right✝ : spec_get(m, k) ≤ 18446744073709551615
  ⊢ spec_get(m, k) == 0
---
error: verex verification failed: the proof of `value_at_missing` does not establish its specification
-/
#guard_rendering in
verex module 0x1::map_value_default where

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

  @[map_del_must_exist (Map)]
  native fun erase {K has Copy, Drop} {V}(self : &mut Map<K, V>, k : K) -> V

  spec erase where
    pragma intrinsic

  @[map_spec_has_key (Map)]
  opaque spec fun spec_contains {K} {V}(t : Map<K, V>, k : K) : Bool

  @[map_spec_get (Map)]
  opaque spec fun spec_get {K} {V}(t : Map<K, V>, k : K) : V

  @[map_spec_set (Map)]
  opaque spec fun spec_set {K} {V}(t : Map<K, V>, k : K, v : V) : Map<K, V>

  @[map_spec_del (Map)]
  opaque spec fun spec_remove {K} {V}(t : Map<K, V>, k : K) : Map<K, V>

  /--
  The value at an updated key is the value.
  -/
  fun value_at_set(m : &mut Map<u8, u64>, k : u8, v : u64) -> Unit := do
    m.add(k, v)
    spec assert spec_get::<u8, u64>(m, k) == v

  /--
  Another key keeps its value when a key is removed.
  -/
  fun value_at_erase(m : &mut Map<u8, u64>, k : u8, p : u8) -> Unit := do
    let _dropped := m.erase(k)
    spec assert spec_get::<u8, u64>(m, p) == spec_get::<u8, u64>(old(m), p)

  spec value_at_erase where
    requires p != k

  /--
  Negative: the value of a key without an entry has none stated.
  -/
  fun value_at_missing(
    m : &Map<u8, u64>, k : u8
  ) -> Unit := spec assert spec_get::<u8, u64>(m, k) == 0

  spec value_at_missing where
    requires !spec_contains::<u8, u64>(m, k)

  -- error: unspecified without an entry

  -- Every cell has a block, the negative too: a cell left to the automatic
  -- route names this file in its failure, which the snapshot of this module
  -- (the same items among the module's own) words differently.

  verify value_at_set by
    on assert 1 {
      apply lemma_get_set(old(m), k, v, k)
    }
    on _ { }

  verify value_at_erase by
    on assert 1 {
      apply lemma_get_remove(old(m), k, p)
    }
    on _ { }

  verify value_at_missing by
    on _ { }

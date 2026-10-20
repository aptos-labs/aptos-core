-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

-- Preparation modifiers must not reserve ordinary identifiers in imported code.
example (values typed : Nat) : values + typed = typed + values := by omega

open VerexIR VerexIR.Proofs.Denote

-- The key and value types are independent. The two removal disciplines
-- preserve their distinct source-visible enumeration orders.
example : SpecMap.remove .sequence [(1, "one"), (2, "two"), (3, "three")] (1 : Int) =
    [(3, "three"), (2, "two")] := by decide

example : SpecMap.remove (.ordered SpecMap.intOrder)
    [(1, "one"), (2, "two"), (3, "three")] 1 = [(2, "two"), (3, "three")] := by decide

example : SpecMap.update (.ordered SpecMap.intOrder) [(1, true), (3, false)] 2 false =
    [(1, true), (2, false), (3, false)] := by decide

-- Key reasoning does not constrain a map's values to integers.
example [Carriers] (owner : StructHandle)
    (m : (NTy.table (.int 64 false) .bool).carrier) (key : Int)
    (present : Maps.hasKey (.nominal owner none
      #[NTy.encode (.table (.int 64 false) .bool) m]) (.integer key) = true) :
    Maps.rank (.nominal owner none #[NTy.encode (.table (.int 64 false) .bool) m]) (.integer key) <
      Maps.size (.nominal owner none #[NTy.encode (.table (.int 64 false) .bool) m]) := by
  verex_lift_maps
  guard_hyp m : SpecMap Int Bool
  exact SpecMap.rank_lt_size present

example [Carriers] (owner : StructHandle)
    (m : (NTy.table (.int 64 false) .bool).carrier)
    (keys : SpecVector (SpecInt (.bits 64) false))
    (returned : RuntimeValue.vector (keys.values.map (Proofs.Codec.specInt (.bits 64) false).encode) =
      Maps.keysOf (.nominal owner none #[NTy.encode (.table (.int 64 false) .bool) m])) :
    (keys.values.size : Int) =
      Maps.size (.nominal owner none #[NTy.encode (.table (.int 64 false) .bool) m]) := by
  verex_lift_maps
  have length := SpecMap.size_of_keys returned
  simpa only [List.length_map, Array.length_toList] using length

-- A guard supplies the bounds for the key read within its branch.
example [Carriers] (owner : StructHandle) (ranks : ValueRanks)
    (m : (NTy.table (.int 64 false) (.int 64 false)).carrier) (i key : Int)
    (high : i < Maps.size (.nominal owner none
      #[NTy.encode (.table (.int 64 false) (.int 64 false)) m]))
    (before : 0 < i ∧ RuntimeValue.order ranks
      (Maps.keyAt (.nominal owner none #[NTy.encode (.table (.int 64 false) (.int 64 false)) m]) (i - 1))
      (.integer key) = .lt) :
    (Maps.keyAt (.nominal owner none
      #[NTy.encode (.table (.int 64 false) (.int 64 false)) m]) (i - 1)).asInt < key := by
  verex_lift_maps
  exact before.2

-- The typed preparation retains the vector-size certificate and the source
-- key range, including small integer types. It needs no RuntimeValue key.
example [Carriers] (owner : StructHandle) (validity : RuntimeValue)
    (m : (NTy.table (.int 8 false) (.int 64 false)).carrier) :
    Maps.size (.nominal owner none
      #[validity, NTy.encode (.table (.int 8 false) (.int 64 false)) m]) < 2 ^ 64 := by
  verex_lift_maps
  guard_hyp m : SpecMap Int Int
  exact m_size_bounds.2

example [Carriers] (owner : StructHandle) (validity : RuntimeValue)
    (m : (NTy.table (.int 8 false) (.int 64 false)).carrier) (i : Int)
    (low : 0 ≤ i)
    (high : i < Maps.size (.nominal owner none
      #[validity, NTy.encode (.table (.int 8 false) (.int 64 false)) m])) :
    IntegerValueFits (.bits 8) false (Maps.keyAt (.nominal owner none
      #[validity, NTy.encode (.table (.int 8 false) (.int 64 false)) m]) i).asInt := by
  verex_lift_maps
  guard_hyp m : SpecMap Int Int
  exact SpecMap.keyAt_satisfies m_keys low high

-- A conditional mutation with fresh validity stamps yields a typed content
-- equality. The conversion is a projection of a hypothesis, never a claim
-- that equal contents imply equal allocation identities or validity stamps.
example [Carriers] (owner : StructHandle) (validity nextValidity : RuntimeValue)
    (m result : (NTy.table (.int 64 false) (.int 64 false)).carrier)
    (layout : Maps.Layout) (ranks : ValueRanks) (key : Int)
    (present : Maps.hasKey (.nominal owner none
      #[validity, NTy.encode (.table (.int 64 false) (.int 64 false)) m]) (.integer key) = true)
    (mutated : Maps.hasKey (.nominal owner none
        #[validity, NTy.encode (.table (.int 64 false) (.int 64 false)) m]) (.integer key) = true →
      ∃ stamp, RuntimeValue.nominal owner none
          #[nextValidity, NTy.encode (.table (.int 64 false) (.int 64 false)) result] =
        layout.restamp (Maps.remove layout (.ordered ranks) (.nominal owner none
          #[validity, NTy.encode (.table (.int 64 false) (.int 64 false)) m]) (.integer key)) stamp) :
    Maps.size (.nominal owner none
        #[nextValidity, NTy.encode (.table (.int 64 false) (.int 64 false)) result]) =
      Maps.size (.nominal owner none
        #[validity, NTy.encode (.table (.int 64 false) (.int 64 false)) m]) - 1 := by
  verex_lift_maps
  guard_hyp m : SpecMap Int Int
  guard_hyp result : SpecMap Int Int
  rw [mutated present, SpecMap.size_remove, present, if_pos rfl]

-- An absent read stays absent: decoding its sentinel as zero does not create
-- an entry at key zero.
example : SpecMap.valueAt ([] : SpecMap Int Int) 0 = 0 ∧
    SpecMap.hasKey ([] : SpecMap Int Int) 0 = false := by decide

private def delayedRank [Carriers] (owner : StructHandle) (validity : RuntimeValue)
    (m : (NTy.table (.int 64 false) (.int 64 false)).carrier) (key : Int) : Int :=
  Maps.rank (.nominal owner none
    #[validity, NTy.encode (.table (.int 64 false) (.int 64 false)) m]) (.integer key)

-- Unfolding a specification later must reuse the existing typed map, rather
-- than generalize a second, unrelated copy of the same contents.
example [Carriers] (owner : StructHandle) (validity : RuntimeValue)
    (m : (NTy.table (.int 64 false) (.int 64 false)).carrier) (key r : Int)
    (present : Maps.hasKey (.nominal owner none
      #[validity, NTy.encode (.table (.int 64 false) (.int 64 false)) m]) (.integer key) = true)
    (observed : delayedRank owner validity m key = r) :
    r < Maps.size (.nominal owner none
        #[validity, NTy.encode (.table (.int 64 false) (.int 64 false)) m]) := by
  verex_lift_maps only [present]
  guard_hyp m : SpecMap Int Int
  unfold delayedRank at observed
  verex_lift_maps only [observed]
  guard_hyp observed : SpecMap.rank m key = r
  rw [← observed]
  exact SpecMap.rank_lt_size present

-- Projecting an equality for content reasoning must also retain the original
-- equality, since a later obligation can still read identity or validity.
example [Carriers] (owner : StructHandle) (leftHandle rightHandle : RuntimeValue)
    (left right : (NTy.table (.int 64 false) (.int 64 false)).carrier)
    : (RuntimeValue.nominal owner none
        #[leftHandle, NTy.encode (.table (.int 64 false) (.int 64 false)) left] =
      .nominal owner none
        #[rightHandle, NTy.encode (.table (.int 64 false) (.int 64 false)) right]) →
    RuntimeValue.nominal owner none
        #[leftHandle, NTy.encode (.table (.int 64 false) (.int 64 false)) left] =
      .nominal owner none
        #[rightHandle, NTy.encode (.table (.int 64 false) (.int 64 false)) right] := by
  intro same
  verex_lift_maps
  all_goals assumption

-- Literal construction uses the same typed model as symbolic maps.
example [Carriers] (owner : StructHandle)
    (m : (NTy.table (.int 64 false) (.int 64 false)).carrier) (layout : Maps.Layout)
    (contents : RuntimeValue.nominal owner none
      #[NTy.encode (.table (.int 64 false) (.int 64 false)) m] =
      layout.build .unit [(.integer 1, .integer 4), (.integer 2, .integer 5)]) :
    (Maps.keyAt (.nominal owner none
      #[NTy.encode (.table (.int 64 false) (.int 64 false)) m]) 1).asInt = 2 := by
  verex_lift_maps
  guard_hyp contents : m = [(1, 4), (2, 5)]
  simp [contents, SpecMap.keyAt]

-- A literal map also supplies the bound needed to transport a nested read.
example [Carriers] (owner : StructHandle)
    (m : (NTy.table (.int 64 false) (.int 64 false)).carrier) (layout : Maps.Layout)
    (contents : RuntimeValue.nominal owner none
      #[NTy.encode (.table (.int 64 false) (.int 64 false)) m] =
      layout.build .unit [(.integer 1, .integer 4), (.integer 2, .integer 5)]) :
    (Maps.valueAt (.nominal owner none
      #[NTy.encode (.table (.int 64 false) (.int 64 false)) m])
      (Maps.keyAt (.nominal owner none
        #[NTy.encode (.table (.int 64 false) (.int 64 false)) m]) 1)).asInt = 5 := by
  verex_lift_maps
  change SpecMap.valueAt m (SpecMap.keyAt m 1) = 5
  simp [contents, SpecMap.keyAt, SpecMap.valueAt, SpecMap.get]

-- Branches may choose fresh stamps independently. Their map contents still
-- project through the disjunction, without identifying the stamps.
example [Carriers] (owner : StructHandle) (validity : RuntimeValue)
    (m result : (NTy.table (.int 64 false) (.int 64 false)).carrier)
    (layout : Maps.Layout) (ranks : ValueRanks) (key : Int) (branch : Bool)
    (changed :
      (branch = true ∧ ∃ stamp, RuntimeValue.nominal owner none
          #[NTy.encode (.table (.int 64 false) (.int 64 false)) result] =
        layout.restamp (Maps.update layout (.ordered ranks) (.nominal owner none
          #[validity, NTy.encode (.table (.int 64 false) (.int 64 false)) m]) (.integer key) (.integer 7)) stamp) ∨
      (branch = false ∧ ∃ stamp, RuntimeValue.nominal owner none
          #[NTy.encode (.table (.int 64 false) (.int 64 false)) result] =
        layout.restamp (Maps.update layout (.ordered ranks) (.nominal owner none
          #[validity, NTy.encode (.table (.int 64 false) (.int 64 false)) m]) (.integer key) (.integer 7)) stamp)) :
    (Maps.valueAt (.nominal owner none
      #[NTy.encode (.table (.int 64 false) (.int 64 false)) result]) (.integer key)).asInt = 7 := by
  verex_lift_maps
  rcases changed with ⟨_, changed⟩ | ⟨_, changed⟩
  all_goals rw [changed, SpecMap.valueAt_update, if_pos rfl]

-- The optional-result projection preserves None and Some 0 as distinct.
-- A successor may be zero when signed keys precede it.
example [Carriers] (owner optionOwner : StructHandle)
    (m : (NTy.table (.int 64 true) (.int 64 false)).carrier)
    (ranks : ValueRanks) (key : Int)
    (present : Maps.hasKey (.nominal owner none
      #[NTy.encode (.table (.int 64 true) (.int 64 false)) m]) (.integer key) = true)
    (valid : Maps.Valid (.ordered ranks) (.nominal owner none
      #[NTy.encode (.table (.int 64 true) (.int 64 false)) m]))
    (next : (Maps.nextKey? ranks (.nominal owner none
        #[NTy.encode (.table (.int 64 true) (.int 64 false)) m]) (.integer key)).elim
      (RuntimeValue.nominal optionOwner (some "None") #[])
      (fun value => RuntimeValue.nominal optionOwner (some "Some") #[value]) =
        RuntimeValue.nominal optionOwner (some "Some") #[.integer 0]) :
    Maps.hasKey (.nominal owner none
      #[NTy.encode (.table (.int 64 true) (.int 64 false)) m]) (.integer 0) = true := by
  verex_lift_maps
  guard_hyp next : SpecMap.nextKey? SpecMap.intOrder m key = some 0
  exact (SpecMap.nextKey_some valid present next).1

example [Carriers] (owner optionOwner : StructHandle)
    (m : (NTy.table (.int 64 false) (.int 64 false)).carrier)
    (ranks : ValueRanks) (key : Int)
    (present : Maps.hasKey (.nominal owner none
      #[NTy.encode (.table (.int 64 false) (.int 64 false)) m]) (.integer key) = true)
    (valid : Maps.Valid (.ordered ranks) (.nominal owner none
      #[NTy.encode (.table (.int 64 false) (.int 64 false)) m]))
    (next : (Maps.nextKey? ranks (.nominal owner none
        #[NTy.encode (.table (.int 64 false) (.int 64 false)) m]) (.integer key)).elim
      (RuntimeValue.nominal optionOwner (some "None") #[])
      (fun value => RuntimeValue.nominal optionOwner (some "Some") #[value]) =
        RuntimeValue.nominal optionOwner (some "None") #[]) :
    Maps.rank (.nominal owner none
        #[NTy.encode (.table (.int 64 false) (.int 64 false)) m]) (.integer key) + 1 =
      Maps.size (.nominal owner none
        #[NTy.encode (.table (.int 64 false) (.int 64 false)) m]) := by
  verex_lift_maps
  guard_hyp next : SpecMap.nextKey? SpecMap.intOrder m key = none
  exact SpecMap.nextKey_none valid present next

-- Selection must leave unrelated maps and their observations untouched.
example [Carriers] (owner : StructHandle)
    (m n : (NTy.table (.int 64 false) (.int 64 false)).carrier) (key : Int)
    (present : Maps.hasKey (.nominal owner none
      #[NTy.encode (.table (.int 64 false) (.int 64 false)) m]) (.integer key) = true)
    (_unrelated : Maps.hasKey (.nominal owner none
      #[NTy.encode (.table (.int 64 false) (.int 64 false)) n]) (.integer key) = true) :
    Maps.rank (.nominal owner none
      #[NTy.encode (.table (.int 64 false) (.int 64 false)) m]) (.integer key) <
      Maps.size (.nominal owner none
        #[NTy.encode (.table (.int 64 false) (.int 64 false)) m]) := by
  verex_lift_maps only [present]
  guard_hyp m : SpecMap Int Int
  guard_hyp n : (NTy.table (.int 64 false) (.int 64 false)).carrier
  guard_hyp _unrelated : Maps.hasKey (.nominal owner none
    #[NTy.encode (.table (.int 64 false) (.int 64 false)) n]) (.integer key) = true
  exact SpecMap.rank_lt_size present

-- Aggregate value carriers need no globally registered Inhabited instance.
example [Carriers] (owner : StructHandle)
    (m : (NTy.table (.int 64 false)
      (.struct owner .nil (.cons (.int 64 false) .nil))).carrier) (key : Int)
    (present : Maps.hasKey (.nominal owner none
      #[NTy.encode (.table (.int 64 false)
        (.struct owner .nil (.cons (.int 64 false) .nil))) m]) (.integer key) = true) :
    Maps.rank (.nominal owner none
      #[NTy.encode (.table (.int 64 false)
        (.struct owner .nil (.cons (.int 64 false) .nil))) m]) (.integer key) <
      Maps.size (.nominal owner none
        #[NTy.encode (.table (.int 64 false)
          (.struct owner .nil (.cons (.int 64 false) .nil))) m]) := by
  verex_lift_maps
  exact SpecMap.rank_lt_size present

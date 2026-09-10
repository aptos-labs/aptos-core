--# publish

import Move

open scoped Move

/-!
The core of Aptos `ordered_map`, represented as a sorted vector.  The generic
implementation below is the same source shape as the Lean-side benchmark;
the public concrete functions exercise it through compiler v2 and MoveVM.
-/
module LeanerOrderedMap where

  @[move_struct]
  structure Entry (K V : Type) where
    key : K
    value : V
    deriving Copy, Drop, Store

  @[move_struct]
  structure Map (K V : Type) where
    entries : Move.Vector (Entry K V)
    deriving Copy, Drop, Store

  @[move_struct]
  structure U64Store where
    map : Map U64 U64
    deriving Key

  @[move_struct]
  structure BoolStore where
    map : Map Bool U64
    deriving Key

  /-! ## Functions -/

  fun empty {K V : Type} : Map K V :=
    { entries := Move.Vector.empty }

  partial fun lower_bound_loop {K V : Type} (entries : &Move.Vector (Entry K V))
      (key : &K) (low high : U64) : Action U64 := do
    if low < high then
      let middle := low + ((high - low) / 2)
      let entryKey ← &entries[middle].key
      if entryKey < key then
        continue lower_bound_loop entries key (middle + 1) high
      else
        continue lower_bound_loop entries key low middle
    else
      pure low

  fun lower_bound {K V : Type} (map : &Map K V) (key : &K) : Action U64 := do
    let entries ← &map.entries
    lower_bound_loop entries key 0 entries.length

  fun length {K V : Type} (map : &Map K V) : Action U64 := do
    let entries ← &map.entries
    pure entries.length

  fun borrow_key_at {K V : Type} (map : &Map K V) (index : U64) : Action (&K) := do
    let entries ← &map.entries
    &entries[index].key

  fun contains {K V : Type} (map : &Map K V) (key : &K) : Action Bool := do
    let index ← lower_bound map key
    let entries ← &map.entries
    if index < entries.length then
      let entryKey ← &entries[index].key
      pure (entryKey == key)
    else
      pure false

  fun borrow {K V : Type} (map : &Map K V) (key : &K) : Action (&V) := do
    let index ← lower_bound map key
    let entries ← &map.entries
    if index < entries.length then
      let entryKey ← &entries[index].key
      if entryKey == key then
        &entries[index].value
      else
        abort 2
    else
      abort 2

  fun get_u64 {K : Type} (map : &Map K U64) (key : &K) : Action U64 := do
    let valueRef ← borrow map key
    (*valueRef)

  fun existing_index {K V : Type} (map : &Map K V) (key : &K) : Action U64 := do
    let index ← lower_bound map key
    let entries ← &map.entries
    if index < entries.length then
      let entryKey ← &entries[index].key
      if entryKey == key then
        pure index
      else
        abort 2
    else
      abort 2

  fun add {K V : Type} (map : &mut Map K V) (key : K) (value : V) :
      Action Unit := do
    let keyView ← &key
    let index ← lower_bound map keyView
    let entryCount ← length map
    if index < entryCount then
      let entriesView ← &map.entries
      let entryKey ← &entriesView[index].key
      if entryKey == keyView then
        abort 1
    let entries ← &mut map.entries
    entries.insert index { key, value }

  fun remove {K V : Type} (map : &mut Map K V) (key : &K) : Action V := do
    let index ← existing_index map key
    let entries ← &mut map.entries
    let removed ← entries.remove index
    pure removed.value

  fun populate_three (address : Address) : Action Unit := do
    let mapRef ← &mut U64Store[address].map
    add mapRef 30 300
    add mapRef 10 100
    add mapRef 20 200

  fun populate_booleans (address : Address) : Action Unit := do
    let mapRef ← &mut BoolStore[address].map
    add mapRef true 10
    add mapRef false 20

  @[move_public]
  fun publish_empty (signer : &Signer) : Action Unit :=
    moveTo signer ({ map := empty } : U64Store)

  @[move_public]
  fun publish_three (signer : &Signer) (address : Address) : Action Unit := do
    moveTo signer ({ map := empty } : U64Store)
    populate_three address

  @[move_public]
  fun publish_booleans (signer : &Signer) (address : Address) : Action Unit := do
    moveTo signer ({ map := empty } : BoolStore)
    populate_booleans address

  @[move_public]
  fun empty_length (address : Address) : Action U64 := do
    let mapRef ← &U64Store[address].map
    length mapRef

  @[move_public]
  fun lookup_three (address : Address) (key : U64) : Action U64 := do
    let mapRef ← &U64Store[address].map
    let keyRef ← &key
    get_u64 mapRef keyRef

  @[move_public]
  fun contains_three (address : Address) (key : U64) : Action Bool := do
    let mapRef ← &U64Store[address].map
    let keyRef ← &key
    contains mapRef keyRef

  @[move_public]
  fun insertion_order (address : Address) : Action U64 := do
    let mapRef ← &U64Store[address].map
    let firstRef ← borrow_key_at mapRef 0
    let first ← *firstRef
    let secondRef ← borrow_key_at mapRef 1
    let second ← *secondRef
    let thirdRef ← borrow_key_at mapRef 2
    let third ← *thirdRef
    pure (first * 100 + second * 10 + third)

  @[move_public]
  fun remove_middle (address : Address) : Action U64 := do
    let mapRef ← &mut U64Store[address].map
    let key : U64 := 20
    let keyRef ← &key
    let removed ← remove mapRef keyRef
    let stillPresent ← contains mapRef keyRef
    let remaining ← length mapRef
    if stillPresent then pure 0 else pure (removed + remaining)

  @[move_public]
  fun remove_edges (address : Address) : Action U64 := do
    let mapRef ← &mut U64Store[address].map
    let firstKey : U64 := 10
    let firstKeyRef ← &firstKey
    let first ← remove mapRef firstKeyRef
    let lastKey : U64 := 30
    let lastKeyRef ← &lastKey
    let last ← remove mapRef lastKeyRef
    let middleKey : U64 := 20
    let middleKeyRef ← &middleKey
    let middleRef ← borrow mapRef middleKeyRef
    let middle ← *middleRef
    pure (first + middle + last)

  @[move_public]
  fun bool_keys (address : Address) : Action U64 := do
    let mapRef ← &BoolStore[address].map
    let key : Bool := false
    let keyRef ← &key
    get_u64 mapRef keyRef

  @[move_public]
  fun duplicate_key (address : Address) : Action Unit := do
    let mapRef ← &mut U64Store[address].map
    add mapRef 10 999

  @[move_public]
  fun missing_remove (address : Address) : Action U64 := do
    let mapRef ← &mut U64Store[address].map
    let key : U64 := 11
    let keyRef ← &key
    remove mapRef keyRef

  @[move_public]
  fun missing_lookup (address : Address) : Action U64 := do
    let mapRef ← &U64Store[address].map
    let key : U64 := 11
    let keyRef ← &key
    get_u64 mapRef keyRef

/-! ## Tests -/

--# run --signers 0x40 -- 0x0::LeanerOrderedMap::publish_empty

--# run 0x0::LeanerOrderedMap::empty_length --args @0x40

--# run --args @0x41 --signers 0x41 -- 0x0::LeanerOrderedMap::publish_three

--# run 0x0::LeanerOrderedMap::lookup_three --args @0x41 10u64

--# run 0x0::LeanerOrderedMap::lookup_three --args @0x41 20u64

--# run 0x0::LeanerOrderedMap::lookup_three --args @0x41 30u64

--# run 0x0::LeanerOrderedMap::contains_three --args @0x41 20u64

--# run 0x0::LeanerOrderedMap::contains_three --args @0x41 11u64

--# run 0x0::LeanerOrderedMap::insertion_order --args @0x41

--# run --args @0x42 --signers 0x42 -- 0x0::LeanerOrderedMap::publish_three

--# run 0x0::LeanerOrderedMap::remove_middle --args @0x42

--# run --args @0x43 --signers 0x43 -- 0x0::LeanerOrderedMap::publish_three

--# run 0x0::LeanerOrderedMap::remove_edges --args @0x43

--# run --args @0x44 --signers 0x44 -- 0x0::LeanerOrderedMap::publish_booleans

--# run 0x0::LeanerOrderedMap::bool_keys --args @0x44

--# run 0x0::LeanerOrderedMap::duplicate_key --args @0x41

--# run 0x0::LeanerOrderedMap::missing_remove --args @0x41

--# run 0x0::LeanerOrderedMap::missing_lookup --args @0x41

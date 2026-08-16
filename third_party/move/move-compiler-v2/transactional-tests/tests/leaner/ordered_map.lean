--# publish

import Move

open scoped Move

/-!
The core of Aptos `ordered_map`, represented as a sorted vector.  The generic
implementation below is the same source shape as the Lean-side benchmark;
the public concrete functions exercise it through compiler v2 and MoveVM.
-/
move_module LeanerOrderedMap where

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

  fun empty {K V : Type} : Map K V :=
    { entries := Move.Vector.empty }

  partial fun lowerBoundLoop {K V : Type} (entries : &Move.Vector (Entry K V))
      (key : &K) (low high : U64) : Action U64 := do
    if low < high then
      let middle := low + ((high - low) / 2)
      let entryKey ← &entries[middle].key
      if entryKey < key then
        continue lowerBoundLoop entries key (middle + 1) high
      else
        continue lowerBoundLoop entries key low middle
    else
      pure low

  fun lowerBound {K V : Type} (map : &Map K V) (key : &K) : Action U64 := do
    let entries ← &map.entries
    lowerBoundLoop entries key 0 entries.length

  fun length {K V : Type} (map : &Map K V) : Action U64 := do
    let entries ← &map.entries
    pure entries.length

  fun borrowKeyAt {K V : Type} (map : &Map K V) (index : U64) : Action (&K) := do
    let entries ← &map.entries
    &entries[index].key

  fun contains {K V : Type} (map : &Map K V) (key : &K) : Action Bool := do
    let index ← lowerBound map key
    let entries ← &map.entries
    if index < entries.length then
      let entryKey ← &entries[index].key
      pure (entryKey == key)
    else
      pure false

  fun borrow {K V : Type} (map : &Map K V) (key : &K) : Action (&V) := do
    let index ← lowerBound map key
    let entries ← &map.entries
    if index < entries.length then
      let entryKey ← &entries[index].key
      if entryKey == key then
        &entries[index].value
      else
        abort 2
    else
      abort 2

  fun getU64 {K : Type} (map : &Map K U64) (key : &K) : Action U64 := do
    let valueRef ← borrow map key
    (*valueRef)

  fun existingIndex {K V : Type} (map : &Map K V) (key : &K) : Action U64 := do
    let index ← lowerBound map key
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
    let index ← lowerBound map keyView
    let entryCount ← length map
    if index < entryCount then
      let entriesView ← &map.entries
      let entryKey ← &entriesView[index].key
      if entryKey == keyView then
        abort 1
    let entries ← &mut map.entries
    entries.insert index { key, value }

  fun remove {K V : Type} (map : &mut Map K V) (key : &K) : Action V := do
    let index ← existingIndex map key
    let entries ← &mut map.entries
    let removed ← entries.remove index
    pure removed.value

  fun populateThree (address : Address) : Action Unit := do
    let mapRef ← &mut U64Store[address].map
    add mapRef 30 300
    add mapRef 10 100
    add mapRef 20 200

  fun populateBooleans (address : Address) : Action Unit := do
    let mapRef ← &mut BoolStore[address].map
    add mapRef true 10
    add mapRef false 20

  @[move_public]
  fun publishEmpty (signer : Signer) : Action Unit :=
    moveTo signer ({ map := empty } : U64Store)

  @[move_public]
  fun publishThree (signer : Signer) (address : Address) : Action Unit := do
    moveTo signer ({ map := empty } : U64Store)
    populateThree address

  @[move_public]
  fun publishBooleans (signer : Signer) (address : Address) : Action Unit := do
    moveTo signer ({ map := empty } : BoolStore)
    populateBooleans address

  @[move_public]
  fun emptyLength (address : Address) : Action U64 := do
    let mapRef ← &U64Store[address].map
    length mapRef

  @[move_public]
  fun lookupThree (address : Address) (key : U64) : Action U64 := do
    let mapRef ← &U64Store[address].map
    let keyRef ← &key
    getU64 mapRef keyRef

  @[move_public]
  fun containsThree (address : Address) (key : U64) : Action Bool := do
    let mapRef ← &U64Store[address].map
    let keyRef ← &key
    contains mapRef keyRef

  @[move_public]
  fun insertionOrder (address : Address) : Action U64 := do
    let mapRef ← &U64Store[address].map
    let firstRef ← borrowKeyAt mapRef 0
    let first ← *firstRef
    let secondRef ← borrowKeyAt mapRef 1
    let second ← *secondRef
    let thirdRef ← borrowKeyAt mapRef 2
    let third ← *thirdRef
    pure (first * 100 + second * 10 + third)

  @[move_public]
  fun removeMiddle (address : Address) : Action U64 := do
    let mapRef ← &mut U64Store[address].map
    let key : U64 := 20
    let keyRef ← &key
    let removed ← remove mapRef keyRef
    let stillPresent ← contains mapRef keyRef
    let remaining ← length mapRef
    if stillPresent then pure 0 else pure (removed + remaining)

  @[move_public]
  fun removeEdges (address : Address) : Action U64 := do
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
  fun boolKeys (address : Address) : Action U64 := do
    let mapRef ← &BoolStore[address].map
    let key : Bool := false
    let keyRef ← &key
    getU64 mapRef keyRef

  @[move_public]
  fun duplicateKey (address : Address) : Action Unit := do
    let mapRef ← &mut U64Store[address].map
    add mapRef 10 999

  @[move_public]
  fun missingRemove (address : Address) : Action U64 := do
    let mapRef ← &mut U64Store[address].map
    let key : U64 := 11
    let keyRef ← &key
    remove mapRef keyRef

  @[move_public]
  fun missingLookup (address : Address) : Action U64 := do
    let mapRef ← &U64Store[address].map
    let key : U64 := 11
    let keyRef ← &key
    getU64 mapRef keyRef

--# run --signers 0x40 -- 0x0::LeanerOrderedMap::publishEmpty

--# run 0x0::LeanerOrderedMap::emptyLength --args @0x40

--# run --args @0x41 --signers 0x41 -- 0x0::LeanerOrderedMap::publishThree

--# run 0x0::LeanerOrderedMap::lookupThree --args @0x41 10u64

--# run 0x0::LeanerOrderedMap::lookupThree --args @0x41 20u64

--# run 0x0::LeanerOrderedMap::lookupThree --args @0x41 30u64

--# run 0x0::LeanerOrderedMap::containsThree --args @0x41 20u64

--# run 0x0::LeanerOrderedMap::containsThree --args @0x41 11u64

--# run 0x0::LeanerOrderedMap::insertionOrder --args @0x41

--# run --args @0x42 --signers 0x42 -- 0x0::LeanerOrderedMap::publishThree

--# run 0x0::LeanerOrderedMap::removeMiddle --args @0x42

--# run --args @0x43 --signers 0x43 -- 0x0::LeanerOrderedMap::publishThree

--# run 0x0::LeanerOrderedMap::removeEdges --args @0x43

--# run --args @0x44 --signers 0x44 -- 0x0::LeanerOrderedMap::publishBooleans

--# run 0x0::LeanerOrderedMap::boolKeys --args @0x44

--# run 0x0::LeanerOrderedMap::duplicateKey --args @0x41

--# run 0x0::LeanerOrderedMap::missingRemove --args @0x41

--# run 0x0::LeanerOrderedMap::missingLookup --args @0x41

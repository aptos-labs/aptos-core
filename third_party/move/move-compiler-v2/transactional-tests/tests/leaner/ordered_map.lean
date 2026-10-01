-- Copyright © Aptos Foundation

--# publish

import LeanerMove

-- The core of Aptos `ordered_map`, represented as a sorted vector. The generic
-- implementation is exercised through compiler v2 and MoveVM at `u64` and
-- `Bool` keys.
leaner module 0x0::LeanerOrderedMap where
  use 0x1::std::cmp
  use 0x1::std::cmp::Ordering

  struct Entry {K has Copy, Drop, Store} {V has Copy, Drop, Store} has Copy, Drop, Store where
    key : K
    value : V

  struct Map {K has Copy, Drop, Store} {V has Copy, Drop, Store} has Copy, Drop, Store where
    entries : Vector<Entry<K, V> >

  struct U64Store has Key where
    map : Map<u64, u64>

  struct BoolStore has Key where
    map : Map<Bool, u64>

  fun empty {K has Copy, Drop, Store} {V has Copy, Drop, Store}() -> Map<K, V> :=
    new Map<K, V> { entries := vector<Entry<K, V> >[] }

  -- Binary search for the first entry whose key is not less than `key`.
  fun lower_bound_loop {K has Copy, Drop, Store} {V has Copy, Drop, Store}
      (entries : &Vector<Entry<K, V> >, key : &K, low : u64, high : u64) -> u64 := do
    let mut low := low
    let mut high := high
    while low < high do
      let middle := low + (high - low) / 2
      let order := (cmp::compare::<K>(&entries[middle].key, key) : Ordering)
      if (cmp::is_lt(&order) : Bool) then
        low := middle + 1
      else
        high := middle
    low

  fun lower_bound {K has Copy, Drop, Store} {V has Copy, Drop, Store}(map : &Map<K, V>, key : &K) ->
      u64 := do
    let entries := &map.entries
    lower_bound_loop::<K, V>(entries, key, 0, (*entries).length)

  fun length {K has Copy, Drop, Store} {V has Copy, Drop, Store}(map : &Map<K, V>) -> u64 := do
    let entries := &map.entries
    (*entries).length

  fun borrow_key_at {K has Copy, Drop, Store} {V has Copy, Drop, Store}(map : &Map<K, V>,
      index : u64) -> &K := do
    let entries := &map.entries
    &entries[index].key

  fun contains {K has Copy, Drop, Store} {V has Copy, Drop, Store}(map : &Map<K, V>,
      key : &K) -> Bool := do
    let index := lower_bound::<K, V>(map, key)
    let entries := &map.entries
    if index < (*entries).length then
      core.prim.equal(&entries[index].key, key)
    else false

  fun borrow {K has Copy, Drop, Store} {V has Copy, Drop, Store}(map : &Map<K, V>,
      key : &K) -> &V := do
    let index := lower_bound::<K, V>(map, key)
    let entries := &map.entries
    if index < (*entries).length then
      if core.prim.equal(&entries[index].key, key) then &entries[index].value
      else abort(2)
    else abort(2)

  fun get_u64 {K has Copy, Drop, Store}(map : &Map<K, u64>, key : &K) -> u64 := do
    let value := borrow::<K, u64>(map, key)
    *value

  fun existing_index {K has Copy, Drop, Store} {V has Copy, Drop, Store}(map : &Map<K, V>,
      key : &K) -> u64 := do
    let index := lower_bound::<K, V>(map, key)
    let entries := &map.entries
    if index < (*entries).length then
      if core.prim.equal(&entries[index].key, key) then index
      else abort(2)
    else abort(2)

  fun add {K has Copy, Drop, Store} {V has Copy, Drop, Store}(map : &mut Map<K, V>,
      key : K, value : V) -> Unit := do
    let index := lower_bound::<K, V>(map, &key)
    let entries := &map.entries
    if index < (*entries).length then
      if core.prim.equal(&entries[index].key, &key) then abort(1)
    let entry := new Entry<K, V> { key := key, value := value }
    let entries := &mut map.entries
    *entries := core.prim.insertVector(*entries, index, entry)

  fun remove {K has Copy, Drop, Store} {V has Copy, Drop, Store}(map : &mut Map<K, V>,
      key : &K) -> V := do
    let index := existing_index::<K, V>(map, key)
    let entries := &mut map.entries
    let (removed, rest) := core.prim.removeVector(*entries, index)
    *entries := rest
    removed.value

  fun populate_three(address : Address) -> Unit := do
    let map := &mut U64Store[address].map
    add::<u64, u64>(map, 30, 300)
    add::<u64, u64>(map, 10, 100)
    add::<u64, u64>(map, 20, 200)

  fun populate_booleans(address : Address) -> Unit := do
    let map := &mut BoolStore[address].map
    add::<Bool, u64>(map, true, 10)
    add::<Bool, u64>(map, false, 20)

  public fun publish_empty(account : &Signer) -> Unit :=
    move_to<U64Store>(account, new U64Store { map := empty::<u64, u64>() })

  public fun publish_three(account : &Signer, address : Address) -> Unit := do
    move_to<U64Store>(account, new U64Store { map := empty::<u64, u64>() })
    populate_three(address)

  public fun publish_booleans(account : &Signer, address : Address) -> Unit := do
    move_to<BoolStore>(account, new BoolStore { map := empty::<Bool, u64>() })
    populate_booleans(address)

  public fun empty_length(address : Address) -> u64 := do
    let map := &U64Store[address].map
    length::<u64, u64>(map)

  public fun lookup_three(address : Address, key : u64) -> u64 := do
    let map := &U64Store[address].map
    get_u64::<u64>(map, &key)

  public fun contains_three(address : Address, key : u64) -> Bool := do
    let map := &U64Store[address].map
    contains::<u64, u64>(map, &key)

  public fun insertion_order(address : Address) -> u64 := do
    let map := &U64Store[address].map
    let first := *borrow_key_at::<u64, u64>(map, 0)
    let second := *borrow_key_at::<u64, u64>(map, 1)
    let third := *borrow_key_at::<u64, u64>(map, 2)
    first * 100 + second * 10 + third

  public fun remove_middle(address : Address) -> u64 := do
    let map := &mut U64Store[address].map
    let key : u64 := 20
    let removed := remove::<u64, u64>(map, &key)
    let still_present := contains::<u64, u64>(map, &key)
    let remaining := length::<u64, u64>(map)
    if still_present then 0 else removed + remaining

  public fun remove_edges(address : Address) -> u64 := do
    let map := &mut U64Store[address].map
    let first_key : u64 := 10
    let first := remove::<u64, u64>(map, &first_key)
    let last_key : u64 := 30
    let last := remove::<u64, u64>(map, &last_key)
    let middle_key : u64 := 20
    let middle := *borrow::<u64, u64>(map, &middle_key)
    first + middle + last

  public fun bool_keys(address : Address) -> u64 := do
    let map := &BoolStore[address].map
    let key := false
    get_u64::<Bool>(map, &key)

  public fun duplicate_key(address : Address) -> Unit := do
    let map := &mut U64Store[address].map
    add::<u64, u64>(map, 10, 999)

  public fun missing_remove(address : Address) -> u64 := do
    let map := &mut U64Store[address].map
    let key : u64 := 11
    remove::<u64, u64>(map, &key)

  public fun missing_lookup(address : Address) -> u64 := do
    let map := &U64Store[address].map
    let key : u64 := 11
    get_u64::<u64>(map, &key)

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

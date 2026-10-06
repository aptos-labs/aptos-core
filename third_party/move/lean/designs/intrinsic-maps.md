# Intrinsic maps

Status: 2026-09-30, stage 1 implemented: `ordered_map` verifies (its own
iterator tests and a client module); `simple_map` and `pool_u64` in part.

A Move structure marked `pragma intrinsic = map` binds roles of the Move
Prover's map theory to functions of its module (`lir-design.md`, "Intrinsic
declarations and role graphs"; the Move-profile registry in
`leaner-move/LeanerMove/Intrinsics.lean` checks the 64 roles). This design
gives the roles their verification meaning. The authority for what each role
guarantees is the Prover's `table_module` in
`move-prover/boogie-backend/src/prelude/native.bpl`; where Leaner deviates,
the deviation is named below.

## Equality and order

Equality stays structural everywhere, in code and in specifications, as in
Move. The Prover instead compares intrinsic maps by content (`$IsEqual` over
length, key set, and values), which is coarser than what Move observes for a
map whose physical order is visible (`SimpleMap`: runtime `==`, `keys`,
`values`, `to_vec_pair`). An equality coarser than the observations is not a
congruence, so Leaner does not adopt it. Instead, the model of a map value is
exactly as fine as what Move can observe of it.

The order is `std::cmp::compare`: `RuntimeValue.order`
(`LeanerIR/Semantics/Runtime.lean`), a total order on runtime values whose
laws are proved in `LeanerIR/Proofs/Order.lean` (oriented, transitive, a tie
is equality). The native `cmp::compare` denotes it, as the Prover's per-type
`$1_cmp_$compare` does (integers and addresses numerically, `false < true`,
vectors and fields lexicographically, enum values by variant position, then
fields).

## Model

A map's model is a list of `(key, value)` runtime-value pairs with distinct
keys, under one of two disciplines:

- **Ordered**: keys strictly ascending under `RuntimeValue.order`. The list is
  canonical, so structural equality of models is equality of contents, and
  enumeration (`key_at`, `rank`, front/back, prev/next, `keys`, `values`,
  iteration) is ascending. `set` inserts at the key's rank or replaces in
  place; `del` removes at the rank.
- **Sequence**: insertion order. `set` replaces in place or appends; `del`
  moves the last entry into the removed position (swap-remove), as
  `SimpleMap` does.

The operations and their laws (membership, lookup, length, update, removal,
enumeration, and the bulk operations below) are defined and proved once in
Lean over runtime values, independent of any Aptos type.

## Views

The model of a physical value is read off the owner's declaration shape,
which the registry already constrains:

- **Entries layout**: the owner holds its entries in one vector of two-field
  entry structures (`SimpleMap { data }`, `OrderedMap::SortedVectorMap
  { entries }`). The model is that vector, read as pairs; the discipline is
  ordered when the owner binds an ordering role (`spec_key_at`, `spec_rank`,
  front/back, prev/next), sequence otherwise. The separate insertion-position
  roles (`spec_insertion_key_at`, `spec_insertion_rank`) read the same model
  positions without selecting the ordered discipline.
- **Table layout**: the entries live in native table storage behind a handle
  (`Table`), and composite owners build on tables (`TableWithLength`,
  `SmartTable`, `BigOrderedMap`). Their model needs the table natives
  (`new_table_handle`, `add_box`, `borrow_box`, `remove_box`, …) with a state
  component for table contents (stage 2). None of them has runtime `==` (no
  `drop`); `SmartTable`'s iteration reads its buckets, whose content is
  itself a table model, so its observations stay functions of the model.

## Validity

Distinct keys, and for the ordered discipline ascending keys, is the
representation invariant (`Maps.Valid`). It is an implicit data invariant of
the owner type, deep as the Prover's data-invariant instrumentation is: a
value is valid wherever it appears, in fields (a variant's fields under its
variant), vector elements, and map values. A function assumes it of its
inputs and establishes it of what it returns and modifies, and a loop keeps
it. The laws carry it through every role that produces a map
(`valid_update`, `valid_remove`).

## Roles

Specification roles denote the model operations of the view. Executable roles
are used through contracts generated from the role, not from the body, and
those contracts are trusted like native contracts: hypotheses of the typed
theorems, as the Prover trusts its prelude. The implementations are not
verified against them (the Prover does not verify them either).

| Roles | Aborts | Result and effect |
|---|---|---|
| `new`, `spec_new` | never | empty |
| `new_with_config` | nonzero degree outside `[4, 4096]` (inner) or `[3, 4096]` (leaf) | empty |
| `new_from` | lengths differ; duplicate key | entries from the pairs |
| `len`, `spec_len`, `is_empty`, `spec_is_empty` | never | size; size is 0 |
| `has_key`, `spec_has_key` | never | membership |
| `borrow`, `spec_get`, `get`, `borrow_with_default` | absent key (`borrow`) | value; `Option`; value or default |
| `borrow_mut`, `iter_borrow_mut`, `borrow_mut_with_default` | absent key or end iterator | reference to the value; the default is inserted first |
| `add_no_override` | present key | `set` |
| `add_override_if_exists`, `upsert`, `spec_set` | never | `set`; `upsert` returns the previous value |
| `del_must_exist`, `del_return_key`, `remove_or_none`, `spec_del` | absent key (not `remove_or_none`) | `del`, returning the value (and key) |
| `destroy_empty` | not empty | consumed |
| `add_all`, `upsert_all` | lengths differ; for `add_all` also a present or duplicate key | `set` in order (last write wins) |
| `append`, `append_disjoint` | a shared key (`append_disjoint`) | `set` of every entry of `other` |
| `trim` | `at` beyond size | keeps the first `at` entries, returns the rest |
| `replace_key_inplace` | absent old key; new key out of order | the entry's key replaced at its position |
| `borrow_front`, `borrow_back`, `front_key`, `back_key`, `pop_front`, `pop_back` | empty | smallest or largest entry; `pop_*` removes it |
| `prev_key`, `next_key` | never | the neighbouring key as `Option` |
| `spec_key_at`, `spec_rank` | — | position ↔ key in key order |
| `spec_insertion_key_at`, `spec_insertion_rank` | — | position ↔ key in sequence order |
| `keys`, `values`, `to_vec_pair`, `to_ordered_map` | never | the entries' keys, values, both; the same entries as an ordered map |
| `spec_aborts_*` | — | the abort condition of the executable role |
| `spec_iter_valid`, `spec_leaf_iter_valid`, `spec_iter_preserved`, `spec_leaf_offset` | — | iterator validity versions (stage 3) |

Deviations from the Prover, all where the model determines more than the
Prover states: `values`, `append`, `upsert_all`, `trim`, and
`replace_key_inplace` are exact rather than bounded, because the ordered
model fixes the positions the Prover leaves open; the sequence discipline
keeps `SimpleMap`'s order.

## Proofs

The closer keeps map values abstract: observations (`size`, `hasKey`,
`valueAt`, `keyAt`, `rank`) are reduced by the laws over the operations, and
never by evaluating a model's list, which would lose the laws' patterns. Where
the Prover's SMT solver has map axioms, the closer adds their instances
(`leaner_denote_map_positions`) at the positions a leaf reads a map at,
deciding the positions by omega:

- a key read at its scalar type and written back is the key, for a map whose
  keys are encodings of that type (`KeysRead`), within the map;
- an order on keys is the order on their readings, and a comparison variant
  equated to a variant is the order's relation;
- a key the map has sits at its rank, and a key at a position differs from
  each key the map lacks;
- the keys of a valid ordered map ascend with their positions, and equal
  positions hold equal keys;
- a hypothesis quantified over the positions of a map holds at each position
  the leaf reads.

## Stages

1. `cmp::compare`; the model library with its laws; the entries-layout view;
   validity as a data invariant; specification roles; executable-role
   contracts.
   Gate: `simple_map`, `pool_u64`, `ordered_map`.
2. Table natives and table-layout owners. Gate: `table`,
   `table_with_length`, `smart_table`, `big_vector`, `smart_vector`,
   `storage_slots_allocator`, `pool_u64_unbound`, `big_ordered_map`.
3. Iterator validity versions (`BigOrderedMap` iterators).

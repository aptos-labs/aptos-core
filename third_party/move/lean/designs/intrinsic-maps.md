# Intrinsic maps

Status: 2026-10-13, stage 2 implemented: a table is an owned value of its
handle and contents, read and written through the roles ("Owned Table
values"); the runtime keeps a table's entries in native storage behind its
handle and runs the roles over it, and the agreement reconciles the two
representations ("Tables in the runtime semantics").
Stage 1 (2026-09-30): `ordered_map` verifies (its own iterator tests and a
client module); `simple_map` and `pool_u64` in part.

A Move structure marked `pragma intrinsic = map` binds roles of the Move
Prover's map theory to functions of its module (`vir-design.md`, "Intrinsic
declarations and role graphs"; the Move-profile registry in
`verex-move/VerexMove/Intrinsics.lean` checks the 64 roles). This design
gives the roles their verification meaning. The authority for what each role
guarantees is the Prover's `table_module` in
`move-prover/boogie-backend/src/prelude/native.bpl`; where Verex deviates,
the deviation is named below.

## Equality and order

Code equality follows Move's physical representation. For entries-layout
maps, specification equality stays structural too. The Prover instead
compares intrinsic maps by content (`$IsEqual` over length, key set, and
values), which is coarser than what Move observes for a map whose physical
order is visible (`SimpleMap`: runtime `==`, `keys`, `values`,
`to_vec_pair`). An equality coarser than these observations is not a
congruence, so Verex does not adopt it for entries-layout maps.

For `Table`, specification equality compares allocation identity, represented
by its handle (user decision, 2026-10-06). Distinct executions of `new`
allocate distinct identities, including executions at the same call site;
two empty tables are therefore unequal. Updating contents preserves identity.
The actual Table types have no `drop` ability and do not support runtime
`==`; their specification equality is available independently of that ability.
This identity rule differs from the Prover's content equality.

A table's value carries its contents, so `old(t)` and `t` across a mutation
are two values of one identity with different contents, and the functional
`spec_set` and `spec_del` yield values of the same identity. Identity
equality is therefore a separate comparison (`NTy.identityRuntime`), never
Lean equality of the values, which would equate the contents across the
mutation. A state label binds a memory; a table parameter's contents are its
value's at that label, as every other parameter's are.

The order is `std::cmp::compare`: `RuntimeValue.order`
(`VerexIR/Semantics/Runtime.lean`), a total order on runtime values whose
laws are proved in `VerexIR/Proofs/Order.lean` (oriented, transitive, a tie
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
- **Table layout**: the owner holds a handle (`Table`), or a handle and a
  cached length (the registry's `extensions::table::Table`), or nothing, and
  the runtime keeps its entries in native storage behind the handle. In the
  denotation, the owner's type has one hidden last field holding the entries
  ("Owned Table values" below), so its model is read as an entries layout's,
  under the sequence discipline; the runtime boundary erases the field and
  relates it to native storage. Composite owners build on tables
  (`TableWithLength`, `SmartTable`, `BigOrderedMap`); none has runtime `==`
  (no `drop`). `SmartTable`'s iteration reads its buckets, whose content is
  itself a table model, so its observations stay functions of the model.

### Ownership of table contents

Move checks an entry reference returned by `borrow` or `borrow_mut` as a
borrow of the whole Table argument. This is the ordinary function-call rule:
returned mutable references borrow from the mutable reference arguments,
and returned shared references borrow from the reference arguments. A live
entry borrow therefore prevents a conflicting mutable call on its Table,
including one at a different key. Shared entry borrows may coexist, and
mutable borrows used successively or on distinct owned Tables are accepted.

The Table type has no `copy` ability, its handle field is private, and `new`
allocates the handle. Ordinary clients cannot manufacture another Table value
with the same handle and bypass the borrow relationship. The module and native
implementation remain responsible for unique allocation and not duplicating
ownership. The Aptos table natives use the same implementation for shared and
mutable borrow, retrieving the entry's `GlobalValue` by handle and serialized
key; they rely on Move's static checking rather than a separate lock per entry.

Consequently, the verifier reasons about contents as an owned value of the
Table, even though runtime storage is external. A mutable entry borrow
transfers the value and reconciles its final value into that owner's
contents; it preserves the handle and unrelated entries. Shared observations
are stable for the duration of their certified borrow. This extends the
existing prophetic-reference discipline instead of introducing independent
aliases through the handle. The agreement of the owned value with native
storage is the runtime boundary of "Owned Table values" below; allocation
freshness is the `new` role's contract over the allocation history.

The source of these rules is
`move-bytecode-verifier/src/reference_safety/abstract_state.rs` (`core_call`)
and `move-compiler-v2/src/pipeline/reference_safety/reference_safety_processor_v3.rs`
(`call_operation`). Five focused compiler probes on 2026-10-06 accepted shared,
successive mutable, and distinct-Table borrows, and rejected overlapping mutable
borrows and removal during a shared borrow. The conflicting keys were distinct.

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

### Owned Table values (2026-10-13)

A table is an owned value of its identity and its contents (user decision,
2026-10-07), as every other intrinsic map is a value of its entries. In the
denotation, a table owner's type (`tableOwner`: an intrinsic map owner whose
declared fields are none, a handle, or a handle and a cached length) has one
hidden last field, `NTy.table K V`, whose carrier is the list of its entries
as carrier pairs and whose encoding is a vector of pairs (`structNTyFuel`).
Every operation of the denotation over values, codecs, type substitution,
skolem transport, typing, and data invariants (`DataInvariant.value`'s
`.table` case: every key and value deep), treats the field as any other.
The specification twin of an owner gets the same hidden field
(`SpecTypes.tableContentsField`, `FieldRep.table`), so a specification reads
a table's contents off its value, at the memory of its clause, and `old(t)`
and `t` are two values of one identity.

**The runtime boundary.** The runtime keeps a table's entries outside its
value, in native storage. The boundary erases: `NTy.erase` is the physical
value, the hidden fields dropped, and `NTy.tables` lists the tables a value
holds, each its owner type, handle, and physical entries. A runtime state
holds a memory when its globals encode it erased (`Encodes`) and its native
storage holds the memory's tables (`TablesHeld`: the entries at the slot of
the owner's type and handle, `tableSlot?`, resolved in the declaring
namespace, every entry loan-free; the allocation history at
`TableMemory.allocationResource`); `StorageHolds` is both. The storage may
hold more: the tables that only the frames of a run hold, a local after
`new` or a parameter whose owner is a caller's local, which no global
reaches. A call starts from a state holding the memory and enclosing its
arguments' tables (`Encloses`) and ends in one holding the final memory and
enclosing the results' and the mutated arguments' (`ReturnedEncloses`,
through the returned row); `propheticRun`, closure invocation, and the
runtime form of a contract state these. A state holding a memory
determines it: erasure is injective on the values a memory encloses
(`NTy.eq_of_erase_eq`, `TablesAt`), which `memory_eq_of_encodes` rests on.
The behavioral predicates (`ensures_of`, `aborts_of`, `result_of`, the
labeled state) read a function value's runs from the starts *encoding* the
memory (`StorageEncodes`: held, and nothing else, `TablesEncode.stored`),
which the memory determines in turn, so that runs from one memory differ
only in their loans and the predicates are functions of the memory. The two
start classes meet in `TablesLocal`, below. The role contracts are
hypotheses of the typed theorems, as every native's contract is.

**Roles.** `Maps.Layout` names the owner, the variant, the entry structure
of an entries layout, and whether a table layout holds a handle and a cached
length; `entriesOf` reads the last field of either, and `Layout.build`
rebuilds a value at a handle, with the length when cached. The model
operations (`update`, `remove`, `updateAll`, `takeEntries`, `replaceKey`)
rebuild at the map's own handle (`handleOf`), so they keep its identity
(`NTy.identityRuntime_update` and its siblings). A table owner's model
(`Contract.mapModel?`) has the sequence discipline, and every executable and
specification role is available to it. Allocation (`map_new`) yields a value
at a fresh handle, one the allocation history lacks, and appends it to the
history. The history is a slot no Move specification names, so the frame of
a function that allocates a table, itself or through a callee at any depth
(`Contract.allocatesTables`), leaves that slot open and keeps every other
resource as its `modifies` clauses say; a function that allocates none keeps
the whole memory. A pure `map_spec_new` of a table is rejected: a
specification function has no allocation to take its identity from, and an
identity it would invent cannot be related to the executable one.

**Equality.** A specification's equality between values of a type holding
table contents compares `NTy.identityRuntime` of both operands
(`Contract.identityOperands`): a table owner with a handle is its handle (the
cached length, which an insertion changes, is not part of the identity), a
handle-less owner is structural, and the identity recurses through fields,
elements, entries, and references. Lean equality of the values stays
available to the denotation, where two values of one identity differ by
their contents, as `old(t)` and `t` across a mutation do.

**Behavioral predicates and named calls** lend a value's encoding to a
callee's meaning; a value holding tables has no encoding without its
storage, so `lendsEncoded` (reference-free and table-free) gates them, as
`refFree` alone did, and `lentFlat` likewise.

**Tables in the runtime semantics (decision 2026-10-13, revised the same
day; implemented 2026-10-13).** The verifier represents a table as a value,
as above, and reads it as any other intrinsic map (`mapModel?`: the table
layout under the sequence discipline, every role available; what is
particular to tables is the identity equality and the allocation history,
both decided, and the `lendsEncoded` gate of the behavioral predicates,
which lend encodings and a table-holding value has none without its
storage). The runtime and the big-step semantics keep the representation
the VM has: the physical value is the handle (and a cached length), the
entries live in native storage at the slot of the owner's type and handle
(`NativeTableStorage`), an entry borrow is a loan into that slot
(`LoanTarget.table`), and the allocation history outlives every table. A
value representation is not adequate there: a table in a resource under a
mutable global borrow would carry its entries inside a hole-bearing value,
an entry borrow would be a loan into a nested value, a fresh handle needs a
history no value holds, and a table behind a type parameter would give a
generic function's physical values a shape that depends on the
instantiation. The two representations are reconciled by the agreement
(`compileFunction_agrees`), whose runtime side is `propheticMeaning`: the
erasing boundary above is the statement of that reconciliation and stays.
The earlier decision of 2026-10-13 to represent tables as values in the
runtime as well and to remove the boundary is withdrawn.

*The roles' runtime semantics* (`Semantics/TableRoles.lean`). A function
bound to an executable role of a table owner with a handle (`tableRole?`)
is provided by the runtime, not run from its body, as the verifier reads it
through the role's contract: `BigStep.runtimeProvided` names it beside the
bodiless natives, `BigStep.nativeCall` runs it by `tableRoleCall` and every
other native by the opaque `hostCall`, and the interpreter follows. The
twelve roles the production `table` and `table_with_length` bind have a run:
`new` allocates at the first handle `table-n` the history lacks (read from
the history alone, so that runs from storages differing in tables they do
not reach allocate alike), `destroy_empty`, `len`, `is_empty`, `has_key`,
`borrow`, `borrow_with_default`, `borrow_mut` (an entry loan,
`borrowTableEntry?`), `borrow_mut_with_default`, `add_no_override`,
`add_override_if_exists`, and `del_must_exist`; a present or absent key
aborts with the natives' codes (100, 101), a nonempty table on destruction
with 102. A mutable map argument is exported at exit as a body exports its
mutable parameters, its cached length changed where the size changes. The
owner's closed type is the role's parameter type (its result type for
`new`) at the frame's instantiation, canonicalized to the first identifier
denoting it (`canonicalTypeId?`), the slot the boundary names; a role at an
open owner type, or one the runtime does not provide (`upsert`,
`remove_or_none`, enumeration), has no run. The owner's layout is one
notion (`tableLayout?`); the hidden contents field keeps its own kernel-evaluated reading (`tableOwner`), unchanged.

*The boundary* was found, on revisiting, to admit no table only a frame
value holds: `TablesEncode.stored` required every slot of native storage to
hold a table some global holds, so no state enclosed a local after `new` or
a parameter whose owner is a caller's local, the runtime form of a contract
(`Contract.prophetic`) had a false precondition at such calls, and
`GlobalsPreserved` failed for a run that allocates. Callers reason through
contracts (`wp_call`), so nothing was unsound; the runtime-form theorems
covered only runs whose argument tables were stored in globals. The call
boundary now holds the memory (`StorageHolds`, `Encloses`) and the storage
may hold more; the behavioral predicates keep the encoding starts
(`StorageEncodes`), which a memory determines. What joins them is
`TablesLocal`, a property of the unit's runs stated as a hypothesis of the
public theorem beside `GlobalsPreserved`: a run from a start holding a
memory is also a run from the start encoding it, with the same outcome and
loan bookkeeping, whose exit encodes every memory the original exit holds,
since a run reads no table its globals and arguments do not reach. Only a
theorem reading a named run takes it (`Contract.assumesLocality`: a
behavioral predicate in its contract, invariants or body, itself or through
verified callees, or an invocation of a function value it does not see);
every other theorem is as it was, at its cost. The solver takes it from
the theorem's context at every named run (`wp_named`, `wp_named_mut`, their
typed forms, the unseen invocations, `returns_of_terminating`). Rejected (2026-10-13): making the native storage
a ghost slot of the memory, which keeps the boundary exact but leaks a
runtime artifact into every contract's frame.

Open:

- Proving `TablesLocal`, by induction on the runs as the loan renaming
  (`LoanIndependence`) is, with `NativesShift` and `NativesTyped` for the
  roles' runs, which the primitives' mirror lemmas
  (`Semantics/TableOperations.lean`) prepare; and `GlobalsPreserved`.
- A role at a frame reads the owner's closed type from the type table; a
  call whose instantiation the table lacks (`RequiredTypes` interns the
  types a body consults, not a role's owner) has no run.
- Tables behind type parameters (nested tables, roles at table-bearing
  instances) remain not carried; the MonoVM differential tests run no
  table program yet.

**Fixtures.** `Check/Storage/TableReads.lean` (membership, shared lookup,
a generic lookup, an opaque caller's `old`, an aggregate payload, and two
rejected claims), `Check/Specifications/TableSnapshots.lean` (mutable and
nested inputs, labels in an opaque callee and its caller, functional
snapshots and identity equality), `Check/Storage/TableStoredInvariants.lean`
(deep data invariants of stored values at resolved type arguments, both
handle layouts, two rejected claims), `Check/Storage/TableStorageKeys.lean`
(one slot per owner type and handle across namespaces, none for a shared
reference), `Check/Storage/TableAllocation.lean` (fresh handles, the
allocation history in callers' frames), `Check/Storage/TableGlobalWrites.lean`
(data invariants of entries written into a stored table), and `VerexSL/Tests/StoredTableInvariantErrors.lean` (an
invariant over two tables' identities, and a rejected one over their
lengths).

The solver keeps map values abstract: observations (`size`, `hasKey`,
`valueAt`, `keyAt`, `rank`) are reduced by the laws over the operations, and
never by evaluating a model's list, which would lose the laws' patterns. Where
the Prover's SMT solver has map axioms, the solver adds their instances
(`verex_map_positions`) at the positions a leaf reads a map at,
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
2. Table owners as owned values: the hidden contents field, the erasing
   runtime boundary, all roles for table layouts, identity equality
   (2026-10-13, above); the roles' runtime semantics and the boundary
   holding tables of other frames (2026-10-13, "Tables in the runtime
   semantics"). Gate: `verify_table`, `bitwise_table`,
   `table_option`, `data_invariant_in_map`, `table_contais_to_length`;
   then `table`, `table_with_length`, `smart_table`, `big_vector`,
   `smart_vector`, `storage_slots_allocator`, `pool_u64_unbound`,
   `big_ordered_map`, which wait on tables behind type parameters.
3. Iterator validity versions (`BigOrderedMap` iterators).

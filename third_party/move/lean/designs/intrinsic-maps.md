# Intrinsic maps

Verification uses an abstract map value for every intrinsic map type.
The runtime executes concrete Move code, with native table roles operating on
handle storage; the agreement relates these representations. A0–A3 are
implemented and A4 remains open ("Abstract representation"). `SmartTable`
is deprecated and excluded from the gates.

A Move structure marked `pragma intrinsic = map` binds roles of the Move
Prover's map theory to functions of its module (`vir-design.md`, "Intrinsic
declarations and role graphs"; the Move-profile registry in
`verex-move/VerexMove/Intrinsics.lean` checks the 65 roles). This design
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

The abstract representation below replaces these views: A1 removes the
entries layout, and the table layout becomes every owner's. Until then the
model of a physical value is read off the owner's declaration shape, which
the registry already constrains:

- **Entries layout**: the owner holds its entries in one vector of two-field
  entry structures (`SimpleMap { data }`, `OrderedMap::SortedVectorMap
  { entries }`). The model is that vector, read as pairs; the discipline is
  ordered when the owner binds an ordering role (`spec_key_at`, `spec_rank`,
  front/back, prev/next), sequence otherwise. The separate insertion-position
  roles (`spec_insertion_key_at`, `spec_insertion_rank`) read the same model
  positions without selecting the ordered discipline.
- **Table layout**: the owner holds a handle (`Table`), or a handle and a
  cached length (the registry's `extensions::table::Table`), or nothing (an
  abstract map of the Prover's tests, whose one field is then the Boolean
  placeholder `dummy_field` Move's compilers give an empty struct, kept as a
  table keeps its handle, though it is no table: it has no handle, so its
  identity is its entries and its pure `spec_new` is the empty map). The
  owner's value holds its entries in one hidden last field, in the denotation
  and at runtime ("Owned Table values" below), so its model is read as an
  entries layout's, under the sequence discipline, or the ordered one where
  the owner binds an ordering role.
None of these owners has runtime `==` (no `drop`).

## Abstract representation (decision 2026-10-17)

In verification every intrinsic map type has one representation: an
abstract map value, as the Prover replaces the type by its map theory. At
runtime the type keeps its Move representation and its Move bodies run; the
two are related in the agreement, not in the solver's terms. `SmartTable` is
deprecated and gets no treatment at all (user, 2026-10-17): its
specification is skipped, its gate dropped.

- **Denotation.** An owner's native type is an abstract nominal
  (`NTy.map owner K V`): its carrier is the map model (entries under the
  owner's discipline, the allocation handle where its Move type holds table
  storage, its validity slots), and its declared fields are no part of it.
  The entries are as many as a Move vector holds (`SpecEntries`, bounded by
  2^64 as `SpecVector` is): the representation keeps them in vectors, and a
  `u64` count of them never overflows, which the physical entries vector
  gave before and the abstract map inherits (2026-10-18).
  Roles denote the model operations; every other function of the module
  (iterators, `SimpleMap`'s `new`, `new_from`, `add_all`, `upsert`,
  `keys`, `values`, `to_vec_pair`, the configuring constructors,
  `allocate_spare_slots`) is read through its specification, trusted as the
  Prover trusts an `opaque` or `verify = false` function; it needs no role
  binding. Iterator types stay as declared; the fields their specifications
  do not read (`node_index`, `child_iter`) stay unconstrained. The unit's
  value ranks are a named constant, not a literal.
- **Runtime.** Values are the Move structures, bodies run, as the VM runs
  them; the interpreter and the MonoVM differential tests exercise the
  real code.
- **Agreement.** The runtime's values are read up to a normalization
  (`normalize unit : RuntimeValue → RuntimeValue`): every well-formed value
  of an owner type, wherever it occurs (a field, an element, a stored
  resource, a closure's capture, the referent of a borrow), is replaced by
  its view, the canonical value of the abstract layout (the held table for
  `TableWithLength`, the sorted entries for `OrderedMap`, the leaves for
  `BigOrderedMap`, the entry vector for `SimpleMap`, a table as it is), its
  ghost slots and, where the type holds table storage, its handle carried
  along; an ill-formed owner value is left as it is. The normalization
  lives in the runtime meaning of a function, `functionSpecAt`
  (`Proofs/Meaning.lean`): the quotient of big-step evaluation, which
  succeeds from a normalized start on normalized arguments with normalized
  results and exit exactly when some representation of them runs; the
  prophetic run, the codecs, and the invocation and closure bridges keep
  their equations over this quotient unchanged. A callee thus receives any
  representation of its abstract arguments and may return any
  representation of its abstract result. Two assumptions, stated in the ledger and in the
  artifact audit's approved list, carry the trust the Prover places in its
  prelude: each role's body and each specified function's body refines its
  contract up to normalization and preserves well-formedness, and a write
  through an entry reference (`borrow_mut`, `iter_borrow_mut`) refines the
  abstract update of that entry. The agreement theorem is an axiom today;
  its statement gains the normalization, its status does not change.
- **Discipline** (ordering revisited with the user, 2026-10-17): ordered,
  canonical, for `OrderedMap` and `BigOrderedMap`; sequence for `SimpleMap`
  (its swap-remove and append are exact), `Table`, and `TableWithLength`.
  With `SmartTable` gone, no owner's code lacks the order its model fixes.
  Open: the deviations where the model is exact and the Prover bounded
  (`values`, `append`, `upsert_all`, `trim`, `replace_key_inplace`) are
  re-checked against each body before its refinement is assumed.
- **Identity:** by handle where the Move type holds table storage, by
  entries otherwise (the rule of 2026-10-06). Specification equality of a
  type without runtime `==` is content equality, as the Prover's; nothing
  in code observes the difference.
- **Construction.** An owner is constructed and destructured only in its
  module: a VerexSL source or an inlined body doing so elsewhere is
  rejected, so well-formedness is the module's alone to keep. Functions
  reading the representation with no specification (`validate_map`,
  `print_map`, `destroy_and_validate`, `internal_leaf_*`,
  `iter_with_path_*`) are rejected where a target reaches them.

Stages:

- A0 (done 2026-10-17). The normalization: one object-level layout per
  intrinsic owner (`Validation/MapOwners.lean`, `ownerLayout?` generalizing
  `tableLayout?`; `abstractLayout?` selects the owners whose layout is
  abstract, the tables alone for now), the views as functions of runtime
  values and `normalize` over values, stores, states, and outcomes
  (`Semantics/MapViews.lean`), and `functionSpecAt` as the quotient. The
  determinism and typing theorems of `Proofs/Invocation.lean` read the
  quotient through `functionSpecAt_ok_iff`/`functionSpecAt_aborts_iff`,
  which hold because `normalize` is the identity while only tables are
  abstract (`normalize_id`); A1 replaces them by the determinism of the
  quotient up to normalization. Gate: every suite green with the
  denotation unchanged.
- A1 (done 2026-10-17). Every owner's native type is its abstract map
  (`structNTy`: the leading fields of its canonical layout, its ghost
  slots, and the hidden contents field; its declared fields and variants
  are no part of it), the specification twin likewise, and every role's
  contract is stated over that value at the canonical layout
  (`Contract.mapModel?` from `ownerLayout?`); the entries layout and the
  composite views (`Maps.held`, `Maps.bucketed`, `Maps.leaves`) are gone.
  Typing: `TypedAs.abstract` reads an owner's abstract value at its
  canonical field types; the runtime's `HasType` keeps the declared shape.
  Two assumptions join the natives' as hypotheses of every theorem that
  assumes typing (`RunsAgree`, `OwnersRepresentable`; `designs/denotation.md`,
  "Assumption ledger"). Gates: the table, `TableWithLength`, `SimpleMap`,
  and `OrderedMap` fixtures and Prover tests; `DenotePerformance`.
- A2 (done 2026-10-17). A function whose body constructs or destructures
  an owner, selects, tests, or updates its declared fields or variants, or
  projects a place through them (`Contract.representationRead?`) has no
  denotation: it is used through its specification (`usedThroughContract`),
  its body is never a target, `verify` of it is refused, and a target
  reaching one without a specification is rejected. Only the owner's module
  reads its representation, by Move's visibility, which the frontend
  enforces. Iterator types stay as declared. The framework's `ordered_map` and
  `big_ordered_map` specifications already set `verify = false` on such
  functions, as the Prover requires. Gate: `AbstractOwners` fixture;
  `ordered_map`'s use-case tests unchanged in outcome (`perf-notes.md`).
- A3 (done 2026-10-17 with A1). The refinement assumptions are the
  hypotheses `RunsAgree` and `OwnersRepresentable` of every theorem that
  assumes typing, stated in the ledger (`denotation.md`, "Assumption
  ledger") and accepted by the artifact audit; a write through an entry
  reference (`borrow_mut`) is a run the quotient reads up to normalization,
  its refinement of the abstract update part of `RunsAgree`. Open: both
  hypotheses discharged by a proof for a unit whose owners are all tables,
  where normalization is the identity (`roadmap.md`).
- A4. `big_ordered_map`'s use-case tests, their cost against today's
  (`perf-notes.md`), and the construction and representation-reading
  diagnostics.

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

Consequently, a table is an owned value of the Table, in the verifier and
in the runtime semantics alike, though the VM keeps its entries in external
storage. A mutable entry borrow transfers the value and reconciles its final
value into that owner's contents; it preserves the handle and unrelated
entries. Shared observations are stable for the duration of their certified
borrow. This extends the existing prophetic-reference discipline instead of
introducing independent aliases through the handle. Allocation freshness is
the `new` role's contract over the allocation history.

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
(`valid_update`, `valid_remove`, `valid_updateAll`, `valid_takeEntries`,
`valid_dropEntries`, and `valid_replaceKey` where the replacement does not
abort).

## Roles

Specification roles denote the model operations of the view. Executable roles
are used through contracts generated from the role, not from the body, and
those contracts are trusted like native contracts: hypotheses of the typed
theorems, as the Prover trusts its prelude. The implementations are not
verified against them (the Prover does not verify them either). The runtime
provides a table owner's roles by the same model operations ("Tables in the
runtime semantics").

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
| `destroy_unchecked` | never | consumed, without checking emptiness |
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
| `spec_iter_valid`, `spec_leaf_iter_valid`, `spec_iter_preserved` | — | iterator validity: slot equality (stage 3, `ghost-fields.md`); `spec_leaf_offset` uninterpreted |

Optional role results use the representation of their declared return type:
a one-vector-field struct, or an enum with an empty variant and a
single-field variant. Runtime execution and generated contracts share the
layout classifier. Assuming the legacy vector wrapper for an enum gives an
impossible result equation, so a caller's proof can become vacuous. The
`OptionalMapResults` Check fixture verifies tags and values and executes
missing, present and replaced results; it also covers the legacy wrapper.


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

**The runtime.** The runtime holds the same value ("Tables in the runtime
semantics" below): the stores encode a memory by its values' encodings
(`Encodes`), a table's entries included, and the handles allocated so far
are the memory's allocation history (`StorageEncodes`, at
`TableMemory.allocationResource`). The role contracts are hypotheses of the
typed theorems, as every native's contract is.

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
the whole memory. A pure `map_spec_new` of a table with a handle is
rejected: a specification function has no allocation to take its identity
from, and an identity it would invent cannot be related to the executable
one; a placeholder layout has no handle, and its `map_spec_new` is the empty
map.

**Laws of the bulk roles (2026-10-13).** The solver reads the bulk
operations as the single ones: membership after `updateAll` is membership
in the keys set or in the map (`hasKey_updateAll_add_all`, `_upsert_all`,
`_new_from`, each under the role's abort condition, which the leaf holds
negated; `hasKey_updateAll_append` for an appended map); the size of an
appended or bulk-updated map lies between the map's and the sum of both
(`size_updateAll_bounds`, `size_updateAll_append_bounds`, the appended
map's size as a lower bound where the appended map is valid), as bounds the
leaf asserts of every `size` of such a map; an exact size where every key
is fresh (`size_updateAll_add_all`, `_new_from`, `_append_disjoint`).
`takeEntries` and `dropEntries` split a map at a position: an entry is in
the prefix exactly when its rank lies below the count (`hasKey_takeEntries`,
`hasKey_dropEntries`), with the sizes `min` and the difference; `replaceKey`
keeps the size and moves one key (`hasKey_replaceKey`). The keys of a built
map are read back as the keys it was built from (`keysRead_updateAll` and
its siblings), so the position view (`spec_key_at`, `spec_rank`) holds of it
as of its source; the solver states of every position such a map is read
at that the key there is one of its keys, at that position
(`mapPositionFacts`), ahead of the membership laws, which would otherwise
rewrite the membership of that key into its sources and lose the position.
The abort conditions of the bulk roles at literal vectors evaluate
(`AbortsNewFrom_toArray` and its siblings): a symbolic vector is a mapped
array, which they leave to the role's hypothesis.

**Equality.** A specification's equality between values of a type holding
table contents compares `NTy.identityRuntime` of both operands
(`Contract.identityOperands`): a table owner with a handle is its handle (the
cached length, which an insertion changes, is not part of the identity), a
handle-less owner is structural, and the identity recurses through fields,
elements, entries, and references. Lean equality of the values stays
available to the denotation, where two values of one identity differ by
their contents, as `old(t)` and `t` across a mutation do. Code equality
(`==`, which no actual table type admits) of a placeholder-layout value
compares its hidden contents structurally (`NTy.eqb_table_decide`, as
vectors compare), so a map equals itself and two maps holding the same
entries in different orders do not.

**Tables in the runtime semantics (decision 2026-10-13).** A table is a
value in the runtime and the big-step semantics too, as in the Move Prover's
map model. A table owner's runtime value holds its declared fields and its
entries in its last field, the encoding of the denotation's value, so the
runtime keeps no table storage beside its values; the handles allocated so
far are a field of the runtime state (`RuntimeState.tableHandles`), which
outlives every table. A function bound to an executable role of a table
owner (`tableRole?`) is provided by the runtime, not run from its body
(`BigStep.runtimeProvided`): the role's meaning on the owner's value by the
map model (`Semantics/TableRoles.lean`, `tableRoleCall`), the model
operations the role's contract states (`Maps.update`, `Maps.remove`,
`Maps.updateAll`, …) under the owner's discipline (`TableLayout.discipline`).
A role reads no loan: it runs on arguments whose values, a mutable
reference's current value among them, hold none, as a well-typed call's do,
and has no run otherwise, so that no run observes a loan's identity through
the order of keys (decided 2026-10-09). The role's step on those values
(`roleStep`) is apart from the state plumbing: `new` of an owner with a
handle takes the first handle `table-n` no table had and records it. A mutable entry reference (`borrow_mut`,
`borrow_mut_with_default`) is a loan into the owner's value: the role
leaves the loan's hole at the entry in the map it exports, as a body that
reborrows its parameter leaves it, so the entry's final value returns into
the table when the loan dies. A present or absent key aborts with the
natives' encoded codes (25607 and 25863). Checked destruction of a nonempty
table uses 196710, the `error::invalid_state(102)` of
`table_with_length::destroy_empty`; unchecked destruction returns normally.
The source declaration binds unchecked destruction to `map_destroy_unchecked`;
the Move Prover and Verex both interpret that role without inspecting the
bound function's name. Bulk length
mismatches use 103; duplicate-key rejection uses 25607. An owner has no constructor at
runtime (`constructorFields?`): only its roles make one, so a `pack` or
`unpack` of an owner in a body is stuck. Runtime typing types an owner's
value with its entries (`contentsTypes`): its declared fields, then a vector
of pairs of its key and value type arguments.

A value holding tables lends as its encoding, so behavioral predicates and
named calls read such values, closures capture them, and a type argument may
hold a table: tables behind type parameters are carried (a table of tables,
an `Option` of a table, a role at a table-bearing instance). The earlier
revision of the same day kept the VM's representation at runtime, entries in
native storage behind the handle, reconciled with the denotation's values by
an erasing boundary. It is withdrawn: with native storage a run depends on
storage outside the memory, through a closure capturing a table or an
argument holding one, so the behavioral predicates could not be functions
of the memory, and the property joining the two (`TablesLocal`) was false
for any unit with a table role.

Open:

- `NativesTyped` quantifies over `nativeCall`, the table roles included: to
  be stated of the host's natives and proved of the roles, as `NativesShift`
  is (`Semantics/TableRoleShifts.lean`: a role's step on loan-free values
  yields loan-free values, and its run mirrors onto renamed arguments,
  `tableRoleCall_shifts`). Proving it needs the role functions' signatures,
  which the Move profile's schema checks, in a form the proof reads.
- The role contracts are hypotheses of the typed theorems; for a table
  layout the runtime provides them by the model the contracts state, so they
  are to be proved.
- `ValueHasType`, the M1 judgment over type identifiers, types no owner's
  entries.
- Specification equality of values at a type parameter is structural in a
  generic function's reading, also where an instantiation makes them tables,
  which the Prover, monomorphizing, compares by allocation identity.

**Fixtures.** `Check/Storage/TableReads.lean` (membership, shared lookup,
a generic lookup, an opaque caller's `old`, an aggregate payload, a variant's
field of a read value, and two rejected claims), `Check/Specifications/TableSnapshots.lean` (mutable and
nested inputs, labels in an opaque callee and its caller, functional
snapshots and identity equality), `Check/Storage/TableStoredInvariants.lean`
(deep data invariants of stored values at resolved type arguments, both
handle layouts, two rejected claims), `Check/Storage/TableAllocation.lean`
(fresh handles, the allocation history in callers' frames, and the roles
run on values by the interpreter), `Check/Storage/TableGlobalWrites.lean`
(data invariants of entries written into a stored table),
`Check/Storage/WrappedTables.lean` (a `TableWithLength` read through the
table it holds: its roles' contracts, a functional update, a rejected size
claim, and its role bodies run by the interpreter), and
`VerexSL/Tests/StoredTableInvariantErrors.lean` (an
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

## Typed maps in authored proofs (2026-10-19)

`VerexIR/Proofs/Maps/Model.lean` provides `SpecMap K V`, its operations and
laws without runtime values. `Maps/Encoding.lean` proves the transport to the
runtime map representation. `Denote/TypedMaps.lean` applies that transport at
the authored-proof boundary: `verex_prepare typed` presents integer keys and
values as `Int`, and aggregate values in their native carrier. Optional keys
become `Option Int`, and key-vector facts use the same typed map model.

Use `verex_prepare values` to normalize the source values first, then
`verex_lift_maps only [fact₁, fact₂]` after specializing the relevant callee
facts. Only the goal and selected facts are lifted; unrelated maps stay in
their original form. The representation equalities remain hidden proof facts,
so transport can be delayed without losing its justification. This separation
avoids repeatedly translating every quantified callee summary.

OrderedMap's companion has no local helper theorems or explicit `RuntimeValue`
references. BigOrderedMap also has no local helper theorems; it uses typed map
laws for insertion, removal, key vectors, successor/predecessor keys, and leaf
slice validity. Some of its remaining scripts still use runtime-map
observations, particularly aggregate leaf values and concrete iterator walks.
The general ghost-field selection law lives with contract translation, not in
either companion. This is an authored-proof boundary cleanup; the runtime
agreement and role contracts continue to use the encoding model.

## Stages

1. `cmp::compare`; the model library with its laws; the entries-layout view;
   validity as a data invariant; specification roles; executable-role
   contracts.
   Gate: `simple_map`, `pool_u64`, `ordered_map`.
2. Table owners as owned values: the hidden contents field, all roles for
   table layouts, identity equality (2026-10-13, above); tables as values in
   the runtime semantics, the roles provided on them by the model
   (2026-10-13, "Tables in the runtime semantics"); wrapped, bucketed,
   and tree layouts (2026-10-14). Gate: `verify_table`, `bitwise_table`,
   `table_option`, `data_invariant_in_map`, `table_contais_to_length`,
   `table`, `table_with_length`, `smart_table`, `pool_u64_unbound` (at five
   times the default budget); then `big_vector`, `smart_vector`,
   `storage_slots_allocator`, `big_ordered_map`.
3. Iterator validity versions: the validity slots of `ghost-fields.md`
   ("Validity slots", 2026-10-15), with keyed and positional iterators and
   the role natives' declared preconditions. Gate: `verify_iterator_validity`
   (the Prover's thirteen failures), `ghost_field_intrinsic_map_ops`;
   `big_ordered_map`'s iterator targets.

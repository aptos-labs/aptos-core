# Intrinsic maps

Verification uses an abstract map value for every intrinsic map type. The
runtime executes the Move code, and the agreement relates the two
representations. `SmartTable` is deprecated and gets no treatment.

A Move structure marked `pragma intrinsic = map` binds roles of the Move
Prover's map theory to functions of its module ([`vir-design.md`](vir-design.md),
"Intrinsic declarations and role graphs"; the Move-profile registry in
`verex-move/VerexMove/Intrinsics.lean` checks the 65 roles). This design
gives the roles their verification meaning. The authority for what each
role guarantees is the Prover's `table_module` in
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

For `Table`, specification equality compares allocation identity,
represented by its handle. Distinct executions of `new` allocate distinct
identities, including executions at the same call site; two empty tables
are therefore unequal. Updating contents preserves identity. The actual
Table types have no `drop` ability and no runtime `==`; their specification
equality is available independently of that ability. This identity rule
differs from the Prover's content equality.

A table's value carries its contents, so `old(t)` and `t` across a
mutation are two values of one identity with different contents, and the
functional `spec_set` and `spec_del` yield values of the same identity.
Identity equality is therefore a separate comparison
(`NTy.identityRuntime`), never Lean equality of the values: a table owner
with a handle is its handle (a cached length is not part of the identity),
a handle-less owner is structural, and the identity recurses through
fields, elements, entries, and references. A state label binds a memory; a
table parameter's contents are its value's at that label.

The order is `std::cmp::compare`: a total order on values (`NTy.order` in
verification, `RuntimeValue.order` at runtime, related by one theorem),
oriented, transitive, a tie being equality. The native `cmp::compare`
denotes it, as the Prover's per-type `$1_cmp_$compare` does (integers and
addresses numerically, `false < true`, vectors and fields
lexicographically, enum values by variant position, then fields).

## Model

A map's model is a list of `(key, value)` pairs with distinct keys, under
one of two disciplines:

- **Ordered**: keys strictly ascending. The list is canonical, so
  structural equality of models is equality of contents, and enumeration
  (`key_at`, `rank`, front/back, prev/next, `keys`, `values`, iteration)
  is ascending. `set` inserts at the key's rank or replaces in place;
  `del` removes at the rank. `OrderedMap` and `BigOrderedMap`.
- **Sequence**: insertion order. `set` replaces in place or appends; `del`
  moves the last entry into the removed position (swap-remove), as
  `SimpleMap` does. `SimpleMap`, `Table`, `TableWithLength`.

The operations and their lemmas (membership, lookup, length, update,
removal, enumeration, and the bulk operations below) are defined and
proved once in Lean (`VerexIR/Proofs/Maps/Model.lean`, `SpecMap K V`),
independent of any Aptos type. The entries are as many as a Move vector
holds (`SpecEntries`, bounded by 2^64 as `SpecVector` is), so a `u64`
count of them never overflows.

## Abstract representation

In verification every intrinsic map type has one representation, an
abstract map value, as the Prover replaces the type by its map theory. At
runtime the type keeps its Move representation and its Move bodies run;
the two are related in the agreement, not in the solver's terms.

- **Denotation.** An owner's native type is an abstract nominal
  (`NTy.map owner K V`): its carrier is the map model (entries under the
  owner's discipline, the allocation handle where its Move type holds
  table storage, its validity slots), and its declared fields are no part
  of it. Roles denote the model operations; every other function of the
  module (iterators, `SimpleMap`'s `new`, `new_from`, `add_all`, `upsert`,
  `keys`, `values`, `to_vec_pair`, the configuring constructors,
  `allocate_spare_slots`) is read through its specification, trusted as the
  Prover trusts an `opaque` or `verify = false` function; it needs no role
  binding. Iterator types stay as declared; the fields their
  specifications do not read stay unconstrained. The unit's value ranks
  are a named constant.
- **Layouts.** The physical value an owner's declaration shape gives is
  read as its model by the normalization: an *entries layout* holds its
  entries in one vector of two-field entry structures (`SimpleMap { data
  }`, `OrderedMap::SortedVectorMap { entries }`); a *table layout* holds a
  handle, or a handle and a cached length, or nothing (the Prover's
  abstract maps, whose one field is Move's `dummy_field` placeholder: no
  handle, so its identity is its entries and its `spec_new` is the empty
  map). The discipline is ordered when the owner binds an ordering role
  (`spec_key_at`, `spec_rank`, front/back, prev/next), sequence otherwise;
  the insertion-position roles (`spec_insertion_key_at`,
  `spec_insertion_rank`) read positions without selecting the ordered
  discipline.
- **Runtime.** Values are the Move structures, bodies run, as the VM runs
  them; the interpreter and the MonoVM differential tests exercise the
  real code.
- **Agreement.** The runtime's values are read up to a normalization
  (`normalize unit : RuntimeValue → RuntimeValue`): every well-formed value
  of an owner type, wherever it occurs, is replaced by its view, the
  canonical value of the abstract layout (the held table for
  `TableWithLength`, the sorted entries for `OrderedMap`, the leaves for
  `BigOrderedMap`, the entry vector for `SimpleMap`, a table as it is),
  its ghost slots and handle carried along; an ill-formed owner value is
  left as it is. The normalization lives in the runtime meaning of a
  function, `functionSpecAt` (`Proofs/Meaning.lean`): the quotient of
  big-step evaluation, which succeeds from a normalized start on
  normalized arguments with normalized results exactly when some
  representation of them runs. A callee thus receives any representation
  of its abstract arguments and may return any representation of its
  abstract result. Two assumptions, `RunsAgree` and `OwnersRepresentable`,
  stated in the assumption ledger ([`denotation.md`](denotation.md)) and in
  the artifact audit's approved list, carry the trust the Prover places in
  its prelude: each role's body and each specified function's body refines
  its contract up to normalization and preserves well-formedness, and a
  write through an entry reference (`borrow_mut`, `iter_borrow_mut`)
  refines the abstract update of that entry.
- **Identity:** by handle where the Move type holds table storage, by
  entries otherwise. Specification equality of a type without runtime `==`
  is content equality, as the Prover's; nothing in code observes the
  difference.
- **Construction.** An owner is constructed and destructured only in its
  module: a function whose body constructs or destructures an owner,
  selects, tests, or updates its declared fields or variants, or projects
  a place through them (`Contract.representationRead?`) has no denotation.
  It is used through its specification (`usedThroughContract`), its body is
  never a target, `verify` of it is refused, and a target reaching one
  without a specification is rejected. Only the owner's module reads its
  representation, by Move's visibility, which the frontend enforces.

### Ownership of table contents

Move checks an entry reference returned by `borrow` or `borrow_mut` as a
borrow of the whole Table argument, the ordinary function-call rule:
returned mutable references borrow from the mutable reference arguments,
and returned shared references borrow from the reference arguments. A
live entry borrow therefore prevents a conflicting mutable call on its
Table, including one at a different key. Shared entry borrows may coexist,
and mutable borrows used successively or on distinct owned Tables are
accepted. The Table type has no `copy` ability, its handle field is
private, and `new` allocates the handle, so no client can manufacture
another Table value with the same handle and bypass the borrow
relationship. The source of these rules is
`move-bytecode-verifier/src/reference_safety/abstract_state.rs`
(`core_call`) and
`move-compiler-v2/src/pipeline/reference_safety/reference_safety_processor_v3.rs`
(`call_operation`).

Consequently, a table is an owned value of its identity and its contents,
in the verifier and in the runtime semantics alike, though the VM keeps
its entries in external storage. A mutable entry borrow transfers the
value and reconciles its final value into the owner's contents; it
preserves the handle and unrelated entries. Shared observations are stable
for the duration of their certified borrow. This extends the prophetic
reference discipline instead of introducing independent aliases through
the handle. Allocation freshness is the `new` role's contract over the
allocation history: a slot no Move specification names, which the frame
of a function that allocates a table, itself or through a callee at any
depth (`Contract.allocatesTables`), leaves open while keeping every other
resource as its `modifies` clauses say. A pure `map_spec_new` of a table
with a handle is rejected: a specification function has no allocation to
take its identity from; a placeholder layout has no handle, and its
`map_spec_new` is the empty map.

### Tables in the runtime semantics

A table is a value in the runtime and the big-step semantics too, as in
the Prover's map model. A table owner's runtime value holds its declared
fields and its entries in its last field, so the runtime keeps no table
storage beside its values; the handles allocated so far are a field of the
runtime state (`RuntimeState.tableHandles`), which outlives every table. A
function bound to an executable role of a table owner (`tableRole?`) is
provided by the runtime, not run from its body (`BigStep.runtimeProvided`):
the role's meaning on the owner's value by the map model
(`Semantics/TableRoles.lean`, `tableRoleCall`) under the owner's
discipline. A role reads no loan: it runs on arguments whose values hold
none, as a well-typed call's do, and has no run otherwise, so that no run
observes a loan's identity through the order of keys. `new` of an owner
with a handle takes the first handle `table-n` no table had and records
it. A mutable entry reference (`borrow_mut`, `borrow_mut_with_default`) is
a loan into the owner's value: the role leaves the loan's hole at the entry
in the map it exports, so the entry's final value returns into the table
when the loan dies. A present or absent key aborts with the natives'
codes (25607 and 25863); checked destruction of a nonempty table uses
196710; bulk length mismatches 103; duplicate keys 25607. An owner has no
constructor at runtime: only its roles make one, so a `pack` or `unpack`
of an owner in a body is stuck. Runtime typing types an owner's value with
its entries (`contentsTypes`).

A value holding tables lends as its encoding, so behavioral predicates and
named calls read such values, closures capture them, and a type argument
may hold a table. Keeping the VM's representation at runtime, entries in
native storage behind the handle, was rejected: a run would then depend on
storage outside the memory, so the behavioral predicates could not be
functions of the memory.

## Validity

Distinct keys, and for the ordered discipline ascending keys, is the
representation invariant (`Maps.Valid`). It is an implicit data invariant
of the owner type, deep as the Prover's data-invariant instrumentation is:
a value is valid wherever it appears, in fields, vector elements, and map
values. A function assumes it of its inputs and establishes it of what it
returns and modifies, and a loop keeps it. The lemmas carry it through every
role that produces a map (`valid_update`, `valid_remove`,
`valid_updateAll`, `valid_takeEntries`, `valid_dropEntries`, and
`valid_replaceKey` where the replacement does not abort). Iterator
validity is a validity slot ([`ghost-fields.md`](ghost-fields.md),
"Validity slots").

## Roles

Specification roles denote the model operations. Executable roles are used
through contracts generated from the role, not from the body, and those
contracts are trusted like native contracts: hypotheses of the typed
theorems, as the Prover trusts its prelude. The implementations are not
verified against them (the Prover does not verify them either). The
runtime provides a table owner's roles by the same model operations.

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
| `spec_iter_valid`, `spec_leaf_iter_valid`, `spec_iter_preserved` | — | iterator validity: slot equality; `spec_leaf_offset` uninterpreted |

Optional role results use the representation of their declared return
type: a one-vector-field struct, or an enum with an empty variant and a
single-field variant. Runtime execution and generated contracts share the
layout classifier; assuming the vector wrapper for an enum would give an
impossible result equation and a vacuous caller proof.

The bulk roles are read as the single ones: membership after `updateAll`
is membership in the keys or in the map, under the role's abort condition;
the size of an appended or bulk-updated map lies between the map's and the
sum of both, exact where every key is fresh; `takeEntries` and
`dropEntries` split a map at a position; `replaceKey` keeps the size and
moves one key; the keys of a built map read back as the keys it was built
from, so the position view holds of it as of its source. The abort
conditions of the bulk roles at literal vectors evaluate.

Deviations from the Prover, all where the model determines more than the
Prover states: `values`, `append`, `upsert_all`, `trim`, and
`replace_key_inplace` are exact rather than bounded, because the ordered
model fixes the positions the Prover leaves open; the sequence discipline
keeps `SimpleMap`'s order.

## Maps in proofs

The solver keeps map values abstract: observations (`size`, `hasKey`,
`valueAt`, `keyAt`, `rank`) are reduced by the lemmas over the operations,
never by evaluating a model's list, which would lose the lemmas' patterns.
Where the Prover's SMT solver has map axioms, the solver adds their
instances at the positions a leaf reads a map at, deciding the positions
by omega: a key the map has sits at its rank, and a key at a position
differs from each key the map lacks; the keys of a valid ordered map
ascend with their positions, and equal positions hold equal keys; an order
on keys is the order on their readings; a hypothesis quantified over the
positions of a map holds at each position the leaf reads.

A leaf states a table's map as `SpecMap.ofEntries keys values x` over the
owner's `SpecEntries` component ([`native-verification.md`](native-verification.md));
`verex_prepare typed` names the maps a leaf reads (`verex_name_maps`), so
that an authored proof argues with the `SpecMap` lemmas under the module's
own specification function names ([`source-verification.md`](source-verification.md),
"Tactics"). A companion proof file needs no local helper theorems about
maps; the general ghost-field selection lemma lives with contract
translation.

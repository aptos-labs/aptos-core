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

Code equality follows Move's physical representation. For entries-layout
maps, specification equality stays structural too. The Prover instead
compares intrinsic maps by content (`$IsEqual` over length, key set, and
values), which is coarser than what Move observes for a map whose physical
order is visible (`SimpleMap`: runtime `==`, `keys`, `values`,
`to_vec_pair`). An equality coarser than these observations is not a
congruence, so Leaner does not adopt it for entries-layout maps.

For `Table`, specification equality compares allocation identity, represented
by its handle (user decision, 2026-10-06). Distinct executions of `new`
allocate distinct identities, including executions at the same call site;
two empty tables are therefore unequal. Updating contents preserves identity.
The actual Table types have no `drop` ability and do not support runtime
`==`; their specification equality is available independently of that ability.
This identity rule differs from the Prover's content equality.

A Table observation also depends on the state at which its contents are
read. Identity equality does not equate those states or the snapshots of
their contents: `old(t)` and `t` can have the same identity and different
values at a key. In particular, Table identity equality must be a separate
predicate, rather than Lean equality of a snapshot carrying contents; Lean
equality would incorrectly allow substitution of every snapshot observation.
Functional `spec_set` and `spec_del` preserve identity while producing a new
content snapshot. State labels and inserted caller specifications must select
the snapshot from their bound memory, without requiring a program point.

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
  The production `aptos_std::table::Table` contains only a handle; the
  registry's `extensions::table::Table` also contains a cached length. Neither
  contains an entries vector. Reusing the model and laws does not add such a
  vector to the physical declaration. The handle, content snapshot, and any
  cached metadata need an explicit relation in the logical view.

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

Consequently, the verifier can reason about contents as an owned map snapshot
associated with the Table, even though runtime storage is external. A mutable
entry borrow transfers the value and reconciles its final value into that
owner's contents; it preserves the handle and unrelated entries. Shared
observations are stable for the duration of their certified borrow. This
extends the existing prophetic-reference discipline instead of introducing
independent aliases through the handle. The representation and its agreement
with native storage still need implementation; the ownership argument alone
does not supply that agreement or allocation freshness.

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

The stage-2 foundation is implemented in
`LeanerIR/Proofs/Maps/Table.lean`: a `Snapshot Identity` carries identity and
entries separately, and `SameIdentity` compares only identity. Update and
removal reuse the entry-level map laws. Mutable entry reconciliation preserves
identity, unrelated values, length, and key positions under the present-key
and validity premises. A monotone logical allocator proves freshness and
non-reuse; its relation to actual native handles remains an agreement
obligation, rather than an axiom in this library.

`LeanerIR/Proofs/Denote/TableMemory.lean` reads an unbounded typed entry array from an
explicit memory at the handle, scoped by owner and key/value types. Its laws
establish observation after a write, frames for distinct handles
and owners (including phantom instances), and commutation of writes to distinct
handles. Absent slots are junk: native contracts and data invariants must require
an allocated Table's slot to exist. Call agreement now names these slots via
`StorageEncodes`; `AgreeUnnamed` preserves resources outside ordinary globals,
Table contents, and allocation history. Shared lookup and membership now use
explicit intrinsic contracts as described below. Allocating and mutating Table
roles remain disabled until their contracts and ownership frames are integrated;
stage 2 is not complete.

The runtime now has `RuntimeState.tables : NativeTableStorage`, with a
contents heap and persistent allocation history. `TableMemory.resourceOf`
recognizes the two supported handle-backed layouts, including their phantom
key/value arguments; closed instances name native contents slots.
`TableMemory.EncodesStorage` relates the heap and allocation history to one
typed memory and is proved unique. Allocation history uses a fixed logical
collection slot of address elements. Both contents and allocation history use
`ResourceKind.collection`, whose carrier is an unbounded finite typed array;
ordinary values use `ResourceKind.value` and retain their native type's bounds.

The canonical-slot follow-up fixes two aliases exposed while preparing native
adapters: shared-reference carrier erasure and the same Table type named in
caller and callee namespaces. `resourceOf` accepts only nominal owners.
`runtimeResourceOf` chooses the declaring namespace and earliest type entry
naming the resource; its uniqueness theorem does not assume an injective type
interner. `slotOf?` resolves a caller's closed type to that physical key, preserving
its logical resource and allocation handle. Logical templates remain available
in both namespaces, and open templates do not resolve to runtime slots.
`Denote/TableStorage.lean` uses uniqueness to prove that a single slot insertion
or removal preserves the entire Table heap encoding. The subsequent storage
agreement increment combines these laws with ordinary globals and persistent
allocation history. Successful runtime allocation, insertion, removal and checked
retirement preserve `StorageEncodes`, with typed operation results. The same
operations preserve `StorageEncodesReturned` when unrelated owners have active
loans; insertion/removal require typed plain contents at the actual runtime slot,
not merely contents obtained by resolving outstanding references.
The two-module `Check/Storage/TableStorageKeys.lean` regression and the existing
Table snapshot proofs pass; the 53-job focused build and six-theorem axiom audit
pass (`/tmp/table-slot-focused-build.log`, `/tmp/table-slot-audit.log`). This
follow-up has focused validation; the snapshot-routing run remains the latest
full benchmark and suite checkpoint.

The returned-storage increment also proves that observation commutes with
insertion/erasure and cannot change a plain stored value, regardless of the
returned row. `StorageEncodesReturned.borrow_table_reconcile` connects actual
key-based borrowing and its registered loan to write-back of the typed replacement,
preserving both stores and allocation history. The proof does not grant aliasing
permission: native adapters must establish the ownership preconditions separately.
All five focused Table/returned-storage test roots pass in a 58-job build, including
two regressions for a colliding global loan and a stale returned Table loan after
replacement/erasure (`/tmp/table-storage-returned-focused-build.log`). The 31-theorem
audit reports only `propext`, `Classical.choice` and `Quot.sound`
(`/tmp/table-storage-returned-audit.log`). No native frontend role is enabled by
this proof increment; no new registry pass or broad validation is claimed.

The handle-only Table native has no total-entry-count guard, so inheriting the
Move vector's unsigned-64 bound would incorrectly restrict native storage.
The variant with a cached `u64` length must relate that field to the contents
in its own invariant and operation contracts. Generic substitution preserves
the storage kind, and the allocation slot contains no type parameters.

`StorageEncodes` now combines ordinary globals with Table storage in
`propheticRun`, closure invocation, runtime contracts, and behavioral predicates.
`StateShifted` relates native contents under loan renaming and preserves
allocation history. `runs_mirror` requires both stores to agree initially;
`state_of` uniqueness determines both stores in the post-state. The fixed
allocation resource encodes history only at `.unit` and requires its other
keys to be absent, so no unobserved keys escape the uniqueness proof.

Native operation contracts and frame clauses must follow ownership of Table
arguments and declared global modifications while preserving unrelated Tables.
Native shift and typed-storage preservation remain explicit hypotheses of the
agreement, as for existing native operations; this extension does not establish
them for Table natives or prove agreement with the production VM.

`Denote/ReturnedStorage.lean` now supplies the loan-aware observation at the
mutable-entry boundary. `exportReturnedFrameLoans` preserves the hole of an
escaping reference; `StorageEncodesReturned` resolves heap values using an
explicit returned row before encoding them. Prophetic call outcomes use the
result's prophecy row; runtime contracts and behavioral predicates use its
current row. Lookup, key ordering, and allocation history are preserved, and
resolution commutes with loan renaming. Plain returned rows leave storage
unchanged, so ordinary value-returning calls keep their previous observation.

`Contract.frame` now receives the result. Source `modifies` clauses retain their
existing state predicates; the runtime frame adapter uses the actual returned
row to observe open storage, as ensures does. It cannot pick unrelated values
to fill holes and thereby hide a mutation. The unconditional frame obligation
still applies when declared failure conditions excuse ensures.

Execution write-back now uses `RuntimeState.storageLoans`, whose `LoanTarget`
explicitly distinguishes globals from native Table contents. A registered entry
hole is filled inside its owning contents vector; the other store, unrelated
entries, and persistent allocation history are preserved. Transfer to a returned
subloan retains the target's storage domain. `FreshStorageLoanIds` and caller
admissibility exclude all stored registrations, and the loan-renaming proof
covers both destinations. Kernel-checked examples exercise colliding keys,
sequential write-back into both stores, and returned subloan transfer.

`Semantics/TableLoans.lean` also supplies the entry-loan primitives. Key lookup
rejects malformed rows and is proved to select exactly the queried key/value
pair. Mutable borrowing places the hole at that value, registers a tagged Table
target, and advances the loan frontier. Shared lookup creates no loan. A focused
entry decomposition proves both borrowing and reconciliation, retaining the key
and other entries; registration preserves the loan discipline and commutes with
loan renaming. Executable tests cover missing keys/slots, malformed entries,
colliding store keys, and successive borrow/write cycles.

`Semantics/TableOperations.lean` now supplies allocation at a proposed fresh
handle, insertion of an absent key, removal with its returned value, and checked
empty retirement. Freshness checks persistent history and all live slots across
owner/type instances; retirement preserves history. Each operation preserves
unrelated slots, globals and loan bookkeeping, preserves sorted storage, and
commutes with loan renaming. These are logical storage primitives, not claims
about injectivity of the VM's hash-based allocator or its exact abort codes.
Removal preserves internal entry order; Table exposes no enumeration order.
The checked retirement helper models an emptiness-checked wrapper, not the
production destroy native's unchecked body.

`Denote/TableOperations.lean` connects typed shared lookup, insertion and removal
to those operations, including missing/duplicate-key results. Insertion/removal
preserve key distinctness and establish their exact size and membership effects.
A successful typed lookup also decomposes the contents into a typed entry focus
and proves the actual key-based mutable borrow. Settling that focus restores
typed contents while preserving its keys and surrounding entries. This is
agreement at a resolved slot; `TableStorage.lean` now lifts these operations and
entry reconciliation to whole-storage agreement. Native argument/owner resolution
and ownership frames remain obligations of the adapter.

The operation test root passes eleven executable checks, including the complete
allocation/add/borrow/write-back/remove/retire lifecycle alongside another Table
and a colliding global loan, plus three generic typed boundary checks. The focused
48-job build and the seventeen-theorem standard-axiom audit pass
(`/tmp/table-operations-focused-final.log`, `/tmp/table-operations-audit.log`).
These modules remain test-imported leaves; they do not change the benchmark's
public import closure. No broad suite or benchmark rerun is claimed for this
increment; the preceding full collection-carrier checkpoint remains the latest.

The primitives do not replace `nativeCall`. Native
contracts and their adapter still need to tie the slot to the owner's type and
handle, connect native allocation, and frame owned effects.
The specification carrier retains content snapshots independently of physical
handles, including `old`, labeled and nested observations, and functional updates.
Allocating and mutating execution roles remain disabled until their adapters
establish agreement. The read-only contract route is described below.
The public storage-preservation hypothesis remains conditional until those
native operations establish it. The existing runtime typing theorem continues
to cover ordinary globals; native-content typing remains part of native
agreement, not a newly proved runtime typing guarantee.

The specification boundary is `LeanerLang/Contract.lean`. Table-bearing inputs
now use the separate logical carrier; ordinary inputs retain their existing
domains. A Table snapshot must survive aggregate projection, specification lets,
function arguments/results, quantification and functional updates. Pairing an
entire aggregate with one memory is insufficient: one specification vector can
contain two functional snapshots of the same nested Table with different values.
The representation must retain content observations per occurrence. Its identity
comparison must remain separate from Lean equality, including inside aggregates.
Observation takes the clause's explicit memory; an inserted caller contract must
not require a callee program point. Pure `spec_new` also needs a defined logical
identity policy distinct from executable fresh allocation; it cannot be treated
as a side-effecting allocator merely because it binds the map-new role.

`Denote/SnapshotValue.lean` supplies that separate observation carrier and is
imported by contract translation. Its `Value` has scalar, aggregate and Table nodes. A Table
keeps its physical handle/metadata plus optional contents whose values are
themselves observations. Missing storage stays `none`, distinct from an empty
allocated Table. Typed `observe` traverses tuples, nominal fields, variants,
vectors, reference pairs and Table value types at an explicit memory. It reads
Table storage only for a registry-bound owner with a supported physical layout.
Closures remain physical callable values; their invocation memory is a separate
behavioral-predicate argument, not an implicitly captured snapshot.

The physical projection of every typed observation equals its existing codec
encoding, and observation at a fixed memory is injective. The projection does
**not** license executing a hypothetical snapshot: a native/behavioral adapter
must establish that its contents agree with the selected execution memory.
`observeInFrame` resolves generic types before observing, so a Table hidden
behind a parameter is discovered at instantiation. Its transport theorem makes
caller and callee observations identical at the same explicit memory.
`observeRuntime?` supplies a checked codec-input bridge, with round-trip and
physical-agreement proofs; already-logical locals must never be observed again
through this bridge.

Logical projection/lookup retains each child's snapshot. Functional insertion
and removal operate on those values and update a cached length when present,
without reading memory or allocating an identity. Lookup/update/frame laws,
identity preservation, and positive size from membership are kernel-proved.
`Value.SameIdentity` recursively compares Table allocation identities inside
aggregates; it is not Lean equality and does not equate their contents.
Eleven kernel-checked regressions cover nested old/new storage, two functional
snapshots of one identity inside one parent, aggregate projection, distinct
allocations, cached lengths, missing storage, an unregistered lookalike struct,
and a Table behind an instantiated generic parameter. The 44-job focused build
and twelve-theorem standard-axiom audit pass (`/tmp/snapshot-value-focused.log`,
`/tmp/snapshot-value-audit.log`).

Contract translation now observes physical inputs at their explicit clause memory,
including old/global/result inputs, and preserves already-logical binders through
lets, expanded specification calls, constructors, projections, branches and vector
operations. Vector-element quantification ranges over the stored observations.
Table length, membership, lookup, functional set and removal roles consume that
carrier. Source equality uses `SameIdentity`. The original registry fixture
`table_contais_to_length` verifies at its unchanged budget; its obsolete diagnostic
baseline was regenerated and the normal filtered registry check passes.

`Check/Specifications/TableSnapshots.lean` covers mutable and nested inputs,
quantified labels in an opaque callee and its caller, and separate functional
snapshots inside one vector. These labels use explicit memory without program
points. All five source proofs and six focused compatibility checks pass. The
fifteen-theorem carrier audit uses only standard logical axioms. The full benchmark
passes 30 samples plus the expected AMM rejection at 15,807,887,568 raw heartbeats
(+0.0029% from the preceding checkpoint); local JSON/HTML were regenerated against
main before broad suites. All four suites now pass, including both cost gates
and 125 Check fixtures. The registry also verifies `map_equality_encoding`; six
other Table fixtures now reach native/reference gaps. The owning runner refreshed
seven changed baselines, all 437 normal checks pass, and the audit finds no
regression in previously verified targets (`/tmp/snapshot-routing-full-tests.log`,
`/tmp/snapshot-routing-prover-recheck.log`,
`/tmp/snapshot-routing-registry-audit.{json,log}`).

Type-domain quantification over Table snapshots, future `final` contents, pure
`spec_new` identity, and opaque/recursive specification-function snapshot domains
remain open. Executable/behavioral operands reject snapshots until their adapter
establishes agreement with the selected execution memory. Entries-layout maps
containing Table values also need logical-value support. These boundaries retain
explicit unsupported diagnostics rather than erasing hypothetical contents.

### Shared Table read contracts

`map_has_key` and `map_borrow` now support handle-only and cached-length Table
owners through the existing intrinsic-contract mechanism. The caller theorem
explicitly assumes the role's `Satisfies` contract; this does not prove the source
wrapper or production native implementation. Both contracts preserve memory and
observe their physical inputs at the entry memory. Membership returns the logical
membership result; shared lookup aborts on an absent key and otherwise returns
the selected logical value observed at that same memory. Key conversion matches
the specification role, including generic keys. Signatures are checked against
the owner's key/value types, and a shared lookup must return a shared reference.
No synthetic contents field or erasure of a hypothetical snapshot is introduced.

`SnapshotValue.observeRuntime?_instantiate` proves caller/callee agreement even
for malformed runtime inputs; integer and boolean observations have bounded
normalization rules. Identity reflexivity is available to the closer.
`Denote/TableReads.lean` proves that snapshot lookup agrees with the typed contents
lookup, including absent slots/keys and values containing nested Tables.
`Check/Storage/TableReads.lean` has five passing source proofs: membership, scalar
lookup, generic lookup, nested Table lookup, and an opaque caller using `old`.
A wrong existing-key lookup claim is rejected. The five-theorem axiom audit is
standard-only (`/tmp/table-read-proof-audit.log`); source logs are
`/tmp/table-read-source.log` and `/tmp/table-read-snapshots-regression.log`.
The five generated source theorems additionally retain the existing
`compileFunction_agrees` axiom from the denotation assumption ledger; no new
axiom was introduced (`/tmp/table-read-source-audit.log`). Their intrinsic
contracts are explicit theorem parameters, not additional axioms.

At the shared-read checkpoint, the registry's `table_option` reached its
assertion instead of rejecting
`borrow_box`. It still failed at the original 25,000 maxHeartbeats. Its prepared
goals exposed two issues: normalization of projections through observed
aggregates, and missing deep data invariants of externally stored values. In
particular, `option::borrow` requires its vector's length to be at most one, but
the physical Table fields do not carry the contained transaction's `Option`
invariant. `carriesInvariant`/`deepInvariantTerms` currently traverse physical
fields and entries-layout maps, not native contents. Do not assume the missing
bound or increase the budget to hide this gap. The saved source, export and
obligations are `/tmp/TableOption.generated.lean`, `/tmp/table-read-option-export`,
and `/tmp/table-option-goals.log`. The bitwise/verify_table fixtures now get past
shared reads and expose their still-unsupported mutation calls; no additional
whole registry fixture is claimed verified by this increment.
The scratch normalization in `/tmp/TableOptionNormalize3.lean` reduces two
remaining obligations to the actual stored vector's `size ≤ 1`, with no such
premise (`/tmp/table-option-normalize3.log`). It is diagnostic proof work, not
an installed companion or a claimed verification.

The aggregate-observation follow-up adds runtime-frame decoding and normalization
for known nominal/vector shapes. Unknown Table observations and unknown field
projections stay opaque, so a guarded lookup postcondition can identify them
before the closer considers cases. List-based observations and array-based
executable codecs normalize to the same array map. `TableReads.payload` now
verifies both equality and length of a nested-vector field returned by a Table
lookup; `wrong_payload_bound` rejects an unsupported `length <= 1` claim.
Existing functional snapshots and identity equality still pass. Eager unfolding
of all observations/projections was rejected: it split unknown snapshots into
constructors and obstructed the existing identity-preservation law.

The aggregate-observation change alone did not supply the missing invariant in
`table_option`. The collection extension described below covers external slots,
whose resource kind is
`.collection` and whose native element is the `(K, V)` entry tuple. The preceding
generated stored-invariant selector only dispatched on nominal global-resource
handles; it never reached those collection entries. A contents predicate can
constrain each entry's value independently of the observing memory, allowing
`MemoryInvariants.set` to keep checking writes without invalidating unrelated
slots. Its lookup consequence must then be related to `lookup_observe_table`.
Generic value instantiations, nested Tables and Table parameters without a global
resource access need coverage; enumerating only the concrete types of this one
fixture would leave the contract boundary incomplete. An invariant about a
Table's observed contents must not be made to hold in every possible memory:
entry, exit, old and labeled observations retain their selected memories.

The core invariant callback now receives both the declaration handle and its
resolved native type-argument row (2026-10-07). A handle alone cannot represent
an invariant that distinguishes phantom instantiations with identical physical
fields. `StoredValueInvariants.table_write` proves preservation when every new
key and value meets its own invariant; the lookup and snapshot theorems retain
the same type arguments. Seven focused examples cover the Option bound,
generic/phantom discrimination, valid and invalid writes, and traversal of
active enum payloads through vectors. The four theorem axiom audit contains
only `propext`, `Classical.choice`, and `Quot.sound`. Logs:
`/tmp/table-generic-invariants-{build,tests,audit}.log`.

Generated contracts now use this predicate (2026-10-07). The selector keeps
its global-resource cases and adds a collection branch traversing the native
entry tuple. Declaration callbacks receive the actual argument row; type-only
generic predicates substitute that row directly, while predicates requiring a
carrier retain a universally quantified compatible frame. Table calls in code
or specifications contribute to memory reach, including Table parameters with
no global access. Lookup consequences use `DataInvariant.table_get_raw`: raw
membership selects a typed entry, so computed keys need no new certificate.

The source fixture `Check/Storage/TableStoredInvariants.lean` verifies nine
cases at 25k: ordinary, generic, concrete phantom, nested and specification-only
reads, a computed key, and both supported handle layouts. Stronger bounds and
claims about another phantom argument remain rejected. The current
`table_option` runner still matches its 25k timeout baseline. A larger-budget
diagnostic closes its proof, but it is not an accepted registry verification.

The callback's memory-independent scope is explicit. Only the length of a bare
vector local may read its physical representation directly. Broadly suppressing
snapshot conversion would turn Table identity into equality of physical cached
metadata, or erase logical contents. `StoredTableInvariantErrors` guards that
boundary. Table-content operations in these predicates are rejected; Table
identity invariants needing observations and carrier-dependent generic predicate
automation remain open. The core write theorem is proved, but mutating native
contracts and their frontend preservation obligations are still pending.

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

### Owned Table values in the denotation (2026-10-07)

The intended prover carrier is an owned value containing allocation identity and
logical contents, as for other intrinsic maps. External storage is a runtime
representation detail. Mutating a Table changes the contents of its value;
borrowing an entry uses the existing current/prophecy discipline to reconcile
that entry into the final owner value. Identity equality remains separate from
Lean equality. Two historical snapshots of the same Table can differ; this does
not grant two live mutable owners of the same identity.

The current carrier is still physical: `NTy.carrier` maps a struct to its field
row, and `Carriers`/`NTy.codec` require an injective encoding into `RuntimeValue`.
An owned Table with its contents cannot satisfy that handle-only codec. Replacing
its carrier while retaining these laws would be incorrect. Nor may the frontend
add fictional physical fields or silently change the existing compiler-agreement
axioms. `Term.propheticRun` currently lends physical rows and relates the whole
runtime store to `Memory`; this is the boundary which must be generalized.

`Proofs/StateView.lean` supplies a first, separately checked boundary: `erase`
projects a logical value to its physical value, while `recover?` reads that value
at a runtime state. `Realizes` states consistency at that state; only realized
values in the same state have injective erasure. Historical or functional
snapshots are not assumed to be live in the current state. `StateView.lift`
recovers returned values at the final state and makes any recovery failure an
undefined obligation. `StateView.function` likewise makes inconsistent input
values undefined rather than allowing vacuous verification. Its weakest-
precondition and refinement laws are kernel proved. Ordinary tight codecs retain
exactly their existing `decodeSpec` semantics.

`Denote/TableValueBoundary.lean` instantiates this interface with the existing
recursive snapshot observer. It handles aggregates, nested Tables and generic
instantiation without program points. This observer retains missing storage as
an explicit absent Table snapshot; the boundary does **not** establish allocated
storage, distinct keys, cached-length consistency or exclusive ownership. Those
are separate validity obligations for executable Table values. The new boundary
is test-imported, and no native role or source carrier changes yet.

The follow-up adds `Observed frame type`, a typed physical carrier together
with its retained logical snapshot and a proof of representability at some state.
`Observed.view.Realizes` separately checks whether that snapshot is live at a
particular state; historical snapshots are still ordinary retained values.
`toSkolem`/`ofSkolem` preserve the exact snapshot and are proved inverse. They do
not erase a generic Table's contents and recover them using a later state.

`ArgumentRow` has the existing physical `HList` shape on its runtime side and
enriched values on its logical side. Top-level mutable parameters carry two
observations. `StateView.prophecy` checks the current and eventual observations
against **different supplied memories**, while plain/shared parameters use the
entry observation. Tuples recursively preserve their components; `resultView`
also handles the compiler's result shape, including a tuple of returned references
and ordinary values. The appropriate memories must come from loan resolution;
they are not necessarily entry/exit physical heaps when a returned loan survives
the call. These row adapters do not establish ownership or replace
`argumentsResolve`/`StorageEncodesReturned`.

In particular, `StateView.function` is a one-state input adapter for ordinary
values. Do not apply it to a mutable argument row by requiring every prophecy to
be live at entry. Future-value consistency belongs with the existing prophetic
resolution relation. References nested inside arbitrary aggregates and reference-
valued type arguments in other profiles also need explicit treatment before
claiming a general replacement of the current denotation. Move rejects reference
type arguments (`Validation/Borrowing.lean`); the standalone row library is not
an argument that another profile does so.

The expanded 19-check `StateView` root and six additional standard-axiom theorem
audits pass (`/tmp/table-observed-carrier-final-tests.log`,
`/tmp/table-observed-carrier-audit.log`). The public compiler, contract generator,
and native dispatch still use their existing carriers; these additions alone do
not fix a registry target or alter benchmark measurements.

Implementation work, in dependency order:

1. Introduce the logical carrier at the denotation boundary and carry it through
   generic parameters, aggregate fields, vectors, closure captures and parameter/
   result rows. Preserve ordinary tight codecs for storage-independent values.
   Prove transport laws at instantiation; never erase nested contents and recover
   them using an unrelated later state.
2. Relate those values and prophetic entry loans to the runtime store, reusing
   `TableOperations`, `TableStorage` and `ReturnedStorage`. Runtime storage frames
   belong in these agreement proofs. The experimental extension of ordinary
   source frames to Table footprints was removed before being enabled.
3. Give Table roles pure value contracts: new/add/remove/shared and mutable
   lookup, membership and cached length. Prove validity preservation, exact
   duplicate/missing-key abort behavior, and allocation freshness/non-reuse.
   Allocation history is still needed at the semantic boundary; it is not the
   Table's content representation.
4. Route old/current/labeled observations through the correct logical snapshots.
   Arbitrarily quantified state labels need enough state information to select
   those snapshots; a state containing only ordinary globals is insufficient.
   Caller-bound labels must work without program points, and no assumed equality
   of global memory may imply that an owned Table's contents stayed unchanged.
5. First end-to-end gate: `verify_table` positives verify, intended false results
   remain rejected, and duplicate/missing-key cases retain their exact aborts.
   Then cover `bitwise_table`, mixed generic instances, nested Tables, Tables
   stored in globals, and returned entry references. Refresh measured benchmark
   data and generated main-relative HTML before the next broad suite checkpoint.

The registered `Tests/StateView.lean` exercises final-state recovery, invalid input
and output rejection (including a bottom computation), abort/undefined preservation,
Table snapshots and typed-storage updates, and generic transport. The 120-job
core/frontend and boundary build passes; the final 46-job focused build and
seven-theorem standard-axiom audit pass (`/tmp/table-value-boundary-checkpoint.log`,
`/tmp/table-value-boundary-final-tests.log`, `/tmp/table-value-boundary-audit.log`).
No new source verification or registry result is claimed by these boundary tests.

This migration supersedes the plan to expose Table contents-slot frames in every
ordinary client contract. The footprint lemmas below remain useful at the runtime
agreement boundary; they are not the proposed client-facing representation.

### Table observation footprints

`Denote/TableFootprint.lean` names every typed slot read by a Table-bearing
observation, including nested Tables reached through stored values. An absent
slot remains in the footprint because later allocation changes its observation.
The pre-state footprint is sufficient: kernel proofs establish that agreement
on those slots preserves the whole snapshot. Disjoint write frames therefore
preserve unrelated observations. Generic instantiation and encoded inputs have
transport/decoding laws, without program-point metadata.

These are dependencies, not ownership grants. The runtime agreement adapter must choose
owned or mutable values, combine their Table effects with declared global
modifications, and handle allocation/history separately. `PreservesOutside`
allows precisely the listed slots and composes across successive writes.
The physical-memory denotation still uses whole-memory equality when no global
modifies clause is present. The owned-value migration above replaces that coupling
at the representation boundary. These lemmas alone do not change source verification.

Eight registered kernel checks cover absent/nested slots, rejecting an outer-only
frame, unrelated handles, unregistered lookalikes, allocation history, phantom
instances, and malformed encoded input. The 45-job focused build and an
eight-theorem standard-axiom audit pass (`/tmp/table-footprint-final-build.log`,
`/tmp/table-footprint-audit.log`). The preceding pool checkpoint remains the last
complete benchmark and four-suite run; the new module is test-imported only.

## Stages

1. `cmp::compare`; the model library with its laws; the entries-layout view;
   validity as a data invariant; specification roles; executable-role
   contracts.
   Gate: `simple_map`, `pool_u64`, `ordered_map`.
2. Table natives and table-layout owners. Gate: `table`,
   `table_with_length`, `smart_table`, `big_vector`, `smart_vector`,
   `storage_slots_allocator`, `pool_u64_unbound`, `big_ordered_map`.
3. Iterator validity versions (`BigOrderedMap` iterators).

# Ghost fields

This design covers model-only fields carried through values and specifications,
their interaction with function values, and their runtime agreement. Registry
problem C8 (`prover-test-problems.md`) and the iterator-validity slots of
`intrinsic-maps.md` depend on this model. Implementation stages and remaining
agreement work are described below.

## The Move Prover's ghost fields

A struct's specification block may declare ghost fields: `spec S { ghost g:
T; }`, optionally with an initializer, `ghost g: T = e;`, over the struct's
fields. A ghost field is a model-only field of every value of the struct (of
every variant of an enum):

- fresh at pack: any value of its type, or the initializer's value over the
  packed fields;
- carried by copy, move, and borrow;
- excluded from equality, in code and in specifications;
- read in specifications (`s.g`), named bare in the struct's invariants;
- written by `update s.g = e;` in an inline specification block, on a local
  or through a mutable reference.

The model keeps ghost fields apart from runtime fields
(`StructData.ghost_field_data`), so bytecode packs carry none; the Boogie
backend appends them as constructor arguments and excludes them from
`$IsEqual`. It also synthesizes a ghost slot `$validity: num` on an
intrinsic map, and on its iterator enums, that binds a validity role
(`synthesize_validity_slots`): validity is slot equality, a structural
mutation havocs the map's slot, a value write keeps it.

## Decisions (2026-10-14, user)

1. A ghost slot is its own value form: `RuntimeValue.ghost v`, `NTy.ghost τ`.
   A value of a struct with ghost fields holds its declared fields, then one
   slot per ghost field in declaration order, then (a table owner) the hidden
   contents. Equality, compilation, and the map readers recognize the slots
   by their form, with no declaration lookups.
2. As in the Move Prover, ghost state belongs to the verification model
   only. Initializers and `update` right sides are specification
   expressions, which only the denotation evaluates: no executability
   requirement (specification functions, quantifiers, `num` are all fine).
   The runtime keeps the slots and ignores their contents.
3. A call means the callee's model, not its runtime run, so that a callee's
   ghost postcondition reaches its callers, as in the Prover's modular
   verification.

4. (2026-10-15) In a unit declaring ghosts, `==` at a type mentioning a type
   parameter compares, in code and in specifications, the values' runtime
   encodings with ghost contents dropped: a generic function is proved once
   over abstract parameters, so equality there cannot rely on the
   instantiation's own (ghost-ignoring) equality, as the Prover's
   monomorphized `$IsEqual` does. Units without ghosts keep carrier
   equality at type parameters.

5. (2026-10-15) In the model, a native call and the invocation of a function
   value mean model meanings, as the Prover interprets both by the model: a
   native is an abstract meaning assumed to satisfy its contract (a map
   role's contract states its validity slot, G5), and an invoked target is
   routed as a callee is (its contract, or its body under the same
   meanings). A unit's behavioral predicates (`ensures_of<f>`, …) and its
   stored functions' invariants (`KeepsMemoryAt`) are defined from these
   meanings. The runtime run of a native or a closure never holds ghost
   contents, so a ghost-bearing result matches no denotation value there:
   left at the runtime meaning, the model would lose the call's outcomes.

Rejected on the way: runtime-evaluated ghosts (the runtime cannot evaluate
specification expressions: every Move specification arithmetic is `num`),
and a ghost table with calls left at the runtime meaning (a callee's ghost
postcondition never reaches its callers).

## Runtime

- `StructDecl.ghosts : Array GhostFieldDecl` (location, name, type, an
  optional initializer: an expression over the struct's locals, as its
  invariants are; an enum's sees only its type parameters). RawUnit schema
  bump.
- A pack takes the declared fields and appends `.ghost .unit` per ghost
  field; a destructor, a pattern, and a variant test read the declared
  fields only. `HasType (.ghost v) ghostSlot` holds for every loan-free `v`.
- `Place.ghostField` and `DataOperation.selectGhost` name a ghost field's
  slot contents; they are specification-only (validation rejects them in
  executable code). An `update` condition's target is a `ghostField` place,
  over a local or through a `&mut` local, or a spec variable's memory field
  (below); the runtime does not execute it. Its value is a specification's
  (at the field type's specification type, `num` for an integer), injected
  into the field type (`spec.intToBitVector`).
- A struct's invariants and initializers read its ghost fields by name, as
  they read its fields: the declaration's specification locals are its
  fields, `this`, then its ghost fields (an enum: `this`, then its ghost
  fields, each read from its variant's slot).
- Compilation to XIR drops the slots, the declarations, and the conditions.

## Spec variables

As in the Move Prover, a spec variable `global x: T` is the ghost memory
resource `Ghost$x { v: T }` at address 0, whose existence is an axiom of its
module; the frontend marks the struct `pragma ghost_memory` (the
move-model's `GHOST_MEMORY_PREFIX`). Only the model's memory holds it:

- `update x = e` stays a specification condition, its target the field
  `global<Ghost$x>(@0x0).v`; the denotation writes the resource
  (`Term.ghostStore`, anchored as a publication is, followed by the
  invariants owed where memory is written), with the value its ghost table
  gives.
- It counts as a write of memory: the frontend gives a function updating a
  spec variable the frame `modifies *`, and a loop whose body updates one
  touches the store.
- Compilation to XIR drops ghost memory structs, renumbering the others.

## Denotation

- `NTy.ghost τ` has `τ`'s carrier and encodes as `.ghost (encode τ x)`; a
  struct's or variant's native row holds a `.ghost` entry per ghost field
  after its declared fields. `Proj.ghost` steps into a slot's contents;
  `Term.ghostSlot`/`Term.ghostRead` convert between a slot and its contents.
- `NTy.num` is a specification's unbounded integer (carrier
  `SpecInt .unbounded true`): a `num` ghost field's or spec variable's type.
- `Meanings` holds the ghost tables: by site, the value an `update` writes
  (over the function's locals and state), and the value an initializer gives
  a pack's slot (over the packed value). A pack fills a slot without an
  initializer with any value (`Term.arbitrary`).
- A write takes the value its table gives; that the table gives one — the
  value lies in the field type's domain — is owed (`ghostAt` certifies it,
  the solver cuts it with `wp_ghostAt` and reports it at the update), as the
  Prover asserts it.
- `NTy.eqb` on a ghost slot is `true`; order (`cmp::compare`) and
  serialization (BCS, hashing) skip ghost slots, in the runtime and the
  denotation. A function value's captures are untyped, so its equality, in
  code and in specifications, compares them with their ghost contents
  dropped (`RuntimeValue.eraseGhosts`), as the runtime holds them.
- Twins carry ghost fields (`FieldRep.ghost`), so specifications read them
  as fields.

## Verification and agreement

The Move Prover keeps ghost state in its instrumented model only: a spec
variable is the ghost memory struct `Ghost$x`, `update x = e` a write-back
of it, `update s.g = e` an `UpdateField` stored back; a non-opaque callee is
inlined, an opaque one replaced by its specification (havoc what it
modifies, apply its `update`s, assume its `ensures`). Verex follows it:

- A function's theorem is modular: every callee used through its contract
  is an abstract meaning assumed to satisfy that contract, as a member of a
  cycle of calls is today (`cycleMeanings`); a callee without one is
  inlined, its body under the same meanings.
- The model of a unit is the least fixed point (`Spec.fixFamily`) of the
  bodies of its contract functions under their ghost tables, a native
  meaning its runtime run. Fixed-point induction (`satisfies_fixFamily`)
  over the modular theorems gives every function's contract on the model;
  the cycle construction is its special case.
- Agreement (assumed; the cycle axiom generalized to the unit): every
  outcome of a function's prophetic big-step run is, with ghost contents
  and ghost memory erased, the erasure of an outcome of its model. On a
  unit without ghost declarations erasure is the identity.
- Built so far: a unit declaring ghost fields or ghost memory
  (`declaresGhosts`) is verified through its model (`compileModelFunction`;
  `compileFunction` refuses it, so the current axioms never meet a ghost
  slot). A contract callee is an abstract meaning with a `Satisfies`
  hypothesis, an inlined callee one refining its body under the same
  meanings. A proof routing a generic function is over every skolem family
  and instantiation, as a cycle's through a generic member is. The
  theorem takes the meaning of invoking a function value at every family
  (`verexInvoke`, `Meanings.invoke`), the hypothesis that an invocation is
  defined where its target's declared precondition holds, and, per routed
  member without type parameters, that a literal closure of it invokes the
  member's meaning on the woven arguments (`invokeAt`). A contract of a
  ghost unit reading the unit takes a `ModelUnit` (the executable and the
  invocation meaning) where another takes the executable; its behavioral
  predicates (`EnsuresOfAt`, `AbortsOfAt`, `ResultOfAt`, `StateOfAt`) and
  stored-function frames (`FramedAtOf`, `KeepsMemoryAtOf`) are over the
  meaning. A value a specification builds carries its type's ghost slots
  with unspecified contents. Not yet: the unit's least fixed point and the
  restated agreement; invocations lending mutable references, `result_of`
  of an unseen closure, and closures of generic targets in the model.

## G4b: the unit model (design, 2026-10-15, for review)

What exists: a function of a ghost unit is proved over abstract meanings
(`routedSelfBinders`): a contract callee or native satisfies its contract,
an inlined callee refines its body under the same meanings, at every family
where a generic function is routed. What is missing is the object those
proofs are about, and with it closures.

1. The unit model. `M : (h : FunctionHandle) → SelfFamily` is the least
   fixed point (`Spec.fixFamily`, indexed by handle, family and
   instantiation) of the unit's bodies: `M h = denoteWith (meanings M)
   compiled_h`, the meanings routing every call, specified or not, to `M`
   and carrying the unit's ghost tables. A native means its contract's
   summary (`Contract.summary`: the outcomes its contract allows), as the
   Prover interprets a native by its specification. `M` is defined once
   per unit, beside the unit's compiled functions.
2. Invocation. `Meanings` gains the meaning of invoking a function value,
   `invoke : ClosureValue → …`, used by `Term.invoke` in place of
   `closureMeaning executable`. Its default is `closureMeaning` (non-ghost
   units, unchanged). In a ghost unit it calls the target's meaning in the
   meanings at hand on the arguments composed with the captures (decoded at
   the target's parameters, which the captures, holding ghost contents,
   decode at).
3. Behavioral predicates. In a ghost unit `requires_of`, `aborts_of`,
   `ensures_of`, `result_of`, a stored function's `KeepsMemoryAt` and its
   write frame are stated over `M`'s invocation, not the runtime run. The
   contract layer selects the form by `declaresGhosts`.
4. Assembly. A specified function's theorem about `M`, `Satisfies (M h)
   contract_h`, follows from the modular theorems by fixed-point induction
   over the unit (`satisfies_fixFamily`, generalized to post-fixed members:
   an unspecified function's approximant refines its body, which is what
   the modular theorems assume of an inlined callee). Within the induction
   a call or an invocation reaches the approximant, so recursion through
   function values is covered as calls are. The solver's closure rules read
   these assembled theorems where they read the runtime theorems today.
5. Agreement (assumed, restating the cycle axiom for the unit): every
   outcome of a function's prophetic big-step run is, with ghost contents
   and ghost memory erased, the erasure of an outcome of `M`; a native's
   runtime run agrees with its contract summary so. On a unit without
   ghost declarations erasure is the identity and `M` is not built.

Reviewed 2026-10-15 (decided: acyclic unfolding, gate first):

- Unspecified functions are not slots of the fixed point. An inlined
  callee means its body under the meanings, unfolded; the unfolding is
  well founded because a cycle member is a contract callee
  (`usedThroughContract`), and a recursive function without a
  specification is rejected in a ghost unit. Rejected: proving the
  denotation monotone in the meanings, which a fixed point over inlined
  members would need (`approx k ⊑ approx (k+1)`).
- A closure of a target with type parameters carries no family: its
  invocation is not routed and stays unseen. A closure of a target without
  type parameters is routed at the frame's family, its captures decoded at
  the target's parameters (`invokeAt`).
- The modular theorem takes the invocation meaning as a binder (`invoke`),
  with one routing equation per routed member (a literal closure of the
  member invokes the member's meaning on the woven arguments) and the
  hypothesis that an invocation is defined where the target's declared
  precondition holds (`RequiresOf`). The behavioral predicates and frames of
  a ghost unit are stated over that binder (`EnsuresOfAt`, `AbortsOfAt`,
  `FramedAtOf`); a contract reading the unit takes it after the frame. A
  literal closure's target is a routed member as a callee is. The solver
  reads the routed binders inside a modular proof, never an assembled
  theorem.
- M depends on the unit's verification: a function without a theorem
  (failed, not attempted, or not carried) means its contract summary in
  M, as its callers assume. M takes the native meanings as parameters,
  since a native's contract may state a behavioral predicate.

Order of work: (2), (3) and the routing of closure targets, to the gate
(`ghost_field_bp`, `ghost_field_closure_equality_ext`, the closure
functions of `loop_memory_havoc`): done 2026-10-15; then (1), (4), (5) as
the statement of what the modular theorems are about; then G5.

Open after the gate: `grind` reports an unknown free variable on two leaves
of `loop_memory_havoc`'s `fv_field_*` functions (which fail as the Prover's
do); the leaves' contexts check, so the variable is grind's own.

## Validity slots (intrinsic maps, stage 3)

An intrinsic map binding a validity role, and its iterator enums, get a
synthesized ghost slot (`$validity`, spelled `_validity` where a VerexSL
rendering spells a hidden name: `VerexSL.Contract.validitySlotName`). The
roles' contracts state it: creation chooses any value; a structural mutation
(`add`, `remove`, `pop`, …) leaves the exit's slot unconstrained; a value
write (`borrow_mut`, `iter_borrow_mut`, `upsert` at an existing key) keeps
it. `spec_iter_valid(it, m)` is the iterator's slot equal to the map's, and
`spec_iter_preserved(new, old)` is slot equality.

Done 2026-10-15:

- Representation. An owner's value lays out its declared fields, then its
  ghost slots, then its hidden contents (`Maps.Layout.ghosts`, `Layout.slot`,
  `slotsOf`, `freshSlots`; `Layout.build handle entries slots` with the slots
  a trailing argument fresh by default, so that authored proofs over layouts
  without slots read as before). The model operations keep a map's slots
  (`update`, `remove`, `updateAll`, `takeEntries`, `replaceKey`); a map built
  afresh (`empty`, `dropEntries`, `map_new`) holds fresh slots.
  `Layout.restamp map stamp` is the map with its slots chosen by `stamp`;
  every observation (`entriesOf`, `size`, `hasKey`, `valueAt`, `keyAt`,
  `rank`, `Valid`, `keysOf`, `valuesOf`, `prevKey?`, `nextKey?`) sees
  through it.
- Roles. For an owner holding its map in its own layout with slots, a
  structural role ensures `∃ stamp, exit = restamp (op …) stamp` (`new`,
  `new_from`, `add`, `del_*`, `upsert` at a new key, `add_all`, `append*`,
  `trim`, `replace_key_inplace`, `pop_*`); a value write (`borrow_mut`,
  `iter_borrow_mut`, `upsert` and `add_override_if_exists` at a present key)
  keeps the slots, as the model operations do. Owners of other layouts
  (wrappers, sorted views) keep their slot as a field (`keptSlot`).
- Predicates. `spec_iter_preserved(new, old)` is `Layout.slot new k =
  Layout.slot old k`; `spec_iter_valid` and `spec_leaf_iter_valid` equate
  the iterator's slot with the map's. An iterator's slot is read by its
  distance from the end of its fields (`VerexSL.Contract.slotFromEnd`): the
  slots follow every variant's fields alike, so the read needs no variant,
  where a variant selection on an abstract iterator left the normal form
  undecidable conditions.
- Equality. In a specification, `==` between two owners holding their map
  in their own layout with slots is `∃ stamp, l = restamp r stamp`: equal
  but for the slots (the Prover's `$IsEqual` excludes ghosts), in the form
  the role contracts state, which the map lemmas carry; owners without
  slots keep the identity projection.
- Iterators. An iterator names its entry by the key it carries (a payload
  field of the map's key type parameter) or by a position (one integer
  field), as the Prover's `iter_key_field` classifies it (`IteratorModel`);
  `iter_borrow_mut` and `spec_aborts_iter_borrow_mut` follow the flavor. A
  role native's declared `requires` clauses are owed by its callers, as the
  Prover asserts them (`iter_borrow_mut`'s validity).
- Solver. Closed side conditions of normal-form lemmas (a slot's position
  against its layout's count) are decided; a discharger's alternatives end
  in `skip`, as a last alternative fails in recovery mode, which logs the
  failure; `eraseGhosts` on scalars is a grind lemma, for a generic
  callee's equality at a type parameter; the plainness search of the role
  steps (`roleStep_plain`) unifies at reducible transparency.

Gate: `verify_iterator_validity` reports the Prover's thirteen failures and
nothing else; `ghost_field_intrinsic_map_ops` reports the two intended
failures and the bulk roles' open clauses (registry C8); the framework's
`ordered_map` proofs verify as before. Open: `big_ordered_map`'s iterator
targets. Of its 43 targets, 6 verify, before G5 as now; before G5, 16
iterator targets were rejected with "role not carried", and with G5 they
are attempted. An owner's slot of an encoded value stays a term
(`slotFromEnd_nominal` reads only nominal literals), as the role and
validity clauses only equate slots. What remains is not the slots: the same
rendering with the validity roles and slots removed fails the same
assertions at the same cost, under the verifier before G5 and now. Leaves
now split on tested variants only after the deciders that need no split
(`perf-notes.md`, "Variant splits after the deciders that need none"),
which halves the walk of `test_verify_iter_next` (0.63G heartbeats) and
verifies `test_verify_iter_valid_after_insert_symbolic`. The normal form
decides literal `Int` equality and closed side conditions, such as
`¬AbortsNewFrom` over literal vectors, by its own lemmas, and computes a
map built from literal vectors (`perf-notes.md`, "Maps built from
literals"): 8 of 43 targets verify (2026-10-17).

## Milestones

- G1. Exchange: ghost fields (name, type, initializer) exported with the
  struct; XAST version 11; decoder.
- G2. VIR: declarations, `RuntimeValue.ghost`, pack, typing, validation of
  ghost reads and `update` targets, JSON, relocation, printing.
- G3. Frontend: encoding of ghost declarations, reads, and `update`
  conditions; VerexSL syntax, lowering, rendering.
- G4a. Denotation: `NTy.ghost`, ghost tables for `update` values and
  initializers built by the contract layer, equality. Gate: the C8 tests
  whose functions call no ghost-observing callee. Done 2026-10-15, with
  spec variables: `ghost_field`, `ghost_field_invariants`,
  `ghost_field_closure_equality` and `loop_memory_havoc` report what the
  Prover does, but for generic functions, closure targets, and functions
  without a specification (Verex verifies specified functions).
- G4b. Modular theorems, the unit model as one least fixed point, the
  agreement up to ghost contents; generic functions and closure targets in
  the model; the solver's call rules over an abstract call meaning. Gate:
  the remaining C8 tests, then the full suites.
- G5. Validity slots on intrinsic maps. Gate: `verify_iterator_validity`,
  `big_ordered_map`'s iterator targets. Done 2026-10-15 (above).

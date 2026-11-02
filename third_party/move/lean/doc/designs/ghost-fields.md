# Ghost fields

Model-only fields carried through values and specifications, their
interaction with function values, and their runtime agreement. Registry
problem C8 ([`prover-test-problems.md`](../status/prover-test-problems.md))
and the iterator-validity slots of [`intrinsic-maps.md`](intrinsic-maps.md)
depend on this model. Open work: [`roadmap.md`](../plan/roadmap.md).

## The Move Prover's ghost fields

A struct's specification block may declare ghost fields: `spec S { ghost g:
T; }`, optionally with an initializer over the struct's fields. A ghost
field is a model-only field of every value of the struct (of every variant
of an enum): fresh at pack (any value, or the initializer's); carried by
copy, move, and borrow; excluded from equality, in code and in
specifications; read in specifications (`s.g`), named bare in the struct's
invariants; written by `update s.g = e;` in an inline specification block,
on a local or through a mutable reference.

The Prover keeps ghost fields apart from runtime fields, so bytecode packs
carry none; its Boogie backend appends them as constructor arguments and
excludes them from `$IsEqual`. It also synthesizes a ghost slot
`$validity: num` on an intrinsic map and on its iterator enums, bound to a
validity role: validity is slot equality, a structural mutation havocs the
map's slot, a value write keeps it.

## Decisions

1. A ghost slot is its own value form: `RuntimeValue.ghost v`, `NTy.ghost
   τ`. A value of a struct with ghost fields holds its declared fields,
   then one slot per ghost field in declaration order, then (a table
   owner) the hidden contents. Equality, compilation, and the map readers
   recognize the slots by their form, with no declaration lookups.
2. As in the Prover, ghost state belongs to the verification model only.
   Initializers and `update` right sides are specification expressions,
   which only the denotation evaluates (specification functions,
   quantifiers, `num` are all fine). The runtime keeps the slots and
   ignores their contents.
3. A call means the callee's model, not its runtime run, so that a
   callee's ghost postcondition reaches its callers, as in the Prover's
   modular verification.
4. In a unit declaring ghosts, `==` at a type mentioning a type parameter
   compares, in code and in specifications, the values' runtime encodings
   with ghost contents dropped: a generic function is proved once over
   abstract parameters, so equality there cannot rely on the
   instantiation's own ghost-ignoring equality, as the Prover's
   monomorphized `$IsEqual` does. Units without ghosts keep carrier
   equality at type parameters.
5. In the model, a native call and the invocation of a function value mean
   model meanings, as the Prover interprets both by the model: a native is
   an abstract meaning assumed to satisfy its contract (a map role's
   contract states its validity slot), and an invoked target is routed as
   a callee is (its contract, or its body under the same meanings). A
   unit's behavioral predicates and its stored functions' invariants
   (`KeepsMemoryAt`) are defined from these meanings. The runtime run of a
   native or a closure never holds ghost contents, so left at the runtime
   meaning the model would lose the call's outcomes.

Rejected: runtime-evaluated ghosts (the runtime cannot evaluate
specification expressions: every Move specification arithmetic is `num`),
and a ghost table with calls left at the runtime meaning (a callee's ghost
postcondition never reaches its callers).

## Runtime

- `StructDecl.ghosts : Array GhostFieldDecl` (location, name, type, an
  optional initializer: an expression over the struct's locals, as its
  invariants are; an enum's sees only its type parameters).
- A pack takes the declared fields and appends `.ghost .unit` per ghost
  field; a destructor, a pattern, and a variant test read the declared
  fields only. `HasType (.ghost v) ghostSlot` holds for every loan-free `v`.
- `Place.ghostField` and `DataOperation.selectGhost` name a ghost field's
  slot contents; they are specification-only (validation rejects them in
  executable code). An `update` condition's target is a `ghostField`
  place, over a local or through a `&mut` local, or a spec variable's
  memory field; the runtime does not execute it. Its value is a
  specification's (`num` for an integer), injected into the field type.
- A struct's invariants and initializers read its ghost fields by name, as
  they read its fields: the declaration's specification locals are its
  fields, `this`, then its ghost fields (an enum: `this`, then its ghost
  fields, each read from its variant's slot).
- Compilation to XIR drops the slots, the declarations, and the conditions.

## Spec variables

As in the Prover, a spec variable `global x: T` is the ghost memory
resource `Ghost$x { v: T }` at address 0, whose existence is an axiom of
its module; the frontend marks the struct `pragma ghost_memory`. Only the
model's memory holds it: `update x = e` stays a specification condition,
its target the field `global<Ghost$x>(@0x0).v`, and the denotation writes
the resource (`Term.ghostStore`, anchored as a publication is, followed by
the invariants owed where memory is written). It counts as a write of
memory: a function updating a spec variable gets the frame `modifies *`,
and a loop whose body updates one touches the store. Compilation to XIR
drops ghost memory structs, renumbering the others.

## Denotation

- `NTy.ghost τ` has `τ`'s carrier; a struct's or variant's native row
  holds a `.ghost` entry per ghost field after its declared fields.
  `Proj.ghost` steps into a slot's contents.
- `NTy.num` is a specification's unbounded integer, the type of a `num`
  ghost field or spec variable.
- `Meanings` holds the ghost tables: by site, the value an `update` writes
  (over the function's locals and state), and the value an initializer
  gives a pack's slot (over the packed value). A pack fills a slot without
  an initializer with any value (`Term.arbitrary`). That the table gives a
  value in the field type's domain is owed (`ghostAt`; the solver cuts it
  with `wp_ghostAt` and reports it at the update), as the Prover asserts
  it.
- Equality on a ghost slot holds; order (`cmp::compare`) and serialization
  (BCS, hashing) skip ghost slots, in the runtime and the denotation. A
  function value's captures are compared with their ghost contents dropped
  (`RuntimeValue.eraseGhosts`), as the runtime holds them.

## Verification and agreement

The Prover keeps ghost state in its instrumented model only: a non-opaque
callee is inlined, an opaque one replaced by its specification (havoc what
it modifies, apply its `update`s, assume its `ensures`). Verex follows it:

- A function's theorem is modular: every callee used through its contract
  is an abstract meaning assumed to satisfy that contract; a callee
  without one is inlined, its body under the same meanings. The unfolding
  is well founded because a cycle member is a contract callee
  (`usedThroughContract`), and a recursive function without a
  specification is rejected in a ghost unit; unspecified functions are not
  slots of a fixed point (rejected: proving the denotation monotone in the
  meanings, which that would need).
- A unit declaring ghost fields or ghost memory (`declaresGhosts`) is
  verified through its model (`compileModelFunction`). The modular theorem
  takes the invocation meaning as a binder (`invoke`), one routing
  equation per routed member (a literal closure of the member invokes the
  member's meaning on the woven arguments, `invokeAt`), and the hypothesis
  that an invocation is defined where the target's declared precondition
  holds and, of a value no closure site encodes, always, as the Move Prover
  calls function values without asserting `requires_of` (`RequiresOf` of
  `functionRequires (fun … => True)`). The behavioral predicates and frames of a ghost
  unit are stated over that binder; a contract reading the unit takes a
  `ModelUnit` where another takes the executable. A closure of a target
  with type parameters carries no family: its invocation is not routed and
  stays unseen. A value a specification builds carries its type's ghost
  slots with unspecified contents.
- The unit model `M : FunctionHandle → SelfFamily` is the least fixed point
  (`Spec.fixFamily`, indexed by handle, family and instantiation) of the
  unit's bodies under their ghost tables, a native meaning its contract's
  summary. A specified function's theorem about `M` follows from the
  modular theorems by fixed-point induction (`satisfies_fixFamily`),
  within which a call or an invocation reaches the approximant, so
  recursion through function values is covered as calls are. A function
  without a theorem means its contract summary in `M`, as its callers
  assume; `M` takes the native meanings as parameters.
- Agreement (assumed, the cycle axiom restated for the unit): every
  outcome of a function's prophetic big-step run is, with ghost contents
  and ghost memory erased, the erasure of an outcome of `M`. On a unit
  without ghost declarations erasure is the identity and `M` is not built.

## Validity slots of intrinsic maps

An intrinsic map binding a validity role, and its iterator enums, get a
synthesized ghost slot (`$validity`, spelled `_validity` in VerexSL). The
roles' contracts state it: creation chooses any value; a structural
mutation (`add`, `remove`, `pop`, …) leaves the exit's slot unconstrained
(`∃ stamp, exit = restamp (op …) stamp`); a value write (`borrow_mut`,
`iter_borrow_mut`, `upsert` at a present key) keeps it. Owners of other
layouts (wrappers, sorted views) keep their slot as a field.

- `spec_iter_preserved(new, old)` is slot equality; `spec_iter_valid` and
  `spec_leaf_iter_valid` equate the iterator's slot with the map's. An
  iterator's slot is read by its distance from the end of its fields: the
  slots follow every variant's fields alike, so the read needs no variant.
- In a specification, `==` between two owners holding their map in their
  own layout with slots is equality but for the slots, in the form the
  role contracts state; owners without slots keep the identity projection.
- An iterator names its entry by the key it carries (a payload field of
  the key type parameter) or by a position (one integer field), as the
  Prover's `iter_key_field` classifies it; `iter_borrow_mut` and
  `spec_aborts_iter_borrow_mut` follow the flavor. A role native's declared
  `requires` clauses are owed by its callers.

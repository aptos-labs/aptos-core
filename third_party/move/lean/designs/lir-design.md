# Unified Leaner intermediate representation

## Status

**Project status: initial Move-profile vertical slice implemented; the full
roadmap remains in progress.**

This document defines a new language-neutral LIR boundary shared by Move,
Leaner source under Move and Rust profiles, and Rust MIR. It is deliberately
separate from E16, the current Move-to-Leaner transpiler milestones, and the
Rust frontend project described in
[`rust-mir-design.md`](rust-mir-design.md). Those projects consume or
migrate into LIR; completing either one is not an LIR phase and vice versa.

The existing `Move.Compiler.LIR` is not the LIR described here. It is a named
executable CFG which omits specification declarations and currently creates
trivial contracts and empty loop specifications when it lowers to
`MoveModel.IR`. This document calls that representation **named stackless IR
(NSIR)**. Treating the loss as an acceptable boundary is an architectural bug
to remove during migration.

The first executable slice now has both requested frontends and a Leaner
backend:

```text
compiler-v2 Move/XAST ------------------------------------------+
                                                                |
Leaner Move source --> elaboration --> NSIR --------------------+
        |                                                       |
        +--> retained source artifact --------------------------+
                                                                |
                                                                v
               raw Move-profile LIR --> checked structurization
                                                                |
                                                                v
                                                         validated LIR
                                                                |
                                                                v
                                                  canonical Leaner source
                                                                |
                                                                v
                                                   fresh Lean elaboration
```

The implementation is split as follows:

| Area | Current implementation | Deliberate limit of this slice |
|---|---|---|
| Neutral core | `leaner-ir/LeanerIR`: the known Move/Rust semantic union plus `Import` raw/structurization and deterministic RawUnit JSON, `Validation` diagnostics and checked boundaries, `Interpreter` execution, and `Proofs` soundness, alongside strong IDs, tables, declarations, canonical namespace uniqueness, qualified declaration ownership/uniqueness, core Unicode-scalar character/vector/reference/lifetime/function/ability/trait/implementation/global-storage structure, typed pure primitives, structured attributes, explicit call/throw nodes, structured expression/pattern/place arenas, contracts, provenance/evidence, nominal field/downcast places, recursive structural generic substitution through interned arena types, resolved and constrained nominal type uses, core `Copy`/`Drop`/`Store`/`Key` satisfaction for instantiations, operations, and nominal declarations, checked constructor-pattern and loop-control/fallthrough result typing, resolved typed constant declarations/uses, and profile-independent intrinsic owner/target/duplicate graph checks | Trait/evidence satisfaction, lifetime-constraint solving, full semantic type checking, canonicalization, and richer indexes are not complete |
| Move profile | `leaner-move/LeanerMove`: Move reference/effect rules and policy for the shared IR, plus a closed 62-role map-intrinsic registry with model, role-kind, owner-shape, required-role, dependency, and exact physical-signature checks | Intrinsic semantic interpretation and deeper borrow/lifetime and effect rules remain |
| Move frontend | `Transpiler.LIR.Encode`: the existing typed compiler-v2 XAST is converted immediately to raw LIR | This is the transitional typed-XAST producer, not yet a direct pre-typecheck Move producer |
| Leaner frontend | `LeanerLang.Lower` lowers the canonical profile-selected surface directly to `RawUnit`; the legacy `Transpiler.LIR.Leaner` bridge also imports whole registered Move-profile namespaces from executable NSIR plus retained parsed source | Direct surface coverage is substantial but not complete; the legacy bridge still imports source specifications construct by construct and records unsupported semantic items as explicit untrusted evidence |
| Leaner backend | `LeanerLang.Print` renders `ValidatedUnit` directly with one integrated width-80 layout engine and no frontend/printer-package projection | Some validated semantic families remain explicit capability errors, and Move/Leaner-native semantic alpha-equivalence and source-map coverage are not yet corpus-wide |
| Validation | Core and Move-profile tests, Move-to-LIR printer regressions, a broad Leaner corpus, loop/reference normalization tests, and live `Leaner → LIR → Leaner` re-elaboration; the canonical loop fixture also compares shared-interpreter outcomes before and after re-import | The live semantic roundtrip currently uses representative modules rather than alpha-equivalence over the complete corpus |

The Leaner source bridge currently preserves executable functions, structures
and enums, generics and Move abilities, constants, supported specification
functions, module axioms, and function contracts including `requires`,
`ensures`, `aborts_if`, frames, and pragmas. It recovers structured diamonds
and natural loops from NSIR, including loops with terminal return/abort exits.
Single-definition reference temporaries are normalized back to their authored
borrow paths so generated nested and global references elaborate correctly.
Reference types and lifetime identities are core LIR nodes rather than opaque
Move payloads. Both current Move frontends create explicit `inference`
lifetimes for elided source lifetimes; generating and solving their constraints
is a later semantic-checking pass.

This is an end-to-end implementation of the first combination, not completion
of every promise in the target architecture below. In particular, a raw unit
can still rely on upstream Lean/compiler-v2 typing, and some source constructs
are carried as named untrusted evidence instead of executable LIR semantics.
No such construct is silently treated as checked.

## Objective and boundary invariant

LIR is the complete, source-independent semantic representation of a checked
Leaner compilation unit:

```text
Move source -----------+
Leaner source ---------+----> raw LIR ----> validated LIR
Rust crate/MIR import -+                         |
                                                 +--> Move source
                                                 +--> Rust source
                                                 +--> profile-selected Leaner source
                                                 +--> profile-specific execution
                                                 +--> verification IR / obligations
```

All frontend paths for the same semantic profile must produce semantically
equivalent LIR for equivalent programs. Source backends must emit a
semantically equivalent program in a compatible target profile. Executable
lowering may erase proof-only declarations, but that lowering is not LIR and
is never the input to source analysis, validation, or reporting.

The central rule, shared with the Rust frontend, is:

> Function origin is metadata; function meaning is semantic.

Origin records whether a body came from Lean elaboration, compiler-v2, rustc,
or generated source. A separate semantic profile records whether its integers,
references, storage, destruction, and throws have Move or Rust
meaning. No semantic pass dispatches on origin.

The second central rule is:

> LIR is the union of known Move and Rust semantics, not merely their
> intersection. Profiles extend that union; they do not hide known nodes.

This does not imply that every source program reaches LIR or that every source
backend can print every valid LIR unit. Frontends and backends are explicitly
partial:

- a frontend rejects a source construct it cannot lower faithfully;
- a backend publishes a capability set and rejects a valid LIR construct it
  cannot represent;
- neither side encodes a known Move/Rust construct as an opaque profile value
  merely to avoid extending the core schema;
- `ProfileValue` is reserved for future or third-party semantics outside the
  currently known union.

The non-negotiable ownership rules are:

- frontend adapters only decode/expand source-specific forms and construct raw
  LIR plus explicit upstream assumptions or alignment evidence;
- authoritative name resolution, type checking, semantic analysis, and
  backend-independent diagnostics operate on LIR;
- backend capability checks and reports operate on validated LIR;
- every backend consumes validated LIR;
- no analysis consults XAST, Lean syntax, Lean environment extensions, or a
  compiler-v2 model after raw LIR construction;
- no backend silently drops or weakens a semantic LIR node;
- source locations are LIR metadata identified uniformly by `LocId`.

“One analysis point” means one public checked-construction boundary,
`LeanerIR.Validation.validate : ProfileRegistry → RawUnit →
Except (Array Diagnostic) ValidatedUnit`. Its implementation
is a pipeline of small LIR passes, not one monolithic function.

## Project ownership and dependencies

LIR must not be owned by a source frontend, transpiler, verifier, or bytecode
backend:

```text
third_party/move/lean/
  leaner-ir/          # neutral core; LeanerIR.{Import,Validation,Interpreter,Proofs}
  leaner-move/        # Move semantic profile; no frontend dependency
  leaner-rust/        # Rust profile, Rustc Public exporter, RawUnit adapter
  leaner-e2e-tests/    # Move/Rust-to-LeanerLang source baselines
  move-model/          # stackless/verification IR and semantics
  move/                # Leaner frontends, verification UI, backends
  transpiler/          # temporary XAST adapter and backend driver
```

`leaner-ir` depends only on Lean base libraries. `move`, `transpiler`,
`move-model`, and `leaner-rust` may depend on it; it must not depend on them.
The neutral raw-LIR serializer owns the transport schema. XAST and the Rustc
Public adapter mechanically construct `RawUnit` documents with source-specific
provenance and import evidence; neither owns semantic policy.

Concrete profile libraries depend on `leaner-ir` and provide schemas,
well-formedness rules, semantic interpretations, and capability definitions.
They must not depend on a frontend AST. Applications assemble a
`ProfileRegistry` and still enter through the single
`LeanerIR.Validation.validate`
pipeline.

The package owns:

- raw, resolved, typed, and validated LIR data;
- strong identifiers, interning, and checked lookup;
- source-independent name resolution and type checking;
- canonicalization and alpha-equivalence;
- backend-independent validation and typed diagnostics;
- derived unit indexes used by analyses and backends;
- semantic-profile registration and profile-independent checking;
- backend feature queries and the data used to build reports;
- deterministic, versioned raw-LIR serialization.

Frontends own source-language parsing, macro expansion, and raw-LIR
construction. They may rely on upstream language admission such as rustc trait
and borrow checking, but every relied-upon fact must be represented as an LIR
hint, certificate, or named trust assumption. Backends own representation
choices and explicit capability checks only.

## Semantic data and provenance

LIR separates three kinds of data without leaving them in frontend-owned side
tables:

- **Semantic data** affects execution or proof meaning: declarations, types,
  bodies, conditions, frames, state anchors, intrinsic bindings, visibility,
  abilities, and language-defined pragmas.
- **Provenance data** affects diagnostics or source reconstruction: source
  locations, comments, documentation formatting, authored spellings, optional
  surface notation, and generated-node reasons.
- **Alignment evidence** does not change the denotation of an LIR body, but
  controls what external claim a theorem supports: producer/schema versions,
  source hashes, target configuration, upstream checking status, imported CFG,
  structurization witness, semantic hashes, and trusted assumptions.

Semantic data is never dropped. A backend may omit provenance only if its
contract explicitly disclaims source-quality reconstruction. The Move and
Leaner source backends retain it. Alignment evidence may be erased for local
reasoning about an LIR function, but must be present and checked before a
theorem is presented as a claim about Move source, Rust source, or imported
MIR.

### Import receipts and source alignment

A serialized `RawUnit` is untrusted transport data, even when the exchange is
an ephemeral file private to the build. The target import gate is owned by Lean:
it invokes the selected Move, Leaner, or Rust producer, consumes that
producer's output, and validates the resulting raw LIR. Privacy is an
implementation detail which prevents accidental substitution; it is not a
proof that the translation is correct.

The import boundary distinguishes three claims:

- **upstream checks** record source-language admission, such as compiler-v2
  type checking or rustc type, trait, and borrow checking;
- an **import receipt**, constructed by the Lean-owned orchestration rather
  than asserted by the exchange file, binds producer version, invocation
  configuration, input identities/hashes, and output semantic hash; and
- **alignment** either proves that the imported LIR represents the source
  artifact or names the frontend translation as an explicit trusted
  assumption.

These rules apply equally to Move, Leaner, and Rust inputs. An in-process
frontend may avoid the serialized file and freshness receipt, but it does not
avoid its alignment obligation. Likewise, upstream type checking does not by
itself prove that the frontend preserved source meaning. LIR validation proves
well-formed internal semantics; a source-level theorem additionally requires
alignment, and a claim about emitted code additionally requires backend
alignment.

The initial implementation's `Alignment.trust` value and description are
descriptive metadata: validation currently checks their references, not the
claimed correspondence or source freshness. Until the import-receipt gate and
alignment checks exist, a frontend-produced `checked` label must be reported
as a producer assertion rather than kernel-checked evidence.

Semantic equivalence deliberately erases provenance:

```text
semanticEquivalent(a, b) :=
  alphaEquivalent(
    canonicalize(semanticProjection(a)),
    canonicalize(semanticProjection(b)))
```

`semanticProjection` erases no executable or specification meaning.
Provenance fidelity is tested separately.

## Representation stages

The target “LIR” family has explicit construction states:

```text
RawUnit --normalize/structurize--> StructuredUnit --resolve--> ResolvedUnit
        --infer/check types--> TypedUnit --validate/index----> ValidatedUnit
```

- `RawUnit` is the only frontend output. It contains declarations, name and
  type spellings, structured bodies or profile-tagged CFG bodies with neutral
  control, specifications,
  profile configurations, provenance, alignment evidence, dependency
  interfaces, and optional frontend-supplied resolution/type hints.
- `StructuredUnit` has one structured body language. An LIR structurization
  pass converts reducible CFG bodies such as imported MIR and records a checked
  block/edge correspondence. Unsupported CFG shapes are diagnosed here.
- `ResolvedUnit` replaces semantic name uses with checked declaration or
  dependency IDs while retaining authored spellings as provenance.
- `TypedUnit` assigns a checked LIR type to every expression, pattern, local,
  parameter, result, and explicit type argument.
- `ValidatedUnit` adds derived indexes and certifies all backend-independent
  well-formedness rules. It is the only input to backends and most analyses.

In the target API, only `RawUnit` and `ValidatedUnit` are public construction
boundaries. The constructors of the intermediate and validated forms are
private. Frontend resolution/type information is a hint which the shared
checker verifies; a disagreement is an LIR diagnostic.

The initial implementation exposes `RawUnit` and `ValidatedUnit` and performs
structurization plus structural/profile validation in one checked call. It
does not yet materialize separate `StructuredUnit`, `ResolvedUnit`, or
`TypedUnit` values. Its inputs already contain typed IDs from compiler-v2 or
Lean elaboration; those upstream facts are recorded in import evidence while
the shared checker independently checks table safety, arena acyclicity,
profile vocabulary, declaration shape, and CFG structure. Splitting out
authoritative resolution and semantic type checking is remaining work, not a
second frontend path.

This staging keeps MIR graph analysis inside LIR. The Rustc Public producer
mechanically exports MIR-shaped raw nodes; it does not own the canonical
structurizer, loop identities, or diagnostics used by downstream tools.
Charon may emit the same raw format only for differential testing or a
labelled temporary experiment adapter.

The raw boundary preserves MIR administrative statements
(`StorageLive`/`StorageDead`, deinitialization, discriminant changes, retags,
place mentions, and user-type ascriptions) and MIR call/drop/assert/cleanup
terminators. The shared structurizer now classifies storage lifetime markers,
place mentions, and user-type ascriptions as inert compiler administration and
erases them after validation. Under the initial `panic=abort` Rust profile it
also lowers ordinary bounds, overflow, division, and remainder assertions to
explicit panic branches. Destructive/provenance-sensitive statements, cleanup
edges, and exceptional terminators remain stable rejection boundaries.

## Compilation units and dependencies

The checked object is a compilation unit, not an isolated module. Calls,
friends, abilities, specifications, and intrinsic bindings can refer across
module boundaries:

```lean
structure RawUnit where
  version : Version
  tables : Tables
  profiles : Array ProfileConfig
  namespaces : Array RawNamespace
  dependencies : Array RawNamespaceInterface
  evidence : Array ImportEvidence

structure ValidatedUnit where
  tables : Tables
  profiles : Array ValidatedProfile
  namespaces : Array ValidatedNamespace
  dependencies : Array ValidatedNamespaceInterface
  indexes : UnitIndexes
```

A namespace is a language-neutral hierarchical declaration container. A Move
module, Rust crate/module, and generated Leaner namespace are profile-specific
views of it; the core does not bake Move address/module identity into every
declaration.

There is exactly one interned `Tables` snapshot per compilation unit. It
contains identities referenced by owned namespaces and dependency interfaces;
it is not duplicated per namespace. Owned namespaces occupy the leading
namespace-table positions so their dense `NamespaceId`s continue to index the
owned namespace array, while dependency-only namespaces may follow. Expression,
pattern, place, and local arenas remain namespace/declaration owned.

A dependency interface is itself LIR data. It contains exported declarations,
types and abilities, signatures, visible contracts and specification
functions, profile metadata, intrinsic declarations, and locations. It is not
a pointer into a Lean environment, compiler-v2 model, or rustc context. Full
dependency namespaces may be provided for analyses requiring bodies. An
analysis reports explicitly when an interface lacks required information.

## Semantic profiles and extension points, not source dialect ASTs

One structured core does not imply one accidental semantics. Known semantic
families are first-class, while numeric IDs are reserved for extensions:

```lean
inductive Profile where
  | move
  | rust
  | extension (id : ProfileId)
```

The unit carries at most one configuration for each `Profile`. Move and Rust
configurations are resolved by their constructors, never by conventional
array positions. Only `extension id` is positional: its `ProfileId` must name
the corresponding configuration-table entry. Checker and semantic registries
are keyed by `Profile` and reject duplicate implementations.

Each function has a `Profile`; origin is stored separately:

```lean
structure FunctionArtifact where
  id : FunctionId
  profile : Profile
  signature : Signature
  body : FunctionBody
  origin : OriginId
  alignment : AlignmentId
```

The core fixes the known Move/Rust declaration, type, ability, trait,
implementation, attribute, place, control, call, contract, location,
diagnostic, and outcome union. A
registered semantic profile selects the language rules for those nodes—for
example Move rollback versus Rust panic cleanup—and defines only additional
types, operations, state components, effects, outcomes, and capabilities that
fall outside the known union:

```lean
structure ProfileSchema where
  profile : Profile
  typeSchema : TypeSchema
  operationSchema : OperationSchema
  stateSchema : StateSchema
  outcomeSchema : OutcomeSchema
  checkReference : ReferenceType → Array Diagnostic
  checkOperation : TypedOperationContext → Array Diagnostic
  checkIntrinsic : RawUnit → RawNamespace → IntrinsicDecl → Array Diagnostic
  interpretation : InterpretationId
```

Extension wire values use stable `(profile, tag, payload)` encodings.
Validation decodes them through `ProfileRegistry` into closed checked profile
values. Structural links to LIR nodes use typed ID fields; they must not be
hidden in profile payload strings. The initial Move adapter still has known
Move constructs encoded as tags; those are migration debt, not the intended
profile contract. References, lifetimes, vectors, function values, abilities,
calls, throws, and attributes are already promoted examples.
Profiles may not add a second control-flow tree, contract representation,
location mechanism, or reporting path.

The profile family is planned as follows; only the Move vocabulary and checker
exist in the initial implementation:

- **Move:** Move rules for core abilities/references/abort plus signer/resource
  types, global resource/event state, and intrinsic maps not yet promoted;
- **Rust:** Rust rules for references/drop/panic plus initializedness, partial
  moves, slices, loan facts, allocation/cell state, and cleanup details not yet
  promoted.

Traits, implementations, associated items, and evidence passing are core LIR,
not Rust-profile extensions. This lets a future Move trait feature use the
same declarations, calls, interpreter interface, contracts, and proof rules.
Profiles may impose source-language admission rules or backend capability
limits, but they may not substitute a different trait representation.

The first Rust slice freezes the declaration, predicate, and associated-item
shape but does not require checked implementation-selection evidence.
`EvidenceId` remains an opaque generic-argument slot; the checked core
evidence table and dictionary semantics are a later compatible schema step,
before trait calls are interpreted or used to transfer proofs.

The independent LIR project migrates and validates the Move-profile library.
The `leaner-rust` package owns the Rust-profile implementation against the
frozen interface. Leaner source and compiler-v2/XAST may construct the Move
profile; the same Leaner language and imported MIR may construct the Rust
profile. Thus the same-origin frontend can construct different profiles, while
different-origin frontends can construct exactly the same profile semantics.

A direct call across incompatible profiles is invalid. Interoperation requires
an explicit, validated boundary adapter defining type conversion, state/effect
projection, and normal/exceptional outcome translation. In particular, a Rust
panic is never printed or interpreted as a Move abort.

## Required language coverage

An LIR unit includes the union of known constructs accepted from the supported
Move and Rust frontends plus registered extensions outside that union:

- hierarchical namespaces, package/crate/module identities, imports, friends,
  structured attributes (including pragma surface placement), docs, and comments;
- constants;
- structures and enums, including type/const/lifetime parameters, fields,
  variants, invariants, Move abilities, trait/implementation declarations, and
  intrinsic/model declarations;
- executable functions, including visibility, entry/native/inline state,
  semantic profile, parameters, results, body, places, effects, attributes,
  contracts, and loop specs;
- specification functions, including uninterpreted/native state, authored
  physical signatures, bodies, attached clauses, and executable-function
  relationships;
- specification variables;
- namespace axioms and global/update invariants;
- in-body specification assertions and assumptions;
- structured executable and specification expressions and patterns;
- raw reducible CFG bodies plus checked structurization evidence at the import
  stage;
- explicit moves, copies, borrows, reads, writes, drops, assertions, function,
  constructor/destructor, closure/invoke calls, returns, and argument-carrying
  throws;
- explicit state anchors for `old`, loop invariants, and program-point specs;
- normalized intrinsic owners and role-to-target bindings;
- generic declarations, trait/implementation declarations, associated items,
  and explicit evidence values;
- source files, macro/expansion origins, qualified names, freshness metadata,
  producer configuration, and alignment/trust evidence.

Lean proofs and arbitrary Lean helper declarations outside a registered
Leaner namespace are not source-language semantics and remain outside LIR.
Their connection to verification obligations is a backend/UI concern.

Phase 0 must produce a coverage matrix mapping every XAST variant, every
Leaner semantic environment extension, and every MIR-exchange node in the
initial Rust subset to an LIR node, profile extension, import-evidence field,
or reviewed exclusion. A construct may not survive only in a frontend side
table.

## Strong identifiers, tables, and arenas

Identifiers are distinct structures rather than interchangeable `Nat`
aliases:

```lean
structure FileId where index : Nat
structure LocId where index : Nat
structure OriginId where index : Nat
structure AlignmentId where index : Nat
structure ProfileId where index : Nat -- extension profiles only
structure TypeId where index : Nat
structure NamespaceId where index : Nat
structure NameId where index : Nat
structure TypeDeclId where index : Nat
structure TraitDeclId where index : Nat
structure ImplDeclId where index : Nat
structure AssociatedItemId where index : Nat
structure FunctionId where index : Nat
structure SpecFunctionId where index : Nat
structure SpecVarId where index : Nat
structure LocalId where index : Nat
structure PlaceId where index : Nat
structure LifetimeId where index : Nat
structure EvidenceId where index : Nat
structure ExprId where index : Nat
structure PatternId where index : Nat
```

Every table has checked lookup returning an `Except Diagnostic`. Unchecked
lookup is internal and usable only after table validation. IDs are stable
within a unit snapshot. Canonicalization does not reorder declaration or arena
tables. A compaction pass, if ever needed, returns and applies one checked
old-to-new remapping.

Types, locations, module references, and qualified names are interned once per
compilation unit. Types are interned bottom-up; recursive data types refer
through declaration IDs.

An interned type has no unique source range because one type entry can occur
at many authored sites. Type occurrences therefore use a located reference:

```lean
structure TypeUse where
  type : TypeId
  loc : LocId

structure Tables where
  files : Array SourceFile
  locations : Array Location
  origins : Array Origin
  lifetimes : Array Lifetime
  types : Array Type
  namespaces : Array NamespaceRef
  names : Array QualifiedName
```

Signatures, fields, locals, and casts use `TypeUse`. Generic call and
constructor instantiations use `GenericArgument`, whose `.typeArg` case carries
a `TypeUse`. The interned `Type` table is semantic and location-free.

The typed namespace shape is finite and deterministic:

```lean
structure TypedNamespace where
  loc : LocId
  identity : NamespaceRef
  profile : Option Profile
  doc : String
  expressions : Array Expr
  patterns : Array Pattern
  places : Array Place
  imports : Array NamespaceId
  profileMetadata : ProfileMetadata
  attributes : Array Attribute
  pragmas : Array Attribute  -- same node, different source/backend placement
  constants : Array ConstantDecl
  structs : Array StructDecl
  associatedItems : Array AssociatedItemDecl
  traits : Array TraitDecl
  implementations : Array ImplDecl
  functions : Array FunctionDecl
  specFunctions : Array SpecFunctionDecl
  specVars : Array SpecVarDecl
  invariants : Array ModuleInvariant
  intrinsics : Array IntrinsicDecl
  comments : Array Comment
```

`TraitDecl` contains generic binders, super-predicates, associated types and
constants, and method declarations with optional default bodies. `ImplDecl`
contains its trait application, target type, generic predicates, and
associated-item bindings to method bodies. These are
ordinary core declaration tables: they are not profile payloads and may be
referenced from another namespace through checked IDs.

Declaration arrays retain source order. Cross-module references use checked
namespace/name IDs, not environment-global numbers.

## Types, places, operations, and structured bodies

The core type language contains the known Move/Rust union. Semantic profiles
select rules for these nodes; only unknown extensions use a profile type:

```lean
inductive Type where
  | unit | never
  | bool | character
  | integer (width : IntWidth) (signed : Bool)
  | tuple (elements : Array TypeId)
  | vector (element : TypeId) (length : Option ConstValue := none)
  | nominal (name : NameId) (arguments : Array GenericArgument)
  | function (arguments : Array TypeId) (result : TypeId)
      (abilities : Array Ability)
  | typeParameter (index : Nat)
  | reference (value : ReferenceType)
  | profile (value : ProfileValue)

inductive Ability where
  | copy | drop | store | key

structure TraitRef where
  trait : QualifiedRef
  arguments : Array GenericArgument

inductive GenericPredicate where
  | ability (type : TypeId) (ability : Ability)
  | implements (type : TypeId) (trait : TraitRef)
  | associatedTypeEq (trait : TraitRef) (name : NameId) (value : TypeId)
  | associatedConstEq (trait : TraitRef) (name : NameId) (value : ConstValue)
  | lifetimeOutlives (longer shorter : LifetimeId)
  | constEq (left right : ConstValue)

inductive Attribute where
  | call (name : String) (arguments : Array Attribute) (loc : Option LocId)
  | assign (name : String) (value : AttributeValue) (loc : Option LocId)

structure ReferenceType where
  profile : Profile
  kind : ReferenceKind
  referent : TypeId
  lifetime : LifetimeId

structure Lifetime where
  kind : LifetimeKind
  loc : LocId
  name : Option String
```

Core vectors carry their element `TypeId` directly. `length = none` represents
Move `vector<T>` and Rust `Vec<T>`; `some n` represents a fixed Rust array.
Function-value abilities, generic predicates, structure abilities, trait
identities, implementations, and associated-item bindings are structural LIR
data. Profiles define their language-specific source rules rather than
replacing them with tags.

Pure tuple/vector operations, indexing, Boolean operations, comparisons, and
integer arithmetic are also core nodes. Fixed-width `add`, `subtract`, and
related ordinary arithmetic is modular. The corresponding `checked*` node
carries its failure `ThrowKind`, so Move emits checked arithmetic with
`abort`, while a Rust MIR frontend can preserve either checked `panic`
behavior or an ordinary modular operation without consulting a profile hook.
Rustc's tuple-producing checked binary rvalues use separate overflowing nodes
which return the modular value and an explicit Boolean flag.
Characters are target-independent Unicode scalar values with a distinct core
type and constant constructor; scalar validation excludes surrogate and
out-of-range code points, and ordering compares scalar values without treating
characters as integers. Literal and range patterns use the same scalar typing
and ordering rules.
Bitwise, shift, cast, and value-level ownership operations use the same typed
primitive family. Unsupported static obligations remain explicit capability
diagnostics, not profile-string dispatch.
Value-level borrow, dereference, and freeze use a typed reference-operation
family until checked place normalization can select the stronger place nodes.
Field/variant operations carry strong qualified references in a typed data
family. Enum variants may record their observable integer discriminant, which
is distinct from variant-array position; the value-producing discriminant
operation is shared by Rust and any future source profile exposing numeric
tags. Range and the common logical connectives are core primitives.

The specification language is also part of the common core. Contract clauses
(`requires`, `ensures`, abort conditions, invariants, axioms, and related
roles) and the `forall`/`exists`/choice quantifiers are typed nodes rather than
Move profile strings. The Rust frontend is expected to use this same logical
vocabulary and Move-Prover-style verification pipeline.
Its logical type vocabulary is shared too: mathematical integers, ranges,
event state, and type/resource/state domains are structural core types rather
than profile payloads.
Logical expression operations use the closed `SpecOperation` family.
Specification-function targets are strong qualified references, while result
indexes, state anchors, trace kinds, memory ranges, and integer widths are
typed payloads. This covers the shared Move/Rust specification vocabulary;
`Operation.profile` remains only for extensions outside that known union.
Function-value summaries are represented by the closed `BehaviorKind` family
(`requiresOf`, `abortsOf`, `ensuresOf`, `resultOf`, `unchangedOf`, `foldsOf`,
and indexed `writeOf`) plus an explicit pre/post `MemoryRange`. Their first
operand has function type; validation checks its projected arguments, packed
result slots, mutable-reference post-state slots, and result type rather than
treating the operation as an untyped backend escape.

The core also owns reference topology: shared/mutable mode, the referent type,
and an explicit lifetime identity. The attached profile owns the rules
governing aliasing, validity, storage, escape, and operational behavior. The
Move `address` and `signer` are first-class core types, and address literals
are core constants. The Move profile supplies reference rules and abort
rollback policy; it has no residual type or operation tags. The Rust profile
supplies Rust reference rules and raw-pointer
exclusions plus extension forms not yet promoted. Thus a structural edge can
be core even when its observable rules differ: `ReferenceType.profile` keeps
that difference explicit.

Move source with an elided lifetime produces a fresh core `inference`
lifetime. A future Move lifetime syntax maps authored binders to `parameter`
entries in the same table, so adding that syntax does not require another LIR
schema migration. Validation currently checks lifetime and referent IDs;
lifetime constraints, inference, and profile-specific borrow checking are
separate checked passes and are not yet implemented.

Places are first-class because both lowerings need them and Rust moves/borrows
cannot be represented faithfully as value-only expressions:

```lean
inductive Place where
  | local (local : LocalId)
  | deref (base : PlaceId)
  | field (base : PlaceId) (field : NameId)
  | index (base : PlaceId) (index : ExprId)
  | subslice (base : PlaceId) (start stop : Nat) (fromEnd : Bool)
  | downcast (base : PlaceId) (variant : NameId)
```

Operations distinguish `move`, `copy`, borrow kinds, read, write, assert, and
drop. `GlobalKind` makes keyed storage first-class with `contains`, `borrow`,
`take`, and `publish`; the first operation type instantiation identifies the
resource family. These correspond to Move's `existsAt`, `borrowGlobal`,
`moveFrom`, and `moveTo`, while retaining names that are not source-language
keywords. Global references use the same projection and mutation machinery as
heap references. `CallKind` separately distinguishes direct function calls,
constructor packing, constructor destruction/unpacking, closure packing,
invocation of a callable expression, and an extension case. The callee
declaration supplies its semantic profile. `throw` is structured control
rather than a primitive operation and carries an argument array, so both
`abort 24` and richer panic or abort payloads are representable. Arithmetic
carries its actual checked, wrapping, aborting, or panicking behavior rather
than inheriting it from surface spelling. Global storage is already core;
remaining known Move event and Rust cell/drop operations must likewise be
promoted. Profile operations are only the extension fallback while that
migration is incomplete.

An operation's optional `SurfaceSyntax` is typed provenance, not semantics.
The core names receiver-call and index notation explicitly; an unknown future
notation uses a profile-checked extension rather than an unchecked string.

Structured, typed bodies are canonical. A stackless CFG is a derived lowering:

```lean
structure Expr where
  loc : LocId
  type : TypeId
  kind : ExprKind

structure Pattern where
  loc : LocId
  type : TypeId
  kind : PatternKind
```

Children are `ExprId`/`PatternId` references into namespace-owned arenas. The
checker rejects invalid IDs and cycles. Function and spec-function bodies are
root expression IDs.

The expression union covers values, constants, locals, parameters, resolved
operations and calls, blocks, conditionals, matches, sequences, loops,
break/continue, returns, argument-carrying throws, assignments,
quantification, spec operations, and in-body spec blocks. Calls carry explicit
type/const/evidence instantiations.
Patterns cover variables, wildcards, tuples, structure/enum destructuring,
literals, and ranges.

Receiver calls and index notation are optional provenance on the normalized
operation, not separate semantics. A validated structured body can lower to a
profile-specific executable IR when such a backend exists. The inverse from a
lower executable IR is intentionally not required.

## Generics, evidence, ownership, and destruction

Generic declarations use one common binder model with first-class abilities
and an extension-predicate tail:

```lean
inductive GenericArgument where
  | typeArg (value : TypeUse)
  | const (value : ConstValue)
  | lifetime (value : LifetimeId)
  | evidence (value : EvidenceId)
```

Move ability constraints and trait/lifetime/const predicates inhabit the same
declaration/signature positions. `copy`, `drop`, `store`, `key`, trait
identities, implementation declarations, associated-item bindings, and
evidence arguments are core variants; source-level admission rules remain
profile specific. In the completed design, trait method calls use explicit
evidence records; neither LIR nor generated source silently relies on rustc, a
future Move trait solver, or Lean instance search to recover a selected
implementation. Imported associated-type/constant normalization and concrete
implementation-selection evidence will then be checked against the call-site
arguments. The first Rust exchange version stops before that evidence check.

A trait declaration contains its generic binders, super-predicates, associated
types/constants, and method signatures (with an optional default body). An
implementation declaration names the trait application and target type,
provides its predicates and associated-item bindings to method bodies. A later
core evidence table gives validated implementation selections stable
identities and checks that they refer to well-formed trait/implementation
applications. The core does not reimplement Rust coherence or a future Move
solver: an importer records the upstream admission result as alignment
evidence, while the eventual interpreter and verifier consume explicit core
selection evidence.

Import never monomorphizes a generic declaration. `RawUnit` and
`ValidatedUnit` contain one generic body with binders and predicates. Once the
evidence layer is enabled, an interpreted call additionally supplies
type/const/lifetime arguments and evidence without creating a cloned body. A
compiler-produced concrete instance
may be recorded only as provenance for alignment to a source call, never as a
replacement body or a mandatory manifest in LIR. If Rustc Public cannot expose
the generic body needed for this translation, the exporter spike is blocked
rather than accepting a monomorphized-only import.

Proof-facing bodies remain generic. Profile-specific executable lowering may
produce a separate concrete artifact only after generic interpretation and a
checked instantiation relation are defined; it never alters or replaces the
LIR unit. Type-, layout-, impl-, or const-dependent behavior may use explicit
equivalence classes or finite covered instances; it is never generalized by
dropping the dependency.

Generic parameter indexes are scoped by the declaration binder list and are
kind-checked: a type-parameter node must name a `.typeArg` binder, and lifetime
parameter nodes must name `.lifetime` binders. A trait method signature sees
the owning trait binders followed by its own binders. Value parameters carry
no `LocalId`; executable and specification-function parameters correspond
positionally to the leading local declarations, while a bodyless trait method
signature requires no synthetic local table.

Rust initializedness, partial moves, loan facts, drop flags, and cleanup edges
are not provenance. Their observable effects become typed places/operations or
Rust-profile state; upstream rustc facts which justify accepting them are
alignment evidence. Reference-bearing ADTs and returned-region relationships
must survive signatures and bodies. Dynamic interior mutability uses explicit
identity-bearing cell state and observable guard drop, not ordinary
prophecy references disguised as a common operation.

## Specifications and state anchors

Specifications are first-class LIR declarations and nodes:

```lean
structure SpecBlock where
  loc : LocId
  pragmas : Array Attribute
  conditions : Array Condition
  frame : Option Frame

structure Condition where
  loc : LocId
  kind : ConditionKind
  properties : Array Attribute
  expression : ExprId
  payload : ConditionPayload
```

`ConditionKind` covers `requires`, normal `ensures`, typed
`exceptional_if/with`, `succeeds_if`, `assert`, `assume`, `decreases`, effects,
updates, structure and function invariants, loop invariants, global/update
invariants, and axioms. Move `aborts_if` and Rust `panics_if` are profile
surface forms of distinct exceptional-outcome tags; they do not share rollback
semantics. Kind-specific operands use a typed payload rather than positional
additional expressions.

State observations are explicit:

```lean
inductive StateAnchor where
  | functionEntry (function : FunctionId)
  | blockEntry (block : ExprId)
  | loopEntry (loop : ExprId)
  | programPoint (expression : ExprId)
```

A loop invariant records its loop `ExprId` and `.loopEntry` anchor. An in-body
assertion or assumption records its `.programPoint` anchor. A backend never
infers the anchor from print position.

Specification functions retain their authored signatures even when a logical
intrinsic interprets applications generically:

```lean
structure SpecFunctionDecl where
  loc : LocId
  name : NameId
  typeParameters : Array TypeParameter
  parameters : Array Parameter
  result : TypeUse
  uninterpreted : Bool
  native : Bool
  executableFunction : Option FunctionId
  usesOld : Bool
  body : Option ExprId
  spec : SpecBlock
  attributes : Array Attribute
```

This retained physical signature is what lets the shared checker reject a
replacement whose signature does not match its declared intrinsic role.
Contracts and loops remain clause-preserving. Conjunctions and profile
verification forms such as `MoveModel.IR.SpecExp` are derived representations.

The common relational outcome distinguishes normal return, a typed throw with
its semantic-profile state behavior, undefined/unsupported behavior, and
divergence. Move instantiates `throw abort` with transaction rollback; Rust
instantiates `throw panic` with retained prior mutation and its configured
terminal/cleanup behavior. Framing and effect clauses use the same profile
state/effect schema as executable operations.

## Locations and diagnostics

Locations are interned and never represented by a magic ID. One entry can
retain a primary span, related spans, macro/desugaring context, and a generated
parent:

```lean
structure SourceRange where
  file : FileId
  start : Nat
  stop : Nat

structure Location where
  primary : Option SourceRange
  related : Array SourceRange
  expansion : Array SourceRange
  generatedBy : Option PassName
  parent : Option LocId

structure SourceFile where
  name : String
  contentHash : String

structure Origin where
  producer : ProducerOrigin
  location : LocId
  sourceIdentity : Option String
```

Every authored node carries a `LocId`: declarations, attributes, pragmas,
located type uses, fields, variants, expressions, patterns, conditions,
intrinsic bindings, and comments. Generated nodes record their pass and parent;
the parent identifies the authored cause. A root with no source has
`primary := none`, an explicit producer, and no parent. Derived lower
instructions retain the location of the LIR node which produced them. Inlining
uses call-site primary plus callee-definition related locations.

Diagnostics are typed data, not formatted strings:

```lean
structure Diagnostic where
  severity : Severity
  code : DiagnosticCode
  primary : LocId
  related : Array (LocId × String)
  data : DiagnosticData
```

Messages are formatted only at a CLI/editor boundary. Stable codes and
payloads make CLI output, VS Code diagnostics, tests, and reports agree.
Duplicate diagnostics point at the later occurrence and relate the first.
Independent diagnostics accumulate in deterministic source order; checking
stops early only when table safety prevents traversal.

Source printers produce a `GeneratedSourceMap` from generated ranges to LIR
node/location IDs. Re-elaboration records the reverse association, so an editor
can show canonical Leaner source while reporting the original `.move` or `.rs`
range. Formatting generated source never changes semantic identities.

## Intrinsic declarations and role graphs

Intrinsic metadata is first-class LIR, not a graph rediscovered from backend
attributes:

```lean
structure IntrinsicBinding where
  role : String
  target : NameId
  loc : LocId

structure IntrinsicDecl where
  model : String
  owner : NameId
  loc : LocId
  executableBindings : Array IntrinsicBinding
  specBindings : Array IntrinsicBinding
```

Raw model and role names remain strings so an older checker can diagnose a
new name at its exact location. The checker resolves them to closed semantic
types in a validated index.

Each intrinsic model has one declarative role schema:

```lean
structure RoleSchema where
  kind : TargetKind
  presence : Presence
  dependencies : Array RoleName
  signature : SignatureSchema
  interpretation : InterpretationId
```

The registered Move-profile LIR pass checks model/role vocabulary, owner shape,
target kind and namespace, required and optional roles, dependencies,
duplicate roles, shared targets, cycles where forbidden, and exact signatures.
It is invoked by the one LIR checker and uses common diagnostics. Neither
frontend, the transpiler, nor a backend owns another role registry.

The shared checker enforces the profile-independent portion of this gate:
owners resolve to nominal declarations in their namespace, executable and
specification targets resolve to the corresponding declaration kind in that
same namespace, and duplicate owners, duplicate roles, and shared targets are
rejected with related locations. The Move-profile pass now owns a closed
`MapRole` schema matching the 37 executable and 25 specification roles in the
six source fixtures. It rejects unknown models and roles, wrong target-role
categories, malformed key/value owner binders, missing required logical
roles, missing declared dependencies, and targets outside the role's exact
physical signature alternatives. The live six-owner corpus exercises all 158
bindings through this pass.

Leaner attributes such as `@[intrinsic_map]` and `@[map_spec_get (M)]` are
surface encodings. The Leaner frontend turns them into `IntrinsicDecl`; the
Leaner backend may reconstruct them after validation. The generic map carrier
is a backend interpretation selected by `InterpretationId`, not the authority
for graph consistency.

## Checked construction and analyses

The public result is:

```lean
structure ValidatedNamespace where
  namespace : TypedNamespace
  indexes : NamespaceIndexes
  intrinsicGraphs : Array ValidatedIntrinsic
  features : SemanticFeatures

structure CheckResult where
  diagnostics : Array Diagnostic
  validated : Option ValidatedUnit
```

The raw unit is not mutated. Derived indexes cover qualified names,
declaration ownership, call targets, scopes, loop identities, intrinsic roles,
and dependency links.

Analysis APIs accept LIR only:

```lean
check                : ProfileRegistry → RawUnit → CheckResult
analyzeEffects       : ValidatedUnit → EffectSummary
analyzeBorrows       : ValidatedUnit → BorrowSummary
analyzeCapabilities  : ValidatedUnit → Backend → CapabilityReport
buildTranspileReport : ValidatedUnit → CapabilityReport → Report
```

`check` composes bounds, normalization/structurization, scope/name, common and
profile type rules, declaration, body, specification, intrinsic, ownership,
ability, borrow-safety, and other backend-independent passes. Profile rules
are registered LIR passes, not callbacks into a frontend. Effect and borrow
analysis can remain separate implementation passes, but their inputs and
diagnostics are LIR.

A report is a view of typed diagnostics and capability data. It never searches
source comments, XAST, or printed output for phrases such as “dropped.” A
feature is reported missing only when an LIR node exists and the selected
backend capability query rejects it.

## Frontend contracts

### Move frontend

The final Move adapter exports source constructs into a Move-profile `RawUnit`,
retaining spellings and source ranges. Any compiler-v2 facts used downstream
are encoded as hints or import evidence rather than consulted through the
model environment.

During migration, compiler-v2 still resolves and type-checks before exporting
XAST. The XAST adapter imports that information as hints and
`LeanerIR.Validation.validate`
verifies it. Because this path cannot produce LIR diagnostics for sources
rejected before XAST export, it is explicitly transitional. The final producer
must expose enough raw LIR to run all shared downstream analysis. Native Move
parse/expansion/type diagnostics may still occur before an artifact exists;
they are compiler diagnostics, not transpilation or verification reports.

Resolved intrinsic mappings are copied mechanically. Each binding gets its
own exact location. Where XAST v4 lacks one, the adapter temporarily uses a
generated location parented by the owner pragma; the final wire producer must
add exact binding ranges.

### Leaner source frontend

The one Leaner surface language parses macro syntax, expands surface sugar,
and constructs profile-tagged `RawUnit`s. Move- and Rust-profile constructs
belong to that language rather than separate dialects; implementing the
Rust-profile forms is deferred. Lean elaboration may locate dependency
interfaces but does not select semantics from the declaration's origin.
Retained syntax is not consulted after construction.

All module semantics move into raw LIR: declarations, structured executable
bodies, contracts, spec functions, axioms/invariants, in-body assert/assume,
loop invariants, attributes, and intrinsics. A semantic construct without an
LIR representation is a located frontend construction error, never an ignored
side declaration.

#### TODO: secure Lean-source discovery and elaboration

Lean source is an active build input, not passive package data. The current
Move package/compiler integration discovers `.lean` beside `.move`, includes
it in root and `sources_deps` inputs, removes it from the normal Move parser
set in compiler v2, and invokes `lake env lean --json <source_path>` for every
discovered Lean file. Lean elaboration can execute commands, metaprograms,
host I/O, and child processes with the compiler user's privileges. As a
result, merely compiling an attacker-controlled package or transitive source
dependency can execute arbitrary host code before compilation succeeds or
fails.

This boundary must be fixed before Lean-source package discovery is treated as
a safe production compiler feature. At minimum:

- automatic discovery must not imply authorization to execute a `.lean` file;
- Lean elaboration must require an explicit opt-in and a clear trust decision
  for root sources and every source dependency, including transitive
  `sources_deps`;
- the compiler must fail closed before spawning Lean when a source is not
  authorized, and diagnostics must identify the package and file requesting
  execution;
- package manifests, lockfiles, caches, and provenance must record the exact
  authorized Lean inputs so dependency or source changes invalidate approval;
  and
- sandboxing or a deliberately restricted elaboration process should be
  evaluated as defense in depth, but must not substitute for the trust gate
  unless it actually prevents Lean metaprograms from performing host effects.

Until that work is complete, documentation and tooling must classify packages
containing Lean sources like packages containing build scripts: trusted code,
not ordinary untrusted Move dependencies. The shared `RawUnit`/LIR boundary
does not mitigate this risk because code execution happens while constructing
the input, before LIR validation.

### Rust MIR frontend

A Leaner-managed `leaner-rust-export` Rustc Public driver runs normal rustc
analysis, mechanically maps the in-memory MIR to a Rust-profile raw CFG in the
versioned generic `RawUnit` schema, serializes it, then stops before code
generation and linking. `ImportEvidence` records the rustc/target/edition/
panic/overflow configuration, source hashes, signatures, generic predicates,
implementation-selection evidence, source origins, and asserted type/borrow
check status. There is no second on-disk MIR exchange format.

LIR performs administrative simplification, dominance/post-dominance and
natural-loop analysis, reducibility checks, and structurization. It records
the CFG-to-tree witness in alignment evidence. An unsupported graph is an LIR
diagnostic at the imported Rust origin; no raw `goto` fallback enters validated
structured LIR.

Successful rustc type, trait, and borrow checking is explicit upstream
admission evidence. LIR does not reimplement rustc's trait solver, but Leaner
does run its own LIR borrow checker and exposes the resulting rules and facts
to the verifier. Its abstract rules may accept more programs than rustc; this
does not widen imported Rust, which must already have passed rustc. The two
results remain visible in `ImportEvidence` and final theorem reports.

### Frontend agreement

Checked equivalent inputs in the same semantic profile satisfy:

```text
canonicalize(semanticProjection(check(moveFrontend(s)).validated))
  ≈α
canonicalize(semanticProjection(check(leanerFrontend(s')).validated))
```

Alpha-equivalence uses an explicit deterministic mapping for declaration,
local, expression, and generated-name IDs. It never compares printed text.
For Rust, the required comparison is between structured imported MIR and a
fresh import of canonical Rust emitted from LIR through the same driver.
Re-elaborated canonical Leaner source under the Rust profile is a separate
optional source-backend comparison. Alignment evidence is compared by its own
freshness/refinement contract, not by semantic equality.

## Alignment and theorem meaning

Verification first proves a statement about profile-aware LIR semantics:

```text
Contract.Satisfies function.semantics function.specification
```

Claiming the corresponding source or MIR artifact satisfies the contract also
requires checked alignment:

```text
UpstreamExecution artifact input outcome
  -> function.semantics.Relates input outcome
```

Move source, Lean-authored declarations, generated/re-elaborated Leaner source,
and Rust MIR each have a distinct producer-alignment obligation but converge
on the same LIR function meaning. Constructing LIR alone never proves the
artifact correspondence.

An initial adapter may mark alignment as trusted and bind it to pinned producer
versions, source/configuration hashes, and a semantic body hash. The theorem
UI reports all trusted importers, upstream checks, and external summaries on
which an end-to-end claim depends. Later checked translation witnesses or
generic refinement theorems can replace those assumptions without changing
contracts or function semantics.

## Backend contracts

### Move source backend

Consumes a compatible namespace view from `ValidatedUnit` and emits canonical,
semantically equivalent Move. It accepts the Move profile or an explicitly
validated profile adapter only. If Move syntax cannot express a valid LIR
feature, the capability pass rejects it before printing. The printer does not
omit it.

### Leaner source backend

Consumes the same validated view and selects profile-appropriate forms in the
one Leaner surface language from semantic profile, never origin. Libraries
provide generic facilities such as intrinsic carriers, but the backend neither
validates nor rediscovers graphs. Attributes are emitted only as surface
encodings of validated LIR.

For a Rust-profile unit, this backend emits readable canonical Leaner source,
not reconstructed Rust tokens. A Rust operation may use a Move-profile form
only when an explicit profile rule proves the meanings equal. Otherwise it
prints an unambiguous Rust-profile primitive. The checked-in successful Rust
RawUnit corpus now passes fresh LeanerLang elaboration and deterministic
reprinting under this profile; unsupported semantic families still fail
explicitly. This profile-selected backend is distinct from the native Rust
source backend below.

### Rust source backend

Consumes a validated Rust-profile view, checked dependency interfaces, and the
recorded target configuration. It emits readable canonical standard Rust
`.rs` source, never a reconstruction of original tokens or macros. Retained
comments are non-semantic LIR provenance and may be emitted canonically.
It must preserve the profile's integer, panic, drop, generic, trait/impl, and
unsafe semantics or report a located capability error. The generated crate is
checked and re-imported through `leaner-rust-export`; its fresh `RawUnit` must
validate to profile-aware semantic LIR equivalent to the input.

### Executable backend

For the Move profile, lowers structured executable bodies to NSIR and then to
`MoveModel.IR`, XIR, and bytecode. Proof-only declarations may be erased only
at this explicit boundary. Locations remain attached to derived lower nodes.
Compiling LIR back to Rust machine code is not an initial backend. A future
Rust executable backend must preserve the Rust profile rather than route
through Move operations.

### Verification backend

Derives relational meaning, conditions, contracts, loop specs, and obligations
from profile plus structured body. It reads these from the same validated LIR
as executable lowering; it never synthesizes `true`/empty specs because NSIR
omitted them. Lean theorem generation is a UI for obligations, not spec
storage. Call summaries may cross origins freely inside a compatible profile;
cross-profile summaries require a validated boundary adapter.

## Move lowering terminology

```text
LIR --erase proof-only declarations / lower structured code--> NSIR
    --assign positional indexes-------------------------------> SIR
    --finite serialization------------------------------------> XIR
```

- **LIR**: the complete profile-aware structured representation defined here;
- **NSIR**: the current named executable `Move.Compiler.LIR`, after rename;
- **SIR**: positional `MoveModel.IR` stackless/verification representation;
- **XIR**: the versioned wire representation of SIR;
- **LIR JSON**: the raw-LIR wire format which replaces XAST's role.

This is one profile-specific lowering, not the definition of LIR. A source
backend consumes LIR directly and never reconstructs structured source/spec
declarations from NSIR or SIR.

## Checked-construction order

1. A frontend constructs a complete `RawUnit`, dependency interfaces, and
   interned provenance/alignment evidence.
2. LIR bounds-checks tables before following an ID.
3. LIR normalizes structured input or structurizes raw CFG input and checks its
   witness.
4. LIR resolves names and checks resolution hints.
5. LIR infers/checks core-union/extension types and lifetime/borrow constraints,
   then checks upstream hints/certificates. The initial slice only validates
   lifetime structure and retains explicit inference variables.
6. LIR validates declarations, bodies, specs, profiles, effects, intrinsics,
   ownership/abilities, borrow rules, and other shared language rules.
7. Reports use check results, alignment status, and capability queries.
8. A selected backend prints or lowers a validated namespace view.

A backend may reject an unsupported valid feature. It may not silently weaken
it.

## Serialization and compatibility

`RawUnit` is the process-boundary wire type. Deserialization never constructs
a `ValidatedUnit`; decoded input always passes through
`LeanerIR.Validation.validate`.

The raw in-memory type and JSON schema share a major version. Profile payloads
carry their own compatible schema versions. Unknown major
versions are rejected. A newer minor version is accepted only when every
unknown field is explicitly ignorable provenance; semantic fields are never
ignored. Closed enums use stable textual tags. Open-ended attributes, pragmas,
and raw intrinsic roles remain strings until validation.

Encoding is deterministic:

```text
decodeRaw(encodeRaw(normalizeRaw(r))) = normalizeRaw(r)
```

The version is currently spelled in four independent places: the codec's
`jsonVersion`, validation's `checkVersion`, `RawUnit.version`'s default, and
the Rust exporter's literal. Raising 1.0 to 1.1 for the field-place owner
therefore took four separate fixes, each surfaced by a different failing
suite. One Lean-side source read by the other two, and a generated constant
for the exporter, would make the next bump a single edit.

XAST v4 remains a compatibility frontend and converts immediately to raw LIR.
The Rust MIR exchange similarly decodes immediately to a Rust-profile raw CFG
body. No new semantic analysis is added to either exchange-specific view.

Persistent imported artifacts are accompanied by a frontend-neutral import
receipt recording the producer, source hashes, target/profile configuration,
and semantic body hash. The Lean-owned import gate checks that receipt and
rejects stale or mismatched artifacts before calling `check`; `check` itself
validates the LIR rather than reading source files. A theorem registry keys
imported claims by the semantic hash and alignment status, so a source change
cannot silently retain an old theorem.

## Semantic and round-trip laws

The project uses executable tests first and states these intended laws:

1. **Canonicalization idempotence:** canonicalizing a canonical semantic
   projection changes nothing.
2. **Serialization round trip:** normalized raw units survive JSON exactly;
   checking the decoded unit yields the same canonical validated semantics.
3. **Frontend agreement:** equivalent inputs for the same profile check to
   alpha-equivalent semantic projections, independent of origin.
4. **Move source round trip:** Move → LIR → Move → LIR preserves semantics.
5. **Leaner source round trip:** Leaner → LIR → Leaner → LIR preserves
   semantics.
6. **Rust source round trip:** imported MIR → structured LIR → canonical Rust
   → MIR import → LIR preserves normalized Rust-profile semantics.
7. **Rust-profile Leaner round trip:** structured Rust-profile LIR → Leaner
   source → LIR preserves normalized profile semantics for the supported
   surface subset.
8. **Compatible cross-source round trip:** one surface may emit another only
   for the subset whose profile meanings agree or through a checked adapter.
9. **Executable preservation:** Move-profile structured LIR → SIR preserves
   outcomes for the executable language.
10. **Specification preservation:** verification lowering preserves clauses,
   frames, anchors, invariants, and axiom denotations.
11. **Structurization alignment:** accepted raw MIR CFG and its structured LIR
    have the same outcomes under the supported Rust profile.
12. **Frontend-independent diagnostics:** corresponding invalid inputs which
    reach raw LIR produce equal codes/data after locations are erased.

Semantic equivalence is not equal text or equal provenance.

## Deferred-work register

A deliberate deferral must be recorded here (and in a frontend-specific design
when applicable) with its reactivation point. It is removed only when the work
lands, is explicitly rejected, or is superseded by another recorded design
decision.

| Deferred item | Current safe boundary | Reactivate by |
|---|---|---|
| Strict RawUnit JSON v1 hardening: an externally frozen compatibility contract | The deterministic Lean encoder round-trips the current schema; its strict parser rejects duplicate object keys and its typed decoder rejects unknown fields recursively against the decoded constructor's canonical closed shape. The Rust serde mirror denies duplicate and unknown record and variant fields. A malformed corpus covers every structural codec family, and exhaustive raw-CFG, core provenance/profile/value/type/place/pattern, operation/expression/specification, and declaration corpora recursively inject unknown fields into every encoded object | Before the Rust exporter M0 gate emits RawUnit JSON outside the repository, approve and publish the current v1 spelling as the external compatibility contract |
| Full dependency interfaces and authoritative cross-namespace declaration resolution | One compilation-unit `Tables` snapshot includes disjoint owned/dependency identities. Dependency interfaces, exports, profiles, and imports are bounds-, ownership-, and uniqueness-checked; every external import requires a declared interface | LIR Phase 2, before dependency signatures or bodies participate in semantic typing, interpretation, or verification |
| Checked trait-selection evidence table, associated normalization, and dictionary semantics | Core traits, implementations, associated items, binder scoping, and generic argument kinds are represented and structurally checked; `EvidenceId` remains opaque | Before Rust M2 trait calls are interpreted or trait contracts transfer proofs |
| Semantics and structurization for remaining raw MIR administration and exceptional control | Direct-call normal continuations, reducible non-Boolean switches, inert storage/place/type administration (including borrow-check-only fake reads normalized to place mentions after rustc admission), ordinary `panic=abort` assertions, unconditional MIR abort terminators, the Rust frontend's well-known abort-intrinsic normalization, and fixed-profile drops with unreachable unwind edges structurize in core LIR. Deinitialization, discriminant updates, and retags have closed RawUnit and Rust-mirror forms; the Rust optimized-MIR mapper resolves the public `SetDiscriminant` case to a stable enum variant, while the pinned public statement API exposes neither `Deinit` nor `Retag`. These destructive/provenance nodes, cleanup edges, pointer-alignment assertions, and reachable unreachable/resume terminators survive JSON and are rejected explicitly | Finish Rust M4 destruction effects, cleanup, and richer panic behavior; unsafe-profile milestone for pointer-alignment assertions |
| Drop flags, cleanup scopes, panic behavior, and unwinding | Raw drop/unwind information is preserved. Under fixed `panic=abort`, an unreachable-unwind drop becomes an explicit typed drop operation and normal continuation; no cleanup behavior is approximated | Finish Rust M4 drop effects and flags; full unwinding is a separately approved post-M4 extension |
| Authoritative resolution, full semantic type checking, lifetime constraints, and LIR borrow checking | Structural IDs, arena acyclicity, declaration types, generic scopes and declaration-local binder identity, ability-list uniqueness and binder-kind validity, and profile vocabulary are checked. Definite initialization covers direct locals, literal-indexed tuple/fixed-vector elements, and owned index/subslice/field paths including enum-downcast-qualified fields; a dynamic index or subslice conservatively consumes its entire owning local. A structured-body loan pass rejects overlapping local-rooted accesses and local loans live at normal or explicit reference-bearing returns, without rejecting an unrelated returned parameter after the local loan dies. Unequal nonnegative literal index projections are disjoint for loan overlap, while equal or dynamic indexes remain conservative. Loans bound or transferred through local value/move/copy/read aliases, direct assignments, nested binding patterns, reborrows, or call results die after every known holder's last use in sequential structured evaluation; overwriting a direct reference local by expression, pattern, or write operation releases its prior holder before evaluating an independent replacement and then attaches the replacement loan. Whole tuple/vector and same-unit struct/enum holders retain source-aligned carrier paths, so projected uses select only overlapping component loans; projected moves/drops release only their selected carriers, conditional/match results preserve alternative carrier paths, tuple/constructor patterns select matching carriers across direct and control-flow producers, and projected assignment/write replaces only the destination carriers while preserving siblings. Wildcard-only declarations and pattern assignments end newly created uncarried loans while preserving loans already held through another alias. Nested operands and enclosing continuations preserve later-used loans by identity without retaining sibling carriers, while mutually exclusive `if` branches, match arms, and loop bodies shorten independently. Loop fixed points retain only loans required by the body or continuation. Supported direct-local place-index reads participate in conflict checking and holder liveness rather than keeping every loan alive. Moving or dropping through a dereference is rejected at preparation while ordinary dereference reads, copies, and writes retain their existing semantics. Loan-site history is distinct from the loans carried by an expression value. Same-unit direct-call result-source selection follows transitive declared predicates through direct, tuple, vector, instantiated type-parameter, and namespace-resolved nominal field shapes, including same-instantiation cycle-cut recursive declarations; temporary loans passed through unselected parameters end after the call, and same-unit calls retain packed multi-result indexes plus structural tuple/vector/reference and acyclic instantiated struct/enum carrier paths when declared lifetimes identify the returned component. Recursive nominal back-edges retain conservative subtree roots without widening siblings; argument-changing recursive and external-dependency source selection remains conservative. Successful preparation retains parameter-reference and loan-site/lifetime/holder facts plus reflexive/static/declared/reborrow/direct-call/function-result-boundary/transitively closed outlives relations. Compatible reference-bearing structural tuples, vectors, references, and function types may cross call and result boundaries with distinct lifetime identities; constraints follow shared-reference covariance, mutable-referent invariance, and function-argument contravariance. Nominal variance is not guessed. General non-lexical loan death, dependency/unknown aggregate alias propagation, authoritative nominal variance, and remaining aggregate/use-site region constraints remain | LIR Phase 2 for resolution/type checking; Rust M3 for remaining ownership paths, regions, and borrowing |
| Artifact/source/configuration freshness enforcement and proof-facing alignment evidence | Validation requires nonempty producer and claim descriptions and retains a checked copy of every trusted/untrusted import-evidence statement in `ValidatedUnit`; it does not yet bind those statements to source, configuration, or semantic hashes, and does not upgrade trust assertions into proof. The Rust driver separately retains and registers a Lean-owned import-mode/input/cache-key/artifact-digest receipt through execution and verification preparation; private receipt/import/prepared-result constructors prevent consumers from minting receipts or recombining one with a different validated unit, and cached artifact bytes must match the sidecar digest, but that cache-integrity receipt is not yet part of the shared validated-unit proof boundary | Rust M1.5 for the shared receipt gate and M7 before a theorem is claimed about an external Rust artifact |
| Raw pointers, allocation/provenance, unsafe cells/unions, and explicit UB semantics | Unsafe-related nodes must be preserved or rejected with source locations; references are not used as a substitute | Rust M5/U0–U2 according to the unsafe roadmap |
| Prophetic-reference residue: loan-typing environment, non-lexical loan death, two-phase borrows and interior mutability | Validated LIR executes and verifies on the prophetic ownership model ([`prophetic-references.md`](prophetic-references.md)): borrows own their values, loan deaths are explicit markers materialized at semantic preparation from the certificate's lexical death records, dying frames export unreconciled loans through the state's pending set, and generated contracts name each export. The reference-vocabulary typing leaf lemmas (dereference inversion, borrow introduction, freeze, hole writes) are proved; walker-level preservation is not stated, because a hole types at every type and only a loan-typing environment can tie a hole's position to its loan's referent. `endLoan` and freeze leave dead placeholder values where consumed borrows rested, so full frame typing must be stated up to dead positions | The M2 stage-5 evaluator induction introduces the loan-typing environment; Rust M3 region work covers non-lexical death (markers move, the model is unchanged); a two-phase/interior-mutability design is required before NLL-dependent or `Cell`-style constructs are admitted |
| Native LIR source printers and semantic alpha-equivalence round trips | `LeanerLang.Print` is the integrated validated-LIR printer and formats canonical Move/Rust-profile output at width 80; source can be re-elaborated and printed again without a separate pretty-printer tree. The Move stdlib's 15 modules all produce parse-checked canonical output; XAST v4 behavior summaries, including the four higher-order `std::vector` spec functions, round-trip as ordinary LeanerLang declarations. Unsupported inputs remain explicit error baselines rather than silently weakened declarations. The `ValidatedUnit`-only Rust backend deterministically re-imports 74 target-independent scalar, character, borrowed-string, aggregate, call, control, reference, assertion, ownership, comment-provenance, and concrete-generic cases with equal observable results and, for stateful reference cases, equal final heaps. The executable corpus additionally passes a provenance-free semantic alpha-projection that preserves names, declarations, types, signatures, and reachable structured bodies while normalizing binder/local spelling and exact standard-Rust/rustc administrative forms; unsupported semantic declaration, contract, attribute, and specification families are rejected rather than ignored. The corpus covers every arm of a non-Boolean integer switch, normal and terminal-panic assertion paths, explicit abort, and full-range `u128`. Enum source preserves declaration, named/positional field, variant, explicit-discriminant, and guarded borrowed-projection shape; complete discriminant/field-selection switches render as direct enum-pattern matches and pass semantic alpha-comparison for both variants, while guarded/fallback outcomes compare target-width behavior across re-import. Never-returning loops, references, borrowed UTF-8 strings and their byte lengths, array indexing, borrowed slices, from-end slice indexing, subslices, and type/lifetime/const-generic nominal declarations preserve recursive declaration/signature shape modulo regenerated inference-lifetime IDs. Rust line and nested-block comments cross RawUnit JSON, canonical LeanerLang, ordinary Lean file elaboration, and canonical Rust re-import without entering semantic equality. Target-width integer execution uses the validated Rust profile's explicit rustc-target width and never the Lean host width; array/slice/string-reference source round trips compare normal outcomes or bounds panics across re-import. From-end subslice borrows are reconstructed as dependency-free slice-rest `let … else` bindings and compare empty and non-empty outcomes semantically; general range indexing remains behind the standard `RangeFrom`/`Index` dependency model. Const binders retain a checked declared type; the Rust exporter derives it from admitted concrete ADT instantiations, and `Tagged<u32, 3, true>` round-trips semantically. Plain type-generic functions and local direct calls preserve one body and inferred instantiations through execution, LeanerLang, canonical Rust, and semantic re-import. Symbolic const-dependent types, constrained functions, trait evidence, and generic-function const metadata remain blocked on unavailable frontend metadata. Validated lifetime/type/const binders, lifetime-outlives predicates, and type-binder `Copy` abilities or equivalent core `Copy` predicates on functions and nominals render as canonical Rust declarations and `where` constraints; function constraints carry mapped provenance. Other constrained/evidence binders remain blocked on unavailable generic metadata. The generated source map records UTF-8 byte ranges and stable node/location IDs for emitted functions and their generic binders, nominal declarations and generic binders, variants, fields, declaration/signature/local/expression type uses, expressions, patterns, place occurrences, and function-scoped locals. Function-owned nodes retain import origin/alignment; data declarations explicitly carry none because the schema supplies none. Dependency imports and unrendered namespace, function, or nominal semantic metadata are rejected rather than silently omitted. Semantic normalization of remaining validation-only bodies, frontend re-import, and Move/Leaner-native semantic alpha-equivalence remain | LIR Phase 6 and Rust M2/M7 for the supported subset |

## Roadmap and implementation progress

The first vertical slice intentionally crosses several phases before any
phase gate is declared complete:

- Phase 1 is partial: the neutral schema, identifiers, compilation-unit tables,
  arenas, checked-and-retained import evidence, diagnostics, deterministic
  derived raw serialization, and
  duplicate-key plus recursive closed-field rejection in both Lean and the Rust
  mirror exist. A negative corpus covers each codec shape. Every raw-CFG and
  core provenance/profile/value/type/place/pattern and
  operation/expression/specification and declaration constructor has canonical
  round-trip plus recursively mutated closed-object coverage. Strict external
  JSON compatibility and the full staged API remain open.
- Phase 2 is partial: common structural checks, reducible CFG structurization
  with checked durable block, edge, branch, switch, natural-loop, and
  direct-call-continuation correspondence witnesses retained by
  `ValidatedUnit`,
  the Move vocabulary checker, typed primitive/call/data/global/reference
  operations including Move address/`Key` global-storage constraints,
  non-consuming enum-discriminant inspection without requiring the payload to
  be `Copy`, and compact fixed-vector repetition, nominal and subslice
  place checking, direct-local value-borrow normalization, intrinsic fixed-type
  well-formedness including fixed-vector
  length shape, recursive nominal-field plus structural call/constructor,
  constructor-pattern, and data selection/update type/lifetime substitution
  through already interned arena nodes, canonical
  namespace, qualified declaration, nominal field, enum-variant, and
  trait-associated-item uniqueness within type/value namespaces and
  bidirectional ownership,
  local and imported-trait implementation binding ownership, required-item
  completeness, and associated-constant typing,
  resolved trait references with generic arity, kind, and first-order ability
  checking, associated-equality item ownership/kind checking plus associated
  constant literal typing after trait-generic substitution, and acyclic local
  supertrait graphs, including owned cross-namespace associated-item IDs
  (without implementation selection, normalization, or evidence construction),
  exclusive struct-versus-enum declaration shape, disjoint dependency
  interfaces and declared external-import checks,
  declaration-local generic binder identity, intrinsic ability-list uniqueness
  and type-binder-kind checks plus core
  ability satisfaction including nominal declarations,
  rejection of direct-call and closure-construction profile crossings without
  a validated boundary adapter,
  path-sensitive local and owned index/subslice/field initialization checking with
  preparation-bound function certificates shared by execution and verification
  consumers, interpreter reclamation of heap slots allocated solely to
  stabilize borrowed frame locals without renumbering nonlocal storage,
  conservative local-rooted shared/mutable loan conflict checking with
  evaluation-order plus branch/arm/loop-entry-specific holder liveness and
  direct-local place-index use-site checking plus disjoint unequal literal-index,
  forward-subslice, and outside-index/subslice loan projections with conservative
  dynamic/from-end indexes,
  conservative iteration fixed points, direct-local, assignment/pattern/call-result
  alias, declared-outlives-aware structural direct-call result-source selection
  through local nominal (including same-instantiation cycle-cut recursive declarations) and
  instantiated type-parameter shapes, structural and acyclic instantiated
  local-nominal packed multi-result call carrier paths plus subtree-conservative
  recursive carrier cycle cuts,
  and reborrow-holder last-use, wildcard-discard, projection-aligned tuple/constructor
  child binding across direct and control-flow aggregate producers,
  projected whole-tuple/vector and same-unit namespace-resolved struct/enum carrier selection,
  carrier-preserving conditional and match result joins, projected
  assignment/write replacement with sibling-carrier preservation,
  loan-identity-specific nested operand and initializer preservation,
  and direct or projected moved/dropped-holder loan death in
  sequential blocks, rejection
  of possible local-loan
  escape at normal and explicit reference-bearing returns, and preparation-bound certificates
  retaining reference-parameter plus checked loan-site/lifetime/holder identities and
  reflexive/static/transitive closure of declared, reborrow, explicit freeze, structurally paired
  direct-call argument/parameter/result, and function-result boundary outlives predicates,
  recursively through structural tuples, vectors, references, and function
  types with shared-reference covariance, mutable-referent invariance, and
  function-argument contravariance,
  resolved/constrained nominal type uses, loop-control/fallthrough result
  typing, Unit typing for fallthrough result-less control and loop bodies, and
  Unit typing for pattern assignment,
  trait-associated method signatures scoped over their owner trait binders
  before method-local binders,
  body-local semantic scoping for type parameters and parameter lifetimes,
  resolved resource-domain nominal targets and generic arguments,
  first-slice verification typing for logical declaration bodies,
  initializers, contracts, predicate conditions, closed condition-auxiliary
  roles, abort-code payloads, same-typed update targets, and specification-function
  calls plus fixed result/type-domain/resource-domain/old/well-formedness, source-level
  bit-vector conversion, consistent generic builtin instantiations,
  logical-`num` specification-vector indexing, logical resource-mutation,
  global-read/can-modify, abort-code, state-anchor/inline-summary, and
  event-store construction/inclusion shapes before their M4 boundary,
  specification function/variable namespace-profile consistency,
  constant declaration/reference, associated-default, and local associated-
  constant binding typing including cross-namespace structural equivalence of
  generic nominal arguments, resolved associated-method default/binding
  function targets, and
  profile-independent
  intrinsic graph integrity checks exist;
  authoritative trait implementation/evidence selection, dependency, effect,
  general non-lexical loan death, dependency/unknown aggregate alias propagation,
  authoritative nominal variance, and remaining nominal/function-variance
  use-site region solving,
  and remaining
  whole-unit type well-formedness do not. The Move profile has
  the complete transported map-role vocabulary plus owner-shape,
  required-role, dependency, and exact physical-signature checks.
- Phase 3 is partial: XAST converts to raw LIR and the canonical native
  LeanerLang backend consumes `ValidatedUnit`; XAST is an
  explicit compatibility wrapper. The intrinsic graph/signature checkpoint is
  complete, and package-wide Leaner action/global-write inference now runs over
  validated LIR. The XAST adapter preserves producer ranges where available
  and records unlocated types, binders, fields, values, and intrinsic bindings
  as generated locations linked to their authored parent. Reporting and
  printing now have a frontend-independent report contract; module identity
  and source-skipped declarations are seeded from validated LIR. The old
  transpiler report/action path still carries a transitional printer-package
  decode for its legacy consumers and remains to migrate; canonical source
  rendering itself no longer uses that projection.
- Phase 4 is partial: `LeanerLang.Lower` is a direct surface-to-raw-LIR
  elaborator for the supported Move and Rust profile surface, while the legacy
  bridge imports whole registered namespaces and a substantial source-spec
  subset with explicit evidence for unsupported items. Completing surface
  coverage, migrating remaining semantic side tables, and retiring the NSIR
  bridge remain.
- Phase 6 has executable gates in two source families: canonical Leaner output
  for BasicCoin is elaborated by a fresh Lean frontend while its loops fixture
  is compared, re-imported, and checked for equal shared-interpreter outcomes
  across boundary inputs after pattern assignments are normalized to their
  required Unit result type; the first standard-Rust backend consumes only
  `ValidatedUnit` and re-imports target-independent scalar, aggregate, ADT,
  reference, ownership, branch, and loop fixtures through the pinned MIR
  frontend with equal interpreter outcomes and equal alpha-normal semantic
  projections after exact renderer/rustc administration is contracted. General
  enum variants and guarded matches join their declaration-shape round trip in
  the executable corpus. Concrete functions over direct, nested, lifetime-
  bearing, const-generic, and enum generic ADTs also compare outcomes after
  re-import. Independently, all forty-nine checked-in successful Rust RawUnit
  fixtures now print as canonical LeanerLang, elaborate through a fresh frontend, reprint byte-for-byte,
  and compare executable semantic projections. This initial LeanerLang slice
  covers scalar and fixed-width signatures, unary and Boolean bitwise operations,
  pure tuple construction, context-directed integer casts, repeated fixed
  vectors, comparisons, shifts, guarded division/remainder and negation, panic
  paths, full-range `u128` literals, and direct recursive calls selected by a
  branch. Unicode scalar literals, ordering, casts, and literal/wildcard
  classification matches also round-trip, as do tuple/fixed-vector construction,
  tuple projection, dynamic fixed-array indexing with its panic path, literal
  indexing, fixed-array destructuring, plain struct declarations, positional
  construction, typed field selection, direct Boolean branches, and exhaustive
  integer literal switches. Direct generic-field instantiation and non-`Copy`
  partial field moves and recursively substituted nested generic-field loads are included. Wrapping arithmetic with an explicit
  overflow flag is preserved for add, subtract, and multiply. Typed integer/Boolean const binders and
  concrete const arguments round-trip in `Tagged<u32, 3, true>`. Plain
  type-generic functions and a local direct generic call retain one body and
  inferred type instantiation through execution and fresh LeanerLang
  elaboration. A non-generic inherent method and receiver call also retain
  their local function/ADT semantics and explicit overflow behavior. Its
  rustc overflowing-tuple plus assertion administration now renders as one
  checked-panic arithmetic primitive; normal and overflow-panic executions
  agree before and after fresh LeanerLang elaboration. Rust fixed-width parameter families
  compare boundary-value outcomes across
  re-import. A shared place-index classifier admits the exact side-effect-free
  length-minus-offset form consistently across preparation, initialization,
  borrowing, and execution, while target-width integers retain `.pointer` identity
  while preparation and interpretation use the validated rustc-target profile
  width, including an executable dynamic array-index result and bounds panic,
  plus fixed-array destructuring and a statically safe literal index after
  redundant rustc guard contraction. The Rust corpus also checks valid
  half-open UTF-8 ranges and complete reachable expression, pattern, and place
  coverage for every emitted source map. Semantic
  normalization of remaining validation-only bodies, Move/Leaner-native
  semantic alpha-equivalence, Move/Leaner source-map coverage, cross-language
  location fidelity, and standard-library/framework corpus-wide source
  roundtrips remain.

No phase gate below should be read as satisfied merely because this vertical
slice passes.

### Phase 0 — approve and freeze the boundary

- Review package, profile, and terminology choices jointly with the
  `leaner-rust` design.
- Produce the XAST/Leaner/MIR-subset coverage matrix.
- Classify every type, operation, effect, outcome, and state component as
  common core, Move profile, Rust profile, or unsupported.
- Freeze the profile interface, raw CFG body, provenance, import evidence, and
  generic RawUnit serializer requirements needed by `leaner-rust`.
- Freeze new frontend-local analysis and report rules.
- Label current `Move.Compiler.LIR` as NSIR in documentation.

Gate: no semantic construct is unclassified.

### Phase 1 — neutral core, profiles, IDs, and wire format

- Create independent `leaner-ir` / `LeanerIR`.
- Define raw/structured/resolved/typed/validated stages, namespace interfaces,
  profile registry, and alignment-evidence envelope.
- Define strong IDs, tables, locations, complete declarations, structured
  places/expressions/patterns, contracts, CFG import form, and deterministic
  raw JSON.
- If a Rust-side raw-LIR encoder is needed, put its serde-only mirror in a
  neutral `leaner-ir-exchange` crate rather than `move-model-exchange`.

Gate: schema, malformed-table, normalization, and JSON tests pass without
importing `move`, `transpiler`, or a Rust frontend.

### Phase 2 — shared checking and Move profile

- Implement shared structurization, name resolution, type checking,
  declaration/body/spec validation, and dependency checking.
- Implement the separate Move-profile library and validate Move rules for core
  nodes plus extension types, operations, outcomes, effects, and state.
- Freeze the profile conformance suite against the Rust design and use a small
  synthetic second profile to test distinct outcomes/effects and explicit
  cross-profile adapter checks; do not implement Rust semantics here.
- Make `LeanerIR.Validation.validate` the only route to `ValidatedUnit`.
- Add stable diagnostics, related locations, deterministic accumulation, and
  semantic feature/alignment indexes.

Gate: constructed raw fixtures exercise core-union, Move-only, synthetic-profile,
and invalid cross-profile operations; no backend or test constructs validated
values directly.

### Phase 3 — XAST migration, first backend, and intrinsics

- Convert XAST v4 to raw LIR, importing compiler-v2 results only as hints.
- Add the intrinsic role schema and complete graph/signature validation.
- Retain exact locations where available and explicit generated parents where
  the compatibility schema is lossy.
- Expose a checked driver entry point over `ValidatedUnit` and quarantine XAST
  construction/validation in a compatibility wrapper.
- Move package-wide action/global-write inference to validated LIR; adapt only
  its qualified names to the transitional printer table.
- Adapt the current Leaner printer and transpile report to validated LIR.
- Delete graph validation and role registries outside the Move-profile LIR
  pass.

Gate: intrinsic error fixtures use shared codes/data and no printer/report pass
accepts `Transpiler.Xast.Module`.

### Phase 4 — Move and Leaner migration

- Build raw LIR directly from the one Leaner language under the selected
  profile; the Rust-profile surface forms may be added in a later session.
- Migrate every semantic elaborator side table, deleting it after its last
  consumer moves to LIR.
- Add a Move producer capable of raw-LIR construction before compiler-v2
  semantic rejection; retire typed XAST as the only Move input.
- Capture authored locations before macro expansion changes them.

Gate: all existing Leaner tests and the Move corpus dump validated LIR; an
inventory test detects side-table-only semantics; paired invalid sources have
matching location-erased diagnostics.

### Phase 5 — `leaner-rust` integration boundary

- Have `leaner-rust` implement the Rust profile and the Rustc Public driver
  against the frozen profile interface.
- Have the driver serialize the generic RawUnit schema with the raw CFG and
  import evidence frozen by the LIR boundary review, then stop after analysis.
- Accept its version/configuration/source hashes, generic predicates,
  implementation-selection evidence, type/borrow status, and multi-file
  origins; reject a producer that can provide only monomorphized bodies.
- Structurize reducible MIR inside `LeanerIR.Validation.validate` and retain a checked
  correspondence witness.
- Emit canonical standard Rust and re-import it through the same driver to
  equivalent Rust-profile LIR; test Rust-profile Leaner printing separately.

Gate: the initial safe sequential Rust subset reaches validated LIR without a
frontend-specific analysis/report path. This gate does not require completing
Rust ownership, drop, panic, or alignment proofs; those remain milestones of
`leaner-rust`.

### Phase 6 — source round trips

- Complete canonical Move, standard Rust, and profile-selected Leaner source
  backends.
- Add same-language and cross-language round-trip fixtures.
- Compare profile-aware semantic LIR, not source text.

Gate: standard-library and framework corpora pass semantic and location
fidelity tests, imported Rust scalar/ADT/control fixtures round-trip through
standard Rust and the same exporter, and the supported Rust-profile Leaner subset
round-trips independently.

### Phase 7 — Move lower backends and retirement

- Rename current executable `Move.Compiler.LIR` to NSIR (or approved name).
- Derive NSIR and verification IR from validated LIR.
- Remove trivial-contract and empty-loop-spec synthesis.
- Remove the transpiler-owned semantic XAST view and all frontend-local
  reports.

Gate: no semantic analysis reads frontend AST/syntax after raw construction,
and no backend accepts unchecked LIR.

Every temporary adapter has a named removal phase and gate. A missing LIR
feature remains explicitly unsupported during migration; a frontend side table
is not a long-term substitute.

## Testing strategy

Current executable gates are:

- `lake test` in `leaner-ir/` for malformed IDs, cycles, structured input,
  reducible diamonds/loops, terminal loop exits, irreducible/multi-exit
  rejection, and intrinsic owner/target/duplicate graph integrity;
- `lake test` in `leaner-rust/` for Rust-profile semantics, file/Cargo import
  registration, and `ValidatedUnit`-to-standard-Rust scalar/control round trips
  through the same pinned exporter;
- `lake test` in `leaner-move/` for registered and unknown Move profile
  values and the complete 62-role intrinsic vocabulary, owner shape, required
  roles, role categories, dependencies, and malformed physical signatures;
- `lake build Transpiler.Tests.Intrinsics.Sources` for all six source owners and
  all 158 intrinsic bindings through XAST, raw LIR, and Move-profile checking;
- `lake test` in `leaner-e2e-tests/` for discoverable Move-to-LeanerLang and
  Rust-to-LeanerLang source/result baselines;
- `lake test` in `v0/transpiler/` for the XAST decoder, adapter, printer baselines,
  source fixtures, Leaner frontend/LIR boundary units, axiom printer unit, and
  intrinsic-source units;
- the existing `MoveTests` build, which exercises the Leaner language and
  verification tree independently of the adapter.

The transpiler unit corpus includes Account, Axioms, BasicCoin, Constants, Counter,
Enums, Generics, Loops, OrderedMap, Vectors, Signer, Mem, and Error, plus a
synthetic mutable-vector NSIR fixture. Some modules deliberately carry
untrusted evidence for source specification forms outside the current bridge;
the test asserts that this loss is reported.

- **Schema:** every variant, invalid ID (including lifetime IDs), cyclic
  arena/type/reference edge, unknown profile/tag, field, and deterministic
  encoding.
- **Locations:** Unicode byte offsets, multiple source files, macro expansion,
  generated-parent chains, inlining, related sites, and lower-IR retention.
- **Checking:** one fixture per diagnostic code plus deterministic multi-error
  ordering.
- **Frontend differential:** paired Move/Leaner-source, imported
  Rust/generated Rust, and imported MIR/Rust-profile Leaner inputs compare
  canonical semantic projections where their profiles agree.
- **Profiles:** identical syntax with different profile semantics stays
  distinct; incompatible cross-profile calls fail without an adapter.
- **Structurization:** reducible CFG agreement, irreducible rejection, stable
  loop identities, and witness mutation tests.
- **Backend round trips:** parse/check printed output and compare LIR.
- **Corpus:** aptos-stdlib and aptos-framework, including every intrinsic map
  type.
- **Mutation:** removing a condition, state anchor, binding, type use, or
  location, or changing source/configuration hashes must fail a gate.

Readable source baselines remain useful but are not the semantic oracle.

## Non-goals

- Preserving byte-for-byte formatting.
- Reconstructing original Rust tokens, exact comment attachment, macro invocations, or surface
  constructs from MIR.
- Replacing compiler-v2 parsing or expansion. Compiler-v2 may retain its type
  checker for its native pipeline, but its imported results are hints;
  `LeanerIR.Validation.validate` is authoritative for LIR consumers.
- Reimplementing rustc's trait solver. Trusted upstream admission is explicit
  alignment evidence, while Leaner's independent LIR borrow checker supplies
  proof-facing borrowing facts.
- Reconstructing structured source from stackless bytecode.
- Compiling LIR back to Rust machine code in the initial project.
- Encoding Lean proofs or arbitrary Lean helper definitions in LIR.
- Making every backend total over every future LIR version; explicit
  capability diagnostics are allowed.
- Completing E16 or another transpiler milestone inside this project.
- Completing the Rust MIR importer or its alignment proof inside this project.

## Risks and mitigations

- **Accidentally adding a fourth permanent representation:** migrate XAST into
  the raw-LIR wire role, decode MIR exchange immediately to raw CFG LIR, and
  rename current executable LIR.
- **A giant Move-plus-Rust union:** require a small structural core and checked
  semantic profiles; profile nodes cannot add parallel control/spec/report
  systems.
- **Duplicated type systems:** define LIR checking as authoritative and test
  imported compiler-v2/Lean/rustc hints at the LIR boundary, while keeping
  upstream source-language admission explicit.
- **Frontend drift:** use canonical paired fixtures and shared diagnostics.
- **Lossy adapters:** require semantic equality tests and a deletion phase.
- **Macro location loss:** capture authored ranges early and retain explicit
  generated-parent chains.
- **Over-normalization:** retain clause structure, state anchors, source order,
  and provenance; derive proof normal forms separately.
- **Schema churn:** use strong versioning and open raw names, while rejecting
  unknown semantic fields/profile tags.
- **Unsound imported claims:** bind proofs to semantic/source/configuration
  hashes and report trusted alignment assumptions in final theorems.
- **Large migration:** keep phase gates independently shippable and prohibit
  new semantic debt in frontend-specific paths.

## Decisions and remaining choices

The initial implementation has exercised and provisionally settled these
choices:

1. `leaner-ir` / `LeanerIR` is the neutral package and namespace.
2. The current `Move.Compiler.LIR` is called **NSIR** in this design, although
   the code rename remains a retirement-phase change.
3. LIR has a new v1 in-memory schema and XAST is a compatibility frontend. The
   current deterministic derived JSON encoding must still be hardened and
   frozen as the external wire contract recorded in the deferred-work register.
4. Comments are a core table whose producers may currently leave it empty.
5. The checked implementation is initially in Lean.
6. Semantic profiles are explicit and the first implemented profile is Move;
   incompatible cross-profile calls will require boundary adapters.

`leaner-rust` must still decide the initial Rust profile's panic mode, target
configuration, and imported type/borrow trust status. These belong in
`ProfileConfig` and `ImportEvidence`, not hard-coded core choices. The generic
RawUnit wire format and criteria for removing the transitional XAST and
printer-package adapters also require approval before their respective phase
gates can close.

These choices do not change the core invariant: all shared semantic analysis,
validation, structurization, capability reporting, and source emission is
downstream of raw LIR, and origin never selects meaning.

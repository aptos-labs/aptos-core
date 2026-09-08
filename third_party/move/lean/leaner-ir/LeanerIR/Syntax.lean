-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Ids

/-!
# Profile-neutral LIR syntax

This module contains the finite, source-independent semantic union required by
the known Move and Rust frontends. Frontends and backends may support only an
explicitly diagnosed subset of this union. `ProfileValue` is an escape hatch
for future constructs outside the known union, not the representation of an
ordinary Move- or Rust-specific construct. Some transitional Move adapters
still use such values while their remaining nodes are promoted into the core.
Frontend-only envelopes live in `LeanerIR.Import`; checked backend envelopes
live in `LeanerIR.Validation`.
-/

namespace LeanerIR

/-- One source file named by a namespace's provenance tables. `contentHash`
binds imported locations to particular contents when the producer provides
one; an empty hash makes no freshness claim. -/
structure SourceFile where
  name : String
  contentHash : String := ""
  deriving Repr, BEq, Inhabited

/-- A half-open byte range `[startByte, endByte)` in a `SourceFile`. Offsets
are bytes rather than Unicode scalar or display-column positions. -/
structure SourceRange where
  file : FileId
  startByte : Nat
  endByte : Nat
  deriving Repr, BEq, Inhabited

/-- Interned provenance for an authored or generated AST node. `primary` is
the diagnostic range, `related` and `expansion` preserve secondary and macro
sites, and generated nodes may name their pass and parent location. -/
structure Location where
  primary : Option SourceRange := none
  related : Array SourceRange := #[]
  expansion : Array SourceRange := #[]
  generatedBy : Option String := none
  parent : Option LocId := none
  deriving Repr, BEq, Inhabited

/-- Source family which produced a declaration. Origin records provenance;
semantic behavior is selected separately by `Profile`. -/
inductive OriginKind where
  | moveSource
  | leanerSource
  | rustMir
  | generated (producer : String)
  deriving Repr, BEq, Inhabited

/-- Provenance record for a source or generated artifact. `sourceIdentity`
can hold a path, crate/module key, or other producer-defined stable identity. -/
structure Origin where
  kind : OriginKind
  location : LocId
  sourceIdentity : Option String := none
  description : String := ""
  deriving Repr, BEq, Inhabited

/-- Strength of the correspondence claim between an origin and imported LIR. -/
inductive Trust where
  | authored
  | checked
  | assumed
  deriving Repr, BEq, Inhabited

/-- Evidence envelope connecting an LIR body to the artifact named by
`source`. The description states what was checked or assumed; it does not
alter the body's semantics. -/
structure Alignment where
  source : OriginId
  trust : Trust
  description : String
  deriving Repr, BEq, Inhabited

/-- Interned hierarchical namespace identity. Profiles interpret segments as
appropriate—for example, a Move address/alias/module tuple or Rust modules. -/
structure NamespaceRef where
  segments : Array String
  deriving Repr, BEq, Inhabited

/-- Interned declaration spelling paired with the namespace table entry that
owns it. A `NameId` indexes an array of these values. -/
structure QualifiedName where
  namespaceId : NamespaceId
  name : String
  deriving Repr, DecidableEq, Inhabited

/-- Semantic reference to an interned name, carrying its expected namespace
so validation can reject a mismatched `NameId`. -/
structure QualifiedRef where
  namespaceId : NamespaceId
  name : NameId
  deriving Repr, BEq, Inhabited

/-- Semantic language family governing the few runtime and static choices
which cannot be shared by Move and Rust. Known source families are first-class;
`ProfileId` is reserved for independently registered future extensions. -/
inductive Profile where
  | move
  | rust
  | extension (id : ProfileId)
  deriving Repr, BEq, DecidableEq, Hashable, Inhabited

/-- Profile-owned checked extension value for a construct outside the known
Move/Rust union. `tag` selects a closed meaning in the registered
`ProfileSchema`; `payload` carries its versioned arguments. It is not
unvalidated metadata or a substitute for adding a known core node. -/
structure ProfileValue where
  profile : Profile
  tag : String
  payload : String := ""
  deriving Repr, BEq, Inhabited

/-- Configuration of one semantic profile used by a compilation unit. Its
`profile` is the semantic key. For `.extension id`, the ID must equal this
configuration's position in `RawUnit.profiles`; known profiles are found by
their explicit constructor. Name and version select a compatible
implementation, while options fix profile-specific semantic choices. -/
structure ProfileConfig where
  profile : Profile
  name : String
  version : Nat := 1
  options : Array (String × String) := #[]
  deriving Repr, BEq, Inhabited

/-- Width class of a neutral integer type: an exact bit width, target pointer
width supplied by a profile, or mathematical/unbounded width. -/
inductive IntWidth where
  | bits (width : Nat)
  | pointer
  | unbounded
  deriving Repr, BEq, Inhabited

/-- Pointer widths supported by Rust compilation targets and executable LIR
integer semantics. Keeping this policy shared prevents profile validation,
preparation, and interpretation from accepting different target states. -/
def supportedTargetPointerWidth (width : Nat) : Bool :=
  width == 16 || width == 32 || width == 64

/-- The supported width a profile option spells, read as the literal it is
written as.  The supported set is closed and the option travels as text, so
comparing the spelling is both exact and something reduction can settle —
parsing it would leave the equation stuck on the string decoder. -/
def supportedTargetPointerWidth? (spelling : String) : Option Nat :=
  if spelling == "16" then some 16
  else if spelling == "32" then some 32
  else if spelling == "64" then some 64
  else none

/-- Mutability of a core reference type. Language profiles impose their own
aliasing, storage, and escape rules on these common modes. -/
inductive ReferenceKind where
  | shared
  | mutable
  deriving Repr, BEq, Inhabited

/-- Source or inference form of a core lifetime. Move frontends use
`inference` for elided lifetimes; future explicit Move and Rust syntax use
`parameter`, while `static` and local regions remain language-independent. -/
inductive LifetimeKind where
  | static
  | parameter (index : Nat)
  | inference
  | local
  deriving Repr, BEq, Inhabited

/-- One core lifetime identity. Constraint generation and region solutions
refer to its position in `Tables.lifetimes`; `name` retains an authored
spelling without determining semantics. -/
structure Lifetime where
  kind : LifetimeKind
  loc : LocId
  name : Option String := none
  deriving Repr, BEq, Inhabited

/-- Structural reference type shared by all profiles. The core owns its
referent and lifetime edges; `profile` selects the Move, Rust, or other rules
which govern aliasing and operational meaning. -/
structure ReferenceType where
  profile : Profile
  kind : ReferenceKind
  referent : TypeId
  lifetime : LifetimeId
  deriving Repr, BEq, Inhabited

/-- Closed compile-time value shared by declarations, literal patterns, and
raw switch cases. Language-specific constants use the checked `profile` case. -/
inductive ConstValue where
  | unit
  | bool (value : Bool)
  | character (value : Nat)
  | integer (value : Int)
  | address (value : String)
  | string (value : String)
  | bytes (value : Array UInt8)
  | vector (elements : Array ConstValue)
  | tuple (elements : Array ConstValue)
  | profile (value : ProfileValue)
  deriving Repr, BEq, Inhabited

/-- A use of an interned type at a particular source location. Locations live
on uses because the same `TypeId` may occur at many authored sites. -/
structure TypeUse where
  typeId : TypeId
  loc : LocId
  deriving Repr, BEq, Inhabited

/-- Actual argument supplied to a generic declaration. Type arguments retain
their occurrence location; lifetime IDs use the core lifetime table, while
evidence IDs are interpreted and checked by the corresponding profile. -/
inductive GenericArgument where
  | typeArg (value : TypeUse)
  | const (value : ConstValue)
  | lifetime (value : LifetimeId)
  | evidence (value : EvidenceId)
  deriving Repr, BEq, Inhabited

/-- Core storage/value abilities. Trait obligations are first-class
`GenericPredicate`s rather than being encoded as abilities. -/
inductive Ability where
  | copy
  | drop
  | store
  | key
  deriving Repr, BEq, Inhabited

/-- Application of a first-class core trait declaration. Its qualified name
and generic arguments are source-language neutral. -/
structure TraitRef where
  trait : QualifiedRef
  arguments : Array GenericArgument := #[]
  deriving Repr, BEq, Inhabited

/-- Generic constraints shared by Rust, a future Move trait system, and
Leaner source. Profile-specific constraints remain an explicitly checked
extension case. -/
inductive GenericPredicate where
  | ability (type : TypeId) (ability : Ability)
  | implements (type : TypeId) (trait : TraitRef)
  | associatedTypeEq (trait : TraitRef) (item : AssociatedItemId) (value : TypeId)
  | associatedConstEq (trait : TraitRef) (item : AssociatedItemId) (value : ConstValue)
  | lifetimeOutlives (longer shorter : LifetimeId)
  | constEq (left right : ConstValue)
  | profile (value : ProfileValue)
  deriving Repr, BEq, Inhabited

/-- Interned type node spanning the known Move/Rust union. Child `TypeId`s
index the containing compilation unit's `Tables.types` arena; only types outside
that known union use `profile`. -/
inductive Ty where
  | unit
  | never
  | bool
  | character
  | string
  | bytes
  | address
  | signer
  | integer (width : IntWidth) (signed : Bool)
  | tuple (elements : Array TypeId)
  | vector (element : TypeId) (length : Option ConstValue := none)
  | range
  | eventStore
  | typeDomain (type : TypeId)
  | resourceDomain (resource : NameId) (arguments : Option (Array TypeId) := none)
  | stateDomain
  | nominal (name : NameId) (arguments : Array GenericArgument)
  | function (arguments : Array TypeId) (result : TypeId) (abilities : Array Ability := #[])
  | typeParameter (index : Nat)
  | reference (value : ReferenceType)
  | profile (value : ProfileValue)
  deriving Repr, BEq, Inhabited

/-- Unicode scalar values exclude the surrogate range even though it lies
inside the 21-bit Unicode code-point space. -/
def isUnicodeScalar (value : Nat) : Bool :=
  value <= 0x10ffff && !(0xd800 <= value && value <= 0xdfff)

/-- Inclusive value bounds of a nonzero fixed-width integer type. Pointer
widths need a selected target and unbounded integers have no finite bounds. -/
def Ty.integerBounds? : Ty → Option (Int × Int)
  | .integer (.bits width) signed =>
      if width == 0 then none
      else if signed then
        let magnitude : Int := (2 : Int) ^ (width - 1)
        some (-magnitude, magnitude - 1)
      else
        some (0, (2 : Int) ^ width - 1)
  | _ => none

/-- Decide whether an integer value inhabits this integer type when its width
is source-independent. Target-pointer widths remain undecided until a profile
selects a target; mathematical integers accept every value. -/
def Ty.integerValueFits? (ty : Ty) (value : Int) : Option Bool :=
  match ty with
  | .integer .unbounded _ => some true
  | .integer .pointer _ => none
  | .integer (.bits _) _ => do
      let (lower, upper) ← ty.integerBounds?
      some (lower <= value && value <= upper)
  | _ => some false

/-- Structured value assigned inside an attribute. `qualifiedName` preserves
source forms whose qualification has not yet been resolved to a namespace. -/
inductive AttributeValue where
  | constant (value : ConstValue)
  | name (namespaceId : Option NamespaceId) (name : String)
  | qualifiedName (name : String)
  deriving Repr, BEq, Inhabited

/-- Unified declaration/specification annotation. Source pragmas and source
attributes differ only in their surface placement; both lower to recursive
call-style or assignment-style nodes. -/
inductive Attribute where
  | call (name : String) (arguments : Array Attribute) (loc : Option LocId := none)
  | assign (name : String) (value : AttributeValue) (loc : Option LocId := none)
  deriving Repr, BEq, Inhabited

/-- Source comment retained as provenance. `isDoc` identifies documentation
text and `ownLine` records whether only whitespace preceded it on its line. -/
structure Comment where
  loc : LocId
  text : String
  isDoc : Bool := false
  ownLine : Bool := false
  deriving Repr, BEq, Inhabited

/-- Compilation-unit interned tables referenced by all strong IDs in owned
namespaces and dependency interfaces. IDs are stable only within this table
snapshot and are bounds-checked before use. -/
structure Tables where
  files : Array SourceFile := #[]
  locations : Array Location := #[]
  origins : Array Origin := #[]
  alignments : Array Alignment := #[]
  lifetimes : Array Lifetime := #[]
  types : Array Ty := #[]
  namespaces : Array NamespaceRef := #[]
  names : Array QualifiedName := #[]
  deriving Repr, BEq, Inhabited

/-- Borrow mode for the neutral borrow operation. Profiles may add modes with
distinct rules, but may not introduce a parallel place or control language. -/
inductive BorrowKind where
  | immutable
  | mutable
  | profile (value : ProfileValue)
  deriving Repr, BEq, Inhabited

/-- Classification of a thrown, non-returning outcome. The active semantic
profile defines state behavior such as Move rollback or Rust panic cleanup. -/
inductive ThrowKind where
  | abort
  | panic
  | profile (value : ProfileValue)
  deriving Repr, BEq, Inhabited

/-- Closed call forms known to the Move/Rust semantic union. Operands of an
`invoke` begin with the callable expression; operands of `closure` are its
captures. `extension` is reserved for call forms outside this known union. -/
inductive CallKind where
  | function (callee : QualifiedRef)
  | constructor (constructor : QualifiedRef) (variant : Option String := none)
  | destructor (constructor : QualifiedRef) (variant : Option String := none)
  | closure (function : QualifiedRef)
  | invoke
  | extension (value : ProfileValue) (targets : Array QualifiedRef := #[])
  deriving Repr, BEq, Inhabited

/-- Preferred source notation for an otherwise normalized operation. This is
provenance and does not alter the operation's semantics. Known Move/Rust forms
are explicit; future source families use a checked extension value. -/
inductive SurfaceSyntax where
  | receiverCall
  | indexNotation
  | extension (value : ProfileValue)
  deriving Repr, BEq, Inhabited

/-- Core global-storage actions shared by Move resources and Rust-style
global cells. `contains` observes a key, `borrow` returns a reference into the
slot, `take` removes its value, and `publish` inserts a new value. The first
operation type instantiation identifies the stored resource family. -/
inductive GlobalKind where
  | contains
  | borrow (kind : BorrowKind)
  | take
  | publish
  deriving Repr, BEq, Inhabited

/-- Pure primitives shared by the known Move/Rust semantic union. These are
typed constructors rather than profile tags so every frontend and backend can
negotiate them directly. Plain fixed-width arithmetic is modular; a `checked*`
constructor makes overflow/domain failure and its `ThrowKind` explicit.
Operations whose semantics are not yet in the known union continue to use
`Operation.profile`. -/
inductive PrimitiveOperation where
  | tuple
  | vector
  /-- Construct a fixed vector by repeating one element. Its nonnegative
  element count is carried by the result's fixed-vector type. -/
  | repeatVector
  /-- Extend a vector with one element at its back, as a value: the operand
  is not modified. Growing a vector in place is this operation composed with
  a reference write, which is how Move's `vector::push_back` and its
  reference-taking siblings reach it. -/
  | pushVector
  /-- Exchange two elements of a vector, as a value. Aborts when either index
  is out of range, matching element access. -/
  | swapVector
  | length
  | index
  | slice
  | add
  | checkedAdd (failure : ThrowKind)
  | overflowingAdd
  | subtract
  | checkedSubtract (failure : ThrowKind)
  | overflowingSubtract
  | multiply
  | checkedMultiply (failure : ThrowKind)
  | overflowingMultiply
  | modulo
  | checkedModulo (failure : ThrowKind)
  | divide
  | checkedDivide (failure : ThrowKind)
  | bitwiseOr
  | bitwiseAnd
  | bitwiseXor
  | bitwiseNot
  | shiftLeft
  | checkedShiftLeft (failure : ThrowKind)
  | shiftRight
  | checkedShiftRight (failure : ThrowKind)
  | logicalAnd
  | logicalOr
  | equal
  | notEqual
  | less
  | greater
  | lessEqual
  | greaterEqual
  | logicalNot
  | negate
  | checkedNegate (failure : ThrowKind)
  | copyValue
  | moveValue
  | cast
  | checkedCast (failure : ThrowKind)
  | range
  | implies
  | equivalent
  | identical
  deriving Repr, BEq, Inhabited

/-- Value-level reference operations shared by Move and Rust. A checked
normalization pass rewrites eligible `borrow` inputs to the stronger
place-based `Operation.borrow`; retaining this typed form is preferable to a
profile tag while that pass is incomplete. `explicit` is source provenance
for an authored freeze/coercion. -/
inductive ReferenceOperation where
  | borrow (kind : BorrowKind)
  | dereference
  | freeze (explicit : Bool := false)
  | mutate
  /-- Validation-synthesized loan-death marker: the named loans die after
  the wrapped operand. Frontends never emit it; the raw checker rejects
  it. See `designs/prophetic-references.md`. -/
  | endLoan (loans : Array LoanId)
  deriving Repr, BEq, Inhabited

/-- Typed field and variant operations over nominal data. Qualified targets
are strong references rather than profile payload strings. -/
inductive DataOperation where
  | select (type : QualifiedRef) (field : String)
  | selectVariants (type : QualifiedRef) (fields : Array String)
  | testVariants (type : QualifiedRef) (variants : Array String)
  | discriminant (type : QualifiedRef)
  | updateField (type : QualifiedRef) (field : String)
  deriving Repr, BEq, Inhabited

/-- Optional pre/post state indexes carried by specification operations. -/
structure MemoryRange where
  pre : Option Nat := none
  post : Option Nat := none
  deriving Repr, BEq, Inhabited

/-- Provenance category for an explicit specification trace point. -/
inductive TraceKind where
  | user
  | automatic
  | subAutomatic
  deriving Repr, BEq, Inhabited

/-- Logical summaries of a first-class function value. The callable is the
first expression operand of `SpecOperation.behavior`; the remaining operands
are the summarized call's inputs and, for canonical two-state forms, result
and mutable-reference post-state slots. `writeOf` indexes mutable-reference
parameters only, matching the Move model rather than the complete parameter
list. -/
inductive BehaviorKind where
  | requiresOf
  | abortsOf
  | ensuresOf
  | resultOf
  | unchangedOf
  | foldsOf
  | writeOf (index : Nat)
  deriving Repr, BEq, Inhabited

/-- Shared Move/Rust specification operations. These remain logical-only until
M4 supplies their interpretation, but their identity and payload are fully
typed and never dispatched through profile strings. -/
inductive SpecOperation where
  | functionCall (function : QualifiedRef) (range : MemoryRange)
  | behavior (kind : BehaviorKind) (range : MemoryRange)
  | result (index : Nat)
  | typeValue
  | typeDomain
  | resourceDomain
  | stateDomain
  | global (label : Option Nat := none)
  | canModify
  | old
  | saveStateAnchor (label : Nat)
  | withStateAnchor (label : Nat)
  | foldsCaptureAnchor (label : Nat)
  | inlineCallSummary
  | trace (kind : TraceKind)
  | publish (range : MemoryRange)
  | remove (range : MemoryRange)
  | update (range : MemoryRange)
  | emptyVector
  | singletonVector
  | updateVector
  | concatVector
  | indexOfVector
  | containsVector
  | lengthVector
  | indexVector
  | sliceVector
  | inRange
  | inVectorRange
  | vectorRange
  | maxValue (width : Nat)
  | bitVectorToInt
  | intToBitVector
  | abortFlag
  | abortCode
  | wellFormed
  | boxValue
  | unboxValue
  | emptyEventStore
  | extendEventStore
  | eventStoreIncludes
  | eventStoreIncludedIn
  | noOp
  deriving Repr, BEq, Inhabited

/-- Normalized primitive applied by `ExprKind.operation`. Place-based common
operations and all known call forms are explicit. `profile` is reserved for
operations outside the known Move/Rust semantic union. -/
inductive Operation where
  | move (place : PlaceId)
  | copy (place : PlaceId)
  | borrow (kind : BorrowKind) (place : PlaceId)
  | read (place : PlaceId)
  | write (place : PlaceId)
  | call (kind : CallKind)
  | global (kind : GlobalKind)
  | primitive (kind : PrimitiveOperation)
  | reference (kind : ReferenceOperation)
  | data (kind : DataOperation)
  | specification (kind : SpecOperation)
  | assert
  | drop (place : PlaceId)
  | profile (value : ProfileValue) (targets : Array QualifiedRef := #[])
  deriving Repr, BEq, Inhabited

/-- Assignable or borrowable storage path. Child `PlaceId`s and index
`ExprId`s refer to the containing namespace's arenas, making nested fields,
indexes, dereferences, and enum downcasts explicit. -/
inductive Place where
  | localVar (localId : LocalId)
  | deref (base : PlaceId)
  /-- Project one field.  The owner is named, as a value-level selection
  names it, so resolving the projection is a static decision rather than a
  question about the value that happens to be at the base. -/
  | field (base : PlaceId) (owner : QualifiedRef) (field : NameId)
  | index (base : PlaceId) (index : ExprId)
  /-- Select `from..to` when `fromEnd` is false, or
  `from..length-to` when it is true. -/
  | subslice (base : PlaceId) (start stop : Nat) (fromEnd : Bool)
  | downcast (base : PlaceId) (variant : NameId)
  deriving Repr, BEq, Inhabited

/-- Logical quantifier understood by the common specification language, with
a profile extension point for operations such as profile-defined choice. -/
inductive QuantifierKind where
  | forall
  | exists
  | choose
  | chooseMin
  | profile (value : ProfileValue)
  deriving Repr, BEq, Inhabited

/-- One structured match arm. Its pattern, optional guard, and body are IDs in
the containing namespace's pattern and expression arenas. -/
structure MatchArm where
  pattern : PatternId
  guard : Option ExprId := none
  body : ExprId
  deriving Repr, BEq, Inhabited

/-- Pattern bound by a logical quantifier together with the expression that
defines its finite, type, resource, or other profile-defined domain. -/
structure QuantifierBinder where
  pattern : PatternId
  domain : ExprId
  deriving Repr, BEq, Inhabited

/-- Shared condition roles used by Move-style specifications for both Move and
Rust. Type-parameter spellings on namespace invariants are retained until the
specification generic binder migration replaces them with strong IDs. -/
inductive ConditionKind where
  | letPost (name : String)
  | letPre (name : String)
  | assertion
  | assumption
  | decreases
  | abortsIf
  | abortsWith
  | succeedsIf
  | emits
  | ensures
  | requires
  | structInvariant
  | functionInvariant
  | loopInvariant
  | globalInvariant (typeParameters : Array String := #[])
  | globalInvariantUpdate (typeParameters : Array String := #[])
  | schemaInvariant
  | axiom_ (typeParameters : Array String := #[])
  | update
  deriving Repr, BEq, Inhabited

/-- One clause of a contract or in-body specification. `kind` has a shared
Move/Rust meaning, the main expression is its proposition, and named auxiliary
expressions carry role-specific operands such as an abort code. -/
structure Condition where
  loc : LocId
  kind : ConditionKind
  properties : Array Attribute := #[]
  expression : ExprId
  auxiliary : Array (String × ExprId) := #[]
  deriving Repr, BEq, Inhabited

/-- Read/write footprint of an in-body specification block. Expression IDs
denote modified places or resources; `reads` names observed types, and the
`*All` flags explicitly represent unbounded sides of the frame. -/
structure Frame where
  modifies : Array ExprId := #[]
  reads : Array TypeUse := #[]
  modifiesAll : Bool := false
  readsAll : Bool := false
  deriving Repr, BEq, Inhabited

/-- Specifications embedded at a structured program point. `loc` locates the
LIR node, while `sourceLoc` can retain the authored specification range when
the surrounding node was generated. -/
structure SpecBlock where
  loc : LocId
  sourceLoc : Option LocId := none
  pragmas : Array Attribute := #[]
  conditions : Array Condition := #[]
  frame : Option Frame := none
  deriving Repr, BEq, Inhabited

/-- Structured executable and specification expression language. Every child
`ExprId`, `PatternId`, and `PlaceId` indexes an arena in the containing
`Namespace`; validated function bodies contain no raw CFG or `goto`. -/
inductive ExprKind where
  | value (value : ConstValue) (sourceConstant : Option String := none)
  | constant (constant : QualifiedRef)
  | localVar (localId : LocalId)
  | operation (operation : Operation) (instantiations : Array GenericArgument)
      (arguments : Array ExprId) (surface : Option SurfaceSyntax := none)
  | block (statements : Array ExprId) (result : Option ExprId)
  | letDecl (pattern : PatternId) (value : Option ExprId) (body : ExprId)
  | ifElse (condition : ExprId) (thenBranch : ExprId) (elseBranch : Option ExprId)
  | match_ (scrutinee : ExprId) (arms : Array MatchArm)
  | loop (label : Option String) (body : ExprId)
  | break_ (nest : Nat) (value : Option ExprId)
  | continue_ (nest : Nat)
  | return_ (values : Array ExprId)
  | throw_ (kind : ThrowKind) (arguments : Array ExprId)
  | assign (place : PlaceId) (value : ExprId)
  | assignPattern (pattern : PatternId) (value : ExprId)
  | quantifier (kind : QuantifierKind) (binders : Array QuantifierBinder)
      (triggers : Array (Array ExprId)) (condition : Option ExprId) (body : ExprId)
  | spec (block : SpecBlock)
  deriving Repr, BEq, Inhabited

/-- Located, typed expression arena node. The checker validates `typeId` and
all child IDs before a backend can receive the enclosing `ValidatedUnit`. -/
structure Expr where
  loc : LocId
  typeId : TypeId
  kind : ExprKind
  deriving Repr, BEq, Inhabited

/-- Binding and matching pattern language shared by declarations, `let`,
assignment destructuring, match arms, and quantifier binders. -/
inductive PatternKind where
  | wildcard
  | variable (localId : LocalId)
  | tuple (elements : Array PatternId)
  | constructor (name : NameId) (instantiations : Array GenericArgument)
      (variant : Option String) (fields : Array PatternId)
  | literal (value : ConstValue)
  | range (lower upper : Option ConstValue) (inclusive : Bool)
  deriving Repr, BEq, Inhabited

/-- Located, typed pattern arena node. Child patterns are referenced through
the containing namespace's `patterns` arena. -/
structure Pattern where
  loc : LocId
  typeId : TypeId
  kind : PatternKind
  deriving Repr, BEq, Inhabited

/-- Sort of a generic formal binder. First-class abilities and extension
predicates constrain its legal arguments without changing its binder sort. -/
inductive BinderKind where
  | typeArg
  | const
  | lifetime
  | evidence
  deriving Repr, BEq, Inhabited

/-- One named generic formal with its binder sort, first-class Move/Rust
abilities, remaining extension predicates, and authored location. -/
structure GenericBinder where
  name : String
  kind : BinderKind
  abilities : Array Ability := #[]
  predicates : Array GenericPredicate := #[]
  /-- Declared type of a const binder. It is absent for every other binder
  kind. Keeping it on the binder preserves source-level const declarations
  without changing the kind of a generic argument. -/
  type : Option TypeUse := none
  loc : LocId
  deriving Repr, BEq, Inhabited

/-- Function or specification-function parameter. Function parameters
correspond positionally to the leading entries of the declaration's `locals`
array. Trait methods have signatures but no body-local table, so parameter
declarations deliberately carry no `LocalId`. -/
structure Parameter where
  name : String
  typeUse : TypeUse
  mutable : Bool := false
  deriving Repr, BEq, Inhabited

/-- Declaration-local symbol available to expressions and patterns. Its `id`
must equal its array position; different declarations own separate local-ID
spaces even when they share a namespace expression arena. -/
structure LocalDecl where
  id : LocalId
  name : String
  type : TypeUse
  mutable : Bool := false
  loc : LocId
  deriving Repr, BEq, Inhabited

/-- Callable signature including generic formals, value parameters, one or
more result types, and profile-defined whole-signature predicates. -/
structure Signature where
  generics : Array GenericBinder := #[]
  parameters : Array Parameter := #[]
  results : Array TypeUse := #[]
  predicates : Array GenericPredicate := #[]
  deriving Repr, BEq, Inhabited

/-- Complete declaration-level function contract. Conditions preserve clause
order; frame presence is distinguished from an omitted frame, and wildcard
read/write permissions are explicit rather than inferred. -/
structure FunctionContract where
  loc : Option LocId := none
  conditions : Array Condition := #[]
  modifies : Array ExprId := #[]
  reads : Array TypeUse := #[]
  hasFrame : Bool := false
  modifiesAll : Bool := false
  readsAll : Bool := false
  pragmas : Array Attribute := #[]
  deriving Repr, BEq, Inhabited

/-- Named typed constant whose value is an expression-arena root. Profile data
retains semantic declaration properties not shared by every language. -/
structure ConstantDecl where
  loc : LocId
  name : NameId
  type : TypeUse
  value : ExprId
  doc : String := ""
  profileData : Array ProfileValue := #[]
  attributes : Array Attribute := #[]
  deriving Repr, BEq, Inhabited

/-- Named field and its located type, used by both structures and enum
variants. Field names are interned in the containing namespace's name table. -/
structure FieldDecl where
  loc : LocId
  name : NameId
  type : TypeUse
  doc : String := ""
  deriving Repr, BEq, Inhabited

/-- One enum variant with its own interned name, location, ordered payload
fields, and optional integer discriminant. The discriminant is its observable
value, not its position in the variant array. Plain structures leave the
enclosing `StructDecl.variants` empty. -/
structure VariantDecl where
  loc : LocId
  name : NameId
  fields : Array FieldDecl := #[]
  discriminant : Option Int := none
  deriving Repr, BEq, Inhabited

/-- Nominal data declaration representing either a structure or enum.
`abilities` carries Move abilities and Rust traits, `properties` retains only
extension semantics, and `contract` stores data invariants. -/
structure StructDecl where
  loc : LocId
  name : NameId
  doc : String := ""
  generics : Array GenericBinder := #[]
  fields : Array FieldDecl := #[]
  variants : Array VariantDecl := #[]
  abilities : Array Ability := #[]
  properties : Array ProfileValue := #[]
  locals : Array LocalDecl := #[]
  contract : FunctionContract := {}
  attributes : Array Attribute := #[]
  deriving Repr, BEq, Inhabited

/-- Executable function declaration parameterized by its construction-stage
body representation: `RawBody` at the frontend boundary and `FunctionBody`
after validation. Origin and alignment are provenance, while `profile`
selects semantics. -/
structure FunctionDecl (Body : Type) where
  loc : LocId
  name : NameId
  doc : String := ""
  profile : Profile
  signature : Signature
  body : Body
  origin : OriginId
  alignment : AlignmentId
  locals : Array LocalDecl := #[]
  contract : FunctionContract := {}
  pragmas : Array Attribute := #[]
  profileData : Array ProfileValue := #[]
  attributes : Array Attribute := #[]
  deriving Repr, BEq, Inhabited

/-- Kind and declaration data of one trait-associated item. Method bodies are
ordinary functions referenced by name so raw and validated namespaces do not
need a second body representation. -/
inductive AssociatedItemKind where
  | type (bounds : Array GenericPredicate := #[]) (default : Option TypeUse := none)
  | constant (type : TypeUse) (default : Option ExprId := none)
  | method (signature : Signature) (defaultImplementation : Option QualifiedRef := none)
  deriving Repr, BEq, Inhabited

/-- Namespace-local associated item owned by a first-class trait. -/
structure AssociatedItemDecl where
  id : AssociatedItemId
  loc : LocId
  owner : TraitDeclId
  name : NameId
  kind : AssociatedItemKind
  doc : String := ""
  attributes : Array Attribute := #[]
  deriving Repr, BEq, Inhabited

/-- First-class trait declaration shared by language profiles. Inheritance is
represented by `superTraits`; other where-clauses use core predicates. -/
structure TraitDecl where
  id : TraitDeclId
  loc : LocId
  name : NameId
  doc : String := ""
  generics : Array GenericBinder := #[]
  superTraits : Array TraitRef := #[]
  predicates : Array GenericPredicate := #[]
  associatedItems : Array AssociatedItemId := #[]
  attributes : Array Attribute := #[]
  deriving Repr, BEq, Inhabited

/-- Value supplied for one associated item by an implementation. -/
inductive AssociatedItemValue where
  | type (value : TypeUse)
  | constant (value : ExprId)
  | method (value : QualifiedRef)
  deriving Repr, BEq, Inhabited

structure AssociatedItemBinding where
  loc : LocId
  item : AssociatedItemId
  value : AssociatedItemValue
  deriving Repr, BEq, Inhabited

/-- First-class implementation declaration. Explicit proof-evidence plumbing
for selected implementations is deferred beyond the first Rust slice. -/
structure ImplDecl where
  id : ImplDeclId
  loc : LocId
  doc : String := ""
  generics : Array GenericBinder := #[]
  trait : TraitRef
  target : TypeUse
  predicates : Array GenericPredicate := #[]
  bindings : Array AssociatedItemBinding := #[]
  attributes : Array Attribute := #[]
  deriving Repr, BEq, Inhabited

/-- Logical/specification function with a physical typed signature and an
optional expression body. Profile data distinguishes uninterpreted, native,
old-state-using, and executable-function relationships. -/
structure SpecFunctionDecl where
  loc : LocId
  name : NameId
  doc : String := ""
  profile : Profile
  signature : Signature
  body : Option ExprId := none
  origin : OriginId
  locals : Array LocalDecl := #[]
  contract : FunctionContract := {}
  profileData : Array ProfileValue := #[]
  deriving Repr, BEq, Inhabited

/-- Namespace-level specification state variable. Its optional initializer is
an expression root and `locals` scopes any binders used by that expression. -/
structure SpecVarDecl where
  loc : LocId
  name : NameId
  generics : Array GenericBinder := #[]
  type : TypeUse
  profile : Profile
  init : Option ExprId := none
  locals : Array LocalDecl := #[]
  profileData : Array ProfileValue := #[]
  deriving Repr, BEq, Inhabited

/-- Namespace axiom or global/update invariant represented as a profile-tagged
condition. `locals` scopes quantified or elaboration-introduced symbols used
by the condition expression. -/
structure NamespaceInvariant where
  loc : LocId
  condition : Condition
  locals : Array LocalDecl := #[]
  deriving Repr, BEq, Inhabited

/-- One named role-to-declaration edge in an intrinsic model. The target is a
qualified semantic reference and the binding has its own diagnostic location. -/
structure IntrinsicBinding where
  loc : LocId
  role : String
  target : QualifiedRef
  deriving Repr, BEq, Inhabited

/-- Profile-owned intrinsic model attached to a nominal owner, with separate
role graphs for executable and specification declarations. Validation, not a
backend, determines whether the graph satisfies the model's role schema. -/
structure IntrinsicDecl where
  loc : LocId
  model : String
  owner : NameId
  profile : Profile
  executableBindings : Array IntrinsicBinding := #[]
  specBindings : Array IntrinsicBinding := #[]
  deriving Repr, BEq, Inhabited

/-- Complete declaration container and owner of namespace-local AST arenas.
All interned files, locations, types, lifetimes, namespaces, and names live in
the enclosing compilation unit's single `Tables` snapshot. `Body` selects the
raw or validated function stage. -/
structure Namespace (Body : Type) where
  loc : LocId
  identity : NamespaceId
  profile : Option Profile := none
  doc : String := ""
  expressions : Array Expr := #[]
  patterns : Array Pattern := #[]
  places : Array Place := #[]
  imports : Array NamespaceId := #[]
  profileMetadata : Array ProfileValue := #[]
  attributes : Array Attribute := #[]
  pragmas : Array Attribute := #[]
  constants : Array ConstantDecl := #[]
  structs : Array StructDecl := #[]
  associatedItems : Array AssociatedItemDecl := #[]
  traits : Array TraitDecl := #[]
  implementations : Array ImplDecl := #[]
  functions : Array (FunctionDecl Body) := #[]
  specFunctions : Array SpecFunctionDecl := #[]
  specVars : Array SpecVarDecl := #[]
  invariants : Array NamespaceInvariant := #[]
  intrinsics : Array IntrinsicDecl := #[]
  comments : Array Comment := #[]
  deriving Repr, BEq, Inhabited

/-- Rebuild a function declaration at a different body stage, preserving every
other field. -/
def FunctionDecl.withBody {α β : Type} (declaration : FunctionDecl α)
    (body : β) : FunctionDecl β where
  loc := declaration.loc
  name := declaration.name
  doc := declaration.doc
  profile := declaration.profile
  signature := declaration.signature
  body := body
  origin := declaration.origin
  alignment := declaration.alignment
  locals := declaration.locals
  contract := declaration.contract
  pragmas := declaration.pragmas
  profileData := declaration.profileData
  attributes := declaration.attributes

/-- Rebuild a namespace at a different body stage from replacement arenas and
converted function declarations, preserving every other field. -/
def Namespace.withStage {α β : Type} (ns : Namespace α) (expressions : Array Expr)
    (patterns : Array Pattern) (functions : Array (FunctionDecl β)) : Namespace β where
  loc := ns.loc
  identity := ns.identity
  profile := ns.profile
  doc := ns.doc
  expressions := expressions
  patterns := patterns
  places := ns.places
  imports := ns.imports
  profileMetadata := ns.profileMetadata
  attributes := ns.attributes
  pragmas := ns.pragmas
  constants := ns.constants
  structs := ns.structs
  associatedItems := ns.associatedItems
  traits := ns.traits
  implementations := ns.implementations
  functions := functions
  specFunctions := ns.specFunctions
  specVars := ns.specVars
  invariants := ns.invariants
  intrinsics := ns.intrinsics
  comments := ns.comments

end LeanerIR

-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

/-!
# Leaner source AST

This is the frontend-owned syntax tree for the profile-general Leaner
language.  It deliberately contains no `LeanerIR` arena identifiers: lowering
is the only boundary which interns these nodes into a `RawUnit`.
-/

namespace LeanerLang

/-- Half-open UTF-8 byte range in the containing Leaner source file. -/
structure Span where
  startByte : Nat := 0
  endByte : Nat := 0
  deriving Repr, BEq, Inhabited

/-- A source value paired with its authored range. -/
structure Located (α : Type) where
  value : α
  span : Span := {}
  deriving Repr, BEq, Inhabited

/-- Source comment retained across parsing, LIR lowering, and printing. The
text includes its delimiter so comments from different source profiles remain
lossless provenance. -/
structure Comment where
  text : String
  isDoc : Bool := false
  ownLine : Bool := true
  span : Span := {}
  deriving Repr, BEq, Inhabited

/-- Semantic profile selected by a namespace header. -/
inductive ProfileName where
  | move
  | rust
  deriving Repr, BEq, DecidableEq, Inhabited

inductive Ability where
  | copy
  | drop
  | store
  | key
  deriving Repr, BEq, DecidableEq, Inhabited

/-- Source spelling of a core lifetime. Parameter names retain their authored
apostrophe when present; inference is the ordinary elided form. -/
inductive SourceLifetime where
  | static
  | parameter (name : String)
  | inference
  | local (name : String)
  deriving Repr, BEq, Inhabited

mutual
/-- Profile-independent source types and their mixed-sort nominal arguments. -/
inductive Ty where
  | unit
  | never
  | bool
  | char
  | string
  | bytes
  | address
  | signer
  | uint (width : Nat)
  | sint (width : Nat)
  | uptr
  | iptr
  | nat
  | int
  | range
  | tuple (elements : Array (Located Ty))
  | vector (element : Located Ty) (length : Option Int := none)
  | function (arguments : Array (Located Ty)) (result : Located Ty)
      (abilities : Array Ability := #[])
  | reference (mutable : Bool) (referent : Located Ty)
      (lifetime : SourceLifetime := .inference)
  | named (segments : Array String) (arguments : Array TypeArgument)
  deriving Repr, BEq, Inhabited

inductive TypeArgument where
  | type (value : Ty)
  | constInteger (value : Int)
  | constBool (value : Bool)
  | lifetime (value : SourceLifetime)
  deriving Repr, BEq, Inhabited
end

inductive BinderKind where
  | type
  | const
  | lifetime
  | evidence
  deriving Repr, BEq, DecidableEq, Inhabited

structure GenericBinder where
  name : String
  kind : BinderKind
  type : Option (Located Ty) := none
  abilities : Array Ability := #[]
  /-- A type parameter no field of the declaration stores. Move spells this
  `phantom`; the marker is recorded as a profile predicate on the binder. -/
  phantom : Bool := false
  span : Span := {}
  deriving Repr, BEq, Inhabited

inductive ThrowKind where
  | abort
  | panic
  deriving Repr, BEq, DecidableEq, Inhabited

inductive QuantifierKind where
  | forall
  | exists
  deriving Repr, BEq, DecidableEq, Inhabited

inductive GlobalOperation where
  | contains
  | borrow (mutable : Bool)
  | take
  | publish
  deriving Repr, BEq, DecidableEq, Inhabited

/-- Source-level kind of a specification summary over a function value. -/
inductive BehaviorOperation where
  | requiresOf
  | abortsOf
  | ensuresOf
  | resultOf
  | unchangedOf
  | foldsOf
  | writeOf (index : Nat)
  deriving Repr, BEq, DecidableEq, Inhabited

/-- Optional numeric pre/post state labels on a specification operation. -/
structure SpecificationMemoryRange where
  pre : Option Nat := none
  post : Option Nat := none
  deriving Repr, BEq, DecidableEq, Inhabited

/-- Canonical specification operations whose state-anchor payload is semantic
and therefore must survive source round trips. -/
inductive SpecificationOperation where
  | behavior (kind : BehaviorOperation) (range : SpecificationMemoryRange := {})
  | old
  | saveStateAnchor (label : Nat)
  | withStateAnchor (label : Nat)
  | foldsCaptureAnchor (label : Nat)
  | result (index : Nat)
  | inlineCallSummary
  | global
  | typeDomain
  | emptyVector
  | singletonVector
  | updateVector
  | concatVector
  | indexOfVector
  | containsVector
  | lengthVector
  | indexVector
  | sliceVector
  | bitVectorToInt
  | intToBitVector
  | inRange
  | inVectorRange
  | vectorRange
  deriving Repr, BEq, DecidableEq, Inhabited

inductive SpecificationConditionKind where
  | let_ (name : String)
  | assertion
  | assumption
  | loopInvariant
  deriving Repr, BEq, DecidableEq, Inhabited

/-- Canonical primitive operation names. Surface operators elaborate to these
values only after a profile has selected identical semantics. -/
inductive Primitive where
  | tuple
  | vector
  | repeatVector (length : Nat)
  | pushVector
  | swapVector
  | length
  | index
  | slice
  | range
  | add
  | checkedAdd (failure : ThrowKind)
  | subtract
  | checkedSubtract (failure : ThrowKind)
  | multiply
  | checkedMultiply (failure : ThrowKind)
  | overflowingAdd
  | overflowingSubtract
  | overflowingMultiply
  | divide
  | checkedDivide (failure : ThrowKind)
  | modulo
  | checkedModulo (failure : ThrowKind)
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
  | logicalNot
  | equal
  | notEqual
  | less
  | greater
  | lessEqual
  | greaterEqual
  | negate
  | checkedNegate (failure : ThrowKind)
  | cast
  | checkedCast (failure : ThrowKind)
  | implies
  | equivalent
  | identical
  /-- Explicit copy or move of a *value*, as opposed to the place operations
  `copy(place)` and `move(place)`. -/
  | copyValue
  | moveValue
  /-- Arithmetic written with a profile-selected source operator. Explicit
  `core.prim.*` applications keep using the constructors above. -/
  | profileAdd
  | profileSubtract
  | profileMultiply
  | profileDivide
  | profileModulo
  | profileShiftLeft
  | profileShiftRight
  | profileNegate
  | profileCast
  deriving Repr, BEq, DecidableEq, Inhabited

/-- Authored storage paths.  Places remain distinct from expressions so a
borrow of `(*reference).field` cannot be confused with borrowing a computed
field value. -/
inductive Place where
  | local (name : String) (span : Span := {})
  | deref (base : Place) (span : Span := {})
  | field (base : Place) (name : String) (span : Span := {})
  deriving Repr, BEq, Inhabited

def Place.span : Place → Span
  | .local _ span | .deref _ span | .field _ _ span => span

inductive PlaceOperation where
  | move
  | copy
  | read
  deriving Repr, BEq, DecidableEq, Inhabited

/-- Binding patterns used by structured local declarations. Constructor
fields are named in source even though LIR stores them in declaration order. -/
inductive PatternLiteral where
  | bool (value : Bool)
  | char (value : Nat)
  | integer (value : Int)
  deriving Repr, BEq, Inhabited

inductive BindingPattern where
  | wildcard (span : Span := {})
  /-- `mutable` marks a binding the surrounding declaration may reassign,
  Rust's `let (mut a, b)`. A `let mut` declaration marks every binding. -/
  | variable (name : String) (span : Span := {}) (mutable : Bool := false)
  | literal (value : PatternLiteral) (span : Span := {})
  | tuple (elements : Array BindingPattern) (span : Span := {})
  | constructor (owner : Located Ty) (variant : Option String)
      (fields : Array (String × BindingPattern)) (span : Span := {})
  deriving Repr, BEq, Inhabited

def BindingPattern.span : BindingPattern → Span
  | .wildcard span | .variable _ span _ | .literal _ span | .tuple _ span |
      .constructor _ _ _ span => span

mutual
inductive Expr where
  | unit (span : Span := {})
  | bool (value : Bool) (span : Span := {})
  | char (value : Nat) (span : Span := {})
  | integer (value : Int) (span : Span := {})
  | typedInteger (value : Int) (type : Located Ty) (span : Span := {})
  | address (value : String) (span : Span := {})
  | string (value : String) (span : Span := {})
  | bytes (value : Array UInt8) (span : Span := {})
  | local (name : String) (span : Span := {})
  | primitive (operation : Primitive) (arguments : Array Expr) (span : Span := {})
  | typedPrimitive (operation : Primitive) (result : Located Ty)
      (arguments : Array Expr) (span : Span := {})
  | call (name : Array String) (arguments : Array Expr) (span : Span := {})
  | closure (name : Array String) (result : Located Ty)
      (captures : Array Expr) (span : Span := {})
  | invoke (callable : Expr) (arguments : Array Expr) (span : Span := {})
  | genericCall (name : Array String) (types : Array (Located Ty))
      (arguments : Array Expr) (span : Span := {})
  | typedCall (name : Array String) (result : Located Ty)
      (arguments : Array Expr) (span : Span := {})
  | typedGenericCall (name : Array String) (result : Located Ty)
      (types : Array (Located Ty)) (arguments : Array Expr) (span : Span := {})
  /-- Receiver notation retains an optional external result annotation because
  dependency interfaces are not required by the standalone source frontend. -/
  | methodCall (name : String) (result : Option (Located Ty))
      (types : Array (Located Ty)) (receiver : Expr) (arguments : Array Expr)
      (span : Span := {})
  | global (operation : GlobalOperation) (resource : Located Ty)
      (arguments : Array Expr) (span : Span := {})
  | construct (name : Array String) (arguments : Array Expr) (span : Span := {})
  | appliedConstruct (owner : Located Ty) (variant : Option String)
      (arguments : Array Expr) (span : Span := {})
  | namedConstruct (owner : Located Ty) (variant : Option String)
      (fields : Array (String × Expr)) (span : Span := {})
  | select (owner : Located Ty) (field : String) (value : Expr) (span : Span := {})
  | field (value : Expr) (name : String) (span : Span := {})
  /-- Move 2's context-sensitive `T[address]` / `value[index]` surface.
  Lowering resolves a one-segment uninstantiated head to a local first and
  otherwise treats it as a global resource type. -/
  | storageIndex (head : Located Ty) (index : Expr) (span : Span := {})
  | index (value index : Expr) (span : Span := {})
  | membership (element collection : Expr) (span : Span := {})
  | variantTest (value : Expr) (variants : Array String) (span : Span := {})
  | selectVariants (owner : Located Ty) (fields : Array String)
      (value : Expr) (span : Span := {})
  | testVariants (owner : Located Ty) (variants : Array String)
      (value : Expr) (span : Span := {})
  | discriminant (owner result : Located Ty) (value : Expr) (span : Span := {})
  | placeOperation (operation : PlaceOperation) (place : Expr) (span : Span := {})
  | borrowPlace (mutable : Bool) (place : Place) (span : Span := {})
  | dropPlace (place : Place) (span : Span := {})
  | borrowValue (mutable : Bool) (value : Expr) (span : Span := {})
  | freezeReference (explicit : Bool) (value : Expr) (span : Span := {})
  | dereference (value : Expr) (span : Span := {})
  | mutateReference (reference value : Expr) (span : Span := {})
  | quantifier (kind : QuantifierKind)
      (binders : Array (BindingPattern × Expr)) (body : Expr) (span : Span := {})
  | specification (operation : SpecificationOperation)
      (types : Array (Located Ty)) (arguments : Array Expr) (span : Span := {})
  | specBlock (conditions : Array (SpecificationConditionKind × Expr))
      (span : Span := {})
  | block (statements : Array Statement) (result : Option Expr) (span : Span := {})
  | ifElse (condition thenBranch : Expr) (elseBranch : Option Expr) (span : Span := {})
  | match_ (scrutinee : Expr)
      (arms : Array (BindingPattern × Option Expr × Expr)) (span : Span := {})
  /-- A half-open range loop. Bounds are evaluated once, in source order,
  before the iterator is introduced. Core LIR retains the equivalent
  let/loop/assignment form; this node is canonical surface syntax. -/
  | forRange (iterator : String) (lower upper body : Expr) (span : Span := {})
  | loop (body : Expr) (span : Span := {})
  | break_ (value : Option Expr) (span : Span := {})
  | continue_ (span : Span := {})
  | assign (place : Place) (value : Expr) (span : Span := {})
  /-- Assignment whose target uses expression-shaped index notation. -/
  | assignExpression (target value : Expr) (span : Span := {})
  | assignPattern (pattern : BindingPattern) (type : Located Ty)
      (value : Expr) (span : Span := {})
  | return_ (value : Expr) (span : Span := {})
  | throw_ (kind : ThrowKind) (arguments : Array Expr) (span : Span := {})
  deriving Repr, BEq, Inhabited

/-- Ordered entries in a structured block. A declaration scopes over every
following statement and the block result. -/
inductive Statement where
  | expression (value : Expr)
  | letDecl (mutable : Bool) (pattern : BindingPattern) (type : Option (Located Ty))
      (value : Expr) (span : Span := {})
  deriving Repr, BEq, Inhabited
end

def Expr.span : Expr → Span
  | .unit span | .bool _ span | .char _ span | .integer _ span |
      .typedInteger _ _ span | .address _ span |
      .string _ span | .bytes _ span | .local _ span |
      .primitive _ _ span | .typedPrimitive _ _ _ span | .call _ _ span |
      .closure _ _ _ span | .invoke _ _ span |
      .genericCall _ _ _ span | .typedCall _ _ _ span |
      .typedGenericCall _ _ _ _ span |
      .methodCall _ _ _ _ _ span |
      .global _ _ _ span |
      .construct _ _ span | .appliedConstruct _ _ _ span |
      .namedConstruct _ _ _ span |
      .select _ _ _ span | .field _ _ span | .storageIndex _ _ span |
      .index _ _ span |
      .membership _ _ span | .variantTest _ _ span | .selectVariants _ _ _ span |
      .testVariants _ _ _ span | .discriminant _ _ _ span |
      .placeOperation _ _ span | .block _ _ span |
      .borrowPlace _ _ span | .dropPlace _ span | .borrowValue _ _ span |
      .freezeReference _ _ span |
      .dereference _ span |
      .mutateReference _ _ span |
      .quantifier _ _ _ span |
      .specification _ _ _ span |
      .specBlock _ span |
      .ifElse _ _ _ span | .match_ _ _ span |
      .forRange _ _ _ _ span |
      .loop _ span | .break_ _ span | .continue_ span |
      .assign _ _ span | .assignExpression _ _ span | .assignPattern _ _ _ span |
      .return_ _ span | .throw_ _ _ span => span

structure Parameter where
  name : String
  type : Located Ty
  mutable : Bool := false
  span : Span := {}
  deriving Repr, BEq, Inhabited

inductive Visibility where
  | private_
  | public_
  | package
  | friend
  deriving Repr, BEq, DecidableEq, Inhabited

structure FunctionModifiers where
  visibility : Visibility := .private_
  isDeprecated : Bool := false
  isView : Bool := false
  isEntry : Bool := false
  isNative : Bool := false
  isOpaque : Bool := false
  deriving Repr, BEq, Inhabited

inductive ContractClause where
  | letPre (name : String) (expression : Expr) (properties : Array String := #[])
      (span : Span := {})
  | letPost (name : String) (expression : Expr) (properties : Array String := #[])
      (span : Span := {})
  | requires (expression : Expr) (properties : Array String := #[]) (span : Span := {})
  | ensures (expression : Expr) (properties : Array String := #[]) (span : Span := {})
  | abortsIf (expression : Expr) (code : Option Expr := none)
      (properties : Array String := #[]) (span : Span := {})
  | invariant (expression : Expr) (properties : Array String := #[]) (span : Span := {})
  | modifies (expression : Expr) (span : Span := {})
  | modifiesAll (span : Span := {})
  | reads (type : Located Ty) (span : Span := {})
  | readsAll (span : Span := {})
  deriving Repr, BEq, Inhabited

def ContractClause.span : ContractClause → Span
  | .letPre _ _ _ span | .letPost _ _ _ span |
      .requires _ _ span | .ensures _ _ span | .abortsIf _ _ _ span |
      .invariant _ _ span | .modifies _ span | .modifiesAll span |
      .reads _ span | .readsAll span => span

structure Pragma where
  name : String
  value : Expr := .bool true
  span : Span := {}
  deriving Repr, BEq, Inhabited

/-- One source declaration attribute: a name applied to name arguments —
the inverted source spelling of namespace-level intrinsic role graphs
(`@[intrinsic_map]` on the owner, `@[map_new (Owner)]` on a target). -/
structure SourceAttribute where
  name : String
  arguments : Array String := #[]
  span : Span := {}
  deriving Repr, BEq, Inhabited

structure FunctionDecl where
  name : String
  modifiers : FunctionModifiers := {}
  generics : Array GenericBinder := #[]
  parameters : Array Parameter := #[]
  result : Located Ty := { value := .unit }
  body : Option Expr := none
  contract : Array ContractClause := #[]
  pragmas : Array Pragma := #[]
  attributes : Array SourceAttribute := #[]
  span : Span := {}
  deriving Repr, BEq, Inhabited

/-- An authored logical function. Opaque declarations have no body; unlike an
executable opaque function, they remain callable only from specifications. -/
structure SpecFunctionDecl where
  name : String
  isOpaque : Bool := false
  generics : Array GenericBinder := #[]
  parameters : Array Parameter := #[]
  result : Located Ty := { value := .unit }
  body : Option Expr := none
  attributes : Array SourceAttribute := #[]
  span : Span := {}
  deriving Repr, BEq, Inhabited

structure ConstantDecl where
  name : String
  type : Located Ty
  value : Expr
  span : Span := {}
  deriving Repr, BEq, Inhabited

structure FieldDecl where
  name : String
  type : Located Ty
  span : Span := {}
  deriving Repr, BEq, Inhabited

structure StructDecl where
  name : String
  generics : Array GenericBinder := #[]
  fields : Array FieldDecl := #[]
  abilities : Array Ability := #[]
  contract : Array ContractClause := #[]
  pragmas : Array Pragma := #[]
  attributes : Array SourceAttribute := #[]
  span : Span := {}
  deriving Repr, BEq, Inhabited

structure VariantDecl where
  name : String
  fields : Array FieldDecl := #[]
  discriminant : Option Int := none
  span : Span := {}
  deriving Repr, BEq, Inhabited

structure EnumDecl where
  name : String
  generics : Array GenericBinder := #[]
  variants : Array VariantDecl := #[]
  abilities : Array Ability := #[]
  contract : Array ContractClause := #[]
  pragmas : Array Pragma := #[]
  attributes : Array SourceAttribute := #[]
  span : Span := {}
  deriving Repr, BEq, Inhabited

inductive Item where
  | constant (declaration : ConstantDecl)
  | struct (declaration : StructDecl)
  | enum (declaration : EnumDecl)
  | function (declaration : FunctionDecl)
  | specFunction (declaration : SpecFunctionDecl)
  deriving Repr, BEq, Inhabited

/-- Another namespace granted privileged (`friend`) access to this one. Move
spells this as a module-level `friend` declaration; other profiles map it onto
their own restricted-visibility relation. -/
structure FriendDecl where
  path : Array String
  span : Span := {}
  deriving Repr, BEq, Inhabited

/-- One profile-pure namespace. Cross-profile adapters are intentionally not
part of the initial source AST. -/
structure Namespace where
  path : Array String
  profile : ProfileName
  /-- Documentation owned by the namespace/module declaration. It is printed
  immediately before the declaration header, never as the first body item. -/
  doc : String := ""
  /-- Imported symbol or namespace paths. The final segment is the name made
  available in this namespace. -/
  uses : Array (Array String) := #[]
  /-- Namespaces granted `friend` access to this namespace. -/
  friends : Array FriendDecl := #[]
  pragmas : Array Pragma := #[]
  comments : Array Comment := #[]
  items : Array Item := #[]
  span : Span := {}
  deriving Repr, BEq, Inhabited

/-- A source file may own multiple namespaces but uses one provenance file. -/
structure CompilationUnit where
  sourceName : String
  namespaces : Array Namespace
  deriving Repr, BEq, Inhabited

end LeanerLang

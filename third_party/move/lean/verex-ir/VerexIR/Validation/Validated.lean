-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Syntax
import VerexIR.Validation.Resolve

/-!
# Validated VIR backend boundary

This module contains the checked forms exposed to backends. Their public
structure is separate from raw frontend input, and `ValidatedUnit` retains a
private constructor so only the shared checker can create one.
-/

namespace VerexIR.Validation

/-- Whether an operation reads through a mutable reference operand without
passing it on: it neither lends it to a callee, writes through it, nor makes
it part of its value. A field selection whose value is a reference passes
its operand on, projected. -/
def readsThroughReference (types : Array Ty) (operation : Operation) (result : TypeId) : Bool :=
  match operation with
  | .reference .dereference | .reference (.freeze _) => true
  | .data _ => !(types[result.index]? matches some (.reference _))
  | .primitive kind => !(kind matches .tuple | .vector | .repeatVector | .copyValue | .moveValue)
  | _ => false

/-- The direct expression children of one expression node, in evaluation
order. Shared by the structural checks and the semantic-preparation
rewrites. -/
def expressionChildren : ExprKind → Array ExprId
  | .value _ _ | .constant _ | .localVar _ | .continue_ _ => #[]
  | .operation _ _ arguments _ => arguments
  | .block statements result => match result with | some id => statements.push id | none => statements
  | .letDecl _ value body => match value with | some value => #[value, body] | none => #[body]
  | .ifElse condition thenBranch elseBranch =>
      match elseBranch with | some id => #[condition, thenBranch, id] | none => #[condition, thenBranch]
  | .match_ scrutinee arms =>
      arms.foldl (fun ids arm => match arm.guard with
        | some guard => (ids.push guard).push arm.body
        | none => ids.push arm.body) #[scrutinee]
  | .loop _ body => #[body]
  | .break_ _ value => value.toArray
  | .return_ values | .throw_ _ values => values
  | .assign _ value => #[value]
  | .assignPattern _ value => #[value]
  | .quantifier _ binders triggers condition body =>
      let ids := binders.foldl (fun ids binder => ids.push binder.domain) #[]
      let ids := triggers.foldl (fun ids trigger => ids ++ trigger) ids
      match condition with | some condition => (ids.push condition).push body | none => ids.push body
  | .spec block =>
      let ids := block.conditions.foldl (fun ids condition =>
        condition.auxiliary.foldl (fun ids auxiliary => ids.push auxiliary.2)
          (ids.push condition.expression)) #[]
      match block.frame with | some frame => ids ++ frame.modifies | none => ids

/-- Backend-visible function body after validation. Raw graphs have been
eliminated, leaving only an absent or structured expression root. -/
inductive FunctionBody where
  | absent
  | structured (root : ExprId)
  deriving Repr, BEq, DecidableEq, Inhabited

/-- The order `compare` gives the unit's values, by namespace and
declaration: the variant names of every nominal declaration in declaration
order, and every function's position in the order of qualified names. -/
structure ValueOrders where
  variants : Array (Array (Array String)) := #[]
  functions : Array (Array Nat) := #[]
  deriving Repr, BEq, Inhabited

/-- How a local validation adds holds a mutable reference no source local
holds (`holdMutableReferences`): for the operation that reads through it,
or until it dies discarded. -/
inductive Holding where
  | read
  | dropped
  deriving Repr, BEq, DecidableEq, Inhabited

/-- The sizes of a unit's tables. -/
structure TableSizes where
  types : Nat := 0
  names : Nat := 0
  namespaces : Nat := 0
  lifetimes : Nat := 0
  deriving Repr, BEq, DecidableEq, Inhabited

/-- Where a namespace's identifiers are in its unit's tables (θ): the unit
index of each entry of the namespace's own tables, per semantic kind. An
identifier θ does not cover is placed past the unit's tables (`beyond`), so
it names no entry and no other identifier. -/
structure Links where
  types : Array Nat := #[]
  names : Array Nat := #[]
  namespaces : Array Nat := #[]
  lifetimes : Array Nat := #[]
  beyond : TableSizes := {}
  deriving Repr, BEq, Inhabited

/-- The sizes of some tables. -/
def TableSizes.of (tables : Tables) : TableSizes :=
  { types := tables.types.size, names := tables.names.size
    namespaces := tables.namespaces.size, lifetimes := tables.lifetimes.size }

namespace Links

variable (links : Links)

/-- The unit identifier of a type identifier. -/
def typeId (id : TypeId) : TypeId :=
  ⟨links.types.getD id.index (links.beyond.types + id.index)⟩

/-- The unit identifier of a name identifier. -/
def nameId (id : NameId) : NameId :=
  ⟨links.names.getD id.index (links.beyond.names + id.index)⟩

/-- The unit identifier of a namespace identifier. -/
def namespaceId (id : NamespaceId) : NamespaceId :=
  ⟨links.namespaces.getD id.index (links.beyond.namespaces + id.index)⟩

/-- The unit identifier of a lifetime identifier. -/
def lifetimeId (id : LifetimeId) : LifetimeId :=
  ⟨links.lifetimes.getD id.index (links.beyond.lifetimes + id.index)⟩

end Links

/-- The links of a namespace whose body is numbered in its unit's tables. -/
def Links.identity (tables : Tables) : Links :=
  { types := Array.range tables.types.size, names := Array.range tables.names.size
    namespaces := Array.range tables.namespaces.size
    lifetimes := Array.range tables.lifetimes.size, beyond := .of tables }

/-- How one raw CFG block is represented by the recovered expression tree.
A block may occur at multiple tree positions when structurization duplicates
a shared region. An empty array is reserved for a compiler-generated
impossible switch default whose block terminates in `unreachable`. -/
structure StructurizedBlock where
  block : BlockId
  expressions : Array ExprId
  deriving Repr, BEq, Inhabited

/-- One reachable raw CFG edge retained by structurization. Region records
below explain the structured control node which owns selection and loop
edges; retaining the complete edge set makes omission and mutation visible. -/
structure StructurizedEdge where
  source : BlockId
  target : BlockId
  deriving Repr, BEq, Inhabited

/-- Structured control construct recovered at a raw CFG block. -/
inductive StructurizedRegionKind where
  | branch
  | switch
  | loop
  | callContinuation
  deriving Repr, BEq, Inhabited

/-- Correspondence between one recovered control region and its raw header.
Loop records additionally retain their complete natural-loop membership and
unique nonterminal exit. -/
structure StructurizedRegion where
  kind : StructurizedRegionKind
  header : BlockId
  expression : ExprId
  members : Array BlockId := #[]
  exit : Option BlockId := none
  deriving Repr, BEq, Inhabited

/-- Durable, checked correspondence between an admitted raw CFG and the
structured function body exposed to backends. -/
structure StructurizationWitness where
  namespaceId : NamespaceId
  functionId : FunctionId
  alignment : AlignmentId
  entry : BlockId
  root : ExprId
  blocks : Array StructurizedBlock
  edges : Array StructurizedEdge
  regions : Array StructurizedRegion
  deriving Repr, BEq, Inhabited

/-- Receipt for the path-sensitive local-path initialization analysis.
Presence attests that every analyzed local read is initialized on every
reaching structured path. The root and initial parameter set make the checked
function/input state explicit to verification consumers. -/
structure InitializationCertificate where
  namespaceId : NamespaceId
  functionId : FunctionId
  root : ExprId
  parameterLocals : Array LocalId
  localCount : Nat
  deriving Repr, BEq, Inhabited

/-- Reference capability entering a function through a parameter. -/
structure ReferenceParameterFact where
  localId : LocalId
  kind : ReferenceKind
  lifetime : LifetimeId
  deriving Repr, BEq, Inhabited

/-- One step of a path into a loaned or holding place. -/
inductive LoanProjection where
  | dereference
  | field (name : NameId)
  | index (literal : Option Nat)
  | subslice (start stop : Nat) (fromEnd : Bool)
  | downcast (variant : NameId)
  deriving Repr, BEq, Inhabited

/-- One recorded death point of a loan: its write-back must run before
(`before := true`) or after the anchor expression evaluates, and what holds it
there. Function-result boundaries explicitly record every mutable loan not
carried by the returned value. A loan may carry several records when its death is
branch-dependent; ending a loan is conditional and idempotent, so the
over-approximation is sound. Only a loan carried by a returned reference remains
live for frame finalization to transfer across the call boundary. -/
structure LoanDeath where
  anchor : ExprId
  before : Bool := false
  /-- The locals holding the loan where it dies. A place holding it elsewhere may
  hold another reference here, whose prophecy this death says nothing about. -/
  holders : Array LocalId := #[]
  /-- Where an aggregate holder holds the loan: a local and the path from it to
  the reference. Empty where a holder is itself a reference. -/
  holderPaths : Array (LocalId × Array LoanProjection) := #[]
  /-- The loan goes on through another holder: only these places die, and they resolve
  what they hold. The run ends no loan here. -/
  alone : Bool := false
  deriving Repr, BEq, Inhabited

/-- Stable identity and lifetime of a loan site accepted by the borrow
analysis. The validated unit retains its kind and place; keeping those
canonical facts in one place also avoids copying recursive place data into
every semantic wrapper. Mutable loans record their death points, where the
semantics ends them (`loanDeathsAt`). -/
structure CheckedLoanFact where
  expression : ExprId
  lifetime : LifetimeId
  deaths : Array LoanDeath := #[]
  deriving Repr, BEq, Inhabited

/-- One solved outlives relation. Besides declared predicates, certificates
contain reflexive facts, `static` axioms, and their deterministic transitive
closure. -/
structure LifetimeRelationFact where
  longer : LifetimeId
  shorter : LifetimeId
  deriving Repr, BEq, Inhabited

/-- Receipt retained by the successful borrow analysis. This initial
certificate covers local-rooted loans and conservative whole-path conflict
checking. -/
structure BorrowCertificate where
  namespaceId : NamespaceId
  functionId : FunctionId
  root : ExprId
  parameters : Array ReferenceParameterFact
  loans : Array CheckedLoanFact
  lifetimeRelations : Array LifetimeRelationFact
  /-- Reference carriers retired before a write replaces them. Unlike a local
  loan death, these can name references lent by the caller, without a lexical
  loan id in this function. The paths identify the old carriers after the
  right-hand side has run. -/
  carrierDeaths : Array LoanDeath := #[]
  /-- Reference parameters checked not to escape, conditional on scoped dictionary requirements. -/
  receiverNoEscape : Array Nat := #[]
  /-- The body's places that dereference a shared reference, ascending. A
  shared reference is the observed value itself, so the semantics reads
  such a place as its base. -/
  sharedDereferences : Array PlaceId := #[]
  /-- Whether the body writes, moves or mutably borrows a local that supplies
  the element index of a place some mutable loan borrows, at a point where
  the borrow analysis has that loan active.  The semantics reads such an index
  again where the loan dies, while the reference denotes the element chosen
  at the borrow, so a function with this fact has no carried denotation
  (`compileFunctionIn`).  Kept per function rather than as a diagnostic, which
  would refuse the whole unit. -/
  writesIndexUnderLoan : Bool := false
  deriving Repr, BEq, Inhabited

/-- A diagnostic of one namespace's function. -/
structure OwnedDiagnostic where
  namespaceId : NamespaceId
  diagnostic : Diagnostic
  deriving Repr, BEq, Inhabited

/-- Checked backend input namespace whose functions contain only structured
bodies (or are explicitly absent). Everything it holds is numbered in its own
tables (`localTables`): the body, the holders, and the validation results.
`tables` is an immutable cached view of the owning compilation-unit table,
whose entries hold unit identifiers; the body's identifiers reach it through
`links`. Provenance (files, locations, origins, alignments) is read in the
namespace's own tables only. -/
structure ValidatedNamespace extends Namespace FunctionBody where
  tables : Tables
  links : Links
  /-- The namespace's own tables. Their semantic entries are what `links`
  translates from; content is read in `tables` through `links`. -/
  localTables : Tables
  /-- The unit's value orders. A cached unit view, as `tables` is. -/
  orders : ValueOrders := {}
  /-- The patterns binding the locals validation added to hold mutable
  references, with how each holds its reference. -/
  holders : Array (PatternId × Holding) := #[]
  /-- The structurization witnesses of its functions. -/
  witnesses : Array StructurizationWitness := #[]
  initializationCertificates : Array InitializationCertificate := #[]
  borrowCertificates : Array BorrowCertificate := #[]
  /-- Diagnostics of its functions the borrow analysis could not certify. The
  analysis still lacks non-lexical loan death and region solving, so its
  rejections stay preparation-stage errors instead of failing `validate`. -/
  borrowRejections : Array OwnedDiagnostic := #[]
  deriving Repr, BEq, Inhabited

namespace ValidatedNamespace

variable (ns : ValidatedNamespace)

/-- The unit identifier of a type identifier of the body. -/
def typeId (id : TypeId) : TypeId := ns.links.typeId id

/-- The unit identifier of a name identifier of the body. -/
def nameId (id : NameId) : NameId := ns.links.nameId id

/-- The unit identifier of a namespace identifier of the body. -/
def namespaceId (id : NamespaceId) : NamespaceId := ns.links.namespaceId id

/-- The unit identifier of a lifetime identifier of the body. -/
def lifetimeId (id : LifetimeId) : LifetimeId := ns.links.lifetimeId id

/-- The type a type identifier of the body names, read in the unit's tables. -/
abbrev typeOf (id : TypeId) : Option Ty := ns.tables.types[(ns.typeId id).index]?

/-- The name a name identifier of the body names, read in the unit's tables. -/
abbrev nameOf (id : NameId) : Option QualifiedName := ns.tables.names[(ns.nameId id).index]?

/-- The namespace a namespace identifier of the body names. -/
abbrev namespaceOf (id : NamespaceId) : Option NamespaceRef :=
  ns.tables.namespaces[(ns.namespaceId id).index]?

/-- A qualified reference of the body, as unit identifiers. -/
def qualifiedRef (reference : QualifiedRef) : QualifiedRef :=
  { namespaceId := ns.namespaceId reference.namespaceId, name := ns.nameId reference.name }

/-- A generic argument of the body, its type or lifetime as a unit identifier. -/
def genericArgument : GenericArgument → GenericArgument
  | .typeArg value => .typeArg { value with typeId := ns.typeId value.typeId }
  | .lifetime value => .lifetime (ns.lifetimeId value)
  | argument => argument

/-- Generic arguments of the body, their types and lifetimes as unit identifiers. -/
def genericArguments (arguments : Array GenericArgument) : Array GenericArgument :=
  arguments.map ns.genericArgument

@[simp] theorem size_genericArguments (arguments : Array GenericArgument) :
    (ns.genericArguments arguments).size = arguments.size := by
  simp [genericArguments]

theorem getElem?_genericArguments (arguments : Array GenericArgument) (index : Nat) :
    (ns.genericArguments arguments)[index]? = arguments[index]?.map ns.genericArgument := by
  simp [genericArguments]

@[simp] theorem genericArguments_typeArg (value : TypeUse) :
    ns.genericArguments #[.typeArg value] =
      #[.typeArg { value with typeId := ns.typeId value.typeId }] := by
  simp [genericArguments, genericArgument]

/-- The namespace's own unit identifier. -/
abbrev unitIdentity : NamespaceId := ns.namespaceId ns.identity

/-- The loan facts of the borrow certificate of a function. -/
def loanFacts (functionId : FunctionId) : Array CheckedLoanFact :=
  -- A list search: the kernel searches an array by reading each element
  -- through the array's list.
  ((ns.borrowCertificates.toList.find? fun certificate =>
    certificate.functionId == functionId).map (·.loans)).getD #[]

/-- Checked receiver slots of a body, conditional on its scoped dictionaries. -/
def receiverNoEscape (functionId : FunctionId) : Array Nat :=
  ((ns.borrowCertificates.toList.find? fun certificate =>
    certificate.functionId == functionId).map (·.receiverNoEscape)).getD #[]

/-- Where a location of the namespace is, in its own tables. -/
def location? (loc : LocId) : Option Location := ns.localTables.locations[loc.index]?

theorem typeId_of_lt {id : TypeId} (lt : id.index < ns.links.types.size) :
    ns.typeId id = ⟨ns.links.types[id.index]⟩ := by
  simp [typeId, Links.typeId, Array.getD, lt]

theorem nameId_of_lt {id : NameId} (lt : id.index < ns.links.names.size) :
    ns.nameId id = ⟨ns.links.names[id.index]⟩ := by
  simp [nameId, Links.nameId, Array.getD, lt]

theorem namespaceId_of_lt {id : NamespaceId} (lt : id.index < ns.links.namespaces.size) :
    ns.namespaceId id = ⟨ns.links.namespaces[id.index]⟩ := by
  simp [namespaceId, Links.namespaceId, Array.getD, lt]

end ValidatedNamespace

private def nameIn (tables : Tables) (id : NameId) : String :=
  ((tables.names[id.index]?).map (·.name)).getD ""

/-- Two qualified function names in the order Move compares function values:
namespace path, addresses by the number they spell, then the function name. -/
private def compareQualified (left right : Array String × String) : Ordering :=
  (List.compareLex compareAddress left.1.toList right.1.toList).then (compare left.2 right.2)

/-- The value orders of a unit's namespaces, each in its own tables. -/
def valueOrdersOf (namespaces : Array (Tables × Namespace FunctionBody)) : ValueOrders :=
  let variants := namespaces.map fun (tables, ns) => ns.structs.map fun declaration =>
    declaration.variants.map fun variant => nameIn tables variant.name
  let keys := namespaces.toList.zipIdx.flatMap fun ((tables, ns), namespaceIndex) =>
    let path := (tables.namespaces[ns.identity.index]?.map (·.segments)).getD #[]
    ns.functions.toList.zipIdx.map fun (function, functionIndex) =>
      ((path, nameIn tables function.name), namespaceIndex, functionIndex)
  let sorted := keys.mergeSort fun left right => compareQualified left.1 right.1 != .gt
  let functions := sorted.zipIdx.foldl
      (init := namespaces.map fun (_, ns) => Array.replicate ns.functions.size 0)
    fun ranks ((_, namespaceIndex, functionIndex), rank) =>
      ranks.modify namespaceIndex (·.set! functionIndex rank)
  { variants, functions }

/-- Checked dependency summary copied into `ValidatedUnit` after unit
validation. It is intentionally distinct from the frontend-constructible raw
form so its schema can grow checked indexes and resolved declarations. -/
structure ValidatedNamespaceInterface where
  namespaceId : NamespaceId
  profile : Option Profile := none
  exportedNames : Array NameId := #[]
  /-- Checked nominal declarations this dependency exports; see
  `RawNamespaceInterface.structs`. -/
  structs : Array StructDecl := #[]
  /-- Checked function signatures this dependency exports, without bodies. -/
  functions : Array (FunctionDecl FunctionBody) := #[]
  /-- Checked specification-function signatures, likewise without bodies. -/
  specFunctions : Array SpecFunctionDecl := #[]
  /-- Its checked scopes; see `RawNamespaceInterface.scopes`. -/
  scopes : Array ScopeDecl := #[]
  deriving Repr, BEq, Inhabited

/-- Checked copy of an upstream producer/check/assumption statement. This is
retained for backend and theorem reporting; `trusted` remains an explicit
assumption marker and is not upgraded into proof by VIR validation. -/
structure ValidatedImportEvidence where
  producer : String
  description : String
  trusted : Bool
  deriving Repr, BEq, Inhabited

/-- Derived unit summary created by validation. These counts are the initial
index surface and can be extended without making frontends construct them. -/
structure UnitIndexes where
  namespaceCount : Nat
  functionCounts : Array Nat
  deriving Repr, BEq, Inhabited

/-- Only public backend input. Its constructor is private; the shared checker
creates it after bounds, arena, declaration, profile, CFG, resolution, typing,
and initialization/borrow validation, and records derived indexes and
analysis certificates alongside structured namespaces. -/
structure ValidatedUnit where
  private mk ::
  tables : Tables
  profiles : Array ProfileConfig
  namespaces : Array ValidatedNamespace
  dependencies : Array ValidatedNamespaceInterface
  evidence : Array ValidatedImportEvidence
  indexes : UnitIndexes
  resolution : ResolutionIndex
  deriving Repr, BEq, Inhabited

/-- The type at a unit identifier. -/
def ValidatedUnit.tyAt (unit : ValidatedUnit) (id : TypeId) : Option Ty :=
  unit.tables.types[id.index]?

/-- The name at a unit identifier. -/
def ValidatedUnit.nameAt (unit : ValidatedUnit) (id : NameId) : Option QualifiedName :=
  unit.tables.names[id.index]?

/-- The namespace reference at a unit identifier. -/
def ValidatedUnit.namespaceAt (unit : ValidatedUnit) (id : NamespaceId) : Option NamespaceRef :=
  unit.tables.namespaces[id.index]?

namespace ValidatedUnit

variable (unit : ValidatedUnit)

/-- The namespaces' structurization witnesses, each in its namespace's tables. -/
def structurizationWitnesses : Array StructurizationWitness :=
  unit.namespaces.flatMap (·.witnesses)

/-- The namespaces' initialization certificates, each in its namespace's tables. -/
def initializationCertificates : Array InitializationCertificate :=
  unit.namespaces.flatMap (·.initializationCertificates)

/-- The namespaces' borrow certificates, each in its namespace's tables. -/
def borrowCertificates : Array BorrowCertificate :=
  unit.namespaces.flatMap (·.borrowCertificates)

/-- The borrow analysis's rejections, each in its namespace's tables. -/
def borrowRejections : Array OwnedDiagnostic :=
  unit.namespaces.flatMap (·.borrowRejections)

/-- The borrow analysis's rejections, with the namespace whose tables
locate each. -/
def ownedBorrowDiagnostics : Array (ValidatedNamespace × Diagnostic) :=
  unit.namespaces.flatMap fun ns => ns.borrowRejections.map (ns, ·.diagnostic)

/-- The borrow analysis's rejections. -/
def borrowDiagnostics : Array Diagnostic :=
  unit.borrowRejections.map (·.diagnostic)

/-- Whether the links of every namespace are consistent with the unit: past
its tables is `beyond`, each link table covers its namespace's own table, and
every link is a unit identifier. -/
def linksConsistent : Bool :=
  let sizes := TableSizes.of unit.tables
  unit.namespaces.all fun ns =>
    ns.links.beyond == sizes &&
      ns.links.types.size == ns.localTables.types.size &&
      ns.links.names.size == ns.localTables.names.size &&
      ns.links.namespaces.size == ns.localTables.namespaces.size &&
      ns.links.lifetimes.size == ns.localTables.lifetimes.size &&
      ns.links.types.all (· < sizes.types) && ns.links.names.all (· < sizes.names) &&
      ns.links.namespaces.all (· < sizes.namespaces) &&
      ns.links.lifetimes.all (· < sizes.lifetimes)

end ValidatedUnit

/-- Whether a place of a namespace dereferences a shared reference, as the
borrow certificate of its function records. -/
def sharedDereference (unit : ValidatedUnit) (namespaceId : NamespaceId) (place : PlaceId) :
    Bool :=
  (unit.namespaces[namespaceId.index]?).any fun ns =>
    ns.borrowCertificates.any (·.sharedDereferences.contains place)

/-- Whether a place goes through a dereference the certificate records as shared:
what it reads of a mutable reference is what that refers to. -/
def placeThroughShared (unit : ValidatedUnit) (ns : ValidatedNamespace) : Nat → PlaceId → Bool
  | 0, _ => false
  | fuel + 1, place =>
      match ns.places[place.index]? with
      | some (.deref base) =>
          sharedDereference unit ns.unitIdentity place || placeThroughShared unit ns fuel base
      | some (.field base _ _) | some (.ghostField base _ _) | some (.index base _)
      | some (.subslice base ..) | some (.unsize base) | some (.downcast base _) =>
          placeThroughShared unit ns fuel base
      | some (.localVar _) | none => false

namespace Internal

/-- Package-internal constructor used by the shared checker. Frontends and
backends must not import or call this namespace. Every namespace views the
unit's tables, and its links place what they do not cover past them. -/
def mkValidatedUnit (tables : Tables) (profiles : Array ProfileConfig)
    (namespaces : Array ValidatedNamespace)
    (dependencies : Array ValidatedNamespaceInterface)
    (evidence : Array ValidatedImportEvidence)
    (indexes : UnitIndexes)
    (resolution : ResolutionIndex) : ValidatedUnit :=
  .mk tables profiles
    (namespaces.map fun ns => { ns with tables, links := { ns.links with beyond := .of tables } })
    dependencies evidence indexes resolution

/-- A view of a unit with its namespaces replaced as given, each keeping the
tables it views (`ValidatedUnit.inUnitIds`). -/
def withNamespaces (unit : ValidatedUnit) (namespaces : Array ValidatedNamespace) :
    ValidatedUnit :=
  .mk unit.tables unit.profiles namespaces unit.dependencies unit.evidence unit.indexes
    unit.resolution

end Internal

end VerexIR.Validation

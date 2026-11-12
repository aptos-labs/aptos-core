-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean
import VerexIR.Semantics.Runtime
import VerexIR.Validation.Link

/-!
# Quotation of validated units

`ToExpr` instances for the VIR syntax and validated-unit chain, so the
verification generator can materialize a registered unit as a literal in
generated statements. Everything downstream — arena lookups, kind
equations — then reduces on literals. One `deriving` command per type:
instances derived in the same command are not visible to each other.
-/

namespace VerexSL.Quote

open Lean
open VerexIR

deriving instance ToExpr for VerexIR.Validation.IndexedArena
deriving instance ToExpr for VerexIR.Validation.KeyTree
open VerexIR.Validation

deriving instance ToExpr for LoanId
/-- A file's index as the kernel's raw literal, as `SourceRange`'s offsets. -/
instance : ToExpr FileId where
  toExpr file := mkApp (mkConst ``FileId.mk) (mkRawNatLit file.index)
  toTypeExpr := mkConst ``FileId
deriving instance ToExpr for LocId
deriving instance ToExpr for OriginId
deriving instance ToExpr for AlignmentId
deriving instance ToExpr for ProfileId
deriving instance ToExpr for TypeId
deriving instance ToExpr for NamespaceId
deriving instance ToExpr for NameId
deriving instance ToExpr for TypeDeclId
deriving instance ToExpr for ConstantDeclId
deriving instance ToExpr for TraitDeclId
deriving instance ToExpr for ImplDeclId
deriving instance ToExpr for AssociatedItemId
deriving instance ToExpr for FunctionId
deriving instance ToExpr for SpecFunctionId
deriving instance ToExpr for SpecVarId
deriving instance ToExpr for LocalId
deriving instance ToExpr for PlaceId
deriving instance ToExpr for LifetimeId
deriving instance ToExpr for EvidenceId
deriving instance ToExpr for ExprId
deriving instance ToExpr for PatternId
deriving instance ToExpr for BlockId
deriving instance ToExpr for IntrinsicId
deriving instance ToExpr for SourceFile
/-- Byte offsets as the kernel's raw literals: locations are provenance, which
no proof computes on, and the most numerous entries of a unit's tables. -/
instance : ToExpr SourceRange where
  toExpr range := mkApp3 (mkConst ``SourceRange.mk) (toExpr range.file)
    (mkRawNatLit range.startByte) (mkRawNatLit range.endByte)
  toTypeExpr := mkConst ``SourceRange

private def noRange : Lean.Expr := toExpr (none : Option SourceRange)
private def noRanges : Lean.Expr := toExpr (#[] : Array SourceRange)
private def noGenerator : Lean.Expr := toExpr (none : Option String)
private def noParent : Lean.Expr := toExpr (none : Option LocId)

/-- A location, its absent parts one shared term each. -/
instance : ToExpr Location where
  toExpr location := mkAppN (mkConst ``Location.mk) #[
    location.primary.elim noRange (toExpr <| some ·),
    if location.related.isEmpty then noRanges else toExpr location.related,
    if location.expansion.isEmpty then noRanges else toExpr location.expansion,
    location.generatedBy.elim noGenerator (toExpr <| some ·),
    location.parent.elim noParent (toExpr <| some ·)]
  toTypeExpr := mkConst ``Location
deriving instance ToExpr for OriginKind
deriving instance ToExpr for Origin
deriving instance ToExpr for Trust
deriving instance ToExpr for Alignment
deriving instance ToExpr for NamespaceRef
deriving instance ToExpr for QualifiedName
deriving instance ToExpr for QualifiedRef
deriving instance ToExpr for Profile
deriving instance ToExpr for ProfileValue
deriving instance ToExpr for ProfileConfig
deriving instance ToExpr for IntWidth
deriving instance ToExpr for ReferenceKind
deriving instance ToExpr for LifetimeKind
deriving instance ToExpr for Lifetime
deriving instance ToExpr for ReferenceType
deriving instance ToExpr for ConstValue
deriving instance ToExpr for TypeUse
deriving instance ToExpr for GenericArgument
deriving instance ToExpr for Ability
deriving instance ToExpr for TraitRef
deriving instance ToExpr for GenericPredicate
deriving instance ToExpr for Ty
deriving instance ToExpr for AttributeValue
deriving instance ToExpr for Attribute
deriving instance ToExpr for Comment
deriving instance ToExpr for Tables
deriving instance ToExpr for BorrowKind
deriving instance ToExpr for ThrowKind
deriving instance ToExpr for CallKind
deriving instance ToExpr for SurfaceSyntax
deriving instance ToExpr for GlobalKind
deriving instance ToExpr for PrimitiveOperation
deriving instance ToExpr for ReferenceOperation
deriving instance ToExpr for DataOperation
deriving instance ToExpr for MemoryRange
deriving instance ToExpr for TraceKind
deriving instance ToExpr for BehaviorKind
deriving instance ToExpr for SpecOperation
deriving instance ToExpr for Operation
deriving instance ToExpr for Place
deriving instance ToExpr for QuantifierKind
deriving instance ToExpr for MatchArm
deriving instance ToExpr for QuantifierBinder
deriving instance ToExpr for ConditionKind
deriving instance ToExpr for Condition
deriving instance ToExpr for Frame
deriving instance ToExpr for SpecBlock
deriving instance ToExpr for ExprKind
deriving instance ToExpr for ExprOrigin
deriving instance ToExpr for VerexIR.Expr
deriving instance ToExpr for PatternKind
deriving instance ToExpr for Pattern
deriving instance ToExpr for BinderKind
deriving instance ToExpr for GenericBinder
deriving instance ToExpr for Parameter
deriving instance ToExpr for VerexIR.LocalDecl
deriving instance ToExpr for Signature
deriving instance ToExpr for ParameterFrame
deriving instance ToExpr for FunctionContract
deriving instance ToExpr for ConstantDecl
deriving instance ToExpr for FieldDecl
deriving instance ToExpr for VariantDecl
deriving instance ToExpr for GhostFieldDecl
deriving instance ToExpr for StructDecl
deriving instance ToExpr for FunctionDecl
deriving instance ToExpr for AssociatedItemKind
deriving instance ToExpr for AssociatedItemDecl
deriving instance ToExpr for TraitDecl
deriving instance ToExpr for AssociatedItemValue
deriving instance ToExpr for AssociatedItemBinding
deriving instance ToExpr for ImplDecl
deriving instance ToExpr for SpecFunctionDecl
deriving instance ToExpr for LemmaDecl
deriving instance ToExpr for SpecVarDecl
deriving instance ToExpr for NamespaceInvariant
deriving instance ToExpr for IntrinsicBinding
deriving instance ToExpr for IntrinsicDecl
deriving instance ToExpr for Namespace
deriving instance ToExpr for FunctionBody
deriving instance ToExpr for LoanDeath
deriving instance ToExpr for ValueOrders
deriving instance ToExpr for Holding
deriving instance ToExpr for TableSizes
deriving instance ToExpr for Links
deriving instance ToExpr for StructurizedBlock
deriving instance ToExpr for StructurizedEdge
deriving instance ToExpr for StructurizedRegionKind
deriving instance ToExpr for StructurizedRegion
deriving instance ToExpr for StructurizationWitness
deriving instance ToExpr for InitializationCertificate
deriving instance ToExpr for ReferenceParameterFact
deriving instance ToExpr for CheckedLoanFact
deriving instance ToExpr for LifetimeRelationFact
deriving instance ToExpr for BorrowCertificate
deriving instance ToExpr for VerexIR.Validation.Severity
deriving instance ToExpr for VerexIR.Validation.RelatedLocation
deriving instance ToExpr for VerexIR.Validation.Diagnostic
deriving instance ToExpr for VerexIR.Validation.OwnedDiagnostic
deriving instance ToExpr for ValidatedNamespace
deriving instance ToExpr for RelocatableNamespace
deriving instance ToExpr for ValidatedNamespaceInterface
deriving instance ToExpr for ValidatedImportEvidence
deriving instance ToExpr for UnitIndexes
deriving instance ToExpr for AnchorDeaths
deriving instance ToExpr for VerexIR.Validation.ResolutionIndex
deriving instance ToExpr for FunctionHandle
deriving instance ToExpr for StructHandle
deriving instance ToExpr for RuntimeValue

/-- The definition of the relocatable namespace registered at a path: the
namespace's body and validation results in its own tables, defined where it
is registered, so a unit quotes a reference to it. -/
def objectName (segments : Array String) : Name :=
  Name.str (segments.foldl Name.str .anonymous) "object"

/-- Where a namespace of a unit is: the path its own tables give it. -/
private def objectPath (ns : ValidatedNamespace) : Array String :=
  (ns.localTables.namespaces[ns.identity.index]?.map (·.segments)).getD #[]

/-- Define a registered relocatable namespace (`objectName`), unless it is
defined. -/
def ensureObjectDefinition (object : RelocatableNamespace) : CoreM Unit := do
  let name := objectName object.path
  if (← getEnv).contains name then return
  addDecl (.defnDecl {
    name, levelParams := []
    type := mkConst ``RelocatableNamespace
    value := toExpr object
    hints := .abbrev, safety := .safe })

/-- A validated unit as a literal. Each namespace is its registered relocatable
namespace (`objectName`) placed in the unit: the unit's tables and value
orders, which each namespace views, and its links, equal views and links
quoted once and shared. The unit's constructor is private, so it is read from
the environment. `registered` is the relocatable namespace registered at a
path. -/
def quoteUnit [Monad m] [MonadEnv m] [MonadError m] (unit : ValidatedUnit)
    (registered : Array String → Option RelocatableNamespace) : m Lean.Expr := do
  let env ← getEnv
  let constructor := (getStructureCtor env ``ValidatedUnit).name
  let tables := toExpr unit.tables
  let orders := unit.namespaces[0]?.map fun ns => (ns.orders, toExpr ns.orders)
  let links := unit.namespaces[0]?.map fun ns => (ns.links, toExpr ns.links)
  let namespaceType := toTypeExpr ValidatedNamespace
  let nil := mkApp (mkConst ``List.nil [levelZero]) namespaceType
  let namespaces ← unit.namespaces.foldrM (init := nil) fun ns rest => do
    let path := objectPath ns
    let shown := "::".intercalate path.toList
    let some object := registered path
      | throwError m!"the namespace `{shown}` of the unit is not registered"
    unless env.contains (objectName path) do
      throwError m!"the registered namespace `{shown}` has no definition `{objectName path}`"
    unless ns.toNamespace == object.body && ns.localTables == object.tables &&
        ns.holders == object.holders && ns.witnesses == object.witnesses &&
        ns.initializationCertificates == object.initializationCertificates &&
        ns.borrowCertificates == object.borrowCertificates &&
        ns.borrowRejections == object.borrowRejections do
      throwError m!"the namespace `{shown}` of the unit is not its registered namespace"
    return mkApp3 (mkConst ``List.cons [levelZero]) namespaceType
      (mkAppN (mkConst ``RelocatableNamespace.place) #[mkConst (objectName path),
        if ns.tables == unit.tables then tables else toExpr ns.tables,
        match links with
        | some (shared, quoted) => if ns.links == shared then quoted else toExpr ns.links
        | none => toExpr ns.links,
        match orders with
        | some (shared, quoted) => if ns.orders == shared then quoted else toExpr ns.orders
        | none => toExpr ns.orders])
      rest
  return mkAppN (mkConst constructor) #[tables, toExpr unit.profiles,
    mkApp2 (mkConst ``List.toArray [levelZero]) namespaceType namespaces,
    toExpr unit.dependencies, toExpr unit.evidence, toExpr unit.indexes,
    toExpr unit.resolution]

/-- The definitions holding quoted value orders, by the hash of their
literal. -/
private initialize valueOrderDefinitions :
    SimplePersistentEnvExtension (UInt64 × Name) (Std.HashMap UInt64 (Array Name)) ←
  registerSimplePersistentEnvExtension {
    addEntryFn := fun map (key, name) => map.insert key ((map.getD key #[]).push name)
    addImportedFn := fun entries =>
      mkStateFromImportedEntries (fun map (key, name) =>
        map.insert key ((map.getD key #[]).push name)) {} entries }

/-- Value orders as the definition holding their literal, defined at the
first use. A unit's table of variant names and function positions would
otherwise be repeated in every comparison and map discipline a generated
statement states; equal tables share one definition. -/
def quoteValueOrders (orders : ValueOrders) : Lean.Meta.MetaM Lean.Expr := do
  let literal := toExpr orders
  let key := literal.hash
  let env ← getEnv
  for name in (valueOrderDefinitions.getState env).getD key #[] do
    if ((env.find? name).bind (·.value?)) == some literal then return mkConst name
  let base := (← getMainModule) ++ `verexValueOrders
  let mut index := (valueOrderDefinitions.getEntries env).length
  while env.contains (base ++ Name.mkSimple s!"table{index}") do index := index + 1
  let name := base ++ Name.mkSimple s!"table{index}"
  addDecl (.defnDecl {
    name, levelParams := []
    type := mkConst ``ValueOrders
    value := literal
    hints := .abbrev, safety := .safe })
  modifyEnv (valueOrderDefinitions.addEntry · (key, name))
  return mkConst name

end VerexSL.Quote

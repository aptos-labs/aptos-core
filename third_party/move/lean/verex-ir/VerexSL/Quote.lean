-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean
import VerexIR.Semantics.Runtime

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
deriving instance ToExpr for ValidatedNamespace
deriving instance ToExpr for ValidatedNamespaceInterface
deriving instance ToExpr for ValidatedImportEvidence
deriving instance ToExpr for UnitIndexes
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
deriving instance ToExpr for AnchorDeaths
deriving instance ToExpr for VerexIR.Validation.Severity
deriving instance ToExpr for VerexIR.Validation.RelatedLocation
deriving instance ToExpr for VerexIR.Validation.Diagnostic
deriving instance ToExpr for VerexIR.Validation.OwnedDiagnostic
deriving instance ToExpr for VerexIR.Validation.ResolutionIndex
deriving instance ToExpr for FunctionHandle
deriving instance ToExpr for StructHandle
deriving instance ToExpr for RuntimeValue

/-- A validated unit as a literal. Each namespace caches views of the unit's
tables and value orders, most of a unit; equal views are quoted once and
shared. The unit's constructor is private, so it is read from the
environment. -/
def quoteUnit [Monad m] [MonadEnv m] (unit : ValidatedUnit) : m Lean.Expr := do
  let constructor := (getStructureCtor (← getEnv) ``ValidatedUnit).name
  let tables := toExpr unit.tables
  let orders := unit.namespaces[0]?.map fun ns => (ns.orders, toExpr ns.orders)
  let namespaceType := toTypeExpr ValidatedNamespace
  let nil := mkApp (mkConst ``List.nil [levelZero]) namespaceType
  let namespaces := unit.namespaces.foldr (init := nil) fun ns rest =>
    mkApp3 (mkConst ``List.cons [levelZero]) namespaceType
      (mkAppN (mkConst ``ValidatedNamespace.mk) #[
        toExpr ns.toNamespace,
        if ns.tables == unit.tables then tables else toExpr ns.tables,
        match orders with
        | some (shared, quoted) => if ns.orders == shared then quoted else toExpr ns.orders
        | none => toExpr ns.orders,
        toExpr ns.holders])
      rest
  return mkAppN (mkConst constructor) #[tables, toExpr unit.profiles,
    mkApp2 (mkConst ``List.toArray [levelZero]) namespaceType namespaces,
    toExpr unit.dependencies, toExpr unit.evidence, toExpr unit.indexes,
    toExpr unit.structurizationWitnesses, toExpr unit.resolution,
    toExpr unit.initializationCertificates, toExpr unit.borrowCertificates,
    toExpr unit.borrowRejections]

end VerexSL.Quote

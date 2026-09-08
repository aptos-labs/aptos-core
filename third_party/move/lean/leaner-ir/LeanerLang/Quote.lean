-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean
import LeanerIR.Semantics.Runtime

/-!
# Quotation of validated units

`ToExpr` instances for the LIR syntax and validated-unit chain, so the
verification generator can materialize a registered unit as a literal in
generated statements. Everything downstream — arena lookups, kind
equations — then reduces on literals. One `deriving` command per type:
instances derived in the same command are not visible to each other.
-/

namespace LeanerLang.Quote

open Lean
open LeanerIR
open LeanerIR.Validation

deriving instance ToExpr for LoanId
deriving instance ToExpr for FileId
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
deriving instance ToExpr for SourceRange
deriving instance ToExpr for Location
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
deriving instance ToExpr for LeanerIR.Expr
deriving instance ToExpr for PatternKind
deriving instance ToExpr for Pattern
deriving instance ToExpr for BinderKind
deriving instance ToExpr for GenericBinder
deriving instance ToExpr for Parameter
deriving instance ToExpr for LeanerIR.LocalDecl
deriving instance ToExpr for Signature
deriving instance ToExpr for FunctionContract
deriving instance ToExpr for ConstantDecl
deriving instance ToExpr for FieldDecl
deriving instance ToExpr for VariantDecl
deriving instance ToExpr for StructDecl
deriving instance ToExpr for FunctionDecl
deriving instance ToExpr for AssociatedItemKind
deriving instance ToExpr for AssociatedItemDecl
deriving instance ToExpr for TraitDecl
deriving instance ToExpr for AssociatedItemValue
deriving instance ToExpr for AssociatedItemBinding
deriving instance ToExpr for ImplDecl
deriving instance ToExpr for SpecFunctionDecl
deriving instance ToExpr for SpecVarDecl
deriving instance ToExpr for NamespaceInvariant
deriving instance ToExpr for IntrinsicBinding
deriving instance ToExpr for IntrinsicDecl
deriving instance ToExpr for Namespace
deriving instance ToExpr for FunctionBody
deriving instance ToExpr for LoanDeath
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
deriving instance ToExpr for LeanerIR.Validation.Severity
deriving instance ToExpr for LeanerIR.Validation.RelatedLocation
deriving instance ToExpr for LeanerIR.Validation.Diagnostic
deriving instance ToExpr for LeanerIR.Validation.ResolutionIndex
deriving instance ToExpr for ValidatedUnit
deriving instance ToExpr for FunctionHandle
deriving instance ToExpr for StructHandle
deriving instance ToExpr for RuntimeValue

end LeanerLang.Quote

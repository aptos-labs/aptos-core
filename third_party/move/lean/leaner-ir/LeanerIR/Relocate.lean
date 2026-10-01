-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Syntax
import LeanerIR.Validation.Validated
import LeanerIR.Validation.Diagnostic
import LeanerIR.Relocate.Deriving

/-!
# Relocating LIR

The relocation of every LIR type and of the validation results a namespace
carries, derived.
-/

namespace LeanerIR

deriving instance Remap for
  SourceFile, SourceRange, Location, OriginKind, Origin, Trust, Alignment, NamespaceRef,
  QualifiedName, QualifiedRef, Profile, ProfileValue, ProfileConfig, IntWidth, ReferenceKind,
  LifetimeKind, Lifetime, ReferenceType, ConstValue, TypeUse, GenericArgument, Ability,
  TraitRef, GenericPredicate, Ty, AttributeValue, Attribute, Comment, BorrowKind, ThrowKind,
  CallKind, SurfaceSyntax, GlobalKind, PrimitiveOperation, ReferenceOperation, DataOperation,
  MemoryRange, TraceKind, BehaviorKind, SpecOperation, Operation, Place, QuantifierKind,
  MatchArm, QuantifierBinder, ConditionKind, Condition, Frame, SpecBlock, ExprKind, Expr,
  PatternKind, Pattern, BinderKind, GenericBinder, Parameter, LocalDecl, Signature,
  FunctionContract, ConstantDecl, FieldDecl, VariantDecl, StructDecl, FunctionDecl,
  AssociatedItemKind, AssociatedItemDecl, TraitDecl, AssociatedItemValue,
  AssociatedItemBinding, ImplDecl, SpecFunctionDecl, SpecVarDecl, NamespaceInvariant,
  IntrinsicBinding, IntrinsicDecl, Namespace

namespace Validation

deriving instance Remap for
  Severity, RelatedLocation, Diagnostic, OwnedDiagnostic, FunctionBody, StructurizedBlock,
  StructurizedEdge, StructurizedRegionKind, StructurizedRegion, StructurizationWitness,
  InitializationCertificate, ReferenceParameterFact, LoanDeath, CheckedLoanFact,
  LifetimeRelationFact, BorrowCertificate, ValidatedNamespaceInterface

end Validation

end LeanerIR

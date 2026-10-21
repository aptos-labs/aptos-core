-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Syntax
import VerexIR.Validation.Validated
import VerexIR.Validation.Diagnostic
import VerexIR.Relocate.Deriving

/-!
# Relocating VIR

The relocation of every VIR type and of the validation results a namespace
carries, derived.
-/

namespace VerexIR

deriving instance Remap for
  SourceFile, SourceRange, Location, OriginKind, Origin, Trust, Alignment, NamespaceRef,
  QualifiedName, QualifiedRef, Profile, ProfileValue, ProfileConfig, IntWidth, ReferenceKind,
  LifetimeKind, Lifetime, ReferenceType, ConstValue, TypeUse, GenericArgument, Ability,
  TraitRef, GenericPredicate, Ty, AttributeValue, Attribute, Comment, BorrowKind, ThrowKind,
  CallKind, SurfaceSyntax, GlobalKind, PrimitiveOperation, ReferenceOperation, DataOperation,
  MemoryRange, TraceKind, BehaviorKind, SpecOperation, Operation, Place, QuantifierKind,
  MatchArm, QuantifierBinder, ConditionKind, Condition, Frame, SpecBlock, ExprKind, Expr,
  PatternKind, Pattern, BinderKind, GenericBinder, Parameter, LocalDecl, Signature,
  ParameterFrame, FunctionContract, ConstantDecl, FieldDecl, VariantDecl, GhostFieldDecl, StructDecl, FunctionDecl,
  AssociatedItemKind, AssociatedItemDecl, TraitDecl, AssociatedItemValue,
  AssociatedItemBinding, ImplDecl, SpecFunctionDecl, LemmaDecl, SpecVarDecl, NamespaceInvariant,
  IntrinsicBinding, IntrinsicDecl, Namespace

namespace Validation

deriving instance Remap for
  Severity, RelatedLocation, Diagnostic, OwnedDiagnostic, FunctionBody, StructurizedBlock,
  StructurizedEdge, StructurizedRegionKind, StructurizedRegion, StructurizationWitness,
  InitializationCertificate, ReferenceParameterFact, LoanDeath, CheckedLoanFact,
  LifetimeRelationFact, BorrowCertificate, ValidatedNamespaceInterface

end Validation

end VerexIR

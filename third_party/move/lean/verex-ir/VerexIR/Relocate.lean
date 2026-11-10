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
  MatchArm, QuantifierBinder, ConditionKind, Condition, Frame, SpecBlock, ExprKind, ExprOrigin, Expr,
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

/-- A namespace's body identifiers as unit identifiers (its links). -/
def ValidatedNamespace.unitIds (ns : ValidatedNamespace) : IdFns Id := {
  file := pure, loc := pure, origin := pure, alignment := pure, profile := pure
  type := fun id => pure (ns.typeId id), namespaceId := fun id => pure (ns.namespaceId id)
  name := fun id => pure (ns.nameId id), lifetime := fun id => pure (ns.lifetimeId id) }

/-- A part of a namespace's body, its identifiers as unit identifiers. -/
def ValidatedNamespace.relocate [Remap α] (ns : ValidatedNamespace) (value : α) : α :=
  Id.run (remap value ns.unitIds)

/-- A namespace with its body numbered in its unit's tables: its identity is
its unit index, and its links are the identity. A type it links reads as its
own entry, translated: the unit's entry with the locations the namespace
wrote, which the location-free unit table lacks. Its own tables keep their
provenance; their semantic entries are these. Its validation results stay in
its own tables. -/
def ValidatedNamespace.inUnitIds (ns : ValidatedNamespace) : ValidatedNamespace :=
  let types := ns.links.types.zipIdx.foldl (init := ns.tables.types) fun types (unitIndex, index) =>
    match ns.localTables.types[index]? with
    | some own => types.set! unitIndex (ns.relocate own)
    | none => types
  let tables := { ns.tables with types }
  { ns with
    toNamespace := ns.relocate ns.toNamespace, tables, links := .identity tables
    localTables := { ns.localTables with
      types, names := ns.tables.names, namespaces := ns.tables.namespaces
      lifetimes := ns.tables.lifetimes } }

/-- A unit with every namespace's body numbered in its tables
(`ValidatedNamespace.inUnitIds`): the view of an analysis written over one
table space. A namespace's types carry the locations it wrote. -/
def ValidatedUnit.inUnitIds (unit : ValidatedUnit) : ValidatedUnit :=
  Internal.withNamespaces unit (unit.namespaces.map (·.inUnitIds))

end Validation

end VerexIR

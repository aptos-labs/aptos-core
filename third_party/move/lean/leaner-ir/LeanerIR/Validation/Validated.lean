-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Syntax
import LeanerIR.Validation.Resolve

/-!
# Validated LIR backend boundary

This module contains the checked forms exposed to backends. Their public
structure is separate from raw frontend input, and `ValidatedUnit` retains a
private constructor so only the shared checker can create one.
-/

namespace LeanerIR.Validation

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

/-- Checked backend input namespace whose functions contain only structured
bodies (or are explicitly absent). `tables` is an immutable cached view of the
owning compilation-unit table. -/
structure ValidatedNamespace extends Namespace FunctionBody where
  tables : Tables
  deriving Repr, BEq, Inhabited

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
  deriving Repr, BEq, Inhabited

/-- Checked copy of an upstream producer/check/assumption statement. This is
retained for backend and theorem reporting; `trusted` remains an explicit
assumption marker and is not upgraded into proof by LIR validation. -/
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

/-- One recorded death point of a loan: its write-back must run before
(`before := true`) or after the anchor expression evaluates. Function-result
boundaries explicitly record every mutable loan not carried by the returned
value. A loan may carry several records when its death is branch-dependent;
loan-death markers are conditional and idempotent, so the over-approximation
is sound. Only a loan carried by a returned reference remains live for frame
finalization to transfer across the call boundary. -/
structure LoanDeath where
  anchor : ExprId
  before : Bool := false
  deriving Repr, BEq, Inhabited

/-- Stable identity and lifetime of a loan site accepted by the borrow
analysis. The validated unit retains its kind and place; keeping those
canonical facts in one place also avoids copying recursive place data into
every semantic wrapper. Mutable loans record the death points the loan
elimination stage materializes as `endLoan` markers. -/
structure CheckedLoanFact where
  expression : ExprId
  lifetime : LifetimeId
  holders : Array LocalId := #[]
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
  structurizationWitnesses : Array StructurizationWitness
  resolution : ResolutionIndex
  initializationCertificates : Array InitializationCertificate
  borrowCertificates : Array BorrowCertificate
  /-- Diagnostics of functions the borrow analysis could not certify. The
  analysis still lacks non-lexical loan death and region solving, so its
  rejections stay preparation-stage errors instead of failing `validate`. -/
  borrowDiagnostics : Array Diagnostic
  deriving Repr, BEq, Inhabited

namespace Internal

/-- Package-internal constructor used by the shared checker. Frontends and
backends must not import or call this namespace. -/
def mkValidatedUnit (tables : Tables) (profiles : Array ProfileConfig)
    (namespaces : Array ValidatedNamespace)
    (dependencies : Array ValidatedNamespaceInterface)
    (evidence : Array ValidatedImportEvidence)
    (indexes : UnitIndexes)
    (structurizationWitnesses : Array StructurizationWitness)
    (resolution : ResolutionIndex)
    (initializationCertificates : Array InitializationCertificate := #[])
    (borrowCertificates : Array BorrowCertificate := #[])
    (borrowDiagnostics : Array Diagnostic := #[]) : ValidatedUnit :=
  .mk tables profiles namespaces dependencies evidence indexes structurizationWitnesses
    resolution initializationCertificates borrowCertificates borrowDiagnostics

end Internal

end LeanerIR.Validation

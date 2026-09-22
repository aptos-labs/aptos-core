-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

/-!
# Strong identifiers

All cross references in LIR use distinct identifier types.  Their numeric
representation is deliberately exposed for serialization, but consumers must
use the checked lookup functions at an untrusted boundary.
-/

namespace LeanerIR

/-- Identity of one accepted loan: its index in the owning function's
`BorrowCertificate.loans`. Minted by validation's loan elimination; never
present in frontend input. -/
structure LoanId where index : Nat deriving Repr, BEq, DecidableEq, Hashable, Inhabited

/-- Index into `Tables.files` for the containing namespace. -/
structure FileId where index : Nat deriving Repr, BEq, DecidableEq, Hashable, Inhabited

/-- Index into `Tables.locations` for the containing namespace. -/
structure LocId where index : Nat deriving Repr, BEq, DecidableEq, Hashable, Inhabited

/-- Index into `Tables.origins` for the containing namespace. -/
structure OriginId where index : Nat deriving Repr, BEq, DecidableEq, Hashable, Inhabited

/-- Index into `Tables.alignments` for the containing namespace. -/
structure AlignmentId where index : Nat deriving Repr, BEq, DecidableEq, Hashable, Inhabited

/-- Index into `RawUnit.profiles` or `ValidatedUnit.profiles`. -/
structure ProfileId where index : Nat deriving Repr, BEq, DecidableEq, Hashable, Inhabited

/-- Index into `Tables.types` for the containing namespace. -/
structure TypeId where index : Nat deriving Repr, BEq, DecidableEq, Hashable, Inhabited

/-- Namespace index in the collection appropriate to its field: unit-level
identities/imports use `RawUnit.namespaces`, while qualified names use the
containing `Tables.namespaces`. -/
structure NamespaceId where index : Nat deriving Repr, DecidableEq, Hashable, Inhabited

/-- Index into `Tables.names` for the containing namespace. -/
structure NameId where index : Nat deriving Repr, BEq, DecidableEq, Hashable, Inhabited

/-- Strong nominal-type declaration identity reserved for validated indexes. -/
structure TypeDeclId where index : Nat deriving Repr, BEq, DecidableEq, Hashable, Inhabited

/-- Index into `Namespace.constants` for the containing namespace. -/
structure ConstantDeclId where index : Nat deriving Repr, BEq, DecidableEq, Hashable, Inhabited

/-- Index into `Namespace.traits` for the containing namespace. -/
structure TraitDeclId where index : Nat deriving Repr, BEq, DecidableEq, Hashable, Inhabited

/-- Index into `Namespace.implementations` for the containing namespace. -/
structure ImplDeclId where index : Nat deriving Repr, BEq, DecidableEq, Hashable, Inhabited

/-- Index into `Namespace.associatedItems` for the containing namespace. -/
structure AssociatedItemId where index : Nat deriving Repr, BEq, DecidableEq, Hashable, Inhabited

/-- Strong executable-function identity reserved for validated indexes. -/
structure FunctionId where index : Nat deriving Repr, BEq, DecidableEq, Hashable, Inhabited

/-- Strong specification-function identity reserved for validated indexes. -/
structure SpecFunctionId where index : Nat deriving Repr, BEq, DecidableEq, Hashable, Inhabited

/-- Strong specification-variable identity reserved for validated indexes. -/
structure SpecVarId where index : Nat deriving Repr, BEq, DecidableEq, Hashable, Inhabited

/-- Index into the `locals` array of the declaration that owns an expression. -/
structure LocalId where index : Nat deriving Repr, BEq, DecidableEq, Hashable, Inhabited

/-- Index into `Namespace.places` for the containing namespace. -/
structure PlaceId where index : Nat deriving Repr, BEq, DecidableEq, Hashable, Inhabited

/-- Index into `Tables.lifetimes` for the containing namespace. -/
structure LifetimeId where index : Nat deriving Repr, BEq, DecidableEq, Hashable, Inhabited

/-- Opaque trait or other proof-evidence identity carried by generic
arguments. A checked evidence table is deferred beyond the first Rust slice. -/
structure EvidenceId where index : Nat deriving Repr, BEq, DecidableEq, Hashable, Inhabited

/-- Index into `Namespace.expressions` for the containing namespace. -/
structure ExprId where index : Nat deriving Repr, BEq, DecidableEq, Hashable, Inhabited

/-- Index into `Namespace.patterns` for the containing namespace. -/
structure PatternId where index : Nat deriving Repr, BEq, DecidableEq, Hashable, Inhabited

/-- Index into `RawCfg.blocks`; meaningful only within that one raw graph. -/
structure BlockId where index : Nat deriving Repr, BEq, DecidableEq, Hashable, Inhabited

/-- Strong intrinsic-declaration identity reserved for validated indexes. -/
structure IntrinsicId where index : Nat deriving Repr, BEq, DecidableEq, Hashable, Inhabited

end LeanerIR

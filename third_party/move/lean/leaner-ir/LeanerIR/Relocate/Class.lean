-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Ids

/-!
# The relocation traversal

An LIR value refers into the tables of the unit that holds it through its
table-indexed identifiers. `Remap` maps every one of them, each kind through
its own function of an `IdFns`, and leaves everything else as it is:
identifiers local to a namespace (expressions, patterns, places, locals,
blocks, declaration indices, loans) and plain values. Its instances are
derived for every LIR type, so no identifier field is missed; the functions
run in any monad, so one traversal both collects and renumbers.
-/

namespace LeanerIR

/-- What relocation does to each kind of table-indexed identifier. -/
structure IdFns (m : Type → Type) where
  file : FileId → m FileId
  loc : LocId → m LocId
  origin : OriginId → m OriginId
  alignment : AlignmentId → m AlignmentId
  profile : ProfileId → m ProfileId
  type : TypeId → m TypeId
  namespaceId : NamespaceId → m NamespaceId
  name : NameId → m NameId
  lifetime : LifetimeId → m LifetimeId

/-- Map every table-indexed identifier of a value. -/
class Remap (α : Type) where
  remap : α → {m : Type → Type} → [Monad m] → IdFns m → m α

export Remap (remap)

/-- A value with no identifiers is its own relocation. -/
@[reducible] def Remap.fixed : Remap α := ⟨fun value _ _ _ => pure value⟩

instance : Remap FileId := ⟨fun id _ _ f => f.file id⟩
instance : Remap LocId := ⟨fun id _ _ f => f.loc id⟩
instance : Remap OriginId := ⟨fun id _ _ f => f.origin id⟩
instance : Remap AlignmentId := ⟨fun id _ _ f => f.alignment id⟩
instance : Remap ProfileId := ⟨fun id _ _ f => f.profile id⟩
instance : Remap TypeId := ⟨fun id _ _ f => f.type id⟩
instance : Remap NamespaceId := ⟨fun id _ _ f => f.namespaceId id⟩
instance : Remap NameId := ⟨fun id _ _ f => f.name id⟩
instance : Remap LifetimeId := ⟨fun id _ _ f => f.lifetime id⟩

-- Identifiers local to a namespace or a function.
instance : Remap LoanId := Remap.fixed
instance : Remap TypeDeclId := Remap.fixed
instance : Remap ConstantDeclId := Remap.fixed
instance : Remap TraitDeclId := Remap.fixed
instance : Remap ImplDeclId := Remap.fixed
instance : Remap AssociatedItemId := Remap.fixed
instance : Remap FunctionId := Remap.fixed
instance : Remap SpecFunctionId := Remap.fixed
instance : Remap SpecVarId := Remap.fixed
instance : Remap LocalId := Remap.fixed
instance : Remap PlaceId := Remap.fixed
instance : Remap EvidenceId := Remap.fixed
instance : Remap ExprId := Remap.fixed
instance : Remap PatternId := Remap.fixed
instance : Remap BlockId := Remap.fixed
instance : Remap IntrinsicId := Remap.fixed

-- Plain values.
instance : Remap Nat := Remap.fixed
instance : Remap Int := Remap.fixed
instance : Remap String := Remap.fixed
instance : Remap Bool := Remap.fixed
instance : Remap Char := Remap.fixed
instance : Remap UInt8 := Remap.fixed
instance : Remap Unit := Remap.fixed

instance [Remap α] : Remap (Array α) :=
  ⟨fun values _ _ f => values.mapM (remap · f)⟩
instance [Remap α] : Remap (List α) :=
  ⟨fun values _ _ f => values.mapM (remap · f)⟩
instance [Remap α] : Remap (Option α) :=
  ⟨fun value _ _ f => value.mapM (remap · f)⟩
instance [Remap α] [Remap β] : Remap (α × β) :=
  ⟨fun (a, b) _ _ f => return (← remap a f, ← remap b f)⟩

end LeanerIR

-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerMove.Intrinsics

/-!
# Move semantic policy, intrinsic schemas, and residual frontend profile

The neutral LIR owns the known Move/Rust semantic union, including executable
and specification operations. This module supplies the small Move policy
surface such as abort rollback and temporarily closes the vocabulary of
frontend-only declaration metadata. It also registers the closed Move map
intrinsic schema. Unknown extension tags and intrinsic roles are rejected at
the shared checked-construction boundary.
-/

namespace LeanerIR.Move

open LeanerIR
open LeanerIR.Import
open LeanerIR.Validation

def profileName : String := "move"
def profileVersion : Nat := 1

def config : ProfileConfig :=
  { profile := .move, name := profileName, version := profileVersion }

private def propertyTags : Array String := #[
  "visibility.private", "visibility.public", "visibility.friend", "visibility.package",
  "function.regular", "function.inlineRetained", "function.native", "function.entry",
  "function.receiver", "struct.native", "typeParameter.phantom",
  "metadata.addressAlias", "metadata.namedAddress", "metadata.friend",
  "metadata.skipped", "specFunction.uninterpreted", "specFunction.native",
  "specFunction.moveFunction", "specFunction.usesOld", "struct.variants"
]

/-! ## M0 semantic inventory

The structural vocabulary above and the semantic classification below are
deliberately separate. `semanticInventoryComplete` checks their agreement, so
accepting a new Move tag requires assigning it a runtime, logical, or
frontend-only role and an implementation status. -/

private def classifySemanticTag (site : ProfileSemanticSite)
    (value : ProfileValue) : Option SemanticClassification :=
  match site with
  | .type => none
  | .constant => none
  | .operation | .borrow | .throw_ | .call => none
  | .surface => none
  | .property =>
      if propertyTags.contains value.tag then some .frontendOnly
      else none
  | .quantifier => none

/-- Move's versioned policy and residual-tag classification. -/
def semantics : SemanticProfile where
  profile := .move
  name := profileName
  version := profileVersion
  classify := classifySemanticTag
  rollbackThrow := fun kind => kind == .abort

/-- Whether every tag admitted by the structural Move schema has an explicit
M0 semantic classification at its owning semantic site. -/
def semanticInventoryComplete : Bool :=
  propertyTags.all fun tag =>
    (classifySemanticTag .property { profile := .move, tag }).isSome

private def unknown (kind : String) (value : ProfileValue) : Array Diagnostic :=
  #[.error "LIR-MOVE-TAG" s!"unknown Move {kind} tag `{value.tag}`"]

private def checkTag (kind : String) (tags : Array String) (value : ProfileValue) : Array Diagnostic :=
  if tags.contains value.tag then #[] else unknown kind value

def schema : ProfileSchema where
  profile := .move
  name := profileName
  version := profileVersion
  checkReference := fun _ => #[]
  checkType := checkTag "type" #[]
  checkOperation := checkTag "operation" #[]
  checkSurface := checkTag "surface" #[]
  checkProperty := checkTag "property" propertyTags
  checkIntrinsic := Intrinsics.check

def propertyValue (tag : String) (payload : String := "") : ProfileValue :=
  { profile := .move, tag, payload }

/-- Validate a Move-only raw unit. Multi-profile tools should construct a
registry explicitly; Move identity is the first-class `.move` constructor and
does not depend on a configuration-table position. -/
def validate (unit : RawUnit) : Except (Array Diagnostic) ValidatedUnit :=
  LeanerIR.Validation.validate #[schema] unit

end LeanerIR.Move

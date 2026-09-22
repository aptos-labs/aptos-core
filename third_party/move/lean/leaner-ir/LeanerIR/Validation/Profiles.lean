-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Import.Raw
import LeanerIR.Validation.Diagnostic

/-!
# Profile registry and profile-independent value semantics

The structural checker and the semantic passes share the registered profile
schemas, the semantic profile-configuration lookup, and the profile-independent
first-slice literal/type relation. This module keeps those definitions below
both consumers so validation can invoke semantic passes without an import
cycle.
-/

namespace LeanerIR.Validation

open LeanerIR.Import

structure ProfileSchema where
  profile : Profile
  name : String
  version : Nat := 1
  checkConfig : ProfileConfig → Array Diagnostic := fun _ => #[]
  checkReference : ReferenceType → Array Diagnostic := fun _ => #[]
  checkType : ProfileValue → Array Diagnostic := fun _ => #[]
  checkOperation : ProfileValue → Array Diagnostic := fun _ => #[]
  checkSurface : ProfileValue → Array Diagnostic := fun _ => #[]
  checkProperty : ProfileValue → Array Diagnostic := fun _ => #[]
  checkIntrinsic : RawUnit → RawNamespace → IntrinsicDecl → Array Diagnostic :=
    fun _ _ _ => #[]

abbrev ProfileRegistry := Array ProfileSchema

/-- Resolve a profile configuration by semantic identity. Extension IDs are
positional; the two known profiles are named directly and may occur anywhere
in the configuration table. -/
def profileConfig? (profiles : Array ProfileConfig) (profile : Profile) : Option ProfileConfig :=
  match profile with
  | .extension id => profiles[id.index]?.filter (·.profile == profile)
  | .move | .rust => profiles.find? (·.profile == profile)

def profileSchema? (registry : ProfileRegistry) (profile : Profile) : Option ProfileSchema :=
  registry.find? (·.profile == profile)

def isFirstSliceConst : ConstValue → Bool
  | .unit | .bool _ | .character _ | .integer _ | .address _ | .string _ | .bytes _ |
      .vector _ | .tuple _ => true
  | _ => false

def isFirstSliceType : Ty → Bool
  | .unit | .bool | .character | .string | .bytes | .address | .signer | .integer _ _ |
      .vector _ _ | .tuple _ => true
  | _ => false

/-- The neutral fixed-length check shared by the literal checker and its
typing lemma. -/
def firstSliceVectorLengthMatches (length : Option ConstValue) (size : Nat) : Bool :=
  match length with
  | none => true
  | some (.integer expected) => expected == Int.ofNat size
  | _ => false

/-- Three-valued conjunction for literal checks: a definite mismatch
dominates, agreement requires both, and everything else stays undecided. -/
def combineFirstSliceChecks : Option Bool → Option Bool → Option Bool
  | some false, _ => some false
  | _, some false => some false
  | some true, some true => some true
  | _, _ => none

mutual

/-- Decide the profile-independent first-slice literal/type relation used by
both raw checking and semantic preparation. `none` leaves profile and later
semantic cases to their owning pass. -/
def firstSliceConstMatchesType (tables : Tables) :
    ConstValue → TypeId → Option Bool
  | value, typeId =>
  match tables.types[typeId.index]? with
  | none => none
  | some ty =>
  match value, ty with
  | .unit, .unit => some true
  | .bool _, .bool => some true
  | .character value, .character => some (isUnicodeScalar value)
  | .string _, .string => some true
  | .bytes _, .bytes => some true
  | .integer value, ty@(.integer _ _) => ty.integerValueFits? value
  | .address _, .address => some true
  | .tuple values, .unit => some values.isEmpty
  | .unit, .tuple types => some types.isEmpty
  | .tuple values, .tuple types =>
      if values.size != types.size then some false
      else firstSlicePairsMatch tables values.toList types.toList
  | .vector values, .vector element length =>
      if !firstSliceVectorLengthMatches length values.size then some false
      else firstSliceAllMatch tables values.toList element
  | value, ty => if isFirstSliceConst value && isFirstSliceType ty then some false else none

/-- Pointwise tuple-member checking with the three-valued conjunction. -/
def firstSlicePairsMatch (tables : Tables) :
    List ConstValue → List TypeId → Option Bool
  | [], [] => some true
  | value :: values, typeId :: typeIds =>
      combineFirstSliceChecks (firstSliceConstMatchesType tables value typeId)
        (firstSlicePairsMatch tables values typeIds)
  | _, _ => some false

/-- Homogeneous vector-member checking with the three-valued conjunction. -/
def firstSliceAllMatch (tables : Tables) :
    List ConstValue → TypeId → Option Bool
  | [], _ => some true
  | value :: values, typeId =>
      combineFirstSliceChecks (firstSliceConstMatchesType tables value typeId)
        (firstSliceAllMatch tables values typeId)

end

end LeanerIR.Validation

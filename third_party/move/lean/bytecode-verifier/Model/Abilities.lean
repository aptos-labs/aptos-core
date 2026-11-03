-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Spec.Syntax
import Model.Result

/-!
# Abilities of signature tokens

UNTRUSTED. Transcribes `BinaryIndexedView::abilities`
(`move-binary-format/src/binary_views.rs`) and
`AbilitySet::polymorphic_abilities` (`move-core/types/src/ability.rs`).
-/

namespace MoveBytecode.Model

/-- `AbilitySet::REFERENCES`. -/
def referenceAbilities : AbilitySet := ⟨true, true, false, false⟩

/-- `AbilitySet::SIGNER`. -/
def signerAbilities : AbilitySet := ⟨false, true, false, false⟩

/-- `AbilitySet::PUBLIC_FUNCTIONS`: copy, drop, store. -/
def publicFunctionAbilities : AbilitySet := ⟨true, true, true, false⟩

/-- `AbilitySet::PRIVATE_FUNCTIONS`: copy, drop. -/
def privateFunctionAbilities : AbilitySet := ⟨true, true, false, false⟩

/-- `Ability::required_by`, over every ability of `a`: the abilities that
require some ability of `a`. Copy requires copy, drop requires drop, and
store and key require store. -/
def requiredBy (a : AbilitySet) : AbilitySet :=
  ⟨a.copy, a.drop, a.store, a.store⟩

/-- `AbilitySet::requires`: what the abilities of `a` require of a type
inside. Copy requires copy, drop requires drop, and store and key require
store. The inverse of `requiredBy`. -/
def requires (a : AbilitySet) : AbilitySet :=
  ⟨a.copy, a.drop, a.store || a.key, false⟩

/-- `AbilitySet::polymorphic_abilities`: the declared abilities, restricted by
every non-phantom type argument. -/
def polymorphicAbilities (declared : AbilitySet) (phantoms : List Bool)
    (args : List AbilitySet) : VResult AbilitySet :=
  if phantoms.length != args.length then reject "VERIFIER_INVARIANT_VIOLATION"
  else pure ((args.zip phantoms).foldl
    (fun acc (a, phantom) => if phantom then acc else acc.inter (requiredBy a)) declared)

/-- `BinaryIndexedView::abilities`. -/
def abilities (m : CompiledModule) (constraints : List AbilitySet) :
    SignatureToken → VResult AbilitySet
  | .bool | .u8 | .u16 | .u32 | .u64 | .u128 | .u256 | .address
  | .i8 | .i16 | .i32 | .i64 | .i128 | .i256 => pure AbilitySet.primitives
  | .reference _ | .mutableReference _ => pure referenceAbilities
  | .signer => pure signerAbilities
  | .typeParameter i => lookup constraints.toArray i "constraints[idx]"
  | .vector t => do polymorphicAbilities AbilitySet.primitives [false] [← abilities m constraints t]
  | .function _ _ a => pure a
  | .struct h => do pure (← lookup m.structHandles h "struct_handle_at").abilities
  | .structInstantiation h args => do
    let sh ← lookup m.structHandles h "struct_handle_at"
    let argAbilities ← args.attach.mapM (fun ⟨a, _⟩ => abilities m constraints a)
    polymorphicAbilities sh.abilities (sh.typeParameters.map (·.isPhantom)) argAbilities

end MoveBytecode.Model

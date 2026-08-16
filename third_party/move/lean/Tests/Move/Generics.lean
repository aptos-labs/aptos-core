-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move
import Tests.Common
import MoveModel.IR.Mono.Transform

namespace Tests.MovePrograms

open Move
open MoveModel.Frontend.XIR
open scoped Move Move.Compiler Move.Spec

move_module Generics where

  @[move_struct]
  structure Box (T : Type) where
    value : T
    deriving Copy, Drop, Store

  @[move_struct]
  structure Pair (T U : Type) where
    first : T
    second : U
    deriving Copy, Drop, Store

  @[move_struct]
  structure Vault (T : Type) where
    value : T
    deriving Key

  @[move_enum]
  inductive Choice (T : Type) where
    | none
    | some (value : T)
    deriving Copy, Drop, Store

  fun identity {T : Type} (value : T) : T := value

  fun box {T : Type} (value : T) : Box T := { value }

  fun unbox {T : Type} (value : Box T) : T := value.value

  fun swap {T U : Type} (value : Pair T U) : Pair U T :=
    { first := value.second, second := value.first }

  fun chooseGeneric {T : Type} (fallback : T) (choice : Choice T) : T :=
    match choice with
    | .none => fallback
    | .some value => value

  fun singleton {T : Type} (value : T) : Move.Vector T := vector![value]

  fun equalGeneric {T : Type} (left right : T) : Bool :=
    left == right

  fun equalU64 (left right : U64) : Bool :=
    left == right

  fun equalBoxes (left right : U64) : Bool :=
    equalGeneric
      ({ value := left } : Box U64)
      ({ value := right } : Box U64)

  fun equalChoices (left right : U64) : Bool :=
    equalGeneric (.some left : Choice U64) (.some right : Choice U64)

  fun equalVectors (left right : U64) : Bool :=
    equalGeneric (vector![left]) (vector![right])

  fun wrap (value : U64) : Box U64 := { value := identity value }

  fun unwrap (box : Box U64) : U64 := box.value

  fun choose (fallback : U64) (choice : Choice U64) : U64 :=
    chooseGeneric fallback choice

  fun swapped (value : U64) : Pair U64 U64 :=
    swap ({ first := value, second := value + 1 } : Pair U64 U64)

  fun singletonLength (value : U64) : U64 :=
    Move.Vector.length (singleton value)

  fun publishGeneric {T : Type} (signer : Signer) (value : T) : Action Unit :=
    moveTo signer ({ value } : Vault T)

  fun hasGeneric {T : Type} (address : Address) : Action Bool :=
    exists_ (Vault T) address

  -- One generic body observing the same resource declaration at `T`, `U`, and
  -- a concrete type exercises all runtime-tag equality combinations used by
  -- verification monomorphization.
  fun tagInteractions {T U : Type} (address : Address) : Action Bool := do
    let hasT ← exists_ (Vault T) address
    let hasU ← exists_ (Vault U) address
    let hasU64 ← exists_ (Vault U64) address
    pure (hasT && hasU && hasU64)

  fun publishVault (signer : Signer) (value : U64) : Action Unit :=
    publishGeneric signer value

  fun takeVault (address : Address) : Action U64 := do
    let vault ← moveFrom (Vault U64) address
    pure vault.value

  fun hasVault (address : Address) : Action Bool := hasGeneric (T := U64) address

  fun publishBoolVault (signer : Signer) (value : Bool) : Action Unit :=
    publishGeneric signer value

  fun takeBoolVault (address : Address) : Action Bool := do
    let vault ← moveFrom (Vault Bool) address
    pure vault.value

  fun hasBoolVault (address : Address) : Action Bool :=
    hasGeneric (T := Bool) address

  spec wrap (value : U64) where
    ensures result = ({ value } : Box U64)

  verify wrap

  spec unwrap (box : Box U64) where
    ensures result = box.value

  verify unwrap

  spec choose (fallback : U64) (choice : Choice U64) where
    ensures
      result =
        match choice with
        | .none => fallback
        | .some value => value

  verify choose

  spec swapped (value : U64) where
    ensures
      result = ({ first := value + 1, second := value } : Pair U64 U64)

  verify swapped

  def compiled : MModule := move_module% "Generics"

namespace Generics

private def run := Tests.run compiled.toMProgram

#test run "identity" [] [.u64 11] = Tests.okU64 11
#test run "wrap" [] [.u64 12] = Tests.okVals [.struct [.u64 12]]
#test run "unwrap" [] [.struct [.u64 13]] = Tests.okU64 13
#test run "choose" [] [.u64 4, .variant 0 []] = Tests.okU64 4
#test run "choose" [] [.u64 4, .variant 1 [.u64 15]] = Tests.okU64 15
#test run "swapped" [] [.u64 16] =
  Tests.okVals [.struct [.u64 17, .u64 16]]
#test run "singletonLength" [] [.u64 17] = Tests.okU64 1
#test run "equalU64" [] [.u64 17, .u64 17] = Tests.okVals [.bool true]
#test run "equalU64" [] [.u64 17, .u64 18] = Tests.okVals [.bool false]
#test run "equalBoxes" [] [.u64 17, .u64 17] = Tests.okVals [.bool true]
#test run "equalBoxes" [] [.u64 17, .u64 18] = Tests.okVals [.bool false]
#test run "equalChoices" [] [.u64 19, .u64 19] = Tests.okVals [.bool true]
#test run "equalChoices" [] [.u64 19, .u64 20] = Tests.okVals [.bool false]
#test run "equalVectors" [] [.u64 21, .u64 21] = Tests.okVals [.bool true]
#test run "equalVectors" [] [.u64 21, .u64 22] = Tests.okVals [.bool false]

private def vaultId := compiled.resourceId "Vault"
private def vaultKey (ty : MoveModel.IR.Ty) := MoveModel.IR.resourceKey vaultId [ty]

#guard vaultKey .u64 != vaultKey .bool
#guard vaultKey (.vector .u64) != vaultKey (.vector .bool)

private def vaultMemory (address value : Nat) : MoveModel.IR.IMem :=
  [(vaultKey .u64, address, .struct [.u64 value])]

private def bothVaults (address value : Nat) (flag : Bool) : MoveModel.IR.IMem :=
  [(vaultKey .bool, address, .struct [.bool flag]),
   (vaultKey .u64, address, .struct [.u64 value])]

#test run "publishVault" [] [.address 7, .u64 18] =
  Tests.okRet (vaultMemory 7 18) []
#test run "takeVault" (vaultMemory 7 19) [.address 7] = Tests.okU64 19
#test run "hasVault" (vaultMemory 7 20) [.address 7] =
  Tests.okRet (vaultMemory 7 20) [.bool true]
#test run "hasVault" [] [.address 7] = Tests.okVals [.bool false]
#test run "publishBoolVault" (vaultMemory 7 21) [.address 7, .bool true] =
  Tests.okRet (bothVaults 7 21 true) []
#test run "hasVault" (bothVaults 7 22 true) [.address 7] =
  Tests.okRet (bothVaults 7 22 true) [.bool true]
#test run "hasBoolVault" (bothVaults 7 23 true) [.address 7] =
  Tests.okRet (bothVaults 7 23 true) [.bool true]
#test run "takeBoolVault" (bothVaults 7 24 true) [.address 7] =
  Tests.okRet (vaultMemory 7 24) [.bool true]

private def monoResult := compiled.toModule.monomorphizeForVerification

private def publishGenericId := compiled.funId "publishGeneric"
private def tagInteractionsId := compiled.funId "tagInteractions"

-- Concrete call sites add both storage-distinct instantiations of the generic
-- publisher to the finite verification plan.
#guard match monoResult with
  | .ok (plan, _) =>
      plan.entries.any (fun key =>
        key == ⟨publishGenericId, [.u64]⟩) &&
      plan.entries.any (fun key =>
        key == ⟨publishGenericId, [.bool]⟩)
  | .error _ => false

private def hasTagCase (plan : MoveModel.IR.MonoPlan) (args : List MoveModel.IR.Ty) : Bool :=
  plan.entries.any fun key =>
    key == ⟨tagInteractionsId,
      MoveModel.IR.rigidifyTypeArgs tagInteractionsId args⟩

-- For `Vault<T>`, `Vault<U>`, and `Vault<u64>`, verify the generic-distinct
-- case, each individual collision, and the simultaneous collision.  These
-- are exactly the equality patterns visible to global storage.
#guard match monoResult with
  | .ok (plan, _) =>
      hasTagCase plan [.typeParam 0, .typeParam 1] &&
      hasTagCase plan [.typeParam 1, .typeParam 1] &&
      hasTagCase plan [.u64, .typeParam 1] &&
      hasTagCase plan [.typeParam 0, .u64] &&
      hasTagCase plan [.u64, .u64]
  | .error _ => false

private def isInstantiatedCall : MoveModel.IR.Instr → Bool
  | .call _ (.functionInst _ _) _ => true
  | _ => false

-- The generated verification module is binder-free and all generic function
-- calls have been resolved to ordinary generated function ids.
#guard match monoResult with
  | .ok (_, mono) =>
      (List.range mono.numFuns).all fun f =>
        match mono.program.funs f with
        | some d =>
            d.typeParams.isEmpty &&
            (List.range d.body.size).all fun b =>
              match d.body.blocks b with
              | some block => !block.instrs.any isInstantiatedCall
              | none => false
        | none => false
  | .error _ => false

end Generics

end Tests.MovePrograms

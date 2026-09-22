-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
-- Test category: Move language.

import Move
import MoveModel.Tests.Common
import MoveModel.IR.Mono.Transform

namespace Tests.MovePrograms

open Move
open scoped Move Move.Compiler Move.Spec

module Generics where

  struct Box (T) has Copy, Drop, Store where
    value : T

  struct Pair (T U) has Copy, Drop, Store where
    first : T
    second : U

  struct Vault (T) has Key where
    value : T

  enum Choice (T) has Copy, Drop, Store where
    | None
    | Some (value : T)

  /-! ## Functions -/

  fun identity {T} (value : T) : T := value

  spec identity {T} [Inhabited T] (value : T) where
    ensures result = value

  fun box {T} (value : T) : Box T := { value }

  spec box {T} [Inhabited T] (value : T) where
    ensures result = ({ value } : Box T)

  fun unbox {T} (value : Box T) : T := value.value

  spec unbox {T} [Inhabited T] (value : Box T) where
    ensures result = value.value

  fun swap {T U} (value : Pair T U) : Pair U T :=
    { first := value.second, second := value.first }

  spec swap {T} {U} [Inhabited T] [Inhabited U]
      (value : Pair T U) where
    ensures
      result = ({ first := value.second, second := value.first } : Pair U T)

  fun choose_generic {T} (fallback : T) (choice : Choice T) : T :=
    match choice with
    | .None => fallback
    | .Some value => value

  spec choose_generic {T} [Inhabited T] (fallback : T) (choice : Choice T) where
    ensures
      result =
        match choice with
        | .None => fallback
        | .Some value => value

  fun singleton {T} (value : T) : Vector T := vector![value]

  spec singleton {T} [Inhabited T] (value : T) where
    ensures result = vector![value]

  fun equal_generic {T} (left right : T) : Bool :=
    left == right

  spec equal_generic {T} [Inhabited T] (left : T) (right : T) where
    ensures result = (left == right)

  fun equal_u64 (left right : U64) : Bool :=
    left == right

  spec equal_u64 (left : U64) (right : U64) where
    ensures result = (left == right)

  fun equal_boxes (left right : U64) : Bool :=
    equal_generic
      ({ value := left } : Box U64)
      ({ value := right } : Box U64)

  spec equal_boxes (left : U64) (right : U64) where
    ensures
      result =
        (({ value := left } : Box U64) ==
          ({ value := right } : Box U64))

  fun equal_choices (left right : U64) : Bool :=
    equal_generic (.Some left : Choice U64) (.Some right : Choice U64)

  spec equal_choices (left : U64) (right : U64) where
    ensures
      result = ((.Some left : Choice U64) == (.Some right : Choice U64))

  fun equal_vectors (left right : U64) : Bool :=
    equal_generic (vector![left]) (vector![right])

  spec equal_vectors (left : U64) (right : U64) where
    ensures result = (vector![left] == vector![right])

  fun wrap (value : U64) : Box U64 := { value := identity value }

  spec wrap (value : U64) where
    ensures result = ({ value } : Box U64)

  fun unwrap (box : Box U64) : U64 := box.value

  spec unwrap (box : Box U64) where
    ensures result = box.value

  fun choose (fallback : U64) (choice : Choice U64) : U64 :=
    choose_generic fallback choice

  spec choose (fallback : U64) (choice : Choice U64) where
    ensures
      result =
        match choice with
        | .None => fallback
        | .Some value => value

  fun swapped (value : U64) : Pair U64 U64 :=
    swap ({ first := value, second := value + 1 } : Pair U64 U64)

  spec swapped (value : U64) where
    ensures
      result = ({ first := value + 1, second := value } : Pair U64 U64)

  fun singleton_length (value : U64) : U64 :=
    Move.Vector.length (singleton value)

  spec singleton_length (value : U64) where
    ensures result = 1

  -- Global-storage primitives are not yet modeled by automatic source
  -- specifications, so the resource functions below intentionally remain
  -- unspecced.
  fun publish_generic {T} (signer : &Signer) (value : T) : Action Unit :=
    moveTo signer ({ value } : Vault T)

  fun has_generic {T} (address : Address) : Action Bool :=
    existsAt (Vault T) address

  -- One generic body observing the same resource declaration at `T`, `U`, and
  -- a concrete type exercises all runtime-tag equality combinations used by
  -- verification monomorphization.
  fun tag_interactions {T U} (address : Address) : Action Bool := do
    let hasT ← existsAt (Vault T) address
    let hasU ← existsAt (Vault U) address
    let hasU64 ← existsAt (Vault U64) address
    pure (hasT && hasU && hasU64)

  fun publish_vault (signer : &Signer) (value : U64) : Action Unit :=
    publish_generic signer value

  fun take_vault (address : Address) : Action U64 := do
    let vault ← moveFrom (Vault U64) address
    let Vault { value } := vault
    pure value

  fun has_vault (address : Address) : Action Bool := has_generic (T := U64) address

  fun publish_bool_vault (signer : &Signer) (value : Bool) : Action Unit :=
    publish_generic signer value

  fun take_bool_vault (address : Address) : Action Bool := do
    let vault ← moveFrom (Vault Bool) address
    let Vault { value } := vault
    pure value

  fun has_bool_vault (address : Address) : Action Bool :=
    has_generic (T := Bool) address

  /-! ## Proofs -/

  verify identity
  verify box
  verify unbox
  verify swap
  verify choose_generic
  verify singleton
  verify equal_generic
  verify equal_u64
  verify equal_boxes
  verify equal_choices
  verify equal_vectors
  verify wrap
  verify unwrap
  verify choose
  verify swapped

  verify singleton_length

  /-! ## Tests -/

  def compiled : MoveModel.IR.Module := lowerToIR ``Tests.MovePrograms.Generics

namespace Generics

private def run := Tests.run compiled

#test run "identity" [] [.u64 11] = Tests.okU64 11
#test run "wrap" [] [.u64 12] = Tests.okVals [.struct [.u64 12]]
#test run "unwrap" [] [.struct [.u64 13]] = Tests.okU64 13
#test run "choose" [] [.u64 4, .variant 0 []] = Tests.okU64 4
#test run "choose" [] [.u64 4, .variant 1 [.u64 15]] = Tests.okU64 15
#test run "swapped" [] [.u64 16] =
  Tests.okVals [.struct [.u64 17, .u64 16]]
#test run "singleton_length" [] [.u64 17] = Tests.okU64 1
#test run "equal_u64" [] [.u64 17, .u64 17] = Tests.okVals [.bool true]
#test run "equal_u64" [] [.u64 17, .u64 18] = Tests.okVals [.bool false]
#test run "equal_boxes" [] [.u64 17, .u64 17] = Tests.okVals [.bool true]
#test run "equal_boxes" [] [.u64 17, .u64 18] = Tests.okVals [.bool false]
#test run "equal_choices" [] [.u64 19, .u64 19] = Tests.okVals [.bool true]
#test run "equal_choices" [] [.u64 19, .u64 20] = Tests.okVals [.bool false]
#test run "equal_vectors" [] [.u64 21, .u64 21] = Tests.okVals [.bool true]
#test run "equal_vectors" [] [.u64 21, .u64 22] = Tests.okVals [.bool false]

private def vaultId := compiled.resourceId "Vault"
private def vaultKey (ty : MoveModel.IR.Ty) := MoveModel.IR.resourceKey vaultId [ty]

#guard vaultKey .u64 != vaultKey .bool
#guard vaultKey (.vector .u64) != vaultKey (.vector .bool)

-- Reading existence and publishing do not acquire a resource; removal does.
#guard match compiled.funMeta? "has_generic" with
  | some info => info.acquires.isEmpty
  | none => false
#guard match compiled.funMeta? "publish_generic" with
  | some info => info.acquires.isEmpty
  | none => false
#guard match compiled.funMeta? "take_vault" with
  | some info => info.acquires == [vaultId]
  | none => false

private def vaultMemory (address value : Nat) : MoveModel.IR.IMem :=
  [(vaultKey .u64, address, .struct [.u64 value])]

private def bothVaults (address value : Nat) (flag : Bool) : MoveModel.IR.IMem :=
  [(vaultKey .bool, address, .struct [.bool flag]),
   (vaultKey .u64, address, .struct [.u64 value])]

#test run "publish_vault" [] [.address 7, .u64 18] =
  Tests.okRet (vaultMemory 7 18) []
#test run "take_vault" (vaultMemory 7 19) [.address 7] = Tests.okU64 19
#test run "has_vault" (vaultMemory 7 20) [.address 7] =
  Tests.okRet (vaultMemory 7 20) [.bool true]
#test run "has_vault" [] [.address 7] = Tests.okVals [.bool false]
#test run "publish_bool_vault" (vaultMemory 7 21) [.address 7, .bool true] =
  Tests.okRet (bothVaults 7 21 true) []
#test run "has_vault" (bothVaults 7 22 true) [.address 7] =
  Tests.okRet (bothVaults 7 22 true) [.bool true]
#test run "has_bool_vault" (bothVaults 7 23 true) [.address 7] =
  Tests.okRet (bothVaults 7 23 true) [.bool true]
#test run "take_bool_vault" (bothVaults 7 24 true) [.address 7] =
  Tests.okRet (vaultMemory 7 24) [.bool true]

private def monoResult := compiled.monomorphizeForVerification

private def publishGenericId := compiled.funId "publish_generic"
private def tagInteractionsId := compiled.funId "tag_interactions"

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

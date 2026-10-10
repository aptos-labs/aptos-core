-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
-- Test category: specification and verification.

import Move
import MoveModel.Tests.Common
open Move
open scoped Move Move.Compiler Move.Spec

/-! Generic global storage under `verify`: a resource family `Vault T` used
through `moveTo`/`moveFrom`/`existsAt` and global borrows at a type parameter
or a concrete instantiation, and named in contracts as `existsAt<Vault T>`,
`(Vault T)[a].f`, `modifies (Vault T)[a]`.  A head's store is in scope for
every instantiation, so a caller reaches a callee's generic family without
spelling the instantiation; distinct heads are independent, two instantiations
of one head are not assumed to be. -/

module GenericStorage where
  struct Vault (T) has Key where
    value : T

  struct Counter has Key where
    value : U64

  fun publish_generic {T} (signer : &Signer) (value : T) : Action Unit :=
    moveTo signer ({ value } : Vault T)

  spec publish_generic {T} (signer : &Signer) (value : T) where
    requires ¬existsAt<Vault T>(signer.address);
    modifies (Vault T)[signer.address];
    ensures (Vault T)[signer.address].value = value;
    aborts_if False

  verify publish_generic

  -- `T` is determined by no argument: the contract instantiates it by name.
  fun has_generic {T} (address : Address) : Action Bool :=
    existsAt (Vault T) address

  spec has_generic {T} (address : Address) where
    ensures result = true ↔ existsAt<Vault T>(address);
    aborts_if False

  verify has_generic

  -- A named type argument at the call site instantiates the callee.
  fun has_u64_vault (address : Address) : Action Bool :=
    has_generic (T := U64) address

  spec has_u64_vault (address : Address) where
    ensures result = true ↔ existsAt<Vault U64>(address);
    aborts_if False

  verify has_u64_vault

  fun take_generic {T} (address : Address) : Action T := do
    let vault ← moveFrom (Vault T) address
    let Vault { value } := vault
    pure value

  spec take_generic {T} (address : Address) where
    requires existsAt<Vault T>(address);
    modifies (Vault T)[address];
    ensures result = old((Vault T)[address].value) ∧ ¬existsAt<Vault T>(address);
    aborts_if False

  verify take_generic

  -- A global invariant may name a concrete instantiation of a generic
  -- resource family. It is registered by the `Vault` head while retaining
  -- `U64` in its store and reestablishment obligations.
  spec module where
    invariant ∀ a, (Vault U64)[a].value = (Vault U64)[a].value

  -- A global field borrow at a concrete instantiation.
  entry fun bump_vault (address : Address) : Action Unit := do
    let value ← &mut (Vault U64)[address].value
    value := *value + 1

  spec bump_vault (address : Address) where
    requires existsAt<Vault U64>(address);
    modifies (Vault U64)[address];
    ensures (Vault U64)[address].value = old((Vault U64)[address].value) + 1;
    aborts_if ¬old((Vault U64)[address].value).toNat + 1 < U64.size
      with Semantics.Checked.arithmeticAbortCode

  verify bump_vault

  -- Two heads, one generic: `Counter` and `Vault U64` are independent.
  entry fun mirror (address : Address) : Action Unit := do
    let counter ← &Counter[address]
    let current ← *counter
    let value ← &mut (Vault U64)[address].value
    value := current.value

  spec mirror (address : Address) where
    requires existsAt<Counter>(address) ∧ existsAt<Vault U64>(address);
    modifies (Vault U64)[address];
    ensures (Vault U64)[address].value = old(Counter[address].value);
    aborts_if False

  verify mirror

  -- The callee's `Vault T` is the caller's `Vault U64`: the head's store is in
  -- scope for every instantiation, and the caller's clauses name the one it
  -- frames.
  entry fun publish_u64 (signer : &Signer) (value : U64) : Action Unit :=
    publish_generic signer value

  spec publish_u64 (signer : &Signer) (value : U64) where
    requires ¬existsAt<Vault U64>(signer.address);
    modifies (Vault U64)[signer.address];
    ensures (Vault U64)[signer.address].value = value;
    aborts_if False

  verify publish_u64

  /-! ## Tests -/

  def compiled : MoveModel.IR.Module := lowerToIR ``GenericStorage

  private def vaultId := compiled.resourceId "Vault"
  private def counterId := compiled.resourceId "Counter"
  private def vaultKey (ty : MoveModel.IR.Ty) := MoveModel.IR.resourceKey vaultId [ty]
  private def vaultMemory (address value : Nat) : MoveModel.IR.IMem :=
    [(vaultKey .u64, address, .struct [.u64 value])]
  private def both (address counter value : Nat) : MoveModel.IR.IMem :=
    [(counterId, address, .struct [.u64 counter]),
     (vaultKey .u64, address, .struct [.u64 value])]
  -- A write moves the written entry to the front of the memory list.
  private def mirrored (address value : Nat) : MoveModel.IR.IMem :=
    [(vaultKey .u64, address, .struct [.u64 value]),
     (counterId, address, .struct [.u64 value])]
  private def run := Tests.run compiled

  #test run "publish_u64" [] [.address 7, .u64 18] = Tests.okRet (vaultMemory 7 18) []
  #test run "has_u64_vault" (vaultMemory 7 20) [.address 7] =
    Tests.okRet (vaultMemory 7 20) [.bool true]
  #test run "has_u64_vault" [] [.address 7] = Tests.okVals [.bool false]
  #test run "bump_vault" (vaultMemory 7 20) [.address 7] = Tests.okRet (vaultMemory 7 21) []
  #test run "mirror" (both 7 5 20) [.address 7] = Tests.okRet (mirrored 7 5) []

-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.Tests.XIRCommon
import MoveModel.Frontend.Elab

/-! True Move generics imported through exchange XIR.  The declarations retain
their constraints and phantom markers; concrete wrappers exercise generic
calls and instantiated struct operations in the Lean interpreter. -/

namespace Tests.Frontend.Generics

open MoveModel.IR
open MoveModel.Frontend.XIR

def imported : MProgram := moveM% "
module 0x42::generics {
    struct Box<T: copy + drop + store> has copy, drop, store { value: T }
    struct Marker<phantom T> has copy, drop, store { id: u64 }
    struct Vault<phantom T> has key { value: u64 }
    enum Choice<T: copy + drop> has copy, drop { None, Some(T) }

    fun identity<T: copy + drop>(value: T): T {
        value
    }

    fun choose<T: copy + drop>(left: T, right: T, use_left: bool): T {
        if (use_left) left else right
    }

    fun round_trip(value: u64): u64 {
        let boxed = Box<u64> { value: identity<u64>(value) };
        let Box { value } = boxed;
        value
    }

    fun choose_u64(left: u64, right: u64, use_left: bool): u64 {
        choose<u64>(left, right, use_left)
    }

    fun marker_id(id: u64): u64 {
        let Marker<u64> { id } = Marker<u64> { id };
        id
    }

    fun inspect_choice(value: u64): u64 {
        let choice = Choice::Some<u64>(value);
        match (choice) {
            Choice::None => 0,
            Choice::Some(inner) => inner,
        }
    }

    fun publish<T>(account: &signer, value: u64) {
        move_to(account, Vault<T> { value })
    }

    fun read<T>(addr: address): u64 acquires Vault {
        borrow_global<Vault<T>>(addr).value
    }

    fun replace<T>(addr: address, value: u64) acquires Vault {
        borrow_global_mut<Vault<T>>(addr).value = value
    }

    fun take<T>(addr: address): u64 acquires Vault {
        let Vault { value } = move_from<Vault<T>>(addr);
        value
    }

    fun contains<T>(addr: address): bool {
        exists<Vault<T>>(addr)
    }

    fun publish_u64(account: &signer, value: u64) {
        publish<u64>(account, value)
    }

    fun publish_bool(account: &signer, value: u64) {
        publish<bool>(account, value)
    }

    fun read_u64(addr: address): u64 acquires Vault {
        read<u64>(addr)
    }

    fun replace_u64(addr: address, value: u64) acquires Vault {
        replace<u64>(addr, value)
    }

    fun take_u64(addr: address): u64 acquires Vault {
        take<u64>(addr)
    }

    fun contains_u64(addr: address): bool {
        contains<u64>(addr)
    }

    fun contains_bool(addr: address): bool {
        contains<bool>(addr)
    }
}
"

private def run := Tests.runXIR imported

#test run "round_trip" [] [.u64 41] = Tests.okU64 41
#test run "choose_u64" [] [.u64 3, .u64 8, .bool true] = Tests.okU64 3
#test run "choose_u64" [] [.u64 3, .u64 8, .bool false] = Tests.okU64 8
#test run "marker_id" [] [.u64 17] = Tests.okU64 17
#test run "inspect_choice" [] [.u64 29] = Tests.okU64 29

private def vault := imported.resourceId "Vault"
private def vaultAt (address value : Nat) : IMem :=
  [(resourceKey vault [.u64], address, .struct [.u64 value])]

private def bothVaults (address u64Value boolValue : Nat) : IMem :=
  [(resourceKey vault [.bool], address, .struct [.u64 boolValue]),
   (resourceKey vault [.u64], address, .struct [.u64 u64Value])]

#test run "publish_u64" [] [.address 7, .u64 31] = Tests.okRet (vaultAt 7 31) []
#test run "read_u64" (vaultAt 7 31) [.address 7] =
  Tests.okRet (vaultAt 7 31) [.u64 31]
#test run "replace_u64" (vaultAt 7 31) [.address 7, .u64 44] =
  Tests.okRet (vaultAt 7 44) []
#test run "take_u64" (vaultAt 7 31) [.address 7] = Tests.okU64 31
#test run "contains_u64" (vaultAt 7 31) [.address 7] =
  Tests.okRet (vaultAt 7 31) [.bool true]
#test run "contains_u64" [] [.address 7] = Tests.okBool false
#test run "publish_bool" (vaultAt 7 31) [.address 7, .u64 9] =
  Tests.okRet (bothVaults 7 31 9) []
#test run "contains_u64" (bothVaults 7 31 9) [.address 7] =
  Tests.okRet (bothVaults 7 31 9) [.bool true]
#test run "contains_bool" (bothVaults 7 31 9) [.address 7] =
  Tests.okRet (bothVaults 7 31 9) [.bool true]

#guard match imported.structs.find? (·.name == "Box"),
    imported.structs.find? (·.name == "Marker"),
    imported.structs.find? (·.name == "Choice") with
  | some box, some marker, some choice =>
      box.typeParams.length = 1 &&
      (marker.typeParams.head?.map (·.phantom)).getD false &&
      choice.variants.isSome
  | _, _, _ => false
#guard match imported.funs.find? (·.name == "identity") with
  | some identity =>
      match identity.typeParams.head? with
      | some param => param.abilities.copy && param.abilities.drop
      | none => false
  | none => false

end Tests.Frontend.Generics

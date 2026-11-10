-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Spec.Syntax
import Model.Result

/-!
# Friend declarations

UNTRUSTED. Transcribes `move-bytecode-verifier/src/friends.rs`: a module may
not befriend itself, and may befriend only modules at its own address (an
Aptos policy, not a technical requirement).
-/

namespace MoveBytecode.Model

/-- `friends::verify_module`. -/
def verifyFriends (m : CompiledModule) : VResult Unit := do
  let selfHandle ← lookup m.moduleHandles m.selfModuleHandleIdx "self_handle"
  if m.friendDecls.contains selfHandle then reject "INVALID_FRIEND_DECL_WITH_SELF"
  else
    let selfAddress ← lookup m.addressIdentifiers selfHandle.address "address_identifier_at"
    let addresses ← m.friendDecls.toList.mapM fun h =>
      lookup m.addressIdentifiers h.address "address_identifier_at"
    if addresses.any (· != selfAddress) then
      reject "INVALID_FRIEND_DECL_WITH_MODULES_OUTSIDE_ACCOUNT_ADDRESS"
    else pure ()

end MoveBytecode.Model

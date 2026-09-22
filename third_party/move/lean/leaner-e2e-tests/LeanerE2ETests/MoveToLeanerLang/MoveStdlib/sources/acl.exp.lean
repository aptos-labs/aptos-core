-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
Access control list (acl) module. An acl is a list of account addresses who
have the access permission to a certain object.
This module uses a `vector` to represent the list, but can be refactored to
use a "set" instead when it's available in the language in the future.
-/
leaner module 0x1::acl where
  use 0x1::std::error::invalid_argument
  use 0x1::std::vector

  /--
  The ACL already contains the address.
  -/
  const ECONTAIN : u64 := 0

  /--
  The ACL does not contain the address.
  -/
  const ENOT_CONTAIN : u64 := 1

  struct ACL has Copy, Drop, Store where
    list : Vector<Address>

  spec ACL where
    invariant ∀ (i in 0 .. list.length; j in 0 .. list.length),
        list[i] == list[j] ==> i == j

  /--
  Return an empty ACL.
  -/
  public fun empty() -> ACL := new ACL { list := vector<Address>[] }

  spec empty where
    pragma opaque
    ensures [inferred] result == new ACL { list := vector<Address>[] }
    aborts_if [inferred] false

  /--
  Add the address to the ACL.
  -/
  public fun add(self : &mut ACL, addr : Address) -> Unit := do
    assert!(!self.list.contains(&addr), invalid_argument(ECONTAIN))
    self.list := core.prim.pushVector(self.list, addr)

  spec add where
    pragma opaque
    aborts_if spec_contains(self, addr) with 1
    ensures spec_contains(self, addr)

  /--
  Remove the address from the ACL.
  -/
  public fun remove(self : &mut ACL, addr : Address) -> Unit := do
    let (found, index) := self.list.index_of(&addr)
    assert!(found, invalid_argument(ENOT_CONTAIN))
    self.list.remove(index)

  spec remove where
    pragma opaque
    aborts_if !spec_contains(self, addr) with 1
    ensures !spec_contains(self, addr)

  /--
  Return true iff the ACL contains the address.
  -/
  public fun contains(self : &ACL, addr : Address) -> Bool :=
    self.list.contains(&addr)

  spec contains where
    pragma opaque
    ensures result == spec_contains(self, addr)
    aborts_if false

  /--
  assert! that the ACL has the address.
  -/
  public fun assert_contains(self : &ACL, addr : Address) -> Unit :=
    assert!(self.contains(addr), invalid_argument(ENOT_CONTAIN))

  spec assert_contains where
    pragma opaque
    aborts_if !spec_contains(self, addr) with 1

  spec fun spec_contains(self : ACL, addr : Address) : Bool :=
    ∃ (a in self.list), a == addr

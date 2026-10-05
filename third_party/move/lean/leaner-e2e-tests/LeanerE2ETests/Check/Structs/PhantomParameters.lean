-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
# Generic structs inside a resource

A resource whose field is an instance of a generic struct, its type
parameters phantom: the resource's specification type holds the instance,
and a read of the resource decodes it.
-/

namespace LeanerLang.Tests.Check.Structs.PhantomParameters

leaner module 0x42::phantom_parameters where
  struct Holder {K : phantom type has Copy, Drop} {V : phantom type} has Store where
    handle : Address
    length : u64

  struct Item has Copy, Drop, Store where
    flag : Bool

  struct Account has Key where
    items : Holder<u64, Item>
    count : u64

  fun count_of(a : Address) -> u64 := Account[a].count
  spec count_of where
    ensures result == global<Account>(a).count

  fun length_of(a : Address) -> u64 := Account[a].items.length
  spec length_of where
    ensures result == global<Account>(a).items.length

end LeanerLang.Tests.Check.Structs.PhantomParameters

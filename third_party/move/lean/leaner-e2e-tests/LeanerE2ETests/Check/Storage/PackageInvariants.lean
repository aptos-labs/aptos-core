-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
# Invariants across the modules of a file

A module invariant holds for every function of the file that writes memory
it reads: one registered before the invariant's module is verified against
it there, and one registered after links the invariant's module, though it
does not use it.
-/

namespace LeanerLang.Tests.Check.Storage.PackageInvariants

leaner module 0x42::store where
  struct R has Key where
    value : u64

  -- Publishes a zero value, which `bounds` excludes.
  public fun put(s : &Signer, v : u64) -> Unit :=
    move_to<R>(s, new R { value := v })
  spec put where
    modifies *

leaner module 0x42::bounds where
  use 0x42::store::R

  spec module where
    invariant forall (a : Address), exists<R>(a) ==> global<R>(a).value > 0

leaner module 0x42::client where
  use 0x42::store::put

  public fun put_one(s : &Signer) -> Unit := put(s, 1)
  spec put_one where
    modifies *

  -- Publishes a zero value.
  public fun put_zero(s : &Signer) -> Unit := put(s, 0)
  spec put_zero where
    modifies *

end LeanerLang.Tests.Check.Storage.PackageInvariants

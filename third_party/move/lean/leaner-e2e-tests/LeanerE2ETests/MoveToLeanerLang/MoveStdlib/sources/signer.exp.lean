-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner module 0x1::signer where
  /--
  signer is a builtin move type that represents an address that has been verfied by the VM.

  VM Runtime representation is equivalent to following:
  ```
  enum signer has drop {
      Master { account: address },
      Permissioned { account: address, permissions_address: address },
  }
  ```

  for bcs serialization:

  ```
  struct signer has drop {
      account: address,
  }
  ```
  ^ The discrepency is needed to maintain backwards compatibility of signer serialization
  semantics.

  `borrow_address` borrows this inner field
  -/
  public native fun borrow_address(self : &Signer) -> &Address

  -- Copies the address of the signer
  public fun address_of(self : &Signer) -> Address := *self.borrow_address()

  opaque spec fun is_txn_signer(self : Signer) : Bool

  opaque spec fun is_txn_signer_addr(a : Address) : Bool

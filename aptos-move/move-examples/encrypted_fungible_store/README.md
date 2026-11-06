# EncryptedFungibleStore accounting prototype

**This example does not encrypt anything. Do not deploy it for confidential custody.**
It models the Move API and backing-asset accounting for a future TEE-backed store.
Resource bytes, object ownership, arguments, and framework deposit/withdraw events
are public today. Owner checks on `balance` do not hide REST/storage reads.

## API

- `create_store(owner, metadata)` creates an owner-controlled, nontransferable
  `EncryptedFungibleStore` with an ordinary `u64` balance. Multiple stores per
  owner/asset are allowed.
- `shield(sender, recipient_store, amount)` atomically moves real assets from the
  sender's primary store to the asset's pooled reserve and credits the recipient.
  The sender may fund another owner's store.
- `transfer(owner, source, destination, amount)` moves balances of the same asset
  without touching public reserves. No trading-specific locks or order logic.
- `unshield(owner, source, public_recipient, amount)` atomically debits the source
  and releases backing assets to a public primary store.
- `balance(owner, store)` checks the owner signer. `reserve_accounting(metadata)`
  returns public reserve balance and aggregate outstanding claims.

No mint or issuer transfer capability is retained. Each reserve object is owned
by `0x0`, has ungated object transfers disabled, and keeps its `ExtendRef` private
to this module. Owning the module account does not authorize direct framework
withdrawals from the reserve. Module upgrades remain a custody trust assumption.

For each asset, the reserve must cover outstanding claims; direct donations can
make it overcollateralized. Successful shield/unshield operations change backing
and claims together, and aborted Move transactions roll back both. Transfers
preserve both totals.

## Scope

Only transferable assets without dispatch/derived-supply hooks are supported;
these restrictions are rechecked during operations. An issuer freezing the pooled
reserve pauses operations. Per-user issuer freezes, clawbacks, and arbitrary
token-specific policies are not implemented by this private-account model.

Real confidentiality requires VM-enforced resource classification, attested key
provisioning, encrypted inputs and canonical committed outputs, protected object
metadata, and authenticated read APIs. There is deliberately no fake encryption,
hardcoded key, unsupported annotation, consensus change, or trading logic here.
All validators executing inside approved TEEs is a future runtime assumption,
not something this example establishes.

## Tests

From the repository root, using the checkout's compiler and Aptos natives:

```sh
RUST_MIN_STACK=67108864 cargo test -p aptos-framework --test move_unit_test \
  move_encrypted_fungible_store_unit_tests -- --nocapture
```

The tests assert exact public/private balances and reserve liabilities across
shield, transfer, self-transfer, and full unshield; they reject unauthorized
access, overdrafts, asset mismatches, frozen reserves, and publisher reserve theft.
This standalone package does not modify the framework or its cached bundle.
The test is registered in the existing framework test harness. Older released
CLIs may not understand this checkout's newer framework specifications.

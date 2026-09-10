/*
Inference returns: exiting with bytecode transformation errors
Inference diagnostics:
error: WP cannot complete `vault::harvest` while a transparent callee lacks a complete opaque contract. Repair the named callee boundary before changing or rerunning the caller. Reasons:
  = transparent callee `0x1::fungible_asset::balance` is outside the editable WP scope and has no complete opaque contract; WP cannot construct a complete caller specification. The package or corpus must provide and verify a complete opaque contract for that callee before the caller is rerun
  = transparent callee `0x1::fungible_asset::deposit` is outside the editable WP scope and has no complete opaque contract; WP cannot construct a complete caller specification. The package or corpus must provide and verify a complete opaque contract for that callee before the caller is rerun
  = transparent callee `0x1::fungible_asset::withdraw` is outside the editable WP scope and has no complete opaque contract; WP cannot construct a complete caller specification. The package or corpus must provide and verify a complete opaque contract for that callee before the caller is rerun
  = a dynamic call has no trusted complete abort summary
   ┌─ tests/inference/vault.move:21:5
   │
21 │ ╭     public fun harvest(caller: &signer, vault_obj: Object<Vault>) {
22 │ │         let vault = &mut Vault[vault_obj.object_address()];
23 │ │
24 │ │         // Withdraw all assets from the vault's store
   · │
32 │ │         fungible_asset::deposit(vault.store, returned_assets);
33 │ │     }
   │ ╰─────^
*/

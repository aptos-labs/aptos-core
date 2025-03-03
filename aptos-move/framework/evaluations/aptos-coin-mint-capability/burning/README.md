# Burning the Aptos Coin Mint Capability
We begin with a brief section on the [procedure and intended outcomes](#procedure-and-intended-outcomes) of burning the Aptos Coin mint capability.

We then evaluate the means of [reversing this procedure](#reversing-the-procedure) and the potential [side effects](#side-effects) of doing so.

We then list all identified [call sites](#call-sites) using the Aptos Coin mint capability.

Finally, we the evaluate the following potential usages of the Aptos Coin mint capability as would affect critical system properties. These are:

1. **[Transaction `prologue` and `epilogue`](#transaction-epilogue-and-prologue):** we assert whether the removal of the mint capability would cause failures of the transaction `prologue` and `epilogue` and thus general transaction processing.
2. **[Token transfers](#token-transfers):** we assert whether the removal of the mint capability would cause failures of token transfers.
3. **[FA migration](#fa-migration):** we assert whether the removal of the mint capability would cause failures of FA migration.

## Procedure and intended outcomes
> [!WARNING]
> In general, a user who has access to the `core_resource_account` signer has the ability to make and publish changes to the framework which can remove restrictions on minting, recreate capabilities, etc. However, we maintain that under the Biarritz Model, these are a tolerable risk--particularly as structures and their storage are preserved by the Aptos Move VM inherently and thus can be restored. 
> 
> Our evaluation thus concerns effectively burning the mint capability in such a manner that a user who holds the `core_resource_account` signer would need to introduce a new framework to restore it--as opposed to simply running a series of transactions against existing code. This renders exploits costly, but not impossible.

### Framework changes
Because Aptos Move scripts cannot borrow structs but instead need to call `public` functions, we must update the framework to expose an `aptos_coin::destory_mint_capability_v2` which wraps the existing `public(friend) aptos_coin::destory_mint_capability`. The body of this function would be as follows:

```rust
public fun destroy_mint_capability_from(account: signer, from: account) acquires Delegations {
    system_addresses::assert_aptos_framework(aptos_framework);
    let MintCapStore { mint_cap } = move_from<MintCapStore>(from);
    coin::destroy_mint_cap(mint_cap);
}
```

Even though `delegate_mint_capability` does not assert that the `core_resource_account` still has the mint capability, when the capability is copied in the [`claim_mint_capability`](https://github.com/movementlabsxyz/aptos-core/blob/aa45303216be96ea30d361ab7eb2e95fb08c2dcb/aptos-move/framework/aptos-framework/sources/aptos_coin.move#L124) function, the borrow would fail. Thus, the resource account both can no longer use the mint capability and cannot delegate it to another account.

### Script
The script to burn the mint capability would then be as follows:

```rust
script {
    use aptos_framework::aptos_governance;
    use aptos_framework::aptos_coin;

    fun main(core_resources: &signer) {

        let core_signer = aptos_governance::get_signer_testnet_only(core_resources, @0x1);

        let framework_signer = &core_signer;

        // for core signer
		aptos_coin::destroy_mint_capability_from(framework_signer, @0x1);

        // for other signers
        aptos_governance::destroy_mint_capability_from(framework_signer, an_account);
        aptos_governance::destroy_mint_capability_from(framework_signer, another_account);

    }
}
```

## Reversing the procedure
To reverse the procedure, a user would need to introduce a new version of the framework which exposes a `create_mint_capability` function. This function would mimic the initialization procedure and look as follows:

```rust
public fun create_mint_capability_v2(account: signer, to: address) {
    system_addresses::assert_aptos_framework(aptos_framework);
    let mint_cap = coin::create_mint_cap();
    let mint_cap_store = MintCapStore { mint_cap };
    move_to<MintCapStore>(to, mint_cap_store);
}
```

In a subsequent script, the user would then call this function to recreate the mint capability and use it as needed.

## Call sites
Call sites for the Aptos Coin mint were identified in the follow ways:

1. **Relevant `.mint_cap` borrows:** we searched for borrows of the `AptosCoinCapabilites` struct that used the `mint_cap` member field. 
2. **`aptos_coin` internals:** We searched for direct usages of the `mint` capability struct within the `aptos_coin` module (this is the only place that struct can be used directly) and subsequent call sites.

### Relevant `.mint_cap` borrows
- **[`distribute_rewards`](https://github.com/movementlabsxyz/aptos-core/blob/aa45303216be96ea30d361ab7eb2e95fb08c2dcb/aptos-move/framework/aptos-framework/sources/stake.move#L1648):** is method used to issue rewards to validators from the reward aggregator pool. We assert that this is not called under the GGP feature flag in the block `epilogue` below. 
- **[`mint_and_refund`](https://github.com/movementlabsxyz/aptos-core/blob/aa45303216be96ea30d361ab7eb2e95fb08c2dcb/aptos-move/framework/aptos-framework/sources/transaction_fee.move#L268):** is a method used to mint and refund transaction fees. We assert that this is not called under the GGP feature flag in the block `epilogue` below.

### `aptos_coin` internals
We identified the following methods, all of which are used faucet or test branches of execution:

1. [`claim_mint_capability`](https://github.com/search?q=repo%3Amovementlabsxyz%2Faptos-core%20claim_mint_capability&type=code)
2. [`delegate_mint_capability`](https://github.com/search?q=repo%3Amovementlabsxyz%2Faptos-core+delegate_mint_capability&type=code)

## Usages and side effects

### Transaction `prologue` and `epilogue`

### `prologue`
There are distinct prologues for executing blocks and transactions within blocks in the Aptos Framework. 

#### [`block_prologue`](https://github.com/movementlabsxyz/aptos-core/blob/aa45303216be96ea30d361ab7eb2e95fb08c2dcb/aptos-move/framework/aptos-framework/sources/block.move#L224)

The [`block_prologue`](https://github.com/movementlabsxyz/aptos-core/blob/aa45303216be96ea30d361ab7eb2e95fb08c2dcb/aptos-move/framework/aptos-framework/sources/block.move#L224) and its DKG variant `block_prologue_ext` both primarily call to [`block_prologue_common`](https://github.com/movementlabsxyz/aptos-core/blob/aa45303216be96ea30d361ab7eb2e95fb08c2dcb/aptos-move/framework/aptos-framework/sources/block.move#L155).

We did not identify any usages of `.mint_cap` on any of the `block_prologue_common` branches.

#### Transaction [`prologue`]
All transactions now call one of the [`*_script_prologue`](https://github.com/movementlabsxyz/aptos-core/blob/aa45303216be96ea30d361ab7eb2e95fb08c2dcb/aptos-move/aptos-vm/src/aptos_vm.rs#L2244) functions, which in turn call [`prologue_common`](https://github.com/movementlabsxyz/aptos-core/blob/aa45303216be96ea30d361ab7eb2e95fb08c2dcb/aptos-move/framework/aptos-framework/sources/transaction_validation.move#L74).

We did not identify any usages of `.mint_cap` on any of the `prologue_common` branches.

### `epilogue`
There are distinct epilogues for executing blocks and transactions within blocks in the Aptos Framework.

#### Block epilogue
The Block Epilogue does not map neatly to a single function. Importantly, it can trigger `reconfiguration::reconfigure`(https://github.com/movementlabsxyz/aptos-core/blob/aa45303216be96ea30d361ab7eb2e95fb08c2dcb/aptos-move/framework/aptos-framework/sources/reconfiguration.move#L107) which calls the following coin invoking methods:

- `transaction_fee::process_collected_fees` which does not invoke any minting capabilities.
- `stake::on_new_epoch` which mints rewards for validators in `distribute_rewards`[https://github.com/movementlabsxyz/aptos-core/blob/aa45303216be96ea30d361ab7eb2e95fb08c2dcb/aptos-move/framework/aptos-framework/sources/stake.move#L1648].

To ensure the block epilogue does not `abort` on the `mint` branch, we would need to either set the reward rate to zero or add logic to skip the minting of rewards under a given feature flag.

#### Transaction [`epilogue`](https://github.com/movementlabsxyz/aptos-core/blob/aa45303216be96ea30d361ab7eb2e95fb08c2dcb/aptos-move/framework/aptos-framework/sources/transaction_validation.move#L262)

The transaction [`epilogue`](https://github.com/movementlabsxyz/aptos-core/blob/aa45303216be96ea30d361ab7eb2e95fb08c2dcb/aptos-move/framework/aptos-framework/sources/transaction_validation.move#L330) does make a call to `mint_and_refund` in the `mint` branch. However, this is [disabled](https://github.com/movementlabsxyz/aptos-core/blob/aa45303216be96ea30d361ab7eb2e95fb08c2dcb/aptos-move/framework/aptos-framework/sources/transaction_validation.move#L330) when the `ggp` feature flag is set.

### Token transfers
- [`coin::transfer`](https://github.com/movementlabsxyz/aptos-core/blob/aa45303216be96ea30d361ab7eb2e95fb08c2dcb/aptos-move/framework/aptos-framework/sources/coin.move#L1151) does not invoke the mint capability (note the lack of an `acquires MintCapability<CoinType>` in the function signature and the lack of an argument requesting the capability).
- [`fa::transfer`](https://github.com/movementlabsxyz/aptos-core/blob/aa45303216be96ea30d361ab7eb2e95fb08c2dcb/aptos-move/framework/aptos-framework/sources/fungible_asset.move#L655) does not invoke the mint capability (note the lack of an `acquires MintCapability<CoinType>` in the function signature and the lack of an argument requesting the capability).

### FA migration
We did not identify any usages of the mint capability on the original coin in the FA migration features, however, minting new representations coin balances is a core feature of the FA migration process.

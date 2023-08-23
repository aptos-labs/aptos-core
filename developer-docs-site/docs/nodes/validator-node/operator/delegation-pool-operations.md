---
title: "Delegation Pool Operations"
slug: "delegation-pool-operations"
---

# Delegation Pool Operations

> Beta: This documentation is in experimental, beta mode. Supply feedback by [requesting document changes](../../../community/site-updates.md#request-docs-changes). See also the related [Staking Pool Operations](./staking-pool-operations.md) instructions.

Validator operators should follow these instructions to carry out delegation pool operations for [staking](../../../concepts/staking.md). You may delegate as little as 10 APT plus a small add stake fee that will be mostly refunded as rewards at the end of the current 2-hour epoch. You might notice that some UIs might use 11 APT as the minimum for a round number. Note that your validator will become part of the *Active Validator Set* only when the delegation pool satisfies the minimum cumulative [staking requirement of 1 million APT](./staking-pool-operations.md).

Once the delegation pool attains 1 million APT, the pool's owner who initiates the delegation pool should set an operator for the pool via the `set_operator` function described in the [Perform pool owner operations](#perform-pool-owner-operations) section. The operator should then start their own Aptos node, as it is a best practice to have a different account for owner and operator. The operator should now [join in the active set of validators](./staking-pool-operations.md#joining-validator-set).

The operator address will receive the pool commission that was set at the initialization of the delegation pool, which is automatically distributed as stake in the delegation pool at the end of each epoch. The operator will act as a normal Delegation Pool account that is able to do all of the operations described in [Perform delegation pool operations](#perform-delegation-pool-operations).


## Prerequisites

1. [Install](../../../tools/aptos-cli/install-cli/index.md) and [configure](../../../tools/aptos-cli/use-cli/use-aptos-cli.md#configuration-examples) the Aptos CLI. If you are looking to develop on the Aptos blockchain, debug apps, or perform node operations, the Aptos tool offers a command line interface for these purposes.
2. [Initialize local configuration and create an account](../../../tools/aptos-cli/use-cli/use-aptos-cli.md#initialize-local-configuration-and-create-an-account) on the Aptos blockchain.

## Connect to Aptos network

To create a delegation pool and obtain information about it, [connect to the Aptos Network](./connect-to-aptos-network.md) and launch your own Aptos node.

You can use the following CLI commands to obtain the delegation pool address depending on where you are in the process:
- Before you create the delegation pool:  
    ```bash
    aptos account derive-resource-account-address --address <owner_address> --seed "aptos_framework::delegation_pool<SEED>" --seed-encoding utf8
    ```
  - The `<SEED>` is a number chosen by you to create the resource account address to host the delegation pool resource.  Once you choose a seed, you should use the same value for all following usages.
- After you create the delegation pool: 
    ```bash
    aptos account derive-resource-account-address
    ```

## Initialize a delegation pool

Now initialize a delegation pool by following these steps:

1. Run the command below, substitute in the profile you previously configured during initialization:
    ```bash
    aptos move run --profile <your-profile> \ 
    --function-id 0x1::delegation_pool::initialize_delegation_pool \
    --args u64:1000 string:00
    ```
    Where `--args`:
  
    - `u64:1000` represents `operator_commission_percentage` - 1000 is equivalent to 10% and 10000 is 100%.
    - `string:00` represents `delegation_pool_creation_seed` - a number chosen by you to create a resource account associated with your owner address; this account is used to host the delegation pool resource. You should use the same number here as the `--seed` you used in the previous step to create the delegation pool. 

    Note that once `operator_commission_percentage` is set, it cannot be changed.

 2. Once this command is executed without error an account for resources is established using the `owner` signer and a provided `delegation_pool_creation_seed` to hold the `delegation pool resource` and possess the underlying stake pool.
 
 3. The `owner` is granted authority over assigning the `operator` and `voter` roles, which are initially held by the `owner`.
 
 4. The delegation pool can now accept a minimum amount of 10 APT from any user who wishes to delegate to it.

## Perform delegation pool operations

This section describes the available operations that can be performed on this recently created pool. Once the delegation pool has been established, use the Aptos CLI to operate the pool. The available actions that can be performed on it include:

* Add `amount` of coins to the delegation pool `pool_address` using the public entry method `add_stake(delegator: &signer, pool_address: address, amount u64)` and substituting your values into the command below before running it:

  ```bash
  aptos move run --profile delegator \
  --function-id 0x1::delegation_pool::add_stake \ 
  --args address:<pool_address> u64:<amount>
  ```
  
* Undelegate (unlock) the amount of funds from the delegator's active and pending active stake up to the limit of the active stake in the stake pool using public entry method `unlock(delegator: &signer, pool_address: address, amount: u64)` and substituting your values into the command below before running it:

  ```bash
  aptos move run --profile delegator \
  --function-id 0x1::delegation_pool::unlock \ 
  --args address:<pool_address> u64:<amount>
  ```
  
* Cancel undelegate (reactivate stake) `amount` of coins from `pending_inactive` state to `active state` using public entry method `reactivate_stake(delegator: &signer, pool_address: address, amount: u64)` with the command and your values:

  ```bash
  aptos move run --profile delegator \
  --function-id 0x1::delegation_pool::reactivate_stake \
  --args address:<pool_address> u64:<amount>
  ```
 
* Withdraw `amount` of owned inactive stake from the delegation pool at `pool_address` using the public entry method ` withdraw(delegator: &signer, pool_address: address, amount: u64)` and the command:

  ```bash
  aptos move run --profile delegator \
  --function-id 0x1::delegation_pool::withdraw \
  --args address:<pool_address> u64:<amount>
  ```

## Perform pool owner operations
 
Delegation pool owners have access to specific methods designed for modifying the `operator` and `voter` roles of the delegation pool. Use the following Aptos CLI commands and include the relevant addresses:

* Set the operator address for the delegation pool:

  ```bash
  aptos move run --profile delegation_pool_owner \
  --function-id 0x1::delegation_pool::set_operator \
  --args address:<new_operator_address>
  ```

* Set the delegated voter address for the delegation pool:

  ```bash
  aptos move run --profile delegation_pool_owner \
  --function-id 0x1::delegation_pool::set_delegated_voter \
  --args address:<new_delegated_voter_address>
  ```
  
## Check delegation pool information

Until the delegation pool has received 1 million APT and the validator has been added to the set of active validators, there will be no rewards to track during each cycle. In order to obtain information about a delegation pool, use the Aptos [View functon](../../../integration/aptos-apis.md#reading-state-with-the-view-function).

* `get_owned_pool_address(owner: address): address` -  Returns the address of the delegation pool belonging to the owner, or produces an error if there is no delegation pool associated with the owner.

* `delegation_pool_exists(addr: address): bool` - Returns true if a delegation pool exists at the provided address `addr`.

* `operator_commission_percentage(pool_address: address): u64` - Returns the operator commission percentage set on the delegation pool at initialization.

* `get_stake(pool_address: address, delegator_address: address): (u64, u64, u64)` - Returns total stake owned by `delegator_address` within delegation pool `pool_address` in each of its individual states: (`active`,`inactive`,`pending_inactive`).

* `get_delegation_pool_stake(pool_address: address): (u64, u64, u64, u64)` - Returns the stake amounts on `pool_address` in the different states:      (`active`,`inactive`,`pending_active`,`pending_inactive`).

* `shareholders_count_active_pool(pool_address: address): u64` - Returns the number of delegators owning an active stake within `pool_address`. 

* `get_pending_withdrawal(pool_address: address, delegator_address: address): (bool, u64)` - Returns if the specified delegator possesses any withdrawable stake. However, if the delegator has recently initiated a request to release some of their stake and the stake pool's lockup cycle has not ended yet, then their funds may not yet be available for withdrawal.

* `can_withdraw_pending_inactive(pool_address: address): bool` - Returns whether `pending_inactive` stake can be directly withdrawn from the delegation pool, implicitly its stake pool, in the special case the validator had gone inactive before its lockup expired.


In the [Aptos TypeScript SDK](../../../sdks/ts-sdk/index.md), a View function request would resemble:

```bash
import {AptosClient, ViewRequest} from "aptos";

const NODE_URL = "https://aptos-testnet.public.blastapi.io";

(async () => {
    const client = new AptosClient(NODE_URL);
    const payload: ViewRequest = {
        function: "0x1::delagation_pool::get_stake",
        type_arguments: [],
        arguments: ["pool_address", "delegator_address"],
    };
    console.log(await client.view(payload));
})();

```
Alternatively, you can use Aptos CLI to call View functions. 

```bash
 aptos move view [OPTIONS] --function-id <FUNCTION_ID>
```

To discover the available options and the process for making an `aptos move view` call, access the help information with `aptos move view --help`. This will display the required arguments for invoking the view functions.


## Compute delegation pool rewards earned
Use this formula to calculate *rewards earned* for `active` and `pending_inactive` staking. This formula assumes that different stake operations such as `unlock` and `reactivate` take out the *principals* first and then *rewards*. Therefore, *rewards earned* may vary based upon how the formula you use is constructed:

1. Get the amount of `active` and `pending_inactive` staking from the [`get_stake`](https://github.com/aptos-labs/aptos-core/blob/ed63ab756cda61439287304ed89bbb156fcbeaed/aptos-move/framework/aptos-framework/sources/delegation_pool.move#L321) view function.

2. Calculate principal:
    - "active principal" = **AddStakeEvent** - **UnlockStakeEvent** + **ReactivateStakeEvent**. If at any point during the iteration, "active principal" < 0, reset to 0. Negative principal could happen when the amount users `unlock` include rewards earned from staking.
    - "pending inactive principal" = **UnlockStakeEvent** - **ReactivateStakeEvent**. If at any point during the iteration, "pending inactive principal" < 0, reset to 0. Negative principal could happen when the amount users `reactivate` include rewards earned from staking.

3. Compute rewards earned:
    - active_rewards = `active` - *active principal*.
    - pending_inactive_rewards = `pending_inactive` - "pending inactive principal".

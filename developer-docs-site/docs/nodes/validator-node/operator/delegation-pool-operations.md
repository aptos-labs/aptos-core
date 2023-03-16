---
title: "Delegation Pool Operations"
slug: "delegation-pool-operations"
---

# Delegation Pool Operations

> Beta: This documentation is in experimental, beta mode. Supply feedback by [requesting document changes](../../../community/site-updates.md#request-docs-changes).

Validator operators should follow these instructions to carry out delegation pool operations for [staking](../../../concepts/staking.md). You may delegate as little as 10 APT. Note that your validator will become part of the *Active Validator Set* only when the delegation pool satisfies the minimum cumulative [staking requirement of 1 million APT](./staking-pool-operations.md).

Once the delegation pool attains 1M APT, the pool's owner who initiates the delegation pool should set an operator for the pool via the `set_operator` function described in the [Pool owner operations](#pool-owner-operations) section. The operator should then start their own Aptos node, as it is a best practice to have a different account for owner and operator. The operator should now [join in the active set of validators](./staking-pool-operations/#joining-validator-set).

The operator address will receive the pool commission that was set at the initialization of the delegation pool and will act as a normal Delegation Pool account that is able to do all of the operations described in [Delegation pool operations](#delegation-pool-operations).


## Prerequisites

[Install](../../../cli-tools/aptos-cli-tool/index.md) and [use the Aptos CLI](../../../cli-tools/aptos-cli-tool/use-aptos-cli.md). If you are looking to develop on the Aptos blockchain, debug apps, or perform node operations, the Aptos tool offers a command line interface for these purposes.

## Initialize local configuration and create an account

Follow the steps below to [start Aptos and create an account](../../../guides/get-test-funds.md) on the blockchain. Once done, you will have a local `.aptos/` directory containing a `config.yaml` configuration file that is used to store configurations between CLI runs. This is local to your run, so you will need to continue running the Aptos CLI from this directory or reinitialize in another directory.

### Step 1: Run Aptos init

The `aptos init` command initializes the configuration with the private key you provide or generates anew if not given:

```bash
$ aptos init
Configuring for profile default
Enter your rest endpoint [Current: None | No input: https://fullnode.devnet.aptoslabs.com]

No rest url given, using https://fullnode.devnet.aptoslabs.com...
Enter your faucet endpoint [Current: None | No input: https://faucet.devnet.aptoslabs.com]

No faucet url given, using https://faucet.devnet.aptoslabs.com...
Enter your private key as a hex literal (0x...) [Current: None | No input: Generate new key (or keep one if present)]

No key given, generating key...
Account 00f1f20ddd0b0dd2291b6e42c97274668c479bca70f07c6b6a80b99720779696 does not exist, creating it and funding it with 10000 coins
Aptos is now set up for account 00f1f20ddd0b0dd2291b6e42c97274668c479bca70f07c6b6a80b99720779696!  Run `aptos help` for more information about commands

{
  "Result": "Success"
}
  ```
### Stept 2: Create other profiles

You can also create other profiles for different endpoints and different keys. Do this by including the `--profile` argument and the value of your choosing to the `aptos init` command as shown below. Note that argument can be used in most other commands to replace command line arguments.

```bash
$ aptos init --profile superuser
Configuring for profile superuser
Enter your rest endpoint [Current: None | No input: https://fullnode.devnet.aptoslabs.com]

No rest url given, using https://fullnode.devnet.aptoslabs.com...
Enter your faucet endpoint [Current: None | No input: https://faucet.devnet.aptoslabs.com]

No faucet url given, using https://faucet.devnet.aptoslabs.com...
Enter your private key as a hex literal (0x...) [Current: None | No input: Generate new key (or keep one if present)]

No key given, generating key...
Account 18B61497FD290B02BB0751F44381CADA1657C2B3AA6194A00D9BC9A85FAD3B04 does not exist, creating it and funding it with 10000 coins
Aptos is now set up for account 18B61497FD290B02BB0751F44381CADA1657C2B3AA6194A00D9BC9A85FAD3B04!  Run `aptos help` for more information about commands
{
  "Result": "Success"
}
```

### Stept 3: Connect to Aptos network

To create a delegation pool and obtain information about it, connect to the [Aptos Network](./connect-to-aptos-network.md) and launch your own Aptos node.

## Initialize a delegation pool

Now initialize a delegation pool by following these steps:

1. Run the command below, substituting in your previously configured profile:

  ```bash
  aptos move run --profile ‘your_profile’ \ 
  --function-id 0x1::delegation_pool::initialize_delegation_pool \
  --args u64:1000 raw:00
  ```

  Where `--args`:

  - `u64:1000` represents `operator_commission_percentage`
  - `raw: 00` represents `delegation_pool_creation_seed`

  Note that once `operator_commission_percentage` is set, it cannot be changed.

 2. Once this command is executed without error an account for resources is established using the `owner` signer and a provided `delegation_pool_creation_seed` to hold the `delegation pool resource` and possess the underlying stake pool.
 
 3. The `owner` is granted authority over assigning the `operator` and `voter` roles, which are initially held by the `owner`.
 
 4. The delegation pool can now accept a minimum amount of 10 APT from any user who wishes to delegate to it.

## Delegation pool operations

This section describes the available operations that can be performed on this recently created pool. Once the delegation pool has been established, use the Aptos CLI to operate the pool. The available actions that can be performed on it include:

* Add `amount` of coins to the delegation pool `pool_address` using the public entry method `add_stake(delegator: &signer, pool_address: address, amount u64)` and substituting your values into the command below before running it:

  ```bash
  aptos move run --profile ‘delegator’ \
  --function-id 0x1::delegation_pool::add_stake \ 
  --args address: pool_address u64: ‘amount’
  ```
  
* Undelegate (unlock) the amount of funds from the delegator's active and pending active stake up to the limit of the active stake in the stake pool using public entry method `unlock(delegator: &signer, pool_address: address, amount: u64)` and substituting your values into the command below before running it:

  ```bash
  aptos move run --profile ‘delegator’ \
  --function-id 0x1::delegation_pool::unlock \ 
  --args address:’pool_address’ u64:’amount’
  ```
  
* Cancel undelegate (reactivate stake) `amount` of coins from `pending_inactive` state to `active state` using public entry method `reactivate_stake(delegator: &signer, pool_address: address, amount: u64)` with the command and your values:

  ```bash
  aptos move run --profile ‘delegator’ \
  --function-id 0x1::delegation_pool::reactivate_stake \
  --args address:’pool_address’ u64:’amount’
  ```
 
* Withdraw `amount` of owned inactive stake from the delegation pool at `pool_address` using the public entry method ` withdraw(delegator: &signer, pool_address: address, amount: u64)` and the command:

  ```bash
  aptos move run --profile ‘delegator’ \
  --function-id 0x1::delegation_pool::withdraw \
  --args address:’pool_address’ u64:’amount’
  ```

## Pool owner operations
 
Delegation pool owners have access to specific methods designed for modifying the `operator` and `voter` roles of the delegation pool. Use the following Aptos CLI commands and include the relevant addresses:

  ```bash
  aptos move run --profile ‘delegation_pool_owner’ \
  --function-id 0x1::delegation_pool::set_operator \
  --args address:’new_operator_address’
  ```
  
  ```bash
 aptos move run --profile ‘delegation_pool_owner’ \
 --function-id 0x1::delegation_pool::set_delegated_voter \
 --args address:’new_delegated_voter_address’
  ```
  
## Check delegation pool information

Until the delegation pool has received 1M APT and the validator has been added to the set of active validators, there will be no rewards to track during each cycle. In order to obtain information about a delegation pool, use the Aptos [View functon](../../../guides/aptos-apis.md#reading-state-with-the-view-function).

1. `get_owned_pool_address(owner: address): address` -  Returns the address of the delegation pool belonging to the owner, or produces an error if there is no delegation pool associated with the owner.

2. `delegation_pool_exists(addr: address): bool` - Returns if a delegation pool exists at the provided address `addr`.

3. `operator_commission_percentage(pool_address: address): u64` - Returns the operator commission percentage set on the delegation pool at initialization.

4. `get_stake(pool_address: address, delegator_address: address): (u64, u64, u64)` - Returns total stake owned by `delegator_address` within delegation pool `pool_address` in each of its individual states: (`active`,`inactive`,`pending_inactive`).

5. `get_delegation_pool_stake(pool_address: address): (u64, u64, u64, u64)` - Returns the stake amounts on `pool_address` in the different states:      (`active`,`inactive`,`pending_active`,`pending_inactive`).

6. `shareholders_count_active_pool(pool_address: address): u64` - Returns the number of delegators owning an active stake within `pool_address`. 

7. `get_pending_withdrawal(pool_address: address, delegator_address: address): (bool, u64)` - Returns if the specified delegator possesses any withdrawable stake. However, if the delegator has recently initiated a request to release some of their stake and the stake pool's lockup cycle has not ended yet, then their funds may not yet be available for withdrawal.

8. `can_withdraw_pending_inactive(pool_address: address): bool` - Returns whether `pending_inactive` stake can be directly withdrawn from the delegation pool, implicitly its stake pool, in the special case the validator had gone inactive before its lockup expired.


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
Alternatively you can use Aptos CLI to call view functions. 

```bash
 aptos move view [OPTIONS] --function-id <FUNCTION_ID>

```

To discover the available options and the process for making an aptos move call, you can use the command aptos move view --help. This will display the required arguments for invoking the view functions.


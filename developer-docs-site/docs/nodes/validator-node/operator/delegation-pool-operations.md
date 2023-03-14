---
title: "Delegation Pool Operations"
slug: "delegation-pool-operations"
---

# Delegation Pool Operations

This document provides instructions on how to carry out delegation pool operations. It is important to note that while you can delegate as little as 10 APT. Note that your validator will only become part of the "Active Validator Set" when the delegation pool satisfies the minimum cumulative staking requirement of 1M APT.

Once the delegation pool attains 1M APT, the pool's owner can set an operator for the pool (via the set_operator operation described in Pool Owner Operations). The operator can then spin up their own Aptos node (it is a best practice to have a different account for owner and operator) and can now [join in the active set of validators.](https://aptos.dev/nodes/validator-node/operator/staking-pool-operations/#joining-validator-set)

The operator address will receive the pool commission that was set at the initialization of the delegation pool, and will act as a normal Delegation Pool account, being able to do all the operations described at Delegation pool operations.

It is not mandatory to operate an Aptos node personally to become a part of the active validators set. Instead, one can designate an individual with an Aptos node as the operator and associate the pool with their node. By doing so, the new operator will be able to successfully join the active validators set.

## Prerequisites:

[Aptos CLI](https://aptos.dev/cli-tools/aptos-cli-tool/use-aptos-cli/) : If you're looking to develop on the Aptos blockchain, debug, or perform node operations, the Aptos tool offers a command line interface (CLI) for these purposes. To obtain the CLI, you can either download it or build it by following the instructions provided in the [Install Aptos CLI](https://aptos.dev/cli-tools/aptos-cli-tool/) section.


## Initialize local configuration and create an account

A local folder named .aptos/ will be created with a configuration config.yaml which can be used to store configuration between CLI runs. This is local to your run, so you will need to continue running CLI from this folder, or reinitialize in another folder.

#### Step 1: Run Aptos init

The aptos init command will initialize the configuration with the private key you provided.

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
#### Stept 2: Creating other profiles
You can also create other profiles for different endpoints and different keys. These can be made by adding the --profile argument, and can be used in most other commands to replace command line arguments.

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

To create a delegation pool and obtain information about it, you must connect to the  [Aptos Network](https://aptos.dev/nodes/validator-node/operator/connect-to-aptos-network/)and launch your own Aptos node.

## Initialize a Delegation Pool
1. Run the following command  [using the Aptos CLI](https://aptos.dev/cli-tools/aptos-cli-tool/use-aptos-cli/):

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
.

Below are the details of the available operations that can be performed on this recently created pool.

## Delegation pool operations with CLI

Once the delegation pool has been established, the available actions that can be performed on it include:

1. Add `amount` of coins to the delegation pool `pool_address` using public entry method - `add_stake(delegator: &signer, pool_address: address, amount: u64)`.
 
 The CLI command is:

  ```bash
  aptos move run --profile ‘delegator’ \
  --function-id 0x1::delegation_pool::add_stake \ 
  --args address: pool_address u64: ‘amount’
  ```
  
2. Undelegate(Unlock) the amount of funds from the delegator's active and pending active stake, up to the limit of the active stake in the stake pool using public entry method - 
`unlock(delegator: &signer, pool_address: address, amount: u64)`.

 The CLI command is:

  ```bash
  aptos move run --profile ‘delegator’ \
  --function-id 0x1::delegation_pool::unlock \ 
  --args address:’pool_address’ u64:’amount’
  ```
  
3. Cancel undelegate (reactivate stake) `amount` of coins from pending_inactive state to active state using public entry method - `reactivate_stake(delegator: &signer, pool_address: address, amount: u64)`

 The CLI command is:

  ```bash
  aptos move run --profile ‘delegator’ \
  --function-id 0x1::delegation_pool::reactivate_stake \
  --args address:’pool_address’ u64:’amount’
  ```
 
4. Withdraw `amount` of owned inactive stake from the delegation pool at `pool_address` using public entry method - ` withdraw(delegator: &signer, pool_address: address, amount: u64)`

 The CLI command is:

  ```bash
  aptos move run --profile ‘delegator’ \
  --function-id 0x1::delegation_pool::withdraw \
  --args address:’pool_address’ u64:’amount’
  ```

## Pool Owner Operations
 
Delegation pool owners have access to specific methods designed for modifying the `operator` and `voter` roles of the delegation pool. The CLI commands to do this are:

  ```bash
  aptos move run --profile ‘delagation_pool_owner’ \
  --function-id 0x1::delegation_pool::set_operator \
  --args address:’new_operator_address’
  ```
  
  ```bash
 aptos move run --profile ‘delagation_pool_owner’ \
 --function-id 0x1::delegation_pool::set_delegated_voter \
 --args address:’new_delegated_voter_address’
  ```
  
  ## Checking delegation pool information

In order to obtain information about a delegation pool, there are view methods available that can be used to retrieve the necessary details. 

Until the delegation pool has received 1M APT and the validator has been added to the set of active validators, there will be no rewards to track during each cycle. 

These methods can be invoked to read the blockchain's state. For additional details on how to use view methods, please refer to the [Reading state with the View function](https://aptos.dev/guides/aptos-api/#reading-state-with-the-view-function) documentation. 

1. `get_owned_pool_address(owner: address): address` -  Returns the address of the delegation pool belonging to the owner, or produces an error if there is no delegation pool associated with the owner.

2. `delegation_pool_exists(addr: address): bool` - Returns if a delegation pool exists at the provided address `addr`.

3. `operator_commission_percentage(pool_address: address): u64` - This method returns the operator commission percentage set on the delegation pool at initialization.

4. `get_stake(pool_address: address, delegator_address: address): (u64, u64, u64)` - Returns total stake owned by `delegator_address` within delegation pool `pool_address` in each of its individual states: (`active`,`inactive`,`pending_inactive`).

5. `get_delegation_pool_stake(pool_address: address): (u64, u64, u64, u64)` - Returns the stake amounts on `pool_address` in the different states:      (`active`,`inactive`,`pending_active`,`pending_inactive`)

6. `shareholders_count_active_pool(pool_address: address): u64` - Returns the number of delegators owning an active stake within `pool_address`. 

7. `get_pending_withdrawal(pool_address: address, delegator_address: address): (bool, u64)` - Returns if the specified delegator possesses any withdrawable stake. However, if the delegator has recently initiated a request to release some of their stake and the stake pool's lockup cycle has not ended yet, then their funds may not yet be available for withdrawal.

8. `can_withdraw_pending_inactive(pool_address: address): bool` - Returns whether `pending_inactive` stake can be directly withdrawn from the delegation pool, implicitly its stake pool, in the special case the validator had gone inactive before its lockup expired


In the TypeScript SDK, a view function request would look like this:

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

Another important thing about ‘view methods’ is that you need to pass ‘--bytecode-version 6’ to the [Aptos CLI](https://aptos.dev/cli-tools/aptos-cli-tool/use-aptos-cli/) when publishing the module.

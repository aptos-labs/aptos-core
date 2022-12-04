---
title: "Gas and Transaction Fees"
slug: "gas-txn-fee"
---

# Gas and Transaction Fees

To conduct any transaction with the Aptos blockchain on the mainnet, you are required to pay a processing fee. This fee is derived from transactions with a client application, stake owner, node operator, or voter. The processing fee you are required to pay is based on the computing and storage resources you use on the blockchain to:

1. Process your transactions on the blockchain.
1. Propagate the validated record throughout the distributed network of the mainnet.
1. Store the validated record in the distributed blockchain storage.  

:::tip 
Conceptually, this fee can be thought of as quite similar to how we pay for our home electric or water utilities.
:::
 
## Unit of gas

Transactions can range from simple and inexpensive to complicated based upon the amount of computation and fetches from and writes to storage. In the Aptos blockchain, a **unit of gas** represents a basic unit of resource consumption for both

- Computation resource, and
- Storage resource.

See [How Base Gas Works](./base-gas.md) for a detailed description of gas fee types and available optimizations.

## Gas price and prioritizing transactions

In the Aptos network, the Aptos governance sets the minimum gas unit price. However, the market determines the actual minimum gas unit price. See [Ethereum Gas Tracker](https://etherscan.io/gastracker), for example, which shows the market price movements of Ethereum gas price.

By specifying a higher gas unit price than the current market price, you can **increase** the priority level for your transaction on the blockchain by paying. While in most cases, this is unnecessary, if the network is under load, this can ensure that your transaction can be processed more quickly.

:::tip Unit of gas
👉 A **unit of gas** is a dimensionless number, expressed as an integer. The total gas units consumed by your transaction depends on the complexity of your transaction. The **gas price**, on the other hand, is expressed in terms of Aptos blockchain’s native coin (Octas). Also see [Transactions and States](/concepts/txns-states) for how a transaction submitted to the Aptos blockchain looks like.
:::

## Specifying gas fees within a transaction

When a transaction is submitted to the Aptos blockchain, the transaction must contain the following mandatory gas fields:

- `max_gas_amount`: The maximum number of gas units that the transaction sender is willing to spend to execute the transaction. This determines the maximum computational resources that can be consumed by the transaction.
- `gas_price`: The gas price the transaction sender is willing to pay. It is expressed in Octa units, where:
    - 1 Octa = 10<sup>-8</sup> APT and 
    - APT is the Aptos coin. 
  
  During the transaction execution, the total gas amount, expressed as:
  ```
  (total gas units consumed) * (gas_price)
  ```
  must not exceed `max_gas_amount`, or else the transaction will abort the execution.

The transaction fee charged to the client will be at the most `gas_price * max_gas_amount`.

## Gas parameters set by governance

The following gas parameters are set by Aptos governance. 

:::tip On-chain gas schedule
These on-chain gas parameters are published on the Aptos blockchain at `0x1::gas_schedule::GasScheduleV2`.
:::

- `txn.maximum_number_of_gas_units`: Maximum number of gas units that can be spent (this is the maximum allowed value for the `max_gas_amount` gas parameter in the transaction). This is to ensure that the dynamic pricing adjustments do not exceed how much you are willing to pay in total.
- `txn.min_transaction_gas_units:` Minimum number of gas units that can be spent. The `max_gas_amount` value in the transaction must be set to greater than this parameter’s value. 

## Dynamic gas pricing for storage

Aptos gas pricing uses dynamic prices for the storage operations. This means that the storage costs, hence the `gas_used`, can increase exponentially as the Aptos blockchain state database is filled up. The storage cost can become as high as `100x` at 100% utilization. However, the expectation is that the validators will make use of larger and cheaper storage hardware to mitigate such exponential rise in the storage costs. 

Dynamic pricing is used to protect the Aptos network in the worse-case scenarios. However, we expect upgrades to Aptos network to happen well before the network gets into the high-cost zones.

## Examples

### Example 1: Account balance vs transaction fee

**The sender’s account must have sufficient funds to pay for the transaction fee.**

If, let's say, you transfer all the money out of your account so that you have no remaining balance to pay for the transaction fee. In such case the Aptos blockchain would let you know that the transaction will fail, and your transfer wouldn't succeed either.

### Example 2: Transaction amounts vs transaction fee

**Transaction fee is independent of transfer amounts in the transaction.**

In a transaction, for example, transaction A, you are transferring 1000 coins from one account to another account. In a second transaction B, with the same gas field values of transaction A, you now transfer 100,000 coins from one account to another one account. Assuming that both the transactions A and B are sent roughly at the same time, then the gas costs for transactions A and B would be near-identical.

## Estimating the gas units via simulation

The gas used for a transaction can be estimated by simulating the transaction. When simulating the transaction, the simulation results represent the **exact** amount that is needed at the **exact** state of the blockchain at the time of simulation. These gas units used may change based on the state of the chain.  For this reason, any amount coming out of the simulation is only an estimate, and when setting the max gas amount, it should include an appropriate amount of headroom based upon your comfort-level and historical behaviors. Setting the max gas amount too low will result in the transaction aborting and the account being charged for whatever gas was consumed.

Transactions can be simulated with the [`SimulateTransaction`](https://fullnode.devnet.aptoslabs.com/v1/spec#/operations/simulate_transaction) API. This API will run the exact transaction that you plan to run.  

:::tip
Note that the `Signature` provided on the transaction must be all zeros. This is to prevent someone from using the valid signature.
:::

To simulate the transaction, there are two flags:

1. `estimate_gas_unit_price`: This flag will estimate the gas unit price in the transaction using the same algorithm as the [`estimate_gas_price`](https://fullnode.devnet.aptoslabs.com/v1/spec#/operations/estimate_gas_price) API.
2. `estimate_max_gas_amount`: This flag will find the maximum possible gas you can use, and it will simulate the transaction to tell you the actual `gas_used`.

### Simulation steps

The simulation steps for finding the correct amount of gas for a transaction are as follows:

1. Estimate the gas via simulation with both `estimate_gas_unit_price` and `estimate_max_gas_amount` set to `true`.
2. Use the `gas_unit_price` in the returned transaction as your new transaction’s `gas_unit_price`.
3. View the `gas_used * gas_unit_price` values in the returned transaction as the **lower bound** for the cost of the transaction.
4. To calculate the upper bound of the cost, take the **minimum** of the `max_gas_amount` in the returned transaction, and the `gas_used * safety factor`. In the CLI a value of `1.5` is used for `safety factor`. Use this value as `max_gas_amount` for the transaction you want to submit. Note that the **upper bound** for the cost of the transaction is `max_gas_amount * gas_unit_price`, i.e., this is the most the sender of the transaction is charged.
5. At this point you now have your `gas_unit_price` and `max_gas_amount` to submit your transaction as follows:
    1. `gas_unit_price` from the returned simulated transaction.
    2. `max_gas_amount` as the minimum of the `gas_used` * `a safety factor` or the `max_gas_amount` from the transaction.
6. If you feel the need to prioritize or deprioritize your transaction, adjust the `gas_unit_price` of the transaction. Increase the value for higher priority, and decrease the value for lower priority.

::: tip
Prioritization is based upon buckets of `gas_unit_price`. The buckets are defined [here](https://github.com/aptos-labs/aptos-core/blob/30b385bf38d3dc8c4e8ee0ff045bc5d0d2f67a85/config/src/config/mempool_config.rs#L8). The current buckets are `[0, 150, 300, 500, 1000, 3000, 5000, 10000, 100000, 1000000]`. Therefore a `gas_unit_price` of 150 and 299 would be priortized nearly the same.
:::

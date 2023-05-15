---
title: "Gas and Storage Fees"
slug: "gas-txn-fee"
---

# Gas and Storage Fees

Any transaction execution on the Aptos blockchain requires a processing fee. As of today, this fee comprises two components:
1. Execution & IO costs
  - This covers your usage of transient computation resources, such as processing your transactions and propagating the validated record throughout the distributed network of the mainnet.
  - It is measured in Gas Units whose price may fluctuate according to the load of the network. This allows execution & io costs to be low when the network is less busy.
2. Storage fees
  - This covers the cost to persistently store validated record in the distributed blockchain storage.
  - It is measured in fixed APT prices.
  - In the future, such fees will evolve into deposits that are refundable, should you choose to delete the data you stored. The refunds can be full or partial, depending on the time frame.

:::tip
Conceptually, this fee can be thought of as quite similar to how we pay for our home electric or water utilities.
:::

## Unit of gas

Transactions can range from simple and inexpensive to complicated based upon what they do. In the Aptos blockchain, a **unit of gas** represents a basic unit of consumption for transient resources, such as doing computation or accessing the storage. The latter should not be conflated with the long-term storage aspect of such operations, as that is covered by the storage fees separately.

See [How Base Gas Works](./base-gas.md) for a detailed description of gas fee types and available optimizations.

:::tip Unit of gas
👉 A **unit of gas** is a dimensionless number or a unit that is not associated with any one item such as a coin, expressed as an integer. The total gas units consumed by your transaction depend on the complexity of your transaction. The **gas price**, on the other hand, is expressed in terms of Aptos blockchain’s native coin (Octas). Also see [Transactions and States](txns-states.md) for how a transaction submitted to the Aptos blockchain looks like.
:::

## Gas price and prioritizing transactions

In the Aptos network, the Aptos governance sets the absolute minimum gas unit price. However, the market determines how quickly a transaction with a particular gas unit price is processed. See [Ethereum Gas Tracker](https://etherscan.io/gastracker), for example, which shows the market price movements of Ethereum gas price.

By specifying a higher gas unit price than the current market price, you can **increase** the priority level for your transaction on the blockchain by paying a larger processing fee. As part of consensus, when the leader selects transactions from its mempool to propose as part of the next block, it will prioritize selecting transactions with a higher gas unit price. While in most cases this is unnecessary, if the network is under load this measure can ensure your transaction is processed more quickly. See the `gas_unit_price` entry under [Estimating the gas units via simulation](#estimating-the-gas-units-via-simulation) for details.

:::caution Increasing gas unit price with in-flight transactions
👉 If you are increasing gas unit price, but have in-flight (uncommitted) transactions for the same account, you should resubmit all of those transactions with the higher gas unit price. This is because transactions within the same account always have to respect sequence number, so effectively the higher gas unit price transaction will increase priority only after the in-flight transactions are included in a block.
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
- `txn.min_transaction_gas_units`: Minimum number of gas units that can be spent. The `max_gas_amount` value in the transaction must be set to greater than this parameter’s value.

There also exists some global per-category limits:
- `txn.max_execution_gas`: The maximum number of gas units a transaction can spend on execution.
- `txn.max_io_gas`: The maximum number of gas units a transaction can spend on IO.
- `txn.max_storage_fee`: The maximum amount of APT a transaction can spend on persistent storage.
These limits help decouple one category from another, allowing us to set `txn.maximum_number_of_gas_units` generously without having to worry about abuses.

##  Calculating Storage Fees

The storage fee for a transaction is calculated based on the following factors:
1. The size of the transaction itself
2. The number of new storage slots used and bytes written
3. The events emitted
For details, see [How Base Gas Works](./base-gas.md).

It should be noted that due to some backward compatibility reasons, the total storage fee of a transaction is currently presented to the client as part of the total `gas_used`. This means, this amount could vary based on the gas unit price even for the same transaction.

Here is an example. Suppose we have a transaction that costs `100` gas units in execution & IO, and `5000` Octa in storage fees. The network will show that you have used
- `100 + 5000 / 100 = 150` gas units if the gas unit price is `100`, or
- `100 + 5000 / 200 = 125` gas units if the unit price is `200`.

We are aware of the confusion this might create, and plan to present these as separate items in the future. However this will require some changes to the transaction output format and downstream clients, so please be patient while we work hard to make this happen.

## Examples

### Example 1: Account balance vs transaction fee

**The sender’s account must have sufficient funds to pay for the transaction fee.**

If, let's say, you transfer all the money out of your account so that you have no remaining balance to pay for the transaction fee. In such case the Aptos blockchain would let you know that the transaction will fail, and your transfer wouldn't succeed either.

### Example 2: Transaction amounts vs transaction fee

**Transaction fee is independent of transfer amounts in the transaction.**

In a transaction, for example, transaction A, you are transferring 1000 coins from one account to another account. In a second transaction B, with the same gas field values of transaction A, you now transfer 100,000 coins from one account to another one account. Assuming that both the transactions A and B are sent roughly at the same time, then the gas costs for transactions A and B would be near-identical.

## Estimating gas consumption via simulation

The gas used for a transaction can be estimated by simulating the transaction on chain as described here or locally via the [gas profiling](../move/move-on-aptos/cli/#profiling-gas-usage) feature of the Aptos CLI. The results of the simulated transaction represent the **exact** amount that is needed at the **exact** state of the blockchain at the time of the simulation. These gas units used may change based on the state of the chain.  For this reason, any amount coming out of the simulation is only an estimate, and when setting the max gas amount, it should include an appropriate amount of headroom based upon your comfort-level and historical behaviors. Setting the max gas amount too low will result in the transaction aborting and the account being charged for whatever gas was consumed.

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

:::tip
Prioritization is based upon buckets of `gas_unit_price`. The buckets are defined in [`mempool_config.rs`](https://github.com/aptos-labs/aptos-core/blob/30b385bf38d3dc8c4e8ee0ff045bc5d0d2f67a85/config/src/config/mempool_config.rs#L8). The current buckets are `[0, 150, 300, 500, 1000, 3000, 5000, 10000, 100000, 1000000]`. Therefore, a `gas_unit_price` of 150 and 299 would be prioritized nearly the same.
:::

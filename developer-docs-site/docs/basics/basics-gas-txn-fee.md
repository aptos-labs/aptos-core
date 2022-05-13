---
title: "Gas and transaction fees"
slug: "basics-gas-txn-fee"
---
import BlockQuote from "@site/src/components/BlockQuote";

When a transaction is executed on the Aptos Blockchain, resource usage is tracked and measured in gas.

# Introduction

Gas ensures that all Move programs running on the Aptos Blockchain eventually terminate. This bounds the computational resources that are used. Gas also provides the ability to charge a transaction fee, partly based on consumed resources during execution.

When a client submits a transaction for execution to the Aptos Blockchain, it contains a specified:

* `max_gas_amount`: The maximum amount of gas units that can be used to execute the transaction. This bounds the computational resources that can be consumed by the transaction.
* `gas_price`: The gas price in the blockchain's utility token. Gas price is a way to translate from gas units (the abstract units of resources consumed by the virtual machine) to a transaction fee in the blockchain's utility token.

The transaction fee charged to the client will be at most `gas_price * max_gas_amount`.

The gas price, and hence the transaction fee, should follow the market characteristics of the Aptos Blockchain as resource supply and demand fluctuate.

## Types of resource usage

For the virtual machine (VM) to execute a transaction, the gas system needs to track the primary resources used by the transaction. These fall into three resource dimensions:

1. The computational cost of executing the transaction.
2. The network cost of propagating the transaction through the Aptos ecosystem.
3. The storage cost of data created and used during transaction execution on the Aptos Blockchain.

The first two of these resources (computational and network) are ephemeral. However, the third (storage), is long lived. Once data is allocated, that data persists until it is deleted. In the case of accounts, the data lives indefinitely.

Each of these resource dimensions can fluctuate independently. However, there is also only one gas price. As a result, it means that each dimension's gas usage must be tracked correctly because the gas price only acts as a single multiplier to the total gas usage. Thus, the gas usage of a transaction needs to be closely correlated with the real-world cost associated with executing the transaction.

## Using gas to compute transaction fees

When you send a transaction, the transaction fee for execution is the gas price multiplied by the VM's computed resource usage for that transaction.

At different times in the transaction flow, different aspects of resource usage are charged. The basics of the transaction flow and the gas-related logic are detailed in the following diagram:
![FIGURE 1.0 Gas and Transaction Flow](/img/docs/using-gas.svg)
<small className="figure">FIGURE 1.0 Gas and Transaction Flow</small>

In the diagram, both the prologue and epilogue sections are marked in the same color. This is because these sections of the transaction flow need to be **unmetered**:
* In the prologue, it's not known if the submitting account has sufficient funds to cover its gas liability, or if the user submitting the transaction even has authority over the submitting account. Due to this lack of knowledge, when the prologue is executed, it needs to be unmetered. Deducting gas for transactions that fail the prologue could allow unauthorized deductions from accounts.
* The epilogue is in part responsible for debiting the execution fee from the submitting account and distributing it. Because of this, the epilogue must run even if the transaction execution has run out of gas. Likewise, we don't want it to run out of gas while debiting the submitting account as this would cause additional computation to be performed without any transaction fee being charged.

This means that the minimum transaction fee, `MIN_TXN_FEE`, needs to be enough to cover the average cost of running the prologue and epilogue.

After the prologue has run, and we've checked in part that the account can cover its gas liability, the rest of the transaction flow starts with the "gas tank" full at `max_gas_amount`. The `MIN_TXN_FEE` is charged, after which the gas tank is then charged for each instruction the VM executes. This per-instruction deduction continues until either:
* The transaction execution is complete, after which the cost of storing the transaction data is charged, and the epilogue is run and the execution fee deducted, or
* The "gas tank" becomes empty, in which case an `OutOfGas` error is raised.

In the former, the fee is collected and the result of the transaction is persisted on the Aptos Blockchain. In the latter, the execution of the transaction stops when the error is raised. After which, the total gas liability of the transaction is collected. No other remnants of the execution are committed other than the deduction in this case.

## Using gas to prioritize a transaction

When you send a transaction, it is prioritized based on different criteria. One of these is the normalized gas price for the transaction.

For example:

* Bob sends a transaction with `gas_price` 10.
* Alice sends a transaction at the same time with `gas_price` 20.

Alice's transaction would be ranked higher than Bob's.

## Core design principles
Three central principles have motivated the design of gas in Aptos and Move:

| Design Principle | Description |
| ---------- | ---------- |
| Move is Turing complete | Because of this, determining if a given Move program terminates cannot be decided statically. However, by ensuring that <br/>  - every bytecode instruction has a non-zero gas consumption, and <br/>  - the amount of gas that any program can be started with is bounded, <br/>  we get this termination property for programs almost free of cost. |
| Discourage DDoS attacks and encourage judicious use of the network | The gas usage for a transaction is correlated with resource consumption of that transaction. The gas price, and hence the transaction fee, should rise-and-fall with contention in the network. At launch, we expect gas prices to be at or near zero. But in periods of high contention, you can prioritize transactions using the gas price, which will encourage sending only vital transactions during such times. |
| The resource usage of a program needs to be agreed upon in consensus | This means that the method of accounting for resource consumption needs to be deterministic. This rules out other means of tracking resource usage, such as cycle counters or any type of timing-based methods as they are not guaranteed to be deterministic across nodes. The method for tracking resource usage needs to be abstract. |

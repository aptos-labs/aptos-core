---
title: "Gas"
id: "gas"
---

# Gas
Blockchain can be thought of as infrastructure provided by the validator set for developers to build decentralized applications
and for users to use them without being worried about a centralized power extract profits from their data and assets. Since the
network is a limited resource (operational costs, network bandwidth, storage, etc.), all participants need to pay gas when
submitting transactions. A portion of the fees will be given to the validators to compensate for operational costs while the
rest will be burnt.

## Transaction and Gas
When sending a transaction, the users, through wallets, need to set a few parameters to make sure they pay the gas cost correctly.
The total gas paid is made of two main components:
1. Gas fee: The amount of gas units needed to execute the code that the transaction is calling. This is generally static for
the same function called and only varies if the state being updated (e.g. [account resources](./resources.md)) changes.
2. Cost per gas unit: The network can be thought of as a free market. As usage increases, there's contention for the network
bandwidth and processing capacity. The cost per gas unit increases as this content increases to balance demand and supply.

The total gas cost = gas fee * cost per gas. This is the final amount charged to the user's account.

For details on how gas is calculated, see [Base Gas](./base-gas.md).

When sending a transaction, users can also specify (often through wallet UIs) the max gas fee they're willing to pay.
This means the transaction cannot use more than max gas * cost per gas unit.

## Gas priority
Transactions are processed in the order of gas paid. The higher the gas the transaction pays, the faster it's picked up and processed.
In order to speed up processing, the users can increase the cost per gas unit to be higher than the current average or mean.

## Gas-related transaction states
1. If user specifies a cost per gas unit lower than the current market rate (floating based on network activity), their transactions
can be stuck waiting in the mempool as higher gas transactions are prioritized. If after ~10 minutes, their transactions are still
not picked up, they will be dropped.
2. Out of gas: If an account does not have enough gas to pay for the specified max gas * cost per gas unit, the transaction will 
be dropped before it's even executed.
3. If a transaction runs out gas due to overly restricted gas cost before the code fully executes, the transaction will fail. The
failure will be recorded on chain and the spent gas will still be collected to compensate for resources used, despite that
transaction failed.
4. If the transaction is executed successfully, the user's account is charged the actual gas fee (based on code executed) * cost per unit.

## Gas simulation
In order to make it easy to calculate the gas amount required for a transaction, most wallets use the simulation API provided by
any full nodes. Simulation runs the transaction in sandbox mode to calculate how much gas fee is needed to execute the code. It also
returns the current average cost per unit of the most recent 100k transactions. Both of the numbers returned will be used to configure
the gas parameters in the transactions sent.

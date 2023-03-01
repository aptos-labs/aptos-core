---
title: "Secure Aptos"
slug: "secure-aptos"
---

# Secure Aptos

At Aptos, the security of our users, developers, node operators, and entire ecosystem is paramount. We built upon the learnings of previous blockchains and undertook all standard security measures.

This page contains a summary of the security features Aptos offers and recommendations for strengthening your own development on the blockchain.

## Aptos security features

Aptos has introduced numerous mechanisms to further strengthen our blockchain’s security, all while remaining scalable and decentralized.


### HAProxy

Aptos employs [HAProxy](http://www.haproxy.org/#desc) to mitigate distributed denial-of-service (DDoS) attacks. HAProxy includes checks for bad state, such as endless loops, and generates a crash with a dump of the problem rather than allowing the issue to take down services or corrupt data.


### Simulation harness

We have a simulation harness that let's users / wallets evaluate a transaction prior to its execution. A wallet or sophisticated user can interpret the results and determine the impact for their account. For the most part, this will protect users from loss due to malicious contracts. There are some probabilistic attacks such as race conditions due to contract upgrades or a timestamp based if statement that does "good" 50% of the time and bad the other.


### Decentralized storage

Every user stores resources under his account, and in such a way, blockchain storage is decentralized. Only the user can register any resource on his account. It makes spam of tokens impossible.


### Upgradable Move code

In the Aptos blockchain, already deployed [Move code can be updated](./move-guides/upgrading-move-code.md). When code is upgraded, all users automatically receive the new code upon execution.

This gives code owners the ability to adapt to needs of the ecosystem using a stable and well-known account address.


### State synchronization

Aptos [state synchronization](./state-sync.md) processes ensure validator nodes and fullnodes are always synchronized to the latest Aptos blockchain state. This ensures all transactions since genesis were correctly agreed upon by consensus, and all transactions were correctly executed by the validators.


### Enforced signed transactions

All transactions executed on the Aptos blockchain [must be signed](./sign-a-transaction.md). In the Aptos blockchain, all data is encoded as Binary Canonical Serialization (BCS). Further, Aptos strongly encourages all transactions to be submitted in this format.


### Key rotation

Accounts on Aptos have the [ability to rotate keys](./system-integrators-guide.md#accounts-on-aptos) so that potentially compromised keys cannot be used to access the accounts. Keys can be rotated via the account::rotate_authentication_key function.


### Replay attack prevention

Aptos processes transactions by [comparing sequence numbers](../concepts/accounts.md#preventing-replay-attacks) of the transaction and sender. The transaction is executed only if those two numbers match; otherwise, it is rejected. This prevents past transactions from being replayed.


### Separate network stacks

The Aptos blockchain supports [distinct networking stacks](../concepts/node-networks-sync.md#separate-network-stacks) for various network topologies. For example, the validator network is independent of the fullnode network. Having separate network stacks provides better support for security preferences (e.g., bidirectional vs server authentication).


## Aptos security recommendations

Here are some recommendations to ensure you make your experience and those of your users safe.


### Protect the signer object

According to the current implementation of the `aptos_std::coin` it is possible for anyone who has access to the signer object to withdraw any coins stored in the account. So for security, you should making `withdraw` and `transfer` functions entry only. In this way, modules may not withdraw any funds. Instead, make it possible for users to get their resources from storage and send to other modules, thereby directly managing all funds.


### Audit with Aptos community

The community platform should be leveraged as a place where folks discuss dApps and collectively audit them. We can imagine an app store model here. The community platform has work to be done in this space to get us there.


### Limit writesets

Limit writesets as part of transaction input -- a user / wallet could specify what are allowed changes and limits as part of a transaction and that can be validated at the end of a transaction. Those that violate those bounds would be aborted. This is pretty complicated because it needs some fuzz factor, non-trivial implementation, and interesting implications for gas during the validation phase


### Leverage a sandbox account

Aptos Wallet has a sandbox account that it moves assets into / out of at the beginning and end of a transaction prior to calling into the actual dapp entry point. Then the dapp only has the signer for the sandbox account. Worst case is the user loses those assets.


### Manage resources safely

It is up to each developer on the Aptos network to practice safe resource management. Remember, access permissions are transmitted through cross-contract calls and other weak points. Code should not merely function but be safe. Become familiar with the differences in calls between modules and calls created from outside, etc.


### Upgrade code with care

When upgrading Move code, take great care in… Consider immutable, which does indeed make upgrading to a new version difficult… (continue summary).


### Sync state from genesis

Whenever possible, [execute transactions from genesis](./state-sync.md#security-implications-and-data-integrity) as the most secure syncing mode. This verifies that all transactions since the beginning of time were correctly agreed upon by consensus and that all transactions were correctly executed by the validators, thereby ensuring data integrity.


### Submit transactions in BCS format

Aptos strongly recommends [developers use the Binary Canonical Serialization (BCS) format](./sign-a-transaction.md#bcs) for submitting transactions to the Aptos blockchain. This format encodes user and transaction data, helping to prevent unwanted access.


### Rotate private keys

Users should rotate their private keys regularly to minimize the effects of a compromise. The Petra Wallet app make this seamless.

## Supporting documentation

* [Upgrading Move Code](./move-guides/upgrading-move-code.md)
* [State Synchronization](./state-sync.md)
* [Creating a Signed Transaction](./sign-a-transaction.md)
* [Aptos Whitepaper](../aptos-white-paper/index.md)
* [haproxy.org](http://www.haproxy.org/) 
* [Aptos: Keep funds safe](https://medium.com/@chestedos/aptos-keep-funds-safe-8ca6f5fdb965)
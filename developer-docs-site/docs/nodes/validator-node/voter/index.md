---
title: "Voter"
slug: "index"
---

import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# Staking Pool Voter 

If you are a [staking pool](../../../concepts/staking.md) voter, then we recommend strongly that you do not store your Aptos voter keys with a custodian before the custodian supports this function. Until then, we suggest you store your voter keys in an Aptos wallet like [Petra](https://petra.app/) or [Martian](https://martianwallet.xyz/).

This document describes how to perform voter operations while using an Aptos wallet. 

### Steps Using Governance UI

To participate as a voter in the Aptos governance, follow the below steps. 

1. Go to the [**Proposals section** of the Aptos Governance page](https://governance.aptosfoundation.org/).
2. Connect your wallet by clicking on **CONNECT WALLET** (top-right):
3. Make sure that wallet is set to connect to Mainnet.
4. View the proposals. When you are ready to vote on a proposal, click on the proposal and vote.
5. You will see a green snackbar indicating that the transaction is successful.

### Steps Using Aptos CLI

1. Get your stake pool info `aptos node get-stake-pool --owner-address <owner-address> --url <REST API for the network>`
2. To see the list of proposal `aptos governance list-proposals --url https://mainnet.aptoslabs.com`
3. To set up your voter profile run `aptos init`
4. To vote on a proposal `aptos governance vote --proposal-id <PROPOSAL_ID> --pool-address <POOL_ADDRESS> --url <URL> --profile <profile>`

# Delegation Pool Voter

If you staked to a [delegation pool](../../../concepts/delegated-staking.md), you can vote proportional to your stake amount in the delegation pool or delegate your votes to another voter address.

### Steps Using Aptos CLI 

To participate as a voter, follow the below steps.

1. Get your delegation pool address from the [Aptos Explorer page](https://explorer.aptoslabs.com/validators/delegation?network=mainnet).
2. To see the list of proposal `aptos governance list-proposals --url https://mainnet.aptoslabs.com`
3. To set up your voter profile run `aptos init`
4. To vote on a proposal `aptos move run --function-id 0x1::delegation_pool::vote --args address:<pool-address> u64:<proposal-id> u64:<voting-power> bool:<true or false>`

To delegate your voting power, follow the below steps.

1. Get your delegation pool address from the [Aptos Explorer page](https://explorer.aptoslabs.com/validators/delegation?network=mainnet).
2. To set up your voter profile run `aptos init`
3. To delegate voting power `aptos move run --function-id 0x1::delegation_pool::delegate_voting_power --args address:<pool-address> address:<delegated-voter-address>`

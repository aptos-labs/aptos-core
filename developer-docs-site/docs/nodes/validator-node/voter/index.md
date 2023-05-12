---
title: "Voter"
slug: "index"
---

import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# Voter 

If you are a voter, then we recommend strongly that you do not store your Aptos voter keys with a custodian before the custodian supports this function. Until then, we suggest you store your voter keys in an Aptos wallet like [Petra](https://petra.app/) or [Martian](https://martianwallet.xyz/).

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
3. To vote on a proposal `aptos governance vote --proposal-id <PROPOSAL_ID> --pool-address <POOL_ADDRESS> --url <URL> --profile <profile>`

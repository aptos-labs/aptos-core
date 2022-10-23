---
title: "Voter"
slug: "index"
---

import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# Voter 

:::tip Petra on Chrome browser only
The [Petra wallet extension](/docs/guides/install-petra-wallet.md) is supported only on the Chrome browser. However, the extensions for [Brave browser](https://brave.com/) and [Kiwi browser](https://kiwibrowser.com/) and [Microsoft Edge browser](https://www.microsoft.com/en-us/edge) will also work.
:::

If you are a voter, then we recommend strongly that you do not store your Aptos voter keys with a custodian before the custodian supports this function. Until then, we suggest you store your voter keys in an Aptos wallet like [Petra](/docs/guides/install-petra-wallet.md) or [Martian](https://martianwallet.xyz/).

This document describes how to perform voter operations while using an Aptos wallet. 

### Steps Using Governance UI

To participate as a voter in the Aptos governance, follow the below steps. 

1. Go to the [**Proposals section** of the Aptos Governance page](https://governance.aptosfoundation.org/).
2. Next you should connect your wallet, but before you click on **CONNECT WALLET** (top-right):
   1. Make sure you selected Mainnnet from the top-right drop-down menu.
   2. Delete any previous versions of Aptos Wallet you have installed on Chrome.
   3. **Install** the Petra (Aptos Wallet) extension on your Chrome browser by [following the instructions here](/guides/install-petra-wallet-extension).
3. Click on **CONNECT WALLET** to connect your wallet to the Aptos Governance. 
4. Make sure that the Mainnet network is selected on the wallet also. On Petra you can do this by the following:
   - On Petra wallet window, click on the bottom right gear box icon. This will take you to **Settings** page on the wallet.
   - Select **Network** > Mainnet.
5. View the proposals. When you are ready to vote on a proposal, click on the proposal and vote.
6. You will see a green snackbar indicating that the transaction is successful.

### Steps Using Aptos CLI

1. Get your stake pool info `aptos node get-stake-pool --owner-address <owner-address> --url <REST API for the network>`
2. To see the list of proposal `aptos governance list-proposals --url https://mainnet.aptoslabs.com`
3. To vote on a proposal `aptos governance vote --proposal-id <PROPOSAL_ID> --pool-address <POOL_ADDRESS> --url <URL> --profile <profile>`

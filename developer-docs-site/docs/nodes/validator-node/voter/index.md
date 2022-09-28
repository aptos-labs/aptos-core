---
title: "Voter"
slug: "index"
---

import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# Voter 

:::tip Petra on Chrome browser only
The [Petra wallet](/docs/guides/install-petra-wallet.md) is supported only on the Chrome browser. You can also use Petra extension on [Brave browser](https://brave.com/) and [Kiwi browser](https://kiwibrowser.com/) and [Microsoft Edge browser](https://www.microsoft.com/en-us/edge).
:::

If you are a voter, then we recommend strongly that you do not store your Aptos voter keys with a custodian before the custodian supports this function. Until then, we suggest you store your voter keys in an Aptos wallet like [Petra](/docs/guides/install-petra-wallet.md) or [Martian](https://martianwallet.xyz/).

This document describes how to perform voter operations while using an Aptos wallet. 

### Summary steps

<center>
<ThemedImage
alt="Signed Transaction Flow"
sources={{
    light: useBaseUrl('/img/docs/voter-flow.svg'),
    dark: useBaseUrl('/img/docs/voter-flow-dark.svg'),
  }}
/>
</center>

### Detailed steps

To participate as a voter in the Aptos governance, follow the below steps. 

1. Go to the [Aptos Community page](https://aptoslabs.com/community) and follow the steps for registering or signing in to your Discord account. Registering on the community page will enable you to receive notifications critical to your voter tasks.

2. Before you click on Step 2 **CONNECT WALLET**:
   1. Delete any previous versions of Aptos Wallet you have installed on Chrome.
   2. **Install** the Petra (Aptos Wallet) extension on your Chrome browser by [following the instructions here](/guides/install-petra-wallet-extension).

3. <span id="create-wallet">Create the voter wallet using Petra</span>.

   1. Open the Petra extension from the Extensions section of the Chrome browser, or by clicking on the puzzle piece on top right of the browser and selecting Aptos Wallet.
   2. Click **Create a new wallet**.
   3. Make sure to store your seed phrase somewhere safe. This account will be used in the future.

4. Click on Step 2 **CONNECT WALLET** to register the voter wallet address to your Aptos Community account. 

5. Click on the Step 3 **COMPLETE SURVEY** to complete the survey.

6. Next, proceed to vote.

## Vote

1. From the Chrome browser, go to the [**Proposals section** of the Aptos Governance page](https://explorer.aptoslabs.com/proposals?network=Devnet).
2. View the proposals. When you are ready to vote on a proposal, click on the proposal.
3. Make sure you connected the wallet with your **voter** wallet account.
4. Provide your **owner** account address and vote “For” or “Against”.
5. You will see a green snackbar indicating that the transaction is successful.



---
title: "Steps in AIT-3"
slug: "steps-in-ait3"
---

import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# Steps in AIT-3

:::caution DRAFT-only
These AIT-3 docs are draft-only for now.
:::

See below the summary flowcharts and detailed steps you will execute while participating.

### Deploy the validator node and register 

Participants in the AIT-3 program must demonstrate the ability to configure and deploy a node, and pass the minimum performance requirements as reported by the [Node Health Checker](/nodes/node-health-checker). 

#### Summary steps

<ThemedImage
alt="Signed Transaction Flow"
sources={{
    light: useBaseUrl('/img/docs/install-node-and-register.svg'),
    dark: useBaseUrl('/img/docs/install-node-and-register.svg'),
  }}
/>

#### Detailed steps

To participate in the AIT-3 program, follow the below steps. Use these steps as a checklist to keep track of your progress. Click on the links in each step for a detailed documentation.

<div class="docs-card-container">
<div class="step">
    <div>
        <div class="circle">1</div>
    </div>
    <div>
        <div class="step-title">Read the Node Requirements</div>
        <div class="step-caption">Before you proceed, make sure that your hardware, storage and network resources satisfy the <a href="./node-requirements"><strong> Node Requirements</strong></a>.  
        </div>
    </div>
</div>
<div class="step">
    <div>
        <div class="circle">2</div>
    </div>
    <div>
        <div class="step-title">Follow the instructions and deploy a validator node in the test mode.</div>
        <div class="step-caption">Follow the detailed node installation steps provided in: <a href="/nodes/validator-node/validators">Install the nodes for AIT-3</a>. <strong>Make sure to set your node in the Test mode.</strong> Instructions are provided in the node installation sections. Test mode is required for Aptos Labs to do a health check on your node.  </div>
    </div>
</div>
<div class="step">
    <div>
        <div class="circle">3</div>
    </div>
    <div>
        <div class="step-title">Register your node in the Aptos Community Platform</div>
        <div class="step-caption">Navigate to the <a href="https://aptoslabs.com/community">Aptos Community page</a> and register your node. Provide your account address, your owner public key, and your validator's network addresses. </div>
    </div>
</div>
<div class="step">
    <div>
        <div class="circle">4</div>
    </div>
    <div>
        <div class="step-title">If your node passes healthcheck, you will be prompted to complete the KYC process</div>
        <div class="step-caption">The Aptos team will perform a node health check on your validator, using the <a href="https://aptos.dev/nodes/node-health-checker">Node Health Checker</a>. When Aptos confirms that your node is healthy, you will be asked to complete the KYC process. </div>
        <div class="step-caption">You will also be automatically enrolled in the Aptos notifications. This will enable you to receive all communication from Aptos Labs throughout the AIT-3 program. </div>
    </div>
</div>
<div class="step">
    <div>
    <div class="step-active circle">5</div>
    </div>
    <div>
    <div class="step-title">If you are selected, then proceed to Install the Wallet step</div>
    </div>
    </div>
    </div>
<div>

</div>
<br />

### Install the Wallet and receive airdropped coins

#### Summary steps

<ThemedImage
alt="Signed Transaction Flow"
sources={{
    light: useBaseUrl('/img/docs/wallet-actions.svg'),
    dark: useBaseUrl('/img/docs/wallet-actions.svg'),
  }}
/>

#### Detailed steps

:::caution Before you proceed
Proceed to the below steps only if your node has passed the NHC for AIT-3 and you are selected to participate in the AIT-3.
:::


## Step 2: Prepare your wallet and accounts

By the end of this step, you will have 3 accounts that serve as the owner, the operator/validator, and the voter.

1. From the Chrome browser, go to the [Aptos Governance page for AIT-3](https://explorer.devnet.aptos.dev/proposals?network=test2). 

2. Install Petra (Aptos Wallet).
    
  Click on the **INSTALL WALLET** button and follow the directions to install the Aptos Wallet Extension on your Chrome browser. 
    
3. Create wallets for the owner, operator and the voter. In this step you will create wallets for the three personas: the owner, the operator and the voter. See [Personas, accounts and keys](#personas-accounts-and-keys) for an explanation of personas. 
    
    1. Create the first wallet. **This first wallet will always be the owner wallet**.
        1. Open the Aptos Wallet extension from the Extensions section of the Chrome browser, or by clicking on the puzzle piece on top right of the browser and selecting Aptos Wallet.
        2. Click **Create a new wallet**. 
        3. When you are done creating the wallet, go to **Extensions** > **Aptos Wallet** and click on the gear icon on the bottom right. You will see the **Settings** screen. Click on the **Network** button and select **Localhost** network. 
        4. Next, click again on the gear icon on the bottom right to go to the **Settings** page. Click on the **Credentials**. Copy and paste and save both the **Private key** and **Address** keys in a separate text file for later use. **This private key is the owner key**.
    2. Create the operator and the voter wallets. 
        1. Open the Aptos Wallet extension. 
        2. Click on the pair of 3 vertical dots at the top right of the Aptos Wallet screen. A slide-up screen will appear showing **+New Wallet** button. Create two more wallets. Name one wallet as **operator** and the other wallet as **voter**. The order of wallet creation does not matter.  
        3. Save the **Private key** and **Address** for the operator and the voter wallets. See the first wallet creation step above that describes how to save. 
        4. Now you have three wallets.



## Step 3: Staking

By the end of this step, you will establish the owner-operator-voter relationship.

1. Staking with UI
    1. Go to [http://localhost:3000/proposals/stake](http://localhost:3000/proposals/stake)
    2. Make sure the wallet is connected with your OWNER account.
    3. Type in inputs
        1. Staking Amount: a positive number (e.g. 3000)
        2. Operator Address: the address of your operator/validator account
        3. Voter Address: the address of your voter account
    4. Submit. You should see a green snack bar indicating that the transaction is successful.
2. Add your validator to the validator set
    1. rotate the consensus key
        1. TODO
    2. Join to the validator set
        1. TODO
3. Verify the owner-operator relationship
    1. Go to [http://localhost:3000/account/0x1](http://localhost:3000/account/0x1)
    2. Command + F, search for “active_validators”. 
    3. You should find your owner’s address in the set.
    

## Step 4: Create Proposals with UI

By the end of this step, you will have some testing proposals ready. Note that in AIT3, we won’t need this step because proposals will be created with CLI.

1. Open network validator file and get `account_address` and `account_private_key`.
    
    ```json
    aptos-core % open .aptos/testnet/0/validator-identity.yaml
    ```
    
2. Create proposals
    1. Go to [http://localhost:3000/proposals/create/?network=local](http://localhost:3000/proposals/create/?network=local)
    2. Input `account_address` in "Account Address", `account_private_key` in "Account Secret Key". (can use the ones from `aptos-core/.aptos/testnet/0/validator-identity.yaml`)
    3. Click “Create a test proposal” for as many times as you want.
    

## Step 5: Vote

You will be testing the voting feature in this step.

1. Go to [http://localhost:3000/proposals](http://localhost:3000/proposals), and click on one of the proposals in the proposals table.
2. Make sure you connected the wallet with your VOTER account. 
3. Input your OWNER account address and vote “For” or “Against”. 
4. You should see a green snack bar indicating that the transaction is successful.

## Step 6

If you are here, it means that we did a good job! Please let us (Zihan and Maayan) know if there’s any feedback. Thank you!


### Connect to AIT-3 and join the validator set

#### Summary steps

<ThemedImage
alt="Signed Transaction Flow"
sources={{
    light: useBaseUrl('/img/docs/connect-to-ait3-and-join-validator-set.svg'),
    dark: useBaseUrl('/img/docs/connect-to-ait3-and-join-validator-set.svg'),
  }}
/>

#### Detailed steps

See [Connecting to Aptos Incentivized Testnet](/nodes/ait/connect-to-testnet) for detailed steps.

### Staking and voting

:::tip Read the Staking document

Make sure you read the [Staking](/nodes/staking) documentation before proceeding further. 
:::

#### Summary steps

<ThemedImage
alt="Signed Transaction Flow"
sources={{
    light: useBaseUrl('/img/docs/owner-staking-op-voter-actions.svg'),
    dark: useBaseUrl('/img/docs/owner-staking-op-voter-actions.svg'),
  }}
/>


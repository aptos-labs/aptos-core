---
title: "Owner"
slug: "index"
---

import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# Owner 

This document describes how to use [Aptos CLI](/docs/cli-tools/aptos-cli-tool/index.md) to perform owner operations during validation.

:::tip Petra on Chrome browser only
The [Petra wallet extension](/docs/guides/install-petra-wallet.md) is supported only on the Chrome browser. However, the extensions for [Brave browser](https://brave.com/) and [Kiwi browser](https://kiwibrowser.com/) and [Microsoft Edge browser](https://www.microsoft.com/en-us/edge) will also work.
:::

## Owner operations with CLI

:::tip Testnet vs Mainnet
The below examples use testnet. See the `--rest-url` value for mainnet in [Aptos Blockchain Deployments](/docs/nodes/aptos-deployments.md).
:::

### Initialize CLI

Initialize CLI with your Petra wallet private key or create new wallet. 

```bash
aptos init --profile testnet-owner \
  --rest-url https://fullnode.testnet.aptoslabs.com/v1
```

You can either enter the private key from an existing wallet, or create new wallet address.

### Initialize staking pool

```bash
aptos stake initialize-stake-owner \
  --initial-stake-amount 100000000000000 \
  --operator-address <operator-address> \
  --voter-address <voter-address> \
  --profile testnet-owner
```

### Transfer coin between accounts

```bash
aptos account transfer \
  --account <operator-address> \
  --amount <amount> \
  --profile testnet-owner
```

### Switch operator

```bash
aptos stake set-operator \
  --operator-address <new-operator-address> \ 
  --profile testnet-owner
```

### Switch voter

```bash
aptos stake set-delegated-voter \
  --voter-address <new-voter-address> \ 
  --profile testnet-owner
```

### Add stake

```bash
aptos stake add-stake \
  --amount <amount> \
  --profile testnet-owner
```

### Increase stake lockup

```bash
aptos stake increase-lockup --profile testnet-owner
```

### Unlock stake

```bash
aptos stake unlock-stake \
  --amount <amount> \
  --profile testnet-owner
```

### Withdraw stake

```bash
aptos stake withdraw-stake \
  --amount <amount> \
  --profile testnet-owner
```

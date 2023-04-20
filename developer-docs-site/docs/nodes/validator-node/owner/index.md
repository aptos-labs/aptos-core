---
title: "Owner"
slug: "index"
---

import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# Owner 

This document describes how to [use the Aptos CLI](../../../cli-tools/aptos-cli-tool/use-aptos-cli.md) to perform owner operations during validation.

## Owner operations with CLI

:::tip Testnet vs Mainnet
The below CLI command examples use mainnet. Change the `--network` value for testnet and devnet. View the values in [Aptos Blockchain Deployments](../../aptos-deployments.md) to see how profiles can be configured based on the network.
:::

### Initialize CLI

Initialize CLI with a private key from an existing account, such as a wallet, or create a new account.

```bash
aptos init --profile mainnet-owner \
  --network mainnet
```

You can either enter the private key from an existing wallet, or create new wallet address.

### Initialize staking pool

```bash
aptos stake initialize-stake-owner \
  --initial-stake-amount 100000000000000 \
  --operator-address <operator-address> \
  --voter-address <voter-address> \
  --profile mainnet-owner
```

### Transfer coin between accounts

```bash
aptos account transfer \
  --account <operator-address> \
  --amount <amount> \
  --profile mainnet-owner
```

### Switch operator

```bash
aptos stake set-operator \
  --operator-address <new-operator-address> \ 
  --profile mainnet-owner
```

### Switch voter

```bash
aptos stake set-delegated-voter \
  --voter-address <new-voter-address> \ 
  --profile mainnet-owner
```

### Add stake

```bash
aptos stake add-stake \
  --amount <amount> \
  --profile mainnet-owner
```

### Increase stake lockup

```bash
aptos stake increase-lockup --profile mainnet-owner
```

### Unlock stake

```bash
aptos stake unlock-stake \
  --amount <amount> \
  --profile mainnet-owner
```

### Withdraw stake

```bash
aptos stake withdraw-stake \
  --amount <amount> \
  --profile mainnet-owner
```

---
title: "Owner"
slug: "index"
---

import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# Owner 

This document describes how to use [Aptos CLI](/docs/cli-tools/aptos-cli-tool/index.md) to perform owner operations during validation.

:::tip Using Petra wallet
This document assumes that you are using [Petra wallet](/docs/guides/install-petra-wallet.md). The [Petra wallet](/docs/guides/install-petra-wallet.md) is supported only on the Chrome browser. You can also use Petra extension on [Brave browser](https://brave.com/) and [Kiwi browser](https://kiwibrowser.com/) and [Microsoft Edge browser](https://www.microsoft.com/en-us/edge).
:::

## Owner operations with CLI

:::tip Examples using testnet
The CLI command examples used in this section use testnet. You can use the same command for mainnet by passing the mainnet URL for the `--rest-url` parameter.
::: 

### Initialize CLI

Initialize CLI with your Petra wallet private key or create new wallet. The below example uses testnet:

```bash
aptos init --profile testnet-owner \
  --rest-url http://testnet.aptoslabs.com
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
  --profile testnet-owner \
  --max-gas 10000
```

:::tip Max gas
You can adjust the above `max-gas` number. Ensure that you sent your operator enough tokens to pay for the gas fee.
:::

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

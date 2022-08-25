// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { AptosClient } from "./aptos_client.js";
import { FAUCET_URL, NODE_URL } from "./util.test.js";
import { FaucetClient } from "./faucet_client.js";
import { AptosAccount } from "./aptos_account.js";
import { CoinClient } from "./coin_client.js";

test(
  "transferCoins and checkBalance works",
  async () => {
    const client = new AptosClient(NODE_URL);
    const faucetClient = new FaucetClient(NODE_URL, FAUCET_URL);
    const coinClient = new CoinClient(client);

    const alice = new AptosAccount();
    const bob = new AptosAccount();
    await faucetClient.fundAccount(alice.address(), 50000);
    await faucetClient.fundAccount(bob.address(), 0);

    await coinClient.transfer(alice, bob, 42, { checkSuccess: true });

    expect(await coinClient.checkBalance(bob)).toBe(42n);
  },
  30 * 1000,
);

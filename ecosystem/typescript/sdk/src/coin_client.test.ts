// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { AptosClient } from "./aptos_client";
import { getFaucetClient, NODE_URL } from "./utils/test_helper.test";
import { AptosAccount } from "./aptos_account";
import { CoinClient } from "./coin_client";

test(
  "transferCoins and checkBalance works",
  async () => {
    const client = new AptosClient(NODE_URL);
    const faucetClient = getFaucetClient();
    const coinClient = new CoinClient(client);

    const alice = new AptosAccount();
    const bob = new AptosAccount();
    await faucetClient.fundAccount(alice.address(), 100_000_000);
    await faucetClient.fundAccount(bob.address(), 0);

    await client.waitForTransaction(await coinClient.transfer(alice, bob, 42), { checkSuccess: true });

    expect(await coinClient.checkBalance(bob)).toBe(BigInt(42));
  },
  30 * 1000,
);

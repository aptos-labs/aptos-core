// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
import fs from "fs";
import * as Gen from "../../generated/index";
import { AptosClient } from "../../providers/aptos_client";
import { getFaucetClient, longTestTimeout, NODE_URL } from "../unit/test_helper.test";
import { AptosAccount } from "../../account/aptos_account";
import { CoinClient } from "../../plugins/coin_client";
import { HexString } from "../../utils";
import { Module } from "../../aptos_types";
import { AptosToken } from "../../plugins";
import { Provider } from "../../providers";

test(
  "transfer and checkBalance coin works",
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

    // Test that `createReceiverIfMissing` works.
    const jemima = new AptosAccount();
    await client.waitForTransaction(await coinClient.transfer(alice, jemima, 717, { createReceiverIfMissing: true }), {
      checkSuccess: true,
    });

    // Check that using a string address instead of an account works with `checkBalance`.
    expect(await coinClient.checkBalance(jemima.address().hex())).toBe(BigInt(717));
  },
  longTestTimeout,
);

test.only(
  "transfer and checkBalance fungible asset works",
  async () => {
    console.log(__dirname);

    const managedCoin = fs.readFileSync(`${__dirname}/managed_coin.mv`);
    const packageMetadata = fs.readFileSync(`${__dirname}/package-metadata.bcs`);
    const client = new AptosClient(NODE_URL);

    const faucetClient = getFaucetClient();
    const coinClient = new CoinClient(client);

    const alice = new AptosAccount(
      new HexString("0x6688d83d3128f41ac2f17d6bbe8aba2ef15907ed8b5dc3a6129892baeb5692cb").toUint8Array(),
    );
    const bob = new AptosAccount();
    await faucetClient.fundAccount(alice.address(), 100_000_000);
    await faucetClient.fundAccount(bob.address(), 0);

    let txnHash = await client.publishPackage(alice, new HexString(packageMetadata.toString("hex")).toUint8Array(), [
      new Module(new HexString(managedCoin.toString("hex")).toUint8Array()),
    ]);
    await client.waitForTransaction(txnHash, { checkSuccess: true });

    const viewPayload: Gen.ViewRequest = {
      function: `${alice.address().hex()}::managed_coin::get_metadata`,
      type_arguments: [],
      arguments: [],
    };
    const tokenAddress = await client.view(viewPayload);

    const mintPayload: Gen.EntryFunctionPayload = {
      function: `${alice.address().hex()}::managed_coin::mint`,
      type_arguments: [],
      arguments: [5, alice.address().hex()],
    };
    const rawTxn = await client.generateTransaction(alice.address(), mintPayload);
    const signed = await client.signTransaction(alice, rawTxn);
    const transaction = await client.submitSignedBCSTransaction(signed);
    await client.waitForTransaction(transaction.hash, { checkSuccess: true });

    await client.waitForTransaction(
      await coinClient.transfer(alice, bob, 1, {
        coinType: `${alice.address().hex()}::managed_coin::LBR`,
        assetAddress: (tokenAddress as any).inner,
      }),
      { checkSuccess: true },
    );

    expect(
      await coinClient.checkBalance(bob, {
        coinType: `${alice.address().hex()}::managed_coin::LBR`,
        assetAddress: (tokenAddress as any).inner,
      }),
    ).toBe(BigInt(1));
  },
  longTestTimeout,
);

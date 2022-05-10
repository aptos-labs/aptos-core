// Copyright (c) The Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import assert from "assert";
import fs from "fs";
import { Account, RestClient, TESTNET_URL, FAUCET_URL, FaucetClient } from "./first_transaction";

const readline = require("readline").createInterface({
  input: process.stdin,
  output: process.stdout
});

class CoinClient extends RestClient {

  /** Publishes a new module to the blockchain within the specified account */
  async publishModule(accountFrom: Account, moduleHex: string): Promise<string> {
    const payload = {
      "type": "module_bundle_payload",
      "modules": [
        {"bytecode": `0x${moduleHex}`},
      ],
    };
    return await this.executeTransactionWithPayload(accountFrom, payload);
  }
  
  /** Initializes the new coin */
  async initializeCoin(accountFrom: Account, coinTypeAddress: string): Promise<string> {
    let payload: { function: string; arguments: any[]; type: string; type_arguments: any[] } = {
      type: "script_function_payload",
      function: `0x1::Coin::initialize`,
      type_arguments: [
        `0x${coinTypeAddress}::MoonCoin::MoonCoin`,
      ],
      arguments: [
        Buffer.from("Moon Coin", "utf-8").toString("hex"),
        "18",
        false,
      ]
    };
    return await this.executeTransactionWithPayload(accountFrom, payload);
  }

  /** Receiver needs to register the coin before they can receive it */
  async registerCoin(coinReceiver: Account, coinTypeAddress: string): Promise<string> {
    let payload: { function: string; arguments: string[]; type: string; type_arguments: any[] };
    payload = {
      "type": "script_function_payload",
      "function": `0x1::Coin::register`,
      "type_arguments": [
        `0x${coinTypeAddress}::MoonCoin::MoonCoin`,
      ],
      "arguments": []
    };
    return await this.executeTransactionWithPayload(coinReceiver, payload);
  }

  /** Mints the newly created coin to a specified receiver address */
  async mintCoin(
    coinOwner: Account,
    coinTypeAddress: string,
    receiverAddress: string,
    amount: number,
  ): Promise<string> {
    let payload: { function: string; arguments: string[]; type: string; type_arguments: any[] };
    payload = {
      "type": "script_function_payload",
      "function": `0x1::Coin::mint`,
      "type_arguments": [
        `0x${coinTypeAddress}::MoonCoin::MoonCoin`,
      ],
      "arguments": [
        receiverAddress,
        amount.toString(),
      ]
    };
    return await this.executeTransactionWithPayload(coinOwner, payload);
  }

  /** Return the balance of the newly created coin */
  async getBalance(accountAddress: string, coinTypeAddress: string): Promise<string> {
    const resource = await this.accountResource(
      accountAddress,
      `0x1::Coin::CoinStore<0x${coinTypeAddress}::MoonCoin::MoonCoin>`,
    );
    if (resource == null) {
      return null;
    } else {
      return resource["data"]["coin"]["value"]
    }
  }
}

/** run our demo! */
async function main() {
  assert(process.argv.length == 3, "Expecting an argument that points to the helloblockchain module");

  const restClient = new CoinClient(TESTNET_URL);
  const faucetClient = new FaucetClient(FAUCET_URL, restClient);

  // Create two accounts, Alice and Bob, and fund Alice but not Bob
  const alice = new Account();
  const bob = new Account();

  console.log("\n=== Addresses ===");
  console.log(`Alice: ${alice.address()}`);
  console.log(`Bob: ${bob.address()}`);

  await faucetClient.fundAccount(alice.address(), 10_000_000);
  await faucetClient.fundAccount(bob.address(), 10_000_000);

  await new Promise<void>(resolve => {
    readline.question("Update the CoinType module with Alice's address, build, copy to the provided path, and press enter.", () => {
      resolve();
      readline.close();
    });
  });
  const modulePath = process.argv[2];
  const moduleHex = fs.readFileSync(modulePath).toString("hex");

  console.log("Publishing...");

  let txHash = await restClient.publishModule(alice, moduleHex);
  await restClient.waitForTransaction(txHash);

  console.log("Alice will initialize the new coin");
  txHash = await restClient.initializeCoin(alice, alice.address());
  await restClient.waitForTransaction(txHash);

  console.log("Bob registers the newly created coin so he can receive it from Alice");
  txHash = await restClient.registerCoin(bob, alice.address());
  await restClient.waitForTransaction(txHash);
  console.log(`Bob's initial balance: ${await restClient.getBalance(bob.address(), alice.address())}`);

  console.log("Alice mints Bob some of the new coin");
  txHash = await restClient.mintCoin(alice, alice.address(), bob.address(), 100);
  await restClient.waitForTransaction(txHash);
  console.log(`Bob's new balance: ${await restClient.getBalance(bob.address(), alice.address())}`);
}

if (require.main === module) {
  main().then((resp) => console.log(resp));
}

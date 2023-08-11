// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import assert from "assert";
import fs from "fs";
import path from "path";
import { NODE_URL, FAUCET_URL } from "./common";
import {
  AptosAccount,
  TxnBuilderTypes,
  MaybeHexString,
  HexString,
  FaucetClient,
  Network,
  Types,
  Provider,
  FungibleAssetClient,
  CustomEndpoints,
} from "aptos";

/**
 This example depends on the FACoin.move module built with fungible asset having already been published to the destination blockchain.

 One method to do so is to use the CLI:
 * Acquire the Aptos CLI, see https://aptos.dev/cli-tools/aptos-cli/use-cli/install-aptos-cli
 * `pnpm your_fungible_asset ~/aptos-core/aptos-move/move-examples/fungible_asset/fa_coin`.
 * Open another terminal and `aptos move compile --package-dir ~/aptos-core/aptos-move/move-examples/fungible_asset/fa_coin --save-metadata --named-addresses FACoin=<Alice address from above step>`.
 * Return to the first terminal and press enter.
 */

const readline = require("readline").createInterface({
  input: process.stdin,
  output: process.stdout,
});

class AdminClient extends Provider {
  constructor(network: Network | CustomEndpoints) {
    super(network);
  }

  /** Admin forcefully transfers the newly created coin to the specified receiver address */
  async transferCoin(
    admin: AptosAccount,
    fromAddress: HexString,
    toAddress: HexString,
    amount: number | bigint,
  ): Promise<string> {
    const rawTxn = await this.generateTransaction(admin.address(), {
      function: `${admin.address().hex()}::fa_coin::transfer`,
      type_arguments: [],
      arguments: [fromAddress.hex(), toAddress.hex(), amount],
    });

    const bcsTxn = await this.signTransaction(admin, rawTxn);
    const pendingTxn = await this.submitTransaction(bcsTxn);

    return pendingTxn.hash;
  }

  /** Admin mint the newly created coin to the specified receiver address */
  async mintCoin(admin: AptosAccount, receiverAddress: HexString, amount: number | bigint): Promise<string> {
    const rawTxn = await this.generateTransaction(admin.address(), {
      function: `${admin.address().hex()}::fa_coin::mint`,
      type_arguments: [],
      arguments: [receiverAddress.hex(), amount],
    });

    const bcsTxn = await this.signTransaction(admin, rawTxn);
    const pendingTxn = await this.submitTransaction(bcsTxn);

    return pendingTxn.hash;
  }

  /** Admin burns the newly created coin from the specified receiver address */
  async burnCoin(admin: AptosAccount, fromAddress: HexString, amount: number | bigint): Promise<string> {
    const rawTxn = await this.generateTransaction(admin.address(), {
      function: `${admin.address().hex()}::fa_coin::burn`,
      type_arguments: [],
      arguments: [fromAddress.hex(), amount],
    });

    const bcsTxn = await this.signTransaction(admin, rawTxn);
    const pendingTxn = await this.submitTransaction(bcsTxn);

    return pendingTxn.hash;
  }

  /** Admin freezes the primary fungible store of the specified account */
  async freeze(admin: AptosAccount, targetAddress: HexString): Promise<string> {
    const rawTxn = await this.generateTransaction(admin.address(), {
      function: `${admin.address().hex()}::fa_coin::freeze_account`,
      type_arguments: [],
      arguments: [targetAddress.hex()],
    });

    const bcsTxn = await this.signTransaction(admin, rawTxn);
    const pendingTxn = await this.submitTransaction(bcsTxn);

    return pendingTxn.hash;
  }

  /** Admin unfreezes the primary fungible store of the specified account */
  async unfreeze(admin: AptosAccount, targetAddress: HexString): Promise<string> {
    const rawTxn = await this.generateTransaction(admin.address(), {
      function: `${admin.address().hex()}::fa_coin::unfreeze_account`,
      type_arguments: [],
      arguments: [targetAddress.hex()],
    });

    const bcsTxn = await this.signTransaction(admin, rawTxn);
    const pendingTxn = await this.submitTransaction(bcsTxn);

    return pendingTxn.hash;
  }

  /** Return the balance of the newly created coin */
  async getMetadata(admin: AptosAccount): Promise<MaybeHexString> {
    const payload: Types.ViewRequest = {
      function: `${admin.address().hex()}::fa_coin::get_metadata`,
      type_arguments: [],
      arguments: [],
    };
    return ((await this.view(payload)) as any)[0].inner as MaybeHexString;
  }
}

/** run our demo! */
async function main() {
  assert(process.argv.length == 3, "Expecting an argument that points to the fa_coin directory.");

  const client = new AdminClient({ fullnodeUrl: NODE_URL, indexerUrl: NODE_URL /* not used */ });
  const fungibleAssetClient = new FungibleAssetClient(client);
  const faucetClient = new FaucetClient(NODE_URL, FAUCET_URL);

  // Create two accounts, Alice and Bob, and fund Alice but not Bob
  const alice = new AptosAccount();
  const bob = new AptosAccount();
  const charlie = new AptosAccount();

  console.log("\n=== Addresses ===");
  console.log(`Alice: ${alice.address()}`);
  console.log(`Bob: ${bob.address()}`);
  console.log(`Charlie: ${charlie.address()}`);

  await faucetClient.fundAccount(alice.address(), 100_000_000);
  await faucetClient.fundAccount(bob.address(), 100_000_000);

  await new Promise<void>((resolve) => {
    readline.question("Update the module with Alice's address, compile, and press enter.", () => {
      resolve();
      readline.close();
    });
  });

  // :!:>publish
  const modulePath = process.argv[2];
  const packageMetadata = fs.readFileSync(path.join(modulePath, "build", "Examples", "package-metadata.bcs"));
  const moduleData = fs.readFileSync(path.join(modulePath, "build", "Examples", "bytecode_modules", "fa_coin.mv"));

  console.log("Publishing FACoin package.\n");
  let txnHash = await client.publishPackage(alice, new HexString(packageMetadata.toString("hex")).toUint8Array(), [
    new TxnBuilderTypes.Module(new HexString(moduleData.toString("hex")).toUint8Array()),
  ]);
  await client.waitForTransaction(txnHash, { checkSuccess: true }); // <:!:publish

  const metadata_addr = await client.getMetadata(alice);

  console.log("All the balances in this exmaple refer to balance in primary fungible stores of each account.");
  console.log(
    `Alice's initial FACoin balance: ${await fungibleAssetClient.getPrimaryBalance(alice.address(), metadata_addr)}.`,
  );
  console.log(
    `Bob's initial FACoin balance: ${await fungibleAssetClient.getPrimaryBalance(bob.address(), metadata_addr)}.`,
  );
  console.log(
    `Charlie's initial balance: ${await fungibleAssetClient.getPrimaryBalance(charlie.address(), metadata_addr)}.`,
  );
  console.log("Alice mints Charlie 100 coins.");
  txnHash = await client.mintCoin(alice, charlie.address(), 100);
  await client.waitForTransaction(txnHash, { checkSuccess: true });
  console.log(
    `Charlie's updated FACoin primary fungible store balance: ${await fungibleAssetClient.getPrimaryBalance(
      charlie.address(),
      metadata_addr,
    )}.`,
  );

  console.log("Alice freezes Bob's account.");
  txnHash = await client.freeze(alice, bob.address());
  await client.waitForTransaction(txnHash, { checkSuccess: true });

  console.log(
    "Alice as the admin forcefully transfers the newly minted coins of Charlie to Bob ignoring that Bob's account is frozen.",
  );
  txnHash = await client.transferCoin(alice, charlie.address(), bob.address(), 100);
  await client.waitForTransaction(txnHash, { checkSuccess: true });
  console.log(
    `Bob's updated FACoin balance: ${await fungibleAssetClient.getPrimaryBalance(bob.address(), metadata_addr)}.`,
  );

  console.log("Alice unfreezes Bob's account.");
  txnHash = await client.unfreeze(alice, bob.address());
  await client.waitForTransaction(txnHash, { checkSuccess: true });

  console.log("Alice burns 50 coins from Bob.");
  txnHash = await client.burnCoin(alice, bob.address(), 50);
  await client.waitForTransaction(txnHash, { checkSuccess: true });
  console.log(
    `Bob's updated FACoin balance: ${await fungibleAssetClient.getPrimaryBalance(bob.address(), metadata_addr)}.`,
  );

  /// Normal fungible asset transfer between primary stores
  console.log("Bob transfers 10 coins to Alice as the owner.");
  txnHash = await fungibleAssetClient.transfer(bob, metadata_addr, alice.address(), 10);
  await client.waitForTransaction(txnHash, { checkSuccess: true });
  console.log(
    `Alice's updated FACoin balance: ${await fungibleAssetClient.getPrimaryBalance(alice.address(), metadata_addr)}.`,
  );
  console.log(
    `Bob's updated FACoin balance: ${await fungibleAssetClient.getPrimaryBalance(bob.address(), metadata_addr)}.`,
  );
  console.log("done.");
}

if (require.main === module) {
  main().then((resp) => console.log(resp));
}

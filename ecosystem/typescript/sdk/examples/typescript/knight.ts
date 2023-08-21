// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { AptosAccount, HexString, Provider, Network, Types, FaucetClient, BCS } from "aptos";
import { NODE_URL, FAUCET_URL } from "./common";

const provider = new Provider(Network.DEVNET);
const faucetClient = new FaucetClient(NODE_URL, FAUCET_URL);

async function getTokenAddr(ownerAddr: HexString, tokenName: string): Promise<HexString> {
  const tokenOwnership = await provider.getOwnedTokens(ownerAddr);
  for (const ownership of tokenOwnership.current_token_ownerships_v2) {
    if (ownership.current_token_data.token_name === tokenName) {
      return new HexString(ownership.current_token_data.token_data_id);
    }
  }
  console.log(`Token ${tokenName} not found`);
  process.exit(1);
}

async function waitForEnter() {
  return new Promise<void>((resolve, reject) => {
    const rl = require("readline").createInterface({
      input: process.stdin,
      output: process.stdout,
    });

    rl.question("Please press the Enter key to proceed ...\n", () => {
      rl.close();
      resolve();
    });
  });
}

class KnightClient {
  async mintCorn(creator: AptosAccount, receiver: HexString, amount: BCS.AnyNumber): Promise<string> {
    const rawTxn = await provider.generateTransaction(creator.address(), {
      function: `${creator.address()}::food::mint_corn`,
      type_arguments: [],
      arguments: [receiver.hex(), amount],
    });

    const bcsTxn = await provider.signTransaction(creator, rawTxn);
    const pendingTxn = await provider.submitTransaction(bcsTxn);

    return pendingTxn.hash;
  }

  async mintMeat(creator: AptosAccount, receiver: HexString, amount: BCS.AnyNumber): Promise<string> {
    const rawTxn = await provider.generateTransaction(creator.address(), {
      function: `${creator.address()}::food::mint_meat`,
      type_arguments: [],
      arguments: [receiver.hex(), amount],
    });

    const bcsTxn = await provider.signTransaction(creator, rawTxn);
    const pendingTxn = await provider.submitTransaction(bcsTxn);

    return pendingTxn.hash;
  }

  async feedCorn(moduleAddr: HexString, creator: AptosAccount, to: HexString, amount: BCS.AnyNumber): Promise<string> {
    const rawTxn = await provider.generateTransaction(creator.address(), {
      function: `${moduleAddr.hex()}::knight::feed_corn`,
      type_arguments: [],
      arguments: [to.hex(), amount],
    });

    const bcsTxn = await provider.signTransaction(creator, rawTxn);
    const pendingTxn = await provider.submitTransaction(bcsTxn);

    return pendingTxn.hash;
  }

  async feedMeat(moduleAddr: HexString, creator: AptosAccount, to: HexString, amount: BCS.AnyNumber): Promise<string> {
    const rawTxn = await provider.generateTransaction(creator.address(), {
      function: `${moduleAddr.hex()}::knight::feed_meat`,
      type_arguments: [],
      arguments: [to.hex(), amount],
    });

    const bcsTxn = await provider.signTransaction(creator, rawTxn);
    const pendingTxn = await provider.submitTransaction(bcsTxn);

    return pendingTxn.hash;
  }

  async mintKnightToken(
    creator: AptosAccount,
    description: string,
    name: string,
    base_uri: string,
    receiver: HexString,
  ): Promise<string> {
    const rawTxn = await provider.generateTransaction(creator.address(), {
      function: `${creator.address()}::knight::mint_knight`,
      type_arguments: [],
      arguments: [description, name, base_uri, receiver.hex()],
    });

    const bcsTxn = await provider.signTransaction(creator, rawTxn);
    const pendingTxn = await provider.submitTransaction(bcsTxn);

    return pendingTxn.hash;
  }

  async healthPoint(module_addr: HexString, token_addr: HexString): Promise<bigint> {
    const payload: Types.ViewRequest = {
      function: `${module_addr.hex()}::knight::health_point`,
      type_arguments: [],
      arguments: [token_addr.hex()],
    };

    const result = await provider.view(payload);
    return BigInt(result[0] as any);
  }
}

/** run our demo! */
async function main(): Promise<void> {
  const client = new KnightClient();
  const admin = new AptosAccount();
  const user = new AptosAccount();
  await faucetClient.fundAccount(admin.address(), 100_000_000);
  await faucetClient.fundAccount(user.address(), 100_000_000);
  console.log(
    "\nCompile and publish the knight module (`aptos-core/aptos-move/move-examples/token_objects/knight`) using the following profile, and press enter:",
  );
  console.log(` knight_admin:`);
  console.log(` private_key: "${admin.toPrivateKeyObject().privateKeyHex}"`);
  console.log(` public_key: "${admin.pubKey()}"`);
  console.log(` account: ${admin.address()}`);
  console.log(` rest_url: "${NODE_URL}"`);
  console.log(` faucet_url: "${FAUCET_URL}"`);
  await waitForEnter();

  const adminAddr = admin.address();
  const userAddr = user.address();
  const tokenName = "Knight #1";

  console.log("\n=== Addresses ===");
  console.log(`Admin: ${adminAddr} `);
  console.log(`User: ${userAddr} `);
  console.log(`User's private key: "${user.toPrivateKeyObject().privateKeyHex}"`);

  // -----------------
  // Mint Knight Token
  // -----------------
  let txnHash = await client.mintKnightToken(
    admin,
    "Knight Token Description",
    tokenName,
    "https://raw.githubusercontent.com/aptos-labs/aptos-core/main/ecosystem/typescript/sdk/examples/typescript/metadata/knight/",
    userAddr,
  );
  await provider.waitForTransaction(txnHash, { checkSuccess: true });
  console.log("\n=== Knight Token Minted ===");
  console.log(`Txn: https://explorer.aptoslabs.com/txn/${txnHash}?network=devnet`);
  // Get the address of the minted token
  const tokenAddr = await getTokenAddr(userAddr, tokenName);
  console.log(`The address of the minted token: ${tokenAddr}`);
  console.log(`The health point of the knight token: ${await client.healthPoint(adminAddr, tokenAddr)}`);
  await waitForEnter();

  // --------------
  // Mint 10 corns
  // --------------
  txnHash = await client.mintCorn(admin, userAddr, 10);
  await provider.waitForTransaction(txnHash, { checkSuccess: true });
  console.log("\n=== Mint 10 corns ===");
  console.log(`Txn: https://explorer.aptoslabs.com/txn/${txnHash}?network=devnet`);
  await waitForEnter();

  // --------------
  // Mint 10 meats
  // --------------
  txnHash = await client.mintMeat(admin, userAddr, 10);
  await provider.waitForTransaction(txnHash, { checkSuccess: true });
  console.log("\n=== Mint 10 meats ===");
  console.log(`Txn: https://explorer.aptoslabs.com/txn/${txnHash}?network=devnet`);
  await waitForEnter();

  // -------------
  // Feed 3 corns
  // -------------
  txnHash = await client.feedCorn(adminAddr, user, tokenAddr, 3);
  await provider.waitForTransaction(txnHash, { checkSuccess: true });
  console.log("\n=== Feed 3 corns ===");
  console.log(`Txn: https://explorer.aptoslabs.com/txn/${txnHash}?network=devnet`);
  console.log(`The health point of the knight token: ${await client.healthPoint(adminAddr, tokenAddr)}`);
  await waitForEnter();

  // -------------
  // Feed 3 meats
  // -------------
  txnHash = await client.feedMeat(adminAddr, user, tokenAddr, 3);
  await provider.waitForTransaction(txnHash, { checkSuccess: true });
  console.log("\n=== Feed 3 meats ===");
  console.log(`Txn: https://explorer.aptoslabs.com/txn/${txnHash}?network=devnet`);
  console.log(`The health point of the knight token: ${await client.healthPoint(adminAddr, tokenAddr)}`);
  await waitForEnter();
}

main().then(() => {
  console.log("Done!");
  process.exit(0);
});

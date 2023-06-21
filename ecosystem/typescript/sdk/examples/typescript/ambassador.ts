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

class AmbassadorClient {
  async setAmbassadorLevel(
    creator: AptosAccount,
    token: HexString,
    new_ambassador_level: BCS.AnyNumber,
  ): Promise<string> {
    const rawTxn = await provider.generateTransaction(creator.address(), {
      function: `${creator.address()}::ambassador::set_ambassador_level`,
      type_arguments: [],
      arguments: [token.hex(), new_ambassador_level],
    });

    const bcsTxn = await provider.signTransaction(creator, rawTxn);
    const pendingTxn = await provider.submitTransaction(bcsTxn);

    return pendingTxn.hash;
  }

  async burn(creator: AptosAccount, token: HexString): Promise<string> {
    const rawTxn = await provider.generateTransaction(creator.address(), {
      function: `${creator.address()}::ambassador::burn`,
      type_arguments: [],
      arguments: [token.hex()],
    });

    const bcsTxn = await provider.signTransaction(creator, rawTxn);
    const pendingTxn = await provider.submitTransaction(bcsTxn);

    return pendingTxn.hash;
  }

  async mintAmbassadorToken(
    creator: AptosAccount,
    description: string,
    name: string,
    uri: string,
    soul_bound_to: HexString,
  ): Promise<string> {
    const rawTxn = await provider.generateTransaction(creator.address(), {
      function: `${creator.address()}::ambassador::mint_ambassador_token`,
      type_arguments: [],
      arguments: [description, name, uri, soul_bound_to.hex()],
    });

    const bcsTxn = await provider.signTransaction(creator, rawTxn);
    const pendingTxn = await provider.submitTransaction(bcsTxn);

    return pendingTxn.hash;
  }

  async ambassadorLevel(creator_addr: HexString, token_addr: HexString): Promise<bigint> {
    const payload: Types.ViewRequest = {
      function: `${creator_addr.hex()}::ambassador::ambassador_level`,
      type_arguments: [],
      arguments: [token_addr.hex()],
    };

    const result = await provider.view(payload);
    return BigInt(result[0] as any);
  }
}

/** run our demo! */
async function main(): Promise<void> {
  const client = new AmbassadorClient();

  const admin = new AptosAccount();
  const user = new AptosAccount();

  await faucetClient.fundAccount(admin.address(), 100_000_000);
  await faucetClient.fundAccount(user.address(), 100_000_000);

  console.log(
    "\nCompile and publish the Ambassador module (`aptos-core/aptos-move/move-examples/token_objects/ambassador`) using the following profile, and press enter:",
  );
  console.log(`  ambassador_admin:`);
  console.log(`    private_key: "${admin.toPrivateKeyObject().privateKeyHex}"`);
  console.log(`    public_key: "${admin.pubKey()}"`);
  console.log(`    account: ${admin.address()}`);
  console.log(`    rest_url: "https://fullnode.devnet.aptoslabs.com"`);
  console.log(`    faucet_url: "https://faucet.devnet.aptoslabs.com"`);

  await waitForEnter();

  const adminAddr = admin.address();
  const userAddr = user.address();
  const tokenName = "Aptos Ambassador #1";

  console.log("\n=== Addresses ===");
  console.log(`Admin: ${adminAddr} `);
  console.log(`User: ${userAddr} `);

  // Mint Ambassador Token
  let txnHash = await client.mintAmbassadorToken(
    admin,
    "Aptos Ambassador Token",
    tokenName,
    "https://raw.githubusercontent.com/aptos-labs/aptos-core/main/ecosystem/typescript/sdk/examples/typescript/metadata/ambassador/",
    userAddr,
  );
  await provider.waitForTransaction(txnHash, { checkSuccess: true });
  console.log("\n=== Ambassador Token Minted ===");
  console.log(`Txn: https://explorer.aptoslabs.com/txn/${txnHash}?network=devnet`);
  // Get the address of the minted token
  const tokenAddr = await getTokenAddr(userAddr, tokenName);
  console.log(`The address of the minted token: ${tokenAddr}`);
  console.log(`The level of the token: ${await client.ambassadorLevel(adminAddr, tokenAddr)}`);
  await waitForEnter();

  // Set Ambassador Level to 15
  txnHash = await client.setAmbassadorLevel(admin, tokenAddr, 15);
  await provider.waitForTransaction(txnHash, { checkSuccess: true });
  console.log("\n=== Level set to 15 ===");
  console.log(`Txn: https://explorer.aptoslabs.com/txn/${txnHash}?network=devnet`);
  console.log(`The level of the token: ${await client.ambassadorLevel(adminAddr, tokenAddr)}`);
  await waitForEnter();

  // Set Ambassador Level to 25
  txnHash = await client.setAmbassadorLevel(admin, tokenAddr, 25);
  await provider.waitForTransaction(txnHash, { checkSuccess: true });
  console.log("\n=== Level set to 25 ===");
  console.log(`Txn: https://explorer.aptoslabs.com/txn/${txnHash}?network=devnet`);
  console.log(`The level of the token: ${await client.ambassadorLevel(adminAddr, tokenAddr)}`);
  await waitForEnter();

  // Burn the token
  txnHash = await client.burn(admin, tokenAddr);
  await provider.waitForTransaction(txnHash, { checkSuccess: true });
  console.log("\n=== Token burned ===");
  console.log(`Txn: https://explorer.aptoslabs.com/txn/${txnHash}?network=devnet`);
  await waitForEnter();
}

main().then(() => {
  console.log("Done!");
  process.exit(0);
});

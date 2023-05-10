// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { AptosAccount, FaucetClient, Provider, Network } from "aptos";

const readline = require("readline").createInterface({
  input: process.stdin,
  output: process.stdout,
});

async function main() {
  const provider = new Provider(Network.TESTNET);
  const faucetClient = new FaucetClient(
    "https://fullnode.testnet.aptoslabs.com",
    "https://faucet.testnet.aptoslabs.com",
  );

  const alice = new AptosAccount();

  console.log("\n=== Addresses ===");
  console.log(`Alice: ${alice.address()}`);

  //await faucetClient.fundAccount(alice.address(), 100_000_000);

  // await new Promise<void>((resolve) => {
  //   readline.question("press enter.", () => {
  //     resolve();
  //     readline.close();
  //   });
  // });

  const tokenOwnership = await provider.getTokenOwnershipV2(
    "0xe2134e886b6be06b6d1e0c6a944213497260a69a7ce2a7198880a04bd475ae0e",
  );
  console.log(`token amounts = ${tokenOwnership.current_token_ownerships_v2.length} `);
  console.log(`first token name is ${tokenOwnership.current_token_ownerships_v2[0].current_token_data?.token_name} `);
}

if (require.main === module) {
  main().then((resp) => console.log(resp));
}

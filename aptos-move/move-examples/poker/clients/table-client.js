#!/usr/bin/env node
/**
 * Table client – runs in TEE.
 * 1. Register table on-chain (with Nitro attestation; in prod use real attestation from NSM).
 * 2. Loop: wait for min players, run hand, submit settle_hand and settle_leaving_players.
 *
 * Usage:
 *   TABLE_PRIVATE_KEY=0x... POKER_MODULE_ADDRESS=0x... NODE_URL=https://... node table-client.js
 *
 * For dev: POKER_MODULE_ADDRESS is the address where the poker module is deployed (e.g. table account).
 */

const { Aptos, AptosConfig } = require("@aptos-labs/ts-sdk");

const NODE_URL = process.env.NODE_URL || "https://fullnode.devnet.aptoslabs.com";
const TABLE_PRIVATE_KEY = process.env.TABLE_PRIVATE_KEY;
const POKER_MODULE_ADDRESS = process.env.POKER_MODULE_ADDRESS;

if (!TABLE_PRIVATE_KEY || !POKER_MODULE_ADDRESS) {
  console.error("Set TABLE_PRIVATE_KEY and POKER_MODULE_ADDRESS");
  process.exit(1);
}

const config = new AptosConfig({ fullnode: NODE_URL });
const aptos = new Aptos(config);

const MODULE_NAME = "poker";
const POKER_MODULE = `${POKER_MODULE_ADDRESS}::${MODULE_NAME}::poker`;

async function main() {
  const accountInfo = await aptos.getAccountInfo({ accountAddress: POKER_MODULE_ADDRESS }).catch(() => null);
  if (!accountInfo) {
    console.error("Table account not found. Create and fund it first.");
    process.exit(1);
  }

  // In production: get attestation bytes from Nitro Enclave NSM (GetAttestationDocument).
  // Here we use a placeholder; on-chain register_table will fail unless verification is bypassed (test only).
  const attestationDoc = new Uint8Array(0);

  const { Account, Ed25519PrivateKey } = require("@aptos-labs/ts-sdk");
  const key = Ed25519PrivateKey.fromString(TABLE_PRIVATE_KEY.replace(/^0x/, ""));
  const tableAccount = Account.fromPrivateKey({ privateKey: key });

  console.log("Registering table (attestation required in prod)...");
  const payload = {
    function: `${POKER_MODULE}::register_table`,
    typeArguments: [],
    functionArguments: [Array.from(attestationDoc)],
  };
  const txn = await aptos.transaction.build.simple({
    sender: tableAccount.accountAddress.toStringLong(),
    withFeePayer: false,
    data: {
      function: payload.function,
      typeArguments: payload.typeArguments,
      functionArguments: payload.functionArguments,
    },
  });
  try {
    const committed = await aptos.signAndSubmitTransaction({ signer: tableAccount, transaction: txn });
    await aptos.waitForTransaction({ transactionHash: committed.hash });
    console.log("Table registered. Tx:", committed.hash);
  } catch (e) {
    console.error("register_table failed (expected if attestation invalid):", e.message);
  }

  console.log("\nTable client loop (conceptual):");
  console.log("1. Poll / query players (player_balance view).");
  console.log("2. When >= min_players, run hand off-chain.");
  console.log("3. Submit settle_hand(deduct_from, deduct_amounts, add_to, add_amounts).");
  console.log("4. Submit settle_leaving_players(leaving_players).");
  console.log("5. Repeat; if not enough players, wait.");
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});

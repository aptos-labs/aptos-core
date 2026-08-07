#!/usr/bin/env node
/**
 * Table client – runs in TEE.
 * 1. Register table on-chain (with Nitro attestation; in prod use real attestation from NSM).
 * 2. Loop: wait for min players, run hand, submit settle_hand and settle_leaving_players.
 *
 * Usage:
 *   TABLE_PRIVATE_KEY=0x... POKER_MODULE_ADDRESS=0x... ATTESTATION_DOC_PATH=/path/doc.bin node table-client.js
 *
 * For dev: POKER_MODULE_ADDRESS is the address where the poker module is deployed (e.g. table account).
 */

const fs = require("fs");
const { Aptos, AptosConfig, Account, Ed25519PrivateKey } = require("@aptos-labs/ts-sdk");

const NODE_URL = process.env.NODE_URL || "https://fullnode.devnet.aptoslabs.com";
const TABLE_PRIVATE_KEY = process.env.TABLE_PRIVATE_KEY;
const POKER_MODULE_ADDRESS = process.env.POKER_MODULE_ADDRESS;
const ATTESTATION_DOC_PATH = process.env.ATTESTATION_DOC_PATH;

if (!TABLE_PRIVATE_KEY || !POKER_MODULE_ADDRESS) {
  console.error("Set TABLE_PRIVATE_KEY and POKER_MODULE_ADDRESS");
  process.exit(1);
}

const config = new AptosConfig({ fullnode: NODE_URL });
const aptos = new Aptos(config);

const MODULE_NAME = "poker";
const POKER_MODULE = `${POKER_MODULE_ADDRESS}::${MODULE_NAME}`;

function accountFromHexPrivateKey(privateKey) {
  const key = new Ed25519PrivateKey(privateKey.replace(/^0x/, ""));
  return Account.fromPrivateKey({ privateKey: key });
}

async function main() {
  // In production, generate this inside the enclave with NSM GetAttestationDocument.
  // The document's user_data must be b"APTOS_POKER_TABLE_V1" || bcs(table_address).
  const attestationDoc = ATTESTATION_DOC_PATH ? fs.readFileSync(ATTESTATION_DOC_PATH) : new Uint8Array(0);
  if (!ATTESTATION_DOC_PATH) {
    console.warn("ATTESTATION_DOC_PATH not set; register_table is expected to fail outside test-only flows.");
  }

  const tableAccount = accountFromHexPrivateKey(TABLE_PRIVATE_KEY);

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

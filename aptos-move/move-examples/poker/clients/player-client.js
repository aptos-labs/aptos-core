#!/usr/bin/env node
/**
 * Player client – enter table (lock APT as chips) and request leave.
 *
 * Usage:
 *   node player-client.js enter <table_address> <amount_octas>
 *   node player-client.js leave <table_address>
 *   node player-client.js balance <table_address>
 *
 * Env: PLAYER_PRIVATE_KEY (hex), NODE_URL, POKER_MODULE_ADDRESS (module deployer = table addr if same).
 */

const { Aptos, AptosConfig, Account, Ed25519PrivateKey } = require("@aptos-labs/ts-sdk");

const NODE_URL = process.env.NODE_URL || "https://fullnode.devnet.aptoslabs.com";
const PLAYER_PRIVATE_KEY = process.env.PLAYER_PRIVATE_KEY;
const POKER_MODULE_ADDRESS = process.env.POKER_MODULE_ADDRESS;

const cmd = process.argv[2];
const tableAddr = process.argv[3];
const amountStr = process.argv[4];

if (!["enter", "leave", "balance"].includes(cmd) || !tableAddr) {
  console.error("Usage: node player-client.js enter <table_address> <amount_octas> | leave <table_address> | balance <table_address>");
  process.exit(1);
}

const config = new AptosConfig({ fullnode: NODE_URL });
const aptos = new Aptos(config);

const moduleAddr = POKER_MODULE_ADDRESS || tableAddr;
const MODULE_NAME = "poker";
const POKER_MODULE = `${moduleAddr}::${MODULE_NAME}::poker`;

async function getPlayerAddress() {
  if (!PLAYER_PRIVATE_KEY) {
    console.error("Set PLAYER_PRIVATE_KEY");
    process.exit(1);
  }
  const key = Ed25519PrivateKey.fromString(PLAYER_PRIVATE_KEY.replace(/^0x/, ""));
  const account = Account.fromPrivateKey({ privateKey: key });
  return { account, address: account.accountAddress.toStringLong() };
}

async function enterTable(account, tableAddress, amount) {
  const payload = {
    function: `${POKER_MODULE}::enter_table`,
    typeArguments: [],
    functionArguments: [tableAddress, amount],
  };
  const txn = await aptos.transaction.build.simple({
    sender: account.accountAddress.toStringLong(),
    withFeePayer: false,
    data: {
      function: payload.function,
      typeArguments: payload.typeArguments,
      functionArguments: payload.functionArguments,
    },
  });
  const committed = await aptos.signAndSubmitTransaction({ signer: account, transaction: txn });
  await aptos.waitForTransaction({ transactionHash: committed.hash });
  console.log("Entered table. Tx:", committed.hash);
}

async function requestLeave(account, tableAddress) {
  const payload = {
    function: `${POKER_MODULE}::request_leave`,
    typeArguments: [],
    functionArguments: [tableAddress],
  };
  const txn = await aptos.transaction.build.simple({
    sender: account.accountAddress.toStringLong(),
    withFeePayer: false,
    data: {
      function: payload.function,
      typeArguments: payload.typeArguments,
      functionArguments: payload.functionArguments,
    },
  });
  const committed = await aptos.signAndSubmitTransaction({ signer: account, transaction: txn });
  await aptos.waitForTransaction({ transactionHash: committed.hash });
  console.log("Leave requested. Tx:", committed.hash);
}

async function balance(account, tableAddress) {
  const res = await aptos.view({
    payload: {
      function: `${POKER_MODULE}::player_balance`,
      typeArguments: [],
      functionArguments: [tableAddress, account.accountAddress.toStringLong()],
    },
  });
  console.log("Chip balance:", res);
}

async function main() {
  const { account } = await getPlayerAddress();

  if (cmd === "enter") {
    const amount = BigInt(amountStr || "0");
    if (amount <= 0n) {
      console.error("Provide amount_octas > 0");
      process.exit(1);
    }
    await enterTable(account, tableAddr, amount.toString());
  } else if (cmd === "leave") {
    await requestLeave(account, tableAddr);
  } else if (cmd === "balance") {
    await balance(account, tableAddr);
  }
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});

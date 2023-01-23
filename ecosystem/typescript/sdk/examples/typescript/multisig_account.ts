/* eslint-disable no-console */

import dotenv from "dotenv";
dotenv.config();

import { AptosClient, AptosAccount, FaucetClient, BCS, TxnBuilderTypes } from "aptos";
import { sha3_256 as sha3Hash } from "@noble/hashes/sha3";
import { aptosCoinStore } from "./common";
import assert from "assert";

const NODE_URL = process.env.APTOS_NODE_URL || "https://fullnode.devnet.aptoslabs.com";
const FAUCET_URL = process.env.APTOS_FAUCET_URL || "https://faucet.devnet.aptoslabs.com";

const { AccountAddress, EntryFunction, MultiSig, TransactionPayloadMultisig } = TxnBuilderTypes;

/**
 * This code example demonstrates the new multisig account module and transaction execution flow.
 */
(async () => {
  const client = new AptosClient(NODE_URL);
  const faucetClient = new FaucetClient(NODE_URL, FAUCET_URL);

  // Create and fund 3 accounts that will be the owners of the multisig account.
  const owner1 = new AptosAccount();
  const owner2 = new AptosAccount();
  const owner3 = new AptosAccount();
  await faucetClient.fundAccount(owner1.address(), 100_000_000);
  await faucetClient.fundAccount(owner2.address(), 100_000_000);
  await faucetClient.fundAccount(owner3.address(), 100_000_000);

  // Create a 2-of-3 multisig account.
  const createMultisig = await client.generateTransaction(owner1.address(), {
    function: "0x1::multisig_account::create_with_owners",
    type_arguments: [],
    arguments: [[owner2.address().hex(), owner3.address().hex()], 2],
  });
  await client.generateSignSubmitWaitForTransaction(owner1, createMultisig.payload);

  // Find the multisig account address.
  let ownedMultisigAccounts = await client.getAccountResource(
    owner1.address(),
    "0x1::multisig_account::OwnedMultisigAccounts",
  );
  const multisigAddress = (ownedMultisigAccounts?.data as any).multisig_accounts[0];

  // Fund the multisig account for transfers.
  await faucetClient.fundAccount(multisigAddress, 100_000_000);

  // Create a multisig transaction to send 1_000_000 coins to an account.
  const recipient = new AptosAccount();
  const transferTxPayload = EntryFunction.natural(
    "0x1::aptos_account",
    "transfer",
    [],
    [BCS.bcsToBytes(AccountAddress.fromHex(recipient.address())), BCS.bcsSerializeUint64(1_000_000)],
  );
  const createMultisigTx = await client.generateTransaction(owner2.address(), {
    function: "0x1::multisig_account::create_transaction",
    type_arguments: [],
    arguments: [multisigAddress, BCS.bcsToBytes(transferTxPayload)],
  });
  await client.generateSignSubmitWaitForTransaction(owner2, createMultisigTx.payload);

  // Owner 1 rejects and owner 3 approves the transaction.
  let rejectTx = await client.generateTransaction(owner1.address(), {
    function: "0x1::multisig_account::reject_transaction",
    type_arguments: [],
    arguments: [multisigAddress, 1],
  });
  await client.generateSignSubmitWaitForTransaction(owner1, rejectTx.payload);
  let approveTx = await client.generateTransaction(owner3.address(), {
    function: "0x1::multisig_account::approve_transaction",
    type_arguments: [],
    arguments: [multisigAddress, 1],
  });
  await client.generateSignSubmitWaitForTransaction(owner3, approveTx.payload);

  // Owner 2 can now execute the transactions as it already has 2 approvals (from owners 2 and 3).
  // We'll simulate the tx first just to try it out.
  const multisigTxExecution = new TransactionPayloadMultisig(new MultiSig(AccountAddress.fromHex(multisigAddress)));
  // We're not doing anything with the simulation response. This is just for demo purposes.
  const [_simulationResp] = await client.simulateTransaction(
    owner2,
    await client.generateRawTransaction(owner2.address(), multisigTxExecution),
  );
  await client.generateSignSubmitWaitForTransaction(owner2, multisigTxExecution);
  let accountResource = await client.getAccountResource(recipient.address(), aptosCoinStore);
  let balance = parseInt((accountResource?.data as any).coin.value);
  assert(balance === 1_000_000);

  //===========================================================================================
  // Create another multisig transaction to send 1_000_000 coins but use payload hash instead.
  const transferTxPayloadHash = sha3Hash.create();
  transferTxPayloadHash.update(BCS.bcsToBytes(transferTxPayload));
  const createMultisigTxWithHash = await client.generateTransaction(owner2.address(), {
    function: "0x1::multisig_account::create_transaction_with_hash",
    type_arguments: [],
    arguments: [multisigAddress, transferTxPayloadHash.digest()],
  });
  await client.generateSignSubmitWaitForTransaction(owner2, createMultisigTxWithHash.payload);
  rejectTx = await client.generateTransaction(owner1.address(), {
    function: "0x1::multisig_account::reject_transaction",
    type_arguments: [],
    arguments: [multisigAddress, 2],
  });
  await client.generateSignSubmitWaitForTransaction(owner1, rejectTx.payload);
  approveTx = await client.generateTransaction(owner3.address(), {
    function: "0x1::multisig_account::approve_transaction",
    type_arguments: [],
    arguments: [multisigAddress, 2],
  });

  await client.generateSignSubmitWaitForTransaction(owner3, approveTx.payload);
  const multisigTxExecution2 = new TransactionPayloadMultisig(
    new MultiSig(AccountAddress.fromHex(multisigAddress), transferTxPayload),
  );
  await client.generateSignSubmitWaitForTransaction(owner2, multisigTxExecution2);
  accountResource = await client.getAccountResource(recipient.address(), aptosCoinStore);
  balance = parseInt((accountResource?.data as any).coin.value);
  assert(balance === 2_000_000);
})();

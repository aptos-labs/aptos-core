/* eslint-disable no-console */

import dotenv from "dotenv";
dotenv.config();

import { AptosClient, AptosAccount, FaucetClient, BCS, TxnBuilderTypes } from "aptos";
import { sha3_256 as sha3Hash } from "@noble/hashes/sha3";
import { aptosCoinStore, FAUCET_URL, NODE_URL } from "./common";
import assert from "assert";

const { AccountAddress, EntryFunction, MultiSig, MultiSigTransactionPayload, TransactionPayloadMultisig } =
  TxnBuilderTypes;

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
  const owner4 = new AptosAccount();
  await faucetClient.fundAccount(owner1.address(), 100_000_000);
  await faucetClient.fundAccount(owner2.address(), 100_000_000);
  await faucetClient.fundAccount(owner3.address(), 100_000_000);

  // Step 1: Setup a 2-of-3 multisig account
  // ===========================================================================================
  const createMultisig = await client.generateTransaction(owner1.address(), {
    function: "0x1::multisig_account::create_with_owners",
    type_arguments: [],
    arguments: [[owner2.address().hex(), owner3.address().hex()], 2, ["Shaka"], [BCS.bcsSerializeStr("Bruh")]],
  });
  await client.generateSignSubmitWaitForTransaction(owner1, createMultisig.payload);
  // Find the multisig account address.
  let ownedMultisigAccounts = await client.getAccountResource(
    owner1.address(),
    "0x1::multisig_account::OwnedMultisigAccounts",
  );
  const multisigAddress = (ownedMultisigAccounts?.data as any).multisig_accounts[0];
  assert((await getSignatureThreshold(client, multisigAddress)) == 2);
  assert((await getNumberOfOwners(client, multisigAddress)) == 3);

  // Fund the multisig account for transfers.
  await faucetClient.fundAccount(multisigAddress, 100_000_000);

  // Step 2: Create a multisig transaction to send 1_000_000 coins to an account.
  // We'll be including the full payload to be stored on chain.
  // ===========================================================================================
  const recipient = new AptosAccount();
  const transferTxPayload = new MultiSigTransactionPayload(
    EntryFunction.natural(
      "0x1::aptos_account",
      "transfer",
      [],
      [BCS.bcsToBytes(AccountAddress.fromHex(recipient.address())), BCS.bcsSerializeUint64(1_000_000)],
    ),
  );
  const multisigTxExecution = new TransactionPayloadMultisig(
    new MultiSig(AccountAddress.fromHex(multisigAddress), transferTxPayload),
  );
  const [simulationResp] = await client.simulateTransaction(
    owner2,
    await client.generateRawTransaction(owner2.address(), multisigTxExecution),
  );
  assert(simulationResp.success);

  // Create the multisig tx on chain.
  const createMultisigTx = await client.generateTransaction(owner2.address(), {
    function: "0x1::multisig_account::create_transaction",
    type_arguments: [],
    arguments: [multisigAddress, BCS.bcsToBytes(transferTxPayload)],
  });
  await client.generateSignSubmitWaitForTransaction(owner2, createMultisigTx.payload);

  // Owner 1 rejects but owner 3 approves.
  await rejectAndApprove(client, owner1, owner3, multisigAddress, 1);

  // Owner 2 can now execute the transactions as it already has 2 approvals (from owners 2 and 3).
  await client.generateSignSubmitWaitForTransaction(owner2, multisigTxExecution);
  let accountResource = await client.getAccountResource(recipient.address(), aptosCoinStore);
  let balance = parseInt((accountResource?.data as any).coin.value);
  assert(balance === 1_000_000);

  // Step 3: Create another multisig transaction to send 1_000_000 coins but use payload hash instead.
  // ===========================================================================================
  const transferTxPayloadHash = sha3Hash.create();
  transferTxPayloadHash.update(BCS.bcsToBytes(transferTxPayload));
  const createMultisigTxWithHash = await client.generateTransaction(owner2.address(), {
    function: "0x1::multisig_account::create_transaction_with_hash",
    type_arguments: [],
    arguments: [multisigAddress, transferTxPayloadHash.digest()],
  });
  await client.generateSignSubmitWaitForTransaction(owner2, createMultisigTxWithHash.payload);
  await rejectAndApprove(client, owner1, owner3, multisigAddress, 2);

  const multisigTxExecution2 = new TransactionPayloadMultisig(
    new MultiSig(AccountAddress.fromHex(multisigAddress), transferTxPayload),
  );
  await client.generateSignSubmitWaitForTransaction(owner2, multisigTxExecution2);
  accountResource = await client.getAccountResource(recipient.address(), aptosCoinStore);
  balance = parseInt((accountResource?.data as any).coin.value);
  assert(balance === 2_000_000);

  // Step 4: Create 2 multisig transactions: one to add a new owner and another one to remove it.
  // ===========================================================================================
  const owner_4 = new AptosAccount();
  const addOwnerPayload = new MultiSigTransactionPayload(
    EntryFunction.natural(
      "0x1::multisig_account",
      "add_owner",
      [],
      [BCS.bcsToBytes(AccountAddress.fromHex(owner_4.address()))],
    ),
  );
  const addOwnerTx = await client.generateTransaction(owner2.address(), {
    function: "0x1::multisig_account::create_transaction",
    type_arguments: [],
    arguments: [multisigAddress, BCS.bcsToBytes(addOwnerPayload)],
  });
  await client.generateSignSubmitWaitForTransaction(owner2, addOwnerTx.payload);
  await rejectAndApprove(client, owner1, owner3, multisigAddress, 3);
  await client.generateSignSubmitWaitForTransaction(
    owner2,
    new TransactionPayloadMultisig(new MultiSig(AccountAddress.fromHex(multisigAddress))),
  );
  // The multisig account should now have 4 owners.
  assert((await getNumberOfOwners(client, multisigAddress)) == 4);

  const removeOwnerPayload = new MultiSigTransactionPayload(
    EntryFunction.natural(
      "0x1::multisig_account",
      "remove_owner",
      [],
      [BCS.bcsToBytes(AccountAddress.fromHex(owner_4.address()))],
    ),
  );
  const removeOwnerTx = await client.generateTransaction(owner2.address(), {
    function: "0x1::multisig_account::create_transaction",
    type_arguments: [],
    arguments: [multisigAddress, BCS.bcsToBytes(removeOwnerPayload)],
  });
  await client.generateSignSubmitWaitForTransaction(owner2, removeOwnerTx.payload);
  await rejectAndApprove(client, owner1, owner3, multisigAddress, 4);
  await client.generateSignSubmitWaitForTransaction(
    owner2,
    new TransactionPayloadMultisig(new MultiSig(AccountAddress.fromHex(multisigAddress))),
  );
  // The multisig account should now have 3 owners.
  assert((await getNumberOfOwners(client, multisigAddress)) == 3);

  // Step 5: Create a multisig transactions to change the signature threshold to 3-of-3.
  // ===========================================================================================
  const changeSigThresholdPayload = new MultiSigTransactionPayload(
    EntryFunction.natural("0x1::multisig_account", "update_signatures_required", [], [BCS.bcsSerializeUint64(3)]),
  );
  const changeSigThresholdTx = await client.generateTransaction(owner2.address(), {
    function: "0x1::multisig_account::create_transaction",
    type_arguments: [],
    arguments: [multisigAddress, BCS.bcsToBytes(changeSigThresholdPayload)],
  });
  await client.generateSignSubmitWaitForTransaction(owner2, changeSigThresholdTx.payload);
  await rejectAndApprove(client, owner1, owner3, multisigAddress, 5);
  await client.generateSignSubmitWaitForTransaction(
    owner2,
    new TransactionPayloadMultisig(new MultiSig(AccountAddress.fromHex(multisigAddress))),
  );
  // The multisig account should now be 3-of-3.
  assert((await getSignatureThreshold(client, multisigAddress)) == 3);
})();

const rejectAndApprove = async (
  client: AptosClient,
  owner1: AptosAccount,
  owner2: AptosAccount,
  multisigAddress: string,
  transactionId: number,
) => {
  let rejectTx = await client.generateTransaction(owner1.address(), {
    function: "0x1::multisig_account::reject_transaction",
    type_arguments: [],
    arguments: [multisigAddress, transactionId],
  });
  await client.generateSignSubmitWaitForTransaction(owner1, rejectTx.payload);
  let approveTx = await client.generateTransaction(owner2.address(), {
    function: "0x1::multisig_account::approve_transaction",
    type_arguments: [],
    arguments: [multisigAddress, transactionId],
  });
  await client.generateSignSubmitWaitForTransaction(owner2, approveTx.payload);
};

const getNumberOfOwners = async (client: AptosClient, multisigAddress: string): Promise<number> => {
  const multisigAccountResource = await client.getAccountResource(
    multisigAddress,
    "0x1::multisig_account::MultisigAccount",
  );
  return Number((multisigAccountResource.data as any).owners.length);
};

const getSignatureThreshold = async (client: AptosClient, multisigAddress: string): Promise<number> => {
  const multisigAccountResource = await client.getAccountResource(
    multisigAddress,
    "0x1::multisig_account::MultisigAccount",
  );
  return Number((multisigAccountResource.data as any).num_signatures_required);
};

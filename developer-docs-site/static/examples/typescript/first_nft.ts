// Copyright (c) The Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { FAUCET_URL, NODE_URL, accountBalance } from "./first_transaction";
import { AptosAccount, TxnBuilderTypes, BCS, AptosClient, HexString, FaucetClient } from "aptos";

//:!:>section_1
function serializeVectorBool(vecBool: boolean[]) {
  const serializer = new BCS.Serializer();
  serializer.serializeU32AsUleb128(vecBool.length);
  vecBool.forEach((el) => {
    serializer.serializeBool(el);
  });
  return serializer.getBytes();
}

const NUMBER_MAX: number = 9007199254740991;
const client = new AptosClient(NODE_URL);
/** Creates a new collection within the specified account */
async function createCollection(account: AptosAccount, name: string, description: string, uri: string) {
  const entryFunctionPayload = new TxnBuilderTypes.TransactionPayloadEntryFunction(
    TxnBuilderTypes.EntryFunction.natural(
      "0x3::token",
      "create_collection_script",
      [],
      [
        BCS.bcsSerializeStr(name),
        BCS.bcsSerializeStr(description),
        BCS.bcsSerializeStr(uri),
        BCS.bcsSerializeUint64(NUMBER_MAX),
        serializeVectorBool([false, false, false]),
      ],
    ),
  );

  const [{ sequence_number: sequenceNumber }, chainId] = await Promise.all([
    client.getAccount(account.address()),
    client.getChainId(),
  ]);

  const rawTxn = new TxnBuilderTypes.RawTransaction(
    TxnBuilderTypes.AccountAddress.fromHex(account.address()),
    BigInt(sequenceNumber),
    entryFunctionPayload,
    1000n,
    1n,
    BigInt(Math.floor(Date.now() / 1000) + 10),
    new TxnBuilderTypes.ChainId(chainId),
  );

  const bcsTxn = AptosClient.generateBCSTransaction(account, rawTxn);
  const pendingTxn = await client.submitSignedBCSTransaction(bcsTxn);
  await client.waitForTransaction(pendingTxn.hash);
}
//<:!:section_1

//:!:>section_2
async function createToken(
  account: AptosAccount,
  collection_name: string,
  name: string,
  description: string,
  supply: number | bigint,
  uri: string,
) {
  // Serializes empty arrays
  const serializer = new BCS.Serializer();
  serializer.serializeU32AsUleb128(0);

  const entryFunctionPayload = new TxnBuilderTypes.TransactionPayloadEntryFunction(
    TxnBuilderTypes.EntryFunction.natural(
      "0x3::token",
      "create_token_script",
      [],
      [
        BCS.bcsSerializeStr(collection_name),
        BCS.bcsSerializeStr(name),
        BCS.bcsSerializeStr(description),
        BCS.bcsSerializeUint64(supply),
        BCS.bcsSerializeUint64(NUMBER_MAX),
        BCS.bcsSerializeStr(uri),
        BCS.bcsToBytes(TxnBuilderTypes.AccountAddress.fromHex(account.address())),
        BCS.bcsSerializeUint64(0),
        BCS.bcsSerializeUint64(0),
        serializeVectorBool([false, false, false, false, false]),
        serializer.getBytes(),
        serializer.getBytes(),
        serializer.getBytes(),
      ],
    ),
  );

  const [{ sequence_number: sequenceNumber }, chainId] = await Promise.all([
    client.getAccount(account.address()),
    client.getChainId(),
  ]);

  const rawTxn = new TxnBuilderTypes.RawTransaction(
    TxnBuilderTypes.AccountAddress.fromHex(account.address()),
    BigInt(sequenceNumber),
    entryFunctionPayload,
    1000n,
    1n,
    BigInt(Math.floor(Date.now() / 1000) + 10),
    new TxnBuilderTypes.ChainId(chainId),
  );

  const bcsTxn = AptosClient.generateBCSTransaction(account, rawTxn);
  const pendingTxn = await client.submitSignedBCSTransaction(bcsTxn);
  await client.waitForTransaction(pendingTxn.hash);
}
//<:!:section_2

//:!:>section_4
async function offerToken(
  account: AptosAccount,
  receiver: HexString,
  creator: HexString,
  collection_name: string,
  token_name: string,
  amount: number,
) {
  const entryFunctionPayload = new TxnBuilderTypes.TransactionPayloadEntryFunction(
    TxnBuilderTypes.EntryFunction.natural(
      "0x3::token_transfers",
      "offer_script",
      [],
      [
        BCS.bcsToBytes(TxnBuilderTypes.AccountAddress.fromHex(receiver.hex())),
        BCS.bcsToBytes(TxnBuilderTypes.AccountAddress.fromHex(creator.hex())),
        BCS.bcsSerializeStr(collection_name),
        BCS.bcsSerializeStr(token_name),
        BCS.bcsSerializeUint64(0),
        BCS.bcsSerializeUint64(amount),
      ],
    ),
  );

  const [{ sequence_number: sequenceNumber }, chainId] = await Promise.all([
    client.getAccount(account.address()),
    client.getChainId(),
  ]);

  const rawTxn = new TxnBuilderTypes.RawTransaction(
    TxnBuilderTypes.AccountAddress.fromHex(account.address()),
    BigInt(sequenceNumber),
    entryFunctionPayload,
    1000n,
    1n,
    BigInt(Math.floor(Date.now() / 1000) + 10),
    new TxnBuilderTypes.ChainId(chainId),
  );

  const bcsTxn = AptosClient.generateBCSTransaction(account, rawTxn);
  const pendingTxn = await client.submitSignedBCSTransaction(bcsTxn);
  await client.waitForTransaction(pendingTxn.hash);
}
//<:!:section_4

//:!:>section_5
async function claimToken(
  account: AptosAccount,
  sender: HexString,
  creator: HexString,
  collection_name: string,
  token_name: string,
) {
  const entryFunctionPayload = new TxnBuilderTypes.TransactionPayloadEntryFunction(
    TxnBuilderTypes.EntryFunction.natural(
      "0x3::token_transfers",
      "claim_script",
      [],
      [
        BCS.bcsToBytes(TxnBuilderTypes.AccountAddress.fromHex(sender.hex())),
        BCS.bcsToBytes(TxnBuilderTypes.AccountAddress.fromHex(creator.hex())),
        BCS.bcsSerializeStr(collection_name),
        BCS.bcsSerializeStr(token_name),
        BCS.bcsSerializeUint64(0),
      ],
    ),
  );

  const [{ sequence_number: sequenceNumber }, chainId] = await Promise.all([
    client.getAccount(account.address()),
    client.getChainId(),
  ]);

  const rawTxn = new TxnBuilderTypes.RawTransaction(
    TxnBuilderTypes.AccountAddress.fromHex(account.address()),
    BigInt(sequenceNumber),
    entryFunctionPayload,
    1000n,
    1n,
    BigInt(Math.floor(Date.now() / 1000) + 10),
    new TxnBuilderTypes.ChainId(chainId),
  );

  const bcsTxn = AptosClient.generateBCSTransaction(account, rawTxn);
  const pendingTxn = await client.submitSignedBCSTransaction(bcsTxn);
  await client.waitForTransaction(pendingTxn.hash);
}
//<:!:section_5

async function cancelTokenOffer(
  account: AptosAccount,
  receiver: HexString,
  creator: HexString,
  token_creation_num: number,
) {
  const entryFunctionPayload = new TxnBuilderTypes.TransactionPayloadEntryFunction(
    TxnBuilderTypes.EntryFunction.natural(
      "0x3::token_transfers",
      "cancel_offer_script",
      [],
      [
        BCS.bcsToBytes(TxnBuilderTypes.AccountAddress.fromHex(receiver.hex())),
        BCS.bcsToBytes(TxnBuilderTypes.AccountAddress.fromHex(creator.hex())),
        BCS.bcsSerializeUint64(token_creation_num),
      ],
    ),
  );

  const [{ sequence_number: sequenceNumber }, chainId] = await Promise.all([
    client.getAccount(account.address()),
    client.getChainId(),
  ]);

  const rawTxn = new TxnBuilderTypes.RawTransaction(
    TxnBuilderTypes.AccountAddress.fromHex(account.address()),
    BigInt(sequenceNumber),
    entryFunctionPayload,
    1000n,
    1n,
    BigInt(Math.floor(Date.now() / 1000) + 10),
    new TxnBuilderTypes.ChainId(chainId),
  );

  const bcsTxn = AptosClient.generateBCSTransaction(account, rawTxn);
  const pendingTxn = await client.submitSignedBCSTransaction(bcsTxn);
  await client.waitForTransaction(pendingTxn.hash);
}

//:!:>section_3
async function tableItem(handle: string, keyType: string, valueType: string, key: any): Promise<any> {
  const getTokenTableItemRequest = {
    key_type: keyType,
    value_type: valueType,
    key,
  };
  return client.getTableItem(handle, getTokenTableItemRequest);
}

async function getTokenBalance(
  owner: HexString,
  creator: HexString,
  collection_name: string,
  token_name: string,
): Promise<number> {
  const token_store = await client.getAccountResource(owner, "0x3::token::TokenStore");

  const token_data_id = {
    creator: creator.hex(),
    collection: collection_name,
    name: token_name,
  };

  const token_id = {
    token_data_id,
    property_version: "0",
  };

  const token = await tableItem(
    (token_store.data as any)["tokens"]["handle"],
    "0x3::token::TokenId",
    "0x3::token::Token",
    token_id,
  );

  return token.amount;
}

async function getTokenData(creator: HexString, collection_name: string, token_name: string): Promise<any> {
  const collections = await client.getAccountResource(creator, "0x3::token::Collections");

  const token_data_id = {
    creator: creator.hex(),
    collection: collection_name,
    name: token_name,
  };

  const token = await tableItem(
    (collections.data as any)["token_data"]["handle"],
    "0x3::token::TokenDataId",
    "0x3::token::TokenData",
    token_data_id,
  );
  return token;
}
//<:!:section_3

async function main() {
  const faucet_client = new FaucetClient(NODE_URL, FAUCET_URL);

  const alice = new AptosAccount();
  const bob = new AptosAccount();
  const collection_name = "Alice's cat collection";
  const token_name = "Alice's tabby";

  console.log("\n=== Addresses ===");
  console.log(`Alice: ${alice.address()}`);
  console.log(`Bob: ${bob.address()}`);

  await faucet_client.fundAccount(alice.address(), 5_000);
  await faucet_client.fundAccount(bob.address(), 5_000);

  console.log("\n=== Initial Balances ===");
  console.log(`Alice: ${await accountBalance(alice.address())}`);
  console.log(`Bob: ${await accountBalance(bob.address())}`);

  console.log("\n=== Creating Collection and Token ===");

  await createCollection(alice, collection_name, "Alice's simple collection", "https://aptos.dev");
  await createToken(
    alice,
    collection_name,
    token_name,
    "Alice's tabby",
    1,
    "https://aptos.dev/img/nyan.jpeg", //TODO: replace with uri link matching ERC1155 off-chain standard
  );

  let token_balance = await getTokenBalance(alice.address(), alice.address(), collection_name, token_name);
  console.log(`Alice's token balance: ${token_balance}`);
  const token_data = await getTokenData(alice.address(), collection_name, token_name);
  console.log(`Alice's token data: ${JSON.stringify(token_data)}`);

  console.log("\n=== Transferring the token to Bob ===");
  await offerToken(alice, bob.address(), alice.address(), collection_name, token_name, 1);
  await claimToken(bob, alice.address(), alice.address(), collection_name, token_name);

  token_balance = await getTokenBalance(alice.address(), alice.address(), collection_name, token_name);
  console.log(`Alice's token balance: ${token_balance}`);
  token_balance = await getTokenBalance(bob.address(), alice.address(), collection_name, token_name);
  console.log(`Bob's token balance: ${token_balance}`);
}

if (require.main === module) {
  main();
}

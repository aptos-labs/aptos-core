// Copyright (c) The Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { FAUCET_URL, NODE_URL, accountBalance } from "./first_transaction";
import { AptosAccount, TxnBuilderTypes, BCS, MaybeHexString, AptosClient, HexString, FaucetClient } from "aptos";

//:!:>section_1
const client = new AptosClient(NODE_URL);
/** Creates a new collection within the specified account */
async function createCollection(account: AptosAccount, name: string, description: string, uri: string) {
  const scriptFunctionPayload = new TxnBuilderTypes.TransactionPayloadScriptFunction(
    TxnBuilderTypes.ScriptFunction.natural(
      "0x1::token",
      "create_unlimited_collection_script",
      [],
      [BCS.bcsSerializeStr(name), BCS.bcsSerializeStr(description), BCS.bcsSerializeStr(uri)],
    ),
  );

  const [{ sequence_number: sequenceNumber }, chainId] = await Promise.all([
    client.getAccount(account.address()),
    client.getChainId(),
  ]);

  const rawTxn = new TxnBuilderTypes.RawTransaction(
    TxnBuilderTypes.AccountAddress.fromHex(account.address()),
    BigInt(sequenceNumber),
    scriptFunctionPayload,
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
  supply: number,
  uri: string,
) {
  const serializer = new BCS.Serializer();
  serializer.serializeBool(true);

  const scriptFunctionPayload = new TxnBuilderTypes.TransactionPayloadScriptFunction(
    TxnBuilderTypes.ScriptFunction.natural(
      "0x1::token",
      "create_unlimited_token_script",
      [],
      [
        BCS.bcsSerializeStr(collection_name),
        BCS.bcsSerializeStr(name),
        BCS.bcsSerializeStr(description),
        serializer.getBytes(),
        BCS.bcsSerializeUint64(supply),
        BCS.bcsSerializeStr(uri),
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
    scriptFunctionPayload,
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
  const scriptFunctionPayload = new TxnBuilderTypes.TransactionPayloadScriptFunction(
    TxnBuilderTypes.ScriptFunction.natural(
      "0x1::tokenTransfers",
      "offer_script",
      [],
      [
        BCS.bcsToBytes(TxnBuilderTypes.AccountAddress.fromHex(receiver.hex())),
        BCS.bcsToBytes(TxnBuilderTypes.AccountAddress.fromHex(creator.hex())),
        BCS.bcsSerializeStr(collection_name),
        BCS.bcsSerializeStr(token_name),
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
    scriptFunctionPayload,
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
  const scriptFunctionPayload = new TxnBuilderTypes.TransactionPayloadScriptFunction(
    TxnBuilderTypes.ScriptFunction.natural(
      "0x1::tokenTransfers",
      "claim_script",
      [],
      [
        BCS.bcsToBytes(TxnBuilderTypes.AccountAddress.fromHex(sender.hex())),
        BCS.bcsToBytes(TxnBuilderTypes.AccountAddress.fromHex(creator.hex())),
        BCS.bcsSerializeStr(collection_name),
        BCS.bcsSerializeStr(token_name),
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
    scriptFunctionPayload,
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
  const scriptFunctionPayload = new TxnBuilderTypes.TransactionPayloadScriptFunction(
    TxnBuilderTypes.ScriptFunction.natural(
      "0x1::tokenTransfers",
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
    scriptFunctionPayload,
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
  const token_store = await client.getAccountResource(creator, "0x1::token::TokenStore");

  const token_id = {
    creator: creator.hex(),
    collection: collection_name,
    name: token_name,
  };

  const token = await tableItem(
    (token_store.data as any)["tokens"]["handle"],
    "0x1::token::TokenId",
    "0x1::token::Token",
    token_id,
  );

  return token.data.value;
}

async function getTokenData(creator: HexString, collection_name: string, token_name: string): Promise<any> {
  const collections = await client.getAccountResource(creator, "0x1::token::Collections");

  const token_id = {
    creator: creator.hex(),
    collection: collection_name,
    name: token_name,
  };

  const token = await tableItem(
    (collections.data as any)["token_data"]["handle"],
    "0x1::token::TokenId",
    "0x1::token::TokenData",
    token_id,
  );
  return token.data;
}
//<:!:section_3

async function main() {
  const faucet_client = new FaucetClient(NODE_URL, FAUCET_URL);

  const alice = new AptosAccount();
  const bob = new AptosAccount();
  const collection_name = "Alice's";
  const token_name = "Alice's first token";

  console.log("\n=== Addresses ===");
  console.log(
    `Alice: ${alice.address()}. Key Seed: ${Buffer.from(alice.signingKey.secretKey).toString("hex").slice(0, 64)}`,
  );
  console.log(`Bob: ${bob.address()}. Key Seed: ${Buffer.from(bob.signingKey.secretKey).toString("hex").slice(0, 64)}`);

  await faucet_client.fundAccount(alice.address(), 5_000);
  await faucet_client.fundAccount(bob.address(), 5_000);

  console.log("\n=== Initial Balances ===");
  console.log(`Alice: ${await accountBalance(alice.address())}`);
  console.log(`Bob: ${await accountBalance(bob.address())}`);

  console.log("\n=== Creating Collection and Token ===");

  await createCollection(alice, collection_name, "Alice's simple collection", "https://aptos.dev");
  await createToken(alice, collection_name, token_name, "Alice's simple token", 1, "https://aptos.dev/img/nyan.jpeg");

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
  main().then((resp) => console.log(resp));
}

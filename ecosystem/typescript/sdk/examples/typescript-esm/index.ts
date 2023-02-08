/* eslint-disable no-console */
import {
  AptosClient,
  AptosAccount,
  FaucetClient,
  BCS,
  TxnBuilderTypes,
  TokenClient,
  IndexerClient,
  Provider,
  Network,
} from "aptos";
import assert from "assert";

const NODE_URL = process.env.APTOS_NODE_URL || "https://fullnode.devnet.aptoslabs.com";
const FAUCET_URL = process.env.APTOS_FAUCET_URL || "https://faucet.devnet.aptoslabs.com";
const INDEXER_URL = process.env.INDEXER_URL || "https://indexer-devnet.staging.gcp.aptosdev.com/v1/graphql";

export const aptosCoinStore = "0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>";

const {
  AccountAddress,
  TypeTagStruct,
  EntryFunction,
  StructTag,
  TransactionPayloadEntryFunction,
  RawTransaction,
  ChainId,
} = TxnBuilderTypes;

/**
 * This code example demonstrates the process of moving test coins from one account to another.
 */
(async () => {
  const client = new AptosClient(NODE_URL);
  const faucetClient = new FaucetClient(NODE_URL, FAUCET_URL);

  // Generates key pair for a new account
  const account1 = new AptosAccount();
  await faucetClient.fundAccount(account1.address(), 100_000_000);
  let resources = await client.getAccountResources(account1.address());
  let accountResource = resources.find((r: any) => r.type === aptosCoinStore);
  let balance = parseInt((accountResource?.data as any).coin.value);
  assert(balance === 100_000_000);
  console.log(`account1 coins: ${balance}. Should be 100_000_000!`);

  const account2 = new AptosAccount();
  // Creates the second account and fund the account with 0 AptosCoin
  await faucetClient.fundAccount(account2.address(), 0);
  resources = await client.getAccountResources(account2.address());
  accountResource = resources.find((r: any) => r.type === aptosCoinStore);
  balance = parseInt((accountResource?.data as any).coin.value);
  assert(balance === 0);
  console.log(`account2 coins: ${balance}. Should be 0!`);

  const token = new TypeTagStruct(StructTag.fromString("0x1::aptos_coin::AptosCoin"));

  // TS SDK support 3 types of transaction payloads: `EntryFunction`, `Script` and `Module`.
  // See https://aptos-labs.github.io/ts-sdk-doc/ for the details.
  const entryFunctionPayload = new TransactionPayloadEntryFunction(
    EntryFunction.natural(
      // Fully qualified module name, `AccountAddress::ModuleName`
      "0x1::coin",
      // Module function
      "transfer",
      // The coin type to transfer
      [token],
      // Arguments for function `transfer`: receiver account address and amount to transfer
      [BCS.bcsToBytes(AccountAddress.fromHex(account2.address())), BCS.bcsSerializeUint64(717)],
    ),
  );

  const [{ sequence_number: sequenceNumber }, chainId] = await Promise.all([
    client.getAccount(account1.address()),
    client.getChainId(),
  ]);

  // See class definiton here
  // https://aptos-labs.github.io/ts-sdk-doc/classes/TxnBuilderTypes.RawTransaction.html#constructor.
  const rawTxn = new RawTransaction(
    // Transaction sender account address
    AccountAddress.fromHex(account1.address()),
    BigInt(sequenceNumber),
    entryFunctionPayload,
    // Max gas unit to spend
    BigInt(10000),
    // Gas price per unit
    BigInt(100),
    // Expiration timestamp. Transaction is discarded if it is not executed within 10 seconds from now.
    BigInt(Math.floor(Date.now() / 1000) + 10),
    new ChainId(chainId),
  );

  // Sign the raw transaction with account1's private key
  const bcsTxn = AptosClient.generateBCSTransaction(account1, rawTxn);

  const transactionRes = await client.submitSignedBCSTransaction(bcsTxn);

  await client.waitForTransaction(transactionRes.hash);

  resources = await client.getAccountResources(account2.address());
  accountResource = resources.find((r: any) => r.type === aptosCoinStore);
  balance = parseInt((accountResource?.data as any).coin.value);
  assert(balance === 717);
  console.log(`account2 coins: ${balance}. Should be 717!`);

  // test IndexerClient
  const tokenClient = new TokenClient(client);
  const collectionName = "AliceCollection";
  const tokenName = "Alice Token";

  // Create collection and token on Alice's account
  await client.waitForTransaction(
    await tokenClient.createCollection(account1, collectionName, "Alice's new collection", "https://aptos.dev"),
    { checkSuccess: true },
  );

  await client.waitForTransaction(
    await tokenClient.createTokenWithMutabilityConfig(
      account1,
      collectionName,
      tokenName,
      "Alice's new token",
      1,
      "https://aptos.dev/img/nyan.jpeg",
      1000,
      account1.address(),
      1,
      0,
      ["TOKEN_BURNABLE_BY_OWNER"],
      [BCS.bcsSerializeBool(true)],
      ["bool"],
      [false, false, false, false, true],
    ),
    { checkSuccess: true },
  );

  let indexerClient = new IndexerClient(INDEXER_URL);
  const accountNFTs = await indexerClient.getAccountNFTs(account1.address().hex());
  console.log(
    `from indexer: account1 token name: ${accountNFTs.current_token_ownerships[0].current_token_data?.name}. Should be Alice Token!`,
  );

  const provider = new Provider(Network.DEVNET);
  const nfts = await provider.getAccountNFTs(account1.address().hex());
  console.log(
    `from provider: account1 token name: ${nfts.current_token_ownerships[0].current_token_data?.name}. Should be Alice Token!`,
  );
})();

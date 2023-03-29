require("dotenv").config();

const aptos = require("aptos");

const NODE_URL = process.env.APTOS_NODE_URL || "https://fullnode.devnet.aptoslabs.com";
const FAUCET_URL = process.env.APTOS_FAUCET_URL || "https://faucet.devnet.aptoslabs.com";

const aptosCoin = "0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>";

(async () => {
  const client = new aptos.AptosClient(NODE_URL);
  const faucetClient = new aptos.FaucetClient(NODE_URL, FAUCET_URL, null);
  const tokenClient = new aptos.TokenClient(client);

  const account1 = new aptos.AptosAccount();
  await faucetClient.fundAccount(account1.address(), 100_000_000);
  let resources = await client.getAccountResources(account1.address());
  let accountResource = resources.find((r) => r.type === aptosCoin);
  console.log(`account1 coins: ${accountResource.data.coin.value}. Should be 100_000_000!`);

  const account2 = new aptos.AptosAccount();
  await faucetClient.fundAccount(account2.address(), 0);
  resources = await client.getAccountResources(account2.address());
  accountResource = resources.find((r) => r.type === aptosCoin);
  console.log(`account2 coins: ${accountResource.data.coin.value}. Should be 0!`);

  const payload = {
    type: "entry_function_payload",
    function: "0x1::coin::transfer",
    type_arguments: ["0x1::aptos_coin::AptosCoin"],
    arguments: [account2.address().hex(), 717],
  };
  const txnRequest = await client.generateTransaction(account1.address(), payload);
  const signedTxn = await client.signTransaction(account1, txnRequest);
  const transactionRes = await client.submitTransaction(signedTxn);
  await client.waitForTransaction(transactionRes.hash);

  resources = await client.getAccountResources(account2.address());
  accountResource = resources.find((r) => r.type === aptosCoin);
  console.log(`account2 coins: ${accountResource.data.coin.value}. Should be 717!`);

  const provider = new aptos.Provider(aptos.Network.DEVNET);
  console.log("\n=== Checking if indexer devnet chainId same as fullnode chainId  ===");
  const indexerLedgerInfo = await provider.getIndexerLedgerInfo();
  const fullNodeChainId = await provider.getChainId();

  console.log(`\n fullnode chain id is: ${fullNodeChainId}, indexer chain id is: ${indexerLedgerInfo}`);
  if (indexerLedgerInfo.ledger_infos[0].chain_id !== fullNodeChainId) {
    console.log(`\n fullnode chain id and indexer chain id are not synced, skipping rest of tests`);
    return;
  }

  console.log("=== Creating account1's NFT Collection and Token ===");

  const collectionName = "Alice's";
  const tokenName = "Alice's first token";

  // Create the collection.
  // :!:>section_4
  const txnHash1 = await tokenClient.createCollection(
    account1,
    collectionName,
    "Alice's simple collection",
    "https://alice.com",
  ); // <:!:section_4
  await client.waitForTransaction(txnHash1, { checkSuccess: true });

  // Create a token in that collection.
  // :!:>section_5
  const txnHash2 = await tokenClient.createToken(
    account1,
    collectionName,
    tokenName,
    "Alice's simple token",
    1,
    "https://aptos.dev/img/nyan.jpeg",
  ); // <:!:section_5
  await client.waitForTransaction(txnHash2, { checkSuccess: true });

  const nfts = await provider.getAccountNFTs(account1.address().hex());
  console.log(`account1 current token ownership: ${nfts.current_token_ownerships[0].amount}. Should be 1`);
})();

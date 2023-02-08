require("dotenv").config();

const aptos = require("aptos");

const NODE_URL = process.env.APTOS_NODE_URL || "https://fullnode.devnet.aptoslabs.com";
const FAUCET_URL = process.env.APTOS_FAUCET_URL || "https://faucet.devnet.aptoslabs.com";

const aptosCoin = "0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>";

(async () => {
  const client = new aptos.AptosClient(NODE_URL);
  const faucetClient = new aptos.FaucetClient(NODE_URL, FAUCET_URL, null);

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

  const tokenClient = new aptos.TokenClient(client);
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
      [aptos.BCS.bcsSerializeBool(true)],
      ["bool"],
      [false, false, false, false, true],
    ),
    { checkSuccess: true },
  );

  let indexerClient = new aptos.IndexerClient("https://indexer-devnet.staging.gcp.aptosdev.com/v1/graphql");
  const response = await indexerClient.getAccountNFTs(account1.address().hex(), { limit: 20, offset: 0 });
  console.log(
    `account1 current token name: ${response.current_token_ownerships[0].current_token_data?.name}. Should be Alice Token!`,
  );
})();

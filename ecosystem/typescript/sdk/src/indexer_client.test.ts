import { AptosAccount } from "./aptos_account";
import { AptosClient } from "./aptos_client";
import { bcsSerializeBool } from "./bcs";
import { FaucetClient } from "./faucet_client";
import { IndexerClient } from "./indexer_client";
import { TokenClient } from "./token_client";

const aptosClient = new AptosClient("https://fullnode.devnet.aptoslabs.com");
const faucetClient = new FaucetClient("https://fullnode.devnet.aptoslabs.com", "https://faucet.devnet.aptoslabs.com");
const tokenClient = new TokenClient(aptosClient);
const alice = new AptosAccount();

describe("IndexerClient", () => {
  beforeAll(async () => {
    await faucetClient.fundAccount(alice.address(), 100000000);
  });

  it("gets account NFTs", async () => {
    const collectionName = "AliceCollection";
    const tokenName = "Alice Token";

    // Create collection and token on Alice's account
    await aptosClient.waitForTransaction(
      await tokenClient.createCollection(alice, collectionName, "Alice's simple collection", "https://aptos.dev"),
      { checkSuccess: true },
    );

    await aptosClient.waitForTransaction(
      await tokenClient.createTokenWithMutabilityConfig(
        alice,
        collectionName,
        tokenName,
        "Alice's simple token",
        1,
        "https://aptos.dev/img/nyan.jpeg",
        1000,
        alice.address(),
        1,
        0,
        ["TOKEN_BURNABLE_BY_OWNER"],
        [bcsSerializeBool(true)],
        ["bool"],
        [false, false, false, false, true],
      ),
      { checkSuccess: true },
    );

    let connection = new IndexerClient("https://indexer-devnet.staging.gcp.aptosdev.com/v1/graphql");
    const response = await connection.getAccountNFTs(alice.address().hex());

    expect(response[0]).toMatchObject({
      name: "Alice Token",
      collection_name: "AliceCollection",
      table_type: "0x3::token::TokenStore",
      property_version: 0,
      amount: 1,
    });
  });
});

import { AptosAccount } from "../../aptos_account";
import { AptosClient } from "../../providers/aptos_client";
import { bcsSerializeBool } from "../../bcs";
import { FaucetClient } from "../../providers/faucet_client";
import { IndexerClient } from "../../providers/indexer";
import { TokenClient } from "../../token_client";
import { API_TOKEN, longTestTimeout } from "../../utils/test_helper.test";
import { Network, NetworkToIndexerAPI, NetworkToNodeAPI, sleep } from "../../utils";

describe("Indexer", () => {
  const aptosClient = new AptosClient(NetworkToNodeAPI[Network.TESTNET]);
  const faucetClient = new FaucetClient(
    "https://fullnode.testnet.aptoslabs.com",
    "https://faucet.testnet.aptoslabs.com",
    { TOKEN: API_TOKEN },
  );
  const tokenClient = new TokenClient(aptosClient);
  const alice = new AptosAccount();
  const collectionName = "AliceCollection";
  const tokenName = "Alice Token";
  const indexerClient = new IndexerClient(NetworkToIndexerAPI[Network.TESTNET]);
  beforeAll(async () => {
    await faucetClient.fundAccount(alice.address(), 100000000);
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
  }, longTestTimeout);

  describe("get data", () => {
    jest.retryTimes(5);
    beforeEach(async () => {
      await sleep(1000);
    });
    it(
      "gets account NFTs",
      async () => {
        const accountNFTs = await indexerClient.getAccountNFTs(alice.address().hex());
        expect(accountNFTs.current_token_ownerships).toHaveLength(1);
        expect(accountNFTs.current_token_ownerships[0]).toHaveProperty("current_token_data");
        expect(accountNFTs.current_token_ownerships[0]).toHaveProperty("current_collection_data");
        expect(accountNFTs.current_token_ownerships[0].current_token_data?.name).toBe("Alice Token");
      },
      longTestTimeout,
    );

    it(
      "gets token activities",
      async () => {
        const accountNFTs = await indexerClient.getAccountNFTs(alice.address().hex());
        const tokenActivity = await indexerClient.getTokenActivities(
          accountNFTs.current_token_ownerships[0].current_token_data!.token_data_id_hash,
        );
        expect(tokenActivity.token_activities).toHaveLength(2);
        expect(tokenActivity.token_activities[0]).toHaveProperty("from_address");
        expect(tokenActivity.token_activities[0]).toHaveProperty("to_address");
      },
      longTestTimeout,
    );
  });
});

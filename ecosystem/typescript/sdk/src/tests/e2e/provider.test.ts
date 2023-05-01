import { AptosAccount } from "../../account/aptos_account";
import { AptosClient } from "../../providers/aptos_client";
import { bcsSerializeBool } from "../../bcs";
import { Provider } from "../../providers/provider";
import { FaucetClient } from "../../plugins/faucet_client";
import { TokenClient } from "../../plugins/token_client";
import { Network, NetworkToIndexerAPI, NetworkToNodeAPI, sleep } from "../../utils";
import { FAUCET_AUTH_TOKEN, longTestTimeout } from "../unit/test_helper.test";

describe("Provider", () => {
  const faucetClient = new FaucetClient(
    "https://fullnode.testnet.aptoslabs.com",
    "https://faucet.testnet.aptoslabs.com",
    { TOKEN: FAUCET_AUTH_TOKEN },
  );
  const alice = new AptosAccount();

  it("uses provided network as API", async () => {
    const provider = new Provider(Network.TESTNET);
    expect(provider.aptosClient.nodeUrl).toBe(NetworkToNodeAPI[Network.TESTNET]);
    expect(provider.indexerClient.endpoint).toBe(NetworkToIndexerAPI[Network.TESTNET]);
  });

  it("uses custom endpoints as API", async () => {
    const provider = new Provider({ fullnodeUrl: "full-node-url", indexerUrl: "indexer-url" });
    expect(provider.aptosClient.nodeUrl).toBe("full-node-url/v1");
    expect(provider.indexerClient.endpoint).toBe("indexer-url");
  });

  it("throws error when endpoint not provided", async () => {
    expect(() => {
      new Provider({ fullnodeUrl: "", indexerUrl: "" });
    }).toThrow("network is not provided");
  });

  describe("requests", () => {
    beforeAll(async () => {
      await faucetClient.fundAccount(alice.address(), 100000000);
    });

    describe("query full node", () => {
      it("gets genesis account from fullnode", async () => {
        const provider = new Provider(Network.TESTNET);
        const genesisAccount = await provider.getAccount("0x1");
        expect(genesisAccount.authentication_key.length).toBe(66);
        expect(genesisAccount.sequence_number).not.toBeNull();
      });
    });

    describe("query indexer", () => {
      const aptosClient = new AptosClient("https://fullnode.testnet.aptoslabs.com");
      const tokenClient = new TokenClient(aptosClient);
      const collectionName = "AliceCollection";
      const tokenName = "Alice Token";

      beforeAll(async () => {
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

      jest.retryTimes(5);
      beforeEach(async () => {
        await sleep(1000);
      });

      it("gets account NFTs from indexer", async () => {
        let provider = new Provider(Network.TESTNET);
        const accountNFTs = await provider.getAccountNFTs(alice.address().hex(), { limit: 20, offset: 0 });
        expect(accountNFTs.current_token_ownerships).toHaveLength(1);
        expect(accountNFTs.current_token_ownerships[0]).toHaveProperty("current_token_data");
        expect(accountNFTs.current_token_ownerships[0]).toHaveProperty("current_collection_data");
        expect(accountNFTs.current_token_ownerships[0].current_token_data?.name).toBe("Alice Token");
      });
    });
  });
});

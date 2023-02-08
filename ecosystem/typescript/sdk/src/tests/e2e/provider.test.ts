import { AptosAccount } from "../../aptos_account";
import { AptosClient } from "../../providers/aptos_client";
import { bcsSerializeBool } from "../../bcs";
import { Provider } from "../../providers/provider";
import { FaucetClient } from "../../providers/faucet_client";
import { TokenClient } from "../../token_client";
import { Network } from "../../utils/api-endpoints";

describe("Provider", () => {
  const faucetClient = new FaucetClient("https://fullnode.devnet.aptoslabs.com", "https://faucet.devnet.aptoslabs.com");
  const alice = new AptosAccount();

  it("uses provided network as API", async () => {
    const provider = new Provider(Network.TESTNET);
    expect(provider.aptosClient.nodeUrl).toBe("https://fullnode.testnet.aptoslabs.com/v1");
    expect(provider.indexerClient.endpoint).toBe("https://indexer-testnet.staging.gcp.aptosdev.com/v1/graphql");
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

  it("gets genesis account from fullnode", async () => {
    await faucetClient.fundAccount(alice.address(), 100000000);
    const provider = new Provider(Network.DEVNET);
    const genesisAccount = await provider.getAccount("0x1");
    expect(genesisAccount.authentication_key.length).toBe(66);
    expect(genesisAccount.sequence_number).not.toBeNull();
  });

  it("gets account NFTs from indexer", async () => {
    const aptosClient = new AptosClient("https://fullnode.devnet.aptoslabs.com");
    const tokenClient = new TokenClient(aptosClient);
    await faucetClient.fundAccount(alice.address(), 100000000);
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

    let provider = new Provider(Network.DEVNET);
    const accountNFTs = await provider.getAccountNFTs(alice.address().hex(), { limit: 20, offset: 0 });

    expect(accountNFTs.current_token_ownerships[0]).toHaveProperty("current_token_data");
    expect(accountNFTs.current_token_ownerships[0]).toHaveProperty("current_collection_data");
    expect(accountNFTs.current_token_ownerships[0].current_token_data?.name).toBe("Alice Token");
  });
});

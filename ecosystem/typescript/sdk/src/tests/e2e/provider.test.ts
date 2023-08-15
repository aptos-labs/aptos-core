import { Provider } from "../../providers/provider";
import { Network, NetworkToIndexerAPI, NetworkToNodeAPI } from "../../utils";

describe("Provider", () => {
  it("uses provided network as API", async () => {
    const providerDevnet = new Provider(Network.DEVNET);
    expect(providerDevnet.aptosClient.nodeUrl).toBe(NetworkToNodeAPI[Network.DEVNET]);
    expect(providerDevnet.indexerClient?.endpoint).toBe(NetworkToIndexerAPI[Network.DEVNET]);

    const providerTestnet = new Provider(Network.TESTNET);
    expect(providerTestnet.aptosClient.nodeUrl).toBe(NetworkToNodeAPI[Network.TESTNET]);
    expect(providerTestnet.indexerClient?.endpoint).toBe(NetworkToIndexerAPI[Network.TESTNET]);

    const providerMainnet = new Provider(Network.MAINNET);
    expect(providerMainnet.aptosClient.nodeUrl).toBe(NetworkToNodeAPI[Network.MAINNET]);
    expect(providerMainnet.indexerClient?.endpoint).toBe(NetworkToIndexerAPI[Network.MAINNET]);
  });

  it("uses custom endpoints as API", async () => {
    const provider = new Provider({ fullnodeUrl: "full-node-url", indexerUrl: "indexer-url" });
    expect(provider.aptosClient.nodeUrl).toBe("full-node-url/v1");
    expect(provider.indexerClient?.endpoint).toBe("indexer-url");
  });

  it("does not set indexer client when indexer url is not provided with a custom network", async () => {
    const provider = new Provider({ fullnodeUrl: "full-node-url" });
    expect(provider.aptosClient.nodeUrl).toBe("full-node-url/v1");
    expect(provider.indexerClient).toBe(undefined);
  });

  it("does not set indexer client when local netowrk is provided", async () => {
    const provider = new Provider(Network.LOCAL);
    expect(provider.aptosClient.nodeUrl).toBe(NetworkToNodeAPI[Network.LOCAL]);
    expect(provider.indexerClient).toBe(undefined);
  });

  it("includes static methods", async () => {
    expect(Provider).toHaveProperty("generateBCSTransaction");
    expect(Provider).toHaveProperty("generateBCSSimulation");
  });

  it("throws error when fullnode url is not provided", async () => {
    expect(() => {
      new Provider({ fullnodeUrl: "" });
    }).toThrow("fullnode url is not provided");
  });

  it("has AptosClient method defined", () => {
    const provider = new Provider(Network.DEVNET);
    expect(provider.getAccount).toBeDefined();
  });

  it("has IndexerClient method defined", () => {
    const provider = new Provider(Network.DEVNET);
    expect(provider.getAccountNFTs).toBeDefined();
  });
});

import { Provider } from "../../providers/provider";
import { Network, NetworkToIndexerAPI, NetworkToNodeAPI } from "../../utils";

describe("Provider", () => {
  it("uses provided network as API", async () => {
    const provider = new Provider(Network.DEVNET);
    expect(provider.aptosClient.nodeUrl).toBe(NetworkToNodeAPI[Network.DEVNET]);
    expect(provider.indexerClient.endpoint).toBe(NetworkToIndexerAPI[Network.DEVNET]);
  });

  it("uses custom endpoints as API", async () => {
    const provider = new Provider({ fullnodeUrl: "full-node-url", indexerUrl: "indexer-url" });
    expect(provider.aptosClient.nodeUrl).toBe("full-node-url/v1");
    expect(provider.indexerClient.endpoint).toBe("indexer-url");
  });

  it("includes static methods", async () => {
    expect(Provider).toHaveProperty("generateBCSTransaction");
    expect(Provider).toHaveProperty("generateBCSSimulation");
  });

  it("throws error when endpoint not provided", async () => {
    expect(() => {
      new Provider({ fullnodeUrl: "", indexerUrl: "" });
    }).toThrow("network is not provided");
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

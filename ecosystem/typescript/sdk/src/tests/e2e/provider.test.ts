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
    const provider = new Provider(Network.TESTNET);
    expect(provider.getAccount).toBeDefined();
  });

  it("has IndexerClient method defined", () => {
    const provider = new Provider(Network.TESTNET);
    expect(provider.getAccountNFTs).toBeDefined();
  });
});

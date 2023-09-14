import { AptosConfig } from "../../src";
import { AptosSettings } from "../../src/types";
import { Network, NetworkToFaucetAPI, NetworkToNodeAPI, NetworkToIndexerAPI } from "../../src/utils/api-endpoints";
import { AptosApiType } from "../../src/utils/const";

describe("aptos config", () => {
  test("it should set urls based on a local network", async () => {
    const settings: AptosSettings = {
      network: Network.LOCAL,
    };
    const aptosConfig = new AptosConfig(settings);
    expect(aptosConfig.network).toEqual("local");
    expect(aptosConfig.getRequestUrl(AptosApiType.FULLNODE)).toBe(NetworkToNodeAPI[Network.LOCAL]);
    expect(aptosConfig.getRequestUrl(AptosApiType.FAUCET)).toBe(NetworkToFaucetAPI[Network.LOCAL]);
    expect(aptosConfig.getRequestUrl(AptosApiType.INDEXER)).toBeUndefined();
  });

  test("it should set urls based on a given network", async () => {
    const settings: AptosSettings = {
      network: Network.TESTNET,
    };
    const aptosConfig = new AptosConfig(settings);
    expect(aptosConfig.network).toEqual("testnet");
    expect(aptosConfig.getRequestUrl(AptosApiType.FULLNODE)).toBe(NetworkToNodeAPI[Network.TESTNET]);
    expect(aptosConfig.getRequestUrl(AptosApiType.FAUCET)).toBe(NetworkToFaucetAPI[Network.TESTNET]);
    expect(aptosConfig.getRequestUrl(AptosApiType.INDEXER)).toBe(NetworkToIndexerAPI[Network.TESTNET]);
  });

  test("it should have undefined urls when network is custom and no urls provided", async () => {
    const settings: AptosSettings = {
      network: Network.CUSTOM,
    };
    const aptosConfig = new AptosConfig(settings);
    expect(aptosConfig.network).toBe("custom");
    expect(aptosConfig.fullnode).toBeUndefined();
    expect(aptosConfig.faucet).toBeUndefined();
    expect(aptosConfig.indexer).toBeUndefined();
  });

  test("getRequestUrl should throw when network is custom and no urls provided", async () => {
    const settings: AptosSettings = {
      network: Network.CUSTOM,
    };
    const aptosConfig = new AptosConfig(settings);
    expect(aptosConfig.network).toBe("custom");
    expect(() => aptosConfig.getRequestUrl(AptosApiType.FULLNODE)).toThrow();
    expect(() => aptosConfig.getRequestUrl(AptosApiType.FAUCET)).toThrow();
    expect(() => aptosConfig.getRequestUrl(AptosApiType.INDEXER)).toThrow();
  });

  test("it should set urls when network is custom and urls provided", async () => {
    const settings: AptosSettings = {
      network: Network.CUSTOM,
      fullnode: "my-fullnode-url",
      faucet: "my-faucet-url",
      indexer: "my-indexer-url",
    };
    const aptosConfig = new AptosConfig(settings);
    expect(aptosConfig.network).toBe("custom");
    expect(aptosConfig.fullnode).toBe("my-fullnode-url");
    expect(aptosConfig.faucet).toBe("my-faucet-url");
    expect(aptosConfig.indexer).toBe("my-indexer-url");
  });
});

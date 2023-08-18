import { Aptos, AptosConfig } from "../../src";
import { Network } from "../../src/utils/api-endpoints";

describe("aptos config", () => {
  test("it should set DEVNET network if network is not provided", async () => {
    const aptos = new Aptos();
    expect(aptos.config.network).toEqual("devnet");
    expect(aptos.config.fullnode).toEqual("https://fullnode.devnet.aptoslabs.com/v1");
    expect(aptos.config.faucet).toEqual("https://faucet.devnet.aptoslabs.com");
    expect(aptos.config.indexer).toEqual("https://indexer-devnet.staging.gcp.aptosdev.com/v1/graphql");
  });

  test("it should set urls based on the provided network", async () => {
    const settings: AptosConfig = {
      network: Network.TESTNET,
    };
    const aptos = new Aptos(settings);
    expect(aptos.config.network).toEqual("testnet");
    expect(aptos.config.fullnode).toEqual("https://fullnode.testnet.aptoslabs.com/v1");
    expect(aptos.config.faucet).toEqual("https://faucet.testnet.aptoslabs.com");
    expect(aptos.config.indexer).toEqual("https://indexer-testnet.staging.gcp.aptosdev.com/v1/graphql");
  });

  test("it should set urls based on a local network", async () => {
    const settings: AptosConfig = {
      network: Network.LOCAL,
    };
    const aptos = new Aptos(settings);
    expect(aptos.config.network).toEqual("local");
    expect(aptos.config.fullnode).toEqual("http://localhost:8080/v1");
    expect(aptos.config.faucet).toEqual("http://localhost:8081");
    expect(aptos.config.indexer).toBeUndefined();
  });

  test("it should have undefined urls when network is custom and no urls provided", async () => {
    const settings: AptosConfig = {
      network: Network.CUSTOM,
    };
    const aptos = new Aptos(settings);
    expect(aptos.config.network).toEqual("custom");
    expect(aptos.config.fullnode).toBeUndefined();
    expect(aptos.config.faucet).toBeUndefined();
    expect(aptos.config.indexer).toBeUndefined();
  });

  test("it should set urls when network is custom and urls provided", async () => {
    const settings: AptosConfig = {
      network: Network.CUSTOM,
      fullnode: "my-fullnode-url",
      faucet: "my-faucet-url",
      indexer: "my-indexer-url",
    };
    const aptos = new Aptos(settings);
    expect(aptos.config.network).toEqual("custom");
    expect(aptos.config.fullnode).toEqual("my-fullnode-url");
    expect(aptos.config.faucet).toEqual("my-faucet-url");
    expect(aptos.config.indexer).toEqual("my-indexer-url");
  });
});

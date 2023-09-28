import { AptosConfig, Aptos } from "../../../src";
import { Network } from "../../../src/utils/api-endpoints";

describe("transaction api", () => {
  test("it queries for the network estimated gas price", async () => {
    const config = new AptosConfig({ network: Network.LOCAL });
    const aptos = new Aptos(config);
    const data = await aptos.getGasPriceEstimation();
    expect(data).toHaveProperty("gas_estimate");
    expect(data).toHaveProperty("deprioritized_gas_estimate");
    expect(data).toHaveProperty("prioritized_gas_estimate");
  });

  test("it queries for transactions on the chain", async () => {
    // TODO - add tests once transaction submission is in
  });
});

import { Aptos, AptosConfig } from "../../../src";
import { Network } from "../../../src/utils/api-endpoints";

// TODO add account getTransactions tests once sdk v2 supports faucet (which needs transaction operation support)

describe("account api", () => {
  describe("throws when account address in invalid", () => {
    test("it throws with a short account address", async () => {
      const config = new AptosConfig({ network: Network.LOCAL });
      const aptos = new Aptos(config);
      expect(
        async () =>
          await aptos.account.getInfo({
            accountAddress: "ca843279e3427144cead5e4d5999a3d0ca843279e3427144cead5e4d5999a3d0",
          }),
      ).rejects.toThrow();
    });

    test("it throws when invalid account address", () => {
      const config = new AptosConfig({ network: Network.LOCAL });
      const aptos = new Aptos(config);
      expect(async () => await aptos.account.getInfo({ accountAddress: "0x123" })).rejects.toThrow();
    });
  });

  describe("fetch data with acount address as string", () => {
    test("it fetches account data", async () => {
      const config = new AptosConfig({ network: Network.LOCAL });
      const aptos = new Aptos(config);
      const data = await aptos.account.getInfo({
        accountAddress: "0x1",
      });
      expect(data).toHaveProperty("sequence_number");
      expect(data.sequence_number).toBe("0");
      expect(data).toHaveProperty("authentication_key");
      expect(data.authentication_key).toBe("0x0000000000000000000000000000000000000000000000000000000000000001");
    });

    test("it fetches account modules", async () => {
      const config = new AptosConfig({ network: Network.LOCAL });
      const aptos = new Aptos(config);
      const data = await aptos.account.getModules({
        accountAddress: "0x1",
      });
      expect(data.length).toBeGreaterThan(0);
    });

    test("it fetches account module", async () => {
      const config = new AptosConfig({ network: Network.LOCAL });
      const aptos = new Aptos(config);
      const data = await aptos.account.getModule({
        accountAddress: "0x1",
        moduleName: "coin",
      });
      expect(data).toHaveProperty("bytecode");
    });

    test("it fetches account resources", async () => {
      const config = new AptosConfig({ network: Network.LOCAL });
      const aptos = new Aptos(config);
      const data = await aptos.account.getResources({
        accountAddress: "0x1",
      });
      expect(data.length).toBeGreaterThan(0);
    });

    test("it fetches account resource", async () => {
      const config = new AptosConfig({ network: Network.LOCAL });
      const aptos = new Aptos(config);
      const data = await aptos.account.getResource({
        accountAddress: "0x1",
        resourceType: "0x1::account::Account",
      });
      expect(data).toHaveProperty("type");
    });
  });

  describe("fetch data with acount address as Uint8Array", () => {
    test("it fetches account data", async () => {
      const config = new AptosConfig({ network: Network.LOCAL });
      const aptos = new Aptos(config);
      const data = await aptos.account.getInfo({
        accountAddress: new Uint8Array([
          0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
        ]),
      });
      expect(data).toHaveProperty("sequence_number");
      expect(data.sequence_number).toBe("0");
      expect(data).toHaveProperty("authentication_key");
      expect(data.authentication_key).toBe("0x0000000000000000000000000000000000000000000000000000000000000001");
    });

    test("it fetches account modules", async () => {
      const config = new AptosConfig({ network: Network.LOCAL });
      const aptos = new Aptos(config);
      const data = await aptos.account.getModules({
        accountAddress: new Uint8Array([
          0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
        ]),
      });
      expect(data.length).toBeGreaterThan(0);
    });

    test("it fetches account module", async () => {
      const config = new AptosConfig({ network: Network.LOCAL });
      const aptos = new Aptos(config);
      const data = await aptos.account.getModule({
        accountAddress: new Uint8Array([
          0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
        ]),
        moduleName: "coin",
      });
      expect(data).toHaveProperty("bytecode");
    });

    test("it fetches account resources", async () => {
      const config = new AptosConfig({ network: Network.LOCAL });
      const aptos = new Aptos(config);
      const data = await aptos.account.getResources({
        accountAddress: new Uint8Array([
          0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
        ]),
      });
      expect(data.length).toBeGreaterThan(0);
    });

    test("it fetches account resource", async () => {
      const config = new AptosConfig({ network: Network.LOCAL });
      const aptos = new Aptos(config);
      const data = await aptos.account.getResource({
        accountAddress: "0x1",
        resourceType: "0x1::account::Account",
      });
      expect(data).toHaveProperty("type");
    });
  });
});

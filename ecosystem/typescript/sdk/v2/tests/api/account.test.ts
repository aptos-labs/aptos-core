import { Aptos } from "../../src";

const config = {
  network: "https://fullnode.testnet.aptoslabs.com/v1",
};

describe("account", () => {
  test("get account", async () => {
    const aptos = new Aptos(config);
    const account = await aptos.account.get("0x1");
    console.log(account);
  });

  test("submit txn", async () => {});
});

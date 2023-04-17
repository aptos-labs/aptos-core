import { AptosAccount } from "../../account";
import { AnsClient } from "../../plugins/ans_client";
import { Provider } from "../../providers";
import { Network } from "../../utils";
import { getFaucetClient, longTestTimeout, NODE_URL } from "../unit/test_helper.test";

const ANS_OWNER_ADDRESS = "0x585fc9f0f0c54183b039ffc770ca282ebd87307916c215a3e692f2f8e4305e82";
const alice = new AptosAccount();
const ACCOUNT_ADDRESS = alice.address().hex();

describe("ANS", () => {
  beforeAll(async () => {
    const faucetClient = getFaucetClient();
    await faucetClient.fundAccount(alice.address(), 100_000_000_000);
  }, longTestTimeout);

  test("fails to create a new ANS class instance", () => {
    const provider = new Provider({ fullnodeUrl: "full-node-url", indexerUrl: "indexer-url" });
    expect(() => new AnsClient(provider)).toThrow("Error: For custom providers, you must pass in a contract address");
  });

  test("creates a new ANS class instance", () => {
    const provider = new Provider({ fullnodeUrl: "full-node-url", indexerUrl: "indexer-url" });
    const ans_client = new AnsClient(provider, ANS_OWNER_ADDRESS);
    expect(ans_client).toHaveProperty("contractAddress");
  });

  test("sets the contract address to be the provided one", () => {
    const provider = new Provider({ fullnodeUrl: "full-node-url", indexerUrl: "indexer-url" });
    const ans_client = new AnsClient(provider, ANS_OWNER_ADDRESS);
    expect(ans_client.contractAddress).toEqual(ANS_OWNER_ADDRESS);
  });

  test("sets the contract address to be the matching node url", () => {
    const provider = new Provider(Network.TESTNET);
    const ans_client = new AnsClient(provider, ANS_OWNER_ADDRESS);
    expect(ans_client.contractAddress).toEqual("0x5f8fd2347449685cf41d4db97926ec3a096eaf381332be4f1318ad4d16a8497c");
  });

  test(
    "mint name",
    async () => {
      const provider = new Provider({ fullnodeUrl: NODE_URL, indexerUrl: NODE_URL });
      const ans = new AnsClient(provider, ANS_OWNER_ADDRESS);

      const txnHash = await ans.mintAptosName(alice, "alice");
      await provider.waitForTransactionWithResult(txnHash, { checkSuccess: true });
    },
    longTestTimeout,
  );

  test(
    "get name by address",
    async () => {
      const provider = new Provider({ fullnodeUrl: NODE_URL, indexerUrl: NODE_URL });
      const ans = new AnsClient(provider, ANS_OWNER_ADDRESS);

      const name = await ans.getPrimaryNameByAddress(ACCOUNT_ADDRESS);
      expect(name).toEqual("alice");
    },
    longTestTimeout,
  );

  test(
    "get address by name",
    async () => {
      const provider = new Provider({ fullnodeUrl: NODE_URL, indexerUrl: NODE_URL });
      const ans = new AnsClient(provider, ANS_OWNER_ADDRESS);

      const address = await ans.getAddressByName("alice");
      expect(address).toEqual(ACCOUNT_ADDRESS);
    },
    longTestTimeout,
  );

  test(
    "get address by name with .apt",
    async () => {
      const provider = new Provider({ fullnodeUrl: NODE_URL, indexerUrl: NODE_URL });
      const ans = new AnsClient(provider, ANS_OWNER_ADDRESS);

      const address = await ans.getAddressByName("alice.apt");
      expect(address).toEqual(ACCOUNT_ADDRESS);
    },
    longTestTimeout,
  );

  test(
    "get address by subdomain_name",
    async () => {
      const provider = new Provider({ fullnodeUrl: NODE_URL, indexerUrl: NODE_URL });
      const ans = new AnsClient(provider, ANS_OWNER_ADDRESS);

      const address = await ans.getAddressByName("sub.alice");
      expect(address).toBeNull;
    },
    longTestTimeout,
  );

  test(
    "get address by subdomain_name with .apt",
    async () => {
      const provider = new Provider({ fullnodeUrl: NODE_URL, indexerUrl: NODE_URL });
      const ans = new AnsClient(provider, ANS_OWNER_ADDRESS);

      const address = await ans.getAddressByName("sub.alice.apt");
      expect(address).toBeNull;
    },
    longTestTimeout,
  );

  test(
    "returns null for an invalid domain",
    async () => {
      const provider = new Provider({ fullnodeUrl: NODE_URL, indexerUrl: NODE_URL });
      const ans = new AnsClient(provider, ANS_OWNER_ADDRESS);

      const address = await ans.getAddressByName("alice-");
      expect(address).toBeNull;
    },
    longTestTimeout,
  );

  test(
    "returns null for an invalid subdomain",
    async () => {
      const provider = new Provider({ fullnodeUrl: NODE_URL, indexerUrl: NODE_URL });
      const ans = new AnsClient(provider, ANS_OWNER_ADDRESS);

      const address = await ans.getAddressByName("sub.alice.apt-");
      expect(address).toBeNull;
    },
    longTestTimeout,
  );
});

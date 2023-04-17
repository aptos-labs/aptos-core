import { AptosAccount } from "../../account";
import { AnsClient } from "../../plugins/ans_client";
import { Provider } from "../../providers";
import { HexString, Network } from "../../utils";
import { getFaucetClient, longTestTimeout, NODE_URL } from "../unit/test_helper.test";

const ANS_OWNER_ADDRESS = "0x585fc9f0f0c54183b039ffc770ca282ebd87307916c215a3e692f2f8e4305e82";
const ANS_OWNER_PK = "0x37368b46ce665362562c6d1d4ec01a08c8644c488690df5a17e13ba163e20221";
const alice = new AptosAccount();
const ACCOUNT_ADDRESS = alice.address().hex();
const DOMAIN_NAME = `alice${Math.floor(Math.random() * 100 + 1)}`;

describe("ANS", () => {
  beforeAll(async () => {
    const faucetClient = getFaucetClient();
    await faucetClient.fundAccount(alice.address(), 100_000_000_000);
    console.log(alice);
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
    "init reverse lookup registry for contract admin",
    async () => {
      const owner = new AptosAccount(new HexString(ANS_OWNER_PK).toUint8Array());
      const provider = new Provider({ fullnodeUrl: NODE_URL, indexerUrl: NODE_URL });
      const ans_client = new AnsClient(provider, ANS_OWNER_ADDRESS);
      const txnHash = await ans_client.initReverseLookupRegistry(owner);
      await provider.waitForTransactionWithResult(txnHash, { checkSuccess: true });
    },
    longTestTimeout,
  );

  test(
    "mint name",
    async () => {
      const provider = new Provider({ fullnodeUrl: NODE_URL, indexerUrl: NODE_URL });
      const ans = new AnsClient(provider, ANS_OWNER_ADDRESS);

      const txnHash = await ans.mintAptosName(alice, DOMAIN_NAME);
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
      expect(name).toEqual(DOMAIN_NAME);
    },
    longTestTimeout,
  );

  test(
    "get address by name",
    async () => {
      const provider = new Provider({ fullnodeUrl: NODE_URL, indexerUrl: NODE_URL });
      const ans = new AnsClient(provider, ANS_OWNER_ADDRESS);

      const address = await ans.getAddressByName(DOMAIN_NAME);
      expect(address).toEqual(ACCOUNT_ADDRESS);
    },
    longTestTimeout,
  );

  test(
    "get address by name with .apt",
    async () => {
      const provider = new Provider({ fullnodeUrl: NODE_URL, indexerUrl: NODE_URL });
      const ans = new AnsClient(provider, ANS_OWNER_ADDRESS);

      const address = await ans.getAddressByName(`${DOMAIN_NAME}.apt`);
      expect(address).toEqual(ACCOUNT_ADDRESS);
    },
    longTestTimeout,
  );

  test(
    "get address by subdomain_name",
    async () => {
      const provider = new Provider({ fullnodeUrl: NODE_URL, indexerUrl: NODE_URL });
      const ans = new AnsClient(provider, ANS_OWNER_ADDRESS);

      const address = await ans.getAddressByName(`sub.${DOMAIN_NAME}`);
      expect(address).toBeNull;
    },
    longTestTimeout,
  );

  test(
    "get address by subdomain_name with .apt",
    async () => {
      const provider = new Provider({ fullnodeUrl: NODE_URL, indexerUrl: NODE_URL });
      const ans = new AnsClient(provider, ANS_OWNER_ADDRESS);

      const address = await ans.getAddressByName(`sub.${DOMAIN_NAME}.apt`);
      expect(address).toBeNull;
    },
    longTestTimeout,
  );

  test(
    "returns null for an invalid domain",
    async () => {
      const provider = new Provider({ fullnodeUrl: NODE_URL, indexerUrl: NODE_URL });
      const ans = new AnsClient(provider, ANS_OWNER_ADDRESS);

      const address = await ans.getAddressByName(`${DOMAIN_NAME}-`);
      expect(address).toBeNull;
    },
    longTestTimeout,
  );

  test(
    "returns null for an invalid subdomain",
    async () => {
      const provider = new Provider({ fullnodeUrl: NODE_URL, indexerUrl: NODE_URL });
      const ans = new AnsClient(provider, ANS_OWNER_ADDRESS);

      const address = await ans.getAddressByName(`sub.${DOMAIN_NAME}.apt-`);
      expect(address).toBeNull;
    },
    longTestTimeout,
  );
});

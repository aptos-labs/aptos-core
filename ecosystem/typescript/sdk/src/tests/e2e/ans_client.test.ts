import { AptosAccount } from "../../account";
import { AccountAddress } from "../../aptos_types";
import { AnsClient } from "../../plugins/ans_client";
import { Provider } from "../../providers";
import { HexString, Network } from "../../utils";
import { ANS_OWNER_ADDRESS, ANS_OWNER_PK, getFaucetClient, longTestTimeout, NODE_URL } from "../unit/test_helper.test";

const alice = new AptosAccount();
const ACCOUNT_ADDRESS = AccountAddress.standardizeAddress(alice.address().hex());
// generate random name so we can run the test against local tesnet without the need to re-run it each time.
// This will produce a string anywhere between zero and 12 characters long, usually 11 characters, only lower-case and numbers
const DOMAIN_NAME = Math.random().toString(36).slice(2);
const SUBDOMAIN_NAME = Math.random().toString(36).slice(2);

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

  test("sets the contract address to be the one that matches the provided node url", () => {
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
    "mint subdomain name",
    async () => {
      const provider = new Provider({ fullnodeUrl: NODE_URL, indexerUrl: NODE_URL });
      const ans = new AnsClient(provider, ANS_OWNER_ADDRESS);

      const txnHash = await ans.mintAptosSubdomain(alice, SUBDOMAIN_NAME, DOMAIN_NAME);
      await provider.waitForTransactionWithResult(txnHash, { checkSuccess: true });

      const txnHashForSet = await ans.setSubdomainAddress(alice, SUBDOMAIN_NAME, DOMAIN_NAME, ACCOUNT_ADDRESS);
      await provider.waitForTransactionWithResult(txnHashForSet, { checkSuccess: true });
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
      const standardizeAddress = AccountAddress.standardizeAddress(address as string);
      expect(standardizeAddress).toEqual(ACCOUNT_ADDRESS);
    },
    longTestTimeout,
  );

  test(
    "get address by name with .apt",
    async () => {
      const provider = new Provider({ fullnodeUrl: NODE_URL, indexerUrl: NODE_URL });
      const ans = new AnsClient(provider, ANS_OWNER_ADDRESS);

      const address = await ans.getAddressByName(`${DOMAIN_NAME}.apt`);
      const standardizeAddress = AccountAddress.standardizeAddress(address as string);
      expect(standardizeAddress).toEqual(ACCOUNT_ADDRESS);
    },
    longTestTimeout,
  );

  test(
    "get address by subdomain_name",
    async () => {
      const provider = new Provider({ fullnodeUrl: NODE_URL, indexerUrl: NODE_URL });
      const ans = new AnsClient(provider, ANS_OWNER_ADDRESS);

      const address = await ans.getAddressByName(`${SUBDOMAIN_NAME}.${DOMAIN_NAME}`);
      const standardizeAddress = AccountAddress.standardizeAddress(address as string);
      expect(standardizeAddress).toEqual(ACCOUNT_ADDRESS);
    },
    longTestTimeout,
  );

  test(
    "get address by subdomain_name with .apt",
    async () => {
      const provider = new Provider({ fullnodeUrl: NODE_URL, indexerUrl: NODE_URL });
      const ans = new AnsClient(provider, ANS_OWNER_ADDRESS);

      const address = await ans.getAddressByName(`${SUBDOMAIN_NAME}.${DOMAIN_NAME}.apt`);
      const standardizeAddress = AccountAddress.standardizeAddress(address as string);
      expect(standardizeAddress).toEqual(ACCOUNT_ADDRESS);
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

      const address = await ans.getAddressByName(`${SUBDOMAIN_NAME}.${DOMAIN_NAME}.apt-`);
      expect(address).toBeNull;
    },
    longTestTimeout,
  );
});

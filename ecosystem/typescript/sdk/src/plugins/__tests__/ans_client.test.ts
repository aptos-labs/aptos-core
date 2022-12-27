import { AptosAccount } from "../../aptos_account";
import { AptosClient } from "../../aptos_client";
import { AnsClient } from "../ans_client";
import { getFaucetClient, longTestTimeout, NODE_URL } from "../../utils/test_helper.test";
import * as Gen from "../../generated/index";
import { HexString } from "../../hex_string";
import { TxnBuilderTypes } from "../../transaction_builder";
import fs from "fs";
import path from "path";

export const ans_owner_address = "0xdc710fee87bd16028864920d50a5e444560fcbf207850f1a68cea2d606825c7c";
jest.setTimeout(100000);
describe("AnsClient", () => {
  it("fails to create a new ANS class instance", () => {
    const client = new AptosClient(NODE_URL);
    expect(() => new AnsClient(client)).toThrow("Please provide a valid contract address");
  });

  it("creates a new ANS class instance", () => {
    const client = new AptosClient(NODE_URL);
    const ans_client = new AnsClient(client, ans_owner_address);
    expect(ans_client).toHaveProperty("contractAddress");
  });

  it("sets the contract address to be the provided one", () => {
    const client = new AptosClient(NODE_URL);
    const ans_client = new AnsClient(client, ans_owner_address);
    expect(ans_client.contractAddress).toEqual(ans_owner_address);
  });

  it("sets the contract address to be the matching node url ", () => {
    const client = new AptosClient("https://testnet.aptoslabs.com/v1/");
    const ans_client = new AnsClient(client);
    expect(ans_client.contractAddress).toEqual("0x5f8fd2347449685cf41d4db97926ec3a096eaf381332be4f1318ad4d16a8497c");
  });

  describe("ans client functions", () => {
    const faucetClient = getFaucetClient();
    const client = new AptosClient(NODE_URL);
    const owner = new AptosAccount(
      new HexString("0x4e72bf7404165543ef7881154db2b49f77df23142bdd1f0bd16ded86bf870eb6").toUint8Array(),
    );
    const alice = new AptosAccount();

    beforeAll(async () => {
      const packageMetadata = fs.readFileSync(path.join(__dirname, "./ans_module/", "package-metadata.bcs"));
      const compiledModules = [
        "utf8_utils.mv",
        "config.mv",
        "verify.mv",
        "token_helper.mv",
        "time_helper.mv",
        "price_model.mv",
        "domains.mv",
      ];

      const moduleDatas = compiledModules.map((module: string) => {
        return fs.readFileSync(path.join(__dirname, "./ans_module/", "bytecode_modules", module));
      });

      await faucetClient.fundAccount(owner.address(), 100_000_000);

      const txnHash = await client.publishPackage(
        owner,
        new HexString(packageMetadata.toString("hex")).toUint8Array(),
        moduleDatas.map(
          (moduleData: Buffer) => new TxnBuilderTypes.Module(new HexString(moduleData.toString("hex")).toUint8Array()),
        ),
      );

      await client.waitForTransaction(txnHash);

      await faucetClient.fundAccount(alice.address(), 100000000000000);

      const payload: Gen.TransactionPayload = {
        type: "entry_function_payload",
        function: `${owner.address().hex()}::domains::register_domain`,
        type_arguments: [],
        arguments: ["alice", 1],
      };

      const txnRequest = await client.generateTransaction(alice.address(), payload);
      const signedTxn = await client.signTransaction(alice, txnRequest);
      const transactionRes = await client.submitTransaction(signedTxn);
      await client.waitForTransactionWithResult(transactionRes.hash);
    }, longTestTimeout);

    it("gets name from address", async () => {
      const ans = new AnsClient(client, ans_owner_address);
      const address = await ans.getAddressByName("alice");
      expect(alice.address().hex()).toEqual(address);
    });
  });
});

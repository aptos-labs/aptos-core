import { AptosAccount } from "../../aptos_account";
import { AptosClient } from "../../aptos_client";
import { AnsClient } from "../ans_client";
import { getFaucetClient, NODE_URL } from "../../utils/test_helper.test";
import * as Gen from "../../generated/index";

const CONTRACT_ADDRESS = "";

test("get name from address", async () => {
  const client = new AptosClient(NODE_URL);

  const account1 = new AptosAccount();
  const faucetClient = getFaucetClient();

  await faucetClient.fundAccount(account1.address(), 100_000_000);

  const payload: Gen.TransactionPayload = {
    type: "entry_function_payload",
    function: `${CONTRACT_ADDRESS}::domains::register_domain`,
    type_arguments: [],
    arguments: ["account1", 1],
  };

  const txnRequest = await client.generateTransaction(account1.address(), payload);
  const signedTxn = await client.signTransaction(account1, txnRequest);
  const transactionRes = await client.submitTransaction(signedTxn);
  const txn = await client.waitForTransactionWithResult(transactionRes.hash);
  console.log(txn);

  const ans = new AnsClient(client);
  const address = ans.getAddressFromName("account1");
  console.log(address);
});

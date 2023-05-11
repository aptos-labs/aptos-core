import { AptosAccount } from "../../account";
import { bcsToBytes, bcsSerializeUint64 } from "../../bcs";
import { Provider } from "../../providers";
import { TxnBuilderTypes } from "../../transaction_builder";
import { getFaucetClient, longTestTimeout, PROVIDER_LOCAL_NETWORK_CONFIG } from "../../tests/unit/test_helper.test";
import { TransactionWorker } from "../transaction_worker";

const provider = new Provider(PROVIDER_LOCAL_NETWORK_CONFIG);

const sender = new AptosAccount();
const recipient = new AptosAccount();

const faucet = getFaucetClient();

describe("transactionWorker", () => {
  beforeAll(async () => {
    await faucet.fundAccount(sender.address(), 1000000000);
  });

  test(
    "index",
    (done) => {
      // Specify the number of assertions expected
      expect.assertions(1);

      // create 5 transactions
      const txn = new TxnBuilderTypes.TransactionPayloadEntryFunction(
        TxnBuilderTypes.EntryFunction.natural(
          "0x1::aptos_account",
          "transfer",
          [],
          [bcsToBytes(TxnBuilderTypes.AccountAddress.fromHex(recipient.address())), bcsSerializeUint64(5)],
        ),
      );
      const payloads = [...Array(5).fill(txn)];

      // start transactions worker
      const transactionWorker = new TransactionWorker(provider, sender);
      transactionWorker.start();

      // push transactions to queue
      for (const payload in payloads) {
        transactionWorker.push(payloads[payload]);
      }

      // stop transaction worker for testing purposes.
      setTimeout(async () => {
        transactionWorker.stop();
        const accountData = await provider.getAccount(sender.address());
        // call done() when all asynchronous operations are finished
        done();
        // expect sender sequence number to be 5
        expect(accountData.sequence_number).toBe("5");
      }, 1000 * 30);
    },
    longTestTimeout,
  );
});

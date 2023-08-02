import { AptosAccount } from "../../account";
import { bcsToBytes, bcsSerializeUint64 } from "../../bcs";
import { Provider } from "../../providers";
import { TxnBuilderTypes } from "../../transaction_builder";
import { getFaucetClient, longTestTimeout, PROVIDER_LOCAL_NETWORK_CONFIG } from "../../tests/unit/test_helper.test";
import { TransactionWorker, TransactionWorkerEvents } from "../transaction_worker";

const provider = new Provider(PROVIDER_LOCAL_NETWORK_CONFIG);

const sender = new AptosAccount();
const recipient = new AptosAccount();

const faucet = getFaucetClient();

describe("transactionWorker", () => {
  beforeAll(async () => {
    await faucet.fundAccount(sender.address(), 1000000000);
  });

  test(
    "throws when starting an already started worker",
    async () => {
      // start transactions worker
      const transactionWorker = new TransactionWorker(provider, sender);
      transactionWorker.start();
      expect(async () => {
        transactionWorker.start();
      }).rejects.toThrow(`worker has already started`);
    },
    longTestTimeout,
  );

  test(
    "throws when stopping an already stopped worker",
    async () => {
      // start transactions worker
      const transactionWorker = new TransactionWorker(provider, sender);
      transactionWorker.start();
      transactionWorker.stop();
      expect(async () => {
        transactionWorker.stop();
      }).rejects.toThrow(`worker has already stopped`);
    },
    longTestTimeout,
  );

  test(
    "adds transaction into the transactionsQueue",
    async () => {
      const transactionWorker = new TransactionWorker(provider, sender);
      transactionWorker.start();
      const txn = new TxnBuilderTypes.TransactionPayloadEntryFunction(
        TxnBuilderTypes.EntryFunction.natural(
          "0x1::aptos_account",
          "transfer",
          [],
          [bcsToBytes(TxnBuilderTypes.AccountAddress.fromHex(recipient.address())), bcsSerializeUint64(5)],
        ),
      );
      transactionWorker.push(txn).then(() => {
        transactionWorker.stop();
        expect(transactionWorker.transactionsQueue.queue).toHaveLength(1);
      });
    },
    longTestTimeout,
  );

  test(
    "submits 5 transactions to chain for a single account",
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

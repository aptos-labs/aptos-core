import {
  AptosAccount,
  TxnBuilderTypes,
  OptionalTransactionArgs,
  AptosClient,
  BCS,
  MaybeHexString,
  HexString,
  TransactionBuilderEd25519,
} from "aptos";
import { Timer } from "timer-node";

export type Transaction = {
  sender: AptosAccount;
  payload: TxnBuilderTypes.TransactionPayload;
  extraArgs?: OptionalTransactionArgs;
};

const MAX_GAS_AMOUNT_ALLOWED = BigInt(2000000);

/**
 * This class submits banch transactions.
 * If number of transactions is higher than the api max_batch_size config, we create `total_transactions` / `max_batch_size` transaction buffers.
 * For each buffer we create a signed-ready-to-be-submitted transaction.
 * To create each transaction, we fetch the current sender sequence number and maintaining a local sequence number that would be increased for every new transaction creation.
 * We then submit each buffer to the `/transactions/batch` endpoint.
 * We handle possible errors (for now, only `mempool_is_full` error is handled by re-submitting the failed buffer)
 */
export class BatchTransaction {
  private transactions: Transaction[] = [];
  private batchSize: number = 0;
  private sequenceNumber: BCS.Uint64 | undefined = undefined;
  //client = new AptosClient("http://0.0.0.0:8080/v1");
  private client = new AptosClient("https://fullnode.devnet.aptoslabs.com");
  private currentIndex: number = 0;
  private currentBuffer: Transaction[] = [];
  private latestTxnHash: string = "";
  private lastRefreshed: Date | undefined = undefined;

  private chainId: BCS.Uint8;
  private gasUnitPrice: BCS.Uint64;
  private maxGasAmount: BCS.Uint64;

  private timer: Timer = new Timer();

  async send(transactions: Transaction[], batchSize: number = 10) {
    this.timer.start();
    this.batchSize = batchSize;
    this.transactions = transactions;
    console.log(`submitting ${transactions.length} transactions`);
    if (this.transactions.length > this.batchSize) {
      this.currentBuffer = this.transactions.slice(this.currentIndex, this.currentIndex + this.batchSize);
      return this.sendInBatch();
    } else {
      const txns = await this.createTransactions(transactions);
      return this.client.submitBatchTransactions(txns);
    }
  }

  async sendInBatch(): Promise<any> {
    if (this.currentBuffer.length === 0) {
      this.timer.stop();
      console.log(this.timer.time());
      return this.latestTxnHash;
    }

    const txns = await this.createTransactions(this.currentBuffer);

    return this.client
      .submitBatchTransactions(txns)
      .then(async (data) => {
        if ((data as any).transaction_failures.length > 0) {
          // TODO - this is for the case all transactions are from the same user,
          // to handle different users we would need to process each index in the transaction_failures array
          const error = (data as any).transaction_failures[0].error;
          console.log("transaction_failures", JSON.stringify((data as any).transaction_failures, null, " "));
          switch (error.error_code) {
            case "mempool_is_full":
              console.log("sleeps");
              await this.sleep(10 * 1000); // 10 seconds
              // re-submit current failed buffer
              this.currentBuffer = this.currentBuffer.slice(
                (data as any).transaction_failures[0].transaction_index,
                (data as any).transaction_failures[0].transaction_index + this.batchSize,
              );
              // re-fetch the account sequence number
              await this.syncSequenceNumber(this.currentBuffer[0].sender);
              return this.sendInBatch();
            default:
              throw new Error(`Unexpected error ${JSON.stringify(error, null, " ")}`);
          }
        }
        this.currentIndex += this.batchSize;
        this.currentBuffer = this.transactions.slice(this.currentIndex, this.currentIndex + this.batchSize);
        return this.sendInBatch();
      })
      .catch((error) => {
        console.error("unexpected error", error);
      });
  }

  async createTransactions(transactions: Transaction[]) {
    const serializer = new BCS.Serializer();
    serializer.serializeU32AsUleb128(transactions.length);
    let result = new Uint8Array(serializer.getBytes());
    result.set(serializer.getBytes(), 0);
    for (let i = 0; i < transactions.length; i++) {
      const txn = transactions[i];
      const bcsTxn = await this.generateBscTxn(txn);
      result = new Uint8Array([...result, ...bcsTxn]);
    }
    return result;
  }

  async generateBscTxn(transaction: Transaction) {
    const txn = transaction;

    if (!this.sequenceNumber) {
      await this.syncSequenceNumber(txn.sender);
    }

    // 5 minutes cache
    if (this.lastRefreshed === undefined || new Date().getTime() - this.lastRefreshed.getTime() > 5 * 60 * 1000) {
      const { chainId, gasUnitPrice, maxGasAmount } = await this.getTransactionArgs(txn.sender.address());
      this.chainId = chainId;
      this.gasUnitPrice = gasUnitPrice;
      this.maxGasAmount = maxGasAmount;
      this.lastRefreshed = new Date();
    }

    // let sequenceNumber;
    // if (accountSequnceNumberMap.has(txn.sender.address())) {
    //   let currSeqNumber = accountSequnceNumberMap.get(txn.sender.address());
    //   currSeqNumber = currSeqNumber + BigInt(1);
    //   sequenceNumber = currSeqNumber;
    // } else {
    //   const { sequence_number } = await client.getAccount(txn.sender.address());
    //   sequenceNumber = BigInt(sequence_number);
    //   accountSequnceNumberMap.set(txn.sender.address(), sequenceNumber);
    // }

    const rawTransaction = new TxnBuilderTypes.RawTransaction(
      // Transaction sender account address
      TxnBuilderTypes.AccountAddress.fromHex(txn.sender.address()),
      this.sequenceNumber!,
      txn.payload,
      // Max gas unit to spend
      txn.extraArgs?.maxGasAmount ?? this.maxGasAmount,
      // Gas price per unit
      txn.extraArgs?.gasUnitPrice ?? this.gasUnitPrice,
      // Expiration timestamp. Transaction is discarded if it is not executed within 20 seconds from now.
      txn.extraArgs?.expireTimestamp ?? BigInt(Math.floor(Date.now() / 1000) + 20),
      new TxnBuilderTypes.ChainId(this.chainId),
    );
    //this.latestTxnHash = await this.getSignedTxnHash(rawTransaction,txn.sender);
    this.sequenceNumber!++;
    const bcsTxn = AptosClient.generateBCSTransaction(txn.sender, rawTransaction);
    this.latestTxnHash = this.getSignedTxnHash(rawTransaction, txn.sender);
    return bcsTxn;
  }

  getSignedTxnHash(rawTxn: TxnBuilderTypes.RawTransaction, account: AptosAccount): string {
    const txnBuilder = new TransactionBuilderEd25519((signingMessage: TxnBuilderTypes.SigningMessage) => {
      // @ts-ignore
      const sigHexStr = account.signBuffer(signingMessage);
      return new TxnBuilderTypes.Ed25519Signature(sigHexStr.toUint8Array());
    }, account.pubKey().toUint8Array());
    const userTxn = new TxnBuilderTypes.UserTransaction(txnBuilder.rawToSigned(rawTxn));
    return HexString.fromUint8Array(userTxn.hash()).hex();
  }

  async getTransactionArgs(senderAddress: MaybeHexString) {
    const [chainId, { gas_estimate: gasUnitPrice }, maxGasAmount] = await Promise.all([
      this.client.getChainId(),
      this.client.estimateGasPrice(),
      this.client.estimateMaxGasAmount(senderAddress),
    ]);

    return {
      chainId: chainId,
      gasUnitPrice: BigInt(gasUnitPrice),
      maxGasAmount: maxGasAmount < MAX_GAS_AMOUNT_ALLOWED ? maxGasAmount : MAX_GAS_AMOUNT_ALLOWED,
    };
  }

  async syncSequenceNumber(account: AptosAccount) {
    const { sequence_number } = await this.client.getAccount(account.address());
    this.sequenceNumber = BigInt(sequence_number);
  }

  async sleep(timeMs: number): Promise<null> {
    return new Promise((resolve) => {
      setTimeout(resolve, timeMs);
    });
  }
}

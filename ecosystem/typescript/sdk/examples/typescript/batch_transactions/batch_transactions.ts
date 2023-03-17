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

export type Transaction = {
  sender: AptosAccount;
  payload: TxnBuilderTypes.TransactionPayload;
  extraArgs?: OptionalTransactionArgs;
};

const MAX_GAS_AMOUNT_ALLOWED = BigInt(2000000);

/**
 * This class submits banch transactions.
 * If number of transactions are higher than the api max_batch_size config, we create `total_transactions` / `max_batch_size` transaction buffers.
 * For each buffer we create a signed-ready-to-be-submitted transaction.
 * To create each transaction, we fetch the current sender sequence number and maintaining a local sequence number that would be increased for every new transaction creation.
 * We then submit each buffer to the `/transactions/batch` endpoint.
 * We handle possible errors (for now, only `mempool_is_full` error is handled by re-submitting the failed buffer)
 */
export class BatchTransaction {
  transactions: Transaction[] = [];
  batchSize: number = 0;
  sequenceNumber: BCS.Uint64 | undefined = undefined;
  //client = new AptosClient("http://0.0.0.0:8080/v1");
  client = new AptosClient("https://fullnode.devnet.aptoslabs.com");
  currentIndex = 0;
  currentBuffer: Transaction[] = [];
  latestTxnHash: string = "";

  async send(transactions: Transaction[], batchSize: number = 10) {
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
      return this.latestTxnHash;
    }

    for (let i = 0; i < this.currentBuffer.length; i++) {
      console.log(`sending buffer no. ${i + 1}`);
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
                console.log("sleeps in", this.sequenceNumber);
                await this.sleep(10 * 1000); // 10 seconds
                // re-submit current failed buffer
                this.currentBuffer = this.currentBuffer.slice(
                  (data as any).transaction_failures[0].transaction_index,
                  (data as any).transaction_failures[0].transaction_index + this.batchSize,
                );
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
          console.error("error", error);
        });
    }
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
      const { sequence_number } = await this.client.getAccount(txn.sender.address());
      this.sequenceNumber = BigInt(sequence_number);
    }

    const { chainId, gasUnitPrice, maxGasAmount } = await this.getTransactionArgs(txn.sender.address());
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
      txn.extraArgs?.maxGasAmount ?? maxGasAmount,
      // Gas price per unit
      txn.extraArgs?.gasUnitPrice ?? gasUnitPrice,
      // Expiration timestamp. Transaction is discarded if it is not executed within 20 seconds from now.
      txn.extraArgs?.expireTimestamp ?? BigInt(Math.floor(Date.now() / 1000) + 20),
      new TxnBuilderTypes.ChainId(chainId),
    );
    //this.latestTxnHash = await this.getSignedTxnHash(rawTransaction,txn.sender);
    this.sequenceNumber++;
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

  async sleep(timeMs: number): Promise<null> {
    return new Promise((resolve) => {
      setTimeout(resolve, timeMs);
    });
  }
}

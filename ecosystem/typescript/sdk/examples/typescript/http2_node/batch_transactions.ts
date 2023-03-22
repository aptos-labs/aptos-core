import { AptosAccount, TxnBuilderTypes, OptionalTransactionArgs, AptosClient, BCS, MaybeHexString } from "aptos";
const { connect } = require("http2");

const MAX_GAS_AMOUNT_ALLOWED = BigInt(2000000);
const URL = "https://fullnode.testnet.aptoslabs.com";

/**
 * This class submits banch transactions.
 * If number of transactions is higher than the api max_batch_size config, we create `total_transactions` / `max_batch_size` transaction buffers.
 * For each buffer we create a signed-ready-to-be-submitted transaction.
 * To create each transaction, we fetch the current sender sequence number and maintaining a local sequence number that would be increased for every new transaction creation.
 * We then submit each buffer to the `/transactions/batch` endpoint.
 * We handle possible errors (for now, only `mempool_is_full` error is handled by re-submitting the failed buffer)
 */
export class BatchTransaction {
  private transaction: any;
  private sequenceNumber: BCS.Uint64 | undefined = undefined;
  private client = new AptosClient(URL);
  private account: AptosAccount | undefined = undefined;
  private lastRefreshed: Date | undefined = undefined;
  private extraArgs?: OptionalTransactionArgs;

  private chainId: BCS.Uint8;
  private gasUnitPrice: BCS.Uint64;
  private maxGasAmount: BCS.Uint64;

  private request: any = connect(URL);

  constructor(
    account: AptosAccount,
    transaction: TxnBuilderTypes.TransactionPayload,
    extraArgs?: OptionalTransactionArgs,
  ) {
    this.transaction = transaction;
    this.account = account;
    this.extraArgs = extraArgs;
  }

  async get(paths: string[]) {
    const requests = paths.map((path) => this.request.request({ ":path": path, "content-type": "application/json" }));
    const promises = requests.map((request) => {
      return new Promise((resolve, reject) => {
        request.on("response", (headers: any) => {
          let chunks = "";
          request.on("data", (chunk: any) => {
            chunks += chunk;
          });
          request.on("end", () => {
            const data = JSON.parse(chunks);
            resolve(data);
          });
        });
        request.on("error", reject);
        request.end();
      });
    });
    const data = await Promise.all(promises);
    this.request.close();
    return data;
  }

  // async send(transaction: Uint8Array) {
  //   return new Promise((resolve, reject) => {
  //     const req = this.request.request({
  //       ":method": "POST",
  //       ":path": "/v1/transactions",
  //       "content-type": "application/x.aptos.signed_transaction+bcs",
  //       "content-length": Buffer.byteLength(transaction),
  //     });

  //     let data = "";

  //     req.on("response", (headers: any, flags: any) => {
  //       //console.log("headers", headers);
  //     });

  //     req.on("data", (chunk: any) => {
  //       data += chunk;
  //     });

  //     req.on("end", () => {
  //       //console.log(`Request completed`, data);
  //       resolve(data);
  //     });

  //     req.on("error", (err: any) => {
  //       console.error(`Request failed: ${err}`);
  //       reject(err);
  //     });

  //     req.write(transaction);
  //     req.end();
  //   });
  // }

  async send(transactions: Uint8Array[]): Promise<any> {
    const promises = transactions.map(async (transaction) => {
      // Create a new stream for each request
      const stream = this.request.request({
        ":method": "POST",
        ":path": "/v1/transactions",
        "content-type": "application/x.aptos.signed_transaction+bcs",
        "content-length": Buffer.byteLength(transaction),
      });

      // Write the request body to the stream and end the stream
      stream.write(transaction);
      stream.end();

      // Return a promise that resolves when the response is received
      return new Promise((resolve, reject) => {
        let data = "";
        stream.on("response", (headers: any) => {});
        stream.on("data", (chunk: any) => {
          data += chunk;
        });
        stream.on("end", () => {
          resolve(data);
        });
        stream.on("error", (error: any) => {
          reject(error);
        });
      });
    });

    // Wait for all promises to complete and collect the responses
    const responses = await Promise.all(promises);

    // Close the session
    this.request.close();

    return responses;
  }

  async generateBscTxn(): Promise<Uint8Array | undefined> {
    if (!this.account) return;

    if (!this.sequenceNumber) {
      await this.syncSequenceNumber(this.account);
    }

    // 5 minutes cache
    if (this.lastRefreshed === undefined || new Date().getTime() - this.lastRefreshed.getTime() > 5 * 60 * 1000) {
      const { chainId, gasUnitPrice, maxGasAmount } = await this.getTransactionArgs(this.account.address());
      this.chainId = chainId;
      this.gasUnitPrice = gasUnitPrice;
      this.maxGasAmount = maxGasAmount;
      this.lastRefreshed = new Date();
    }
    const rawTransaction = new TxnBuilderTypes.RawTransaction(
      // Transaction sender account address
      TxnBuilderTypes.AccountAddress.fromHex(this.account.address()),
      this.sequenceNumber!,
      this.transaction,
      // Max gas unit to spend
      this.extraArgs?.maxGasAmount ?? this.maxGasAmount,
      // Gas price per unit
      this.extraArgs?.gasUnitPrice ?? this.gasUnitPrice,
      // Expiration timestamp. Transaction is discarded if it is not executed within 20 seconds from now.
      this.extraArgs?.expireTimestamp ?? BigInt(Math.floor(Date.now() / 1000) + 20),
      new TxnBuilderTypes.ChainId(this.chainId),
    );
    this.sequenceNumber!++;
    const bcsTxn = AptosClient.generateBCSTransaction(this.account, rawTransaction);
    return bcsTxn;
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
}

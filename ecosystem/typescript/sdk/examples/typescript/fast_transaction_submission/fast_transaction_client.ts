import {
  AptosClient,
  AptosAccount,
  BCS,
  TxnBuilderTypes,
  OptionalTransactionArgs,
  MaybeHexString,
  Types,
  ApiError,
} from "aptos";

const { RawTransaction, ChainId } = TxnBuilderTypes;

const MAX_GAS_AMOUNT_ALLOWED = BigInt(2000000);

type TTLCallback = (txn: Types.PendingTransaction) => void;

export class FastTransactionClient {
  private sequenceNumber: BCS.Uint64 | undefined = undefined;
  private lastRefreshed: Date | undefined = undefined;

  private chainId: BCS.Uint8;
  private gasUnitPrice: BCS.Uint64;
  private maxGasAmount: BCS.Uint64;

  private pendingTxns: Types.PendingTransaction[] = [];

  private ttlCallbacks: TTLCallback[] = [];

  constructor(private readonly client: AptosClient) {
    setInterval(this.checkTxnTTL.bind(this), 3 * 1000);
  }

  private async getTransactionArgs(senderAddress: MaybeHexString) {
    const [chainId, { gas_estimate: gasUnitPrice }, maxGasAmount] = await Promise.all([
      this.client.getChainId(),
      this.client.estimateGasPrice(),
      this.client.estimateMaxGasAmount(senderAddress),
    ]);

    this.chainId = chainId;
    this.gasUnitPrice = BigInt(gasUnitPrice);
    this.maxGasAmount = maxGasAmount < MAX_GAS_AMOUNT_ALLOWED ? maxGasAmount : MAX_GAS_AMOUNT_ALLOWED;
  }

  private async resyncSeqNumber(senderAddress: MaybeHexString) {
    const { sequence_number: sequenceNumber } = await this.client.getAccount(senderAddress);
    this.sequenceNumber = BigInt(sequenceNumber);
  }

  // When chain is congested, txn might be ttled. In such case, we need to resync the seq number.
  private async checkTxnTTL() {
    if (this.pendingTxns.length === 0) return;

    const first = this.pendingTxns[0];

    try {
      await this.client.getTransactionByHash(first.hash);
    } catch (e) {
      if (e instanceof ApiError) {
        if (e.errorCode === "transaction_not_found") {
          // Resync seq number
          await this.resyncSeqNumber(first.sender);

          this.ttlCallbacks.forEach((cb) => cb(first));
          this.pendingTxns.shift();

          this.checkTxnTTL();
          return;
        }
      }
      throw e;
    }
  }

  subscribeTTLedTxn(onTTL: TTLCallback) {
    this.ttlCallbacks.push(onTTL);
  }

  getSequenceNumber() {
    return this.sequenceNumber;
  }

  async submitTxn(
    sender: AptosAccount,
    payload: TxnBuilderTypes.TransactionPayload,
    extraArgs?: OptionalTransactionArgs,
  ): Promise<Types.PendingTransaction> {
    if (this.sequenceNumber === undefined) {
      await this.resyncSeqNumber(sender.address());
    }

    if (this.lastRefreshed === undefined || new Date().getTime() - this.lastRefreshed.getTime() > 5 * 60 * 1000) {
      await this.getTransactionArgs(sender.address());
      this.lastRefreshed = new Date();
    }

    const rawTxn = new RawTransaction(
      // Transaction sender account address
      TxnBuilderTypes.AccountAddress.fromHex(sender.address()),
      this.sequenceNumber!,
      payload,
      // Max gas unit to spend
      extraArgs?.maxGasAmount ?? this.maxGasAmount,
      // Gas price per unit
      extraArgs?.gasUnitPrice ?? this.gasUnitPrice,
      // Expiration timestamp. Transaction is discarded if it is not executed within 20 seconds from now.
      extraArgs?.expireTimestamp ?? BigInt(Math.floor(Date.now() / 1000) + 20),
      new ChainId(this.chainId),
    );

    const signedTxn = AptosClient.generateBCSTransaction(sender, rawTxn);

    try {
      this.sequenceNumber = this.sequenceNumber! + BigInt(1);

      const pendingTxn = await this.client.submitSignedBCSTransaction(signedTxn);
      this.pendingTxns.push(pendingTxn);

      return pendingTxn;
    } catch (e) {
      if (e instanceof ApiError) {
        const error = e;
        // Txn with same seq no already exists
        if (error.errorCode === "invalid_transaction_update") {
          if (error.message?.includes("Transaction already in mempool")) {
            // Contine with next sequnce number
            return this.submitTxn(sender, payload, extraArgs);
          }
        }

        // Txn submitted is too old
        if (error.errorCode === "vm_error" && error.message.includes("SEQUENCE_NUMBER_TOO_OLD")) {
          await this.resyncSeqNumber(sender.address());
          return this.submitTxn(sender, payload, extraArgs);
        }

        this.sequenceNumber = this.sequenceNumber! - BigInt(1);

        // Mempool is full or reached capacity for the account. This can be used as back pressure to instruct senders to
        // slow down. Therefore, rethrow here.
        if (error.errorCode === "mempool_is_full") {
          throw e;
        }
      }

      throw e;
    }
  }
}

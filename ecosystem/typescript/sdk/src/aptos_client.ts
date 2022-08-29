// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Memoize } from "typescript-memoize";
import { HexString, MaybeHexString } from "./hex_string";
import { fixNodeUrl, sleep } from "./util";
import { AptosAccount } from "./aptos_account";
import * as Gen from "./generated/index";
import {
  TxnBuilderTypes,
  TransactionBuilderEd25519,
  BCS,
  TransactionBuilderRemoteABI,
  RemoteABIBuilderConfig,
} from "./transaction_builder";

/**
 * Provides methods for retrieving data from Aptos node.
 * For more detailed API specification see {@link https://fullnode.devnet.aptoslabs.com/v1/spec}
 */
export class AptosClient {
  client: Gen.AptosGeneratedClient;

  /**
   * Build a client configured to connect to an Aptos node at the given URL.
   *
   * Note: If you forget to append `/v1` to the URL, the client constructor
   * will automatically append it. If you don't want this URL processing to
   * take place, set doNotFixNodeUrl to true.
   *
   * @param nodeUrl URL of the Aptos Node API endpoint.
   * @param config Additional configuration options for the generated Axios client.
   */
  constructor(nodeUrl: string, config?: Partial<Gen.OpenAPIConfig>, doNotFixNodeUrl: boolean = false) {
    if (!nodeUrl) {
      throw new Error("Node URL cannot be empty.");
    }
    const conf = config === undefined || config === null ? {} : { ...config };
    if (doNotFixNodeUrl) {
      conf.BASE = nodeUrl;
    } else {
      conf.BASE = fixNodeUrl(nodeUrl);
    }
    this.client = new Gen.AptosGeneratedClient(conf);
  }

  /**
   * Queries an Aptos account by address
   * @param accountAddress Hex-encoded 32 byte Aptos account address
   * @returns Core account resource, used for identifying account and transaction execution
   * @example An example of the returned account
   * ```
   * {
   *    sequence_number: "1",
   *    authentication_key: "0x5307b5f4bc67829097a8ba9b43dba3b88261eeccd1f709d9bde240fc100fbb69"
   * }
   * ```
   */
  async getAccount(accountAddress: MaybeHexString): Promise<Gen.AccountData> {
    return this.client.accounts.getAccount(HexString.ensure(accountAddress).hex());
  }

  /**
   * Queries transactions sent by given account
   * @param accountAddress Hex-encoded 32 byte Aptos account address
   * @param query Optional pagination object
   * @param query.start The start transaction version of the page. Default is the latest ledger version
   * @param query?.limit The max number of transactions should be returned for the page. Default is 25.
   * @returns An array of on-chain transactions, sent by account
   */
  async getAccountTransactions(
    accountAddress: MaybeHexString,
    query?: { start?: BigInt | number; limit?: number },
  ): Promise<Gen.Transaction[]> {
    return this.client.transactions.getAccountTransactions(
      HexString.ensure(accountAddress).hex(),
      query?.start?.toString(),
      query?.limit,
    );
  }

  /**
   * Queries modules associated with given account
   * @param accountAddress Hex-encoded 32 byte Aptos account address
   * @param query.version Specifies ledger version of transactions. By default latest version will be used
   * @returns Account modules array for a specific ledger version.
   * Module is represented by MoveModule interface. It contains module `bytecode` and `abi`,
   * which is JSON representation of a module
   */
  async getAccountModules(
    accountAddress: MaybeHexString,
    query?: { ledgerVersion?: BigInt | number },
  ): Promise<Gen.MoveModuleBytecode[]> {
    return this.client.accounts.getAccountModules(
      HexString.ensure(accountAddress).hex(),
      query?.ledgerVersion?.toString(),
    );
  }

  /**
   * Queries module associated with given account by module name
   * @param accountAddress Hex-encoded 32 byte Aptos account address
   * @param moduleName The name of the module
   * @param query.version Specifies ledger version of transactions. By default latest version will be used
   * @returns Specified module.
   * Module is represented by MoveModule interface. It contains module `bytecode` and `abi`,
   * which JSON representation of a module
   */
  async getAccountModule(
    accountAddress: MaybeHexString,
    moduleName: string,
    query?: { ledgerVersion?: BigInt | number },
  ): Promise<Gen.MoveModuleBytecode> {
    return this.client.accounts.getAccountModule(
      HexString.ensure(accountAddress).hex(),
      moduleName,
      query?.ledgerVersion?.toString(),
    );
  }

  /**
   * Queries all resources associated with given account
   * @param accountAddress Hex-encoded 32 byte Aptos account address
   * @param query.version Specifies ledger version of transactions. By default latest version will be used
   * @returns Account resources for a specific ledger version
   * @example An example of an account resource
   * ```
   * {
   *    type: "0x1::AptosAccount::Coin",
   *    data: { value: 6 }
   * }
   * ```
   */
  async getAccountResources(
    accountAddress: MaybeHexString,
    query?: { ledgerVersion?: BigInt | number },
  ): Promise<Gen.MoveResource[]> {
    return this.client.accounts.getAccountResources(
      HexString.ensure(accountAddress).hex(),
      query?.ledgerVersion?.toString(),
    );
  }

  /**
   * Queries resource associated with given account by resource type
   * @param accountAddress Hex-encoded 32 byte Aptos account address
   * @param resourceType String representation of an on-chain Move struct type
   * @param query.version Specifies ledger version of transactions. By default latest version will be used
   * @returns Account resource of specified type and ledger version
   * @example An example of an account resource
   * ```
   * {
   *    type: "0x1::AptosAccount::Coin",
   *    data: { value: 6 }
   * }
   * ```
   */
  async getAccountResource(
    accountAddress: MaybeHexString,
    resourceType: Gen.MoveStructTag,
    query?: { ledgerVersion?: BigInt | number },
  ): Promise<Gen.MoveResource> {
    return this.client.accounts.getAccountResource(
      HexString.ensure(accountAddress).hex(),
      resourceType,
      query?.ledgerVersion?.toString(),
    );
  }

  /** Generates a signed transaction that can be submitted to the chain for execution. */
  static generateBCSTransaction(accountFrom: AptosAccount, rawTxn: TxnBuilderTypes.RawTransaction): Uint8Array {
    const txnBuilder = new TransactionBuilderEd25519((signingMessage: TxnBuilderTypes.SigningMessage) => {
      // @ts-ignore
      const sigHexStr = accountFrom.signBuffer(signingMessage);
      return new TxnBuilderTypes.Ed25519Signature(sigHexStr.toUint8Array());
    }, accountFrom.pubKey().toUint8Array());

    return txnBuilder.sign(rawTxn);
  }

  /** Generates a BCS transaction that can be submitted to the chain for simulation. */
  static generateBCSSimulation(accountFrom: AptosAccount, rawTxn: TxnBuilderTypes.RawTransaction): Uint8Array {
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    const txnBuilder = new TransactionBuilderEd25519((_signingMessage: TxnBuilderTypes.SigningMessage) => {
      // @ts-ignore
      const invalidSigBytes = new Uint8Array(64);
      return new TxnBuilderTypes.Ed25519Signature(invalidSigBytes);
    }, accountFrom.pubKey().toUint8Array());

    return txnBuilder.sign(rawTxn);
  }

  /** Generates a transaction request that can be submitted to produce a raw transaction that
   * can be signed, which upon being signed can be submitted to the blockchain
   * @param sender Hex-encoded 32 byte Aptos account address of transaction sender
   * @param payload Transaction payload. It depends on transaction type you want to send
   * @param options Options allow to overwrite default transaction options.
   * Defaults are:
   * ```bash
   *   {
   *     sender: senderAddress.hex(),
   *     sequence_number: account.sequence_number,
   *     max_gas_amount: "1000",
   *     gas_unit_price: "1",
   *     // Unix timestamp, in seconds + 10 seconds
   *     expiration_timestamp_secs: (Math.floor(Date.now() / 1000) + 10).toString(),
   *   }
   * ```
   * @returns A transaction object
   */
  async generateTransaction(
    sender: MaybeHexString,
    payload: Gen.EntryFunctionPayload,
    options?: Partial<Gen.SubmitTransactionRequest>,
  ): Promise<TxnBuilderTypes.RawTransaction> {
    const config: RemoteABIBuilderConfig = { sender };
    if (options?.sequence_number) {
      config.sequenceNumber = options.sequence_number;
    }

    if (options?.gas_unit_price) {
      config.gasUnitPrice = options.gas_unit_price;
    }

    if (options?.max_gas_amount) {
      config.maxGasAmount = options.max_gas_amount;
    }

    if (options?.expiration_timestamp_secs) {
      const timestamp = Number.parseInt(options.expiration_timestamp_secs, 10);
      config.expSecFromNow = timestamp - Math.floor(Date.now() / 1000);
    }

    const builder = new TransactionBuilderRemoteABI(this, config);
    return builder.build(payload.function, payload.type_arguments, payload.arguments);
  }

  /** Converts a transaction request produced by `generateTransaction` into a properly
   * signed transaction, which can then be submitted to the blockchain
   * @param accountFrom AptosAccount of transaction sender
   * @param rawTransaction A raw transaction generated by `generateTransaction` method
   * @returns A transaction, signed with sender account
   */
  // eslint-disable-next-line class-methods-use-this
  async signTransaction(
    accountFrom: AptosAccount,
    rawTransaction: TxnBuilderTypes.RawTransaction,
  ): Promise<Uint8Array> {
    return Promise.resolve(AptosClient.generateBCSTransaction(accountFrom, rawTransaction));
  }

  /**
   * Queries events by event key
   * @param eventKey Event key for an event stream. It is BCS serialized bytes
   * of `guid` field in the Move struct `EventHandle`
   * @returns Array of events assotiated with given key
   */
  async getEventsByEventKey(eventKey: string): Promise<Gen.Event[]> {
    return this.client.events.getEventsByEventKey(eventKey);
  }

  /**
   * Extracts event key from the account resource identified by the
   * `event_handle_struct` and `field_name`, then returns events identified by the event key
   * @param address Hex-encoded 32 byte Aptos account from which events are queried
   * @param eventHandleStruct String representation of an on-chain Move struct type.
   * (e.g. `0x1::Coin::CoinStore<0x1::aptos_coin::AptosCoin>`)
   * @param fieldName The field name of the EventHandle in the struct
   * @param query Optional query object
   * @param query.start The start sequence number in the EVENT STREAM, defaulting to the latest event.
   * The events are returned in the reverse order of sequence number
   * @param query?.limit The number of events to be returned for the page default is 5
   * @returns Array of events
   */
  async getEventsByEventHandle(
    address: MaybeHexString,
    eventHandleStruct: Gen.MoveStructTag,
    fieldName: string,
    query?: { start?: BigInt | number; limit?: number },
  ): Promise<Gen.Event[]> {
    return this.client.events.getEventsByEventHandle(
      HexString.ensure(address).hex(),
      eventHandleStruct,
      fieldName,
      query?.start?.toString(),
      query?.limit,
    );
  }

  /**
   * Submits a signed transaction to the transaction endpoint.
   * @param signedTxn A transaction, signed by `signTransaction` method
   * @returns Transaction that is accepted and submitted to mempool
   */
  async submitTransaction(signedTxn: Uint8Array): Promise<Gen.PendingTransaction> {
    return this.submitSignedBCSTransaction(signedTxn);
  }

  /** Submits a transaction with fake signature to the transaction simulation endpoint. */
  async simulateTransaction(
    accountFrom: AptosAccount,
    rawTransaction: TxnBuilderTypes.RawTransaction,
  ): Promise<Gen.UserTransaction[]> {
    const signedTxn = AptosClient.generateBCSSimulation(accountFrom, rawTransaction);
    return this.submitBCSSimulation(signedTxn);
  }

  /**
   * Submits a signed transaction to the the endpoint that takes BCS payload
   * @param signedTxn A BCS transaction representation
   * @returns Transaction that is accepted and submitted to mempool
   */
  async submitSignedBCSTransaction(signedTxn: Uint8Array): Promise<Gen.PendingTransaction> {
    // Need to construct a customized post request for transactions in BCS payload
    return this.client.request.request<Gen.PendingTransaction>({
      url: "/transactions",
      method: "POST",
      body: signedTxn,
      mediaType: "application/x.aptos.signed_transaction+bcs",
    });
  }

  /**
   * Submits a signed transaction to the the endpoint that takes BCS payload
   * @param signedTxn output of generateBCSSimulation()
   * @returns Simulation result in the form of UserTransaction
   */
  async submitBCSSimulation(bcsBody: Uint8Array): Promise<Gen.UserTransaction[]> {
    // Need to construct a customized post request for transactions in BCS payload
    return this.client.request.request<Gen.UserTransaction[]>({
      url: "/transactions/simulate",
      method: "POST",
      body: bcsBody,
      mediaType: "application/x.aptos.signed_transaction+bcs",
    });
  }

  /**
   * Queries on-chain transactions
   * @param query Optional pagination object
   * @param query.start The start transaction version of the page. Default is the latest ledger version
   * @param query?.limit The max number of transactions should be returned for the page. Default is 25
   * @returns Array of on-chain transactions
   */
  async getTransactions(query?: { start?: BigInt | number; limit?: number }): Promise<Gen.Transaction[]> {
    return this.client.transactions.getTransactions(query?.start?.toString(), query?.limit);
  }

  /**
   * @param txnHashOrVersion - Transaction hash should be hex-encoded bytes string with 0x prefix.
   * Transaction version is an uint64 number.
   * @returns Transaction from mempool or on-chain transaction
   */
  async getTransactionByHash(txnHash: string): Promise<Gen.Transaction> {
    return this.client.transactions.getTransactionByHash(txnHash);
  }

  /**
   * @param txnHashOrVersion - Transaction hash should be hex-encoded bytes string with 0x prefix.
   * Transaction version is an uint64 number.
   * @returns Transaction from mempool or on-chain transaction
   */
  async getTransactionByVersion(txnVersion: BigInt | number): Promise<Gen.Transaction> {
    return this.client.transactions.getTransactionByVersion(txnVersion.toString());
  }

  /**
   * Defines if specified transaction is currently in pending state
   * @param txnHash A hash of transaction
   *
   * To create a transaction hash:
   *
   * 1. Create hash message bytes: "Aptos::Transaction" bytes + BCS bytes of Transaction.
   * 2. Apply hash algorithm SHA3-256 to the hash message bytes.
   * 3. Hex-encode the hash bytes with 0x prefix.
   *
   * @returns `true` if transaction is in pending state and `false` otherwise
   */
  async transactionPending(txnHash: string): Promise<boolean> {
    try {
      const response = await this.client.transactions.getTransactionByHash(txnHash);
      return response.type === "pending_transaction";
    } catch (e) {
      if (e instanceof Gen.ApiError) {
        return e.status === 404;
      }
      throw e;
    }
  }

  /**
   * Wait for a transaction to move past pending state.
   *
   * There are 4 possible outcomes:
   * 1. Transaction is processed and successfully committed to the blockchain.
   * 2. Transaction is rejected for some reason, and is therefore not committed
   *    to the blockchain.
   * 3. Transaction is committed but execution failed, meaning no changes were
   *    written to the blockchain state.
   * 4. Transaction is not processed within the specified timeout.
   *
   * In case 1, this function resolves with the transaction response returned
   * by the API.
   *
   * In case 2, the function will throw an ApiError, likely with an HTTP status
   * code indicating some problem with the request (e.g. 400).
   *
   * In case 3, if `checkSuccess` is false (the default), this function returns
   * the transaction response just like in case 1, in which the `success` field
   * will be false. If `checkSuccess` is true, it will instead throw a
   * FailedTransactionError.
   *
   * In case 4, this function throws a WaitForTransactionError.
   *
   * @param txnHash The hash of a transaction previously submitted to the blockchain.
   * @param timeoutSecs Timeout in seconds. Defaults to 10 seconds.
   * @param checkSuccess See above. Defaults to false.
   * @returns See above.
   *
   * @example
   * ```
   * const rawTransaction = await this.generateRawTransaction(sender.address(), payload, extraArgs);
   * const bcsTxn = AptosClient.generateBCSTransaction(sender, rawTransaction);
   * const pendingTransaction = await this.submitSignedBCSTransaction(bcsTxn);
   * const transasction = await this.aptosClient.waitForTransactionWithResult(pendingTransaction.hash);
   * ```
   */
  async waitForTransactionWithResult(
    txnHash: string,
    extraArgs?: { timeoutSecs?: number; checkSuccess?: boolean },
  ): Promise<Gen.Transaction> {
    const timeoutSecs = extraArgs?.timeoutSecs ?? 10;
    const checkSuccess = extraArgs?.checkSuccess ?? false;

    let isPending = true;
    let count = 0;
    let lastTxn: Gen.Transaction | undefined;
    while (isPending) {
      if (count >= timeoutSecs) {
        break;
      }
      try {
        // eslint-disable-next-line no-await-in-loop
        lastTxn = await this.client.transactions.getTransactionByHash(txnHash);
        isPending = lastTxn.type === "pending_transaction";
        if (!isPending) {
          break;
        }
      } catch (e) {
        if (e instanceof Gen.ApiError) {
          if (e.status === 404) {
            isPending = true;
            // eslint-disable-next-line no-continue
            continue;
          }
          if (e.status >= 400) {
            throw e;
          }
        } else {
          throw e;
        }
      }
      // eslint-disable-next-line no-await-in-loop
      await sleep(1000);
      count += 1;
    }
    if (isPending) {
      throw new WaitForTransactionError(
        `Waiting for transaction ${txnHash} timed out after ${timeoutSecs} seconds`,
        lastTxn,
      );
    }
    if (!checkSuccess) {
      return lastTxn;
    }
    if (!(lastTxn as any)?.success) {
      throw new FailedTransactionError(
        `Transaction ${lastTxn.hash} committed to the blockchain but execution failed`,
        lastTxn,
      );
    }
    return lastTxn;
  }

  /**
   * This function works the same as `waitForTransactionWithResult` except it
   * doesn't return the transaction in those cases, it returns nothing. For
   * more information, see the documentation for `waitForTransactionWithResult`.
   */
  async waitForTransaction(
    txnHash: string,
    extraArgs?: { timeoutSecs?: number; checkSuccess?: boolean },
  ): Promise<void> {
    await this.waitForTransactionWithResult(txnHash, extraArgs);
  }

  /**
   * Queries the latest ledger information
   * @param params Request params
   * @returns Latest ledger information
   * @example Example of returned data
   * ```
   * {
   *   chain_id: 15,
   *   epoch: 6,
   *   ledgerVersion: "2235883",
   *   ledger_timestamp:"1654580922321826"
   * }
   * ```
   */
  async getLedgerInfo(): Promise<Gen.IndexResponse> {
    return this.client.general.getLedgerInfo();
  }

  /**
   * @returns Current chain id
   */
  @Memoize()
  async getChainId(): Promise<number> {
    const result = await this.getLedgerInfo();
    return result.chain_id;
  }

  /**
   * Gets a table item for a table identified by the handle and the key for the item.
   * Key and value types need to be passed in to help with key serialization and value deserialization.
   * @param handle A pointer to where that table is stored
   * @param data Object, that describes table item
   * @param data.key_type Move type of table key (e.g. `vector<u8>`)
   * @param data.value_type Move type of table value (e.g. `u64`)
   * @param data.key Value of table key
   * @param params Request params
   * @returns Table item value rendered in JSON
   */
  async getTableItem(
    handle: string,
    data: Gen.TableItemRequest,
    query?: { ledgerVersion?: BigInt | number },
  ): Promise<any> {
    const tableItem = await this.client.tables.getTableItem(handle, data, query?.ledgerVersion?.toString());
    return tableItem;
  }

  /**
   * Generates a raw transaction out of a transaction payload
   * @param accountFrom
   * @param payload
   * @param extraArgs
   * @returns
   */
  async generateRawTransaction(
    accountFrom: HexString,
    payload: TxnBuilderTypes.TransactionPayload,
    extraArgs?: { maxGasAmount?: BCS.Uint64; gasUnitPrice?: BCS.Uint64; expireTimestamp?: BCS.Uint64 },
  ): Promise<TxnBuilderTypes.RawTransaction> {
    const { maxGasAmount, gasUnitPrice, expireTimestamp } = {
      maxGasAmount: 2000n,
      gasUnitPrice: 1n,
      expireTimestamp: BigInt(Math.floor(Date.now() / 1000) + 20),
      ...extraArgs,
    };

    const [{ sequence_number: sequenceNumber }, chainId] = await Promise.all([
      this.getAccount(accountFrom),
      this.getChainId(),
    ]);

    return new TxnBuilderTypes.RawTransaction(
      TxnBuilderTypes.AccountAddress.fromHex(accountFrom),
      BigInt(sequenceNumber),
      payload,
      maxGasAmount,
      gasUnitPrice,
      expireTimestamp,
      new TxnBuilderTypes.ChainId(chainId),
    );
  }

  /**
   * Helper for generating, signing, and submitting a transaction.
   *
   * @param sender AptosAccount of transaction sender.
   * @param payload Transaction payload.
   * @param extraArgs Extra args for building the transaction payload.
   * @returns The transaction response from the API.
   */
  async generateSignSubmitTransaction(
    sender: AptosAccount,
    payload: TxnBuilderTypes.TransactionPayload,
    extraArgs?: {
      maxGasAmount?: BCS.Uint64;
      gasUnitPrice?: BCS.Uint64;
      expireTimestamp?: BCS.Uint64;
    },
  ): Promise<string> {
    // :!:>generateSignSubmitTransactionInner
    const rawTransaction = await this.generateRawTransaction(sender.address(), payload, extraArgs);
    const bcsTxn = AptosClient.generateBCSTransaction(sender, rawTransaction);
    const pendingTransaction = await this.submitSignedBCSTransaction(bcsTxn);
    return pendingTransaction.hash;
    // <:!:generateSignSubmitTransactionInner
  }

  /**
   * Publishes a move package. `packageMetadata` and `modules` can be generated with command
   * `aptos move compile --save-metadata [ --included-artifacts=<...> ]`.
   * @param sender
   * @param packageMetadata package metadata bytes
   * @param modules bytecodes of modules
   * @param extraArgs
   * @returns Transaction hash
   */
  async publishPackage(
    sender: AptosAccount,
    packageMetadata: BCS.Bytes,
    modules: BCS.Seq<TxnBuilderTypes.Module>,
    extraArgs?: {
      maxGasAmount?: BCS.Uint64;
      gasUnitPrice?: BCS.Uint64;
      expireTimestamp?: BCS.Uint64;
    },
  ): Promise<string> {
    const codeSerializer = new BCS.Serializer();
    BCS.serializeVector(modules, codeSerializer);

    const payload = new TxnBuilderTypes.TransactionPayloadEntryFunction(
      TxnBuilderTypes.EntryFunction.natural(
        "0x1::code",
        "publish_package_txn",
        [],
        [BCS.bcsSerializeBytes(packageMetadata), codeSerializer.getBytes()],
      ),
    );

    return this.generateSignSubmitTransaction(sender, payload, extraArgs);
  }

  /**
   * Helper for generating, submitting, and waiting for a transaction, and then
   * checking whether it was committed successfully. Under the hood this is just
   * `generateSignSubmitTransaction` and then `waitForTransactionWithResult`, see
   * those for information about the return / error semantics of this function.
   */
  async generateSignSubmitWaitForTransaction(
    sender: AptosAccount,
    payload: TxnBuilderTypes.TransactionPayload,
    extraArgs?: {
      maxGasAmount?: BCS.Uint64;
      gasUnitPrice?: BCS.Uint64;
      expireTimestamp?: BCS.Uint64;
      checkSuccess?: boolean;
      timeoutSecs?: number;
    },
  ): Promise<Gen.Transaction> {
    const txnHash = await this.generateSignSubmitTransaction(sender, payload, extraArgs);
    return this.waitForTransactionWithResult(txnHash, extraArgs);
  }
}

/**
 * This error is used by `waitForTransactionWithResult` when waiting for a
 * transaction times out.
 */
export class WaitForTransactionError extends Error {
  public readonly lastSubmittedTransaction: Gen.Transaction | undefined;

  constructor(message: string, lastSubmittedTransaction: Gen.Transaction | undefined) {
    super(message);
    this.lastSubmittedTransaction = lastSubmittedTransaction;
  }
}

/**
 * This error is used by `waitForTransactionWithResult` if `checkSuccess` is true.
 * See that function for more information.
 */
export class FailedTransactionError extends Error {
  public readonly transaction: Gen.Transaction;

  constructor(message: string, transaction: Gen.Transaction) {
    super(message);
    this.transaction = transaction;
  }
}

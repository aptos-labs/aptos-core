import { AxiosRequestConfig, AxiosResponse } from "axios";
import { Accounts } from "./api/Accounts";
import { Events } from "./api/Events";
import { Transactions } from "./api/Transactions";
import { HttpClient, RequestParams } from "./api/http-client";
import { HexString, MaybeHexString } from "./hex_string";
import { sleep } from "./util";
import { AptosAccount } from "./aptos_account";
import { Types } from "./types";
import { Tables } from "./api/Tables";
import { AptosError } from "./api/data-contracts";

export class RequestError extends Error {
  response?: AxiosResponse<any, Types.AptosError>;

  requestBody?: string;

  constructor(message?: string, response?: AxiosResponse<any, Types.AptosError>, requestBody?: string) {
    const data = JSON.stringify(response.data);
    const hostAndPath = [response.request?.host, response.request?.path].filter((e) => !!e).join("");
    super(`${message} - ${data}${hostAndPath ? ` @ ${hostAndPath}` : ""}${requestBody ? ` : ${requestBody}` : ""}`);
    this.response = response;
    this.requestBody = requestBody;
    Object.setPrototypeOf(this, new.target.prototype); // restore prototype chain
  }
}

export type AptosClientConfig = Omit<AxiosRequestConfig, "data" | "cancelToken" | "method">;

export function raiseForStatus<T>(
  expectedStatus: number,
  response: AxiosResponse<T, Types.AptosError>,
  requestContent?: any,
) {
  if (response.status !== expectedStatus) {
    if (requestContent) {
      throw new RequestError(response.statusText, response, JSON.stringify(requestContent));
    }
    throw new RequestError(response.statusText, response);
  }
}

export class AptosClient {
  nodeUrl: string;

  client: HttpClient;

  // These are the different routes
  accounts: Accounts;

  tables: Tables;

  events: Events;

  transactions: Transactions;

  constructor(nodeUrl: string, config?: AptosClientConfig) {
    this.nodeUrl = nodeUrl;

    // `withCredentials` ensures cookie handling
    this.client = new HttpClient<unknown>({
      withCredentials: false,
      baseURL: nodeUrl,
      validateStatus: () => true, // Don't explode here on error responses; let our code handle it
      ...(config || {}),
    });

    // Initialize routes
    this.accounts = new Accounts(this.client);
    this.tables = new Tables(this.client);
    this.events = new Events(this.client);
    this.transactions = new Transactions(this.client);
  }

  /** Returns the sequence number and authentication key for an account */
  async getAccount(accountAddress: MaybeHexString): Promise<Types.Account> {
    const response = await this.accounts.getAccount(HexString.ensure(accountAddress).hex());
    raiseForStatus(200, response);
    return response.data;
  }

  /** Returns transactions sent by the account */
  async getAccountTransactions(
    accountAddress: MaybeHexString,
    query?: { start?: number; limit?: number },
  ): Promise<Types.OnChainTransaction[]> {
    const response = await this.accounts.getAccountTransactions(HexString.ensure(accountAddress).hex(), query);
    raiseForStatus(200, response);
    return response.data;
  }

  /** Returns all modules associated with the account */
  async getAccountModules(
    accountAddress: MaybeHexString,
    query?: { version?: Types.LedgerVersion },
  ): Promise<Types.MoveModule[]> {
    const response = await this.accounts.getAccountModules(HexString.ensure(accountAddress).hex(), query);
    raiseForStatus(200, response);
    return response.data;
  }

  /** Returns the module identified by address and module name */
  async getAccountModule(
    accountAddress: MaybeHexString,
    moduleName: string,
    query?: { version?: Types.LedgerVersion },
  ): Promise<Types.MoveModule> {
    const response = await this.accounts.getAccountModule(HexString.ensure(accountAddress).hex(), moduleName, query);
    raiseForStatus(200, response);
    return response.data;
  }

  /** Returns all resources associated with the account */
  async getAccountResources(
    accountAddress: MaybeHexString,
    query?: { version?: Types.LedgerVersion },
  ): Promise<Types.AccountResource[]> {
    const response = await this.accounts.getAccountResources(HexString.ensure(accountAddress).hex(), query);
    raiseForStatus(200, response);
    return response.data;
  }

  /** Returns the resource by the address and resource type */
  async getAccountResource(
    accountAddress: MaybeHexString,
    resourceType: string,
    query?: { version?: Types.LedgerVersion },
  ): Promise<Types.AccountResource> {
    const response = await this.accounts.getAccountResource(
      HexString.ensure(accountAddress).hex(),
      resourceType,
      query,
    );
    raiseForStatus(200, response);
    return response.data;
  }

  /** Generates a transaction request that can be submitted to produce a raw transaction that
   * can be signed, which upon being signed can be submitted to the blockchain. */
  async generateTransaction(
    sender: MaybeHexString,
    payload: Types.TransactionPayload,
    options?: Partial<Types.UserTransactionRequest>,
  ): Promise<Types.UserTransactionRequest> {
    const senderAddress = HexString.ensure(sender);
    const account = await this.getAccount(senderAddress);
    return {
      sender: senderAddress.hex(),
      sequence_number: account.sequence_number,
      max_gas_amount: "1000",
      gas_unit_price: "1",
      gas_currency_code: "XUS",
      // Unix timestamp, in seconds + 10 seconds
      expiration_timestamp_secs: (Math.floor(Date.now() / 1000) + 10).toString(),
      payload,
      ...(options || {}),
    };
  }

  /** Converts a transaction request by `generate_transaction` into it's binary hex BCS representation, ready for
   * signing and submitting.
   * Generally you may want to use `signTransaction`, as it takes care of this step + signing */
  async createSigningMessage(txnRequest: Types.UserTransactionRequest): Promise<Types.HexEncodedBytes> {
    const response = await this.transactions.createSigningMessage(txnRequest);
    raiseForStatus(200, response, txnRequest);

    const { message } = response.data;
    return message;
  }

  /** Converts a transaction request produced by `generate_transaction` into a properly signed
   * transaction, which can then be submitted to the blockchain. */
  async signTransaction(
    accountFrom: AptosAccount,
    txnRequest: Types.UserTransactionRequest,
  ): Promise<Types.SubmitTransactionRequest> {
    const message = await this.createSigningMessage(txnRequest);
    const signatureHex = accountFrom.signHexString(message.substring(2));

    const transactionSignature: Types.TransactionSignature = {
      type: "ed25519_signature",
      public_key: accountFrom.pubKey().hex(),
      signature: signatureHex.hex(),
    };

    return { signature: transactionSignature, ...txnRequest };
  }

  async getEventsByEventKey(eventKey: Types.HexEncodedBytes): Promise<Types.Event[]> {
    const response = await this.events.getEventsByEventKey(eventKey);
    raiseForStatus(200, response, `eventKey: ${eventKey}`);
    return response.data;
  }

  async getEventsByEventHandle(
    address: MaybeHexString,
    eventHandleStruct: Types.MoveStructTagId,
    fieldName: string,
    query?: { start?: number; limit?: number },
  ): Promise<Types.Event[]> {
    const response = await this.accounts.getEventsByEventHandle(
      HexString.ensure(address).hex(),
      eventHandleStruct,
      fieldName,
      query,
    );
    raiseForStatus(200, response, { address, eventHandleStruct, fieldName });
    return response.data;
  }

  /** Submits a signed transaction to the blockchain. */
  async submitTransaction(signedTxnRequest: Types.SubmitTransactionRequest): Promise<Types.PendingTransaction> {
    const response = await this.transactions.submitTransaction(signedTxnRequest);
    raiseForStatus(202, response, signedTxnRequest);
    return response.data;
  }

  async getTransactions(query?: { start?: number; limit?: number }): Promise<Types.OnChainTransaction[]> {
    const response = await this.transactions.getTransactions(query);
    raiseForStatus(200, response);
    return response.data;
  }

  async getTransaction(txnHashOrVersion: string): Promise<Types.Transaction> {
    const response = await this.transactions.getTransaction(txnHashOrVersion);
    raiseForStatus(200, response, { txnHashOrVersion });
    return response.data;
  }

  async transactionPending(txnHash: Types.HexEncodedBytes): Promise<boolean> {
    const response = await this.transactions.getTransaction(txnHash);

    if (response.status === 404) {
      return true;
    }
    raiseForStatus(200, response, txnHash);
    return response.data.type === "pending_transaction";
  }

  /** Waits up to 10 seconds for a transaction to move past pending state */
  async waitForTransaction(txnHash: Types.HexEncodedBytes) {
    let count = 0;
    // eslint-disable-next-line no-await-in-loop
    while (await this.transactionPending(txnHash)) {
      // eslint-disable-next-line no-await-in-loop
      await sleep(1000);
      count += 1;
      if (count >= 10) {
        throw new Error(`Waiting for transaction ${txnHash} timed out!`);
      }
    }
  }

  async getLedgerInfo(params: RequestParams = {}): Promise<Types.LedgerInfo> {
    const result = await this.client.request<Types.LedgerInfo, AptosError>({
      path: "/",
      method: "GET",
      format: "json",
      ...params,
    });
    return result.data;
  }

  async getTableItem(handle: string, data: Types.TableItemRequest, params?: RequestParams): Promise<any> {
    const tableItem = await this.tables.getTableItem(handle, data, params);
    return tableItem;
  }
}
